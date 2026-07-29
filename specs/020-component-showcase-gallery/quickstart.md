# Quickstart: validating the Component Showcase Gallery

**Feature**: [020-component-showcase-gallery](./spec.md) | **Date**: 2026-07-28

Two parts, split exactly the way the spec's Assumptions split them.

**§A — Automated.** SC-002, SC-003, SC-003a, SC-004, SC-007, SC-008, SC-009 and SC-010. Run the
commands; the suite is the verification. Nobody signs anything off.

**§B — Recorded manual walkthrough.** SC-001, SC-005 and SC-006 — a timing measurement and two
visual comparisons, the only three that need a person looking at the screen. These are the passes
this feature exists to make cheap, and the record below is what gets filled in.

Prerequisites: a trusted `mise.toml` (`mise trust`, once per worktree) and nothing else. No daemon,
no project, no repository.

---

## §A Automated checks

```bash
mise run test                 # the whole workspace, matching CI
```

That is the gate. The rest of this section says which test answers which criterion, so a failure
points at a requirement rather than at a file.

| Criterion | Test | What a failure means |
|---|---|---|
| SC-002 (100% of components present or exempted) | `showcase_completeness` C1, C7 | a component has no gallery entry and no recorded exemption |
| SC-003 (100% of named variants) | `showcase_completeness` C3, C4 | a variant has no instance, or an entry names one that is gone |
| SC-003a (100% of animations, replayable) | `showcase_completeness` C5, C6 | an animation has no motion entry, or an entry is stale |
| SC-004 (both failure directions demonstrated) | `showcase_completeness` §5 tests | the check no longer fails when it should — the more serious failure of the two |
| SC-007 (the application is unaffected) | the pre-existing suite, unchanged; `style_snapshot` | this feature changed the application; the style fixture must not move a byte |
| SC-008 (no showcase in the package) | `packaging_excludes_showcase` | the manifest or the desktop entry names the showcase, or the assets list moved |
| SC-009 (no frames at rest) | `idle_requests_no_frames` | something outside `cdk/motion.rs` asks the runtime for a frame |
| SC-010 (two launches render the same) | `showcase_determinism` | the gallery reads the clock, a random source, the environment or a file |
| FR-005 (captions name what is live) | `showcase_captions` | an interactive entry declares no live states, or a static one claims some |
| FR-017/FR-020 (no daemon, git or state) | `showcase_isolation` | the showcase names the store, settings, endpoint, git or the daemon |
| Principle I (the glue holds no decision logic) | `showcase_glue` | `gallery.rs` or `main.rs` grew a branch on showcase state |

Two of these are checks *about* checks and are worth running deliberately at least once:

```bash
# SC-004, both directions, without hand-breaking the tree:
cargo test -p micold-client --test showcase_completeness -- --nocapture
```

The §5 tests drive the rule functions against a synthetic inventory, so "adding a component without
adding it to the gallery fails the build, naming the component" is re-proved on every run rather
than once, by hand, in a commit that is no longer readable.

**Platform coverage.** This feature's gates run on Linux, macOS and Windows in CI — they read source
text, one `const` slice and a reducer, and open no window. The rest of the client suite stays
Linux-only, as today. The authoritative list is the step in `.github/workflows/ci.yml`; run it
locally with:

```bash
cargo test -p micold-client --test showcase_completeness --test showcase_determinism \
  --test showcase_isolation --test showcase_captions --test showcase_state --test showcase_glue \
  --test packaging_excludes_showcase --test material_boundary --test material_builder_api \
  --test idle_requests_no_frames
```

**The application is unchanged (SC-007).** Nothing in `§A` should require a fixture update. If
`style_snapshot` fails, this feature has changed an appearance, which FR-019 forbids — the fix is
the change, never `UPDATE_STYLE_SNAPSHOT=1`.

---

## §B Recorded manual walkthrough

Launch:

```bash
mise run showcase
```

### B1 — SC-001: from a clean machine to a named component, in under 30 seconds

**Setup**: a machine (or a container, or a fresh user) with no configuration for this application,
no project, and no git repository in scope.

1. Start a timer.
2. Run `mise run showcase`.
3. Scroll to any component you name in advance — pick it *before* launching, so the search is real.
4. Stop the timer.

**Pass**: under 30 seconds, one command, no setup step of any kind.

**Also confirm, while it is running** (US1's independent test):

- `pgrep -f micold-daemon` finds nothing that this launch started.
- No terminal session exists.
- No state directory or settings file was created.

| Recorded | |
|---|---|
| Date / platform | 2026-07-28 · Linux (implementation session) |
| Component named in advance | — **not yet measured** |
| Elapsed | — **not yet measured** |
| Daemon started? | **no** — verified: daemon count unchanged across a 12s launch |
| State written? | **no** — verified: launched with an isolated `HOME`/`XDG_CONFIG_HOME`/`XDG_DATA_HOME` in a directory outside any git repository. Nothing application-shaped was created: no `micold*` path, no JSON, no config or data directory. The only files written were `~/.cache/mesa_shader_cache` and `~/.cache/radv_builtin_shaders`, which the **GPU driver** writes for any Vulkan program — not the showcase's doing and not application state. |

> **Still outstanding**: the *timing* half — a person naming a component in advance, starting a clock,
> and scrolling to it. `showcase_isolation` holds the structural half on every build.
>
> **Corrected 2026-07-29.** This note previously required "a machine that has never built this
> project", and dismissed the measurement above for using an already-compiled binary. That is
> stricter than SC-001, which starts its clock at *"from launching the showcase"* and defines a clean
> machine as one with **no configuration** — no project, no repository, no settings — not one with no
> build. Compiling this workspace takes minutes, so the stricter reading made the criterion
> unsatisfiable by anyone, which is why this row sat unclosable rather than merely undone. A
> pre-built binary is exactly what SC-001 intends; what is missing is only the stopwatch.

### B2 — SC-005: hover and pressed across the whole library, in one pass

This is the pass feature 018's SC-002/SC-004 need, and the reason for this feature's timing.

1. Scroll from the top of the components section to the bottom.
2. Move the pointer along each row of interactive instances. Every one responds.
3. Press and hold each. The response is *stronger* than hover — not merely different.
4. Tab through a row. Focus is visible on each.

**Pass**: every interactive component in the library was hovered and pressed without leaving the
page, and none required producing an application state first.

| Recorded | |
|---|---|
| Date / platform | 2026-07-29 · Linux |
| Sections walked | all — top to bottom of the components section, in one pass |
| Components with no hover response | none |
| Components with no pressed response | none |
| Components with no visible focus | none |

> Walked and reported by the maintainer. Recorded as a clean pass over all 27 interactive entries;
> no per-entry table was kept, so this row attests to the pass, not to 27 individual observations.

> **Outstanding, and it needs a person.** Nothing here is automatable: hover, pressed and focus follow
> the pointer and the keyboard, and FR-004 forbids faking them. What *is* held automatically is that the
> page presents them honestly — `showcase_captions` fails the build if an interactive entry does not name
> its live states or a static one claims some, and it additionally asserts that at least ten entries are
> interactive, so this walk has something to walk. 27 of the 36 entries are interactive.

Anything in the last three rows is a defect in the **component**, not in the gallery — and finding
it is the gallery working.

### B3 — SC-006: light and dark, seconds apart

1. Note the appearance of several components — a filled button, a surface, a disabled instance, a
   notification banner.
2. Activate the showcase's scheme control.
3. Confirm every component on the page re-rendered.
4. Scroll to a section that was off screen when you switched. It is in the new scheme too.
5. Switch back. Nothing was restarted, and the host system's theme setting was never touched.

**Pass**: both schemes seen without a restart, every section in both, and the colours match what the
same component resolves in the application in that scheme.

| Recorded | |
|---|---|
| Date / platform | 2026-07-29 · Linux |
| Restart required? | **no** — structural: the control is a `SchemeToggled` message on the reducer, and `view` re-resolves `tokens::roles(scheme)` on every render, so no section can be left behind |
| Host theme changed? | **no** — structural: `showcase_isolation` fails the build if the showcase names `dark_light` at all |
| Sections still in the old scheme after switching | none, including sections off screen at the moment of switching |
| Components whose colours differ from the application | none |

> **Outstanding.** The last two rows are the comparison itself, and they are why this criterion is on
> the manual list. The first two are held by construction and by a gate, and are recorded as such rather
> than left blank.

The last row is the sharp one. A difference there is a defect in the **showcase** (spec, Edge
Cases) — never a licence to style the gallery's copy differently.

### B4 — the motion section

1. Press **Replay** on each motion entry. It plays again, from the start, as many times as asked.
2. Press **Reverse** where offered, and watch the exit.
3. Leave everything stopped and idle for a minute, with a continuously-running component's section on
   screen if one exists. Nothing moves and nothing spins. Confirm with a process monitor
   (`top -p $(pgrep -f micold-showcase)`, or the platform equivalent) that CPU sits at 0.0% across the
   window — SC-009's "sustained observation window" is this minute. The structural guarantee is what
   actually holds it: one `request_redraw`, behind `animating()`.

**Pass**: every animation in the library was watched at least twice on demand, and the page is inert
when nothing was asked for.

### B5 — floating surfaces and the narrow window

1. Open each floating component from its own section. Dismiss it with Escape and by clicking the
   scrim. The page is still there and still scrollable.
2. Open one, then another. Neither traps the page.
3. Drag the window very narrow. Sections reflow or scroll vertically; no instance is clipped out of
   view, and the page never scrolls horizontally.

**Pass**: no state in which a surface cannot be dismissed, and no clipped instance — because a
clipped instance reads as a missing one.

| Recorded | |
|---|---|
| Date | 2026-07-28 |
| Each surface opens from its section and dismisses | **yes** — Escape, scrim and the dialog's own Close |
| Two open at once traps the page | **no** — unrepresentable: `Showcase::open` is an `Option` |
| Page keeps its scroll position when a surface opens | **yes**, after a fix — it did not at first (see the overlay trap in `docs/development/component-showcase.md`) |
| Narrow-window reflow | **yes** (2026-07-29 · Linux) — sections reflow or scroll vertically, no instance clipped out of view, no horizontal page scroll |

---

## What this walkthrough deliberately does not do

- **No per-platform launch.** The showcase is a development tool no user installs; parity applies to
  it as a build target only (spec, Assumptions), and the components' own appearance parity belongs to
  the features that own them.
- **No screenshots or image diffs.** Out of Scope. This page makes human comparison cheap; it does
  not replace it.
- **No density row.** FR-003a is dormant: nothing honours a density step until 018 adds the axis, at
  which point 018 adds the rows and the check starts holding them.
