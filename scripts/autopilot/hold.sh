#!/usr/bin/env bash
# Wait for something without losing the prompt cache.
#
#   hold.sh <file> [<regex>]          a unit: comes back within 4 minutes
#   hold.sh --long <file> [<regex>]   the orchestrator: comes back within 50 minutes; run it with
#                                     run_in_background and a timeout above 50 minutes
#
# A subagent's prompt cache expires after 5 idle minutes, the main session's after 60. The next call
# then writes the whole context to the cache again, which costs as much as about 12 calls (20 in the
# main session). Measured runs spent a quarter of their tokens on that. A call that comes back
# before the cache expires keeps it, for the price of one call.
#
# Waits until a line of <file> matches <regex> (default a line starting `<NAME>_EXIT=`, what the detached gate and
# wait-merge.sh write last), or the hold time passes. The file need not exist: to hold while a
# subagent runs, name a file nothing will write.
#
#   DONE <matching line>   exit 0
#   HOLD <n>/<max>         exit 0   nothing yet: call again
#   STOP <n>/<max>         exit 3   more holds would cost more than the one rebuild they avoid: wait
#                                   once in the background instead, and do not call again
#
# The count is per <file> and starts again after DONE, or when the last hold on it is older than
# three hold times. AUTOPILOT_HOLD_SECS and AUTOPILOT_HOLD_MAX override the hold time and <max>.
set -uo pipefail
secs=240 max=10
if [ "${1:-}" = --long ]; then secs=3000 max=12; shift; fi
file="${1:?usage: hold.sh [--long] <file> [<regex>]}"
pattern="${2:-^[A-Z_]*_EXIT=}"
secs="${AUTOPILOT_HOLD_SECS:-$secs}"
max="${AUTOPILOT_HOLD_MAX:-$max}"

state="${TMPDIR:-/tmp}/autopilot-hold-$(printf '%s' "$(realpath -m "$file")" | cksum | cut -d' ' -f1)"
n=0
if [ -f "$state" ] && [ $(( $(date +%s) - $(stat -c %Y "$state") )) -lt $(( secs * 3 )) ]; then
  n="$(cat "$state")"
fi

matched() { [ -f "$file" ] && grep -E -m1 -- "$pattern" "$file"; }

if line="$(matched)"; then rm -f "$state"; echo "DONE $line"; exit 0; fi
if [ "$n" -ge "$max" ]; then echo "STOP $n/$max"; exit 3; fi

end=$(( $(date +%s) + secs ))
poll=$(( secs < 5 ? secs : 5 ))
while [ "$(date +%s)" -lt "$end" ]; do
  sleep "$poll"
  if line="$(matched)"; then rm -f "$state"; echo "DONE $line"; exit 0; fi
done
n=$(( n + 1 ))
echo "$n" > "$state"
echo "HOLD $n/$max"
