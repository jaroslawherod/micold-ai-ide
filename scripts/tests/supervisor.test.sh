#!/usr/bin/env bash
# Drives scripts/autopilot/supervisor.sh against fixture ledgers and readings: ledger, next, gate,
# and grep cases over the skill text. No gh, MCP or run-status call (specs/1813-supervisor-skill).
set -uo pipefail
# shellcheck source=lib/autopilot-fixtures.sh
. "$(dirname "$0")/lib/autopilot-fixtures.sh"
SUP="$S/supervisor.sh"
SKILL="$ROOT/.claude/skills/autopilot-supervisor/SKILL.md"
AUTO="$ROOT/.claude/skills/speckit-autopilot/SKILL.md"
SUPMD="$ROOT/.claude/skills/speckit-autopilot/rules/supervised.md"
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
check "gate: a non-numeric --running fails" 1 '^FAIL' gt --running abc
check "gate: a non-numeric --cap fails" 1 '^FAIL' gt --cap abc
check "gate: a non-numeric memory reading fails" 1 '^FAIL memory' env AUTOPILOT_SUP_MEM_MIB=x "$SUP" gate --ledger "$L"
check "gate: a non-numeric load fails" 1 '^FAIL load' env AUTOPILOT_SUP_LOAD=abc "$SUP" gate --ledger "$L"
check "gate: an option without a value is a usage error, not a hang" 2 'needs a value' timeout 5 "$SUP" gate --cap
check "init: an option without a value is a usage error" 2 'needs a value' timeout 5 "$SUP" ledger init --ledger "$D/x.md" --cap
unset AUTOPILOT_SUP_MEM_MIB AUTOPILOT_SUP_LOAD AUTOPILOT_SUP_CORES
D2="$(mktemp -d "$tmp/c.XXXXXX")"
check "init: a bad issue number leaves no ledger" 2 'issue numbers' "$SUP" ledger init --ledger "$D2/s.md" --owner o --cap 1 --source x 12 ab
check "init: no partial file after the failure" 2 'No such file' ls "$D2/s.md"
check "init: duplicate issue numbers are rejected" 2 'duplicate' "$SUP" ledger init --ledger "$D2/s.md" --owner o --cap 1 --source x 12 12
check "init: works after a rejected attempt" 0 '^$' "$SUP" ledger init --ledger "$D2/s.md" --owner o --cap 1 --source x 12

# --- T014: map, retry, check-owner -------------------------------------------------------------
js() { printf '{"state":"%s","underlying_state":"%s","issue":"#1","escalation":null}\n' "$1" "$1" > "$D/j.json"; }
mp() { js "$1"; shift; "$SUP" map "$D/j.json" "$@"; }
mk 3 12
check "map: done is done at any time" 0 '^done$' mp done --started 1000 --now 1001
check "map: dead-no-ledger within Grace is running" 0 '^running$' mp dead-no-ledger --started 1000 --now 1600 --grace 600
check "map: dead-no-ledger past Grace is lost" 0 '^lost$' mp dead-no-ledger --started 1000 --now 1601 --grace 600
check "map: Grace comes from the ledger header" 0 '^lost$' mp dead-no-ledger --started 1000 --now 1601 --ledger "$L"
check "map: Grace of the ledger is honoured within" 0 '^running$' mp dead-no-ledger --started 1000 --now 1500 --ledger "$L"
for s in asking stuck blocked waiting-ci running; do
  check "map: $s keeps the issue running" 0 '^running$' mp "$s" --started 1000 --now 99999 --grace 600
done
check "map: reads the JSON from stdin" 0 '^done$' bash -c 'echo "{\"state\":\"done\"}" | "$0" map - --started 1 --now 2' "$SUP"
check "map: --exit 2 is lost" 0 '^lost$' "$SUP" map --exit 2 --started 1000
check "map: --exit 3 is lost" 0 '^lost$' "$SUP" map --exit 3 --started 1000
check "map: an unknown state is a usage error" 2 'state' mp bogus --started 1 --now 2
check "map: --started is required" 2 'started' mp done
check "map: an unreadable file is a usage error" 2 'json' "$SUP" map "$D/none.json" --started 1
check "retry: first failed start goes back to queued" 0 '^queued$' bash -c '"$0" ledger set --ledger "$1" 12 Tries 1 && "$0" retry --ledger "$1" 12 && "$0" ledger get --ledger "$1" 12 State' "$SUP" "$L"
check "retry: 3 tries is failed" 0 '^failed$' bash -c '"$0" ledger set --ledger "$1" 12 Tries 3 && "$0" retry --ledger "$1" 12' "$SUP" "$L"
check "retry: failed is written to the ledger" 0 '^failed$' "$SUP" ledger get --ledger "$L" 12 State
check "retry: an unknown issue is rejected" 2 'no row' "$SUP" retry --ledger "$L" 99
mk 3 12 14; st 12 skipped
check "skipped via ledger set releases the slot" 0 '^14$' bash -c '"$0" ledger set --ledger "$1" 12 Note closed && "$0" next --ledger "$1"' "$SUP" "$L"
mk 3 12
check "check-owner: the owner itself is OWN" 0 '^OWN$' "$SUP" check-owner --ledger "$L" --live-sessions "s-0 s-1" --me s-0
check "check-owner: an owner not listed is FREE" 0 '^FREE$' "$SUP" check-owner --ledger "$L" --live-sessions "s-1" --me s-5
check "check-owner: no live sessions is FREE" 0 '^FREE$' "$SUP" check-owner --ledger "$L" --live-sessions "" --me s-5
check "check-owner: a live other owner is HELD" 1 '^HELD s-0$' "$SUP" check-owner --ledger "$L" --live-sessions "s-0 s-1" --me s-5
check "check-owner: an empty Owner is FREE" 0 '^FREE$' bash -c '"$0" ledger owner --ledger "$1" "" ; "$0" check-owner --ledger "$1" --live-sessions "s-1" --me s-5' "$SUP" "$L"
check "check-owner: needs --me" 2 'me' "$SUP" check-owner --ledger "$L" --live-sessions "s-1"
check "check-owner: FREE then ledger owner sets the owner" 0 'Owner\*\*: s-5' bash -c '"$0" check-owner --ledger "$1" --live-sessions "s-1" --me s-5 >/dev/null && "$0" ledger owner --ledger "$1" s-5 && cat "$1"' "$SUP" "$L"

# --- T016: watcher -----------------------------------------------------------------------------
WATCH="$S/supervisor-watch.sh"
mk 3 12 14; st 12 running; st 14 running
"$SUP" ledger set --ledger "$L" 12 Worktree "$D/w12"; "$SUP" ledger set --ledger "$L" 14 Worktree "$D/w14"
cat > "$D/rs.sh" <<'STUB'
#!/usr/bin/env bash
# stub run-status: --json <dir>; the state is the content of <dir>.state, "running" when absent
d="${@: -1}"; echo x >> "$(dirname "$d")/calls"; s="$(cat "$d.state" 2>/dev/null || echo running)"
printf '{"state":"%s"}\n' "$s"
STUB
chmod +x "$D/rs.sh"
export AUTOPILOT_RUN_STATUS="$D/rs.sh"
check "watch: no change exits CEILING" 0 '^CEILING$' "$WATCH" --ledger "$L" --interval 1 --ceiling 2
# chg k=v...: flips states once the stub has been called for both rows (the watcher's baseline is read)
chg() { : > "$D/calls"; ( while [ "$(cat "$D/calls" 2>/dev/null | wc -l)" -lt 2 ]; do sleep 0.1; done
  for kv in "$@"; do echo "${kv#*=}" > "$D/w${kv%%=*}.state"; done ) & }
chg 14=asking
check "watch: a changed state exits CHANGED" 0 '^CHANGED 14 running asking$' "$WATCH" --ledger "$L" --interval 1 --ceiling 5
echo running > "$D/w14.state"; chg 12=done
check "watch: reports the first changed issue" 0 '^CHANGED 12 running done$' "$WATCH" --ledger "$L" --interval 1 --ceiling 5
echo running > "$D/w12.state"
check "watch: AUTOPILOT_WATCH_CEILING sets the ceiling" 0 '^CEILING$' env AUTOPILOT_WATCH_CEILING=1 "$WATCH" --ledger "$L" --interval 1 --ceiling 1
mk 3 12; st 12 queued
check "watch: only starting/running rows are watched" 0 '^CEILING$' "$WATCH" --ledger "$L" --interval 1 --ceiling 1
check "watch: a zero interval is a usage error" 2 'interval' "$WATCH" --ledger "$L" --interval 0 --ceiling 1
check "watch: an empty interval is a usage error" 2 'interval' "$WATCH" --ledger "$L" --interval "" --ceiling 1
check "watch: a missing ledger is a usage error" 2 'no ledger' "$WATCH" --ledger "$D/none.md" --interval 1 --ceiling 1
unset AUTOPILOT_RUN_STATUS

# --- T007: skill text (supervised marker) ------------------------------------------------------
check "speckit-autopilot documents the supervised argument" 0 'supervised' grep -n -i 'trailing `supervised`' "$AUTO"
check "speckit-autopilot names the literal ledger line" 0 'Supervised\*\*: yes' grep -nF -- "- **Supervised**: yes" "$SUPMD"
check "the literal line is the one supervised.sh matches" 0 'Supervised\*\*: yes' grep -nF -- '- **Supervised**: yes' "$S/supervised.sh"
check "supervisor skill names the supervised prompt" 0 'speckit-autopilot #<n> supervised' grep -nF -- '/speckit-autopilot #<n> supervised' "$SKILL"
check "supervisor skill never reads a transcript" 1 '^$' grep -n 'read_session_output' "$SKILL"
check "supervisor skill runs the gate before create_worktree" 0 'supervisor.sh gate' bash -c 'g=$(grep -n "supervisor.sh gate" "$0" | head -1 | cut -d: -f1); w=$(grep -n "create_worktree. with" "$0" | head -1 | cut -d: -f1); [ "$g" -lt "$w" ] && grep -n "supervisor.sh gate" "$0"' "$SKILL"

# --- T018: skill text (restart, report, watcher) -----------------------------------------------
check "skill has a Restart section" 0 '## Restart' grep -n '^## Restart' "$SKILL"
check "skill checks the owner at start" 0 'check-owner' grep -n 'supervisor.sh check-owner' "$SKILL"
check "skill sets the owner after FREE" 0 'ledger owner' grep -n 'ledger owner <' "$SKILL"
check "skill uses list_sessions for liveness" 0 'list_sessions' grep -n 'list_sessions' "$SKILL"
check "skill maps run-status JSON with map" 0 'supervisor.sh map' grep -n 'supervisor.sh map' "$SKILL"
check "skill reads run-status --json" 0 'run-status.sh --json' grep -n 'run-status.sh --json' "$SKILL"
check "skill recovers a starting row with empty cells" 0 'empty' grep -n -i 'starting.*empty\|empty.*starting' "$SKILL"
check "skill counts tries with retry" 0 'supervisor.sh retry' grep -n 'supervisor.sh retry' "$SKILL"
check "skill reports asking, stuck, blocked and lost runs" 0 'dead-no-ledger' grep -n 'asking.*stuck.*blocked.*dead-no-ledger' "$SKILL"
check "skill does not answer or restart a reported run" 0 'Never answer' grep -n -i 'never answer' "$SKILL"
check "skill starts the watcher in the background" 0 'run_in_background' grep -n 'run_in_background' "$SKILL"
check "skill names the watcher script" 0 'supervisor-watch.sh' grep -n 'supervisor-watch.sh' "$SKILL"

# --- T019: touches, overlap, next holds overlapping issues ------------------------------------
B="$tmp/body.md"
printf 'Intro\nTouches: crates/a, docs/x.md\nspecs/9-x/spec.md\n' > "$B"
check "touches: a declared line wins" 0 '^crates/a,docs/x.md$' bash -c '"$0" touches "$1" 2>/dev/null' "$SUP" "$B"
check "touches: declared is named on stderr" 0 '^declared$' bash -c '"$0" touches "$1" 2>&1 >/dev/null' "$SUP" "$B"
printf 'Touches: crates/a docs/x.md\n' > "$B"
check "touches: space separated paths" 0 '^crates/a,docs/x.md$' bash -c '"$0" touches "$1" 2>/dev/null' "$SUP" "$B"
printf 'see specs/1813-supervisor-skill/spec.md and\nspecs/1813-supervisor-skill/tasks.md\n' > "$B"
check "touches: spec paths are inferred" 0 '^specs/1813-supervisor-skill$' bash -c '"$0" touches "$1" 2>/dev/null' "$SUP" "$B"
check "touches: inference is named on stderr" 0 '^inferred$' bash -c '"$0" touches "$1" 2>&1 >/dev/null' "$SUP" "$B"
printf 'nothing to see\n' > "$B"
check "touches: a flow label is inferred" 0 'flows/bug.md' bash -c '"$0" touches "$1" --labels "enhancement flow:bug" 2>/dev/null' "$SUP" "$B"
check "touches: nothing inferable prints -" 0 '^-$' bash -c '"$0" touches "$1" 2>/dev/null' "$SUP" "$B"
check "touches: nothing inferable says none" 0 '^none$' bash -c '"$0" touches "$1" 2>&1 >/dev/null' "$SUP" "$B"
check "touches: a missing body file is a usage error" 2 'body' "$SUP" touches "$tmp/none.md"
check "overlap: equal paths" 0 '^yes$' "$SUP" overlap a/b.rs a/b.rs
check "overlap: a directory contains a file" 0 '^yes$' "$SUP" overlap crates/a crates/a/src/x.rs
check "overlap: containment works both ways" 0 '^yes$' "$SUP" overlap crates/a/src/x.rs crates/a
check "overlap: any pair of the lists" 0 '^yes$' "$SUP" overlap docs/x.md,crates/a crates/a/y.rs
check "overlap: a name prefix is not containment" 0 '^no$' "$SUP" overlap crates/a crates/ab
check "overlap: disjoint paths" 0 '^no$' "$SUP" overlap crates/a docs
check "overlap: a trailing glob is its directory" 0 '^yes$' "$SUP" overlap 'crates/a/**' crates/a/x.rs
check "overlap: none overlaps nothing" 0 '^no$' "$SUP" overlap - crates/a
check "overlap: none against none" 0 '^no$' "$SUP" overlap - -
check "overlap: missing argument is a usage error" 2 'usage' "$SUP" overlap a
tset() { "$SUP" ledger set --ledger "$L" "$1" Touches "$2" >/dev/null; }
mk 3 12 14 15; tset 12 crates/a; tset 14 crates/a/src; tset 15 docs
check "next: starts the earlier, holds the later overlapping one" 0 '^12 15$' nx
st 12 running
check "next: holds while the earlier one is running" 0 '^15$' nx
st 12 starting
check "next: holds while the earlier one is starting" 0 '^15$' nx
for s in done failed lost skipped; do
  st 12 "$s"
  check "next: $s releases the held issue" 0 '^14 15$' nx
done
mk 3 12 14 15; st 12 running
check "next: an empty Touches cell is disjoint" 0 '^14 15$' nx
tset 14 -
check "next: a - Touches cell is disjoint" 0 '^14 15$' nx
mk 3 12 14 15; tset 12 shared; tset 14 shared; tset 15 shared
check "next: three overlapping issues start one" 0 '^12$' nx
mk 3 12 14 15; tset 12 a; tset 14 b; tset 15 a,b
check "next: a later issue overlapping two earlier ones is held" 0 '^12 14$' nx

# --- T021: skill text (touches step) -----------------------------------------------------------
check "skill has a Touches section" 0 '## Touches' grep -n '^## Touches' "$SKILL"
check "skill reads the issue body with gh issue view" 0 'gh issue view' grep -n 'gh issue view <n> --json body' "$SKILL"
check "skill runs touches" 0 'supervisor.sh touches' grep -n 'supervisor.sh touches' "$SKILL"
check "skill stores touches in the ledger" 0 'ledger set <n> Touches' grep -n 'ledger set <n> Touches' "$SKILL"
check "skill notes a disjoint treatment for none" 0 'disjoint' grep -n -i 'treated as disjoint' "$SKILL"

echo "supervisor: $cases case(s), $failures failure(s)"
[ "$failures" -eq 0 ]
