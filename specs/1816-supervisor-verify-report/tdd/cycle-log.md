# Cycle Log: 1816 M1 verify-run.sh

Append only. Newest last.

## Cycle 1 — US1.1 to US1.5, FR-001..FR-005 (T001–T006 → T007)

- tests: `scripts/tests/close-out.test.sh` (new): all-merged VERIFIED, open and UNKNOWN PR, dirty tree,
  unmerged commit, open phase, no ledger, no PRs (verified), several open ledgers, not a worktree,
  issue open / in-progress / flow label missing / UNKNOWN, second-clone isolation, read-only checksums.
- red: the same file run with `scripts/autopilot/verify-run.sh` removed → `close-out: 22 cases, 21 failed`
  (the lone pass is a usage-error case that passes without the script). Evidence kept in the session scratchpad (`red1.log`).
- green: `verify-run.sh` added (T007) → `close-out: 22 cases, 0 failed`.

## Cycle 2 — review A F1, F2, F3 (ledger-less open PR, several done ledgers, run-status failure)

- tests: three cases in `close-out.test.sh`, committed before the fix (commit "test(1816): red cases …").
- red: `close-out: 25 cases, 3 failed` — "the branch's open PR missing from the ledger is listed" (exit 0,
  want 1: VERIFIED), "several done ledgers are exit 3 too" (exit 0, want 3), "a failing run-status.sh
  is NOT DONE" (exit 0, want 1).
- green: `verify-run.sh` lists every PR run-status.sh reports not MERGED once, refuses several
  ledgers with exit 3, maps a run-status.sh failure to NOT DONE, and needs state `done` for VERIFIED
  → `close-out: 25 cases, 0 failed`.

## Cycle 3 — M3 report: US3.1 to US3.3, FR-008..FR-010, SC-002 (T013 → T014)

- tests: report cases in `close-out.test.sh`: two verified runs (one entry each: issue, PRs, decisions
  by user/agent, interventions, tokens), open-PR run only under Not done, escalated section with the
  Question line, `none` / `unknown` without interventions or tokens, zero runs and absent supervisor
  ledger (empty sections, exit 0), a run that regresses after being reported done moves to Not done,
  errored runs never abort the report.
- red: the file run before `close-out-report.sh` existed → all report cases failed with exit 127
  (`close-out-report.sh: No such file or directory`); the earlier 62 cases still passed.
- green: `close-out-report.sh` added (T014) → report cases pass.

## Cycle 4 — M3 parent filtering, FR-016 (T020)

- tests: three runs (own parent, another parent, none) plus a removed-parent row; `--parent` without
  `--rows` and unreadable rows exit 2.
- red: same run, the script absent → exit 127 for each case, written in the same red pass as cycle 3.
- green: `--parent <ref> --rows <file>` calls `runs-of-parent.sh --check` per run and names the parent
  → `close-out: 78 cases, 0 failed`.

## Cycle 5 — M4 safe-to-remove list, FR-011..FR-013, FR-016, SC-005 (T015 → T016, T021)

- tests: `close-out.test.sh` cases for SAFE (clean verified, `--live` elsewhere, `--no-live-sessions`) and
  KEEP (live session, no session list, dirty tree, unmerged commit, open PR, not a worktree, several ledgers);
  opt-in off (no ledger, absent file, `maybe`) gives zero `REMOVE` and every worktree still exists; opt-in on
  gives `REMOVE` after `SAFE` only; a text guard finds no removal command in the script; `--parent/--rows`
  keep to the own parent; the report's Safe to remove section. Commit "test(1816): red cases …".
- red: `close-out: 110 cases, 24 failed`, each with exit 127 (`safe-to-remove.sh: No such file`) or the report
  printing `- none` for a SAFE worktree; the earlier 78 cases still passed.
- green: `safe-to-remove.sh` added, `close-out-report.sh` calls it for its last section. The first green run
  failed 5 cases on my fixtures (two runs shared PR 7 and one `GH_FIXTURES`); fixed in the tests, not the script
  → `close-out: 104 cases` passing apart from the rules cases.

## Cycle 6 — M4 rules file, FR-015 (T017 → T018, T019)

- test: the rules file names `verify-run.sh`, `follow-up.sh`, `close-out-report.sh`, `safe-to-remove.sh`,
  `Auto-remove` and `delete_worktree`.
- red: 6 failures (`the rules file does not name …`), file absent.
- green: `rules/supervisor-close-out.md` written, the pressure scenario S13 and the README entries added
  → `close-out: 110 cases, 0 failed`.
