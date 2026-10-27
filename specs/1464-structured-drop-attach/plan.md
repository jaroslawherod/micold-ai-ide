# Implementation Plan: Structured View — Drop Files and "Add attachment" Button

**Branch**: `feat/1464_structured-view-drag-and-drop-files-into-the` | **Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1464-structured-drop-attach/spec.md`

## Summary

Spec 941 already routes a file dropped on a structured pane to its composer
(`shell/drops.rs::read_for_composer` -> `composer::Msg::Dropped` -> `State::apply_drop`). This
feature (1) makes that route the single **admission path** and feeds it from a second source, a
native multi-file picker opened by a new **Add attachment** icon button in the composer row and a
shortcut; (2) refuses a drop or picker result for a session that cannot accept input, with a
notice; (3) shows a drop-target cue while files hover, where the OS reports it; (4) checks and, if
needed, fixes drop delivery on Linux (Wayland, X11), macOS, Windows. The picker is `rfd`'s async
dialog, run as an `iced::Task`, so the UI never blocks. See [research.md](research.md).

## Technical Context

**Language/Version**: Rust stable (mise-managed), edition of the workspace.

**Primary Dependencies**: iced 0.14 (`window::Event::{FileDropped, FileHovered, FilesHoveredLeft}`);
new `rfd` (async `AsyncFileDialog`, `xdg-portal` + `tokio` features on Linux; MIT, maintained) in
`micold-client` only; existing `micold_core::structured::attachment` (`classify_drop`, `admit`).

**Storage**: N/A. Picker-open flag and hover flag are in-memory UI state; nothing persisted.

**Testing**: `cargo test` core units; client reducer tests (`tests/features_composer.rs`,
`tests/features_session.rs`, `tests/pane_drop_target.rs`); geometry/layout gate
(`tests/layout_snapshot.rs`); showcase + quickstart §B visual pass. Native dialog is not driven in
tests: the picker is a seam (`Task` returning paths) replaced by a direct message in tests.

**Target Platform**: Linux (Wayland, X11), macOS, Windows.

**Project Type**: desktop app (iced), crates `micold-core` (render-free), `micold-client`.

**Performance Goals**: UI responds within 100 ms while the picker is open (SC-004); cue gone by next
frame (SC-005).

**Constraints**: offline; no new daemon/wire change; spec 941 limits unchanged.

**Scale/Scope**: one composer row control, one reducer path, one shell task, one cue overlay.

## Constitution Check

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Each unit starts with a failing test (see Test strategy); tdd-plan derives the list. |
| II. Multi-Session | PASS | Picker flag and results carry `SessionId`; a late result for a closed/ended session is refused (FR-012). Cue/flag is window state, no session state shared. |
| III. Worktree Integration | PASS | Paths quoted for the session's sandbox share exactly as for a drop; no VCS operation added. |
| IV. Local-First | PASS | Local picker and files only; no network. |
| V. Rust + iced | PASS | Rust/iced only; `rfd` is a dialog library, not a GUI framework. "Picker open" is an `Option`-style state on the composer entry, not a bool pair. |
| VI. Cross-Platform Parity | PASS | rfd covers all three; shortcut uses Ctrl/Cmd; cue best-effort per Clarification (hover not emulated), results identical. CI builds all three. |
| VII. Documentation | PASS | User-guide page for structured input updated in the same change (Docs below). |
| VIII. Reusable UI Components | PASS | Button is `material::IconButton` (builder, `.into()`); cue is a small shared overlay widget wrapper in `ui/material`, not feature-local. |

No violations; Complexity Tracking empty. Re-checked after design: unchanged.

## Design

### D1 Admission path (FR-001..003, 008, 011, 016)
Keep `Msg::Dropped { session, files }` + `apply_drop` as the one admission. Add a reducer guard in
`composer::State`: `apply_drop` takes a `can_accept: bool` (from the session's input state; same
test as `can_send`'s session half) and, when false, applies nothing and sets `notice`
"<n> file(s) not attached: the session is not accepting input". Empty `plan` (no images, no
paths) returns without touching `notice` (FR-016). Text drops never reach it: iced only emits
`FileDropped`. See [contracts/admission.md](contracts/admission.md), [data-model.md](data-model.md).

### D2 Read helper shared by both routes (FR-008, SC-001)
`shell/drops.rs::read_for_composer` is the only reader (sniff, read images, build `DroppedFiles`).
The picker's result goes through it unchanged, so chips, quoted paths and refusals are identical.
Shell/sandbox share are resolved at the call as for a drop.

### D3 Picker (FR-007, 009, 012, 015)
New `shell/attach_picker.rs`: `open(session) -> Task<Message>` runs
`rfd::AsyncFileDialog::new().pick_files().await` (no filter, default location, title
"Add attachment"); maps `None`/empty to `Msg::AttachPickerClosed(session)` and `Some(paths)` to
`Msg::AttachPicked { session, paths }`. Reducer: `AttachRequested(session)` is ignored unless the
composer can accept input and `picker_open` is `None`; it sets `picker_open` and the shell spawns
the task. `AttachPickerClosed` clears it silently; `AttachPicked` clears it, then checks the
session still accepts input (else a notice, FR-012) and hands paths to `read_for_composer`. The
flag is keyed by session, so the result can only land on the requester. The dialog is parentless
on Linux/Windows and parented to the main window on macOS where rfd needs it; if the parent handle
is awkward in iced, parentless is acceptable (research R2).

### D4 Button and shortcut (FR-006, 010)
In `ui/composer.rs` input row, push a `material::IconButton` (new `Icon::Attach`, paperclip,
added to the icon set with its docs in `docs/user-guide/icons.md`) before Stop/Send, with a
tooltip "Add attachment (Ctrl+Shift+U)" / "(Cmd+Shift+U)". Disabled (not hidden) when the
session cannot accept input or `picker_open` is set. Keyboard: the button joins the focus order via
the standard button focus; a composer chord `Ctrl+Shift+U` / `Cmd+Shift+U` is a new
`KeyIntent::Attach` in `composer_key` (checked against `keymap.rs` reserved chords D H W O T E F P
and composer keys; U is free). The shortcut sheet gains a "Composer" section listing it (D7).

### D5 Drop cue (FR-005, SC-005)
`SplitView` also forwards `FileHovered` / `FilesHoveredLeft` / `FileDropped` as
`PaneMsg::FilesHovering(bool)`. App holds `files_hovering: bool` (window-level; the OS gives no
pointer position during a drag, so the cue marks every structured pane that accepts input, not one
pane). `ui/panes.rs` wraps such a pane's body in a small `drop_cue` overlay (outline + tinted fill
from `Roles`, in `ui/material`) while the flag is set; the flag clears on leave and on drop. Where
no hover event arrives, no cue and the drop still works. Terminal panes get no cue (feature 487).

### D6 Drop delivery per platform (FR-004, SC-003)
Verify with the existing `pane_drop_target` tests plus a manual matrix in quickstart §B. Known
risks: iced/winit `FileDropped` on Wayland requires the window to have focus and gives no
position; transcript/composer widgets that capture events could swallow the window event
(`SplitView::update` sees it first, before children). Any failure found is fixed in the same
milestone with a regression test; recorded in the ledger.

### D7 Shortcut sheet (FR-010)
`features/changes/keys.rs::shortcut_sheet` is Review-only and gated on the Review tab
(`ShortcutSheet::open_in`). Extend `SheetModel` with a second group (title "Composer") built from a
`COMPOSER_KEYS` table, and let the sheet open from a structured session via the existing
Keyboard-shortcuts command. Task 1 of that milestone confirms the gate; if opening from a
structured session needs a new palette command, add it (name "Keyboard shortcuts").

## Project Structure

```text
specs/1464-structured-drop-attach/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md
├── contracts/admission.md        # drop/picker admission + messages
└── tasks.md                      # tasks unit
crates/micold-core/src/structured/attachment.rs     # unchanged API; tests for refusal text
crates/micold-client/
├── Cargo.toml (+ rfd), ../../Cargo.toml (workspace dep)
├── src/features/composer.rs      # Msg::{AttachRequested,AttachPicked,AttachPickerClosed}, picker_open, can_accept guard, KeyIntent::Attach
├── src/shell/attach_picker.rs    # NEW: rfd task        src/shell/drops.rs  # read_for_composer pub(crate)
├── src/shell/composer.rs         # route new Msgs        src/shell/panes.rs  # FilesHovering
├── src/ui/composer.rs            # IconButton + tooltip  src/ui/panes.rs     # cue wrap
├── src/ui/material/split_view.rs # forward hover events  src/ui/material/drop_cue.rs # NEW
├── src/ui/shortcut_sheet.rs + features/changes/keys.rs # Composer group
├── src/showcase/sections/*.rs    # button states + cue
└── tests/ (features_composer, pane_drop_target, layout_snapshot)
docs/user-guide/ (structured input page; icons.md)
```

**Structure Decision**: extend the existing composer/drops modules; two new small files (picker
shell task, cue widget). No core or daemon change beyond tests.

## Requirement map

| FR | Section |
|---|---|
| 001, 002, 003 | D1, D2 (existing path kept) |
| 004 | D6 |
| 005 | D5 |
| 006, 010 | D4, D7 |
| 007, 009, 012, 015 | D3 |
| 008 | D2 |
| 011, 016 | D1, D4 |
| 013 | Test strategy |
| 014 | Test strategy (showcase) |

## Test strategy

| Requirement | Layer |
|---|---|
| FR-001..003, 011, 016, SC-001, SC-006 | client reducer tests in `tests/features_composer.rs`: same file set via `Dropped` and via `AttachPicked` give equal draft/notice (FR-013); ended session refused with notice; empty plan no notice |
| FR-007/009/012/015 | reducer tests: `AttachRequested` twice opens one; `AttachPickerClosed` silent; `AttachPicked` after session ended refused; result keyed to requester |
| FR-001 routing, multi-pane | `tests/pane_drop_target.rs` / `features_session.rs` |
| FR-005, SC-005 | reducer test for the hover flag set/clear; showcase for the look |
| FR-006, 010 | `layout_snapshot` geometry gate (row fits, button size), key-intent unit test for the chord, sheet model test |
| FR-004, SC-003, SC-004 | quickstart §B manual matrix (Wayland, X11, macOS, Windows; native picker) |
| FR-014 | showcase section + visual-pass |
| Core | `micold-core` unit tests unchanged; add none unless refusal wording moves |

## Documentation

Update the structured-input user-guide page (create `docs/user-guide/structured-input.md` section
if absent; list in `SUMMARY.md`): drag onto the view, the button, the shortcut, cue best-effort.
`icons.md` lists the new Attach icon.

## Complexity Tracking

None.
