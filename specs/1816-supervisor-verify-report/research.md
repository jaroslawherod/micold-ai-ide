# Research: supervisor verify handoff and report

## D1: The verdict reuses run-status.sh, not handoff-check.sh directly (FR-002)
- **Decision**: `verify-run.sh <worktree>` runs `run-status.sh --json <worktree>` and reads `state`,
  `phase`, `ledger`, `issue`, `flow`, `prs`, `notes`. `state == done` means the ledger reads `done`
  and `handoff-check.sh` passed in that worktree. Otherwise the remaining items are `notes` (the
  handoff lines, when the phase is `done`), or one line `ledger phase is <phase>`, or `no ledger`.
  Exits 2 and 3 of `run-status.sh` pass through (not a worktree; several ledgers).
- **Rationale**: it already picks the ledger from the worktree's branch, runs the handoff check
  inside that worktree and treats an `UNKNOWN` PR as not done. Calling it adds no logic that could
  drift and gives the issue number and flow for the issue check.
- **Rejected**: calling `handoff-check.sh` directly (repeats the ledger lookup and the
  several-ledgers rule); parsing `resume.sh` text (exits 3/4, no worktree argument); a new copy of
  the three checks (FR-002 forbids it).

## D2: Issue check by `gh issue view`
- **Decision**: after the handoff step, `gh issue view <n> --json state,labels`. Remaining lines:
  `issue #<n> is open`, `issue #<n> still labelled in-progress`, `issue #<n> lacks flow:<flow>`.
  A failed call is `issue #<n> state UNKNOWN`. The ledger's Issue field is `#<n>`: the leading `#` is stripped before `gh issue view`. A ledger with issue `none` skips the step.
  `issue.sh done` is what closes the issue and removes the label, so the check names what it left.
- **Rationale**: GitHub is the source of truth for an issue as for a PR (FR-003; #1812 FR-004).
- **Rejected**: trusting the ledger's Issue field alone (it records the number, not the state).

## D3: Follow-up counts live in the supervisor ledger; the script decides, the supervisor acts (FR-006, FR-007)
- **Decision**: `follow-up.sh <worktree> --supervisor-ledger <file> --session live|stopped
  [--session-id <id>] [--check <id>]` verifies, then prints one action line: `DONE`, `SEND
  <session-id>` followed by the message, `RESUME <session-id>`, `ESCALATE`, `HOLD` or `SKIP`. It
  keeps a row per run (key: the run's branch) in a `## Follow-ups` table of the supervisor ledger:
  count, hash of the last remaining set, last check id. When the count is 3 and the run is still
  `NOT DONE` it writes one category 5 block to `## Escalations` and prints `ESCALATE`; after that
  it prints `HOLD` and changes nothing.
- **Rationale**: scripts cannot call MCP tools, so the supervisor performs the printed action. The
  cap of 3 comes from category 5 ("CI red after 3 fix attempts", `rules/escalation.md`). The hash
  and check id give the "not sent twice within one check" edge case without a lock.
- **Rejected**: counting in the run's own ledger (it belongs to the run, `rules/ownership.md`); a
  cap read from config (no second source for a number the rules already fix); sending from the
  script (it has no session transport).

## D4: Safe to remove is a conjunction the script can prove, plus a liveness input (FR-011)
- **Decision**: `safe-to-remove.sh <worktree>... [--live <path>]... [--no-live-sessions]
  [--supervisor-ledger <file>]` calls `verify-run.sh` (`VERIFIED` covers clean tree, no commit off
  `origin/main`, merged PRs, closed issue) and rejects a path passed as `--live` (the supervisor
  builds the list from `list_sessions`). Prints `SAFE <path>` or `KEEP <path> <reason>`. When the
  supervisor ledger has `- **Auto-remove**: yes` it also prints `REMOVE <path>` for each SAFE one;
  the supervisor passes those to `delete_worktree`, which asks confirmation when nobody
  pre-approved it. The script runs no removal.
- **Rationale**: removal is destructive and the app owns it (Principle III, FR-013). The script
  cannot see sessions, so liveness is an explicit input: with neither `--live` nor
  `--no-live-sessions` every worktree is `KEEP no session list given`, never assumed idle.
- **Rejected**: removing from the script by git or `rm` (bypasses the app's confirmation and its
  session cleanup); reading session state from files in the app's data directory (an undocumented
  store).

## D5: Tokens only from a recorded field (FR-009)
- **Decision**: read `- **Tokens**: <n>` from the run's ledger; otherwise print `unknown`.
- **Rationale**: no run total exists today (`docs/development/autopilot-tokens.md`: each `CONTEXT`
  line is one unit's); summing guesses would report a number nobody recorded.
- **Rejected**: summing `checkpoint.sh` lines; calling `scripts/autopilot-tokens.py` (it reports a
  worktree's sessions, not a run, and is unavailable once a worktree is removed).

## D6: Report is markdown on stdout, entries from the ledger and run-status (FR-008, FR-010)
- **Decision**: `close-out-report.sh <worktree>... [--supervisor-ledger <file>] [--live <path>]...
  [--no-live-sessions]` prints sections `Completed`, `Not done`, `Escalated`, `Safe to remove`,
  each present even when empty. Per run: issue, PRs with GitHub state (from `run-status.sh
  --json`), decisions by `user` and by anything else (counted from the *Decisions* **By** column),
  interventions (rows of the supervisor ledger's `## Interventions` for the run, else `none`),
  tokens. A run is Completed only when `verify-run.sh` says `VERIFIED`; Escalated when the
  supervisor ledger has a block for it and the run is not `VERIFIED`; Not done otherwise. Precedence:
  `VERIFIED` wins, then Escalated, then Not done (a test covers a verified run with an old block). One run's error (exit 2 or 3) is a
  line in Not done, never an abort. The Safe-to-remove section comes from `safe-to-remove.sh`.
- **Rejected**: JSON output (the user reads it; a supervisor needing data calls `run-status.sh
  --json`); a Rust renderer (workflow glue stays out of the app).

## D7: The supervisor ledger shape is a minimal contract defined here
- **Decision**: #1813/#1814 have not landed, so contracts/follow-up.md fixes only what this feature
  reads and writes: the field `- **Auto-remove**:` and the sections `## Interventions`,
  `## Follow-ups`, `## Escalations`. A missing file or section reads as empty; writing creates the
  section. When #1813 lands its template adopts these names or this contract moves (a follow-up in
  the ledger).
- **Rejected**: waiting for #1813 (M5 is listed independent of the loop); guessing its columns
  beyond what is needed.
