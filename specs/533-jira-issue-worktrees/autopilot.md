# Autopilot ledger — #533 jira-issue-worktrees

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Create worktrees from Jira issues (#533); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #533
- **Worktree branch**: feat/533_create-worktrees-from-jira-issues
- **Started**: 2026-10-12
- **Phase**: tasks
- **Next step**: M1 implementation WIP committed, see Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #785 | Design (spec, plan, tasks) | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T021, T034 | full | Jira switch, settings fields, protocol 41 and the opt-in gate | | pending |
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
| Tasks | 1 | bae8666f588552c1c7a2447bfe65696f43efeeae:c779f5c824f46d2e9d3649c1647476dc206ab227 | CLEAN (2 MINOR fixed; speckit-analyze 2 HIGH, 4 MEDIUM fixed before review) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 (T001-T021, T034) is WRITTEN BUT NOT YET COMPILED OR TESTED, committed as one WIP commit on
`feat/533_create-worktrees-from-jira-issues` (not pushed, no PR). branch-start.sh 785 ran (design PR
#785 merge SHA a9ea34a2d7d101cbf3b9ce52a88ad9d6e59aeb61). No `mise run` build finished: every
attempt queued behind other worktrees' build locks (`scripts/build-lock.sh`); run the next build as a
detached job and `scripts/autopilot/hold.sh` it.

Done (written): core `jira.rs` (SiteOrigin, ProjectKey, AccountEmail, JiraConnection, FieldProblem,
`jira_visible`, `connection_key`, `known_connections`), `settings.rs` (jira_enabled, jira_connections,
`JiraConnectionChange{project, change: JiraChange::Upsert|Remove}`), `store::project_id_of`,
messages.rs (SettingsSet `jira_enabled`/`jira_connection`; DaemonSettings `jira_enabled`/
`jira_connections`), PROTOCOL_VERSION 41, daemon catalog/state/server setters
(`persist_service_settings_with` merges the change into the on-disk map), client draft
(`SettingsSection::Jira`, `JiraDraft{enabled, project, connection}`, `JiraEnabledToggled`), persist.rs,
daemon_sync.rs (`app.core.settings.jira_enabled/jira_connections`), `ui/settings/jira.rs`,
`Icon::JiraIssues` (E85D), CI step, user guide, tests T003-T008, T015-T017.

Next step:
1. Build and fix compile errors. Known to fix: every `DaemonSettings { .. }` literal lacks
   `jira_enabled`/`jira_connections` (tests in micold-core protocol_roundtrip, micold-daemon tests,
   micold-client main_tests.rs/persist.rs tests ~line 1129, covered_states); `SettingsDraft { .. }`
   literals in tests/support/covered_states.rs lack `jira: ..Default`; `tests/settings_sections.rs`
   needs `("jira_connections", "M2 ...")` in DEFERRED (jira.rs claims `jira_enabled`);
   `SettingsSection` exhaustive matches elsewhere; `SettingsSet` literals the perl pass missed.
2. Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` (new covered states
   settings-view-jira-off, settings-view-jira-on-no-connection) and check any icon/provenance tests.
3. `mise run gate`; tick T001-T021, T034 in tasks.md; TDD evidence per speckit-implement hook;
   then verify.md, review, review-rounds, gate, pr per the unit task files.
Caveat: tests were written after the code (no red-phase evidence); note it in the cycle log.

## Open escalation

None.

## Follow-ups not done

- GitHub issue picker brought under an integrations switch (pending clarify answer).
