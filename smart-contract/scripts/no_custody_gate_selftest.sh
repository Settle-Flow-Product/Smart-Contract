#!/usr/bin/env bash
#
# Proves the no-custody gate can actually fail.
#
# A gate nobody has watched reject something is not a gate. This plants a violation in a scratch
# directory, asserts the gate catches it, then asserts the gate ignores the same words inside a
# comment — the false positive that made the first version of this script unusable.

set -euo pipefail
cd "$(dirname "$0")/.."

GATE=./scripts/no_custody_gate.sh
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# 1 · a real token call must be caught
cat > "$tmp/violation.rs" <<'RS'
pub fn pay(env: &Env, asset: &Address, to: &Address, amount: i128) {
    token::Client::new(env, asset).transfer(&env.current_contract_address(), to, &amount);
}
RS
if "$GATE" "$tmp" >/dev/null 2>&1; then
    echo "SELFTEST FAILED: the gate did not catch a token transfer"
    exit 1
fi

# 2 · the same words in a comment must NOT be caught
rm "$tmp/violation.rs"
cat > "$tmp/documented.rs" <<'RS'
//! There is no token::Client and no .transfer( call anywhere in this crate.
/* Nor a .mint( in a block comment. */
pub fn nothing() {}
RS
if ! "$GATE" "$tmp" >/dev/null 2>&1; then
    echo "SELFTEST FAILED: the gate flagged a comment"
    exit 1
fi

echo "no-custody gate self-test passed: catches code, ignores comments"
