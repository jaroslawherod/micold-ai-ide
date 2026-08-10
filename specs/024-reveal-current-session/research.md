# Research: Reveal the current session in the sidebar

Feature 024. Everything below was checked against the code and against iced 0.14's own sources, not
recalled. Line references are to the tree at the time of writing.

## R1 — Is the mark actually missing, or just unseen?

**Decision**: Unseen. The mark exists; nothing opens the row that carries it.

`session_tree_item` already computes `selected = active_session == Some(session.id)`
(`ui/sidebar.rs:455`) and `TreeView` already draws a selected row as a `secondary_container` pill
with `on_secondary_container` text at the `FULL` corner (`ui/material/tree_view.rs:437-454`). Session
rows are only built inside `if node.expanded` (`ui/sidebar.rs:430`, `:514`), and on a switch
`restore_after_activation` sets `default_expanded = false` (`features/session.rs:99`) while
`set_worktrees` runs `self.expanded.retain(|d| names.contains(d))` (`app.rs:1534`). So after a
switch the incoming project's rows are closed and the pill is not drawn.

**Rationale**: This is why the feature is small. Only the reveal and FR-003a's non-colour cue are
new; FR-002's "exactly one marked" is existing behaviour that needs a test, not an implementation.

**Alternatives considered**: Treating the whole thing as new work, which would have duplicated the
selected-pill path and been rejected by the component-reuse gate.

## R2 — Where does revealed-ness live, given the spec says "derived, not stored"?

**Decision**: `expanded` and `default_expanded` keep meaning *the user's* open set. One new field,
`reveal_suppressed_for: Option<SessionId>`, records that the user closed the revealed row for a
particular current session. A location reads as open when:

```text
user_open(loc) || (loc == location_of(current_session) && reveal_suppressed_for != current_session)
```

**Rationale**: Three requirements fall out rather than being implemented.

- FR-001b (survives late arrival and wholesale replacement): there is nothing stored for pruning to
  drop. `set_worktrees`'s `retain` cannot lose a reveal it never held.
- FR-005 / SC-006 (a close sticks): one comparison, and it is scoped to *that* session, so the next
  reveal is not suppressed by an old collapse.
- FR-007 (does not carry between projects): there is one current session, therefore one revealed
  location. No per-project map can drift out of step with it.

**Alternatives considered**: A per-project `BTreeMap<PathBuf, BTreeSet<String>>` of revealed rows —
rejected, it re-introduces exactly the pruning/ordering problem the derived model removes, and FR-007
becomes a reset to remember rather than a consequence. A single `revealed: Option<SessionLocation>`
written by the reducer — rejected, it is a cache of a function of `active_session` and would go
stale on precisely the async path R7 covers.

## R3 — FR-001c says a row must not close when its session stops being current. Does that break the derived model?

**Decision**: No, but it needs one explicit step. When `active_session` changes (including to
`None`), the outgoing forced-open location is committed into the user's set — inserted into
`expanded`, or `default_expanded = true` — before the derivation runs against the new current
session.

**Rationale**: Read strictly, a derived-only model closes the row the instant its session stops
being current: closing the current session would snap its location shut, taking its siblings out of
view with it. That is what validation caught in the spec (FR-001c, and US3 scenario 3). Committing
on change turns "was revealed" into ordinary user-open state, which is also the honest description
of what the user saw — a row that was open.

Committing accumulates open rows within a project. That is intended and matches how a file tree
behaves; FR-007 keeps it from leaking across projects, because `expanded` is pruned by directory
name on the incoming project's `set_worktrees` and `default_expanded` is reset outright.

**Alternatives considered**: Leaving the row to close and rewording FR-001c — rejected; the user's
own spec pass identified the snap-shut as wrong before any code existed. A second field remembering
"last revealed" — rejected as a cache with no reader after the commit.

## R4 — What is the non-colour cue for FR-003a?

**Decision**: The current session's name renders at the type scale's **500 weight**; other session
rows stay at 400. The `secondary_container` pill stays exactly as it is.

**Rationale**: `ui/material/text.rs:79-82` ships exactly two Roboto faces, 400 and 500, because
"weight 400 and 500 are the only weights the Material 3 type scale specifies". Weight is therefore
the one non-colour channel available without inventing a token, and Material already uses it to mark
emphasis (`Action` is `label_large` at 500 for the same reason). It is legible in greyscale, and it
survives a truncated label, which is what FR-003 asks for.

The obvious alternative — a leading indicator glyph — is closed off deliberately. A session row's
leading slot holds the activity dot and *only* that: BUG-005 removed an unconditional
`Icon::ActiveMarker` from exactly this slot because it competed with the dot that does vary
(`ui/sidebar.rs:456-461`). Re-adding a leading glyph would re-open a bug this repo has already
fixed, and FR-003a explicitly forbids displacing the signals the row already carries.

**Alternatives considered**: A left edge bar — not in the design system, and it would need a new
token plus a new anatomy figure. Shape alone (the pill's `FULL` radius) — rejected, the shape is
only perceivable *because* of the fill, so it is a colour cue wearing a shape's clothes. An outline
on the pill — viable, and it can be added later if §B judges weight too subtle; not taken now
because two cues for one meaning is what FR-003a's second sentence warns against.

## R5 — How does the FR-012a chip reach the row without putting a view concern in core?

**Decision**: `WorktreeNode` gains `shown_for_current_session: bool`. `ui/sidebar.rs` appends one
chip to the row's existing `tags(...)` slot when it is set. `micold-core::naming::Tag` is not
touched.

**Rationale**: `Tag` is derived from branch naming — `worktree_tags` reads the branch and yields
`Type`, `Issue`, `Status`, `Agent`. "Holds the current session" is not a fact about a branch name; it
is a fact about this run's session state, and it changes without the worktree changing. Putting it in
`Tag` would mean `worktree_tags` needed session state to compute a naming tag.

`Tag::Agent` is a real precedent for a label-only tag that is never a filter
(`features/sidebar.rs:196-199`), so the *chip* is idiomatic — only its source differs.

**Alternatives considered**: `Tag::Current` in core — rejected as above. A separate trailing badge —
rejected, worktree rows already have a trailing action cluster whose width is reserved to stop
hover reflow (`ui/sidebar.rs:423-427`), and a badge there would fight it.

## R6 — How is "is the row visible" answered, given `Scrollable` has no id and iced 0.14 has no `visible_bounds`?

**Decision**: Compute it. Row heights in this list are deterministic, so a pure function over the
ordered rows plus the viewport height plus the current offset decides whether to scroll and to what
offset. `main.rs` applies the answer with `iced::widget::operation::scroll_to`.

The three inputs:

- **Row heights**: `TreeView` floors each row at `density::height(base, step)`, where `base` is
  `LIST_ROW_ONE_LINE_BASE` or `LIST_ROW_TWO_LINE_BASE` depending on whether the row has tags
  (`tree_view.rs:367-396`), and stacks rows in a `column![].spacing(spacing::XS)`
  (`tree_view.rs:221`, `spacing::XS == 4.0`). Both figures are already asserted in
  `ui/material/anatomy_size.rs:471`, `:519`.
- **Current offset**: already in state — `sidebar_scroll_offset: u32`, fed by
  `Scrollable::on_scroll_offset` through `scroll_offset_px` (`app.rs:569`, `:1007`).
- **Viewport height**: new. `Scrollable` gains `.on_viewport_resize(f)`, implemented with iced
  0.14's `Sensor` (`iced_widget-0.14.2/src/sensor.rs`), which reports its content's laid-out `Size`
  on first appearance and on every resize. `on_scroll`'s `Viewport` was rejected for this: it only
  fires when something scrolls, and the case that matters most is the first draw after a switch,
  where nothing has scrolled.

**Rationale**: iced 0.14 has no "scroll this child into view" operation and no `visible_bounds`
(checked: `iced_widget-0.14.2`, `iced_core-0.14.0` — only `snap_to`, `scroll_to`, `scroll_by`, all
`Id`-targeted, re-exported as tasks from `iced_runtime-0.14.0/src/widget/operation.rs`). Something
has to do the arithmetic, and doing it in a pure function is what lets FR-008 and FR-009 be tested
rather than eyeballed.

**This is the feature's one real risk**: a computed height that disagrees with the rendered height
scrolls to the wrong place, and the disagreement is silent. Two mitigations, both required:
the metric function is asserted against the same `density::height` figures `anatomy_size.rs` uses
(so a density change breaks the test rather than the scroll), and quickstart §B4 checks a 30-location
project by eye at both densities.

**Alternatives considered**: `Sensor` on the *row* with `on_show`/`on_hide` — attractive because
FR-009 would fall out for free, but rejected: `Sensor` reports a `Size`, never a position, so there
is nothing to scroll *to*, and its notifications are transitions, so a row that is already off-screen
on first draw may report nothing at all. `snap_to` with a relative offset derived from the row index
— rejected, it is only correct when every row has the same height, and tagged rows do not.

## R7 — What happens on the async path, where the worktree list arrives after the switch?

**Decision**: Nothing special, by construction. The derivation in R2 runs on every view, so the row
opens on the first view in which its location is known.

The scroll is the part that needs care: it is a one-shot task, and firing it before the target row
exists would scroll to a stale offset. So `pending_reveal_scroll` is armed by the reveal and drained
by `main.rs` only once a target row is actually present in the projection; if the location is not yet
known, the field stays armed until it is.

**Rationale**: FR-001b names arrival order explicitly, and `set_worktrees` is called both from the
`WorktreesLoaded` reducer arm and from the binary's direct re-discovery (`app.rs:1525-1530`), so
"the list is replaced" is a normal event, not an edge case.

**Alternatives considered**: Re-arming the reveal from the `WorktreesLoaded` arm — rejected, it is
the one-shot model R2 exists to avoid, and it would re-open a row the user closed on every worktree
refresh (SC-008).

## R8 — FR-010a asks for animation "the same as a user-initiated expand". What is that, today?

> **Outcome**: FR-010a was reworded to say what it was for — the app's reveal must not be a
> different experience from the user's own expand, whatever that experience is — because as written
> it demanded a motion that does not exist. See spec Clarifications, session 2026-08-10. The
> decision below is unchanged by that rewording; it is what prompted it.

**Decision**: Instant. A user-initiated expand in this codebase is not animated — `TreeView` builds
its rows from the item list and expansion simply adds items (`ui/material/tree_view.rs:223`,
`ui/sidebar.rs:430`). `animation.rs`'s wrappers animate per-row icon reveal, not the twisty.

So FR-010a is satisfied by *using the same code path for both cases* and adding no motion. The
requirement is not vacuous: it forbids special-casing the reveal, so if expansion later gains
motion, the in-place reveal inherits it and the switch path keeps its no-transition guarantee
(SC-002) because there the panel's whole contents are replaced anyway.

**Rationale**: Inventing an animation for the reveal alone would give the app two different
expansion behaviours depending on who triggered it — the opposite of what FR-010a asks. It would
also be the only motion in the sidebar's list, with no token or duration in the design system to
draw on.

**Alternatives considered**: Animating the scroll for the in-place case — deferred, not rejected. It
needs a per-frame offset driver (`scroll_to` is a single jump) and the motion vocabulary to say how
long; worth revisiting if §B judges the jump abrupt. Recorded here so a future reader knows the
current instant behaviour is a decision, not an oversight.

## R9 — Does `micold-core` change?

**Decision**: No.

**Rationale**: The reveal reads `Session::location` and compares it against a location the panel
already knows. Which row a panel draws open is not a fact about a session, a worktree, or a project,
and the constitution's storage principle (IV) means anything landing in core invites the question of
whether it should persist — for this feature the answer is explicitly no (spec Assumptions:
"Nothing about this reveal is remembered across restarts").

## R10 — Where is the delivery risk concentrated?

Ordered by expected trouble:

1. **Scroll arithmetic (R6)** — the only place a silent wrong answer is possible. Contained by the
   metric test sharing `anatomy_size.rs`'s figures.
2. **The `Sensor` wrapper (R6)** — new widget in this codebase. `Ripple`'s `operate` comment
   (`ui/material/ripple.rs:248-256`) is the warning to heed: a wrapper that does not forward
   `operate` silently swallows `scroll_to` for its whole subtree. The sidebar's `Scrollable` sits
   above `Ripple`-wrapped rows, so the `Id` must be on the scrollable itself, and any new wrapper
   between the two must forward.
3. **FR-003a's weight (R4)** — cheap to implement, and the one thing only a human eye (or §B) can
   judge sufficient. If it reads as too subtle, the outline alternative is pre-argued in R4.
4. **Everything else** — predicate and flag changes in a tested projection.

## R11 — Sessions, worktrees, persistence, reduced motion?

- **Sessions/worktrees**: untouched. No lifecycle call, no worktree operation (Principles II, III).
- **Persistence**: nothing written or read (Principle IV).
- **Reduced motion**: not applicable — R8 adds no motion.
- **Cross-platform**: arithmetic and iced operations only, no platform branch (Principle VI).

## R12 — FR-001 listed "restore at launch" as a trigger. Does that path exist?

> **Outcome**: no — and the spec was corrected rather than the code. FR-001 is now a rule over every
> path that makes a session current, FR-001d states the negative, and SC-004 no longer counts paths.
> See spec Clarifications, session 2026-08-10. The finding below is kept as written, because the
> reasoning is what justifies the shape the arming rule ended up with.

**Decision**: Not today. Nothing makes a session current at launch, so the trigger is vacuous — and
the arming rule is written so it costs nothing if that ever changes.

**Rationale**: `boot()` loads the workspace from `JsonFileStore`, refreshes availability, prunes
empty sessions and discovers the active project's worktrees (`main.rs:519-556`) — but
`active_session` comes from `State::default()` and stays `None`. Sessions themselves arrive later
from the daemon (`reconcile_catalog`, `main.rs:2281`), which only ever *clears* a dangling
`active_session` (`main.rs:2399-2403`); it never sets one. `restore_after_activation` — the function
that does set it, via `restore_foreground` (`features/session.rs:92`) — has exactly one call site,
`Message::FolderChosen` (`main.rs:1312`), which is the project switch. And
`foreground_by_project` lives on `State`, not in the persisted `Workspace` (`app.rs:547-549`), so
there is nothing for a launch restore to restore *from*. After a cold start the panel therefore has
no current session until the user selects or starts one, which FR-013 / contract §1.2 already
covers.

So the honest statement is: **launch restore is not a second trigger, it is the absence of one.**
Two consequences, both cheap:

- The open-ness derivation (R2) needs no launch case at all. The moment anything makes a session
  current — however that comes about — the row reads as open, because open-ness is a function of
  `active_session` rather than of an event that fired.
- Only the scroll needs arming, and it is armed on **any app-initiated transition of
  `active_session`**, not on a named list of call sites. `Message::SessionSelected` is the one
  transition excluded, because FR-006 says a user's own click scrolls nothing. A future launch
  restore inherits the reveal without a new arming site.

**Alternatives considered**: Arming from an enumerated set of messages — rejected, it is the shape
that made this discrepancy possible: an enumeration invites a caller that sets `active_session`
without arming, and the contract would go on claiming a trigger the code does not have. Adding a
launch-restore path so FR-001's third trigger becomes true — out of scope; the spec asks for the
reveal, not for changing which session is current (spec Assumptions, contract §8).
