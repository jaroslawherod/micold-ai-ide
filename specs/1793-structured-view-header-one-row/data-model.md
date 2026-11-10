# Data model: feature 1793

All render-free, in `micold-core::header_bar` unless noted; none persisted.

- **HeaderBarModel** (`Option` = None on Overview): `meter: MeterText`, `status: Status`, `changes: ChangesButton` (existing `micold_core::review_tabs`), `worktree_menu: WorktreeMenuModel`. Built from the selected `SessionId`/worktree; reads that session only.
- **MeterText**: `ratio: Option<f32>` (None = empty state), `text: String` (`8% · $1.28`), `tooltip: String` (tokens used/total, cost, turns; "no usage yet" when unavailable), `accessible: String` (tooltip content, carries cost), `warning: bool`. Source: `meter_view`/`session_bar_view` in the existing insights view model.
- **Status**: `Idle | Working` per session (existing `session_status`).
- **Collapse**: `ring_text: bool, changes_label: bool` computed from available width and fixed rank order ring text then label.
- **WorktreeMenuModel**: `show_raw: bool`, `view: Option<(SessionKind, SessionKind)>` (current, other; None for non-Claude CLIs).
- **PlusMenuModel** (`session_strip.rs`): items `AnotherSession(SessionLocation)`, `Terminal`.
- **HiddenChip**: `Option<String>` text `N hidden` from `shown::hidden_text` count; None at zero.
- **HeaderMenu** (client state): `None | Plus | Worktree`; reset on selection change.
