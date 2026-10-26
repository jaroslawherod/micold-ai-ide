# Data model: Compact session header

No new or changed data. Derived view inputs only:

- **Switch slot** (view-local, not stored): `Option<(caption: Option<String>, entry: SwitchEntry)>`, from `State::held_queue_line(session)` and `features::session::switch_entries`. `None` for tiled panes, a transcript, a non-terminal session, a CLI without a switch, or while the Review tab is shown.
- **Header text**: `micold_core::context_header::HeaderText` unchanged.
- **StripModel**: unchanged.
