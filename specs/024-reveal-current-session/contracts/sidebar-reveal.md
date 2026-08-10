# Contract: the sidebar reveal

Feature 024. The UI contract for what the side panel must show about the current session. Every
clause is stated so it can be asserted against the render-free projection in
`crates/micold-client/src/features/sidebar.rs` or the reducer in `app.rs` — not against pixels.

## §1 The forced-open location

**§1.1** For any location `loc`, the panel shows it open exactly when

```text
user_open(loc) || (Some(loc) == location_of_current_session() && reveal_suppressed_for != active_session)
```

**§1.2** `location_of_current_session()` is `None` when `active_session` is `None`, when its session
record is absent from the active project, or when the session's location is not among the project's
known locations. In each case §1.1 reduces to `user_open(loc)` and no row is forced. *(FR-013)*

**§1.3** §1.1 is evaluated on every view. No caller stores its result. *(FR-001b — a list that
arrives late or is replaced wholesale changes the answer's inputs, never a stored answer.)*

**§1.4** Exactly one location can be forced at a time, because there is at most one current session.
*(Invariant I1.)*

## §2 What closes a forced row

**§2.1** A user collapse of the forced row sets `reveal_suppressed_for = active_session`. From then
until `active_session` changes, §1.1 yields `user_open(loc)` for that location. *(FR-005, SC-006)*

**§2.2** No other event closes it. Specifically, replacing the worktree list — from the
`WorktreesLoaded` arm or the binary's re-discovery — must not close a forced row and must not clear
`reveal_suppressed_for`. *(SC-008)*

**§2.3** When `active_session` changes — **including a change to `None`** — the outgoing forced
location is first committed to the user's set (`expanded.insert(dir)` or `default_expanded = true`),
then `reveal_suppressed_for` is cleared. A location that stops holding the current session therefore
stays open. *(FR-001c, Invariant I3)*

## §3 Which events arm a reveal

**§3.0** The rule is *any app-initiated transition of `active_session` **to `Some`***, not a list of
call sites. `SessionSelected` is the single excluded transition to `Some`. Stated this way so a
future path that makes a session current inherits the reveal without a new arming site — which is
exactly what FR-001 now requires of every path rather than of a named list. *(research R12.)*

**§3.0a** A transition to `None` arms nothing. It still runs §2.3's commit — that is what keeps the
outgoing row open (FR-001c) — but there is no row to mark and none to scroll to, and FR-001a
forbids scrolling on the user's behalf when they close the session they were on. Every writer below
therefore goes through the same function; only the *arming* is conditional on the new value.

| Event | Arms a reveal | Clause |
|---|---|---|
| Project switch (`restore_after_activation`, `main.rs:1312`) | yes | FR-001 |
| `SessionStarted` (`app.rs:1279`) | yes | FR-001 |
| `SessionSelected` (user clicked the row, `app.rs:1286`) | **no** | FR-006 |
| `SessionCloseRequested` / `SessionRemoveConfirmed` (`app.rs:1357`, `:1390`) | **no**, and no successor is promoted | FR-001a, §3.0a |
| Active project forgotten (`app.rs:877`) | **no** — a transition to `None` | §3.0a |
| Dangling pointer cleared by `reconcile_catalog` (`main.rs:2401`) | **no** — a transition to `None` | §3.0a |
| `WorktreesLoaded` / worktree re-discovery | **no** | §2.2, SC-008 |
| Launch | **nothing to arm** — no session is current at launch | §3.2 |

**§3.1** Arming a reveal sets `pending_reveal_scroll` and clears `reveal_suppressed_for`. It writes
nothing to `expanded` / `default_expanded` beyond §2.3's commit of the *outgoing* location.

**§3.1a** The table lists every writer of `active_session` in the tree at the time of writing, and
that completeness is the point: an enumeration nobody checks is how a future caller silently stops
arming. It is asserted by a source gate rather than maintained by hand — see §3.0's rule, which is
what the gate encodes.

**§3.2** Nothing makes a session current at launch: `boot()` never sets `active_session`,
`reconcile_catalog` only clears a dangling one, and `foreground_by_project` is not persisted
(research R12). A cold start therefore has no current session and §1.2 applies until the user
selects or starts one — which is what FR-001d requires. This feature does not add such a path; §3.0
is what makes one free if it is ever added.

## §4 The mark

**§4.1** Exactly one session row carries the current mark whenever `location_of_current_session()` is
`Some`; none carries it otherwise. *(FR-002)*

**§4.2** The mark is the existing `secondary_container` pill **plus** the name rendered at the type
scale's 500 weight; a non-current session row's name renders at 400. *(FR-003a — the mark does not
depend on colour alone.)*

**§4.3** The mark does not read from, alter, or suppress the row's lifecycle tint or its activity
indicator. *(FR-003a second sentence, and BUG-005's reasoning about the leading slot.)*

**§4.4** The mark is independent of `terminal_focused` and of the session's `lifecycle`. A stopped,
failed, or interrupted current session is still marked. *(FR-014, FR-015)*

## §5 The filter exemption

**§5.1** The location holding the current session appears in the panel even when the active tag
filters, or `show_agent_worktrees == false`, would exclude it. The exemption resolves against **all**
worktrees, not `visible_worktrees`, because the hidden-agent setting excludes rows before the tag
filters run. *(FR-011)*

**§5.2** No other excluded location is admitted. *(FR-012, SC-005)*

**§5.3** The exemption ends as soon as that location stops holding the current session; the row
returns to being hidden if the filters still exclude it. Note the interaction with §2.3: the row's
*open* state is committed and survives, its *presence* does not. *(FR-012)*

**§5.4** A node admitted only by §5.1 carries `shown_for_current_session = true` and renders one
extra chip in the row's tag slot saying it is shown because it holds the current session. A node the
filters admit on their own carries `false` and renders no such chip. *(FR-012a)*

**§5.5** The exempt row appears in the position it would occupy unfiltered — the exemption changes
membership, never order. *(FR-012a)*

**§5.6** `available_tag_filters` is unaffected: an exempt row must not conjure a filter chip, for the
same reason a hidden agent worktree must not (`features/sidebar.rs:177-179`, feature 014 R7).

## §6 Scroll into view

**§6.1** `scroll_target(heights, index, viewport_height, current_offset)` returns `None` when the
row's full extent already lies within `[current_offset, current_offset + viewport_height]`.
*(FR-009, SC-007)*

**§6.2** Otherwise it returns the minimal offset bringing the row fully into view — clamped to the
list's scrollable range, scrolling up for a row above the viewport and down for one below. Not
centred. *(FR-008)*

**§6.3** `viewport_height == 0.0` yields `None`. No scroll is issued before the first layout.

**§6.4** `pending_reveal_scroll` is drained only when the projection holds a row for the current
session; otherwise it stays armed. *(FR-001b + research R7 — the async path must not scroll to a
stale offset.)*

**§6.5** After a drain, the field is `None` and the user's subsequent scrolling is not overridden
until the next reveal is armed. *(FR-010, SC-007)*

## §7 Motion

**§7.1** The reveal uses the same expansion path as a user-initiated expand, which is instant today.
No motion is added, and the reveal is not special-cased. *(FR-010a, research R8)*

**§7.2** On a project switch, the first drawn frame of that project's panel already
shows the row open, marked, and scrolled. No intermediate frame shows the current session unmarked or
hidden. *(SC-002)*

## §8 What this contract does not cover

- Which session becomes current. Unchanged — `restore_foreground` still decides
  (`features/session.rs:111`). *(spec Assumptions)*
- Any change to sessions, worktrees, or persistence. There is none. *(Principles II, III, IV)*
- Marking a *location* row as containing the current session while it is closed. Out of scope by the
  spec's Assumptions; the row is opened, so the session row carries the signal.
