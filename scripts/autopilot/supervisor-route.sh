# Routing of session questions (#1815): the `## Routing` section of the supervisor ledger. Sourced by
# supervisor.sh (uses its LEDGER, die and need_ledger; bash 3.2). Contract:
# specs/1815-supervisor-question-routing/contracts/routing-cli.md. Prints; never calls gh or MCP.
ROUTE_STATES="pending asked answered resolved"
ROUTE_KEY='^\| R[0-9]+\.Q[0-9]+ \|'

# rcell <text>: one pipe-free line (`|` stored as `/`, newlines to spaces).
rcell() { printf '%s' "$1" | tr '\n\r' '  ' | tr '|' '/' | sed -E 's/^ +| +$//g'; }
rrows() { grep -E "$ROUTE_KEY" "$LEDGER" || true; }
# rhas <key>: the row exists.
rhas() { rrows | awk -F'|' -v k=" $1 " '$2 == k { f = 1 } END { exit !f }'; }
# rget <key> <cell number>: a trimmed cell (Key 1 State 2 Session 3 Via 4 Question 5 Options 6 Rec 7 Evidence 8 Answer 9 Sent 10).
rget() { rrows | awk -F'|' -v k=" $1 " -v c="$2" '$2 == k { v = $(c + 1); gsub(/^ +| +$/, "", v); print v }'; }
# rset <key> <cell number> <value>
rset() {
  rhas "$1" || die "no routing row $1"
  K=" $1 " C="$2" V="$(rcell "$3")" awk -F'|' -v OFS='|' '
    $2 == ENVIRON["K"] && $0 ~ /^\| R[0-9]+\.Q[0-9]+ \|/ { $(ENVIRON["C"] + 1) = " " ENVIRON["V"] " " }
    { print }' "$LEDGER" > "$LEDGER.tmp" && mv "$LEDGER.tmp" "$LEDGER"
}
route_section() {
  grep -q '^## Routing$' "$LEDGER" && return 0
  { echo; echo "## Routing"; echo
    echo "| Key | State | Session | Via | Question | Options | Rec | Evidence | Answer | Sent |"
    echo "|---|---|---|---|---|---|---|---|---|---|"; } >> "$LEDGER"
}
isnum() { case "$1" in ''|*[!0-9]*) return 1 ;; esac; }

route_add() {
  [ $# -eq 3 ] || die "usage: route add <issue> <session> <escalation-json-file>"
  isnum "$1" || die "route add: issue number only: $1"
  [ -f "$3" ] || die "no escalation file $3"
  jq -e . "$3" >/dev/null 2>&1 || die "unreadable escalation json $3"
  route_section
  local n=0 q count opts line
  count="$(jq '(.questions // []) | length' "$3")"
  if [ "$count" = 0 ]; then
    q="$(jq -r '.banner // ""' "$3")"
    rhas "R$1.Q1" || printf '| R%s.Q1 | pending | %s |  | %s |  |  |  |  |  |\n' "$1" "$(rcell "$2")" "$(rcell "$q")" >> "$LEDGER"
    echo "R$1.Q1"; return 0
  fi
  while [ "$n" -lt "$count" ]; do
    line="$(jq -r --argjson i "$n" '.questions[$i] | [(.number // ($i + 1) | tostring), (.question // ""),
      ([(.options // [])[] | (.label // "") + (if (.description // "") != "" then ": " + .description else "" end)] | join(" // ")),
      (.recommendation // ""), (.evidence // "")] | map(tostring | gsub("[\n\r]"; " ")) | join("\u001f")' "$3")"
    local num qq oo rr ee; IFS=$'\037' read -r num qq oo rr ee <<< "$line"
    if ! rhas "R$1.Q$num"; then
      printf '| R%s.Q%s | pending | %s |  | %s | %s | %s | %s |  |  |\n' "$1" "$num" "$(rcell "$2")" \
        "$(rcell "$qq")" "$(rcell "$oo")" "$(rcell "$rr")" "$(rcell "$ee")" >> "$LEDGER"
    fi
    echo "R$1.Q$num"; n=$((n + 1))
  done
}

# rkeys <state|-> : keys ascending by issue then number.
rkeys() {
  rrows | awk -F'|' -v s="$1" '{ k = $2; st = $3; gsub(/ /, "", k); gsub(/ /, "", st)
    if (s == "-" || st == s) { i = k; sub(/^R/, "", i); sub(/\..*/, "", i); n = k; sub(/.*\.Q/, "", n); print i, n, k } }' \
    | sort -k1,1n -k2,2n | awk '{ print $3 }'
}

route_next() {
  [ $# -eq 0 ] || die "usage: route next"
  local k
  rkeys pending | head -n 4 | while read -r k; do
    printf '%s | %s | %s | %s | %s\n' "$k" "$(rget "$k" 5)" "$(rget "$k" 6)" "$(rget "$k" 7)" "$(rget "$k" 8)"
  done
}

route_state() {
  [ $# -eq 2 ] || die "usage: route state <key> <pending|asked|answered|resolved>"
  case " $ROUTE_STATES " in *" $2 "*) ;; *) die "unknown state $2 (one of: $ROUTE_STATES)" ;; esac
  rset "$1" 2 "$2"
}

route_answer() {
  [ $# -ge 3 ] || die "usage: route answer <key> user|parent|supervisor <text>"
  local key="$1" via="$2" st=answered; shift 2
  case "$via" in user|parent) ;; supervisor) st=resolved ;; *) die "route answer: via is user, parent or supervisor" ;; esac
  rset "$key" 4 "$via"; rset "$key" 9 "$*"; rset "$key" 2 "$st"
}

# rissue <issue>: the keys of one issue, ascending.
rissue() { rkeys - | awk -v p="R$1.Q" 'index($0, p) == 1'; }

route_compose() {
  [ $# -eq 1 ] || die "usage: route compose <issue>"
  local keys k st miss="" first=1 allsup=1 body="" n
  keys="$(rissue "$1")"; [ -n "$keys" ] || die "no routing rows for issue $1"
  for k in $keys; do
    st="$(rget "$k" 2)"
    case "$st" in answered|resolved) ;; *) miss="$miss ${k#*.Q}" ;; esac
    [ "$(rget "$k" 4)" = supervisor ] || allsup=0
  done
  if [ -n "$miss" ]; then echo "missing:$miss"; return 1; fi
  echo "session $(rget "$(echo "$keys" | head -n 1)" 3)"
  set -- $keys
  for k in "$@"; do
    # a single question is answered bare (no ### Q<n> batch), as answer.sh expects
    n="${k#*.Q}"
    if [ $# -eq 1 ]; then body="$(rget "$k" 9)"; else body="$n. $(rget "$k" 9)"; fi
    if [ "$first" = 1 ] && [ "$allsup" = 1 ]; then body="--by supervisor $body"; fi
    if [ "$first" = 1 ]; then body="resume $body"; fi
    first=0; echo "$body"
  done
}

route_sent() {
  [ $# -eq 2 ] || die "usage: route sent <issue> <epoch>"
  isnum "$2" || die "route sent: epoch seconds only"
  local k keys; keys="$(rissue "$1")"; [ -n "$keys" ] || die "no routing rows for issue $1"
  for k in $keys; do rset "$k" 10 "$2"; done
}

route_drop() {
  [ $# -eq 1 ] || die "usage: route drop <issue>"
  isnum "$1" || die "route drop: issue number only: $1"
  P="R$1.Q" awk -F'|' '{ k = $2; gsub(/ /, "", k) } index(k, ENVIRON["P"]) == 1 && $0 ~ /^\| R/ { next } { print }' \
    "$LEDGER" > "$LEDGER.tmp" && mv "$LEDGER.tmp" "$LEDGER"
}

route_table() { awk '/^## Routing$/ { f = 1; next } f && /^## / { f = 0 } f && NF { print }' "$LEDGER"; }

route_cmd() {
  local sub="${1:-}"; [ $# -gt 0 ] && shift
  need_ledger
  case "$sub" in
    add) route_add "$@" ;; next) route_next "$@" ;; state) route_state "$@" ;;
    answer) route_answer "$@" ;; compose) route_compose "$@" ;; sent) route_sent "$@" ;;
    drop) route_drop "$@" ;; table) route_table ;;
    *) die "usage: route add|next|state|answer|compose|sent|drop|table" ;;
  esac
}
