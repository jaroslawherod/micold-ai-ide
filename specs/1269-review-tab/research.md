# Research: Review as a per-worktree tab that keeps its state (#1269)

Each decision names what was chosen, why, and what was rejected. Paths are workspace-relative;
`client/` is `crates/micold-client/`, `core/` is `crates/micold-core/`. Spec sections are cited as
`spec.md FR-nnn` or `US<n>.<m>` (story, acceptance scenario).

## R1. Where the tab rules live: a generic `micold_core::review_tabs`

**Decision**: A new render-free core module `core/src/review_tabs.rs` (`pub mod review_tabs` in
`core/src/lib.rs`) holds every rule FR-016 names: the open-tabs map keyed by entry, which Review tab
is the shown tab, open-or-show, hide, close (and which session to show after), the drop rules
(worktree removed, project removed, nothing else), the freshness state machine behind the changed
mark, the last-shown session per location, and the words and marks of the tab and the Changes
button. It is generic over the kept view: `ReviewTabs<V>`. The client instantiates it with its
`features::changes::OpenView`; the core tests use a plain `u32` or `String` as `V`.

**Rationale**: FR-016 and the issue's acceptance ("pure logic in micold-core with unit tests") ask
for these rules to be testable with no window. `OpenView` is a client type (it holds `Load<…>`,
`Shaped`, `FilesPane`, all in `client/src/features/changes.rs`), so a concrete map of it cannot live
in core; a generic container can, and its rules never look inside `V`. The module names nothing of
#1125's sidebar or #1127's title bar, so it can merge first (milestone M1) while those are unmerged.

**Alternatives rejected**:
- Keep the map and rules in `client/src/features/changes.rs`: render-free, but the file is 4,179
  lines already, and the issue asks for core. Its tests run only in the client build.
- Move `OpenView` and its parts to core: drags the diff shaping, browse stack and PR list types with
  it, a refactor out of this slice's scope (spec *Out of scope*: no change to diff reading).
- A trait `KeptView` that core calls into (for in-flight reads): the rules need nothing from the
  view; the client settles in-flight reads itself (R6).

## R2. The key, and what "shown" means

**Decision**: The key is `EntryKey = (PathBuf, SessionLocation)`, the same pair
`features::changes::State::remembered` already uses (`client/src/features/changes.rs:1181`).
`ReviewTabs` holds `shown: Option<EntryKey>`, invariant: `shown` names a key in the map. "The
Review tab is the shown tab" is `shown == Some(key)` **and** `key.0` is the window's active project
(`State::review_shown()` in the client); a shown key of another project is hidden by the follow step
(R5). There is no per-location "shown tab" table: a session selection always shows that session
(D1), so the only per-location memory needed is which session to return to (R4).

**Rationale**: one window-level `shown` makes "exactly one shown tab" structural (Principle V): a
location shows either the Review tab (the key is `shown`) or the current session; nothing else can
be recorded. `review-redesign.md` §0 asks for "the map plus the active entry's tab selection" — this
is that selection.

**Alternatives rejected**:
- `BTreeMap<EntryKey, ShownTab>` per location: invites two locations both "showing" their Review
  tab, and a returning location reopening its Review tab, which US1.4 does not ask for (coming back
  to A by a session selection shows that session).
- Key by the wire's `worktree_dir` string like `reviews`: loses the Default/worktree distinction the
  rest of the view keys on.

## R3. Freshness: the changed mark and the lazy re-read

**Decision**: Each tab carries `Freshness`:

- `Fresh`: its last read is current as far as this window knows.
- `Changed`: the location's files changed while the location was shown and the tab hidden; the tab
  draws the changed mark (FR-010).
- `Unwatched`: no watch saw the location while the tab was hidden or covered (its location was not
  the shown location, the Overview, Settings or Compare covered it, a read was in flight when it was
  hidden, or its comments changed while hidden); no mark, since nothing is known to have changed,
  but it is re-read when shown (FR-011, US4.5).

Transitions are the core rules T5 ([contracts/review-tabs.md](contracts/review-tabs.md)): a files
change on the shown location re-reads a shown tab and marks a hidden one `Changed`; leaving or
covering a location turns `Fresh` into `Unwatched` (`Changed` stays); showing a `Changed` or
`Unwatched` tab answers `reread: true` once and sets `Fresh`.

**Rationale**: D3 (ledger) — only the shown location is watched (feature 1126 R6
`State::header_watch_entry`, `client/src/app.rs:1349`), so the window can only *know* of changes on
that location; elsewhere it can only know it was not looking. Two states keep the mark honest (no
mark without an observed change) and the re-read complete (SC-003: exactly one read on show).

**Alternatives rejected**:
- One `stale: bool`: either marks tabs that did not change, or skips the re-read FR-011 requires
  for a location that was not watched.
- Watch every location with a Review tab: D3 rejected it (a recursive watch of entries nobody looks
  at wakes the window on every build, feature 482 R9).
- Re-read hidden tabs in the background: FR-010 forbids reading for a hidden tab.

## R4. Which session a location shows: last-shown, else main

**Decision**: `ReviewTabs` also keeps `last_session: BTreeMap<EntryKey, SessionId>`, written by the
client's follow step whenever a session of a location is the current session (`remember`).
`session_for(key, live, main)` answers the last-shown session if it is still live, else `main`, else
`None` (a sessionless location). It serves FR-003 (**Review changes** on another location) and
FR-007 (× shows "the session shown before, else main"): a tab created while a session of its
location was current also records that session as its `return_to`.

**Rationale**: nothing in the client records a per-location last session today (searched
`last_shown`, `last_selected`, `recent_session`: no match; `workspace.foreground_by_project` is per
project, `client/src/features/session.rs:1054`). The main session is already derived by
`features::navigation::location_node(..).main_session()` (used by `StripModel::of`,
`client/src/features/session_strip.rs:347`), so core takes it as an argument and stays free of the
sidebar tree.

**Alternatives rejected**:
- Always show the main session: contradicts the clarification (US2.5 "its last-shown, else main").
- Store it on `features::sidebar` state: that file is #1125's territory, being rebuilt in parallel.

## R5. The shown location, and sessionless locations

**Decision**: A new render-free `app::State::shown_location()` answers the shown review's location
when `review_shown()`, else `current_session_location()` (`client/src/features/sidebar.rs:1493`).
`follow_header` (`client/src/app.rs:1311`) follows `shown_location()` instead of
`current_session_location()`, and also feeds `ReviewTabs`: `remember` the current session's
location, `location_left` for the location it stops following, `hide` when the shown key's project is
not the active project (project switch, US1.5). The header and the strip are drawn for
`shown_location()`, so a sessionless worktree whose Review tab is shown gets its own header and a
strip of only its Review tab and **New session** (US2.6, clarification 2).

**Rationale**: the follow step already runs once after every outermost message (feature 1126 R6), so
every route that changes what is shown is covered without a call per site. Today the Changes view of
a location other than the current session's is drawn under the *current session's* header
(`ui/session_area.rs::over_changes` uses `current_session_location()`), which clarification 2
forbids.

**Alternatives rejected**:
- Keep `session.active` and add a separate "shown location" field: two writable facts that can
  disagree; `shown_location()` derives one from the other.
- Select some session of another location for a sessionless review: the location has none.

## R6. Routing messages to the right view; reads in flight

**Decision**: Every message of the Changes view acts on the **shown** view only:
`state.open.as_mut()` becomes `state.tabs.shown_mut()` (a borrow of the `tabs` field alone, so the
existing `&mut state.next_seq` beside it still compiles). Read results carry a global `seq`
(`State::next_seq`); the shown view's existing `seq == current` checks drop any other view's result,
so a hidden view's result is discarded, never applied to another location's view (spec edge case
*A read in flight*). A view hidden while a read was in flight is `Unwatched` (R3); when shown, the
client first settles its in-flight loads (`Load::Loading { last, .. }` → its `last` or `Idle`, the
same for `pull_requests`, `browse`, `files.tree` and a PR's `refresh`), then issues one re-read.
No effect is ever issued for a hidden view.

**Rationale**: the least change to the ~89 `state.open` uses in `features/changes.rs`, and to the
shell (`shell/changes.rs`, `shell/pr_status.rs`), which already ask "is this the current read" before
logging (`diff_read_is_current`). Applying hidden results would need every arm to find its view by
seq; discarding is allowed by the spec and the re-read on show restores the state.

**Alternatives rejected**:
- Route each result to the view whose load awaits that seq: correct but touches every result arm,
  and a list result would then start a diff read for a hidden view, which FR-010 forbids.
- Per-view `next_seq`: seqs would collide across views and the shell's staleness logs would lie.

## R7. Session selection hides; × and worktree removal drop

**Decision**: `Msg::SessionSelected` (sent by `main.rs` on `SessionMsg::Selected`,
`client/src/main.rs:1307`, and by `Outcome::ChangesClosedForCompare`, `client/src/app.rs:1555`)
becomes "hide the shown review": `ReviewTabs::hide`. `Msg::Closed` becomes
`Msg::TabClosed { project, entry }` (×), which drops that tab and, when it was shown, makes the root
show `session_for` (R4) through `set_current_session` and give the terminal the keyboard as a session
press does. `Msg::WorktreesListed(names)` drops tabs of the active project's gone worktrees
(`drop_gone_worktrees`); `ProjectMsg::ForgetConfirmed` drops the project's tabs (`drop_project`).
Nothing else removes a tab (SC-002, core property test T4).

**Rationale**: spec FR-005, FR-007, FR-008; the acceptance line "today's `Msg::Closed |
SessionSelected` no longer drops the view". `WorktreesReplaced` already reaches every window
(feature 482 V3), so FR-008's "in every window" holds without new wiring.

**Alternatives rejected**: closing the view when the last session of its location closes — the spec's
edge case keeps it (R8).

## R8. The last session of a location closes

**Decision**: When the current session closes (`features::session::closed`,
`client/src/features/session.rs:1771`) and its location has a Review tab and no live session left,
that Review tab becomes the shown tab (`ReviewTabs::show_if_present`), so the location stays shown
with its header and a strip of Review + **New session**. With sessions left, today's behaviour stands
(no current session); the tab remains (US3.4).

**Rationale**: spec edge case *Location's last session closed*. Without it, the location would
vanish from the main area and its kept view would be unreachable except from the sidebar.

**Alternatives rejected**:
- Leave the location with nothing shown (today's `set_current_session(None)`): the spec's edge case
  keeps the location shown with its header and strip.
- Close the tab with the last session: the tab is dropped only by × or worktree removal (FR-007,
  FR-008).

## R9. The Changes button and the Review tab press

**Decision**: Both send `Message::Sidebar(sidebar::Msg::ReviewChangesRequested(location))`, as the
button does today (`ui/session_area.rs::changes_press`), so all three entry points meet in
`Outcome::ChangesRequested` (`client/src/app.rs:1542`), which now: closes the Overview (as today);
when the location is not `shown_location()`, sets the current session to `session_for` (R4) directly
through `set_current_session` (not through `SessionMsg::Selected`, which would hide the review);
then `ReviewTabs::open_or_show`. While the Review tab is shown the button press is a no-op (T8): the
reducer answers `AlreadyShown` and changes nothing (FR-012, clarification 1). The button's words come
from core `ChangesButton::of` (T8): selected iff shown; tooltip *Open the review* / *Show the review*
/ *The review is shown*, followed by today's counts line.

**Rationale**: one path means one place for FR-002 (no duplicate) and FR-003. Clarification 1
replaces 1126 FR-007's close-on-second-press.

**Alternatives rejected**: a new `Msg::TabShown(key)` for the strip press — a second path to the same
rule; it would also need the "other location" handling duplicated.

## R10. The Review tab in the strip

**Decision**: `features::session_strip::Member` gains `Review { project, entry, face, shown }` and
`MemberKey::Review`; `StripModel::of` builds the strip from `shown_location()` rather than
`session.active`, appends the Review member after the last session/shell/"+" member when the location
has a tab, and for a sessionless location yields `[Review]` (no status, no restart). "Exactly one
marked member" (1126 S2) extends: when the Review tab is shown it is the marked member and no session
or shell member is marked. The press sends R9's message; the × sends `Msg::TabClosed`. Its width is a
fixed `tab::REVIEW_WIDTH` (new, derived like `tab::SESSION_WIDTH`, `ui/material/tab.rs:124`) that
holds "Review", a badge of up to "99+", the dot and the × at their widest, so badge and mark changes
never move a neighbour (FR-015) and `member_offsets` stays derived (1126 S6). While the Review tab is
shown, the status cluster and the split buttons stay in place, disabled (they act on panes that are
not drawn), so the strip's child list never varies (1126 FR-011).

**Rationale**: spec FR-001 (after session and shell tabs, before **New session**: New session is
outside the scrolling members already), FR-015, D4. The keyed row (`ui/cdk/keyed.rs`, 1126 R10)
keeps a press in flight on its member when the badge or mark appears.

**Alternatives rejected**:
- Content-sized Review tab: its width would change with the badge, moving nothing else today (it is
  last) but breaking the derived offsets and the reveal arithmetic.
- Hiding the split buttons while Review is shown: varies the child list (1126 FR-011 lost presses).

## R11. Components: count badge and changed mark

**Decision**: The tab is the shared `material::Tab` (`ui/material/tab.rs:168`) with `.trailing(..)`
holding the badge, the mark and the ×. The draft count is a new shared `material::CountBadge`
(`ui/material/count_badge.rs`, new): a filled pill in `primary`/`on_primary`, label role, `99+` cap,
builder `CountBadge::new(count, roles).tooltip(..)` ending in `.into()`; the section list's badge
chip (`ui/material/section_list.rs`, `badge_accent`) moves onto it where the look is the same. The
changed mark is the shared `material::UnreadMark::new(roles)` dot (`ui/material/unread_mark.rs`), the
same "you have not looked since" meaning it has for sessions. Both appear in the showcase.

**Rationale**: Principle VIII (reuse, promote rather than fork); no filled count badge component
exists (`ActivityBadge` is an activity signal, `Button::trailing_mark` draws an unread dot and a plain
number inside a button).

**Alternatives rejected**: drawing the badge inside `ui/session_area.rs` (a one-off widget);
`ActivityBadge` (wrong meaning, no number).

## R12. Focus, chords and attention

**Decision**: Every place that asks `changes.open.is_some()` / `is_none()` asks
`review_shown()` instead: `State::terminal_focused` (`client/src/app.rs:619`), `State::view_facts`'s
`main_area_taken` (`client/src/app.rs:653`), `ui::session_view_visible` (`ui/mod.rs:329`),
`ui::main_content_key` (`ui/mod.rs:1282`), and the main-area branch in `ui::view_with`
(`ui/mod.rs:426`). A session chord sends `SessionMsg::Selected`, which now hides the review (R7);
no chord handler changes otherwise.

**Rationale**: FR-013, FR-014: a shown Review tab counts as the open Changes view does today; a shown
session tab as the terminal does. Keeping the predicates' shape keeps chords and focus identical.

**Alternatives rejected**: a separate "review focused" flag beside `tabs.shown` — a second fact that
can disagree with what is drawn, the defect class 1126 S2 removed for the strip.

## R13. The watch

**Decision**: The view's own watch (`shell/subscriptions.rs:152`) runs only for the shown review and
only while it is not covered (Overview, Settings draft, Compare): `State::review_watch_entry()`.
`header_watch_entry` keeps its rule, comparing against the shown review rather than `open`. One
`Msg::Changed` means "the shown location changed": it re-reads the shown review's list (today), the
header counts (today), and marks a hidden Review tab of the header's location `Changed` (T5). While
covered, the follow step marks the shown review `Unwatched` (R3), so uncovering re-reads it once.

**Rationale**: D3; SC-003 (no read while hidden or covered); no new watcher.

**Alternatives rejected**: watching every location with a tab, or one watcher per tab — D3 (a
recursive watch of entries nobody looks at wakes the window on every build, feature 482 R9).

## R14. Docs, layout snapshot, visual pass

**Decision**: M2 updates `docs/user-guide/reviewing-changes.md` (*Opening the Changes view*: kept
view, the button shows it, *Keeping up with changes*: re-read when shown). M3 adds **Review changes** on another or a sessionless location. M4 updates it again for the
tab (badge, changed mark, ×), `docs/user-guide/terminal-panes.md` (the strip's Review tab, chords
with it shown) and `docs/user-guide/worktrees-and-sessions.md` where it describes the strip;
`docs/development/layout-snapshot.md` *What is covered*; covered states
`session-area-review-shown`, `session-area-review-hidden` (badge and changed mark),
`session-area-review-sessionless` replace `session-area-changes-open`
(`tests/support/covered_states.rs:1611`); fixture regenerated; visual pass at 1280×800 both themes
and at the minimum window (quickstart §B).

**Rationale**: spec FR-017, Principle VII; the 1126 pattern.

**Alternatives rejected**: regenerating the fixture in M2 — M2 adds no covered state and changes no
geometry the snapshot records (the Changes button keeps its size; `selected` is a layer), so a
regeneration there would only record another thread's drift; all docs in M4 only — M2 changes what a
switch does, which Principle VII asks to document in the same change.
