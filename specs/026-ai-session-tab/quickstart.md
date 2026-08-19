# Quickstart: Validating the AI Session as a Tab

Runnable validation for feature 026. Proves the strip is always visible, that it contains the
session's AI CLI process as a right-anchored unclosable tab, that exactly one tab is marked in every
state, that a stopped process says so, and that none of it breaks when the tabs outgrow the bar.
Maps each step to Success Criteria (SC-00x).

## Prerequisites

- `claude` CLI installed and on `PATH`.
- A git repository open as a project, with a session running (see feature 010's quickstart).
- Toolchains: `cargo` (stable). The GUI run needs the `gui` feature.

## Automated checks (fast, no GUI)

```bash
# Render-free: the marked tab is a total function of (mode, active_shell); the stopped predicate
# over both lifecycles; the menu's items and whether it opens at all.
mise run test-core

# The client's gates, including the tab geometry gates the AI tab now runs under.
cargo test -p micold-client --test layout_snapshot
cargo test -p micold-client --test terminal_tabs --test terminal_bar_stability

# Whole workspace, matching CI.
mise run test
```

Expected: `tab_children_fit::every_control_inside_a_tab_holds_its_touch_target` and
`::a_tabs_content_sits_on_its_tabs_midline` pass **for the AI tab as well** — the second is what
holds FR-010a's centred icon once the trailing slot is empty. `terminal_bar_stability` still passes:
the bar's child list did not gain a conditional member, and the strip stopped being one.

Two covered states move in `layout_snapshot.txt`, and both should be checked deliberately:
`session-terminal-instance-tabs` (the AI tab joins the strip) and **`session-terminal-bottom-bar`**
(which drew no strip at all until FR-003 — this is where the single-instance user's whole visible
change lands).

## Manual end-to-end (GUI)

```bash
mise run run
```

### 1. The strip exists before there is anything to switch between — SC-003, FR-003 (US1)

- Open a session with **no** Regular Terminal instances. **Expect**: the strip is visible in the
  bottom bar, showing the AI tab alone, carrying the indicator. This is the state feature 012
  deliberately rendered nothing in, so it is the one most likely to look like a stray control rather
  than a deliberate strip — judge that, not just its presence.
- Open one instance. **Expect**: a tab joins to the **left** of the AI tab; the AI tab keeps the
  right-hand end.

### 2. Exactly one tab is marked, always — SC-001, FR-005 (US1)

- In AI CLI mode: the AI tab carries the indicator, no terminal tab does.
- Switch to a terminal instance: that tab carries it, the AI tab does not.
- Press the **mode toggle** rather than a tab. **Expect**: the pane and the indicator move together
  (FR-008) — the toggle and the strip cannot disagree.
- At every point in the above, count the marked tabs. **Expect**: exactly one. Never zero, never
  two.

### 3. Reaching the AI CLI in one press — SC-002, FR-006/FR-007 (US2)

- From a displayed terminal instance, press the AI tab. **Expect**: the AI conversation, and no
  process started, stopped or restarted — the terminal instance is still running when you go back
  to it, at the same scroll position.
- Press the AI tab again while it is displayed. **Expect**: nothing at all — no flicker, no output
  disturbed.
- **Expect**: the AI tab has no close control, in any state (SC-005).

### 4. A stopped process says so — SC-007, FR-012–FR-012e (US3)

- With two instances open, `exit` in the one that is **not** displayed. **Expect**: its tab gains
  the stopped mark, without being selected, and the displayed instance's tab is unchanged.
- **Expect**: the mark is legible on an *inactive* tab and on the *active* one, and is not mistaken
  for the active indicator (FR-012a). Select the stopped tab and look again.
- Restart it from its tab's menu. **Expect**: the mark clears from that tab and no other tab
  changes.
- Stop the **AI CLI** process. **Expect**: the AI tab wears the same mark, in the same place.
- While a process is **starting or restarting**, look at its tab. **Expect**: no stopped mark — the
  bar's status text says `starting…` / `restarting…` instead (FR-012e). This is the check that
  fails if the mark was wired to "not running" rather than to "restartable".

### 5. The menu, and the silence — FR-006a, FR-006b

- Right-click the AI tab while the AI CLI is **running**. **Expect**: nothing happens. No empty
  panel.
- Right-click it while the AI CLI is **stopped**. **Expect**: a menu with **Restart** and **no
  Close**.
- Right-click a terminal tab. **Expect**: the same menu **with** Close.

### 6. Overflow — SC-008, SC-009, FR-002a–FR-002f (US1)

- Open instances until the tabs cannot all fit (about five at a 1280dp-wide window).
- **Expect**: the AI tab is still at the right-hand end **at full size**; the "+", the mode toggle,
  the session title and the status are all still present at full size. This is the check that fails
  on `main` today, before this feature: they are silently squeezed, and at enough instances they are
  laid out at zero.
- **Expect**: no tab is narrower than the others, ellipsised or missing — they scroll instead.
- Turn the mouse wheel over the strip. **Expect**: the terminal tabs scroll; the AI tab does not
  move.
- **Expect**: the edge with tabs beyond it is faded, and there are no scroll-arrow buttons.
- Scroll the **marked** tab out of view. **Expect**: the edge it lies beyond says so specifically.
  Then select any tab. **Expect**: the newly marked tab is scrolled back into view.

### 7. Sessions do not leak into each other — FR-011

- With two sessions open, each with several instances and different marked tabs, switch between
  them. **Expect**: each session's strip shows its own tabs, its own marked tab, its own marks and
  its own scroll position; nothing about one reflects the other.

### 8. Appearance — the half no gate can see

Run with the repo's `visual-pass` skill and record the result in `visual-pass.md`. In **both**
schemes:

- **The stopped mark against both tints.** A tab is active (accent) or inactive
  (`on_surface_variant`); the mark has to be legible against both and must not be mistaken for
  either. This is the one FR-012a names and the one a tonal cue would have failed.
- **The mark and the indicator together**, on a tab that is active *and* stopped.
- **The AI tab beside a terminal tab**: same width, same form, no container, icon on the midline,
  empty trailing slot that does not read as a missing control.
- **The edge fade**: visible when content is beyond, absent when it is not, and distinct when the
  marked tab is the thing beyond it. Drawn, not laid out — no geometry gate can see any of this.
- **The strip at zero instances**: one tab, marked, reading as a deliberate strip.
- **Squint test**: which tab is marked, and which tabs are stopped, both legible without reading.

Also re-run **feature 012's `quickstart.md` §8**. This feature changes 012's terminal tabs — every
one of them gains a slot — and §8 is that strip's appearance section.
