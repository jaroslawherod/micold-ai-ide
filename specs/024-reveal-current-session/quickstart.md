# Quickstart: Reveal the current session in the sidebar

Two parts. **§A** is what the machine checks. **§B** is what has to be looked at, because two of this
feature's claims are about what is *on screen at a given moment* — a row's weight, and whether a row
is inside the viewport — and no test in this repository can see either.

§B is runnable without a human: drive it with the repo's `visual-pass` skill on a private display.

---

## §A — The automated suite

```bash
mise run test        # whole workspace, matching CI
mise run test-core   # render-free logic only; faster while iterating on the predicate
```

Green is the gate. What each gate is watching, for this feature:

| Gate | Watching |
|---|---|
| `tests/features_sidebar.rs` | `effective_open` — forced by the current session, suppressed by a user collapse, and unaffected by a replaced worktree list (contract §1, §2) |
| `tests/features_sidebar.rs` | the filter exemption admits exactly one location, from **all** worktrees rather than `visible_worktrees`, and orders it where it would sit unfiltered (§5.1, §5.2, §5.5) |
| `tests/features_sidebar.rs` | `shown_for_current_session` is `true` only for a node the filters would have excluded (§5.4); an exempt row conjures no filter chip (§5.6) |
| `tests/sidebar_tree.rs` | `scroll_target` — `None` when already visible, minimal offset otherwise, `None` at zero viewport height (§6.1–§6.3) |
| `tests/sidebar_tree.rs` | `row_heights` agrees with `density::height(LIST_ROW_ONE_LINE_BASE \| LIST_ROW_TWO_LINE_BASE, step)` at both densities, sharing the figures `src/ui/material/anatomy_size.rs` already asserts. **The gate that matters most** — a computed height that drifts from the rendered one scrolls to the wrong place silently (research R6, R10) |
| `tests/app_state.rs` | the arming rule of §3.0 — every app-initiated change of `active_session` arms, `SessionSelected` does not — and the non-arming events of §3's table, including that closing the current session promotes no successor (FR-001a) |
| `tests/app_state.rs` | commit-on-clear: a location stops holding the current session and stays open (§2.3, FR-001c) |
| `tests/sidebar_state.rs` | `WorktreesLoaded` / re-discovery neither closes a forced row nor clears suppression (§2.2, SC-008) — the file that already covers `set_worktrees`'s pruning |
| `tests/switch_active.rs` | the switch path arms a reveal, and view state still does not carry between projects (FR-007) |
| `tests/features_are_render_free.rs` | the new predicate and metrics stayed out of the rendering layer |
| `tests/material_builder_api.rs` | `Scrollable` still constructs with required inputs and terminates in `.into()`; `id` and `on_viewport_resize` are chainable, not positional |
| `src/ui/material/anatomy_size.rs` | unchanged figures — this feature adds no anatomy |
| `micold-core/tests/tokens_contrast.rs` | the current row's pairing (`secondary_container` / `on_secondary_container`) still clears AA in both schemes |
| `tests/type_role_call_sites.rs` | the 500-weight session name is a role from the scale, not an ad-hoc font weight at a call site |
| `tests/showcase_glue.rs` | the render glue still holds no branch on state — the check that keeps Principle I's exception honest |

**What §A cannot tell you**: whether the 500-weight name is actually distinguishable, and whether the
row is inside the viewport on the frame after a switch. Both are §B.

---

## §B — The manual pass

```bash
mise run run     # the application; it spawns/attaches the daemon itself
```

Prerequisites: two projects registered, each with at least one worktree **and** at least one session.
For B4, one project with enough worktrees that the list overflows the panel — 30 is SC-003's figure.

### B1 — The reported bug is gone (US1, SC-001, SC-002)

1. Open project A, start a session in a worktree, leave it running.
2. Switch to project B; start a session in *its* worktree.
3. Switch back to A.

The row holding A's restored session is **already open** and that session's row is marked, on the
first frame — not after a beat, and with no collapse-then-expand flicker. Watch for a frame in which
the main area shows a session while the panel shows nothing marked; that frame is the bug, and
SC-002 forbids it.

Repeat with a session in the **Default** (project-root) row. Same result — FR-001 is not a
worktree-only promise.

### B2 — The mark reads without colour (FR-003, FR-003a, §4.2)

With the current session's row on screen:

1. Its name is visibly heavier than the sibling session rows in the same location.
2. Hover a *different* session row. The hovered row and the current row are tellable apart — the
   hover state layer must not read as "current".
3. Hover the current row. It reads as both, neither signal cancelling the other.
4. Check the row's activity dot and its lifecycle-tinted name are unchanged by being current (§4.3).
5. Screenshot both schemes, then look at them in greyscale. The current row must still be
   identifiable — this is the whole of FR-003a and the only way to check it.

Stop a session while it is current: it keeps the mark (§4.4, FR-015).

### B3 — A close sticks (FR-005, SC-006)

1. After a switch, collapse the row the app opened.
2. It stays collapsed. Create a worktree, or trigger a re-discovery, and it is *still* collapsed
   (SC-008) — the case a one-shot implementation gets wrong.
3. Switch away and back. It is open again, because the current session changed and was re-revealed.

### B4 — It is actually on screen (US2, FR-008, FR-009, SC-003)

In the 30-worktree project, with the current session's location near the bottom:

1. Switch to that project. The current session's row is visible without scrolling.
2. Scroll it into view yourself, then switch away and back: the panel does not scroll needlessly
   (FR-009) — watch for a jump when nothing needed to move.
3. Scroll the panel after the reveal. It stays where you put it (FR-010, SC-007).
4. Repeat at both densities. This is where a row-height drift shows up as a row landing just off the
   viewport edge.

### B5 — Past the filters (US4, §5)

1. Apply a tag filter that excludes the location holding the current session. That location appears,
   open, marked, **and carrying a chip saying why it is there** (§5.4). No other excluded location
   appears (§5.2).
2. Confirm it sits where it would sit unfiltered, not pinned to the top (§5.5).
3. Confirm the filter chip row itself gained nothing — the exempt row offers no new filter (§5.6).
4. With agent worktrees hidden and the current session inside one: same result, other agent
   worktrees still hidden.
5. Select a session in a location the filter *does* admit. The exempt row disappears (§5.3) — but if
   it was open, note that its open state survives; only its presence goes.

### B6 — Nothing else moved

Start a session, select a session by clicking, close the current session. In order: the new session
is revealed and marked; the clicked one is marked with nothing opened or scrolled (FR-006); the
closed one leaves nothing marked, no row closed on your behalf, and no successor promoted
(FR-001a, US3 scenario 3).

---

## Recording the pass

§B is evidence, recorded the way features 006, 010, 020, 021 and 022 recorded theirs: the date, the
platform, and any step that did not behave as written. A step that fails is a defect, not a note.

**On honesty about what was checked.** B1, B2 and B4 are this feature's three headline claims and
none of them can be automated — B1 is about a single frame, B2 about perceptual weight, B4 about
geometry against a real viewport. A green §A is not this feature working. If §B was not run, say so
here rather than leaving the table blank and implying it was.

| Recorded | |
|---|---|
| Date | 2026-08-10 |
| Platform | Linux. **Xvfb :77 + Mesa lavapipe (software Vulkan), not a real display**, driving `micold-showcase` — see the caveat below |
| B1 — first frame after a switch, worktree and Default | **NOT RUN** — needs the client with a live daemon and two projects holding real sessions |
| B2 — weight, hover, greyscale, both schemes | **PARTIAL.** Weight and greyscale ✅ (evidence below). Hover-vs-current, and the second scheme, **not run** — both need the sidebar in the client |
| B3 — close sticks across a worktree refresh | **NOT RUN** — needs live sessions. Covered automatically by `tests/sidebar_state.rs` and `tests/app_state.rs`, which is not the same as seen |
| B4 — 30 locations, both densities, no needless scroll | **NOT RUN** — this is the geometry check, and it is the one §A cannot stand in for |
| B5 — one exempt row, chipped, in place | **NOT RUN** — needs live sessions |
| B6 — the three non-arming paths | **NOT RUN** — needs live sessions |

### What was actually verified, and what it cost

**B2's weight claim is verified.** `SidebarSession` (400) and `SidebarSessionCurrent` (500) were
cropped at identical geometry, stacked, magnified 6× and converted to greyscale. The current role is
visibly heavier with colour removed, which is the whole of FR-003a — see
`evidence/b2-weight-greyscale.png`.

**Getting there found a real defect, in the gallery rather than in this feature.**
`showcase/main.rs` registered the icon font but neither Roboto face, so every weight-500 role fell
back to a serif and rendered *lighter* than the weight-400 roles beside it. The gallery showed the
distinction backwards, on the one screen a reviewer would go to in order to check it — and it had
been that way since the gallery was built, affecting `Caption`/`Label` and `Body`/`Action` as much
as this feature's pair. Fixed in the same change; `evidence/b2-type-scale.png` is the corrected
scale.

**Why the rest is unrun, stated plainly.** B1, B3, B4, B5 and B6 all need the *client*, with a
session daemon and at least two projects holding real `claude` sessions. The showcase renders
components, not the application, so it cannot reach any of them. B1 (one frame after a switch), B4
(geometry against a real viewport) and B2's hover comparison remain exactly what this quickstart
said they were: claims a green §A does not establish. **This feature has not been seen working end
to end.**

### Screenshots

`mise run screenshot` pulls a frame off the monitor's PipeWire node — the only route to a screenshot
on a stock GNOME/Wayland session. Worth capturing: B1's first frame after a switch, and B2's pair of
schemes (which is what a reviewer can re-check in greyscale later without re-running anything).
