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
- **Decision**: `Ctrl+Shift+A` / `Cmd+Shift+A`, composer-scoped via `KeyIntent::Attach`, checked in `composer_key` before its non-plain guard.
- **Rationale**: not among the reserved chords (D H W O T E F P) nor composer keys; no Linux input method binds it; A = attach.
- **Rejected**: `Ctrl+Shift+U` (GTK/IBus Unicode-entry chord on Linux, would break FR-004 there); plain `Ctrl+O` (terminal/readline semantics); `Alt+` chords (non-uniform on macOS).

## R6 Disabled vs hidden
- **Decision**: disabled with the standard disabled look, tooltip unchanged.
- **Rationale**: no layout shift (spec Assumption).

## R7 Delivery of drops
- **Decision**: treat as test-and-fix (D6); `SplitView::update` already sees `FileDropped` before children.
- **Rationale**: FR-004 says a platform failure is a defect to fix here; cannot be known without running on each platform.

### R7 findings (M1, T001 and T007)
- **Baseline (T001)**: the three targets of the milestone's Verify command ran green on this branch with the M1 changes (the guard, its tests and the characterisation tests); the drop route they exercise was unchanged apart from the new `can_accept` argument.
- **Drop route today**: window `FileDropped(path)` (one event per file) -> `SplitView::update` (resolves the pane under the pointer) -> `PaneMsg::FileDropped(pane, path)` -> `shell/drops.rs::on_file_dropped` (collects, 40 ms settle) -> `on_drops_settled` -> `read_for_composer` (sniff, read images off the UI thread) -> `composer::Msg::Dropped` -> `shell/composer.rs` computes `can_accept` -> `State::apply_drop(session, files, can_accept)`.
- **Delivery (T007)**: `SplitView::update` handles `FileDropped` before it forwards the event to any child and never captures it, and `ui/panes.rs` renders a structured pane (transcript and composer) as a child of that `SplitView`, so neither widget can swallow the event. No defect found in code. Wayland, X11, macOS and Windows all deliver the same iced `window::Event::FileDropped`; none could be exercised on real hardware here, so the per-platform cells below are code-reading only and a real-desktop drop stays a manual check.

| Platform | Finding |
|---|---|
| Linux Wayland | event path as above; not run on a real compositor |
| Linux X11 | event path as above; not run on a real display |
| macOS | event path as above; not run |
| Windows | event path as above; not run |
