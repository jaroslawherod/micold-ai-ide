# Tasks: Default prompt template for a new worktree's session

**Input**: `specs/1161-default-prompt-template/` (plan.md, spec.md, research.md, data-model.md, contracts/prompt-template.md, quickstart.md)

**Tests**: mandatory and written first (Constitution I). A test task precedes the implementation it covers and must fail for the right reason before the implementation lands. Widget layout under `ui/settings/` is glue: quickstart §B covers it.

**Format**: `- [ ] T### [P?] [US?] Description with path`

## Phase 1: Setup

- [x] T001 Add `pub mod prompt_template;` to `crates/micold-core/src/lib.rs` and an empty `crates/micold-core/src/prompt_template.rs` so the crate compiles.

## Phase 2: Foundational (blocks every story)

- [x] T002 [P] Write failing core tests in `crates/micold-core/tests/prompt_template.rs`: parse and render all six placeholders (`{issue}`, `{issue_title}`, `{issue_body}`, `{branch}`, `{type}`, `{name}`); missing value renders empty (FR-004); `{{`/`}}` give literal braces; an inserted body containing `{issue}` is not re-expanded (US2 s4); blank-after-trim result is `None` (FR-006); `Unknown("issu")` error and `Unbalanced` error with the exact `Display` text of contracts/prompt-template.md (US3 s1); one shared fixture (template plus facts incl. a name with a prefix or tag, expected output) exported for FR-011 reuse.
- [x] T003 Implement `PromptTemplate::parse`, `PromptTemplate::render(&self, &WorktreeFacts) -> Option<String>`, `Placeholder`, `PLACEHOLDERS`, `TemplateError`, `WorktreeFacts` in `crates/micold-core/src/prompt_template.rs`: one left-to-right scan, values never re-scanned (research R2; data-model §2, §3).
- [x] T004 [P] Write failing settings tests in `crates/micold-core/tests/settings_default_prompt_template.rs`: a settings file without the key reads as empty (FR-001); round trip keeps newlines verbatim; stored text that fails `PromptTemplate::parse` still loads and is kept in the file (FR-013).
- [x] T005 Add `Settings.default_prompt_template: String` with `#[serde(default)]` (no `settings_version` bump) and the mirrored `DaemonSettings` field in `crates/micold-core/src/settings.rs`; default empty.
- [x] T006 [P] Write failing daemon test `crates/micold-daemon/tests/settings_default_prompt_template.rs`, modelled on `settings_default_ai_cli.rs`: setting it through the set-settings message stores it; an unrelated settings change does not revert it; `settings_wire()` carries it (FR-001 service-owned path).
- [x] T007 Add the `Option<String>` field to the client-to-daemon set-settings message in `crates/micold-core/src/protocol/messages.rs`; handler arm in `crates/micold-daemon/src/server.rs`; `set_default_prompt_template` in `crates/micold-daemon/src/catalog.rs` and an accessor on `DaemonState` in `crates/micold-daemon/src/state.rs`; `settings_wire` returns it (research R3).
- [x] T008 [P] Update `crates/micold-core/tests/github_privacy.rs` first for the new `Issue.body` field (not persisted, not serialised), then add `Issue.body: String` (full plain text from the list query, which already reads `bodyText`; `description` stays the truncated row text) in `crates/micold-core/src/github.rs`.

**Checkpoint**: the renderer, the setting and its daemon path exist.

## Phase 3: User Story 1 and 2 - Dialog pre-fill with placeholders (P1) 🎯 MVP

**Goal**: a stored template pre-fills the create-worktree dialog's first prompt, with all placeholders, until the user edits the field. (US2's rendering is T002/T003; US2 s1-4 are asserted at the dialog level here.)

**Independent test**: set the template, open the dialog, pick issue and CLI, read the first-prompt field, edit it, change the issue, create.

- [x] T009 [P] [US1] Write failing reducer tests in `crates/micold-client/tests/features_worktree_form.rs`: US1 s1 (`/speckit-autopilot #{issue}` with issue #42 gives `/speckit-autopilot #42`), s2 (edited text reaches `session_definition` unchanged, SC-004 dialog half), s3 (cleared field stays empty and is not re-applied), s4 (empty template leaves the field empty, SC-003), s5 (typed text kept when issue, type or name changes; an unedited field follows the template); US2 s1 and s2 through the form (no issue gives `/speckit-autopilot #`); issue details arriving later re-render an unedited field; the output equals the T002 shared fixture's output (FR-011).
- [x] T010 [US1] Implement `WorktreeForm.first_prompt_edited` (set by `first_prompt_changed`, cleared on form open and when a CLI is newly chosen from none) and `refresh_default_prompt` (after a change of CLI, issue, issue details, type or name, only when `!first_prompt_edited` and a CLI is set; `None` gives empty) in `crates/micold-client/src/features/worktree_form.rs`; the template reaches the form from the client's settings state, filled from the daemon's `DaemonSettings` where the client stores `default_ai_cli` (`grep -n default_ai_cli crates/micold-client/src`); the first-prompt field exists only once a CLI is chosen (feature 776), so the reset on a CLI newly chosen from none cannot discard typed text; facts come from `picked_issue` (incl. `Issue.body`), `type_`, the name field and the derived branch.
- [x] T011 [P] [US1] Write failing draft tests in `crates/micold-client/tests/features_settings.rs`: the draft holds the template as a `text_editor::Content`, opens with the stored text, and saving sends the extracted string in the set-settings message.
- [x] T012 [US1] Add the draft field (`text_editor::Content`) and the send-on-save path to `crates/micold-client/src/features/settings.rs`.
- [x] T013 [US1] Settings UI glue in `crates/micold-client/src/ui/settings/mod.rs` (new field section module if the file is long): a multi-line field reusing `ui/material/text_area.rs`, and the list of `PromptTemplate::PLACEHOLDERS` (FR-002). No feature-local widget (Constitution VIII); quickstart §B.1 covers it.
- [x] T014 [P] [US1] User guide: document the template, its placeholders and the dialog pre-fill in `docs/user-guide/settings.md` and `docs/user-guide/worktrees-and-sessions.md`.

**Checkpoint**: M1 is observable in the app.

## Phase 4: User Story 3 - Validation in Settings (P2)

**Goal**: an invalid template is refused on save with a message; an invalid stored one never breaks startup.

**Independent test**: type `{issu}` in the setting and observe the message.

- [x] T015 [P] [US3] Write failing tests in `crates/micold-client/tests/features_settings.rs`: `SettingsDraft::validate` refuses `Do {issu}` with `Unknown placeholder {issu}; valid: {issue}, {issue_title}, {issue_body}, {branch}, {type}, {name}` and an unbalanced brace with its message, and nothing is sent or stored (US3 s1, FR-005); a valid template passes; opening Settings with an invalid stored template yields the error (US3 s2).
- [x] T016 [P] [US3] Write failing test in `crates/micold-client/tests/features_worktree_form.rs`: an invalid stored template renders as empty in the dialog and the form still opens (FR-013).
- [x] T017 [US3] Call `PromptTemplate::parse` from `SettingsDraft::validate` and on open in `crates/micold-client/src/features/settings.rs`; treat a parse `Err` as empty in `refresh_default_prompt` in `crates/micold-client/src/features/worktree_form.rs`.
- [x] T018 [US3] Error line under the field in `crates/micold-client/src/ui/settings/mod.rs` (glue; quickstart §B.1).
- [x] T019 [P] [US3] User guide: the validation message and the valid placeholder list in `docs/user-guide/settings.md`.

## Phase 5: User Story 4 - MCP `create_session` and `create_worktree` (P2)

**Goal**: an MCP-started session without an explicit prompt gets the rendered template.

**Independent test**: `create_session` on an issue worktree with and without `prompt`, then `read_session_output`.

- [x] T020 [P] [US4] Write failing tests: extend `crates/micold-core/tests/github_privacy.rs` for `IssueSnapshot.body_text` (not persisted or serialised) and update the Jira privacy test `jira_is_requested_only_on_named_events` (find it with `grep -rn jira_is_requested_only_on_named_events crates`) for the added field.
- [x] T021 [US4] Add `IssueSnapshot.body_text` and `bodyText` to the single-issue `ISSUE_QUERY` (a different query from T008's list query) in `crates/micold-core/src/github.rs`.
- [x] T022 [P] [US4] Write failing daemon tests in `crates/micold-daemon/tests/mcp_default_prompt.rs`, beside `mcp_create_session.rs`, with a fake `IssueLookup`: US4 s1 (template rendered), s2 (`prompt: "hi"` wins, template not parsed or looked up), s3 (`create_worktree` with `ai_cli` and no prompt), s4 (empty template unchanged), s5 (neither `ai_cli` nor `prompt` creates no session), s6 (lookup failure and a lookup that outlasts 5 s render title and body empty: `Fix  (#42)`), s7 (Jira key `ABC-12` gives `{issue}` = `ABC-12`, no lookup); blank explicit prompt falls through to the template; a template error means no prompt and the reply is as today; output equals the T002 shared fixture (FR-011); the lookup timeout test uses `tokio::time::pause` or an injected timeout, never a real 5 s wait; existing `mcp_create_session.rs` and `mcp_create_worktree.rs` pass unchanged (SC-003).
- [x] T023 [US4] Implement `first_prompt_for(explicit, template, facts)` in `crates/micold-daemon/src/mcp/tools.rs` and call it from `create_session` and the `create_worktree` start path; gather facts in `crates/micold-daemon/src/ops.rs` (`{issue}` = `naming::ticket(dir)`, type from `naming::parse_tags`, name from `naming::display_name`, branch from the catalog, title and body through `IssueLookup` under `tokio::time::timeout(5 s)` on the blocking pool; any error or timeout leaves them empty) (FR-008, FR-009, FR-012, FR-014; research R5, R8).
- [x] T024 [P] [US4] Mention the default in the `create_session` and `create_worktree` tool descriptions in `crates/micold-core/src/mcp/tools.rs` (reply schema unchanged) and update `docs/user-guide/agent-tools.md`.

## Dependencies

- Phase 1 then Phase 2. Within Phase 2: T002 before T003; T004 before T005; T006 before T007; T005 before T007; T001 before T003; T008 is independent.
- US1 needs Phase 2 (T003, T005, T007, T008). US3 needs US1's draft and form code (T010, T012); T017 follows T015 and T016 (same files). US4 needs Phase 2 only, but ships after.
- Parallel: T002, T004, T006, T008 touch disjoint files; T014 beside T011-T013.

## Implementation Strategy

MVP is the dialog pre-fill (M1). Validation (M2) and the MCP path (M3) follow. Quickstart §B is run by the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Template in Settings and dialog pre-fill 🎯 MVP

- **Tasks**: T001–T014
- **Deliverable**: A template saved in Settings (multi-line field, placeholders listed) pre-fills the create-worktree dialog's first prompt for the chosen issue and CLI, keeping the user's edits.
- **Satisfies**: US1 acceptance scenarios 1–5; US2 acceptance scenarios 1–4; FR-001, FR-002, FR-003, FR-004, FR-006, FR-007, FR-010, FR-011 (dialog half), FR-012; SC-001
- **Verify**: `mise run test-core`, `cargo test -p micold-client --test features_worktree_form --test features_settings`, `cargo test -p micold-daemon --test settings_default_prompt_template`, quickstart §B steps 1 (second half), 2 and 3
- **Depends on**: —
- **Tier**: full

### M2 — Validation in Settings

- **Tasks**: T015–T019
- **Deliverable**: Saving `Do {issu}` is refused with the message naming `{issu}`; an invalid stored template does not stop the app and renders as empty.
- **Satisfies**: US3 acceptance scenarios 1–2; FR-005, FR-013; SC-002
- **Verify**: `cargo test -p micold-client --test features_settings --test features_worktree_form`, quickstart §B step 1
- **Depends on**: M1
- **Tier**: light

### M3 — MCP default prompt

- **Tasks**: T020–T024
- **Deliverable**: `create_session` and `create_worktree` (with `ai_cli`) without an explicit prompt type the rendered template; an explicit prompt wins; a failed lookup renders title and body empty within 5 s.
- **Satisfies**: US4 acceptance scenarios 1–7; FR-008, FR-009, FR-011, FR-012, FR-014; SC-003, SC-004
- **Verify**: `cargo test -p micold-daemon --test mcp_default_prompt --test mcp_create_session --test mcp_create_worktree`, quickstart §B step 4
- **Depends on**: M1
- **Tier**: full

Polish (quickstart §B results, task ticks) changes no code: left to the close unit. M1 has 14 tasks (over the 10-task guide) but US1 and US2 share one rendering path and the dialog needs the setting, its daemon path and the Settings field to be observable; no acceptance scenario split yields a deliverable on its own.

## Phase 6: TDD remediation

Source: `tdd/verification.md` (verdict BLOCKED). The feature is not done until T025 and T026 are cleared.

- [ ] T025 (finding 1) Run `/speckit-tdd-plan` to create `specs/1161-default-prompt-template/tdd/test-list.md`, then run the whole suite once the disk has room (`mise run gate`) and record the result; re-run `/speckit-tdd-verify` to lift BLOCKED. Test-first order of M1-M3 cannot be recovered; record it as `LIKELY` at best.
- [ ] T026 (findings 2, 3) Add tests in `crates/micold-client/tests/features_worktree_form.rs` that drive `Msg::TypeSelected`, `TicketChanged`, `NameChanged`, issue pick and listing arrival through the reducer with a template set and assert `first_prompt`; add one test that a stored `DaemonSettings.default_prompt_template` reaches the opened form (`crates/micold-client/src/shell/`). Proof: removing the `refresh_default_prompt()` call at `worktree_form.rs:1221` makes one fail; `cargo test -p micold-client --test features_worktree_form`.
- [ ] T027 (finding 4) Pin `PROMPT_LOOKUP_BOUND` to 5 s in a unit test in `crates/micold-daemon/src/state.rs` (US4 s6). Proof: `cargo test -p micold-daemon --lib state::`.
- [ ] T028 (finding 5) Add a case to `crates/micold-daemon/tests/mcp_default_prompt.rs` with `github_issues_enabled` off asserting no lookup and empty title/body. Proof: deleting the gate at `ops.rs:600` fails it; `cargo test -p micold-daemon --test mcp_default_prompt`.
- [ ] T029 (finding 7) Split the two-scenario tests at `mcp_default_prompt.rs:274` and `:315` into one scenario each. Proof: `cargo test -p micold-daemon --test mcp_default_prompt`.
