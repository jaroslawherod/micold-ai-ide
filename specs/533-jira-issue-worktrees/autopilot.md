# Autopilot ledger — #533 jira-issue-worktrees

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Create worktrees from Jira issues (#533); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #533
- **Worktree branch**: feat/533_create-worktrees-from-jira-issues
- **Started**: 2026-10-12
- **Phase**: milestone M2
- **Next step**: M2 implemented, uncommitted tests unverified: see Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #785 | Design (spec, plan, tasks) | merged | a9ea34a2d7d101cbf3b9ce52a88ad9d6e59aeb61 |
| #819 | M1 | merged | 270c5480611867f8038b9a7a2d4e21bbca6f7b8b |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T021, T034 | full | Jira switch, settings fields, protocol 45 and the opt-in gate | #819 | merged |
| M2 | T022–T031 | full | Connect, change and remove a project's connection and token | | pending |
| M3 | T032–T033, T035–T053, T068–T069 | full | Start a worktree from a Jira issue (list, pick, notices); kept whole: core, adapter and dialog are one wired path (no half-wired UI) | | pending |
| M4 | T054–T058, T070 | full | Search Jira issues incl. beyond the loaded list | | pending |
| M5 | T059–T065 | full | GitHub / Jira source choice per project | | pending |

## Decisions

- Plan: switch and connections are service-owned settings (like pr_status_enabled), token a client-side owner-only file; spec Edge "Daemon and sessions" reworded from "not in a session or a daemon" to "no session depends on it".
- Plan: new crate `ureq` 3 (rustls, platform verifier) in the client only; loopback http accepted under cfg(debug_assertions) for the fake site.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | a2227c4095c7db1a63b01b0a70523db4e3314256:13165b3a | CHANGES: 4 MAJOR (all fixed; F5-F7 MINOR fixed) |
| Spec | 2 | 27281b4a37b70e88a0745093f36f29294f5df30f:13165b3a | pending |
| Plan | 1 | b464635f9860e95f65afac4e26f67e12622c7041:0fff8148e25af0f12869d0e5b4ec5db7ca6db297 | CHANGES: 2 MAJOR (F1 multi-daemon/spec wording, F2 loopback fake vs https-only), 4 MINOR; all fixed. F1 amended spec Edge "Daemon and sessions" wording; F2 and F4 add behaviour claims, so re-review (round 2) |
| Plan | 2 | b240cd6856a484a346554254053aaee983f769a8:0fff8148e25af0f12869d0e5b4ec5db7ca6db297 | CHANGES: 1 MAJOR (release test unwired), 2 MINOR; fixed |
| Plan | 3 | adf44a395a8299f73ed08629565c5b6f44af926b:0fff8148e25af0f12869d0e5b4ec5db7ca6db297 | CHANGES: 1 MAJOR (plan self-contradiction on persist_service_settings merge vs copy), 2 MINOR; fixed, prose only (plan/contract wording made to match the merge mechanism round 3 already read; no 4th round, round limit) |
| Plan | confirm (sonnet, tasks unit) | bae8666f588552c1c7a2447bfe65696f43efeeae:c779f5c824f46d2e9d3649c1647476dc206ab227 | CHANGES: 1 MAJOR (plan.md risk row still said persist_service_settings merges); fixed, prose only; rest consistent |
| Code A (M1) | 1 | 1a86b2ebe74b3bdc92bcb8e2f960d87581bde677:5bd6892383a878b47a76d49455fc3e8bf7809d3c | CLEAN (3 MINOR: port range check, empty project key, silent drop on read; declined for M1, see below) |
| Conformance B (M1, sonnet) | 1 | 0382303712ba5169e5a0b928a79cbcec442fa916:5bd6892383a878b47a76d49455fc3e8bf7809d3c | CHANGES: 1 MAJOR (no client test for FR-022 adopt/save), 2 MINOR declined; fixed with 2 tests |
| Conformance B (M1, sonnet) | 2 | e9271b767957cb77284ebf522020edafcff7229e:5bd6892383a878b47a76d49455fc3e8bf7809d3c | CLEAN |
| Code A (M1, sonnet, post-rebase fixes) | 2 | 5b7730bd1c00e525627e260e6bdb497674bc4c91:5f00fbbe33617bf46639612eba895030677f8d3c | CLEAN (3 MINOR: F2 wire-leniency test added, F3 version pin dropped from the 776 test; F1 write-order declined, same order as every other field) |
| Tasks | 1 | bae8666f588552c1c7a2447bfe65696f43efeeae:c779f5c824f46d2e9d3649c1647476dc206ab227 | CLEAN (2 MINOR fixed; speckit-analyze 2 HIGH, 4 MEDIUM fixed before review) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | MINOR F1-F3 (port u16 range; non-absolute project key; silent drop log) | MINOR only; port and project validation are covered by M2's connect form (T022+) which owns site/key input; no new round needed |

## Handover

M2 (T022-T031) code and tests are written on the branch (reset to origin/main, based on #819's merge) and
`cargo check --workspace --all-targets` was green; nothing is gate-run, reviewed or pushed yet.
Done: core `ApiToken`, `ConnectionAction`, `store::jira_token_dir/jira_token_path/jira_token_file_name`;
client `shell/jira.rs` (apply/write/remove/probe), `Capabilities.jira_token_dir`, persist.rs save wiring
(token file first, `ServiceChanges.jira_connection`, keep-or-remove prompt raised in `on_settings_saved`),
`JiraDraft` form fields + `TokenInput` + `KeepOrRemovePrompt` + `Msg::Jira*`, `TextField::secure`,
`ui/settings/jira.rs` form, `FieldId::SettingsJira{Site,Key,Email,Token}`, user guide (T031), tests:
`core/tests/jira_connection.rs` additions, `features_settings.rs` `jira_connection_form`,
`jira_connection_shell.rs`, `jira_token_never_leaks.rs`, `covered_states.rs` 3 states.
Next: (1) run `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot` via build-lock
to regenerate `layout_snapshot.txt` (T026), commit it; (2) optionally add a `main_tests.rs` test of save ->
token file + SettingsSet (use `connected_app()`, push a `Project`, set `app.caps` token dir needs a
`#[cfg(test)] with_jira_token_dir` that was removed as unused); (3) tick T022-T031 in tasks.md; (4) scoped
gate + review A (code-review high) + review B (conformance) per verify.md/review.md; (5) full `mise run
gate` detached; (6) push and open PR "feat(533): connect, change and remove a Jira connection (#533)".
Known gaps to check in review: ui text for status uses "Connected to {site} - {KEY}."; M1 declined MINORs
(port range, key validation) are covered by `JiraConnection::parse` in the form; a Save of an invalid Jira
form blocks the whole Settings save (by design, FR-006). Build lock is heavily contended (1h+ waits).

## Open escalation

None.

## Follow-ups not done

- Main took PROTOCOL_VERSION 41 and 42 (feature 776) before M1 merged: M1 is now 42 -> 43, then main took 43 and 44: M1 is now 44 -> 45; spec artifacts updated.
- Main then took 43 (feature 552) too: M1 is now 44 -> 45.

- M1 had no test-first red-phase evidence (tests written after the code in part 1); no tdd/cycle-log.md; coverage is by the tests T003-T008, T015-T017 passing in the gate, not by red-then-green history.
- GitHub issue picker brought under an integrations switch (pending clarify answer).
