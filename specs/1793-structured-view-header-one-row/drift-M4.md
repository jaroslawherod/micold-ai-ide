# Plan drift, M4

- crates/micold-client/src/ui/session_area.rs: the strip is a 40 px row (`ROW_HEIGHT`), the title row, `running` text and status cluster are gone; the `+` menu anchor is `plus_anchor` (bar bottom edge plus one row, x clamped into the tab region so a scrolled-out `+` hangs from its edge).
- crates/micold-client/src/features/session_strip.rs: `PLUS_WIDTH` is `ROW_HEIGHT`; per-tab activity dot and the `N hidden` chip (`ShowRawEvents(true)`).
- crates/micold-client/src/ui/panes.rs: the split buttons take the row's height as touch target (`split_buttons_in_row`).
- tests/gates: `session_bar_fits.rs` retired (T035), `header_row_fits.rs` added, `session_area_fits.rs`, `strip_controls_*`, `containment.rs` (transcript and strip paths shift one level, the transcript overflow exemptions lose the session bar), `review_tab_fits.rs` (smallest-window exemptions that no longer overflow dropped) re-pointed; tests/fixtures/layout_snapshot.txt regenerated (strip 40 px, content starts at y 84).
- tests/support/covered_states.rs: anchors `session.header*` and `session.strip.status` removed; `SESSION_STRIP_SPLIT` is the controls group's first child, `SESSION_STRIP_SPLIT_AFTER_RESTART` the second.
- tests/settings_sections.rs: `show_raw_events` is chosen by the strip chip; tests/terminal_size_reporting.rs, terminal_tabs.rs, transcript_insights_view.rs, layout_text_overflow.rs follow the one-row header.
- docs/user-guide/structured-transcript.md, worktrees-and-sessions.md: the one-row header, chip, dot, narrow windows.
- FR-011 (tabs shrink before they scroll) is NOT done: tabs keep their fixed width and the strip scrolls; the `+` and chip stay reachable. Recorded as a follow-up in the ledger.
