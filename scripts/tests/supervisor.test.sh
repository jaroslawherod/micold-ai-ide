#!/usr/bin/env bash
# Drives scripts/autopilot/supervisor.sh against fixture ledgers and readings: ledger, next, gate,
# and grep cases over the skill text. No gh, MCP or run-status call (specs/1813-supervisor-skill).
set -uo pipefail
# shellcheck source=lib/autopilot-fixtures.sh
. "$(dirname "$0")/lib/autopilot-fixtures.sh"
SUP="$S/supervisor.sh"
SKILL="$ROOT/.claude/skills/autopilot-supervisor/SKILL.md"
AUTO="$ROOT/.claude/skills/speckit-autopilot/SKILL.md"
cd "$tmp" || exit 1

# --- helpers -----------------------------------------------------------------------------------
# mk <cap> <issues...>: a fresh ledger L in a per-case directory.
mk() { local cap="$1"; shift; D="$(mktemp -d "$tmp/c.XXXXXX")"; L="$D/supervisor.md"
  "$SUP" ledger init --ledger "$L" --owner s-0 --cap "$cap" --source "issues" "$@" >/dev/null; }
st() { "$SUP" ledger set --ledger "$L" "$1" State "$2"; }
nx() { "$SUP" next --ledger "$L" | tr '\n' ' ' | sed 's/ $//'; }
gt() { "$SUP" gate --ledger "$L" "$@"; }

# --- T002: the ledger directory is ignored -----------------------------------------------------
check "/.autopilot/ is gitignored" 0 'supervisor.md' git -C "$ROOT" check-ignore .autopilot/supervisor.md

# --- T003: ledger ------------------------------------------------------------------------------
mk 3 12 14 15
check "init writes the header" 0 '^- \*\*Cap\*\*: 3$' cat "$L"
check "init writes one queued row per issue" 0 '^3$' bash -c 'grep -c "^| 1[245] | queued |" "$0"' "$L"
check "init refuses an existing file" 2 'exists' "$SUP" ledger init --ledger "$L" --owner s-1 --cap 1 --source x 1
check "owner sets the header" 0 'Owner\*\*: s-9' bash -c '"$0" ledger owner --ledger "$1" s-9 && cat "$1"' "$SUP" "$L"
check "set then get a cell" 0 '^s-1$' bash -c '"$0" ledger set --ledger "$1" 12 Session s-1 && "$0" ledger get --ledger "$1" 12 Session' "$SUP" "$L"
check "set touches only that row" 0 '^queued$' "$SUP" ledger get --ledger "$L" 14 State
for f in Touches Worktree Started Tries Note; do
  check "set/get $f" 0 '^v1$' bash -c '"$0" ledger set --ledger "$1" 14 "$2" v1 && "$0" ledger get --ledger "$1" 14 "$2"' "$SUP" "$L" "$f"
done
check "an unknown State is rejected" 2 'state' "$SUP" ledger set --ledger "$L" 12 State bogus
for s in queued starting running done failed lost skipped; do
  check "State $s is accepted" 0 '^$' "$SUP" ledger set --ledger "$L" 12 State "$s"
done
check "a cell with a pipe is rejected" 2 'pipe' "$SUP" ledger set --ledger "$L" 12 Note 'a|b'
check "an unknown issue is rejected" 2 'no row' "$SUP" ledger set --ledger "$L" 99 Note x
check "an unknown field is rejected" 2 'field' "$SUP" ledger get --ledger "$L" 12 Nope
printf 'not a ledger\n' > "$D/bad.md"
check "a malformed ledger exits 2" 2 'malformed' "$SUP" ledger get --ledger "$D/bad.md" 12 State
check "a missing ledger exits 2" 2 'no ledger' "$SUP" ledger table --ledger "$D/none.md"
check "table is short: header plus a line per issue" 0 '^4$' bash -c '"$0" ledger table --ledger "$1" | wc -l | tr -d " "' "$SUP" "$L"

# --- T005: next --------------------------------------------------------------------------------
mk 2 12 14 15
check "three queued, cap 2: the two lowest" 0 '^12 14$' bash -c 'echo "$("$0" next --ledger "$1" | tr "\n" " " | sed "s/ $//")"' "$SUP" "$L"
mk 2 15 12 14
check "ascending whatever the init order" 0 '^12 14$' bash -c '"$0" next --ledger "$1" | tr "\n" " " | sed "s/ $//"' "$SUP" "$L"
mk 2 12 14 15; st 12 starting
check "a starting row occupies a slot" 0 '^14$' bash -c '"$0" next --ledger "$1"' "$SUP" "$L"
st 12 running
check "a running row occupies a slot" 0 '^14$' bash -c '"$0" next --ledger "$1"' "$SUP" "$L"
st 14 running
check "cap reached: nothing printed" 0 '^$' bash -c '"$0" next --ledger "$1"' "$SUP" "$L"
for s in done failed lost skipped; do
  st 12 "$s"
  check "$s releases the slot" 0 '^15$' bash -c '"$0" next --ledger "$1"' "$SUP" "$L"
done
mk 3 12; st 12 done
check "nothing queued: nothing printed, exit 0" 0 '^$' bash -c '"$0" next --ledger "$1"' "$SUP" "$L"

# --- T011: gate --------------------------------------------------------------------------------
mk 2 12 14
export AUTOPILOT_SUP_MEM_MIB=8000 AUTOPILOT_SUP_LOAD=1.5 AUTOPILOT_SUP_CORES=8
check "gate PASS" 0 '^PASS$' gt
check "gate FAIL cap" 1 '^FAIL cap' gt --running 2
check "gate cap from --cap" 1 '^FAIL cap' gt --cap 1 --running 1
check "gate counts occupied rows by default" 1 '^FAIL cap' bash -c '"$0" ledger set --ledger "$1" 12 State running && "$0" ledger set --ledger "$1" 14 State starting && "$0" gate --ledger "$1"' "$SUP" "$L"
mk 2 12
check "memory below 4096 MiB fails" 1 '^FAIL memory' env AUTOPILOT_SUP_MEM_MIB=4095 "$SUP" gate --ledger "$L"
check "memory at 4096 MiB passes" 0 '^PASS$' env AUTOPILOT_SUP_MEM_MIB=4096 "$SUP" gate --ledger "$L"
check "--min-mem-mib overrides" 0 '^PASS$' env AUTOPILOT_SUP_MEM_MIB=100 "$SUP" gate --ledger "$L" --min-mem-mib 50
check "load above cores fails" 1 '^FAIL load' env AUTOPILOT_SUP_LOAD=8.5 "$SUP" gate --ledger "$L"
check "load equal to cores passes" 0 '^PASS$' env AUTOPILOT_SUP_LOAD=8.00 "$SUP" gate --ledger "$L"
check "--max-load overrides" 1 '^FAIL load' "$SUP" gate --ledger "$L" --max-load 1
check "failure order: cap first" 1 '^FAIL cap' env AUTOPILOT_SUP_MEM_MIB=1 AUTOPILOT_SUP_LOAD=99 "$SUP" gate --ledger "$L" --running 2
check "failure order: memory before load" 1 '^FAIL memory' env AUTOPILOT_SUP_MEM_MIB=1 AUTOPILOT_SUP_LOAD=99 "$SUP" gate --ledger "$L"
check "the machine is read without overrides" 0 '^(PASS|FAIL (memory|load))' env -u AUTOPILOT_SUP_MEM_MIB -u AUTOPILOT_SUP_LOAD -u AUTOPILOT_SUP_CORES "$SUP" gate --ledger "$L" --cap 9
check "gate without a cap is a usage error" 2 'cap' "$SUP" gate --ledger "$D/none.md"
unset AUTOPILOT_SUP_MEM_MIB AUTOPILOT_SUP_LOAD AUTOPILOT_SUP_CORES

# --- T007: skill text (supervised marker) ------------------------------------------------------
check "speckit-autopilot documents the supervised argument" 0 'supervised' grep -n -i 'trailing `supervised`' "$AUTO"
check "speckit-autopilot names the literal ledger line" 0 'Supervised\*\*: yes' grep -nF -- '- **Supervised**: yes' "$AUTO"
check "the literal line is the one supervised.sh matches" 0 'Supervised\*\*: yes' grep -nF -- '- **Supervised**: yes' "$S/supervised.sh"
check "supervisor skill names the supervised prompt" 0 'speckit-autopilot #<n> supervised' grep -nF -- '/speckit-autopilot #<n> supervised' "$SKILL"
check "supervisor skill never reads a transcript" 1 '^$' grep -n 'read_session_output' "$SKILL"
check "supervisor skill runs the gate before create_worktree" 0 'gate --running' grep -nF -- 'gate --running' "$SKILL"

echo "supervisor: $cases case(s), $failures failure(s)"
[ "$failures" -eq 0 ]
