# Data model

All in-memory, client side; nothing persisted or sent on the wire.

- **Composer entry (existing)**: gains `picker_open: bool` (private; true between `AttachRequested` accepted and `AttachPickerClosed`/`AttachPicked`). Keyed by session through the entry map, so one session's picker never blocks another's (FR-015 is per session).
- **Admission result (existing, implicit)**: `DroppedFiles { plan: DropPlan{images, paths}, reads, shell, sandbox }` -> chips pushed to `draft.images`, quoted text inserted at caret, refusals joined into `notice`. Identical for both routes.
- **Messages (new)** in `composer::Msg`: `AttachRequested(SessionId)`, `AttachPicked { session, paths: Vec<PathBuf> }`, `AttachPickerClosed(SessionId)`; `KeyIntent::Attach`.
- **`apply_drop(session, files, can_accept)`**: `can_accept == false` -> no change except `notice`.
- **Hover flag (new)**: `App.files_hovering: bool`; set by `FileHovered`, cleared by `FilesHoveredLeft` and `FileDropped`.
- **Transitions**: closed --AttachRequested(accepting)--> open; open --Picked|Closed--> closed; open + AttachRequested -> unchanged (FR-015).

- **Sheet (new)**: `App.shortcuts_open: bool`, `app::Message::ShortcutsToggled(bool)`; `SheetModel` groups `Vec<(title, rows)>`; App command `/shortcuts` (`SendIntent::ShowShortcuts`).
