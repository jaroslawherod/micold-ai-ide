#!/usr/bin/env bash
# One state for one autopilot run, from its ledger and GitHub. Read-only.
#
# Usage: run-status.sh [--json] [<worktree-dir>]      (default: the current directory)
# States, first match wins: dead-no-ledger, done, blocked, asking, stuck, waiting-ci, running.
# Exit: 0 a state was printed · 2 usage error or not a git worktree · 3 several ledgers name the
# branch (listed on stderr, no state). Contract: specs/1812-supervisor-run-status/contracts/run-status.md
# Env: AUTOPILOT_PR_STATE (helper that reports a PR's dwell; default pr-state.sh beside this file),
#      AUTOPILOT_NOW (epoch seconds replacing the clock), AUTOPILOT_SUPERVISED.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
. "$here/ledger-of.sh"

json=0; dir=""
for a in "$@"; do
  case "$a" in
    --json) json=1 ;;
    -h|--help) sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) echo "run-status.sh: unknown option $a" >&2; exit 2 ;;
    *) [ -z "$dir" ] || { echo "run-status.sh: one worktree only" >&2; exit 2; }; dir="$a" ;;
  esac
done
dir="${dir:-.}"
top="$(git -C "$dir" rev-parse --show-toplevel 2>/dev/null)" || { echo "run-status.sh: $dir is not a git worktree" >&2; exit 2; }
cd "$top" || exit 2
b="$(git branch --show-current)"
now="${AUTOPILOT_NOW:-$(date +%s)}"
pr_state_helper="${AUTOPILOT_PR_STATE:-$here/pr-state.sh}"

# --- which ledger ------------------------------------------------------------------------------
all=""; open=""; nopen=0
while read -r f; do
  [ -n "$f" ] || continue
  all="${all:+$all$'\n'}$f"
  if [ "$(phase_of "$f")" != done ]; then open="${open:+$open$'\n'}$f"; nopen=$((nopen + 1)); fi
done < <(ledgers_of .)
if [ "$nopen" -gt 1 ]; then
  echo "run-status.sh: several ledgers name branch $b:" >&2
  printf '%s\n' "$open" | sed 's/^/  /' >&2
  exit 3
fi
ledger=""
if [ "$nopen" -eq 1 ]; then ledger="$open"; elif [ -n "$all" ]; then ledger="$(sed -n 1p <<<"$all")"; fi

state=""; underlying=""; notes=""; prs_txt=""; since=""; dwell_source=""
note() { notes="${notes:+$notes$'\n'}$1"; }

# --- escalation: the ledger's Open escalation, as JSON -----------------------------------------
# Fields per contracts/supervised-escalation.md; a free-text section gives its first line as the question.
escalation_json() {  # escalation_json <section-text>
  printf '%s\n' "$1" | awk '
    function trim(s) { gsub(/^[ \t]+|[ \t]+$/, "", s); return s }
    function out(k, n, t) { gsub(/\t/, " ", t); printf "%s\t%s\t%s\n", k, n, t }
    BEGIN { qn = 1; cur = ""; fields = 0; first = "" }
    /^### Q[0-9]+/ { qn = $0; sub(/^### Q/, "", qn); sub(/[^0-9].*$/, "", qn); cur = ""; fields = 1; next }
    /^- \*\*(Category|Question|Options|Recommended|Evidence|Banner)\*\*:/ {
      name = $0; sub(/^- \*\*/, "", name); sub(/\*\*:.*$/, "", name)
      val = $0; sub(/^- \*\*[A-Za-z]+\*\*:[ \t]*/, "", val)
      fields = 1; cur = name; txt = trim(val); n = qn
      if (cur == "Category" || cur == "Banner") n = 0
      if (cur != "Options") buf[cur SUBSEP n] = txt
      key = cur; kq = n; next
    }
    /^[ \t]*[0-9]+\.[ \t]/ && cur == "Options" { t = $0; out("O", qn, trim(t)); next }
    /^[ \t]*$/ { next }
    /^[ \t]*```/ { next }
    { if (first == "") first = trim($0)
      if (cur != "" && cur != "Options") buf[cur SUBSEP (cur == "Category" || cur == "Banner" ? 0 : qn)] = buf[cur SUBSEP (cur == "Category" || cur == "Banner" ? 0 : qn)] " " trim($0) }
    END {
      for (k in buf) { split(k, p, SUBSEP); out(substr(p[1], 1, 1), p[2], buf[k]); if (p[1] == "Question") hq = 1 }
      if (!hq && (!fields && first != "" || fields)) out("Q", 1, first)
    }' | jq -R -s '
      (split("\n") | map(select(length > 0) | split("\t"))) as $r
      | def field($k; $n): ($r | map(select(.[0] == $k and .[1] == ($n | tostring))) | first | .[2]) // "";
        def opt: sub("^[0-9]+\\.[ ]*"; "") | (split(" — ")) as $p
                 | {label: (($p[0] // "") | sub(" \\(Recommended\\)$"; "")), description: ($p[1:] | join(" — "))};
      ($r | map(select(.[0] != "C" and .[0] != "B") | .[1] | tonumber) | unique) as $ns
      | {category: field("C"; 0), banner: field("B"; 0),
         questions: [$ns[] as $n | {number: $n, question: field("Q"; $n),
           options: [$r[] | select(.[0] == "O" and .[1] == ($n | tostring)) | .[2] | opt],
           recommendation: ((field("R"; $n) | split(" — ")) + [""] | .[0] | sub(" \\(Recommended\\)$"; "")),
           evidence: field("E"; $n)}]}'
}

if [ -z "$ledger" ]; then
  state=dead-no-ledger
  git fetch -q origin 2>/dev/null || true
  if [ -n "$b" ] && onmain="$(ledger_on_main "$b")"; then
    note "ledger only on origin/main: $onmain; scripts/autopilot/branch-start.sh is next"
  fi
  issue=""; flow=""; phase=""; next=""; ledger_json=null; supervised=false
  escalation=null
else
  phase="$(phase_of "$ledger")"
  next="$(sed -n 's/^- \*\*Next step\*\*: //p' "$ledger" | head -1)"
  issue="$(sed -n 's/^- \*\*Issue\*\*: //p' "$ledger" | head -1)"; [ -n "$issue" ] || issue=none
  flow="$(sed -n 's/^- \*\*Kind\*\*: //p' "$ledger" | head -1)"; [ "$flow" = quick ] && flow=chore
  ledger_json="$(jq -n --arg l "$ledger" '$l')"
  supervised=false
  "$here/supervised.sh" "$ledger" && supervised=true

  # GitHub is the only source of a PR's state; the ledger supplies the numbers.
  open_prs=""
  for n in $(ledger_prs "$ledger"); do
    st="$(pr_state_of "$n")"
    prs_txt="${prs_txt:+$prs_txt$'\n'}$n $st"
    [ "$st" = OPEN ] && open_prs="${open_prs:+$open_prs }$n"
  done
  if [ -n "$b" ]; then
    fin="$(final_pr_open "$b")"
    if [ -n "$fin" ] && ! grep -q "^$fin " <<<"$prs_txt"; then
      prs_txt="${prs_txt:+$prs_txt$'\n'}$fin OPEN"; open_prs="${open_prs:+$open_prs }$fin"
    fi
  fi

  section="$(sed -n '/^## Open escalation/,/^## /p' "$ledger" | grep -v '^## ')"
  esc_body="$(grep -vE '^\s*$|^None\.' <<<"$section")"
  escalation=null

  if [ "$phase" = done ]; then
    if hc="$(cd "$top" && "$here/handoff-check.sh" "$ledger" 2>&1)"; then state=done
    else notes="$(grep -vx 'NOT DONE' <<<"$hc" | sed 's/^- //')"; fi
  fi
  if [ -z "$state" ] && [ -n "$esc_body" ]; then
    if grep -qiF 'blocked by work outside my flow' <<<"$esc_body"; then state=blocked; else state=asking; fi
    escalation="$(escalation_json "$esc_body")"
  fi
  if [ -z "$state" ] && [ -n "$open_prs" ]; then
    state=waiting-ci; maxdwell=""
    if [ -x "$pr_state_helper" ]; then
      for n in $open_prs; do
        line="$("$pr_state_helper" "$n" 2>/dev/null)"; rc=$?
        if [ "$rc" -eq 1 ] || grep -qE '(^| )STUCK( |$)' <<<"$line"; then state=stuck; underlying=waiting-ci; fi
        d="$(sed -n 's/.* DWELL \([0-9][0-9]*\).*/\1/p' <<<"$line" | head -1)"
        [ -n "$d" ] && { [ -z "$maxdwell" ] || [ "$d" -gt "$maxdwell" ]; } && maxdwell="$d"
      done
    fi
    if [ -n "$maxdwell" ]; then since=$((now - maxdwell)); dwell_source=pr-state; fi
  fi
  [ -n "$state" ] || state=running
  if [ -z "$since" ]; then
    if [ -n "$(git status --porcelain -- "$ledger")" ]; then
      since="$(python3 -c 'import os,sys; print(int(os.path.getmtime(sys.argv[1])))' "$ledger")"
    else
      since="$(git log -1 --format=%ct -- "$ledger" 2>/dev/null)"
      [ -n "$since" ] || since="$(python3 -c 'import os,sys; print(int(os.path.getmtime(sys.argv[1])))' "$ledger")"
    fi
    dwell_source=ledger
  fi
fi
[ -n "$underlying" ] || underlying="$state"

prs_json="$(printf '%s\n' "$prs_txt" | jq -R -s 'split("\n") | map(select(length > 0) | split(" ") | {number: (.[0] | tonumber), state: .[1]})')"
notes_json="$(printf '%s\n' "$notes" | jq -R -s 'split("\n") | map(select(length > 0))')"
since_json=null; secs_json=null
[ -n "$since" ] && { since_json="$since"; secs_json=$((now - since)); }

result="$(jq -c -n --arg state "$state" --arg under "$underlying" --arg issue "$issue" --arg flow "$flow" \
  --arg phase "$phase" --arg next "$next" --argjson ledger "$ledger_json" --argjson sup "$supervised" \
  --argjson prs "$prs_json" --argjson since "$since_json" --argjson secs "$secs_json" --arg src "$dwell_source" \
  --argjson esc "$escalation" --argjson notes "$notes_json" \
  '{state: $state, underlying_state: $under, issue: $issue, flow: $flow, phase: $phase, next_step: $next,
    ledger: $ledger, supervised: $sup, prs: $prs, since: $since, seconds_in_state: $secs, dwell_source: $src,
    escalation: $esc, notes: $notes}')"

if [ "$json" -eq 1 ]; then printf '%s\n' "$result"; exit 0; fi
jq -r '
  "STATE \(.state)",
  "underlying_state: \(.underlying_state)", "issue: \(.issue)", "flow: \(.flow)", "phase: \(.phase)",
  "next_step: \(.next_step)", "ledger: \(.ledger // "")", "supervised: \(.supervised)",
  (.prs[] | "PR #\(.number) \(.state)"),
  "since: \(.since // "")", "seconds_in_state: \(.seconds_in_state // "")", "dwell_source: \(.dwell_source)",
  (.escalation // empty | "category: \(.category)", "banner: \(.banner)",
    (.questions[] | . as $q |
      "question \($q.number): \($q.question)",
      ($q.options | to_entries[] | "option \($q.number).\(.key + 1): \(.value.label) — \(.value.description)"),
      "recommended \($q.number): \($q.recommendation)", "evidence \($q.number): \($q.evidence)")),
  (.notes[] | "note: \(.)")' <<<"$result"
