#!/usr/bin/env bash
# Select a supervisor's own runs from `list_worktrees` rows (specs/1857-worktree-parent, FR-013).
#
# Usage: runs-of-parent.sh <parent-ref> [--rows <file>]                 print own runs, one ref per line
#        runs-of-parent.sh <parent-ref> --check <run> [--rows <file>]  exit 0 own, 1 refused (reason on stdout)
#        runs-of-parent.sh <parent-ref> --orphans [--rows <file>]       print "<ref> <last parent>" of runs whose parent is removed
# Rows come from stdin or --rows: the `list_worktrees` result, a JSON array or an object with a
# `worktrees` array of rows `{ref, display_name, branch, path, ..., parent, parent_state}`. A run is
# named by its `ref` (the directory name); --check also takes its `path`. A row is
# own when its `parent` equals <parent-ref> and `parent_state` is not `removed`. A row with another
# parent, none, or a removed parent is never own: report an orphan, never adopt it. --orphans needs
# the unfiltered `list_worktrees`: the daemon's `parent: <ref>` filter drops removed-parent rows. Exit 2: usage or
# empty, missing or unreadable rows. Prints only; never calls MCP or gh.
set -uo pipefail
usage() { echo "usage: runs-of-parent.sh <parent-ref> [--check <run> | --orphans] [--rows <file>]" >&2; exit 2; }
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
bad() { echo "unreadable worktree rows (want a JSON array or {\"worktrees\": [...]} of row objects)" >&2; exit 2; }
list="$(jq -c 'if type == "object" and has("worktrees") then .worktrees else . end
  | if type == "array" and all(.[]; type == "object") then . else error("no rows") end' <<<"$in" 2>/dev/null)" || bad
[ -n "$list" ] || bad
case "$mode" in
  list)
    jq -r --arg p "$ref" '.[] | select(.parent == $p and (.parent_state // "known") != "removed") | .ref' <<<"$list" || bad ;;
  orphans)
    jq -r '.[] | select(.parent_state == "removed") | "\(.ref) \(.parent // "-")"' <<<"$list" || bad ;;
  check)
    row="$(jq -c --arg n "$name" '[.[] | select(.ref == $n or .path == $n)][0] // empty' <<<"$list")" || bad
    [ -n "$row" ] || { echo "refused: $name is not listed"; exit 1; }
    id="$(jq -r '.ref' <<<"$row")"
    if [ "$(jq -r '.parent_state // "known"' <<<"$row")" = removed ]; then
      echo "refused: $id has a removed parent ($(jq -r '.parent // "-"' <<<"$row")); report it, never adopt it"; exit 1
    fi
    p="$(jq -r '.parent // empty' <<<"$row")"
    [ -n "$p" ] || { echo "refused: $id has no parent"; exit 1; }
    [ "$p" = "$ref" ] || { echo "refused: $id has parent $p, not $ref"; exit 1; }
    echo "own: $id" ;;
esac
