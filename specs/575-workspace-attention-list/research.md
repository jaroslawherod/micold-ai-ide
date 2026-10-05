# Research: Attention Indicator on the Sidebar's Worktree Rows

**Feature**: 575 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

No `NEEDS CLARIFICATION` was left in the Technical Context. Each decision below names the code it
rests on and what was rejected.

## R1 — One predicate for a counted session, in the core (FR-002, FR-006, FR-010)

**Decision**: add `micold_core::attention::counts_as_unread(session: &Session, in_view:
Option<SessionId>) -> bool` = `session.unread && !session.archived && in_view != Some(session.id)`.
`Workspace::unread_session_count` (`crates/micold-core/src/workspace.rs:334`) filters through it,
and so do the sidebar's location counts (R2). The switcher's rows, the switcher's button
(`other_projects_unread`, which calls `unread_session_count`) and the location rows then count
with the same rule, which is what makes FR-010's sum hold by construction.

**Rationale**: today the switcher filters on `s.unread && Some(s.id) != in_view` only, so a closed
session that was unread still counts (spec Clarifications; 039 US1 scenario 9). Putting the rule
in one core function means the fix and the new count cannot disagree, and it is unit-testable in
`micold-core` without iced (Principle I).

**Alternatives considered**:
- *Have the session service clear `unread` when a session is closed.* Rejected: a daemon and
  persistence change that alters 039's clearing rule (out of scope, spec Out of Scope), and
  records already closed while unread would still count until rewritten.
- *Filter archived sessions only in the client's sidebar code.* Rejected: the switcher would keep
  counting them and FR-010's sum would fail.
- *Extend `features::attention::row_unread` (client) instead.* Rejected: the switcher's count is
  computed in the core, so the core needs the rule; the client reuses it, not the reverse.
  `row_unread` stays as it is for session rows, whose sessions are never closed ones (the sidebar
  filters `!s.archived`, `crates/micold-client/src/features/sidebar.rs:622,670,703`).

## R2 — The location count is derived from the sidebar's own nodes (FR-001, FR-002, FR-003, FR-006)

**Decision**: add `SidebarEntry::unread_count(&self, in_view: Option<SessionId>) -> usize` in
`crates/micold-client/src/features/sidebar.rs`, counting the entry's `sessions` (Default or
worktree node) with `counts_as_unread`. The view (`crates/micold-client/src/ui/sidebar.rs`
`build_items` / `build_default_item`) calls it with `features::attention::in_view(&state.attention)`,
the same `in_view` `switcher_entries` uses (`crates/micold-client/src/app.rs:690`).

**Rationale**: `sidebar_entries()` already joins each location with its non-closed sessions,
whether the row is expanded or not (`WorktreeNode::sessions`, `DefaultNode::sessions`), so the
count is a filter over data already there: no new state, nothing stored (FR-006), the same rows
the user sees, and expansion cannot change it (FR-003). The re-admitted row of feature 024 and
missing or invalid worktrees are nodes like any other, so they get a count with no extra case.
A worktree hidden by a tag filter or the agent-worktree setting has no node and so no count,
while its sessions remain in `Workspace::sessions` for the switcher.

**Alternatives considered**:
- *A `unread` field on `WorktreeNode` / `DefaultNode`, filled when the tree is built.* Rejected:
  seven construction sites, tests included, change for a value the node can compute from what it
  holds; and it would freeze `in_view` into a structure that is otherwise independent of it.
- *A core `Workspace::location_unread_count(project, location, in_view)`.* Rejected: it would
  repeat the session-to-location matching the sidebar already does (`is_worktree(dir)`,
  `== SessionLocation::Default`), giving two places that decide which sessions a row holds.
- *Count in the view glue directly.* Rejected: decision logic in `src/ui/` falls outside
  Principle I's glue exception.
- *A per-location count sent by the session service.* Rejected: a wire change and a second
  source of truth (FR-006).

## R3 — The host: `TreeItem` gains a counted-unread variant (FR-004, Principle VIII)

**Decision**: replace `TreeItem::unread: bool` with a private enum
`RowUnread { Read, Unread, Count(NonZeroUsize) }` and add the builder
`TreeItem::unread_count(n: usize)` (`0` → `Read`). `.unread(bool)` keeps its meaning and
signature (session rows, 039). `Count(n)` renders `UnreadMark::new(r).count(n).role(label_role)`
in the place `Unread` renders the bare mark: after the label and annotation, before the trailing
element (`tree_view.rs` around line 394).

**Rationale**: the mark already lives in that slot for session rows (039 contract `unread-mark.md`,
host `TreeItem`), the label is `Ellipsized` so a narrow row shortens the name and keeps the mark
(U8), and the slot is one line so the row keeps its height. An enum makes "marked and counted at
once" unrepresentable (Principle V). `UnreadMark` is reused unchanged; no widget is forked.

**Alternatives considered**:
- *Put the mark in the `badge` slot.* Rejected: that is the leading activity slot; 039 FR-018
  separates the unread mark from the activity badge by place.
- *Put it inside the row's `trailing_element` cluster.* Rejected: the worktree cluster is wrapped
  in `HoverReveal` and is invisible at rest (`ui/sidebar.rs` `row_actions_cluster`).
- *Reuse `.unread(true)` with no count.* Rejected: FR-001/FR-002 require the number.
- *A second `unread_count: usize` field beside `unread: bool`.* Rejected: allows both at once.

## R4 — The form: `● 2`, not `● 2 unread`; the label weight unchanged (FR-004)

**Decision**: the location row shows the mark with the bare number (`UnreadMark::count(n)`, not
`.worded(true)`), the number in the row's label role and the theme's text colour (also on a
missing or invalid worktree, whose name is error-tinted). The location row's label keeps its
role: no emphasis.

**Rationale**: the spec's indicator is "the shared unread mark followed by the number"; its
"form the switcher's rows use" distinguishes the counted mark from the bare mark of a session row. So "form" in FR-004 is read as the counted mark `● n`, not the
worded `● n unread` the switcher's menu rows add for their second number; a conformance check
should hold the indicator to `● n`.
`worded` exists for a host that shows another number beside this one (`unread_mark.rs` doc on
`worded`, 039 FR-021: the switcher's running count); a location row shows no other number, and
the sidebar's width is the scarce resource (U8). The words are given by the tooltip (R5), as the
switcher's button does (`button.rs` `unread_total_tooltip`). The emphasised label role of the
tree is the current-session role (`TypeRole::SidebarSessionCurrent`, `ui/sidebar.rs`
`selected_label_role`), designed for depth-1 session rows; putting it on a depth-0 location row
would read as "current", and the spec asks for the indicator only. The number in the error tint
would read as a fault count, so it keeps the text colour.

**Alternatives considered**:
- *`● 2 unread`, muted, as in the switcher's menu rows.* Rejected for width and because the
  muted tint is the menu's trailing-text convention, not the sidebar's.
- *Emphasise the location row's name as 039 does for a session row.* Rejected above.

## R5 — The meaning in words: the row tooltip (FR-005)

**Decision**: a pure `features::sidebar::unread_tooltip_line(n) -> Option<String>` returns
`"1 unread session"` / `"{n} unread sessions"` (`None` at zero), and
`features::sidebar::with_unread_line(tooltip: String, n) -> String` appends it as the last line of
a location row's tooltip: after `worktree_tooltip(...)`'s lines on a worktree row, after
`DEFAULT_LOCATION_LABEL` on the Default row.

**Assistive technology**: iced 0.14 exposes no accessibility tree to the platform (no `accesskit`
in `Cargo.lock`; no accessibility API in `crates/micold-client/src`), so no widget of this
application, 039's marks included, can state anything to a screen reader. The tooltip line is the
text equivalent the stack can carry, and it is the string an accessible label would use once iced
gains one. This is recorded as a known limitation of the stack in plan.md, not a deviation of
this feature.

**Alternatives considered**:
- *Change `worktree_tooltip`'s signature to take the count.* Rejected: 21 call sites, mostly
  tests of other facts; appending keeps those tests unchanged.
- *Word the number on the row itself.* Rejected in R4.

## R6 — Hover and the row actions (FR-004, edge case "Hover")

**Decision**: no change to `row_actions_cluster`. The indicator sits before the trailing element;
the cluster's width is reserved on every row and only its opacity animates (`HoverReveal`,
`ui/sidebar.rs` doc on `row_actions_cluster`), so hovering neither covers nor moves the
indicator. A geometry test pins this: the indicator's bounds are equal with the row hovered and
at rest, and do not intersect the cluster's.

**Alternatives considered**: *hide the indicator on hover to make room for the actions.*
Rejected: FR-004 forbids it, and the reserved width already makes room.

## R7 — Live, every window, after restart: no new mechanism (FR-007, FR-008, FR-009)

**Decision**: nothing new. The sidebar is rebuilt from `State` on every update; catalog updates
from the session service already update `Workspace::sessions` (039), and `in_view` already drops
the session in view at once (039 FR-019). Each window is its own client process fed by the same
service (039 research R1, FR-024), and `unread` is persisted by the service (039 FR-008a). No
reducer for expand, collapse or hover touches attention state, which a test pins (FR-009).

**Alternatives considered**: *a cached per-location count updated on catalog messages.* Rejected:
a second state to keep in sync, for a count over at most tens of sessions per project.

## R8 — FR-010's sum, and what it does not cover

**Decision**: test the sum on `State` for a project with no hidden worktrees: the sum of
`SidebarEntry::unread_count(in_view)` over `sidebar_entries()` equals the active project's
`SwitcherEntry::unread_count`. Sessions whose location has no row — a hidden worktree, or a
worktree not (yet) in the discovered list — count on the switcher only; the spec's condition
("none of whose worktrees the sidebar hides") excludes those projects from the sum.

**Alternatives considered**: *a "hidden: n" line in the sidebar for the difference.* Rejected:
not asked for (spec Out of Scope: no indicator outside the location rows).

## R9 — Contrast in every row state (FR-013)

**Decision**: extend `composition_contrast.rs`'s
`the_unread_mark_is_legible_on_every_fill_a_host_draws_it_on` with the hovered sidebar row's fill
(`style::state_layer(sidebar fill, on_surface, state::HOVER)`), and add a text check that the
count's colour (`on_surface`) meets 4.5:1 on the sidebar fill, the hovered fill and the selected
pill (`secondary_container`), in both schemes. Location rows are never drawn selected today; the
selected pill is checked anyway because FR-013 names it and `TreeItem` allows it.

**Alternatives considered**: *rely on the visual pass alone.* Rejected: contrast is measurable,
and the gate already exists for this mark.
