# Data Model: Reveal the current session in the sidebar

Feature 024. Nothing here is persisted (Principle IV — there is nothing to store, not a decision to
store nothing carefully). Every field below is in-memory view state on `State`, or a field on a
projection `State` rebuilds each view.

## New state on `State` (`crates/micold-client/src/app.rs`)

| Field | Type | Meaning | Lifetime |
|---|---|---|---|
| `reveal_suppressed_for` | `Option<SessionId>` | The user closed the revealed row while *this* session was current. Scoped to the session so an old collapse cannot suppress the next reveal (FR-005). | Cleared whenever `active_session` changes. |
| `sidebar_viewport_height` | `f32` | The sidebar scroll viewport's laid-out height, reported by the `Scrollable`'s `Sensor` (research R6). `0.0` until the first layout, which reads as "cannot decide visibility yet" — never as "zero tall". | Overwritten on every layout/resize. |
| `pending_reveal_scroll` | `Option<f32>` | An armed scroll target in absolute pixels from the top of the list, set by the reducer, drained by `main.rs`. `None` means nothing to do. | Armed by a reveal; drained once a target row exists in the projection (research R7). |

`expanded: BTreeSet<String>` and `default_expanded: bool` are **unchanged in type and gain a
narrowed meaning**: they are now strictly *the user's* open set. Nothing about a reveal is written to
them except by the commit-on-clear rule below.

### Invariants

1. **I1 — At most one location is force-open.** There is at most one current session, so the
   derivation in §"Effective open state" yields at most one forced location. Not enforced by a type;
   guaranteed by `active_session: Option<SessionId>` being the only input.
2. **I2 — `reveal_suppressed_for` is only ever the current session or stale-cleared.** It is set to
   `active_session` on a user collapse of the forced row, and set to `None` on every change of
   `active_session`. It therefore never suppresses a reveal for a session other than the one the
   user collapsed against.
3. **I3 — Commit precedes re-derivation.** On a change of `active_session`, the *outgoing* forced
   location is committed to the user's set before the new current session is derived against
   (FR-001c). Order is load-bearing in the same way `switch_active`'s already is
   (`features/session.rs:57-74`).
4. **I4 — `pending_reveal_scroll` is never applied blind.** It is drained only when the projection
   contains a row for the current session; otherwise it stays armed (research R7).
5. **I5 — Only a transition to `Some` arms.** A change of `active_session` to `None` runs I3's
   commit and clears suppression, but arms nothing: there is no row to scroll to, and FR-001a
   forbids scrolling when the user closes the session they were on. Without this, closing a session
   would leave `pending_reveal_scroll` armed with no target — armed forever by I4, and then applied
   against whatever row appeared next (contract §3.0a).

### State transitions

```text
                      app makes a session current
        (switch / new session / any future path)
                              │
                              ▼
              ┌───────────────────────────────┐
              │ commit outgoing forced row     │  I3, FR-001c
              │ into expanded/default_expanded │
              └───────────────┬───────────────┘
                              ▼
              ┌───────────────────────────────┐
              │ reveal_suppressed_for = None   │  I2
              │ armed ONLY if the new value    │  I5
              │ is Some: pending_reveal_scroll │  FR-008
              └───────────────┬───────────────┘
                              ▼
   ┌──────────── derived every view ───────────────┐
   │ effective_open(loc) per §below                 │  FR-001b
   │ exempt loc from filters if it holds current    │  FR-011
   └────────────────────────────────────────────────┘
             │                          │
   user collapses forced row      main.rs drains scroll
             ▼                          ▼
   reveal_suppressed_for =        scroll_to(offset),
     active_session  (FR-005)     field cleared (I4)
```

The paths that make a session current are the existing ones — no new message is introduced for them:
`restore_after_activation` (`features/session.rs:90`, reached only from `Message::FolderChosen`) and
`Message::SessionStarted` (`app.rs:1261`). `Message::SessionSelected` (`app.rs:1285`) deliberately
does **not** arm a reveal (FR-006), and the close/remove arms (`app.rs:1356`, `:1389`) deliberately
do not promote a successor (FR-001a).

Four writers clear `active_session` rather than set it: the close and remove arms above, forgetting
the active project (`app.rs:877`), and `reconcile_catalog` dropping a dangling pointer
(`main.rs:2401`). All four route through the same function so I3's commit runs, and none of them
arms (I5). That list is what contract §3's table enumerates and what the source gate checks.

FR-001 also names "restore at launch". No such path exists: `boot()` leaves `active_session` at
`None` and `reconcile_catalog` only clears a dangling one (research R12). The arming condition is
therefore written as *any app-initiated transition of `active_session`* rather than as a list of
messages, so the trigger becomes real the day a launch restore is added, with nothing here to
change.

## Effective open state (`crates/micold-client/src/features/sidebar.rs`)

A pure predicate, the single answer to "is this row open", replacing the two direct reads of
`expanded` / `default_expanded` in the projections:

```text
effective_open(loc) =
      user_open(loc)
   || (Some(loc) == location_of_current_session() && reveal_suppressed_for != active_session)

user_open(Worktree(dir)) = expanded.contains(dir)
user_open(Default)       = default_expanded
```

`location_of_current_session()` resolves `active_session` through the active project's sessions to a
`SessionLocation`, and is `None` when there is no current session or its record is gone — which is
what makes FR-013's "the location no longer exists" case fall out rather than need a branch.

## Changed projection: `WorktreeNode`

| Field | Change |
|---|---|
| `expanded` | Now fed by `effective_open`, not by `expanded.contains(...)` directly. |
| `shown_for_current_session` | **New** `bool`. True only when this node is in the list *because* it holds the current session and the active filters or the hidden-agent setting would otherwise have excluded it (FR-012a). False for a node the filters admit on their own — that distinction is the requirement, not an implementation detail. |

`DefaultNode` gains nothing: it is already exempt from tag filtering
(`features/sidebar.rs:147-149`), so FR-011 is already true for it and `shown_for_current_session`
would be permanently false.

### Filter exemption

`filtered_worktree_tree` changes from "filter `worktree_tree`" to "filter, then re-admit the one
location holding the current session". Two sources must be re-admitted from, because two independent
mechanisms hide a row:

- the tag filters (`matches_filters`, `features/sidebar.rs:82`), and
- the hidden-agent-worktree setting, which excludes rows *earlier*, in `visible_worktrees`
  (`features/worktree.rs:77`), so `worktree_tree` never sees them.

The exemption therefore resolves against **all** worktrees, not `visible_worktrees`. FR-012's "only
that one" is what keeps this from becoming a filter bypass.

## Row metrics and the scroll target

Two pure functions in `features/sidebar.rs`, both testable without a renderer (research R6):

```text
row_heights(rows)  -> Vec<f32>     # per row: density::height(one_line|two_line, step)
scroll_target(heights, index, viewport_height, current_offset) -> Option<f32>
```

`scroll_target` returns `None` when the row is already fully visible (FR-009) and otherwise the
**minimal** new offset that brings it fully into view — scrolling up if it is above the viewport,
down if below. Minimal, not centred, because the spec's Assumptions say scroll-into-view does not
require centring, and a minimal move is the one least likely to disturb what the user was looking at.

`viewport_height == 0.0` (no layout yet) yields `None`: nothing is scrolled on a guess.

## Entity mapping to the spec

| Spec entity | Here |
|---|---|
| Current session | `State::active_session`, resolved through the active project's sessions |
| Location | `SessionLocation` (`micold-core`, unchanged) — `Worktree(dir_name)` or `Default` |
| Reveal | The derivation above plus `pending_reveal_scroll`; not a stored object |
| Side panel | The `Scrollable` + `TreeView` in `ui/sidebar.rs` |
