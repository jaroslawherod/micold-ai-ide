# Implementation Plan: Default prompt template for a new worktree's session

**Branch**: `feat/1161_default-prompt-template-in-settings-for-a-new` | **Date**: 2026-10-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1161-default-prompt-template/spec.md`

## Summary

One global template string lives in Settings. A pure, render-free module in `micold-core`
(`prompt_template`) validates it and renders it against `WorktreeFacts`. The create-worktree dialog
pre-fills the first-prompt field from it while the user has not edited the field; the daemon's MCP
`create_session` / `create_worktree` render it when no non-blank `prompt` is given, with the issue
title and body read from GitHub under a 5 s bound. See [research.md](research.md).

## Technical Context

**Language/Version**: Rust (workspace edition), iced for the client.

**Primary Dependencies**: existing only (`serde`, `serde_json`, `iced`, `tokio`); no new crate.

**Storage**: the settings file (`micold-core::settings::Settings`), one new key
`default_prompt_template`, `#[serde(default)]` so older files read as empty (FR-001). No
`settings_version` bump (additive key, as the neighbouring keys).

**Testing**: `cargo test` via `mise run test-core` (render/validate logic, settings round trip),
`micold-client/tests` (form reducer, settings draft), `micold-daemon/tests` (MCP, beside
`mcp_create_session.rs`), `mise run gate` before pushing. Visual pass: quickstart §B.

**Target Platform**: Linux, macOS, Windows; the render is a pure function, no host branching.

**Project Type**: desktop app (client) plus background service (daemon).

**Performance Goals**: render is microseconds; the only wait is the MCP issue lookup, bounded at 5 s
(FR-012, US4 s6).

**Constraints**: offline-capable (a failed lookup renders empty); rendering never fails creation.

**Scale/Scope**: one string setting, one module, three call sites (dialog, `create_session`,
`create_worktree`).

## Constitution Check

- [x] **I. Test-First**: all decision logic (parse, validate, render, "has the user edited",
  explicit-prompt-wins choice) lands in `micold-core` or render-free client/daemon modules, each
  test-first. Only `ui/settings/` and `ui/worktree_form.rs` widget wiring uses the quickstart
  exception.
- [x] **II. Multi-Session**: the template is read at render time and copied into a session's first
  input; no session state is shared or changed afterwards.
- [x] **III. Worktree Integration**: no worktree operation changes; MCP `create_worktree` keeps
  creating a session only with `ai_cli`/`prompt` (FR-009).
- [x] **IV. Local-First**: stored in the local settings file; the only network use is the existing
  `gh` issue lookup, which already exists for the tool and degrades to empty offline.
- [x] **V. Rust + iced**: a `PromptTemplate` newtype makes an unvalidated string unrenderable
  (`validate` returns the type `render` takes).
- [x] **VI. Cross-Platform Parity**: pure function, no OS branch; CI covers all three platforms.
- [x] **VII. Documentation**: `docs/user-guide/settings.md`, `worktrees-and-sessions.md` and
  `agent-tools.md` updated in the same change.
- [x] **VIII. Reusable UI**: the Settings field reuses the existing multi-line/text field primitive
  in `ui/cdk` (see research R6); no feature-local widget.

Re-checked after design: unchanged, no violations. Complexity Tracking: none.

## Project Structure

### Documentation (this feature)

```text
specs/1161-default-prompt-template/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/prompt-template.md
└── tasks.md            # next unit
```

### Source Code

```text
crates/micold-core/src/
├── prompt_template.rs      # NEW: PromptTemplate, WorktreeFacts, TemplateError, validate, render
├── settings.rs             # Settings.default_prompt_template + DaemonSettings field (daemon-to-client)
├── protocol/messages.rs    # NEW Option<String> field on the set-settings message (client-to-daemon)
├── github.rs               # IssueSnapshot.body_text, Issue.body (full plain text), query adds bodyText
└── lib.rs                  # pub mod prompt_template
crates/micold-client/src/
├── features/worktree_form.rs  # first_prompt_edited flag, prefill on CLI/issue/type/name/details change
├── features/settings.rs       # draft field (text_editor::Content; string extracted for validate) + validation in SettingsDraft::validate; send path to daemon
└── ui/settings/               # reuses ui/material/text_area.rs; placeholder list, error line (glue)
crates/micold-daemon/src/
├── mcp/tools.rs            # resolve_first_prompt in create_session / start in create_worktree
├── ops.rs                  # facts gathering with 5 s bound (reuses IssueLookup)
├── server.rs               # NEW handler arm, as for default_ai_cli
└── catalog.rs / state.rs   # NEW set_default_prompt_template; settings_wire carries it; accessor on DaemonState
docs/user-guide/{settings,worktrees-and-sessions,agent-tools}.md
```

**Structure Decision**: logic in `micold-core` so dialog and MCP share one renderer (FR-011).

## Requirement map

| FR | Where |
|---|---|
| FR-001 | `Settings.default_prompt_template: String`, `#[serde(default)]`; service-owned, written via the set-settings message (research R3); data-model §1 |
| FR-002 | `ui/settings/` field + list of `PromptTemplate::PLACEHOLDERS` |
| FR-003, FR-004 | `prompt_template::render`, contracts/prompt-template.md |
| FR-005 | `PromptTemplate::validate` called from `SettingsDraft::validate`; refusal keeps stored value (existing save path writes only on `Ok`) |
| FR-006 | `render` returns `Option<String>`, `None` when blank after trim |
| FR-007 | `WorktreeForm.first_prompt_edited` + `refresh_default_prompt`; submit sends field text (`session_definition`) |
| FR-008, FR-009 | `mcp/tools.rs::first_prompt_for` (explicit non-blank wins, else render) |
| FR-010 | single field, no per-CLI/type keys |
| FR-011 | both call `prompt_template::render` |
| FR-012 | render is total; MCP lookup wrapped in `tokio::time::timeout(5 s)`; any error gives `None` |
| FR-013 | settings load keeps the text; `PromptTemplate::validate` at use gives `Err` so callers treat as empty; Settings draft validates on open and shows the error |
| FR-014 | MCP facts: `{issue}` = `naming::ticket(dir)`; `{type}` = the `Tag::Type` of `naming::parse_tags(dir)`, empty if none; `{name}` = `naming::display_name(dir)` (public; its own fallback applies); `{branch}` = the worktree's branch from the catalog (empty if unknown); title/body via `IssueLookup`. Dialog facts: from the form (`picked_issue`, `type_`, name field, derived branch). Neither new field (`Issue.body`, `IssueSnapshot.body_text`) is persisted or serialised; `tests/github_privacy.rs` and `jira_is_requested_only_on_named_events.rs` are updated for the added field |

## Test strategy

| Requirement | Layer |
|---|---|
| FR-003/004/005/006, US2, US3 s1 | core unit (`prompt_template` tests) |
| FR-001, FR-013, US3 s2 | core unit (settings round trip, old file, invalid stored text) |
| FR-007, US1 s1-5 | client reducer tests (`tests/features_worktree_form.rs`) |
| FR-005 in Settings, FR-002 | client `tests/features_settings.rs` (draft validate); layout is glue, quickstart §B |
| FR-001 service-owned path | daemon test like `tests/settings_default_ai_cli.rs`: set via message; an unrelated settings change does not revert it; wire carries it |
| FR-011 | one core test fixture renders the same template/facts; dialog and daemon tests assert against that fixture's output |
| SC-004 dialog half | client reducer test: an explicit edited prompt reaches `session_definition` unchanged |
| FR-008, FR-009, US4, SC-004 | daemon tests beside `mcp_create_session.rs` with a fake `IssueLookup` (incl. timeout/failure, Jira key) |
| SC-003 | existing dialog and MCP tests unchanged with empty template |
| Look of the Settings field and the pre-filled prompt | quickstart §B visual pass |
