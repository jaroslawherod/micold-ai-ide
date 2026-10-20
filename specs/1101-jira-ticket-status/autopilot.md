# Autopilot ledger — #1101 jira-ticket-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Show a Jira ticket's current status (issue #1101); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1101
- **Worktree branch**: feat/1101_show-a-jira-ticket-s-current-status
- **Started**: 2026-10-20
- **Phase**: implement
- **Next step**: M2 implement: finish per Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1152 | Design PR | merged | 537bf0d5193e751dbd41b377a70dd6488db31ba0 |
| #1155 | M1 | merged | 473538658f1b4835f0fba0eddf67d95bfe608269 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Status chip in the issue picker | #1155 | merged |
| M2 | T014–T027 | full | Current status on the sidebar | | implementing (handover 1) |
| M3 | T028–T030 | full | Status on Overview cards | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| #1152 | Design PR | open | |
| Spec | 1 | 952ec53aa3d23346f6fed8f12d836bcf43d073d0:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CHANGES: 4 MAJOR |
| Spec | 2 | 2791b5d842f73b2d98560fad5de1801a962e828a:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CHANGES: 1 MAJOR (not-found contradiction), 2 MINOR; fixed, checked by author (wording reconciliation only) |
| Plan | 1 | 87105a5c80cf11e780ace91fd3629036eddc60f5:f3cacdb0b33caca2142ce3ac8f3e8ef396c86102 | CHANGES: 3 MAJOR (400 on unknown key, project-open hook, epoch), 3 MINOR; all fixed in plan/research/data-model/contracts (design decisions added: bisection on 400, epoch, daemon_sync hook) |
| Plan | 2 | 1efaa84c3741213ab69f9dbf3dfb2877567fc01f:f3cacdb0b33caca2142ce3ac8f3e8ef396c86102 | CLEAN (1 MINOR fixed, prose only) |
| Tasks | 1 | 6807400cb49973f201fe92f29c7f7bf9c08cd521:1a784129401f5b853ac8d121376a401053af3260 | CHANGES: 2 MAJOR (shared-file [P], accessible_text test order), 3 MINOR; fixed, checked by author (task wording/ordering only); analyze HIGH C1, C2, I1 fixed |
| M1 review A | 1 | aad5399d8:f11ffb94 | 10 findings; 1 fixed (chip aligned to first line), rest declined/MINOR |
| M1 review B | 1 | aad5399d8:f11ffb94 | CLEAN (2 MINOR) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | stale flag hard-coded false in picker row | stale and the board belong to M2 (T014+) |
| M1 | A | SHORT_NAME char heuristic, double tooltip, Tag line height, contrast, status not searchable | MINOR/taste; status excluded from matching is FR-011; tooltip is FR-013 |
| M1 | B | no tdd/cycle-log.md | tests were written with the code; no red-phase log exists |
| M1 | B | fake_jira example unchanged, no In Progress ticket | statuses reach the example through the shared fixture (To Do, In Review=indeterminate, Done, none) |

## Handover

M2 unit 1 stopped at the 150k context cap. branch-start.sh ran (branch reset to origin/main, M1 merged).
Work is in the tree, committed as one WIP commit (not pushed, no PR). Nothing built since the core
tests: `cargo test -p micold-core --lib jira` was green (21 tests); the client crate has NEVER been
compiled with these edits, expect compile errors. Builds waited long on the shared build lock held by
other worktrees: start every build detached (`setsid nohup ... &`) and hold.

Done (tasks.md boxes NOT yet ticked):
- T014-T016, T020 core: `crates/micold-core/src/jira_status.rs` (statuses_jql -> Vec<StatusQuery>,
  status_jql, parse_statuses, read_statuses (chunks, abort on first error), Trigger, StatusEntry,
  StatusBoard with due/begin/apply/clear/status_of/is_idle) with tests. Red was seen (compile errors,
  StatusBoard undeclared) before the impl.
- T021: `JiraSource::statuses` default `Ok(vec![])`, `FakeJiraSource::with_statuses/script_statuses/status_calls`
  in `jira.rs`; `JiraCloud::statuses` with 400-bisection (depth 7) in `shell/jira_http.rs`; T017 tests appended to
  `crates/micold-client/tests/jira_http_adapter.rs` (written together with the impl, red not recorded).
- T022-T023 (written, uncompiled): lib `features/jira_status.rs` (State{board, open_pending}, linked_keys, plan, apply,
  now()), `Message::JiraStatusesRead{epoch,keys,result,at}` + `State.jira_status` field + arm in `app.rs`;
  bin `shell/jira_status.rs` (on_focus, after_update, trigger), `jira_request`/`JiraRequest::run` widened in `shell/issues.rs`;
  hooks in `main.rs` (`WindowFocusChanged(true)` -> on_focus; `update` batches `after_update`),
  `daemon_sync.rs` (`project_opened()` beside ProjectSwitched), `startup.rs` (`project_opened()` after restore).
  Design: the open read waits (open_pending) until worktrees are non-empty; gate closed -> board cleared.
- T024: `TreeItem::chip_after_tag(index, Element)` in `ui/material/tree_view.rs`; `ui/sidebar.rs` draws
  `StatusChip::new(&entry.status, r).stale(entry.stale)` from `state.jira_status.board.status_of(key)`.
- T025: disclosure text changed in `ui/settings/jira.rs` (DATA_USE) and `features/worktree_form.rs` (jira_notice);
  `tests/jira_source_state.rs` expected text updated. A new test that the text is shown is still TODO (T025).
- T026: `examples/fake_jira.rs` serves `key in (...)`, stdin `status KEY CATEGORY NAME`, `missing KEY`.
- T027: docs written in `docs/user-guide/settings.md` and `worktrees-and-sessions.md` ("A ticket's status" section).

Still to do:
1. Compile: `cargo check -p micold-client --all-targets` (detached), fix errors (watch: `Message` derives Eq with
   the new variant; `Tag::Issue` match in sidebar; `crate::App` import in shell/jira_status.rs; docs link anchor).
2. T018 `crates/micold-client/tests/jira_status_sidebar.rs` and T019 extend `jira_is_requested_only_on_named_events.rs`
   (caller scan for `shell/jira_status.rs::trigger`, `.jira_source()` still once), `jira_token_never_leaks` (chip tooltip/accessible text
   has no token/site). Integration tests can only use the lib (bin shell is not importable): test via
   `State.jira_status.plan/apply`, `read_statuses` + `FakeJiraSource`, and `ui::view` with worktrees (see tests/support/sidebar_fixture.rs,
   layout.rs `view_of`/`painted_text_settled`). Cover: chip beside key tag, follows a changed fake after focus >5 min
   (injected `now`), Done stays, failure keeps dimmed chip, not-found retires, other project's key no chip, clear on switch off,
   429 keeps last status, 50 keys -> 1 call, 101 -> 2, coalescing, restored project one read, disclosure text shown.
3. Stale-flag finding from M1 (declined to M2): picker `ui/material/picker.rs` hard-codes `StatusChip::maybe(.., false, ..)`.
   Plan: add `TypeaheadRow::status_stale(bool)`, add `issue_rows_with(form, &StatusBoard)` (keep `issue_rows(form)` delegating
   with an empty board) so a row whose key is on the board shows the board's status and its stale flag (FR-010); update caller
   at `ui/worktree_form.rs:524` to pass `state.jira_status.board`.
4. Write `specs/1101-jira-ticket-status/tdd/cycle-log.md` (M1 left none; dir does not exist) with the red evidence above; tick T014-T027 in tasks.md.
5. Then verify.md / review.md flow: scoped gate, Review A (high) + B, visual pass (sidebar chip, dimmed chip), full gate
   (`GIT_CONFIG_GLOBAL=/dev/null mise run gate`), PR "feat(1101): show a Jira ticket's status on the sidebar (#1101)", body per pr.md.

## Open escalation

None.

## Follow-ups not done

None.
