#!/usr/bin/env bash
# PreToolUse(Read) hook: on an autopilot branch, a Read of a long file must say how much it wants.
#
# Everything a unit reads is re-read on each of its later calls. In measured runs, Read results
# were over half of the tool output the milestone units carried, and the largest were whole files
# of 10-16k tokens. unit.md tells units to grep first and to use brief.py; this hook holds them
# to it.
#
# A no-op unless the session's directory is on a branch an autopilot ledger names (as
# gate-hook.sh). There it blocks, with exit 2 and the reason on stderr, a Read without `limit` of
#   a file of more than AUTOPILOT_READ_LINES lines (default 400)
#   a ledger of more than 80 lines              brief.py ledger prints what a unit needs of it
# Passing `limit` always goes through, so a caller that needs the whole file says so. Images, PDFs,
# notebooks and this skill's own files are never blocked.
set -uo pipefail
in=$(cat)
IFS=$'\t' read -r lim f cwd < <(jq -r '[(.tool_input.limit // .tool_input.pages // "-" | tostring),
  (.tool_input.file_path // "-"), (.cwd // ".")] | @tsv' <<<"$in") || exit 0
[ "$lim" = - ] || exit 0
[ -f "$f" ] || exit 0
case "$f" in
  *.png|*.jpg|*.jpeg|*.gif|*.webp|*.pdf|*.ipynb|*/.claude/skills/*) exit 0 ;;
esac
n=$(wc -l <"$f")
max="${AUTOPILOT_READ_LINES:-400}"
is_ledger=0
case "$f" in *specs/*autopilot.md) is_ledger=1; max=80 ;; esac
[ "$n" -gt "$max" ] || exit 0

cd "$cwd" 2>/dev/null || exit 0
top=$(git rev-parse --show-toplevel 2>/dev/null) && cd "$top" || exit 0
b=$(git branch --show-current 2>/dev/null)
[ -n "$b" ] || exit 0
grep -qxF -- "- **Worktree branch**: $b" specs/*/autopilot.md specs/*/bugs/*.autopilot.md specs/quick/*.autopilot.md 2>/dev/null || exit 0

if [ "$is_ledger" = 1 ]; then
  echo "autopilot read: the ledger has $n lines, most of them history you re-read on every later call. Run scripts/autopilot/brief.py ledger $f [<M-id>] for its state and your milestone's rows. To edit it, grep -n for the line, then Read with offset and limit." >&2
else
  echo "autopilot read: $f has $n lines, and a whole Read stays in your context for every later call. grep -n for the lines you need, then Read with offset and limit; a spec artifact goes through scripts/autopilot/brief.py. If you do need all of it, pass limit: $n." >&2
fi
exit 2
