#!/usr/bin/env bash
# Background watcher of the autopilot supervisor: polls run-status for every starting/running row of
# the supervisor ledger and exits on the first state change or at the ceiling, so the supervisor
# session wakes. Prints `CHANGED <issue> <old> <new>` or `CEILING`; exit 0. Exit 2: usage error.
# Usage: supervisor-watch.sh --ledger <f> [--interval S] [--ceiling S]
# Env: AUTOPILOT_RUN_STATUS (run-status.sh replacement), AUTOPILOT_WATCH_CEILING (default 600).
# Contract: specs/1813-supervisor-skill/contracts/supervisor-cli.md
HERE="$(cd "$(dirname "$0")" && pwd)"
LEDGER=".autopilot/supervisor.md"; interval=30; ceiling="${AUTOPILOT_WATCH_CEILING:-600}"
RS="${AUTOPILOT_RUN_STATUS:-$HERE/run-status.sh}"
die() { echo "supervisor-watch.sh: $*" >&2; exit 2; }
while [ $# -gt 0 ]; do
  case "$1" in --ledger|--interval|--ceiling) [ $# -ge 2 ] || die "$1 needs a value" ;; esac
  case "$1" in
    --ledger) LEDGER="$2"; shift 2 ;; --interval) interval="$2"; shift 2 ;;
    --ceiling) ceiling="$2"; shift 2 ;; *) die "unknown option $1" ;;
  esac
done
[ -f "$LEDGER" ] || die "no ledger at $LEDGER"
case "$interval$ceiling" in ''|*[!0-9]*) die "--interval and --ceiling are whole seconds" ;; esac

# watched: `issue worktree` of every starting/running row with a worktree.
watched() {
  awk -F'|' '$2 ~ /^ *[0-9]+ *$/ { for (i = 2; i <= 5; i++) gsub(/^ +| +$/, "", $i)
    if (($3 == "starting" || $3 == "running") && $5 != "") print $2, $5 }' "$LEDGER"
}
reading() { # worktree -> state; an unreadable run-status counts as `unreadable`
  local s; s="$("$RS" --json "$1" 2>/dev/null | jq -r '.state // empty' 2>/dev/null)"
  echo "${s:-unreadable}"
}
base="$(watched | while read -r n w; do echo "$n $(reading "$w")"; done)"
start="$(date +%s)"
while :; do
  now="$(date +%s)"
  if [ $((now - start)) -ge "$ceiling" ]; then echo CEILING; exit 0; fi
  sleep "$interval"
  while read -r n w; do
    [ -n "$n" ] || continue
    old="$(printf '%s\n' "$base" | awk -v n="$n" '$1 == n { print $2 }')"
    new="$(reading "$w")"
    if [ -n "$old" ] && [ "$old" != "$new" ]; then echo "CHANGED $n $old $new"; exit 0; fi
  done <<EOF2
$(watched)
EOF2
done
