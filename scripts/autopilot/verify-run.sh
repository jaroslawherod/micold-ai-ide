#!/usr/bin/env bash
# Verifies one finished autopilot run from its worktree, read-only: the handoff check on the run's
# ledger (through run-status.sh, in the run's worktree) plus the run's GitHub issue.
#
# Usage: verify-run.sh [<worktree-dir>]      (default: the current directory)
# Prints VERIFIED, or NOT DONE and one `- ` line per thing that remains.
# Exit: 0 VERIFIED · 1 NOT DONE · 2 usage error or not a git worktree · 3 several ledgers name the
# branch (listed on stderr, no verdict). Contract: specs/1816-supervisor-verify-report/contracts/verify-run.md
# Env: AUTOPILOT_PR_STATE, AUTOPILOT_NOW pass through to run-status.sh; AUTOPILOT_RUN_STATUS replaces it.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
. "$here/ledger-of.sh"
dir=""
for a in "$@"; do
  case "$a" in
    -h|--help) sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) echo "verify-run.sh: unknown option $a" >&2; exit 2 ;;
    *) [ -z "$dir" ] || { echo "verify-run.sh: one worktree only" >&2; exit 2; }; dir="$a" ;;
  esac
done
dir="${dir:-.}"

# run-status.sh rejects several open ledgers only; here any several are ambiguous.
if [ "$(ledgers_of "$dir" | grep -c .)" -gt 1 ]; then
  echo "verify-run.sh: several ledgers name the branch of $dir:" >&2
  ledgers_of "$dir" | sed 's/^/  /' >&2
  exit 3
fi

json="$("${AUTOPILOT_RUN_STATUS:-$here/run-status.sh}" --json "$dir")"; rc=$?
case "$rc" in
  0) ;;
  2|3) exit "$rc" ;;
  *) printf 'NOT DONE\n- run-status.sh failed (exit %s)\n' "$rc"; exit 1 ;;
esac

state="$(jq -r .state <<<"$json")"
phase="$(jq -r .phase <<<"$json")"
issue="$(jq -r .issue <<<"$json")"
flow="$(jq -r .flow <<<"$json")"
left=()

if [ "$state" = dead-no-ledger ]; then
  left+=("no ledger")
else
  if [ "$phase" = done ]; then
    # run-status.sh ran handoff-check.sh and kept its remaining lines (ledger PRs not MERGED included).
    while IFS= read -r l; do [ -n "$l" ] && left+=("$l"); done < <(jq -r '.notes[]' <<<"$json")
  else
    left+=("ledger phase is ${phase:-unset}")
  fi
  # A PR GitHub knows (the branch's open PR) may be missing from the ledger: list every PR not MERGED once.
  while IFS= read -r l; do
    printf '%s\n' "${left[@]:-}" | grep -qxF -- "$l" || left+=("$l")
  done < <(jq -r '.prs[] | select(.state != "MERGED") | "PR #\(.number) is \(.state)"' <<<"$json")
  # Never VERIFIED unless run-status.sh itself reads done.
  if [ "$phase" = done ] && [ "$state" != done ] && [ "${#left[@]}" -eq 0 ]; then
    left+=("run state is $state, not done")
  fi
  n="${issue#\#}"
  if [ -n "$n" ] && [ "$n" != none ]; then
    if out="$(gh issue view "$n" --json state,labels -q '.state, (.labels[].name)' 2>/dev/null)" && [ -n "$out" ]; then
      ist="$(sed -n 1p <<<"$out")"; labels="$(sed 1d <<<"$out")"
      case "$ist" in
        CLOSED) ;;
        OPEN) left+=("issue #$n is open") ;;
        *) left+=("issue #$n state UNKNOWN") ;;
      esac
      grep -qxF in-progress <<<"$labels" && left+=("issue #$n still labelled in-progress")
      if [ -n "$flow" ] && ! grep -qxF "flow:$flow" <<<"$labels"; then left+=("issue #$n lacks flow:$flow"); fi
    else
      left+=("issue #$n state UNKNOWN")
    fi
  fi
fi

if [ "${#left[@]}" -eq 0 ]; then echo VERIFIED; exit 0; fi
echo "NOT DONE"
printf -- '- %s\n' "${left[@]}"
exit 1
