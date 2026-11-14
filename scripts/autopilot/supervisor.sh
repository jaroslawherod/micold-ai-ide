#!/usr/bin/env bash
# Decisions of the autopilot supervisor skill, reproducible from fixtures: the supervisor ledger
# (.autopilot/supervisor.md) and the start gate and queue. Prints; never calls gh or MCP.
# Usage: supervisor.sh <ledger init|owner|set|get|table | next | gate | map | retry | check-owner> [--ledger <file>] ...
# Exit: 0 answered, 1 gate FAIL, 2 usage error or malformed ledger.
# Contract: specs/1813-supervisor-skill/contracts/supervisor-cli.md (bash 3.2: no associative arrays)
STATES="queued starting running done failed lost skipped"
FIELDS="Issue State Touches Worktree Session Started Tries Note"
LEDGER=".autopilot/supervisor.md"

die() { echo "supervisor.sh: $*" >&2; exit 2; }

# Pull `--ledger <f>` out of the arguments wherever it stands; the rest stays in ARGS.
ARGS=()
while [ $# -gt 0 ]; do
  case "$1" in
    --ledger) [ $# -ge 2 ] || die "--ledger needs a file"; LEDGER="$2"; shift 2 ;;
    *) ARGS[${#ARGS[@]}]="$1"; shift ;;
  esac
done
set -- ${ARGS[@]+"${ARGS[@]}"}
[ $# -ge 1 ] || die "usage: supervisor.sh <ledger|next|gate> ..."
cmd="$1"; shift

hdr() { grep -m1 -- "^- \*\*$1\*\*:" "$LEDGER" | sed 's/^- \*\*[A-Za-z]*\*\*: *//'; }
need_ledger() {
  [ -f "$LEDGER" ] || die "no ledger at $LEDGER"
  grep -q '^- \*\*Cap\*\*: [0-9][0-9]*$' "$LEDGER" && grep -q '^| Issue | State |' "$LEDGER" \
    || die "malformed ledger $LEDGER"
}
# rows: one `issue state` pair per ledger row.
rows() { awk -F'|' '$2 ~ /^ *[0-9]+ *$/ { a=$2; b=$3; gsub(/ /,"",a); gsub(/ /,"",b); print a, b }' "$LEDGER"; }
occupied() { rows | awk '$2 == "starting" || $2 == "running"' | wc -l | tr -d ' '; }
col() { # field name -> column number (1-based among the row's cells)
  local i=0 f; for f in $FIELDS; do i=$((i + 1)); [ "$f" = "$1" ] && { echo "$i"; return; }; done
  die "unknown field $1 (one of: $FIELDS)"
}

ledger_cmd() {
  local sub="${1:-}"; [ $# -gt 0 ] && shift
  case "$sub" in
    init)
      local owner="" cap="" grace=600 source=""
      while [ $# -gt 0 ]; do
        case "$1" in --owner|--cap|--grace|--source) [ $# -ge 2 ] || die "$1 needs a value" ;; esac
        case "$1" in
          --owner) owner="$2"; shift 2 ;; --cap) cap="$2"; shift 2 ;;
          --grace) grace="$2"; shift 2 ;; --source) source="$2"; shift 2 ;;
          -*) die "unknown option $1" ;; *) break ;;
        esac
      done
      [ -e "$LEDGER" ] && die "ledger exists: $LEDGER"
      case "$cap" in ''|*[!0-9]*) die "ledger init needs --cap <number>" ;; esac
      local n; for n in "$@"; do
        case "$n" in ''|*[!0-9]*) die "issue numbers only: $n" ;; esac
      done
      [ "$(printf '%s\n' "$@" | sort | uniq -d | wc -l | tr -d ' ')" = 0 ] || die "duplicate issue numbers"
      mkdir -p "$(dirname "$LEDGER")"
      {
        echo "# Autopilot supervisor ledger"
        echo "- **Owner**: $owner"; echo "- **Cap**: $cap"; echo "- **Grace**: $grace"; echo "- **Source**: $source"
        echo; echo "| Issue | State | Touches | Worktree | Session | Started | Tries | Note |"
        echo "|---|---|---|---|---|---|---|---|"
        for n in "$@"; do
          echo "| $n | queued | - |  |  |  | 0 |  |"
        done
      } > "$LEDGER" ;;
    owner)
      need_ledger; [ $# -eq 1 ] || die "usage: ledger owner <id>"
      V="$1" awk '/^- \*\*Owner\*\*:/ { print "- **Owner**: " ENVIRON["V"]; next } { print }' "$LEDGER" > "$LEDGER.tmp" \
        && mv "$LEDGER.tmp" "$LEDGER" ;;
    set)
      need_ledger; [ $# -eq 3 ] || die "usage: ledger set <issue> <field> <value>"
      local c; c="$(col "$2")" || exit 2
      case "$3" in *'|'*) die "a cell holds no pipe" ;; esac
      if [ "$2" = State ]; then case " $STATES " in *" $3 "*) ;; *) die "unknown state $3 (one of: $STATES)" ;; esac; fi
      rows | awk -v n="$1" '$1 == n { f = 1 } END { exit !f }' || die "no row for issue $1"
      N="$1" C="$c" V="$3" awk -F'|' -v OFS='|' '
        { k = $2; gsub(/ /, "", k) }
        k == ENVIRON["N"] && $2 ~ /^ *[0-9]+ *$/ { $(ENVIRON["C"] + 1) = " " ENVIRON["V"] " " }
        { print }' "$LEDGER" > "$LEDGER.tmp" && mv "$LEDGER.tmp" "$LEDGER" ;;
    get)
      need_ledger; [ $# -eq 2 ] || die "usage: ledger get <issue> <field>"
      local c; c="$(col "$2")" || exit 2
      rows | awk -v n="$1" '$1 == n { f = 1 } END { exit !f }' || die "no row for issue $1"
      N="$1" C="$c" awk -F'|' '{ k = $2; gsub(/ /, "", k) }
        k == ENVIRON["N"] && $2 ~ /^ *[0-9]+ *$/ { v = $(ENVIRON["C"] + 1); gsub(/^ +| +$/, "", v); print v }' "$LEDGER" ;;
    table)
      need_ledger
      echo "cap $(hdr Cap) occupied $(occupied)"
      awk -F'|' '$2 ~ /^ *[0-9]+ *$/ { for (i = 2; i <= 9; i++) { gsub(/^ +| +$/, "", $i) }
        printf "#%s %s%s%s\n", $2, $3, ($6 != "" ? " " $6 : ""), ($9 != "" ? " " $9 : "") }' "$LEDGER" ;;
    *) die "usage: ledger init|owner|set|get|table" ;;
  esac
}

next_cmd() {
  need_ledger
  local slots=$(( $(hdr Cap) - $(occupied) ))
  [ "$slots" -gt 0 ] || return 0
  rows | awk '$2 == "queued" { print $1 }' | sort -n | head -n "$slots"
}

mem_mib() {
  [ -n "${AUTOPILOT_SUP_MEM_MIB:-}" ] && { echo "$AUTOPILOT_SUP_MEM_MIB"; return; }
  if [ "$(uname)" = Darwin ]; then
    local ps; ps="$(sysctl -n hw.pagesize 2>/dev/null || echo 4096)"
    vm_stat | awk -v ps="$ps" '/^Pages (free|inactive|speculative):/ { gsub(/\./, "", $NF); p += $NF } END { printf "%d\n", p * ps / 1048576 }'
  else
    awk '/^MemAvailable:/ { printf "%d\n", $2 / 1024 }' /proc/meminfo
  fi
}
load1() {
  [ -n "${AUTOPILOT_SUP_LOAD:-}" ] && { echo "$AUTOPILOT_SUP_LOAD"; return; }
  uptime | sed -E 's/.*load averages?: *//' | awk '{ gsub(/,/, "", $1); print $1 }'
}
cores() {
  [ -n "${AUTOPILOT_SUP_CORES:-}" ] && { echo "$AUTOPILOT_SUP_CORES"; return; }
  nproc 2>/dev/null || sysctl -n hw.ncpu
}

gate_cmd() {
  local cap="" running="" minmem=4096 maxload=""
  while [ $# -gt 0 ]; do
    case "$1" in --cap|--running|--min-mem-mib|--max-load) [ $# -ge 2 ] || die "$1 needs a value" ;; esac
    case "$1" in
      --cap) cap="$2"; shift 2 ;; --running) running="$2"; shift 2 ;;
      --min-mem-mib) minmem="$2"; shift 2 ;; --max-load) maxload="$2"; shift 2 ;;
      *) die "unknown gate option $1" ;;
    esac
  done
  if [ -z "$cap" ] || [ -z "$running" ]; then
    if [ -f "$LEDGER" ]; then
      need_ledger
      [ -n "$cap" ] || cap="$(hdr Cap)"; [ -n "$running" ] || running="$(occupied)"
    fi
  fi
  [ -n "$cap" ] || die "gate needs --cap or a ledger with a Cap"
  [ -n "$running" ] || running=0
  isnum() { case "$1" in ''|*[!0-9]*) return 1 ;; esac; }
  isnum "$cap" && isnum "$running" || { echo "FAIL cap: unreadable cap or running ($cap, $running)"; return 1; }
  if [ "$running" -ge "$cap" ]; then echo "FAIL cap: $running running, cap $cap"; return 1; fi
  local m l; m="$(mem_mib)"; l="$(load1)"; [ -n "$maxload" ] || maxload="$(cores)"
  isnum "$minmem" && isnum "$m" || { echo "FAIL memory: unreadable ('$m', min '$minmem')"; return 1; }
  if [ "$m" -lt "$minmem" ]; then echo "FAIL memory: $m MiB free, need $minmem"; return 1; fi
  isfloat() { case "$1" in ''|*[!0-9.]*|*.*.*|.) return 1 ;; esac; }
  isfloat "$l" && isfloat "$maxload" || { echo "FAIL load: unreadable ('$l', max '$maxload')"; return 1; }
  if awk -v l="$l" -v m="$maxload" 'BEGIN { exit !(l > m) }'; then echo "FAIL load: load $l above $maxload"; return 1; fi
  echo PASS
}

# map <json|-> --started E [--now E] [--grace S] | map --exit <2|3> ...: the ledger state (FR-011).
map_cmd() {
  local src="" started="" now="" grace="" ex=""
  while [ $# -gt 0 ]; do
    case "$1" in --started|--now|--grace|--exit) [ $# -ge 2 ] || die "$1 needs a value" ;; esac
    case "$1" in
      --started) started="$2"; shift 2 ;; --now) now="$2"; shift 2 ;;
      --grace) grace="$2"; shift 2 ;; --exit) ex="$2"; shift 2 ;;
      -) src="-"; shift ;; -*) die "unknown map option $1" ;; *) src="$1"; shift ;;
    esac
  done
  case "$started" in ''|*[!0-9]*) die "map needs --started <epoch seconds>" ;; esac
  if [ -n "$ex" ]; then
    case "$ex" in 2|3) echo lost; return 0 ;; *) die "map --exit takes 2 or 3" ;; esac
  fi
  [ -n "$src" ] || die "map needs a run-status JSON file or -"
  local json
  if [ "$src" = - ]; then json="$(cat)"; else [ -f "$src" ] || die "no json file $src"; json="$(cat "$src")"; fi
  local state; state="$(printf '%s' "$json" | jq -r '.state // empty' 2>/dev/null)" || true
  [ -n "$state" ] || die "unreadable json: no state"
  [ -n "$now" ] || now="$(date +%s)"
  if [ -z "$grace" ]; then
    if [ -f "$LEDGER" ]; then need_ledger; grace="$(hdr Grace)"; fi
    [ -n "$grace" ] || grace=600
  fi
  case "$now$grace" in *[!0-9]*) die "--now and --grace are whole seconds" ;; esac
  case "$state" in
    done) echo done ;;
    dead-no-ledger) if [ $((now - started)) -le "$grace" ]; then echo running; else echo lost; fi ;;
    asking|stuck|blocked|waiting-ci|running) echo running ;;
    *) die "unknown run-status state $state" ;;
  esac
}

# retry <issue>: a failed start. Tries < 3 -> back to queued; 3 or more -> failed. Prints the state.
retry_cmd() {
  need_ledger; [ $# -eq 1 ] || die "usage: retry <issue>"
  local t; t="$(ledger_cmd get "$1" Tries)" || exit 2
  case "$t" in ''|*[!0-9]*) t=0 ;; esac
  local s=queued; [ "$t" -ge 3 ] && s=failed
  ledger_cmd set "$1" State "$s"; echo "$s"
}

# check-owner --live-sessions "ids" --me <id>: OWN, FREE, or HELD <owner> (exit 1).
owner_cmd() {
  local live="" me="" have_me=0
  while [ $# -gt 0 ]; do
    case "$1" in --live-sessions|--me) [ $# -ge 2 ] || die "$1 needs a value" ;; esac
    case "$1" in
      --live-sessions) live="$2"; shift 2 ;; --me) me="$2"; have_me=1; shift 2 ;;
      *) die "unknown check-owner option $1" ;;
    esac
  done
  [ "$have_me" = 1 ] && [ -n "$me" ] || die "check-owner needs --me <id>"
  need_ledger
  local owner; owner="$(hdr Owner)"
  [ "$owner" = "$me" ] && { echo OWN; return 0; }
  case " $live " in
    *" $owner "*) [ -n "$owner" ] && { echo "HELD $owner"; return 1; } ;;
  esac
  echo FREE
}

case "$cmd" in
  map) map_cmd "$@" ;;
  retry) retry_cmd "$@" ;;
  check-owner) owner_cmd "$@" ;;
  ledger) ledger_cmd "$@" ;;
  next) next_cmd "$@" ;;
  gate) gate_cmd "$@" ;;
  *) die "unknown subcommand $cmd" ;;
esac
