# Phase 0 Research: Material 3 Visual System

**Feature**: `specs/018-material3-visual-system` | **Date**: 2026-07-26

Findings here concern **visual** capability. Architecture findings — per-instance widget state, the token move, the boundary measurements — live in [feature 017's research](../017-material-component-architecture/research.md).

All findings are verified against the vendored sources actually in `Cargo.lock` —
`iced 0.13.1` / `iced_core 0.13.2` / `iced_widget 0.13.4` — not against documentation or memory.
Each entry names the file that was read.

---

## R1 — Does the rendering stack support drop shadows?

**Decision**: Yes. Elevation is implementable as specified. Use `iced::Shadow` on
`container::Style` and `button::Style`.

**Evidence**: `iced_core-0.13.2/src/shadow.rs` defines
`Shadow { color: Color, offset: Vector, blur_radius: f32 }`.
`iced_widget-0.13.4/src/container.rs:576` — `container::Style` has a `shadow: Shadow` field.
`iced_widget-0.13.4/src/button.rs:474` — `button::Style` also has `shadow: Shadow`.

**Consequence for the contract**: exactly **one shadow per widget**. The contract's §4 key +
ambient two-shadow model cannot be drawn as two layers; the contract already anticipated this
("Where the rendering stack supports only a single shadow per surface, the key shadow is
authoritative and the ambient shadow is folded into it by widening the blur"). That hedge is now
the confirmed path, so §4 must be flattened to one `(offset_y, blur, alpha)` triple per level.

**Rationale**: The spec's premise — "there is not a single shadow anywhere in the rendering layer"
— described a *usage* gap, not a capability gap. No new dependency and no custom renderer is
needed.

**Alternatives considered**: a `canvas`-drawn shadow behind each surface (rejected: reimplements
what the widget already offers, and would not compose with `container`'s own border/background);
a nine-slice shadow image (rejected: asset weight, and does not tint per scheme).

---

## R2 — Can type roles carry a line height?

**Decision**: Yes. `text(...).line_height(LineHeight::Absolute(px.into()))`.

**Evidence**: `iced_core-0.13.2/src/text.rs:91` — `LineHeight::{Relative(f32), Absolute(Pixels)}`.

**Rationale**: The contract's §2.2 line heights are absolute dp values, so `Absolute` maps
directly and no ratio arithmetic is needed at the call site.

---

## R3 — Can type roles carry a weight, and can the app ship its own typeface?

**Decision**: Yes to both. `iced::Font { weight, family, .. }` selects the weight;
`iced::application(..).font(BYTES)` registers an embedded font and `.default_font(F)` makes it
the app default.

**Evidence**: `iced_core-0.13.2/src/font.rs:74` — `Weight::{Thin … Normal, Medium, Semibold,
Bold … Black}`. `crates/micold-client/src/main.rs:373-374` already does exactly this for the icon
font: `.default_font(iced::Font::DEFAULT).font(micold_client::ui::MATERIAL_SYMBOLS_BYTES)`.

**Consequence**: Roboto 400 → `Weight::Normal`, Roboto 500 → `Weight::Medium`. Adding the two
faces is two more `.font(...)` calls plus swapping `.default_font(...)` to the Roboto family. The
Material Symbols font stays registered unchanged, so icons are unaffected.

**Alternatives considered**: relying on a system-installed Roboto (rejected — defeats FR-008's
cross-platform parity, which is the whole point of shipping it); a variable font (rejected in
clarification D2 — the scale needs only 400 and 500).

---

## R4 — Can every interactive element show a keyboard focus indicator? ⚠️

**Decision**: **No — this is a hard capability limit and FR-022 / SC-005 must be narrowed.**

**Evidence**:
- `iced_widget-0.13.4/src/button.rs:462` — `button::Status` is exactly
  `{ Active, Hovered, Pressed, Disabled }`. There is **no `Focused` variant**.
- `iced_core-0.13.2/src/widget/operation/focusable.rs:7` — the `Focusable` trait exists, but
  grepping `iced_widget-0.13.4/src/` shows only **two** implementors: `text_input.rs` and
  `text_editor.rs`.
- `iced_widget-0.13.4/src/text_input.rs:1532` — `text_input::Status` *does* have `Focused`.

So in iced 0.13 the only widgets that can hold keyboard focus at all are text inputs and text
editors. Buttons, list rows, tree items, menu items and chips cannot receive focus, and there is
no tab-ring to render.

**Mitigating finding**: the application has no keyboard navigation for those elements either.
`crates/micold-client/src/keymap.rs` maps arrow keys, Tab and PageUp/Down into **terminal escape
sequences** (`csi_letter`, `csi_tilde`) — it is the terminal keymap, not an app navigation model.
There is no widget-to-widget focus traversal today, so nothing regresses; the requirement was
simply unsatisfiable as written.

**Resolution**: narrow FR-022 to "every element that *can* hold keyboard focus" — text fields and
the select control — and correct SC-005, which currently conflates *interactive* with *focusable*.
Record this as accepted fidelity gap #2 alongside tracking (R6). Building a custom focus and
tab-traversal system across every widget is a large feature in its own right and is explicitly not
what this feature was scoped to do.

**Alternatives considered**: a bespoke app-level focus model with a `focused: Option<WidgetId>` in
state and manual Tab handling (rejected — it is a behavior addition, colliding with FR-036, and it
is a far larger change than the visual system this feature is about); forking iced widgets to add
a `Focused` status (rejected — violates Principle V's single-stack rule and Principle VIII's reuse
rule, and would have to be re-forked on every iced upgrade).

---

## R6 — What is the test invocation, and does `--no-default-features` still exist?

**Decision**: It does not. The command is `cargo test --workspace` (`mise run test`). This is [feature 017](../017-material-component-architecture/research.md)'s concern; recorded here because every task in this feature runs under the same command.

**Evidence**:
- `mise.toml` on the current tip: `[tasks.test] run = "cargo test --workspace"`, described as
  "Test the whole workspace (core + client + daemon), matching CI". A separate `test-core` task
  exists.
- `crates/micold-client/Cargo.toml` has **no `[features]` section at all** and depends on `iced`
  unconditionally. There is no `--no-default-features` build of the client any more.

**Note — pre-existing doc drift, out of scope for this feature**: `CLAUDE.md` still documents
`mise run test` as `cargo test --no-default-features --all-targets`. That is stale against the
repository's own `mise.toml` and predates this feature. Flagged, not fixed here.

---

## R7 — How are the tonal ramps produced without a new dependency?

**Decision**: Bake the **published Material 3 baseline scheme** role values as constant tables in
`micold-core`. No HCT color-science code, no build script, no runtime computation.

**Rationale**: Clarification D1 chose "bake M3 tonal ramps as core data" and clarification Q1
chose Material's own baseline seed `#6750A4`. Those two answers together collapse the problem:
because the seed is Material's *default*, the resulting tonal palettes and the light/dark role
assignments are already published reference values. Nothing has to be *derived* — the ramps are
transcribed, and the role→tone map in `contracts/design-tokens.md` §1.2 selects from them.

This keeps the D1 property that mattered: roles are `(palette, tone)` pairs, so contrast follows
structurally from the tone delta and a role added later cannot silently break AA.

**Alternatives considered**: implementing HCT/CAM16 tone solving in core (rejected — several
hundred lines of color science to reproduce values that are already published constants, and it
would need its own correctness tests); pulling in the `material-colors` crate (rejected —
Principle-level dependency vetting cost for a one-time transcription, and D1 explicitly chose no
new dependency); build-time generation (rejected in D1).

**Risk**: transcription error. Mitigated by testing the *invariants* rather than the digits — the
AA contrast test (FR-004) checks every pair, and a tone-monotonicity test asserts each ramp
decreases in luminance as tone decreases.

---

## R8 — Acquiring and vendoring Roboto

**Decision**: Vendor two static instances into `assets/fonts/`, following the exact pattern
already established for the icon font.

**Evidence of the existing pattern**: `assets/fonts/` currently holds
`MaterialSymbolsOutlined.ttf`, a `LICENSE` (Apache-2.0), and a `PROVENANCE.md` recording upstream
repository, the original variable-font artifact, the `fonttools varLib.instancer` command used to
produce the static instance, and the pinned axis values. `crates/micold-client` already has
`ttf-parser` as a **dev-dependency**, and `crates/micold-client/tests/icons_font.rs` already
asserts the shipped font actually contains the codepoints the code references.

**Consequence**: the same three artifacts and the same style of test are required for Roboto —
`Roboto-Regular.ttf`, `Roboto-Medium.ttf`, a license file, a `PROVENANCE.md` section, and a test
asserting both faces load and report the expected weight. Roboto is Apache-2.0, matching both the
repository license and the icon font's, so SC-012 is satisfiable.

**Open implementation detail**: whether to subset. The icon font deliberately ships **full**
coverage so that adding an icon never requires regenerating the binary. Roboto should follow the
same reasoning — ship the upstream full Latin/Greek/Cyrillic instance rather than subsetting to
today's strings, since UI copy changes constantly. Fallback for glyphs outside coverage (FR-013)
is provided by the renderer's own font fallback, which is why `.default_font()` rather than a
hard per-`text` font is the right registration point.

---

## R9 — Are list rows and menu items able to take state layers?

**Decision**: Yes, and cheaply — they are already `button` widgets.

**Evidence**:
- `crates/micold-client/src/ui/material/tree_view.rs:253-259` — "The whole row is a low-emphasis
  button when it has a press action", built as `button(body).style(style::text_button(r))`.
- `crates/micold-client/src/ui/material/menu.rs:70-73` — menu items are
  `button(content).style(style::text_button(r))`.

**Consequence**: FR-021 ("state layers on every interactive surface, not buttons alone") is far
less invasive than the spec's framing suggests. Rows, tree items and menu items already receive
`button::Status`, so applying the shared state layers is a change to the **style function** they
all share, not a restructuring of the widget tree. The spec's diagnosis was that state layers were
*applied* to buttons only — not that the other surfaces were incapable of them.

---

## R10 — What drives the app bar's elevate-on-scroll?

**Decision**: `scrollable(...).on_scroll(|viewport| Message::…)` on the sidebar's existing
scrollable, reading `viewport.absolute_offset().y > 0.0`.

**Evidence**: `iced_widget-0.13.4/src/scrollable.rs:166` — `pub fn on_scroll(mut self, f: impl
Fn(Viewport) -> Message + 'a)`. `scrollable.rs:1370` — `Viewport::absolute_offset() ->
AbsoluteOffset`. The sidebar already wraps its list in a `scrollable`
(`crates/micold-client/src/ui/sidebar.rs:143`).

**Consequence**: one new `Message` variant and one `bool` on view state. Per clarification Q5 the
sidebar is the driving signal. The elevation transition itself is animated through the existing
`Animator` (`crates/micold-client/src/motion.rs`) with a new `MotionKey`.

---

## R11 — Is letter-spacing / tracking really unavailable?

**Decision**: Confirmed unavailable. The accepted fidelity gap (FR-042) stands on verified ground.

**Evidence**: grepping `iced_core-0.13.2/src/text.rs` for `letter_spacing` and `tracking` returns
**no matches**. There is no shaping-level tracking control exposed anywhere in the text API.

---

## Summary of spec amendments this research forces

| # | Spec item | Change required | Why |
|---|-----------|-----------------|-----|
| 2 | FR-022 | Narrow "every focusable element" to text fields and select — the only focusable widgets that exist | iced 0.13 has no focus concept for buttons/rows/menu items (R4) |
| 3 | SC-005 | Split: hover/pressed apply to *interactive* elements; focus applies only to *focusable* ones | The criterion conflated the two (R4) |
| 4 | FR-042 | Add a second accepted fidelity gap for keyboard focus, beside tracking | R4 is a capability limit, not a defect |
| 5 | Contract §4 | Flatten key + ambient shadow into one shadow per level | One `shadow` field per widget (R1) |

Amendments 1–4 are applied to `spec.md` as part of this planning pass; amendment 5 is applied to
`contracts/design-tokens.md`. None of them changes a product decision made during clarification —
each replaces a mechanism the spec named before the workspace split, or records a capability limit
of the rendering stack.
