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
| Date | 2026-08-10 (§B2's showcase-only half); **2026-08-18 — §B run end to end against the client** |
| Platform | Linux. **Xvfb :77 + Mesa lavapipe (software Vulkan), not a real display.** The 2026-08-18 run drove the real client (`micold-ai-ide`) with its own daemon, three seeded git projects and six live `claude` sessions, via the repo's `visual-pass` skill — see the method note below. The 2026-08-10 half drove `micold-showcase`, which is why it could not reach any step but B2's token comparison |
| B1 — first frame after a switch, worktree and Default | **PASS**, both — [evidence](./evidence/B1-first-frame-after-switch.png). Switching away and back, the location holding the current session is already open and that session's row is already marked on the earliest frame the capture pipeline can take, and on the three consecutive frames after it. No collapse-then-expand flicker, and no frame with a session in the main area and nothing marked in the panel (SC-002). Repeated for a session in the **Default** (project-root) row with the same result |
| B2 — weight, hover, greyscale, both schemes | **FAIL — [BUG-001](./bugs/BUG-001.md).** The current session's name is **not** drawn heavier: normalised ink area 185.3 vs 184.8 in dark and 185.4 vs 185.0 in light (ratio 1.003 / 1.002, where a 400→500 step is worth 8–15%), and both labels measure 67–68px wide with a 9px cap height — [evidence](./evidence/B2-weight-absent-both-schemes.png). `Ellipsized::at_role` keeps only `role.size()` and draws with the renderer's default font, so the role the tree view selects for the marked row reaches the glyphs as a pixel size and nothing else. FR-003a's colour-independent channel does not exist in the sidebar. **Steps 2–4 pass**: hovering a different row is tellable apart from the current row in greyscale (fill 42 vs 86 against a 28 background), the current row hovered reads as both (86 → 96), and the activity dot and lifecycle tint are pixel-identical before and after the row becomes current — [evidence](./evidence/B2-hover-vs-current-greyscale.png), [evidence](./evidence/B2-activity-dot-unchanged.png). **Both schemes captured**; in light the mark survives greyscale on a 16-level fill step alone (228 vs 244) — [evidence](./evidence/B2-light-scheme-greyscale.png) |
| B3 — close sticks across a worktree refresh | **PASS**, all three — [evidence](./evidence/B3-close-sticks-then-reopens.png). Collapsing the row the app opened sticks across four consecutive frames and across seconds of further interaction. It then survives a real re-discovery: creating a worktree through the app's own form redrew the list with two new locations (one of them created behind the app's back) and left the collapsed row collapsed, and the user's own expansion of a different row expanded — [evidence](./evidence/B3-survives-rediscovery.png), SC-008. Switching away and back re-opens it, because the current session was re-revealed |
| B4 — 30 locations, both densities, no needless scroll | **FAIL — [BUG-002](./bugs/BUG-002.md).** Arriving at the 30-worktree project leaves the list at the top (`Default`, `301 w01` … `319 w19`); the current session's row, 28 locations down, is never scrolled into view — identical across the switch frames and after settling ([evidence](./evidence/B4-arrival-list-at-top.png)). Scrolling by hand shows the reveal's *other* half did run: `328 w28` is open and its session marked ([evidence](./evidence/B4-marked-row-below-the-fold.png)). So the row is opened for a session the panel never shows you. Reproduced on two arrivals. Step 3 (the list stays where you put it, FR-010/SC-007) holds — the sidebar is pixel-identical five seconds after a manual scroll — but that is worth little while nothing moves the list at all. Step 4, "repeat at both densities", **has no user-facing route in this build**: `SIDEBAR_DENSITY` is a compile-time constant and settings carries no density field, so there is no second density to repeat at |
| B5 — one exempt row, chipped, in place | **PASS**, all five — [evidence](./evidence/B5-exempt-row-chipped.png). Filtering to `fix` while the current session sits in a `feat` worktree leaves that location showing, open, marked, and carrying a **`current session`** chip beside its type tag (§5.4); no other excluded location appears (§5.2); it sits where it would sit unfiltered rather than pinned to the top (§5.5); and the chip row gains nothing (§5.6). With agent worktrees hidden and the current session inside one, the same holds and the *other* agent worktree stays hidden — and the `untyped` filter chip that existed only because of those rows goes with them. Selecting a session in an admitted location makes the exempt row disappear entirely (§5.3) |
| B5 — re-run 2026-08-19 under the `MICOLD_SIDEBAR_FILTER` hook | **Steps 1, 2 and 5 PASS** — [evidence](./evidence/B5-exempt-row-under-hook.png). With `MICOLD_SIDEBAR_FILTER=feat` and the current session in `W25` (a `fix` worktree), the excluded location appears anyway, expanded, its session marked, carrying `fix`, `ABC-125` **and a `current session` chip** (§5.4); the four other `fix` worktrees do not appear (§5.2); and it sits after `W30` — where it sits unfiltered — rather than pinned to the top (§5.5). Starting a session in `W30`, which the filter admits, removed the exempt row from the list entirely (§5.3). **Steps 3 and 4 not driven**: the chip row's own contents (§5.6) and the agent-worktree case need the filter *panel* open, which is the thing the hook exists to avoid. The arrival also **scrolled** — 479 — so B4 holds with a filter on |
| B6 — the three non-arming paths | **PASS**, all three — [evidence](./evidence/B6-click-and-close.png). Starting a session reveals and marks it. Clicking an already-visible session marks it with nothing else opened and no scroll (FR-006). Closing the current session leaves nothing marked, closes no row on your behalf, and promotes no successor, with other sessions available to promote (FR-001a, US3 scenario 3) |

### Method, so the run can be disbelieved or repeated

Xvfb `:77` at 1600×1400 + lavapipe, its own `XDG_DATA_HOME` and a short `XDG_RUNTIME_DIR`
(`/tmp/vp77` — the scratchpad path exceeds `sun_path`), driven with `xdotool`. Three throwaway git
projects: two small ones and a 30-worktree one for SC-003. Sessions were created **through the UI**,
so they are real `claude` processes with real terminals, not seeded records — hand-seeded sessions
are archived by the daemon's own reconciliation on attach, which is worth knowing before trying to
shortcut this.

**Both binaries were pinned to a private directory, and the first attempt was wrong.** The first copy
out of `target-shared/` took a `micold-daemon` another worktree had just built, and the client came
up with a contract-mismatch banner (client v6, daemon v5) — the one symptom that says the pair is
mixed. Rebuilt, re-copied, and confirmed banner-free before any of the above was recorded.

**Transcript saving was off** for the sessions (an inherited `CLAUDE_CODE_CHILD_SESSION` marker), so
nothing here says anything about what survives a restart. Nothing in §B asks it to; 025's §B does.

### What this run could not answer

The two the skill names, unchanged: a chosen frame of a 150ms transition, and perceived smoothness on
real hardware. B1's "first frame" is therefore the earliest frame `import` can capture plus the three
after it, not a guarantee about frame zero — a wrong intermediate frame shorter than the capture
interval would not be seen. Every other B1 claim (already open, already marked, no flicker across
consecutive frames) is directly observed.

### The 2026-08-10 half, and why its PASS did not survive

**B2's weight claim was recorded as verified on 2026-08-10 and was wrong about the sidebar.**
`SidebarSession` (400) and `SidebarSessionCurrent` (500) were cropped from `micold-showcase` at
identical geometry, stacked, magnified 6× and converted to greyscale, and the current role *is*
visibly heavier there — `evidence/b2-weight-greyscale.png` is a real image of a real difference. The
showcase poses those roles through `Text`, which applies `role.font()`. The sidebar draws its session
rows through `Ellipsized`, which does not. The tokens differ; the screen does not. See
[BUG-001](./bugs/BUG-001.md).

**Getting there found a real defect, in the gallery rather than in this feature.**
`showcase/main.rs` registered the icon font but neither Roboto face, so every weight-500 role fell
back to a serif and rendered *lighter* than the weight-400 roles beside it. The gallery showed the
distinction backwards, on the one screen a reviewer would go to in order to check it — and it had
been that way since the gallery was built, affecting `Caption`/`Label` and `Body`/`Action` as much
as this feature's pair. Fixed in the same change; `evidence/b2-type-scale.png` is the corrected
scale.

**The lesson worth keeping**: a component gallery answers questions about components. Both of this
feature's headline claims are about the application, and both of the defects above waited in the gap
between those two sentences until §B was run against the client.

### Screenshots

Captured with `import -window root` on the private Xvfb display, per the `visual-pass` skill — not
with `mise run screenshot`, which pulls a frame off the logged-in desktop's PipeWire node and puts
whatever is on the user's monitor into the record. Comparisons are cropped at **identical geometry**
and stacked, so a difference in the image is a difference in the pixels rather than in the framing;
the weight comparisons are magnified 6× and desaturated, which is the only way to judge a 400-vs-500
step, and are backed by a measured ink area because that judgement is exactly the one an eye
argues about.
