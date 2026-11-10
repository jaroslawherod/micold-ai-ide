# Research: supervisor run status

## D1: Implementation language and shape
- **Decision**: three bash scripts (`run-status.sh`, `supervised.sh`, `answer.sh`) using `jq`, `git`,
  `gh`.
- **Rationale**: every helper in `scripts/autopilot/` is bash; `pr-state.sh` already needs `jq`;
  tests exist in the same style; no build step.
- **Rejected**: python (`brief.py` exists but the helpers it sits with are bash and `resume.sh`
  logic must be shared); a Rust subcommand in `micold-core` (heavier than the problem, would put
  workflow glue in the app, and would not be callable from a bare worktree).

## D2: Reuse by extraction, not copy (FR-007)
- **Decision**: move `ledgers()` (as `ledgers_of <dir>`, returning all matches as paths relative to
  the toplevel, so `resume.sh` output stays byte-identical; `ledger_of` adds the absolute prefix),
  `ledger_prs` and `phase_of` from `resume.sh`, and the inline `LEDGER-ON-MAIN` scan and the
  `gh pr list --head` lookup (as `ledger_on_main <branch>` and `final_pr_open <branch>`), into the
  sourced `ledger-of.sh`. Add `pr_state_of <n>`: `gh pr view <n> --json state -q .state`, `UNKNOWN`
  on failure, the one query `resume.sh` uses, behind one function (spec Assumptions); `resume.sh` and `run-status.sh`
  source it. `handoff-check.sh` is called as is, from inside the target worktree. `ledger_of`
  stays `ledgers_of | head -1`, so the hooks are unchanged. `handoff-check.sh` has its own copy of
  the PR extraction sed: switch it to `ledger_prs` in the same change.
- **Rationale**: `ledger_of` already finds done ledgers (no phase filter) but returns only the
  first of several; status must detect several (an error). The existing `resume.sh` tests prove no
  regression.
- **Rejected**: calling `resume.sh` and parsing its text (it exits 3/4 for cases status needs as
  data, prints at most 5 escalation lines, and has no worktree argument); copying its code.

## D3: Dwell and `stuck` from `pr-state.sh` (FR-006, FR-006a)
- **Decision**: for each open PR (state from `pr_state_of`) run `$AUTOPILOT_PR_STATE <pr>` (default
  `$(dirname "$0")/pr-state.sh`, independent of the caller's cwd). With several open PRs, any
  `STUCK` makes the run `stuck` and `since` comes from the longest `DWELL`. Its line `PR n STATE s DWELL d LIMIT l OK|STUCK ACTION a` and
  exit 1 give `stuck`; `since = now - DWELL`, `dwell_source = pr-state`. The underlying state
  (`waiting-ci`) goes to `underlying_state`. Without an open PR, or with the helper absent or
  failing, no `stuck`; `since` is the ledger's last commit time (uncommitted ledger: file mtime),
  `dwell_source = ledger`.
- **Rationale**: #1806 owns the limits and `AUTOPILOT_LIMIT_*`; spec forbids a second table. The env
  override of the helper path lets the tests (and this branch before PR #1810 merges) stub it;
  the contract used is the documented one-line output and exit codes on that PR's branch.
- **Rejected**: own limits (spec clarification); importing `pr-state.sh` by `source` (it is a
  script with `exit`); waiting for #1810 (tests stub it, so the unit is independent; the first
  milestone to touch real `pr-state.sh` rebases on main after it merges).
- **Risk**: if #1810 changes the output line before merging, the parse in `run-status.sh` is the one
  place to adjust; the stub in the test pins the format.

## D4: State resolver order and `done` (FR-001a, FR-001b, FR-005)
- **Decision**: resolve in the spec's order. `dead-no-ledger`: no ledger names the branch, or
  the only ledger is on `origin/main` (note: `branch-start.sh` next). Several ledgers: error exit 3, no state, counted over non-`done` ledgers as `resume.sh`
  does; `done` ledgers are used only when none is open (fixture: one `done` plus one open). `done`: phase `done` and `handoff-check.sh` exits 0 in the target worktree. A `done`
  ledger failing the check falls through: open PR gives `waiting-ci`, else `running`. `blocked`:
  *Open escalation* contains `blocked by work outside my flow` (case-insensitive, fixed string);
  other non-empty escalation is `asking`. `waiting-ci`: an open PR (from the ledger's PR numbers,
  plus `gh pr list --head <branch> --state open` for the final PR a `done` ledger does not list yet,
  as `resume.sh` does). Otherwise `running`.
- **Rationale**: matches `resume.sh` outcomes one for one (FR-001b table in the data model).
- **Rejected**: treating `HANDOVER` as its own state (not one of the seven; it is `running` with
  the phase visible).

## D5: Structured *Open escalation* in the ledger (FR-003, FR-010)
- **Decision**: the section holds `- **Question**:`, `- **Options**:` (nested list `<n>. <label> — <why>`),
  `- **Recommended**:`, `- **Evidence**:`, `- **Category**:` fields; a batch uses `### Q<n>`
  subsections, each with those fields; the banner is kept under `- **Banner**:` or below the fields.
  JSON always has `escalation.questions[]`, one element for a single question. A legacy free-text
  escalation (no fields) yields `question` = first non-blank line, other fields empty.
- **Rationale**: the banner has no options list, so options and evidence cannot be parsed from it;
  a field list is greppable with `sed`, no markdown parser. A uniform array avoids two shapes.
- **Rejected**: YAML/JSON block in the ledger (unreadable by humans, a new format); parsing the
  banner (lossy).

## D6: Supervised marker (FR-009)
- **Decision**: `AUTOPILOT_SUPERVISED` non-empty and not `0`, or ledger line
  `- **Supervised**: yes`. At start the orchestrator copies the env var into the ledger field so it
  survives `resume` in a new session; `supervised.sh [ledger]` is the single test, also used for the
  "ledger predates it" edge case (env alone suffices). `run-status.sh` reports `supervised`.
- **Rejected**: env only (lost on resume); field only (cannot mark an existing run without editing).

## D7: Answer path (FR-011, FR-011a, FR-012)
- **Decision**: message `resume <answer>`. The orchestrator passes the text after `resume` to
  `answer.sh <ledger> <text>`, which appends one *Decisions* row per question (`By` = `user`,
  evidence `supervised answer <date>`), then resets the section to `None.`. Batch: the answer has
  lines `<n>. <text>` (or `<n>: <text>`), one per question; a missing or extra number is exit 2
  and nothing changes. No open escalation: exit 3 with the note, nothing recorded. Free text and
  option label are stored verbatim, with `|` escaped as `\|` and newlines flattened to spaces in the
  Decisions row. Batch or single is decided by the escalation's `### Q<n>` count, never by the
  answer text, so a single answer starting `1.` stays one answer.
- **Rationale**: puts the testable logic in a script rather than prose; the only ledger writer is
  separate from the read-only status command.
- **Rejected**: the unit editing *Decisions* by hand (untestable, FR-014 asks for tests).

## D8: Fixtures and `gh` stub
- **Decision**: extract the stub, `new_repo`, `ledger` and `check` from `autopilot.test.sh` into
  `scripts/tests/lib/autopilot-fixtures.sh`; `autopilot.test.sh` and `run-status.test.sh` source it.
  The stub gains `pr list --head/--state` support if missing (it has `pr list` already).
- **Rejected**: a second stub copy (the duplication budget, `mise run duplication`).
