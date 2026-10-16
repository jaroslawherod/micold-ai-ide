# Autopilot ledger — #533 jira-issue-worktrees

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Create worktrees from Jira issues (#533); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #533
- **Worktree branch**: feat/533_create-worktrees-from-jira-issues
- **Started**: 2026-10-12
- **Phase**: milestone M3
- **Next step**: M3 implement continues from Handover (T032–T033, T035–T053, T068–T069)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #785 | Design (spec, plan, tasks) | merged | a9ea34a2d7d101cbf3b9ce52a88ad9d6e59aeb61 |
| #819 | M1 | merged | 270c5480611867f8038b9a7a2d4e21bbca6f7b8b |
| #871 | M2 | merged | bcc36d0bf1c2908d967f669178733d1cb2865fd3 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T021, T034 | full | Jira switch, settings fields, protocol 45 and the opt-in gate | #819 | merged |
| M2 | T022–T031 | full | Connect, change and remove a project's connection and token | #871 | merged |
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

M3 implement is in progress (unit hit 150k). Nothing committed yet (branch reset to origin/main, M2 merged #871; ledger M2 row/PR table already updated, uncommitted).

Design decisions (all under specs/533 contracts; deviations to note in the PR):
- No `count_open` in `JiraSource`; Jira `IssuePage.total_open` = issues on page; `complete` = no cursor left.
- New `IssueLoadError::Unreachable(String)` for Jira transport failures ("Couldn't reach {host}: {reason}."); GitHub `Offline` unchanged. `message()` takes `impl Into<NoticeSource>` (`NoticeSource::{Github(&repo), Jira{host,key}}`).
- M3 shows Jira only for a project WITHOUT a GitHub remote (`WorktreeForm.provider` resolved in `remotes_listed`/`JiraSettingsChanged`); Jira never starts a search beyond the cap or a description pass in M3 (search is M4: cap caption has no "search also looks on" yet).
- Switch-off / connection change while form open: `JiraSettingsChanged(JiraAvailability)` -> form.source back to New, issues reset, `issue_request_seq` bumped (late result drops).
- Missing token file at load: no request, `IssueLoadError::Other("...enter your token in Settings > Jira")`.

STATE AT RETURN: both workers were still running, blocked on the build lock held by another worktree (`build-lock.sh: another build holds the lock`); working-tree files are uncommitted and may be mid-edit. Check `git status`, then `scripts/build-lock.sh cargo check --workspace --all-targets` before relying on them. The deps worker already finished T068/T069 (features_settings green) and edited Cargo.toml/Cargo.lock/docs; the core worker has tests+impl written and was running clippy/check.

Background workers dispatched by the previous unit (their files are in the working tree; if the tree shows them unfinished, finish them yourself, do not re-dispatch blindly):
1. core worker: T035-T039, T044-T046 (crates/micold-core: jira.rs, github.rs, issue_types.rs, naming.rs, tests jira_jql/jira_parse/jira_classify/jira_load/issue_jira_rows, fixtures/jira). API per brief: `jira::{list_jql, search_jql, parse_page, classify, Outcome, JiraSource{list_open,search_open}, FakeJiraSource, load, IssueProvider}`, `Issue::{jira,key,ticket,with_type_name,type_name}`, `issue_types::type_for_issue`.
2. deps+docs worker: T032, T033 (ureq 3 + base64 in micold-client Cargo.toml/Cargo.lock, research.md R2 addendum), T068, T069, T052, T053.

Done by me so far: crates/micold-client/src/features/worktree_form.rs partly edited (imports, JiraSite, JiraAvailability, WorktreeForm.jira/provider fields, jira_notice, jira_site, issue_source_ready, notice_source, resolve_provider, Jira cap caption). Still to do in it: use `notice_source()` in issue_search_status (line ~700 `error.message(repo)`), `source_changed` use `!form.issue_source_ready()`, `opened` set `jira` from state.settings.jira_enabled/jira_connections + state.workspace.active, `remotes_listed` call `resolve_provider()`, `issues_loaded` descriptions=Done for Jira, `issue_query_changed` no Pending search for Jira, `issue_picked` ticket = issue.ticket(), `Msg::JiraSettingsChanged(JiraAvailability)` + reducer, `State::issue_search_request`/description request only for GitHub provider.

Remaining tasks: T040 (tests/jira_http_adapter.rs), T041, T042, T043 (covered_states + regenerate layout_snapshot), T047 (shell/jira_http.rs `JiraCloud` + Capabilities field/accessor `jira_source()` built only in `Capabilities::real()`, declare in shell/mod.rs), T048 (shell/jira.rs `start_load` + provider dispatch in shell/issues.rs `start_issue_load`; call `FormMsg::JiraSettingsChanged` from `adopt_daemon_settings` in shell/daemon_sync.rs), T050 (ui/worktree_form.rs: chip label "Issue" while connected, Jira notice, Loading "Loading issues from {host}…", Failed+Retry via notice_source, "{KEY} has no open issues.", placeholder "Search by key, summary or label", enable chip via issue_source_ready), T051 (examples/fake_jira.rs). Then verify.md flow (scoped gate + review A high, review B, visual pass if anything visible, full gate, PR).

## Open escalation

None.

## M2 review notes

- Review A (code-review high): CLEAN; MINOR F1 (failed token I/O still saves rest of settings) and F2 (orphan token file with no connection never removed) declined: MINOR, outside M2 scope.
- Review B (sonnet): MAJOR F1 no cycle-log: recorded honestly in tdd/cycle-log.md as a process deviation (red not captured); MINOR F2 doc prompt text fixed; F3 same as A F1, declined.

## Follow-ups not done

- Main took PROTOCOL_VERSION 41 and 42 (feature 776) before M1 merged: M1 is now 42 -> 43, then main took 43 and 44: M1 is now 44 -> 45; spec artifacts updated.
- Main then took 43 (feature 552) too: M1 is now 44 -> 45.

- M1 had no test-first red-phase evidence (tests written after the code in part 1); no tdd/cycle-log.md; coverage is by the tests T003-T008, T015-T017 passing in the gate, not by red-then-green history.
- GitHub issue picker brought under an integrations switch (pending clarify answer).
