#!/usr/bin/env bash
# Drives scripts/autopilot/runs-of-parent.sh against a stubbed `list_worktrees` result, and greps
# the supervisor skill for the ownership and relay rules (specs/1857-worktree-parent, FR-013, FR-017).
set -uo pipefail
# shellcheck source=lib/autopilot-fixtures.sh
. "$(dirname "$0")/lib/autopilot-fixtures.sh"
RUNS="$S/runs-of-parent.sh"
SKILL="$ROOT/.claude/skills/autopilot-supervisor/SKILL.md"
cd "$tmp" || exit 1

# Rows in the real `list_worktrees` shape (crates/micold-daemon/src/mcp/tools.rs, worktree_rows):
# ref, display_name, branch, path, status, session_count, and parent/parent_state when recorded.
# Two supervisors (sup-a, sup-b) own runs; one run has no parent, one lost its parent.
row() { # ref [parent state]
  jq -nc --arg r "$1" --arg p "${2:-}" --arg s "${3:-}" \
    '{ref: $r, display_name: ("Name " + $r), branch: $r, path: ("/work/proj/" + $r), status: "clean", session_count: 1}
     + (if $p != "" then {parent: $p, parent_state: $s} else {} end)'
}
{
  echo '['
  echo '{"ref":"default","display_name":"default","branch":null,"path":"/work/proj","status":"clean","session_count":0},'
  row sup-a default known; echo ,
  row run-1 sup-a known; echo ,
  row run-2 sup-a known; echo ,
  row run-3 sup-b known; echo ,
  row run-4; echo ,
  row run-5 sup-gone removed
  echo ']'
} > rows.json
jq '{worktrees: .}' rows.json > wrapped.json
# What the daemon returns for `list_worktrees` with `parent: sup-a`: only known direct children.
jq '[.[] | select(.parent == "sup-a" and .parent_state == "known")]' rows.json > filtered.json

check "own runs only" 0 '^run-1 run-2$' bash -c '"$0" sup-a < rows.json | tr "\n" " " | sed "s/ $//"' "$RUNS"
check "the other supervisor gets its own" 0 '^run-3$' bash -c '"$0" sup-b < rows.json' "$RUNS"
check "default lists only its direct children" 0 '^sup-a$' bash -c '"$0" default < rows.json' "$RUNS"
check "a worktrees object is read too" 0 '^run-1 run-2$' bash -c '"$0" sup-a < wrapped.json | tr "\n" " " | sed "s/ $//"' "$RUNS"
check "a daemon-filtered list gives the same own runs" 0 '^run-1 run-2$' bash -c '"$0" sup-a < filtered.json | tr "\n" " " | sed "s/ $//"' "$RUNS"
check "no runs: empty output, exit 0" 0 '^$' bash -c '"$0" sup-z < rows.json' "$RUNS"
check "own run is accepted by ref" 0 '^own: run-1$' "$RUNS" sup-a --check run-1 --rows rows.json
check "own run is accepted by path (the ledger Worktree cell)" 0 '^own: run-1$' "$RUNS" sup-a --check /work/proj/run-1 --rows rows.json
check "another parent's run is refused" 1 'refused.*parent sup-b' "$RUNS" sup-a --check run-3 --rows rows.json
check "a run with no parent is refused" 1 'refused.*no parent' "$RUNS" sup-a --check run-4 --rows rows.json
check "the default row has no parent and is refused" 1 'refused.*no parent' "$RUNS" sup-a --check default --rows rows.json
check "a run with a removed parent is refused" 1 'refused.*removed' "$RUNS" sup-a --check run-5 --rows rows.json
check "a removed parent is refused even for its old name" 1 'refused.*removed' "$RUNS" sup-gone --check run-5 --rows rows.json
check "an unknown run is refused" 1 'refused.*not listed' "$RUNS" sup-a --check nope --rows rows.json
check "a display name is not an id" 1 'refused.*not listed' "$RUNS" sup-a --check "Name run-1" --rows rows.json
check "orphans are reported, never selected" 0 '^run-5 sup-gone$' bash -c '"$0" sup-a --orphans < rows.json' "$RUNS"
check "a daemon-filtered list has no orphans to see" 0 '^$' bash -c '"$0" sup-a --orphans < filtered.json' "$RUNS"
check "removed rows are not own runs" 0 '^$' bash -c '"$0" sup-gone < rows.json' "$RUNS"
check "no parent ref is a usage error" 2 'usage' bash -c '"$0" < rows.json' "$RUNS"
check "unreadable json is an error" 2 'unreadable' bash -c 'echo nope | "$0" sup-a' "$RUNS"
check "empty input is an error, not zero runs" 2 'unreadable' bash -c ': | "$0" sup-a' "$RUNS"
check "an object without worktrees is an error" 2 'unreadable' bash -c 'echo "{}" | "$0" sup-a' "$RUNS"
check "rows that are not objects are an error" 2 'unreadable' bash -c 'echo "[1, 2]" | "$0" sup-a --check run-1' "$RUNS"
check "a missing rows file is an error" 2 'unreadable' "$RUNS" sup-a --rows /nonexistent

# --- the supervisor ledger records the parent per run (T025) ----------------------------------
SUP="$S/supervisor.sh"; L="$tmp/supervisor.md"
"$SUP" ledger init --ledger "$L" --owner s-0 --cap 2 --source issues 12 >/dev/null
check "ledger: Parent starts empty" 0 '^$' "$SUP" ledger get --ledger "$L" 12 Parent
check "ledger: Parent is recorded per run" 0 '^sup-a$' bash -c '"$0" ledger set --ledger "$1" 12 Parent sup-a && "$0" ledger get --ledger "$1" 12 Parent' "$SUP" "$L"
check "ledger: Note and the table are unaffected" 0 '^#12 queued$' bash -c '"$0" ledger table --ledger "$1" | tail -1' "$SUP" "$L"

old="$tmp/old.md"
sed -e 's/ | Parent |$/ |/' -e 's/|---|---|---|---|---|---|---|---|---|/|---|---|---|---|---|---|---|---|/' -e 's/ |  |$/ |/' "$L" > "$old"
check "ledger: an 8-column ledger refuses Parent instead of writing past the row" 2 'Parent' "$SUP" ledger set --ledger "$old" 12 Parent sup-a
check "ledger: an 8-column ledger still works for its own fields" 0 '^x$' bash -c '"$0" ledger set --ledger "$1" 12 Note x && "$0" ledger get --ledger "$1" 12 Note' "$SUP" "$old"

# --- skill text: ownership (T025) and relay (T033) ---------------------------------------------
sect() { awk '/^## Ownership/,/^## Advance loop|^## Restart/' "$SKILL"; }
has() { local r; r="$(sect)"; for w in "$@"; do printf '%s' "$r" | grep -qF -- "$w" || { echo "missing: $w"; return 1; }; done; echo ok; }
check "skill: ownership section exists" 0 '^ok$' has "runs-of-parent.sh"
check "skill: create with the default parent, record parent per run" 0 '^ok$' has "default" "Parent" "ledger set"
check "skill: never act on another parent's run or none" 0 '^ok$' has "another parent" "none"
check "skill: removed parent is reported, never adopted" 0 '^ok$' has "removed" "never adopt" "orphan"
check "skill: a supervisor in default is still bound" 0 '^ok$' has "not limited by the daemon" "still applies"
check "skill: relay rule, worker contacts parent and siblings only" 0 '^ok$' has "parent and siblings" "forwards" "never expects a worker"
check "skill: orphans come from an unfiltered list" 0 '^ok$' has "worktrees-all.json" "--orphans --rows"
check "skill: the check takes the ref" 0 '^ok$' has "--check <ref>"
check "skill: the relay text has no stray punctuation" 0 '^ok$' bash -c '! awk "/^### Relay/,/^## Start/" "$0" | grep -qF ",." && echo ok' "$SKILL"
check "skill: the worker prompt carries the relay rule" 0 '^ok$' has "worker prompt"

echo "supervisor-ownership: $cases case(s), $failures failure(s)"
[ "$failures" -eq 0 ]
