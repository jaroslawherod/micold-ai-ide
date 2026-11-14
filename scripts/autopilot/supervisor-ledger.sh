#!/usr/bin/env bash
# Sourced by follow-up.sh (and the close-out report scripts): reads and writes the parts of the
# supervisor's ledger. Shapes: specs/1816-supervisor-verify-report/data-model.md, Supervisor ledger.
# Bash 3.2 safe (no associative arrays, no mapfile). A missing file reads as empty.
#
#   sl_auto_remove <file>                 prints yes when the line `- **Auto-remove**: yes` is there, else no
#   sl_interventions <file> <branch>      the *Interventions* rows of the run, one per line
#   sl_followup_row <file> <branch>       `<count> <hash> <check>` of the run's *Follow-ups* row, or `0 - -`
#   sl_followup_set <file> <branch> <count> <hash> <check>   writes (adds or replaces) that row
#   sl_escalation_exists <file> <branch>  exit 0 when the run has a block in *Escalations*
#   sl_escalation_write <file> <branch>   appends the block read from stdin under `### <branch>`
#   sl_escalations <file> <branch>        the run's block, without its heading

sl_auto_remove() {
  if [ -f "$1" ] && grep -qxF -- '- **Auto-remove**: yes' "$1"; then echo yes; else echo no; fi
}

# sl_section <file> <heading>: the lines of `## <heading>`, without the heading.
sl_section() {
  [ -f "$1" ] || return 0
  awk -v h="## $2" '$0 == h { on = 1; next } /^## / { on = 0 } on' "$1"
}

sl_interventions() {
  sl_section "$1" Interventions | awk -F'|' -v b="$2" '
    { r = $2; gsub(/^ +| +$/, "", r) } r == b { print }'
}

sl_followup_row() {
  local row
  row="$(sl_section "$1" Follow-ups | awk -F'|' -v b="$2" '
    { r = $2; gsub(/^ +| +$/, "", r) }
    r == b { for (i = 3; i <= 5; i++) { v = $i; gsub(/^ +| +$/, "", v); if (v == "") v = "-"; printf "%s%s", (i > 3 ? " " : ""), v } printf "\n"; exit }')"
  echo "${row:-0 - -}"
}

# sl_put <file> <heading> <header-lines> <replace-key> <block>: puts <block> at the end of the
# section (the heading and <header-lines> are added first when the section is missing). With a
# <replace-key>, a line of the section that starts with it is replaced instead.
sl_put() {
  local file="$1" heading="$2" header="$3" key="$4" block="$5" tmpf
  [ -f "$file" ] || : > "$file"
  if ! grep -qxF -- "## $heading" "$file"; then
    { [ -s "$file" ] && echo; printf '## %s\n\n' "$heading"; [ -n "$header" ] && printf '%s\n' "$header"; } >> "$file"
  fi
  tmpf="$(mktemp "${TMPDIR:-/tmp}/sl.XXXXXX")" || return 1
  SL_BLOCK="$block" SL_KEY="$key" awk -v h="## $heading" '
    function cell(l,   a) { split(l, a, "|"); gsub(/^ +| +$/, "", a[2]); return a[2] }
    function release(   i) { for (i = 1; i <= nh; i++) print hold[i]; nh = 0 }
    $0 == h { print; on = 1; next }
    /^## / && on { if (!done) { print ENVIRON["SL_BLOCK"]; done = 1 } release(); on = 0 }
    on && $0 == "" { hold[++nh] = $0; next }
    on && ENVIRON["SL_KEY"] != "" && /^\|/ && cell($0) == ENVIRON["SL_KEY"] { release(); print ENVIRON["SL_BLOCK"]; done = 1; next }
    on { release() }
    { print }
    END { if (on && !done) { print ENVIRON["SL_BLOCK"] } release() }
  ' "$file" > "$tmpf" && cat "$tmpf" > "$file"
  rm -f "$tmpf"
}

sl_followup_set() {
  sl_put "$1" Follow-ups $'| Run | Count | Remaining hash | Check |\n|---|---|---|---|' "$2" \
    "| $2 | $3 | $4 | $5 |"
}

sl_escalation_exists() {
  [ -f "$1" ] && grep -qxF -- "### $2" "$1"
}

sl_escalation_write() {
  local body; body="$(cat)"
  sl_put "$1" Escalations "" "" "### $2
$body"
}

# sl_followup_reset <file> <branch>: zeroes the run's count and removes its escalation block, so the
# supervisor may try again and escalate again at the next cap.
sl_followup_reset() {
  [ -f "$1" ] || return 0
  local tmpf; tmpf="$(mktemp "${TMPDIR:-/tmp}/sl.XXXXXX")" || return 1
  awk -v b="### $2" '
    $0 == b { skip = 1; next }
    skip && /^##/ { skip = 0 }
    skip && $0 == "" { next }
    !skip { print }' "$1" > "$tmpf" && cat "$tmpf" > "$1"
  rm -f "$tmpf"
  grep -q . <<<"$(sl_section "$1" Follow-ups | awk -F'|' -v b="$2" '{ r = $2; gsub(/^ +| +$/, "", r) } r == b')" \
    && sl_followup_set "$1" "$2" 0 - -
  return 0
}

sl_escalations() {
  sl_section "$1" Escalations | awk -v h="### $2" '$0 == h { on = 1; next } /^### / { on = 0 } on'
}

# Run directly: supervisor-ledger.sh followup-reset <file> <branch>
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  case "${1:-}" in
    followup-reset) [ $# -eq 3 ] || { echo "usage: supervisor-ledger.sh followup-reset <file> <branch>" >&2; exit 2; }
      sl_followup_reset "$2" "$3" ;;
    *) echo "usage: supervisor-ledger.sh followup-reset <file> <branch>" >&2; exit 2 ;;
  esac
fi
