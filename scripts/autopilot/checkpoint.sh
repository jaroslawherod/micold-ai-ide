#!/usr/bin/env bash
# One call for what a unit checks at every checkpoint, instead of one probe per call.
#
#   checkpoint.sh "<unit description>" [<ledger>]
#
# Prints the branch and how it stands against origin/main (no fetch), up to 10 changed files, the
# commits not on origin/main by patch, the ledger's Phase, Next step and open Handover or
# escalation, and last the CONTEXT line of context.py.
# Exit: context.py's (0 OK, 3 OVER: hand over, 2 context unknown).
set -uo pipefail
desc="${1:?usage: checkpoint.sh \"<unit description>\" [<ledger>]}"
ledger="${2:-}"
here="$(cd "$(dirname "$0")" && pwd)"

git status -sb | head -1 | sed 's/^## /BRANCH /'
changed="$(git status --porcelain)"
if [ -n "$changed" ]; then
  echo "CHANGED $(wc -l <<<"$changed" | tr -d ' ') file(s)"
  head -10 <<<"$changed" | sed 's/^/  /'
else
  echo "CHANGED none"
fi
unmerged="$(git cherry origin/main HEAD 2>/dev/null | grep -c '^+')"
echo "UNMERGED ${unmerged:-0} commit(s)"
git log --oneline -3 | sed 's/^/  /'

if [ -n "$ledger" ] && [ -f "$ledger" ]; then
  echo "PHASE $(sed -n 's/^- \*\*Phase\*\*: //p' "$ledger" | head -1)"
  echo "NEXT $(sed -n 's/^- \*\*Next step\*\*: //p' "$ledger" | head -1)"
  for sec in Handover "Open escalation"; do
    tag="$(tr '[:lower:] ' '[:upper:]_' <<<"$sec")"
    sed -n "/^## $sec/,/^## /p" "$ledger" | grep -v '^## ' | grep -vE '^\s*$|^None\.' \
      | head -5 | sed "s/^/$tag /"
  done
elif [ -n "$ledger" ]; then
  echo "LEDGER missing: $ledger"
fi

"$here/context.py" "$desc"
