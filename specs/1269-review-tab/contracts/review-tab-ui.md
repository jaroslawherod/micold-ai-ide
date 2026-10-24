# Contract: the Review tab in the client (`features::changes`, `app.rs`, `features::session_strip`, `ui/session_area.rs`)

Each U rule names its test layer. Client unit tests sit beside the code (`#[cfg(test)]` in the
render-free module); integration tests drive `micold_client::app::State::update` from
`crates/micold-client/tests/`. Core rules T1–T8: [review-tabs.md](review-tabs.md). Decisions:
[../research.md](../research.md).

## State and routing (milestones M2 and M3)

U3's other-location and sessionless cases, U7's follow of `shown_location()` with `remember`, U8
and U16's header are M3; the rest is M2.

### U1. The map replaces `open` (FR-002, FR-006, US1.3, US1.6)

`features::changes::State.open: Option<OpenView>` becomes `tabs: ReviewTabs<OpenView>`. Every
message of the view acts on `tabs.shown_mut()`; with nothing shown it does nothing, as a closed view
does today. Test (`tests/features_changes.rs`): open `(P, W)`, select a file, scroll list and diff,
reveal a gap, toggle whole file, toggle Committed, open the composer and type, pick lines, hide the
panel, choose a pull request source, browse a file; `SessionSelected`; `Opened (P, W)` again: every
one of those fields equals its value before (`OpenView` compared whole, minus `notice`). The same
with a second tab `(P, X)` opened and edited in between: neither view holds the other's file,
comments or picks (US1.6).

### U2. Session selection hides, never drops (FR-004, FR-005, SC-002)

`Msg::SessionSelected` calls `ReviewTabs::hide`. Tests: the existing
`tests/features_changes.rs:261` (which asserts the view is dropped) flips to assert it is kept and
hidden; `Outcome::ChangesClosedForCompare` hides; opening and closing the Overview and Settings keep
`tabs` equal (`State::update` sequence).

### U3. Open or show from every entry point (FR-002, FR-003, US2.1, US2.2, US2.5, US2.6)

`Outcome::ChangesRequested(location)` (`app.rs`): close the Overview (today); when `location` is not
`shown_location()`, `set_current_session(tabs.session_for(..))` (no `SessionMsg::Selected`); then
`Msg::Opened` → `open_or_show`. `Created` reads as today's first open (`request_read`); `Shown
{ reread: true }` settles in-flight loads (U5) and re-reads once; `Shown { reread: false }` and
`AlreadyShown` return `Effect::None`. Integration tests: repeated opens from the Changes button and
**Review changes** leave `tabs.len() == 1` for the location (SC-005); **Review changes** on another
location's worktree makes it `shown_location()` with its last-shown, else main, session current; on a
sessionless worktree, `session.active == None` and `shown_location() == Some(that worktree)`.

### U4. × (FR-007, US3.1, US3.2)

`Msg::TabClosed { project, entry }` → `ReviewTabs::close`; when `was_shown`, the root sets the current
session to `show` and calls `focus_terminal`, as a session press does. `reviews` and `remembered` are
untouched. Integration test: after ×, `Opened` again starts a fresh `OpenView` (toggles default,
nothing selected) while the location's comments and remembered pull request are still there. In M2
the view's **Close** control sends `TabClosed` for the shown key; M4 removes it (U13).

### U5. Hidden reads (spec edge case *A read in flight*, SC-003)

A `ListRead`/`DiffRead`/`PullRequestsRead`/`PullRequestDiffRead`/`FileRead`/`TreeRead` whose `seq`
belongs to a hidden view changes nothing in any view. Hiding a view with a load in flight sets it
`Unwatched`; showing it settles every `Loading` (to its `last`, else `Idle`) and then issues exactly
one `ReadList`. Client unit test in `features/changes.rs` counts effects: zero read effects between
hide and show, whatever `Changed` messages arrive; exactly one `ReadList` on a show with the mark.

### U6. Changed (FR-010, FR-011, US4.3–US4.6)

`Msg::Changed` (the shown location's watch): the shown review re-reads its list (today) and the
header re-reads its counts (today); a hidden tab of the header's location is marked
(`files_changed` → `Marked`) and no read is issued for it. `ReviewChanged` for a hidden tab's entry
calls `comments_changed`. Unit tests through `update`.

### U7. Follow step (FR-003, FR-005, US1.4, US1.5, R5)

`State::follow_header` follows `shown_location()`; it `remember`s the current session's location,
calls `location_left` for the location it stops following, `hide`s a shown review of a project that
is not active, and marks the shown review `Unwatched` while the Overview, a Settings draft or Compare
covers it (`uncovered` re-reads it once). Integration tests: switch to a session of `X` and back to
`W` (sidebar, session tab, chord, quick switcher messages): `W`'s tab and view kept, a re-read issued
only when `W`'s tab is next shown; switch project `P → Q → P`: `P`'s tabs and views kept, none shown.

### U8. Last session closed (edge case, R8)

Closing the current session when its location has a Review tab and no live session left shows that
Review tab; with other sessions left, today's behaviour (no current session) and the tab remains
hidden. Integration test through `SessionsClosed`.

### U9. Worktree and project removal (FR-008, US3.3)

`WorktreesListed` calls `drop_gone_worktrees` for the active project; `ProjectMsg::ForgetConfirmed`
calls `drop_project`. Integration tests: the removed worktree's tab is gone, a sibling's and another
project's stay.

### U10. Focus, chords, attention (FR-013, FR-014, SC-004)

`review_shown()` replaces `changes.open.is_some()` in `terminal_focused`, `view_facts`,
`session_view_visible` and `main_content_key`. Tests: `tests/terminal_focus.rs` and
`tests/attention_view_report.rs` cases that open the Changes view keep their expectations with the
Review tab shown, and gain: with the tab hidden (a session tab shown) the terminal is focused and the
main area is not taken; `tests/keyboard.rs`, `tests/navigation.rs` and the lib `app_chord_tests` stay
green unchanged, plus one case per session chord with the Review tab shown asserting the session is
shown and the tab kept.

### U11. The watch (R13, SC-003)

`State::review_watch_entry()` is the shown review's entry while not covered, else `None`;
`header_watch_entry` excludes the shown review's entry (one watcher per worktree). Unit tests in
`app.rs`; `tests/idle_subscriptions.rs` keeps "no watch while nothing is shown".

### U12. The Changes button (FR-012, US2.3, US2.4)

`ui/session_area.rs::changes_press` answers `(ReviewChangesRequested(location), selected)` with
`selected` and the tooltip from `ChangesButton::of`. The existing test
`the_changes_press_opens_the_shown_location_or_closes_its_view` becomes: never `Msg::Closed`;
selected exactly while the location's tab is shown; a press while shown leaves `tabs` and the shown
tab equal.

## The strip (milestone M4)

### U13. Members (FR-001, D4, R10)

`StripModel::of` builds from `shown_location()`:

- Location with sessions and a tab, session `M` shown with 2 shells:
  `[Session(M, shown, marked), Shell(M,1), Shell(M,2), Plus, Session(H1), Review(hidden)]`.
- The Review tab shown: the same members, `Review(shown, marked)`, no session or shell member
  marked; `Session(M).shown` stays true (it is the current session).
- Sessionless location with a tab: `[Review(shown, marked)]`, `status` empty, `restart` `None`.
- No tab: today's members exactly (1126 S1 tests unchanged).

`Member::Review` carries `face: ReviewTabFace` (T8) with `drafts` = pending comments of
`reviews[(project, entry_dir(entry))]` (`is_pending`), `None` while that entry's comments are
unknown. The view's **Close** control is gone from `ui/changes.rs`. Client unit tests in
`features/session_strip.rs`.

### U14. Presses (FR-015, US5.4)

| Member part | Press |
|---|---|
| Review tab | `Sidebar(ReviewChangesRequested(location))` |
| its × | `Changes(TabClosed { project, entry })` |

`MemberKey::Review` (a unit variant: one Review member per strip); the keyed row keeps a press in flight on its member when a badge
or mark appears or a tab opens or closes (`tests/session_area_stability.rs`, new cases).

### U15. Layout (FR-015, SC-006, edge case *Many tabs*)

`Member::Review.width() == tab::REVIEW_WIDTH`, fixed whatever the badge (`1`, `99+`) or mark;
`member_offsets` and `scroll_into_view` include it; showing it scrolls it into view. While it is shown
the status cluster and split buttons keep their place, disabled. Geometry gates
(`tests/gates/session_area_fits.rs`, `tests/gates/tab_children_fit.rs`,
`tests/gates/strip_controls_hold_their_size.rs`) gain the Review member at 640×480 and 1280×800, light
and dark: label, badge, mark and × whole, nothing overlapping, nothing clipped.

### U16. Header and strip for every shown location (FR-001, US2.6, clarification 2)

`ui::view_with`'s main area: Review shown → the header of `shown_location()` over the Changes view
(`session_area::over_review`, replacing `over_changes`); the review is never drawn without its own
location's header. In M3 it is header-only, as `over_changes` is today (`StripModel::of` still
starts from `session.active`); in M4 `over_review` adds the strip, built from `shown_location()`
with `Member::Review` (U13), the sessionless strip included. Covered states: `session-area-review-shown`, `session-area-review-hidden` (badge
`3`, changed mark), `session-area-review-sessionless`.
