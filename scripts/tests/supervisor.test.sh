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
check "the machine is read without overrides" 0 '^(PASS|FAIL memory: [0-9]+ MiB free|FAIL load: load [0-9])' bash -c 'env -u AUTOPILOT_SUP_MEM_MIB -u AUTOPILOT_SUP_LOAD -u AUTOPILOT_SUP_CORES "$0" gate --ledger "$1" --cap 9; exit 0' "$SUP" "$L"
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
check "skill reports stuck, blocked and lost runs" 0 'dead-no-ledger' grep -n 'stuck.*blocked.*dead-no-ledger' "$SKILL"
check "skill does not unblock or restart a reported run" 0 'Never unblock' grep -n 'Never unblock' "$SKILL"
check "skill starts the watcher in the background" 0 'run_in_background' grep -n 'run_in_background' "$SKILL"
check "skill names the watcher script" 0 'supervisor-watch.sh' grep -n 'supervisor-watch.sh' "$SKILL"

# --- T019: touches, overlap, next holds overlapping issues ------------------------------------
B="$tmp/body.md"
printf 'Intro\nTouches: crates/a, docs/x.md\nspecs/9-x/spec.md\n' > "$B"
check "touches: a declared line wins" 0 '^crates/a,docs/x.md$' bash -c '"$0" touches "$1" 2>/dev/null' "$SUP" "$B"
printf 'Touches:\nTouches: crates/z\n' > "$tmp/body-empty.md"
check "touches: an empty Touches line before a populated one is skipped" 0 '^crates/z$' bash -c '"$0" touches "$1" 2>/dev/null' "$SUP" "$tmp/body-empty.md"
check "touches: declared is named on stderr" 0 '^declared$' bash -c '"$0" touches "$1" 2>&1 >/dev/null' "$SUP" "$B"
printf 'Touches: crates/a docs/x.md\n' > "$B"
check "touches: space separated paths" 0 '^crates/a,docs/x.md$' bash -c '"$0" touches "$1" 2>/dev/null' "$SUP" "$B"
printf 'Touches: crates/a, docs/x.md\r\n' > "$B"
check "touches: a CRLF body loses the carriage return" 0 '^crates/a,docs/x.md$' bash -c '"$0" touches "$1" 2>/dev/null | od -c | grep -c "\\\\r" | grep -qx 0 && "$0" touches "$1" 2>/dev/null' "$SUP" "$B"
printf -- '- **Touches:** crates/a\n' > "$B"
check "touches: a bulleted bold key is recognised" 0 '^crates/a$' bash -c '"$0" touches "$1" 2>/dev/null' "$SUP" "$B"
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
check "overlap: a file glob overlaps a file in its directory" 0 '^yes$' "$SUP" overlap 'crates/a/src/*.rs' crates/a/src/lib.rs
check "overlap: a glob component overlaps by its parent" 0 '^yes$' "$SUP" overlap 'crates/*/Cargo.toml' crates/a/Cargo.toml
check "overlap: a glob stays out of other directories" 0 '^no$' "$SUP" overlap 'crates/a/src/*.rs' docs/x.md
check "overlap: a bare glob is the whole repo" 0 '^yes$' "$SUP" overlap '*' docs/x.md
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


# --- T022: small context (US5, FR-007, FR-010, SC-004) -----------------------------------------
check "context: run-status.sh is named exactly once" 0 '^1$' bash -c 'grep -o "run-status\.sh" "$0" | wc -l | tr -d " "' "$SKILL"
check "context: every line naming run-status.sh has --json" 0 '^0$' bash -c 'grep "run-status\.sh" "$0" | grep -vc -- "--json"; true' "$SKILL"
check "context: the one read is --json, per row of the mapping step" 0 'run-status.sh --json <dir>' bash -c 'awk "/^## Restart/,/^## Report/" "$0" | grep -n "run-status.sh --json <dir>"' "$SKILL"
check "context: a read is written to one file per issue" 0 '<scratch>/<n>.json' grep -n 'run-status.sh --json <dir> > <scratch>/<n>.json' "$SKILL"
check "context: the report reads that issue's file, not run-status again" 0 '<scratch>/<n>.json' bash -c 'awk "/^## Report/,/^## Watcher/" "$0" | grep -n "<scratch>/<n>.json"' "$SKILL"
check "context: the advance loop runs Report" 0 'Report' bash -c 'awk "/^## Advance loop/,/^## Restart/" "$0" | grep -n "\*Report\*"' "$SKILL"
check "context: the watcher wake runs Report" 0 'Report' bash -c 'awk "/^## Watcher/,0" "$0" | grep -n "\*Report\*"' "$SKILL"
check "context: no transcript or session output read is instructed" 1 '^$' grep -n -i 'read_session_output\|get_session_output\|session transcript.*read\|tail.*session' "$SKILL"
check "context: the skill says never to read a transcript" 0 'never read a session' grep -n -i 'never read a session' "$SKILL"
check "context: the ledger is read only through ledger table" 0 'only through `supervisor.sh ledger table`' grep -nF 'only through `supervisor.sh ledger table`' "$SKILL"
check "context: the advance loop reports through ledger table" 0 'ledger table' bash -c 'awk "/^## Advance loop/,/^## Restart/" "$0" | grep -n "ledger table"' "$SKILL"
check "context: the report ends with ledger table" 0 'Finish with `supervisor.sh ledger table`' bash -c 'awk "/^## Report/,/^## Watcher/" "$0" | grep -nF "Finish with \`supervisor.sh ledger table\`"' "$SKILL"
check "context: an asking run is reported from the JSON escalation" 0 'escalation' bash -c 'awk "/^## Report/,/^## Watcher/" "$0" | grep -n "JSON .escalation"' "$SKILL"
check "context: the report does not read the session" 0 'without reading the session' grep -n 'without reading the session' "$SKILL"
check "context: no section reads the ledger file directly" 1 '^$' grep -n -E '(cat|Read|head|tail|sed -n) .*supervisor\.md' "$SKILL"
mk 3 12 14 15 16 17
for occ in 0 1 2 3; do
  for i in 12 14 15; do st "$i" queued; done
  [ "$occ" -ge 1 ] && st 12 running; [ "$occ" -ge 2 ] && st 14 starting; [ "$occ" -ge 3 ] && st 15 running
  case "$occ" in 0) want=3 gw=PASS gx=0 ;; 1) want=2 gw=PASS gx=0 ;; 2) want=1 gw=PASS gx=0 ;; 3) want=0 gw=FAIL gx=1 ;; esac
  check "slots: $occ occupied, cap 3: next prints $want" 0 "^$want\$" bash -c 'AUTOPILOT_SUP_MEM_MIB=8000 "$0" next --ledger "$1" | wc -l | tr -d " "' "$SUP" "$L"
  check "slots: $occ occupied: gate agrees ($gw)" "$gx" "^$gw" env AUTOPILOT_SUP_MEM_MIB=8000 AUTOPILOT_SUP_LOAD=1 AUTOPILOT_SUP_CORES=8 "$SUP" gate --ledger "$L"
done

# --- #1815 M1: question routing (route add|next|state|answer|compose|sent|drop|table) ----------
# esc <file> <category> <n> <question> [<n> <question>...]: an escalation JSON with two options each.
esc() { local f="$1" cat="$2" qs="" sep=""; shift 2
  while [ $# -ge 2 ]; do
    qs="$qs$sep{\"number\":$1,\"question\":$(jq -Rn --arg q "$2" '$q'),\"options\":[{\"label\":\"A\",\"description\":\"a\"},{\"label\":\"B\",\"description\":\"b\"}],\"recommendation\":\"A\",\"evidence\":\"e\"}"
    sep=","; shift 2
  done
  printf '{"category":"%s","banner":"b","questions":[%s]}\n' "$cat" "$qs" > "$f"; }
# sessions <file> <id>[:<parent>]...: a session-list JSON with an optional parent field.
sessions() { local f="$1" out="" sep=""; shift; local x
  for x in "$@"; do out="$out$sep{\"id\":\"${x%%:*}\",\"parent\":\"$( [ "$x" != "${x#*:}" ] && echo "${x#*:}")\"}"; sep=","; done
  printf '[%s]\n' "$out" > "$f"; }
rt() { "$SUP" route "$@" --ledger "$L"; }
mk 3 12 14
E12="$D/e12.json"; E14="$D/e14.json"; E5="$D/e5.json"
esc "$E12" "1 Product" 1 "Q for 12?"; esc "$E14" "1 Product" 1 "Q for 14?"
cp "$L" "$D/before.md"
check "route table on a ledger without Routing prints nothing" 0 '^$' rt table
check "route: ledger table unchanged without Routing" 0 '^cap 3 occupied 0$' bash -c '"$0" ledger table --ledger "$1" | head -1' "$SUP" "$L"
check "route: add gains an empty Routing section" 0 '^R12.Q1$' rt add 12 s-12 "$E12"
check "route: the Routing section exists" 0 '^## Routing$' cat "$L"
check "route: ledger set still works with a Routing section" 0 '^running$' bash -c '"$0" ledger set --ledger "$1" 12 State running && "$0" ledger get --ledger "$1" 12 State' "$SUP" "$L"
check "route: ledger table still works with a Routing section" 0 '^#12 running$' bash -c '"$0" ledger table --ledger "$1" | sed -n 2p' "$SUP" "$L"
check "route: next works with a Routing section" 0 '^14$' bash -c '"$0" next --ledger "$1"' "$SUP" "$L"
check "route: a usage error exits 2" 2 'usage' rt bogus
check "route: add without arguments exits 2" 2 'usage' rt add 12
printf 'not a ledger\n' > "$D/bad.md"
check "route: a malformed ledger exits 2" 2 'malformed' "$SUP" route table --ledger "$D/bad.md"
check "route: a missing escalation file exits 2" 2 'no escalation' rt add 14 s-14 "$D/none.json"
# add / next
check "route add: a pending row" 0 '^pending$' bash -c '"$0" route table --ledger "$1" | awk -F"|" "/R12.Q1/ { gsub(/ /,\"\",\$3); print \$3 }"' "$SUP" "$L"
check "route add: re-add is a no-op" 0 '^1$' bash -c '"$0" route add --ledger "$1" 12 s-12 "$2" >/dev/null; grep -c "^| R12.Q1 |" "$1"' "$SUP" "$L" "$E12"
rt add 14 s-14 "$E14" >/dev/null
check "route next: carries question, options, rec, evidence" 0 '^R12.Q1 \| Q for 12\? \| A: a // B: b \| A \| e$' bash -c '"$0" route next --ledger "$1" | head -1' "$SUP" "$L"
esc "$E5" "1 Product" 1 "q1" 2 "q2" 3 "q3" 4 "q4" 5 "q5"
rt add 15 s-15 "$E5" >/dev/null
check "route next: at most 4, then the rest" 0 '^4$' bash -c '"$0" route next --ledger "$1" | wc -l | tr -d " "' "$SUP" "$L"
check "route next: the first four are issue 12 14 then 15's first two" 0 '^R12.Q1 R14.Q1 R15.Q1 R15.Q2$' bash -c '"$0" route next --ledger "$1" | cut -d" " -f1 | paste -sd" " -' "$SUP" "$L"
for k in R12.Q1 R14.Q1 R15.Q1 R15.Q2; do rt state "$k" asked; done
check "route next: five questions give four then one" 0 '^R15.Q3 R15.Q4 R15.Q5$' bash -c '"$0" route next --ledger "$1" | cut -d" " -f1 | paste -sd" " -' "$SUP" "$L"
for k in R15.Q3 R15.Q4 R15.Q5; do rt state "$k" asked; done
check "route next: empty when none pending" 0 '^$' rt next
printf '{"category":"1","banner":"free text ask","questions":[]}\n' > "$D/free.json"
check "route add: a free-text escalation gives Q1" 0 '^R16.Q1$' "$SUP" route add --ledger "$L" 16 s-16 "$D/free.json"
esc "$D/pipe.json" "1" 1 "a|b?"
check "route add: a pipe in a question is stored as a slash" 0 'a/b\?' bash -c '"$0" route add --ledger "$1" 17 s-17 "$2" >/dev/null; "$0" route next --ledger "$1"' "$SUP" "$L" "$D/pipe.json"
# state / answer
check "route state: unknown state exits 2" 2 'unknown state' rt state R12.Q1 bogus
check "route state: unknown key exits 2" 2 'no routing row' rt state R99.Q1 asked
check "route answer: via must be known" 2 'via' rt answer R12.Q1 nobody x
check "route answer: sets answered" 0 '^answered$' bash -c '"$0" route answer --ledger "$1" R12.Q1 user "Option A" && "$0" route table --ledger "$1" | awk -F"|" "/R12.Q1/ { gsub(/ /,\"\",\$3); print \$3 }"' "$SUP" "$L"
check "route answer: a pipe keeps the cells" 0 '^answered\|s-17\|user$' bash -c '"$0" route answer --ledger "$1" R17.Q1 user "x|y" && "$0" route table --ledger "$1" | awk -F"|" "/R17.Q1/ { gsub(/ /,\"\",\$3); gsub(/ /,\"\",\$4); gsub(/ /,\"\",\$5); print \$3 \"|\" \$4 \"|\" \$5 }"' "$SUP" "$L"
# compose
check "route compose: a free-text answer is bare, session first" 0 '^session s-16 resume Fix it$' bash -c '"$0" route answer --ledger "$1" R16.Q1 user "Fix it" && "$0" route compose --ledger "$1" 16 | paste -sd" " -' "$SUP" "$L"
check "route compose: exit 1 with missing while a question has no answer" 1 '^missing: 3 4 5$' bash -c '"$0" route answer --ledger "$1" R15.Q1 user a1; "$0" route answer --ledger "$1" R15.Q2 user a2; "$0" route compose --ledger "$1" 15' "$SUP" "$L"
for n in 3 4 5; do rt answer R15.Q$n user "a$n"; done
check "route compose: one line per question" 0 '^session s-15 resume 1. a1 2. a2 3. a3 4. a4 5. a5$' bash -c '"$0" route compose --ledger "$1" 15 | paste -sd" " -' "$SUP" "$L"
check "route compose: only that issue's session and answer" 0 '^session s-12 resume Option A$' bash -c '"$0" route compose --ledger "$1" 12 | paste -sd" " -' "$SUP" "$L"
check "route compose: a supervisor-resolved issue leads with --by supervisor" 0 '^session s-14 resume --by supervisor ok$' bash -c '"$0" route answer --ledger "$1" R14.Q1 supervisor ok && "$0" route compose --ledger "$1" 14 | paste -sd" " -' "$SUP" "$L"
check "route compose: a single question with options is bare too" 0 '^session s-12 resume Option A$' bash -c '"$0" route compose --ledger "$1" 12 | paste -sd" " -' "$SUP" "$L"
esc "$D/nl.json" "1" 1 "Which?
second line"
check "route add: a newline in a question keeps options and evidence" 0 'Which\? second line \| A: a // B: b \| A \| e' bash -c '"$0" route add --ledger "$1" 18 s-18 "$2" >/dev/null; "$0" route next --ledger "$1" | grep R18' "$SUP" "$L" "$D/nl.json"
check "route compose: an unknown issue exits 2" 2 'no routing rows' rt compose 99
# gate and the supervisor via (#1815 M2)
check "route gate: category 2 is user-only" 0 '^user-only$' rt gate "2 Irreversible or external action"
check "route gate: 3 and 4 are user-only" 0 '^user-only user-only$' bash -c '{ "$0" route gate --ledger "$1" 3; "$0" route gate --ledger "$1" "4 x"; } | paste -sd" " -' "$SUP" "$L"
check "route gate: 1, 5 and 6 are judge" 0 '^judge judge judge$' bash -c '{ for c in 1 "5 Non-convergence" 6; do "$0" route gate --ledger "$1" "$c"; done; } | paste -sd" " -' "$SUP" "$L"
check "route gate: an unknown category exits 2" 2 'category' rt gate 7
check "route gate: an empty category exits 2" 2 'category' rt gate ""
check "route gate: no argument exits 2" 2 'usage' rt gate
check "route answer supervisor: state resolved" 0 '^resolved$' bash -c '"$0" route add --ledger "$1" 21 s-21 "$2" >/dev/null; "$0" route add --ledger "$1" 22 s-22 "$2" >/dev/null; "$0" route answer --ledger "$1" R21.Q1 supervisor ok && "$0" route table --ledger "$1" | awk -F"|" "/R21.Q1/ { gsub(/ /,\"\",\$3); print \$3 }"' "$SUP" "$L" "$E12"
esc "$D/e23.json" "1 Product" 1 "Q1?" 2 "Q2?"
check "route compose: mixed supervisor and user rows get no --by" 0 '^session s-23 resume 1. a 2. b$' bash -c '"$0" route add --ledger "$1" 23 s-23 "$2" >/dev/null; "$0" route answer --ledger "$1" R23.Q1 supervisor a; "$0" route answer --ledger "$1" R23.Q2 user b; "$0" route compose --ledger "$1" 23 | paste -sd" " -' "$SUP" "$L" "$D/e23.json"
check "route compose: all-supervisor batch leads with --by supervisor once" 0 '^session s-23 resume --by supervisor 1. a 2. b$' bash -c '"$0" route answer --ledger "$1" R23.Q2 supervisor b; "$0" route compose --ledger "$1" 23 | paste -sd" " -' "$SUP" "$L"
# sent / drop
check "route sent: sets Sent on the issue's rows" 0 '^1$' bash -c '"$0" route sent --ledger "$1" 12 1700 && "$0" route table --ledger "$1" | grep -c "| 1700 |"' "$SUP" "$L"
check "route sent: a bad epoch exits 2" 2 'epoch' rt sent 12 soon
check "route drop: removes only that issue's rows, pipes intact" 0 '^0 1$' bash -c '"$0" route drop --ledger "$1" 15; echo "$(grep -c "^| R15" "$1") $(grep -c "^| R17.Q1 | answered | s-17 | user | a/b? " "$1")"' "$SUP" "$L"
check "route drop: a later add starts fresh" 0 '^pending$' bash -c '"$0" route add --ledger "$1" 15 s-15 "$2" >/dev/null; "$0" route table --ledger "$1" | awk -F"|" "/R15.Q1/ { gsub(/ /,\"\",\$3); print \$3 }"' "$SUP" "$L" "$E5"
check "route: the existing ledger still parses after all of it" 0 '^cap 3 occupied 1$' bash -c '"$0" ledger table --ledger "$1" | head -1' "$SUP" "$L"

# --- #1815 M1: skill text (Route questions, supervised notification) ---------------------------
RQ() { awk '/^## Route questions/,/^## Watcher/' "$SKILL"; }
ord() { local a="$1" b="$2" x y; x="$(RQ | grep -n -m1 -F -- "$a" | cut -d: -f1)"; y="$(RQ | grep -n -m1 -F -- "$b" | cut -d: -f1)"; [ -n "$x" ] && [ -n "$y" ] && [ "$x" -le "$y" ]; }
check "skill has a Route questions section" 0 'Route questions' grep -n '^## Route questions' "$SKILL"
check "route: collects each asking run's <n>.json" 0 '<scratch>/<n>.json' bash -c 'awk "/^## Route questions/,/^## Watcher/" "$0" | grep -n "<scratch>/<n>.json"' "$SKILL"
order_ok() { ord "route add" "tasks/ask.md" && ord "tasks/ask.md" "PushNotification" && ord "PushNotification" "route state <key> asked" &&
  ord "route state <key> asked" "AskUserQuestion" && ord "AskUserQuestion" "route answer <key> user" && ord "route answer <key> user" "route compose" &&
  ord "route compose" "send_session_input" && ord "send_session_input" "route sent" && ord "route sent" "route drop" && echo ok; }
check "route: steps in order add, banner, notification, asked, ask, answer, compose, sent, drop" 0 '^ok$' order_ok
check "route: the notification text" 0 'need a decision' grep -nF '🛑 <issues> need a decision: <topic>' "$SKILL"
check "route: one AskUserQuestion of at most 4" 0 'at most 4' bash -c 'awk "/^## Route questions/,/^## Watcher/" "$0" | grep -n "at most 4"' "$SKILL"
check "route: the banner carries #<issue>, unit supervisor and (Recommended)" 0 '^ok$' bash -c 'r="$(awk "/^## Route questions/,/^## Watcher/" "$0")"; for w in "#<issue>" "supervisor" "(Recommended)" "cold"; do printf "%s" "$r" | grep -qF -- "$w" || exit 1; done; echo ok' "$SKILL"
check "route: send only on compose exit 0, to the printed session" 0 'Exit 0' bash -c 'awk "/^## Route questions/,/^## Watcher/" "$0" | grep -n "Exit 0"' "$SKILL"
check "route: a compose exit 1 re-asks only the missing question" 0 'missing' bash -c 'awk "/^## Route questions/,/^## Watcher/" "$0" | grep -n -i "only the missing"' "$SKILL"
check "route: more than 4 go in the next batch" 0 'next batch' bash -c 'awk "/^## Route questions/,/^## Watcher/" "$0" | grep -n "next batch"' "$SKILL"
check "route: a failed send leaves rows answered and sets Note, never loops" 0 '^ok$' bash -c 'r="$(awk "/^## Route questions/,/^## Watcher/" "$0")"; for w in "ledger set <n> Note" "answered" "never retry"; do printf "%s" "$r" | grep -qF -- "$w" || exit 1; done; echo ok' "$SKILL"
check "route: not_found drops the rows and reports the session lost" 0 'not_found' bash -c 'awk "/^## Route questions/,/^## Watcher/" "$0" | grep -n "not_found.*route drop"' "$SKILL"
check "route: never names read_session_output" 1 '^$' grep -n 'read_session_output' "$SKILL"
check "route: Report routes an asking run, stuck blocked lost stay reported" 0 'Route' bash -c 'awk "/^## Report/,/^## Route questions/" "$0" | grep -n "asking.*Route questions"' "$SKILL"
check "route: Report still never answers a stuck or blocked run" 0 'Never' bash -c 'awk "/^## Report/,/^## Route questions/" "$0" | grep -n "Never unblock"' "$SKILL"

# --- #1815 M2: resolve routine questions (SKILL step 2) ----------------------------------------
rq_has() { local r; r="$(RQ)"; for w in "$@"; do printf '%s' "$r" | grep -qF -- "$w" || { echo "missing: $w"; return 1; }; done; echo ok; }
check "route2: judges the subject by route gate, not the category label" 0 '^ok$' rq_has "route gate" "subject" "not the category label"
check "route2: cites repo evidence in the answer" 0 '^ok$' rq_has "evidence" "route answer <key> supervisor"
check "route2: no evidence goes to the user batch as pending" 0 '^ok$' rq_has "no evidence" "pending"
check "route2: sends with resume --by supervisor" 0 '^ok$' rq_has "resume --by supervisor"
check "route2: a route gate exit 2 goes to the user" 0 '^ok$' rq_has "exits 2" "user"
check "route2: step 2 sits between Collect and Batch" 0 '^ok$' bash -c 'ord() { :; }; r="$(awk "/^## Route questions/,/^## Watcher/" "$0")"; a="$(printf "%s" "$r" | grep -n "^1\. \*\*Collect" | cut -d: -f1)"; b="$(printf "%s" "$r" | grep -n "route gate" | head -1 | cut -d: -f1)"; c="$(printf "%s" "$r" | grep -n "route next" | head -1 | cut -d: -f1)"; [ "$a" -lt "$b" ] && [ "$b" -lt "$c" ] && echo ok' "$SKILL"
check "route2: Batch with nothing pending still sends resolved, unsent issues" 0 '^ok$' bash -c 'r="$(awk "/^## Route questions/,/^## Watcher/" "$0")"; printf "%s" "$r" | grep "^3\. " | grep -qF "run steps 9 to 11" && echo ok' "$SKILL"
check "route2: the follow-up batch goes back to step 3" 0 'back to step 3' bash -c 'awk "/^## Route questions/,/^## Watcher/" "$0" | grep -o "back to step 3"' "$SKILL"
check "route gate: works on a malformed ledger" 0 '^judge$' "$SUP" route gate --ledger "$D/bad.md" 1

# --- #1815 M3: context-cap survival (route next --reask, route overdue, SKILL text) -----------
L3="$D/m3.md"; "$SUP" ledger init --ledger "$L3" --owner s-1 --cap 3 --source x 31 32 >/dev/null
esc "$D/m3a.json" "1 Product" 1 "Qa1?" 2 "Qa2?" 3 "Qa3?"
esc "$D/m3b.json" "1 Product" 1 "Qb1?" 2 "Qb2?" 3 "Qb3?"
"$SUP" route add --ledger "$L3" 31 s-31 "$D/m3a.json" >/dev/null; "$SUP" route add --ledger "$L3" 32 s-32 "$D/m3b.json" >/dev/null
for k in R32.Q1 R31.Q2 R31.Q1 R32.Q2 R31.Q3; do "$SUP" route state --ledger "$L3" "$k" asked; done
check "reask: every asked row, no limit, ascending" 0 '^R31.Q1 R31.Q2 R31.Q3 R32.Q1 R32.Q2$' bash -c '"$0" route next --ledger "$1" --reask | cut -d" " -f1 | paste -sd" " -' "$SUP" "$L3"
check "reask: carries the question, options, rec and evidence" 0 '^R31.Q1 \| Qa1\? \| A: a // B: b \| A \| e$' bash -c '"$0" route next --ledger "$1" --reask | head -1' "$SUP" "$L3"
check "reask: prints no banner or notification text" 1 '^$' bash -c '"$0" route next --ledger "$1" --reask | grep -E "Cheaper|need a decision|banner"' "$SUP" "$L3"
check "reask: leaves pending rows out" 0 '^0$' bash -c '"$0" route next --ledger "$1" --reask | grep -c "R32.Q3"; true' "$SUP" "$L3"
check "reask: an unknown option exits 2" 2 'usage' "$SUP" route next --ledger "$L3" --bogus
"$SUP" route sent --ledger "$L3" 31 1000
check "overdue: Sent older than the ceiling is listed" 0 '^31$' "$SUP" route overdue --ledger "$L3" --now 1700 --ceiling 600
check "overdue: exactly at the ceiling is not overdue" 0 '^$' "$SUP" route overdue --ledger "$L3" --now 1600 --ceiling 600
check "overdue: an empty Sent is never overdue" 0 '^31$' bash -c '"$0" route overdue --ledger "$1" --now 99999 --ceiling 1 | paste -sd" " -' "$SUP" "$L3"
check "overdue: an issue listed once" 0 '^1$' bash -c '"$0" route overdue --ledger "$1" --now 1700 --ceiling 600 | wc -l | tr -d " "' "$SUP" "$L3"
check "overdue: --now and --ceiling are required" 2 'usage' "$SUP" route overdue --ledger "$L3" --now 1700
check "overdue: a non-numeric --now exits 2" 2 'epoch' "$SUP" route overdue --ledger "$L3" --now soon --ceiling 600
sk_has() { local r; r="$(RQ)"; for w in "$@"; do printf '%s' "$r" | grep -qF -- "$w" || { echo "missing: $w"; return 1; }; done; echo ok; }
check "skill3: OVER ends the turn after banners, notification and asked" 0 '^ok$' bash -c 'ord() { :; }; r="$(awk "/^## Route questions/,/^## Watcher/" "$0")"; a="$(printf "%s" "$r" | grep -n -m1 "OVER" | cut -d: -f1)"; b="$(printf "%s" "$r" | grep -n -m1 "route state <key> asked" | cut -d: -f1)"; [ -n "$a" ] && [ "$a" -gt "$b" ] && printf "%s" "$r" | grep "OVER" | grep -qF "end the turn" && echo ok' "$SKILL"
check "skill3: names the context check" 0 '^ok$' sk_has "AUTOPILOT_CONTEXT_CAP=80000 scripts/autopilot/context.py" "OVER"
check "skill3: OVER makes no AskUserQuestion call" 0 '^ok$' sk_has "no \`AskUserQuestion\`"
check "skill3: the Cheaper line, verbatim" 0 'Cheaper' grep -nF 'Cheaper: /clear, then /autopilot-supervisor — it asks this again from the ledger.' "$SKILL"
check "skill3: Restart re-asks with route next --reask, no banner or notification" 0 '^ok$' bash -c 'r="$(awk "/^## Restart/,/^## Report/" "$0")"; for w in "route next --reask" "without a banner or notification"; do printf "%s" "$r" | grep -qF -- "$w" || exit 1; done; echo ok' "$SKILL"
check "skill3: Restart drops a session that left asking" 0 '^ok$' bash -c 'r="$(awk "/^## Restart/,/^## Report/" "$0")"; for w in "route drop" "left \`asking\`"; do printf "%s" "$r" | grep -qF -- "$w" || exit 1; done; echo ok' "$SKILL"
check "skill3: Restart checks the cap before asking, and loops in fours" 0 '^ok$' bash -c 'r="$(awk "/^## Restart/,/^## Report/" "$0")"; for w in "context.py" "Cheaper:" "at most 4" "until every printed row is answered"; do printf "%s" "$r" | grep -qF -- "$w" || exit 1; done; echo ok' "$SKILL"
check "skill3: an overdue report is noted once" 0 '^ok$' bash -c 'r="$(awk "/^11\. \*\*Confirm/,/^## Watcher/" "$0")"; printf "%s" "$r" | grep -qF "Note \"not accepting input\"" && echo ok' "$SKILL"
check "skill3: Confirm names route overdue, reports, never re-sends" 0 '^ok$' bash -c 'r="$(awk "/^11\. \*\*Confirm/,/^## Watcher/" "$0")"; for w in "route overdue" "Watcher ceiling" "never re-sent" "not accepting input"; do printf "%s" "$r" | grep -qF -- "$w" || exit 1; done; echo ok' "$SKILL"

# --- #1815 M4: upward relay (route target, SKILL text) ----------------------------------------
# list_sessions rows: ref, worktree, parent (the parent worktree's name; omitted at top level).
T="$D/sess.json"
cat > "$T" <<'J'
[{"ref":"s-par","worktree":"par-wt","is_caller":false},
 {"ref":"s-me","worktree":"me-wt","parent":"par-wt","parent_state":"known","is_caller":true},
 {"ref":"s-top","worktree":"default","is_caller":false},
 {"ref":"s-empty","worktree":"e-wt","parent":""},
 {"ref":"s-gone","worktree":"g-wt","parent":"nowhere-wt","parent_state":"known"},
 {"ref":"s-def","worktree":"d-wt","parent":"default","parent_state":"known"}]
J
check "target: a reported parent maps to its session" 0 '^parent s-par$' "$SUP" route target --sessions "$T" --me s-me
check "target: an absent parent field is the user" 0 '^user$' "$SUP" route target --sessions "$T" --me s-top
check "target: an empty parent is the user" 0 '^user$' "$SUP" route target --sessions "$T" --me s-empty
check "target: a parent with no session is the user" 0 '^user$' "$SUP" route target --sessions "$T" --me s-gone
check "target: a parent of default maps to the session in default" 0 '^parent s-top$' "$SUP" route target --sessions "$T" --me s-def
check "target: the daemon's {sessions:[...]} shape" 0 '^parent s-par$' bash -c 'jq "{sessions: .}" "$1" > "$1.obj" && "$0" route target --sessions "$1.obj" --me s-me' "$SUP" "$T"
check "target: an empty --me falls back to the is_caller row" 0 '^parent s-par$' "$SUP" route target --sessions "$T" --me ""
check "target: a failed session in the parent worktree is passed over" 0 '^parent s-live$' bash -c 'echo "[{\"ref\":\"s-dead\",\"worktree\":\"w\",\"lifecycle\":\"failed\"},{\"ref\":\"s-live\",\"worktree\":\"w\",\"lifecycle\":\"running\"},{\"ref\":\"m\",\"worktree\":\"x\",\"parent\":\"w\",\"parent_state\":\"known\"}]" > "$1" && "$0" route target --sessions "$1" --me m' "$SUP" "$D/live.json"
check "target: a parent_state other than known is the user" 0 '^user$' bash -c 'echo "[{\"ref\":\"p\",\"worktree\":\"w\"},{\"ref\":\"m\",\"parent\":\"w\",\"parent_state\":\"removed\"}]" > "$1" && "$0" route target --sessions "$1" --me m' "$SUP" "$D/ps.json"
check "target: an unknown --me falls back to the is_caller row" 0 '^parent s-par$' "$SUP" route target --sessions "$T" --me s-nobody
check "target: no matching row and no is_caller row is the user" 0 '^user$' bash -c 'jq "map(del(.is_caller))" "$1" > "$1.nc" && "$0" route target --sessions "$1.nc" --me s-nobody' "$SUP" "$T"
check "target: an unreadable file exits 2" 2 'unreadable' "$SUP" route target --sessions "$D/none.json" --me s-me
check "target: a non-JSON file exits 2" 2 'unreadable' "$SUP" route target --sessions "$D/free.json.nope" --me s-me
check "target: --sessions and --me are required" 2 'usage' "$SUP" route target --sessions "$T"
check "target: needs no ledger" 0 '^parent s-par$' bash -c 'cd "$2" && "$0" route target --sessions "$1" --me s-me' "$SUP" "$T" "$D"
check "skill4: step 6 names route target and the parent send" 0 '^ok$' sk_has "route target" "send_session_input" "Via \`parent\`" "route answer <key> parent"
check "skill4: not_found records the refusal and asks the user, never retried" 0 '^ok$' sk_has "ledger set <n> Note \"<refusal>\"" "Via \`user\`" "never retried"
check "skill4: exit 2 and an unmatched row mean the user" 0 '^ok$' sk_has "exits 2" "is_caller"
check "skill4: top tier asks the user" 0 '^ok$' sk_has "top-tier" "only from \`list_sessions\`"

echo "supervisor: $cases case(s), $failures failure(s)"
[ "$failures" -eq 0 ]
