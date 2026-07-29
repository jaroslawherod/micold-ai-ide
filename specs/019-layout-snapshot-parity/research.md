# Phase 0 Research: Layout Snapshot Parity Gate

**Feature**: `specs/019-layout-snapshot-parity` | **Date**: 2026-07-28

Every finding below was verified against `iced 0.14.0` as vendored in this workspace, or measured
by a throwaway spike run under `mise exec -- cargo test -p micold-client`. Measured numbers are
quoted as observed rather than as expectations. The spikes were deleted; what they established is
recorded here.

---

## R1 — Resolving layout with no display, no GPU and no window manager (FR-001)

**Decision**: Construct the concrete `iced::Renderer` through
`<iced::Renderer as iced::advanced::renderer::Headless>::new(font, size, Some("tiny-skia"))`,
then call `Widget::layout` on the element returned by `ui::view`.

**Rationale**. The renderer type is not a free choice. `ui::view` returns
`iced::Element<'a, Message>`, which is `Element<'a, Message, Theme, iced::Renderer>` — the
*concrete* renderer, not a generic parameter. A hand-rolled measuring renderer therefore cannot be
substituted without changing the application's own signature, which this feature is not permitted
to do (FR-019).

What makes the concrete renderer usable anyway is that this workspace takes `iced`'s default
features (`crates/micold-client/Cargo.toml` sets no `default-features = false`), so both `wgpu` and
`tiny-skia` are enabled and

```text
iced::Renderer = iced_renderer::fallback::Renderer<iced_wgpu::Renderer, iced_tiny_skia::Renderer>
```

a public enum with public variants. Its `Headless::new` tries the primary first and falls back to
the secondary. Passing the backend hint `Some("tiny-skia")` makes `iced_wgpu`'s implementation
return `None` on its *first line* — before it constructs a `wgpu::Instance` or requests an adapter
— so the GPU is never probed, let alone required. `iced_tiny_skia`'s implementation accepts the
hint and is a plain struct construction: no device, no surface, no window.

**Measured**: the renderer constructs and reports `name() == "tiny-skia"`. Laying out `ui::view`
with `State::default()` at 1280×800 produced a **50-node** tree with a 1280×800 root, in 0.85s
including compilation of the test binary.

**Alternatives considered**:

- **The built-in null renderer `()`** — rejected, twice over. Its `text::Paragraph` implementation
  returns `Size::ZERO` from both `bounds()` and `min_bounds()`, so every string measures 0×0 and
  every ancestor that hugs its content collapses with it; the recorded geometry would be fiction.
  Measured: the same string that shapes to 244.5×20.8 under real shaping measures 0.0×0.0 here.
  Separately, `iced_core::renderer`'s `mod null` is `#[cfg(debug_assertions)]`, so it does not
  exist in a release-profile test build.
- **`iced_wgpu` headless** — rejected. It requires a real adapter, which FR-001 forbids and which
  no CI runner can be relied on to provide identically.
- **A custom `Renderer` implementation in-repo** — rejected as impossible, per the concrete-type
  argument above, not merely as undesirable.
- **Pixel screenshots.** `Headless` also exposes `screenshot()`, and tiny-skia rasterises on the
  CPU, so image snapshots are genuinely available here. Rejected for this feature because the spec
  chose geometry: a pixel diff says "something changed" where FR-004 requires naming the element
  that moved, and image fixtures are unreviewable in a pull request. Recorded because it is a real
  option for a later feature, not because it was overlooked.

---

## R2 — Deterministic text measurement across machines (FR-006, resolves D1)

**Decision**: The snapshot constructs its own renderer, and therefore chooses its own default
font. Commit Roboto as a test fixture and pass it as that default. Text metrics become identical on
every machine immediately, with text-derived geometry fully in scope, independent of feature 018's
schedule.

**Rationale**. The threat is real and was measured. `iced_graphics::text::font_system()` builds its
global `cosmic_text::FontSystem` via `new_with_fonts`, whose `load_fonts` calls
`db.load_system_fonts()` **unconditionally**, then sets `sans_serif_family` to `"Open Sans"` and
installs a per-operating-system `PlatformFallback` chain. Measured on this machine: **391 font
faces** in the database. `iced::Font::DEFAULT` is `Family::SansSerif`, so it resolves through that
host-dependent table. A fixture built on those metrics would pass only where it was generated —
exactly what the spec rules out.

What makes the fix cheap is a property of this codebase: **`crates/micold-client/src/ui/material/text.rs`
sets no font at all.** Body text carries no explicit `Font`, so it falls back to the renderer's
`default_font()` — and the snapshot owns that renderer. Pinning one font at construction pins
every measurement the fixture records.

**Why not the two orderings the spec listed**:

- *Sequence after 018* — 018 is 0 of 76 tasks implemented, and it changes row densities, app-bar
  height, dialog padding and touch targets. Waiting means 018 makes exactly the class of change
  this gate exists to watch, unwatched, and 019 delivers nothing meanwhile.
- *Structural coverage first, excluding text-derived widths* — the exclusion does not stay local.
  A text node's width propagates into every ancestor that shrinks to fit it, so excluding it
  removes most of the tree rather than one leaf category. It would produce a gate far narrower
  than its name implies, which is the specific failure FR-015 exists to prevent.

**Relationship to 018**. This does not front-run 018's product decision. 018's FR-008/FR-008a make
Roboto the *application's* font because users should see the same typeface everywhere; 019 makes
Roboto the *fixture's* measuring basis because CI should compute the same number everywhere. When
018 lands, the app default becomes Roboto too, the override becomes a no-op in spirit, and the
fixture regenerates once. **Coordination note for 018's T015**: reuse this same font file rather
than committing a second copy.

**Honest limitation, to be documented under FR-015**: until 018 ships, the fixture records layout
under a typeface that is not the one users see. The gate detects *drift*, which is its job; it does
not certify production typography.

**Residual risk and its guard**. `load_system_fonts()` still runs, so a host with its own font
named `Roboto` could win the family-name lookup and shift every measurement — silently, since the
fixture would simply disagree everywhere at once. Mitigation: a guard test pinning a known
measurement of a known string, so a hijacked family fails loudly and specifically instead of
looking like a mass layout regression. This risk survives 018 unchanged — Roboto is a *common*
system font name — so the guard is permanent, not scaffolding.

---

## R3 — Element identity in the record (FR-002, FR-004)

**Decision**: Identify each element by its **path** through the layout tree (the child index at
each level, e.g. `0/2/1/0`), and pair it with the widget's own identity where the tree exposes one.
Order is the tree's own depth-first order, which is deterministic by construction.

**Rationale**. `iced::advanced::layout::Node` carries geometry and children and nothing else: no
type name, no tag, no id. Nothing in the layout pass can report "this is the sidebar's close
button". A depth-first index path is fully deterministic, stable under re-runs, and cheap.

Its weakness is honest and must be designed around: a path is stable only while the tree shape is
stable. Inserting a container near the root renumbers everything below it, so one structural edit
produces a fixture diff far larger than the change. That is tolerable — the diff is still correct,
still reviewable, and still names positions — but FR-004 asks for a failure that identifies *the
element*, and a bare path is a weak answer.

**Mitigation**: emit the path together with the geometry and, for covered states, a small
hand-maintained set of **named anchors** — a label attached to the path of an element the feature
cares about (the sidebar row's text, its close button, the dialog's action row). Anchors are what
FR-018's demonstration asserts against, and what a failure message quotes. They are cheap, they are
in one place (FR-016), and they degrade gracefully: an anchor whose path no longer resolves is
itself a failure, satisfying FR-014.

**Alternatives considered**: iced's `widget::Operation` traversal can reach widget `Id`s, but only
widgets that implement `operate` participate and most do not, so coverage would be partial and
silently so. Rejected for exactly the reason FR-015 exists.

---

## R4 — Numeric normalisation (FR-012)

**Decision**: Record each coordinate and dimension rounded to **one decimal place** (0.1 logical
pixel), formatted with a fixed number of digits so the text form is canonical.

**Rationale**. Resolved geometry is `f32` and genuinely fractional — measured values from the spike
include `87.4`, `20.8`, `244.52802` and `1003.1`. Committing full precision would make the fixture
flap on any change to shaping or rounding order. One decimal place is far below the threshold of a
visible difference (a tenth of a pixel is not perceptible, and the motivating overlap defect was
tens of pixels) while still distinguishing any real movement. Formatting must be explicit — a
fixed-precision decimal, never `{:?}` on an `f32` — so the same number always prints the same way.

**Note on `-0.0`**: normalise negative zero to `0.0` before formatting, or the fixture can differ
by sign on a value that is not different.

---

## R5 — Overlay coverage (FR-009)

**Decision**: Record two passes per covered state where one exists — the base tree, and any
widget-attached overlay obtained from `Widget::overlay`.

**Rationale**. This application builds overlays two different ways, and only one of them is covered
by a base-tree walk.

- **Dialogs, menus and the scrim** are composed *in-tree* (`material::Modal`, `material::Menu`).
  The spike confirms this directly: laying out `State::default()` produced a second top-level
  subtree at `[1052.0, 52.0 220.0×195.0]` alongside the main content, in the same pass. These need
  no special handling.
- **`material::Select`** wraps iced's `pick_list`, which is a genuine `Widget::overlay`
  implementor — its own module says so, and calls it "the mechanism every `pick_list`/`combo_box`/
  tooltip in any iced app already relies on". Its dropdown is laid out in a *separate* pass and is
  invisible to the base walk. `material::animation` and `material::navigation_drawer` also
  implement `overlay`, forwarding their child's.

An open `Select` dropdown is therefore only covered if the walk explicitly calls
`as_widget_mut().overlay(...)` and lays out the returned element. FR-009 requires it, so the record
must include it — and FR-015 requires saying so where it is not covered.

---

## R6 — Animated geometry (FR-010)

**Decision**: Record at the **resting** state — a freshly built `Tree`, with no frames advanced.

**Rationale**. Feature 017 moved every animation into the component that plays it, each owning a
`cdk::motion::Progress` in its widget-tree state. A `Tree::new(...)` built for the snapshot
therefore starts each component at its constructed value with nothing in flight, and 017's own
`Progress::animating()`-false-at-rest property is already unit-tested. "Resting" is thus both the
natural and the reproducible choice; it needs no clock, no frame pumping and no timing tolerance.

Consequence to document under FR-015: geometry that exists *only* mid-transition — a drawer caught
half-open — is out of coverage. Both endpoints are covered, because each is a resting state of some
covered configuration; the interpolation between them is not.

---

## R7 — Scroll-dependent geometry (FR-011)

**Decision**: Record at **offset zero**, the state of a freshly constructed scrollable.

**Rationale**. Scroll offset lives in the widget's tree state, so a fresh `Tree` is at the top by
construction — reproducible for free, with no way for a stale offset to leak in. Covering a
scrolled position would mean driving events into the tree before the walk, which adds a second
source of nondeterminism for little gain: what scrolling changes is which children are visible,
not how any of them is laid out.

---

## R8 — Where the check lives

**Decision**: An integration test at `crates/micold-client/tests/layout_snapshot.rs`, with its
fixture at `crates/micold-client/tests/fixtures/layout_snapshot.txt` and the reference font
alongside it.

**Rationale**. This differs deliberately from the style snapshot, which lives *inside* the crate at
`src/ui/material/style_snapshot.rs` — feature 017's T036 moved it there because it asserts against
`material::style`, which became `pub(crate)` and is unreachable from `tests/`. That constraint does
not apply here: this feature asserts against `micold_client::ui::view`, which is public, so the
ordinary integration-test location works and the crate stays free of test-only code.

The fixture directory `crates/micold-client/tests/fixtures/` already exists and already holds
`style_snapshot.txt`, so the shape is established.

**Covered-state construction** reuses what is already there: `State::default()` is the idiom in
`tests/logical_state_ownership.rs`, and `tests/support/mod.rs` already builds multi-project
workspaces from in-memory fixtures with a `FakeScanner` that touches no filesystem — which is
FR-007 satisfied by existing scaffolding rather than new work.

---

## R9 — Runtime cost (SC-006)

**Decision**: No special measure needed; verify rather than assume.

**Rationale**. The spike's two layout passes over the full view completed within a 0.85s test run
that included linking. Layout is a pure tree walk with cached text shaping; the dominant cost is
the one-time construction of the font system. SC-006's budget is 10 seconds locally and 10% of
suite runtime, and there is no evident mechanism by which a few dozen covered states would approach
it. Left as a measurement to record at delivery, not as a risk to design around.

**Outcome (2026-07-29): the rationale above was wrong, and "verify rather than assume" is what
caught it.** The dominant cost is not one-time font-system construction but shaping real text,
repeated per covered state and per scheme — about **2.21s for each covered state**, measured by
adding a tenth and removing it again. Nine states in two schemes is ~12s, and the gates together
are ~22.7s of a 35.1s suite.

The spike could not have shown this: it resolved a view twice, so it measured exactly the fixed cost
this note reasoned from and none of the per-state cost that turned out to dominate. The error was
not the estimate but generalising a two-pass measurement to "a few dozen covered states" without
varying the thing that scales.

Two consequences, both recorded rather than absorbed: SC-006 was amended to budget the suite and the
per-state growth instead of a binary and a share (see spec.md), and the caching in
`tests/support/layout.rs` exists because six tests were independently paying that per-state cost —
~71 full-view resolutions where 18 were needed.

---

## Summary of decisions

| # | Decision |
|---|---|
| R1 | Headless `iced::Renderer` via `Headless::new(..., Some("tiny-skia"))`; no GPU, no window |
| R2 | Pin a committed Roboto as the snapshot's default font — resolves D1 without waiting for 018 |
| R3 | Depth-first index paths, plus named anchors for the elements failures should quote |
| R4 | One decimal place, fixed-precision formatting, `-0.0` normalised |
| R5 | Two passes: base tree, plus `Widget::overlay` for widget-attached dropdowns |
| R6 | Resting animation state from a fresh `Tree` |
| R7 | Scroll offset zero from a fresh `Tree` |
| R8 | `tests/layout_snapshot.rs` + `tests/fixtures/`, reusing `tests/support/mod.rs` |
| R9 | Cost measured at delivery against SC-006 |

**No NEEDS CLARIFICATION remains.** D1 is resolved by R2 with the user's decision recorded.
