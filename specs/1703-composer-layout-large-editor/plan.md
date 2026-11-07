# Implementation Plan: Structured view composer layout and large editor

**Branch**: `feat/1703_structured-view-composer-layout-large-editor` | **Date**: 2026-11-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1703-composer-layout-large-editor/spec.md`

## Summary

Rearrange the structured view's composer (`crates/micold-client/src/ui/composer.rs`): the activity
indicator moves into the input row as an icon-only control; model, effort and permission selectors
move into a "session options row" toggled by a settings icon (state persisted in `Settings`, the
`filter_open` precedent); the three selectors share one `MenuSelect` trigger style; a new modal
large editor (Ctrl/Cmd+Shift+M, icon button) edits the draft. Persistence mechanism (left open by
clarify): one new `Settings.composer_options_open: bool` field, see [research.md](research.md) R1.

## Technical Context

**Language/Version**: Rust (workspace edition), iced (workspace pin)

**Primary Dependencies**: `micold-core` (iced-free logic, `Settings`), `micold-material` (icons,
`IconButton`, `BusyIndicator`, `Modal`, `TextArea`), `micold-client` (`ui/material/menu_select.rs`,
`ui/composer.rs`, `features/composer.rs`, `overlay/registry.rs`, `shell/persist.rs`, `shell/startup.rs`)

**Storage**: the existing local settings file via `SettingsStore` (`micold-core/src/settings.rs`); no new file

**Testing**: `cargo test` via mise: core unit (settings round trip), client reducer/unit tests in
`crates/micold-client/tests/` and `src/**` test modules, geometry gates (selector anatomy),
quickstart §B visual pass

**Target Platform**: Linux, macOS, Windows desktop (iced)

**Project Type**: desktop-app (Rust workspace)

**Performance Goals**: none beyond existing 60 fps redraw; the editor opens in one frame

**Constraints**: offline, local-only; selector choices and disabled rules unchanged (FR-018)

**Scale/Scope**: one composer per session, one options-row flag per window, one large editor per window

## Constitution Check

| Principle | Result |
|---|---|
| I. Test-First | PASS. Decision logic (chord, toggle, auto-expand, editor open/confirm/cancel, settings round trip) lands in render-free modules with failing tests first; only `ui/` glue uses the quickstart exception. |
| II. Multi-Session | PASS. The large editor is bound to the opening `SessionId` (FR-019); drafts stay per session; the options flag is window-level by design (FR-011). |
| III. Worktree | PASS. N/A: no file or VCS operation. |
| IV. Local-First | PASS. One boolean in the local settings file. |
| V. Rust + iced | PASS. The editor is `Option<LargeEditor>`: no editor without a session; no second editor representable. |
| VI. Cross-Platform | PASS. Chord resolves Ctrl vs Cmd in one function, with tests for both; tooltip text per platform as for Add attachment. |
| VII. Documentation | PASS. `structured-input.md`, `terminal-panes.md` shortcut table, in-app sheet (FR-005). |
| VIII. Reusable UI | PASS. Selector unification extends the shared `MenuSelect` (disabled state, filter row, 32 dp trigger) instead of a composer-local widget; large-editor body composes `Modal`, `TextArea`, `Button`, `IconButton`. New/extended components keep builder APIs and get showcase entries. |

Complexity Tracking: no violations.

## Project Structure

### Documentation (this feature)

```text
specs/1703-composer-layout-large-editor/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/composer-layout.md
└── tasks.md   (next unit)
```

### Source Code

```text
crates/micold-core/src/settings.rs             # + composer_options_open (Settings, serde mirror, default false)
crates/micold-client/src/shell/persist.rs      # + persist_composer_options_open; form-save exclusion list
crates/micold-client/src/shell/startup.rs      # read it into core state at boot
crates/micold-client/src/shell/composer.rs     # NEW arm: Message::Composer is routed here (main.rs:1157); call persist_composer_options_open whenever options_open changes (toggle or auto-expand on a refusal)
crates/micold-client/src/main.rs               # no change expected beyond the existing routing
crates/micold-client/src/features/composer.rs  # options_open flag, toggle, auto-expand on model/effort/permission refusal; chord intent LargeEditorRequested
crates/micold-client/src/features/large_editor.rs  # NEW render-free: LargeEditor{session,content}, open/confirm/cancel reducer
crates/micold-client/src/overlay/registry.rs   # register the large-editor dialog
crates/micold-client/src/ui/large_editor.rs    # NEW dialog body (Modal, TextArea, Done, Cancel)
crates/micold-client/src/ui/composer.rs        # new layout: indicator before the text area, settings IconButton after it, then attach beside Send (#1764, D8), options row, open-editor IconButton
crates/micold-client/src/ui/mode_control.rs    # permission selector becomes a MenuSelect (the `Select` use here is dropped; `Select` stays for other callers); leaves the input row
crates/micold-client/src/ui/material/menu_select.rs  # NEW API: disabled state + tooltip, MenuSelectRow::filter; shared trigger 32dp/8dp/states/disabled+tooltip; MenuSelectRow::filter
crates/micold-client/src/features/changes/keys.rs    # shortcut-sheet entry (next to Add attachment, l.480)
crates/micold-material/src/icons.rs            # Icon::Settings exists; Icon::OpenInFull is NEW (codepoint, Icon list entry, mapping)
crates/micold-client/src/showcase/sections/controls.rs  # showcase MenuSelect disabled and filter variants (catalogue.rs if an entry is needed)
docs/user-guide/structured-input.md, docs/user-guide/terminal-panes.md
```

**Structure Decision**: existing client/core split; no new crate.

## Requirement map

| FR | Where |
|---|---|
| 001, 002, 003, 019 | `features/large_editor.rs` reducer; chord in `composer_key` (`features/composer.rs`); `ui/large_editor.rs`; registry |
| 004 | `ui/composer.rs` input row; visibility rule is a pure fn in `features/composer.rs` (lines>1 or focused or hovered) |
| 005 | `features/changes/keys.rs` sheet entry, both docs |
| 006, 007, 008 | `ui/composer.rs` input row; `BusyIndicator` label as tooltip; `mode_control::seams` drops `control` |
| 009, 010, 013, 014 | `ui/composer.rs` (settings `IconButton` `.selected()`, options row, Bypass marker); `status_row` keeps Stop session and refusal note |
| 011, 012 | `Settings.composer_options_open` ([data-model.md](data-model.md)); reducer auto-expand |
| 015, 016, 017, 018 | `ui/material/menu_select.rs`, `ui/composer.rs` `model_control`/`effort_control`, `ui/mode_control.rs` |

## Test strategy

| Requirement | Layer |
|---|---|
| Settings field default/round trip/old file/unreadable -> collapsed (011) | core unit, `settings.rs` tests |
| Auto-expand on a model/effort/permission refusal sets and persists `composer_options_open` (012) | client `shell/composer.rs` test with a mock `SettingsStore` |
| Persist on toggle, form save keeps it, boot reads it (011) | client `shell/persist.rs`, `shell/startup.rs` tests |
| Toggle, auto-expand on refusal, Stop/refusal visible (009, 012, 013) | client reducer tests `tests/features_composer.rs` |
| Chord Ctrl/Cmd+Shift+M only when focused; open/confirm/cancel; empty draft; second open ignored; session binding; ended session (001-003, 019, edge cases) | client reducer tests (render-free `large_editor.rs`) |
| Button visibility rule (004) | client unit test of the pure predicate |
| Indicator placement, no permission selector, labels (006-008) | client view-structure tests like `tests/composer_tab.rs` |
| Selector geometry/states (015-017) | geometry gate test `ui/material/menu_select_anatomy.rs` (NEW, modelled on `select_anatomy.rs`) |
| Shortcut listed (005) | client test on sheet entries + docs check in CI |
| Look, state treatments, ripple, tonal icon, Bypass marker | quickstart §B visual pass |

## Phase 1 design check

Post-design Constitution Check unchanged: all PASS.
