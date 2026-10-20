# Autopilot ledger — #1161 default-prompt-template

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1161 Default prompt template in settings for a new worktree's initial session prompt; labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1161
- **Worktree branch**: feat/1161_default-prompt-template-in-settings-for-a-new
- **Started**: 2026-10-20
- **Phase**: milestone M1
- **Next step**: M1 implement, verify, review, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1163 | Design | merged | 7406e4cdc61cbf75bc88c92a2bee08913b6bdf71 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | Template in Settings, pre-fills the dialog's first prompt | #… | pending |
| M2 | T015–T019 | light | Validation in Settings; bad stored template renders empty | #… | pending |
| M3 | T020–T024 | full | MCP create_session/create_worktree render the template | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | create_worktree with neither ai_cli nor prompt starts a session when a template exists? | No | agent-resolved | spec.md FR-009 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 2da4e91e074c34b2608429055921e3a617527a46:d9c2e378c1ef6e6a8f142a5369dd16c856e9324c | CHANGES: 3 MAJOR |
| Spec | 2 | 3399ba1e782776dc6bc887373536e91692f57891:e62f06e9aef3c487cbfc825ed1ce3e1a9e3a7437 | CLEAN (3 MINOR fixed, prose only) |
| Plan | 1 | 9c1a3ea4c4eafd8fac474ff9036262b2110d35ce:3f9fcba65f32e6fe61a0c04eb2b35443fbcb36f1 | CHANGES: 2 MAJOR, 2 MINOR (all fixed) |
| Plan | 2 | 67a3b302e232e0f97f90ca37f8ed269e5ed8e9c9:4bbf066bfe6165d2de73d7111a10784a402091ba | CLEAN |
| Tasks | 1 | 15866164d7f79a3d92d85e3fb046d236eedcf7c4:c4edd43e6fdac03c072a44bd4b8263fdc3df7b3b | CLEAN (2 MINOR fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 (T001-T014) partly done by unit 1, committed as WIP (not gated, not pushed, no PR). Branch is on origin/main after design PR #1163 (merge SHA 7406e4cd).
Done and green: T001-T003 (`crates/micold-core/src/prompt_template.rs` + `shared_fixture` module + tests/prompt_template.rs, 9 tests pass); T004-T005 (Settings.default_prompt_template, tests pass); T006-T007 (daemon test tests/settings_default_prompt_template.rs, catalog/state/server arms, SettingsSet and DaemonSettings fields, PROTOCOL_VERSION 57 -> 58 with tests updated); T008 (Issue.body set by `described()`, `Issue::body()`, github_privacy test passes); T011-T012 (EnvironmentDraft.default_prompt_template + default_prompt_editor (app::Editor), `Msg::DefaultPromptTemplateAction`, ValidSettings/into_settings, persist.rs ServiceChanges/set_changed/window_settings, daemon_sync, startup, `session.default_prompt_template`; tests appended to tests/features_settings.rs, not yet run). All struct literals across workspace were patched; last full `cargo build --workspace --all-targets` had only features_settings/main_tests errors, which were then fixed (rebuild to confirm).
NOT done: T009 tests are written (module `default_prompt_template` at the end of crates/micold-client/tests/features_worktree_form.rs) but red/uncompiled. T010 to implement in `crates/micold-client/src/features/worktree_form.rs`: `WorktreeForm.default_prompt_template: String` (copied from `state.session.default_prompt_template` in `opened()`), `first_prompt_edited: bool`, methods `refresh_default_prompt(&mut self)` (only when !edited and session_cli set; facts: issue=ticket.trim(), title/body from `issues.held()` issue with number == picked_issue, branch=preview().ok().branch, type=type_.as_str(), name; use `micold_core::prompt_template::render_text`, None gives empty), `edit_first_prompt(String)` (sets edited, ignored without CLI), `choose_session_cli(Option<AiCli>)` (None clears prompt; newly chosen from none clears edited and refreshes). Wire into session_cli_chosen, first_prompt_changed, type_selected, ticket_changed, name_changed, issue_picked, issue_details_loaded/issues loaded, opened. Test needs `GithubAvailability`, `IssueList`, `SearchState` public in that module and `SessionDefinition.prompt`. T013 (ui/settings/mod.rs: multi-line field via ui/material/text_area.rs bound to `Msg::DefaultPromptTemplateAction`, plus placeholder list from `PromptTemplate::PLACEHOLDERS`; find where Environment section is rendered, ui/settings/environment.rs). T014 docs (docs/user-guide/settings.md, worktrees-and-sessions.md). Then tick tasks in tasks.md, tdd cycle log not kept (no tdd/test-list.md exists; tasks.md already orders tests first), then verify.md steps: scoped gate, review A (high) and B, visual pass (quickstart section B steps 1 second half, 2, 3), full gate, PR "feat(1161): ... (#1161)".

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
