#!/usr/bin/env bash
# Is this autopilot run supervised? Exit 0 when AUTOPILOT_SUPERVISED is set, non-empty and not `0`,
# or the ledger has `- **Supervised**: yes`; exit 1 otherwise. Prints nothing.
# Usage: supervised.sh [<ledger>]       Contract: specs/1812-supervisor-run-status/contracts/supervised-escalation.md
{ [ -n "${AUTOPILOT_SUPERVISED:-}" ] && [ "${AUTOPILOT_SUPERVISED}" != 0 ]; } && exit 0
[ -n "${1:-}" ] && [ -f "$1" ] && grep -qxF -- '- **Supervised**: yes' "$1" && exit 0
exit 1
