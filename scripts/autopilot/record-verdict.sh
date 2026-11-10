#!/usr/bin/env bash
# Append a verdict row to a ledger's *Review rounds* table, bound to the head it covered.
#
# Usage: record-verdict.sh <ledger> <review> <round> <scope> <snapshot> <head> <verdict>
#   scope    a PR number, a milestone ID (M2) or `design`
#   head     the full 40-hex commit the reviewer covered (`git rev-parse HEAD`, fixes committed)
#   verdict  CLEAN, MINOR-ONLY, CHANGES or pending; free text may follow
# Upgrades a legacy `| Review | Round | Snapshot | Verdict |` header in place (old rows keep empty
# Scope and Head). Exit 0 on success, 2 with a message on bad input.
set -uo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/verdict-lib.sh"

die() { echo "record-verdict: $*" >&2; exit 2; }
[ $# -eq 7 ] || die "usage: record-verdict.sh <ledger> <review> <round> <scope> <snapshot> <head> <verdict>"
ledger="$1" review="$2" round="$3" scope="$4" snapshot="$5" head="$6" verdict="$7"
# A cell holds one line without `|`.
cell() { local v="${1//|//}"; v="${v//$'\n'/ }"; printf '%s' "${v//$'\r'/ }"; }
review="$(cell "$review")" scope="$(cell "$scope")" snapshot="$(cell "$snapshot")" verdict="$(cell "$verdict")"

vl_is_sha "$head" || die "head must be 40 hex characters, got '$head'"
vl_vocab_ok "$verdict" || die "verdict must start with CLEAN, MINOR-ONLY, CHANGES or pending, got '$verdict'"
[[ "$round" =~ ^[0-9]+$ ]] || die "round must be an integer, got '$round'"
[ -f "$ledger" ] || die "no ledger at $ledger"
grep -qxF '## Review rounds' "$ledger" || die "$ledger has no '## Review rounds' section"
[ -n "$(vl_table "$ledger" "Review rounds" | head -1)" ] || die "$ledger: the Review rounds section has no table"

tmp="$(mktemp "$ledger.XXXXXX")" || die "cannot write beside $ledger"
trap 'rm -f "$tmp"' EXIT
RV_Review="$review" RV_Round="$round" RV_Scope="$scope" RV_Snapshot="$snapshot" RV_Head="$head" \
RV_Verdict="$verdict" awk '
function trim(s) { gsub(/^[ \t]+|[ \t]+$/, "", s); return s }
function cells(line,   n) { sub(/^\|/, "", line); sub(/\|[ \t]*$/, "", line); return split(line, c, "|") }
function emit(   i, s) { s = "|"; for (i = 1; i <= m; i++) s = s " " v[i] " |"; print s }
function dump(   i) { for (i = 1; i <= nb; i++) print blank[i]; nb = 0 }
function newrow(   i) {
  if (done) return
  done = 1; m = nh
  for (i = 1; i <= nh; i++) v[i] = ENVIRON["RV_" hdr[i]]
  emit()
}
/^## / { if (insec) newrow(); dump(); insec = (substr($0, 4) == "Review rounds"); print; next }
!insec { print; next }
/^\|/ {
  dump()
  isep = ($0 ~ /^\|[ \t]*:?-+/)
  m = cells($0)
  for (i = 1; i <= m; i++) v[i] = trim(c[i])
  if (!seen) {                                   # header
    seen = 1
    legacy = 1; for (i = 1; i <= m; i++) if (v[i] == "Head") legacy = 0
    ri = 2; for (i = 1; i <= m; i++) if (v[i] == "Round") ri = i
    if (legacy) { for (i = m; i > ri; i--) v[i + 2] = v[i]; v[ri + 1] = "Scope"; v[ri + 2] = "Head"; m += 2 }
    nh = m; for (i = 1; i <= m; i++) hdr[i] = v[i]
    emit(); next
  }
  if (legacy) { for (i = m; i > ri; i--) v[i + 2] = v[i]; v[ri + 1] = isep ? "---" : ""; v[ri + 2] = isep ? "---" : ""; m += 2 }
  if (isep) { for (i = 1; i <= m; i++) v[i] = "---" }
  emit(); next
}
/^[ \t]*$/ { blank[++nb] = $0; next }
{ if (seen) newrow(); dump(); print }
END { if (insec) newrow(); dump() }
' "$ledger" >"$tmp" || die "cannot rewrite $ledger"
chmod 644 "$tmp"
mv "$tmp" "$ledger" || die "cannot replace $ledger"
trap - EXIT
