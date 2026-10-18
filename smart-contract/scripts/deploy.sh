#!/usr/bin/env bash
#
# Deploy the payment request registry to testnet and initialise it IN THE SAME TRANSACTION.
#
# A contract that exists but is not initialised is a window in which anyone can call `initialize`
# and become its admin. `stellar contract deploy … -- <constructor args>` closes that window.
#
# Re-runnable from nothing. Testnet resets, and when it does the deployed contract goes with it;
# every identifier here comes from the CLI or from .deploy/env, so nothing is hard-coded and the
# rebuild is a script rather than a search. Target is under ten minutes — see docs/15-risks.md.
#
#   make deploy KEY=deployer

set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/lib.sh

preflight

ADMIN_KEY="${KEY}"
SETTLER_KEY="${SETTLER_KEY:-$KEY}"

ADMIN=$(address_of "$ADMIN_KEY")
SETTLER=$(address_of "$SETTLER_KEY")

say
say "${C_B}Deploying payment_request_registry${C_0}  network=$NETWORK"
note "admin   $ADMIN"
note "settler $SETTLER"
if [ "$ADMIN" = "$SETTLER" ]; then
    note "admin and settler are the same key. Fine for a local run; question 01 in docs/15-risks.md"
    note "asks whether the real settler should be a dedicated backend key. Set SETTLER_KEY to split."
fi
say

step "Building the release wasm"
cargo build --workspace --target wasm32v1-none --release >/dev/null
WASM="target/wasm32v1-none/release/payment_request_registry.wasm"
[ -f "$WASM" ] || die "expected $WASM, but the build produced nothing"
note "$(wc -c < "$WASM" | tr -d ' ') bytes"

step "Checking the no-custody gate before anything reaches a network"
./scripts/no_custody_gate.sh >/dev/null || die "the registry contains a token call; refusing to deploy"
ok "no token calls"

step "Deploying and initialising in one transaction"
REGISTRY=$(stellar contract deploy \
    --wasm "$WASM" \
    --source "$ADMIN_KEY" \
    --network "$NETWORK" \
    -- \
    --admin "$ADMIN" \
    --settler "$SETTLER" 2>/dev/null | tail -1)

[ -n "$REGISTRY" ] || die "deployment produced no contract id"
case "$REGISTRY" in C*) ;; *) die "expected a contract id starting with C, got: $REGISTRY" ;; esac

record "E1" "Deploy registry contract" "$REGISTRY" "$(expert_contract "$REGISTRY")"

step "Reading the configuration back from the contract"
READ_ADMIN=$(stellar contract invoke --id "$REGISTRY" --source "$ADMIN_KEY" --network "$NETWORK" \
    -- get_admin 2>/dev/null | tr -d '"')
READ_SETTLER=$(stellar contract invoke --id "$REGISTRY" --source "$ADMIN_KEY" --network "$NETWORK" \
    -- get_settler 2>/dev/null | tr -d '"')

[ "$READ_ADMIN" = "$ADMIN" ]     || die "admin read back as $READ_ADMIN, expected $ADMIN"
[ "$READ_SETTLER" = "$SETTLER" ] || die "settler read back as $READ_SETTLER, expected $SETTLER"
ok "admin and settler confirmed on chain"

write_env \
    "REGISTRY=$REGISTRY" \
    "ADMIN=$ADMIN" \
    "SETTLER=$SETTLER" \
    "ADMIN_KEY=$ADMIN_KEY" \
    "SETTLER_KEY=$SETTLER_KEY"

write_evidence

say
ok "Registry deployed"
note "contract  $REGISTRY"
note "next      ./backend/scripts/setup_treasury.sh"
