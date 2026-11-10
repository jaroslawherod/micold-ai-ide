#!/usr/bin/env bash
# Decide merge-or-refuse for a PR from the ledger's recorded review verdicts.
#
# Usage: verdict-guard.sh <pr>        (run it from a clone of the repository)
# Reads the PR's head fresh from GitHub, finds the ledger of the PR's head branch and allows only
# when every required review's latest row is approved for that head, or for a head that differs
# only in ledger files. Writes no file in the repository and never pushes.
#
# Last line, and exit code:
#   ALLOWED <pr> <head>                                                        0
#   UNGUARDED <pr> no ledger                                                   0  no flow ledger names the branch
#   VERDICT-REFUSED <pr> reviewed <H|none> current <P|unknown> re-review: <why> 7
set -uo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/verdict-lib.sh"
trap vl_cleanup EXIT

pr="${1:?usage: verdict-guard.sh <pr>}"; pr="${pr#\#}"
P=""
refuse() { # refuse <reviewed> <reason>
  echo "VERDICT-REFUSED $pr reviewed ${1:-none} current ${P:-unknown} re-review: $2"; exit 7
}

info="$(gh pr view "$pr" --json headRefOid,headRefName -q '[.headRefOid,.headRefName]|@tsv' 2>/dev/null)" ||
  refuse none "PR head unreadable (gh failed)"
P="${info%%$'\t'*}"; branch="${info#*$'\t'}"
vl_is_sha "$P" && [ -n "$branch" ] || { P=""; refuse none "PR head unreadable"; }

vl_find_ledger "$branch"
case $? in
  0) ;;
  1) echo "UNGUARDED $pr no ledger"; exit 0 ;;
  *) refuse none "ledger unreadable" ;;
esac
ledger="$VL_LEDGER"

vl_has_head "$ledger" || refuse none "ledger has no Head column (legacy): re-review and record the verdict"
rows="$(vl_required "$ledger" "$pr")"
[ -n "$rows" ] || refuse none "no verdict recorded for this PR"

# Heads missing locally (the guard may run on another clone than the one that pushed).
have() { git cat-file -e "$1^{commit}" 2>/dev/null; }
have "$P" || git fetch -q origin "$branch" 2>/dev/null

while IFS=$'\t' read -r review round scope H verdict; do
  vl_approved "$verdict" || refuse "$H" "$review round $round is not approved ($verdict)"
  vl_is_sha "$H" || refuse none "$review round $round has no valid head"
  [ "$H" = "$P" ] && continue
  have "$H" && have "$P" || refuse "$H" "$review round $round: reviewed head or current head unreachable"
  vl_ledger_diff "$H" "$P"
  case $? in
    0) ;;
    1) refuse "$H" "$review round $round: head changed since the review" ;;
    *) refuse "$H" "$review round $round: heads cannot be compared" ;;
  esac
done <<<"$rows"
echo "ALLOWED $pr $P"
