#!/usr/bin/env bash
# Decide merge-or-refuse for a PR from the ledger's recorded review verdicts.
#
# Usage: verdict-guard.sh <pr>        (run it from a clone of the repository)
# Reads the PR's head fresh from GitHub, finds the ledger of the PR's head branch and allows only
# when every required review's latest row is approved for that head, for a head that differs only
# in ledger files, or (carry-forward) for a head with the same patch-ids over the same touched
# paths, recorded as a `verdict carried H -> P` PR comment, once per (H,P). Writes no file in the
# repository and never pushes.
#
# Last line, and exit code:
#   ALLOWED <pr> <head>                                                        0
#   CARRIED <pr> <H> <P>                                                       0  an approved verdict carried over a clean replay
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

info="$(gh pr view "$pr" --json headRefOid,headRefName,baseRefName -q '[.headRefOid,.headRefName,(.baseRefName // "main")]|@tsv' 2>/dev/null)" ||
  refuse none "PR head unreadable (gh failed)"
P="${info%%$'\t'*}"; rest="${info#*$'\t'}"; branch="${rest%%$'\t'*}"; base="${rest#*$'\t'}"
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

# Carry a verdict from H to P (FR-005/FR-006). Returns 0 carried, 1 not carried (sets WHY).
carried=""; B=""; WHY=""
carry() { # carry <H>
  local ih ip body
  if [ -z "$B" ]; then
    git fetch -q origin "$base" 2>/dev/null && B="$(git rev-parse -q --verify FETCH_HEAD 2>/dev/null)" || { WHY="base unreadable"; return 1; }
  fi
  ih="$(vl_identity "$1" "$B")"; case $? in 0) ;; 1) WHY="reviewed head has a merge commit or no change"; return 1 ;; *) WHY="heads cannot be compared"; return 1 ;; esac
  ip="$(vl_identity "$P" "$B")"; case $? in 0) ;; 1) WHY="current head has a merge commit or no change"; return 1 ;; *) WHY="heads cannot be compared"; return 1 ;; esac
  [ "$ih" = "$ip" ] || { WHY="head changed since the review"; return 1; }
  case " $carried " in *" $1 "*) return 0 ;; esac
  body="$(gh pr view "$pr" --json comments -q '.comments[].body' 2>/dev/null)" || { WHY="comments unreadable (gh failed)"; return 1; }
  if ! grep -qF -- "verdict carried $1 -> $P" <<<"$body"; then
    gh pr comment "$pr" --body "verdict carried $1 -> $P" >/dev/null 2>&1 || { WHY="carry comment not posted (gh failed)"; return 1; }
  fi
  carried="$carried $1"; return 0
}

while IFS=$'\t' read -r review round scope H verdict; do
  vl_approved "$verdict" || refuse "$H" "$review round $round is not approved ($verdict)"
  vl_is_sha "$H" || refuse none "$review round $round has no valid head"
  [ "$H" = "$P" ] && continue
  have "$H" && have "$P" || refuse "$H" "$review round $round: reviewed head or current head unreachable"
  vl_ledger_diff "$H" "$P"
  case $? in
    0) ;;
    1) carry "$H" || refuse "$H" "$review round $round: $WHY" ;;
    *) refuse "$H" "$review round $round: heads cannot be compared" ;;
  esac
done <<<"$rows"
if [ -n "$carried" ]; then echo "CARRIED $pr ${carried##* } $P"; else echo "ALLOWED $pr $P"; fi
