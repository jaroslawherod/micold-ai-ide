#!/usr/bin/env bash
# Record the answer to a supervised run's open escalation: one *Decisions* row per question (By `user`),
# then *Open escalation* back to `None.`. The only writer of those rows.
#
# Usage: answer.sh <ledger> <answer-text>     (the text after the word `resume`)
#   a batch (`### Q<n>` subsections) takes one line per question, `<n>. <answer>` or `<n>: <answer>`
# Exit: 0 recorded (rows printed) · 2 batch answer misses or repeats a question number (ledger
#       unchanged) · 3 no open escalation (`ignored: no open escalation`, ledger unchanged)
# Contract: specs/1812-supervisor-run-status/contracts/supervised-escalation.md
set -uo pipefail
[ $# -eq 2 ] && [ -f "$1" ] || { echo "usage: answer.sh <ledger> <answer-text>" >&2; exit 2; }
ledger="$1"; answer="$2"

section() { awk -v h="## $1" '$0 == h { f = 1; next } f && /^## / { exit } f' "$ledger"; }
cell() { tr '\n' ' ' <<<"$1" | sed 's/|/\\|/g; s/  */ /g; s/^ //; s/ $//'; }
field() {  # field <text> <name>: the first line of "- **<name>**: ..."
  sed -n "s/^- \*\*$2\*\*:[ ]*//p" <<<"$1" | head -1
}

esc="$(section "Open escalation" | grep -vE '^[[:space:]]*(```.*)?$')"
if [ -z "$esc" ] || [[ "$esc" == None.* ]]; then echo "ignored: no open escalation"; exit 3; fi

phase="$(sed -n 's/^- \*\*Phase\*\*: //p' "$ledger" | head -1)"
last="$(section Decisions | grep -oE '^\| D[0-9]+ ' | grep -oE '[0-9]+' | sort -n | tail -1)"
n=${last:-0}
rows=""
add_row() {  # add_row <question> <answer> <evidence>
  n=$((n + 1))
  rows+="| D$n | $phase | $(cell "$1") | $(cell "$2") | user | $(cell "$3") |"$'\n'
}

qs="$(grep -oE '^### Q[0-9]+' <<<"$esc" | grep -oE '[0-9]+')"
if [ -z "$qs" ]; then
  q="$(field "$esc" Question)"; [ -n "$q" ] || q="$(sed -n 1p <<<"$esc")"
  add_row "$q" "$answer" "answered $(date +%F)"
else
  parsed="$(awk '
    match($0, /^[ \t]*[0-9]+[.:][ \t]*/) { n = $0; sub(/^[ \t]*/, "", n); sub(/[^0-9].*$/, "", n)
      t = $0; sub(/^[ \t]*[0-9]+[.:][ \t]*/, "", t); cur = n; txt[n] = (n in txt ? txt[n] " " : "") t
      cnt[n]++; next }
    NF && cur != "" { txt[cur] = txt[cur] " " $0 }
    END { for (k in txt) printf "%s\t%d\t%s\n", k, cnt[k], txt[k] }' <<<"$answer")"
  bad=""
  for k in $qs; do
    c="$(awk -F'\t' -v k="$k" '$1 == k { print $2 }' <<<"$parsed")"
    [ -z "$c" ] && bad+="missing answer to question $k; "
    [ "${c:-0}" -gt 1 ] && bad+="question $k answered twice; "
  done
  [ -n "$bad" ] && { echo "answer.sh: ${bad%; }" >&2; echo "answer.sh: ${bad%; }"; exit 2; }
  for k in $qs; do
    block="$(awk -v k="$k" '/^### Q[0-9]+/ { m = $0; sub(/^### Q/, "", m); sub(/[^0-9].*$/, "", m); f = (m == k); next } f' <<<"$esc")"
    add_row "$(field "$block" Question)" "$(awk -F'\t' -v k="$k" '$1 == k { print $3 }' <<<"$parsed")" "answered $(date +%F), Q$k"
  done
fi

rowsf="$(mktemp)"; tmp="$(mktemp "$ledger.XXXXXX")"
printf '%s' "$rows" > "$rowsf"
awk -v rf="$rowsf" '
  function flush(   i, last, line) {
    last = 0; for (i = 1; i <= nb; i++) if (buf[i] ~ /^\|/) last = i
    if (!last) { while (nb > 0 && buf[nb] ~ /^[ \t]*$/) nb--; last = nb
      buf[++nb] = ""; buf[++nb] = "| # | Phase | Question | Answer | By | Evidence |"; buf[++nb] = "|---|---|---|---|---|---|"; last = nb; buf[++nb] = "" }
    for (i = 1; i <= nb; i++) { print buf[i]; if (i == last) while ((getline line < rf) > 0) print line }
    nb = 0
  }
  /^## / { if (dec) { flush(); dec = 0 } skip = 0
    if ($0 == "## Decisions") { dec = 1; print; next }
    if ($0 == "## Open escalation") { print; print ""; print "None."; print ""; skip = 1; next } }
  skip { next }
  dec { buf[++nb] = $0; next }
  { print }
  END { if (dec) flush() }' "$ledger" > "$tmp" && chmod --reference="$ledger" "$tmp" && mv "$tmp" "$ledger"
rm -f "$rowsf" "$tmp"
printf '%s' "$rows"
