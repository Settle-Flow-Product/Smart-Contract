#!/usr/bin/env bash
#
# The no-custody gate.
#
# SOW 1's registry contract can reach testnet without a security audit for exactly one reason: it
# has no function that can move an asset. That property is worth nothing if it is only remembered,
# so it is checked mechanically on every push, and it is the first item on the Day 4 review
# checklist. See ../../docs/04-contract-interface.md.
#
# The naive version of this gate — grep the sources for `transfer` — fails immediately, because the
# file that documents the rule necessarily contains the words the rule forbids. So comments are
# stripped before scanning: block comments first, then line and doc comments. What remains is code.
#
# Exit 0 = clean. Exit 1 = a token call reached the crate.

set -euo pipefail
cd "$(dirname "$0")/.."

SRC="${1:-src}"

# Forbidden in code: the SDK token client, any transfer call, and the SAC interface.
# Parentheses are escaped: in an extended regular expression a bare `(` opens a group, and BSD
# grep answers that with "empty (sub)expression" on stderr and an exit status that reads as "no
# match" — a gate that silently passes. A false pass here is worse than a false failure.
PATTERN='token::Client|TokenClient|StellarAssetClient|\.transfer\(|\.transfer_from\(|\.burn\(|\.mint\('

fail=0
while IFS= read -r file; do
    # Strip /* ... */ across lines, then // ... to end of line. Anything left is code.
    code=$(perl -0777 -pe 's{/\*.*?\*/}{}gs; s{//.*$}{}gm' "$file")
    hits=$(printf '%s' "$code" | grep -nE "$PATTERN") || true
    if [ -n "$hits" ]; then
        echo "no-custody gate FAILED in $file:"
        printf '%s\n' "$hits"
        fail=1
    fi
done < <(find "$SRC" -name '*.rs' -type f)

if [ "$fail" -ne 0 ]; then
    cat <<'MSG'

The registry has left the no-custody scope of SOW 1.

A contract that can move a token needs a security review before it reaches any network, and it is
not what the statement of work describes. Either remove the call, or open the conversation about
scope before going further.
MSG
    exit 1
fi

echo "no-custody gate passed: no token calls in $SRC"
