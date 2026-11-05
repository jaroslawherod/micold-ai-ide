# Research: composer layout and large editor

## R1. Persistence of the options-row state (open from clarify)

- **Decision**: new `composer_options_open: bool` on `Settings` (`micold-core/src/settings.rs`), `#[serde(default)]` (false), mirrored in the on-disk struct, excluded from the settings form's save (`shell/persist.rs` exclusion list), written by `persist_composer_options_open` after the toggle, read in `shell/startup.rs`. Additive, so `settings_version` does not move.
- **Rationale**: exact precedent in BUG-1470 (`filter_open`) and feature 551 (`folded_origins`): client-owned UI state kept in the user's local settings file; unreadable or missing -> default false, which is the spec's fallback. The app is one window per process sharing one settings file, so "per window" is that file.
- **Rejected**: `Workspace` store (`store.rs`; daemon-written, project/session scoped, wrong owner and scope); a new state file (extra I/O path and load/save code for one bool); per-session flag (spec says per window); not persisting (violates FR-011).

## R2. Large editor surface

- **Decision**: a registered dialog (`overlay/registry.rs`, like the rename dialogs) holding `Option<LargeEditor>` in client state; body from `Modal`, `TextArea` (multi-line, `max_height` large), `Button`s.
- **Rationale**: the registry gives modality and focus handling; `Option` makes "no second editor" unrepresentable.
- **Rejected**: an inline expanding composer (not modal, spec requires a modal); a separate OS window (windowing is single-window, cross-platform cost).

## R3. Shortcut handling

- **Decision**: extend `composer_key` (`features/composer.rs`, already special-cases the attachment chord before the non-plain guard) with a `LargeEditorRequested` intent; text-area focus is the existing gate, so an unfocused composer or terminal pane never receives it.
- **Rejected**: an app-level chord (would need focus checks again and could reach terminal panes).

## R4. Unified selectors

- **Decision**: extend `ui/material/menu_select.rs` `MenuSelect` as the single trigger (32 dp, 8 dp radius, XS/SM padding, 1 dp outline-variant, "Label: Value", chevron, min 140 dp, ellipsis; hover/focus/pressed/open/disabled; disabled tooltip) and add a `MenuSelectRow::filter` first row for the model list. The effort control already uses it; model (`Typeahead`/`Button::text`) and permission (`Select`) migrate.
- **Rationale**: Principle VIII; effort is already a MenuSelect, so two migrations rather than three.
- **Rejected**: restyling each of the three locally (drift, the duplication budget); keeping `Typeahead` for model (spec FR-017 requires the same floating list).

## R5. Auto-expand

- **Decision**: the reducer sets `options_open` where it sets a composer notice from a model, effort or permission refusal (`features/composer.rs` notice sites, `permission_prompts` refusal), and persists. Stop session and the permission refusal note render in `status_row`, outside the options row.
- **Rejected**: expanding on any notice (over-expands on attachment drops).

## R6. Icons

- **Decision**: `Icon::Settings` exists (`micold-material/src/icons.rs`). The open-editor glyph reuses an existing expand glyph if one exists, else adds one in the same file; a task confirms.
