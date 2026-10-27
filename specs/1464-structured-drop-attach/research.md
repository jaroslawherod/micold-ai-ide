# Research: Structured drop and Add attachment

## R1 File picker
- **Decision**: `rfd` async `AsyncFileDialog::pick_files`, features `xdg-portal` + `tokio` (Linux), default elsewhere; run inside `Task::future`.
- **Rationale**: only mainstream cross-platform native picker with async multi-select; the portal backend needs no GTK system library and works on Wayland and X11; MIT/Apache, maintained. The client already runs on tokio via iced.
- **Rejected**: `native-dialog` (blocking, would need a thread and gives no better portal story); `gtk` backend of rfd (system GTK dependency in packaging); an in-app iced file browser (large new UI, off-platform look, violates "system file picker"); `iced` has no dialog API.

## R2 Dialog parent window
- **Decision**: start parentless; add the main-window handle (iced `window::run` + `set_parent`) only if a platform shows the dialog behind the window.
- **Rationale**: keeps the first milestone small; macOS/Windows show dialogs on top regardless. Manual matrix in quickstart decides.
- **Rejected**: always parenting (raw-window-handle plumbing for no proven gain).

## R3 One admission path
- **Decision**: picker reuses `read_for_composer` -> `Msg::Dropped` -> `apply_drop`.
- **Rationale**: FR-008/SC-001 hold by construction; one place to add the can-accept guard.
- **Rejected**: separate picker reducer (two paths to keep equal, duplication).

## R4 Drop cue without pointer position
- **Decision**: window-level `files_hovering` flag from `FileHovered`/`FilesHoveredLeft`/`FileDropped`; cue on every input-accepting structured pane.
- **Rationale**: iced gives no position during an OS drag (spec Assumptions); emulating it would use a stale pointer and mislead. Best-effort per clarification.
- **Rejected**: cue on the pane under the last pointer (often stale, wrong pane); no cue (US2 unmet where hover is reported).

## R5 Shortcut
- **Decision**: `Ctrl+Shift+U` / `Cmd+Shift+U` ("upload"), composer-scoped via `KeyIntent::Attach`.
- **Rationale**: not among the reserved chords (D H W O T E F P) nor composer keys; U is the familiar upload mnemonic.
- **Rejected**: plain `Ctrl+O` (clashes with terminal/readline semantics, spec 941 composer keys), `Alt+` chords (non-uniform on macOS).

## R6 Disabled vs hidden
- **Decision**: disabled with the standard disabled look, tooltip unchanged.
- **Rationale**: no layout shift (spec Assumption).

## R7 Delivery of drops
- **Decision**: treat as test-and-fix (D6); `SplitView::update` already sees `FileDropped` before children.
- **Rationale**: FR-004 says a platform failure is a defect to fix here; cannot be known without running on each platform.
