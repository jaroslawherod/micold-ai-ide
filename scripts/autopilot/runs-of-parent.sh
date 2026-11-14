#!/usr/bin/env bash
# Select a supervisor's own runs from `list_worktrees` rows (specs/1857-worktree-parent, FR-013).
#
# Usage: runs-of-parent.sh <parent-ref> [--rows <file>]                 print own runs, one name per line
#        runs-of-parent.sh <parent-ref> --check <name> [--rows <file>]  exit 0 own, 1 refused (reason on stdout)
#        runs-of-parent.sh <parent-ref> --orphans [--rows <file>]       print "<name> <last parent>" of runs whose parent is removed
# Rows come from stdin or --rows: a JSON array, or an object with a `worktrees` array. A row is
# own when its `parent` equals <parent-ref> and `parent_state` is not `removed`. A row with another
# parent, none, or a removed parent is never own: report an orphan, never adopt it. Exit 2: usage or
# unreadable JSON. Prints only; never calls MCP or gh.
set -uo pipefail
usage() { echo "usage: runs-of-parent.sh <parent-ref> [--check <name> | --orphans] [--rows <file>]" >&2; exit 2; }
[ $# -ge 1 ] && [ "${1#-}" = "$1" ] || usage
ref="$1"; shift
mode=list; name=""; rows=""
while [ $# -gt 0 ]; do
  case "$1" in
    --check) [ $# -ge 2 ] || usage; mode=check; name="$2"; shift 2 ;;
    --orphans) mode=orphans; shift ;;
    --rows) [ $# -ge 2 ] || usage; rows="$2"; shift 2 ;;
    *) usage ;;
  esac
done
if [ -n "$rows" ]; then in="$(cat "$rows")" || { echo "unreadable rows file $rows" >&2; exit 2; }; else in="$(cat)"; fi
list="$(jq -c 'if type == "object" then (.worktrees // []) else . end | if type == "array" then . else error("no rows") end' <<<"$in" 2>/dev/null)" \
  || { echo "unreadable worktree rows (want a JSON array or {\"worktrees\": [...]})" >&2; exit 2; }
case "$mode" in
  list)
    jq -r --arg p "$ref" '.[] | select(.parent == $p and (.parent_state // "known") != "removed") | .name' <<<"$list" ;;
  orphans)
    jq -r '.[] | select(.parent_state == "removed") | "\(.name) \(.parent // "-")"' <<<"$list" ;;
  check)
    row="$(jq -c --arg n "$name" '[.[] | select(.name == $n)][0] // empty' <<<"$list")"
    [ -n "$row" ] || { echo "refused: $name is not listed"; exit 1; }
    if [ "$(jq -r '.parent_state // "known"' <<<"$row")" = removed ]; then
      echo "refused: $name has a removed parent ($(jq -r '.parent // "-"' <<<"$row")); report it, never adopt it"; exit 1
    fi
    p="$(jq -r '.parent // empty' <<<"$row")"
    [ -n "$p" ] || { echo "refused: $name has no parent"; exit 1; }
    [ "$p" = "$ref" ] || { echo "refused: $name has parent $p, not $ref"; exit 1; }
    echo "own: $name" ;;
esac
