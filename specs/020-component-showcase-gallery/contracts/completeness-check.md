# Contract: the completeness check

**Feature**: [020-component-showcase-gallery](../spec.md) | Covers FR-011–FR-016, FR-013a,
SC-002, SC-003, SC-003a, SC-004

The check is an integration test in `crates/micold-client/tests/showcase_completeness.rs`. It
compares two sets — what the library contains, and what the gallery declares — and fails in both
directions. It is built in the shape of 017's existing gates because they are the reason it can
exist (spec, *Relationship to other features*).

---

## §1 What it reads

| Side | Source | How |
|---|---|---|
| The library | `src/ui/material/*.rs`, `src/ui/cdk/*.rs` | The **shared inventory module**, `tests/inventory/mod.rs` — the same code `tests/material_builder_api.rs` uses (FR-014). |
| The gallery | `micold_client::showcase::catalogue::{COMPONENTS, MOTION, EXEMPTIONS}` | As data, through the library's public API. Not by scanning the gallery's source. |

The definition of "a component" is **not restated here**. It is
`inventory::Declared::is_component()`: a `pub struct` under the two library directories that either
converts into something (`From<Self> for …`) or is a documented terminal type. Changing it changes
both gates at once, which is what FR-014 asks for.

Components are keyed by **`(module, component)`**, because `material/surface.rs::Surface` and
`cdk/overlay.rs::Surface` are different components, and because `material/animation.rs` declares
both a `Fade` wrapper and a private widget-tree tag of the same name. Duplicates within one module
collapse to one.

## §2 The rules

Each rule is one test, and each failure names the thing.

| # | Rule | Requirement | Failure message names |
|---|---|---|---|
| C1 | Every component in the inventory has an `Entry` **or** an `Exemption`. | FR-011, SC-002 | the module and component with no entry |
| C2 | Every `Entry` names a component the inventory still finds. | FR-012 | the stale entry |
| C3 | Every variant name declared by a `pub enum` anywhere in the library is named by some `Entry`'s `variants` — from any module. | FR-013, SC-003 | the enum, the module it lives in, and the variant with no instance |
| C4 | Every name in an `Entry::variants` still exists as a variant of some library `pub enum`. | FR-013 | the entry and the vanished variant |
| C5 | Every `pub fn` in `material/animation.rs` has exactly one `MotionEntry`. | FR-013a, SC-003a | the animation with no entry |
| C6 | Every `MotionEntry::animation` names a function that still exists. | FR-013a | the stale motion entry |
| C7 | Every `Exemption` names a component the inventory still finds, and carries a non-blank reason. | FR-015 | the stale exemption, or the entry missing a reason |
| C8 | No component appears in both `COMPONENTS` and `EXEMPTIONS`; `(module, component)` is unique within each. | FR-011/FR-015 | the duplicate |
| C9 | Every `Entry` whose `section` is `Motion` is a component that the library implements as an animation, and vice versa. | FR-007a | the entry in the wrong section |
FR-005's caption rule is deliberately **not** here. It reads only the catalogue and needs no
inventory, so it belongs to the catalogue's own contract and its own test —
[gallery-catalogue.md §4](./gallery-catalogue.md) and `tests/showcase_captions.rs`. This file
describes one test; splitting that would be the first step toward two places to look.

**Why variant attribution is library-wide rather than per-module.** An earlier draft required a
variant to be covered by an entry for a component *in the same module*. That rule is unsatisfiable
for `cdk/overlay.rs`: it declares `pub enum Anchor { Point, TopEnd, Center }`, and both of its
components (`Surface`, `Overlay`) are exempted as behaviour-layer hosts with no appearance of their
own — so no entry could ever carry `Anchor`'s variants, and the check could only be made green by
weakening it during implementation. Library-wide attribution matches what the spec actually asks
("every variant has an instance in the gallery") and is honest about where an anchor is visible: in
the floating section, because every floating component converts into a `cdk::Surface` with one.

**Variant identity is the name.** Several of these enums carry payloads —
`Kind::Notification(NoticeLevel)`, `Kind::Chip(Rgb)`, `Anchor::Point(Point)`,
`Anchor::TopEnd { top, end }`. The scanner extracts the name only, and one instance per name
satisfies C3; which payload it poses is the entry's choice.

## §3 The vacuity guards (FR-016)

A check that finds nothing must fail, not pass. These are separate tests, so a relocation is
reported as a relocation rather than as a hundred missing components.

| # | Guard | Why |
|---|---|---|
| V1 | The inventory finds at least 30 components across both library directories (it finds 38 today). | The library moved or was renamed; without this the whole check passes over an empty set (FR-016). |
| V2 | The inventory finds both `material/surface.rs::Surface` and `cdk/overlay.rs::Overlay`. | Named landmarks, mirroring `material_builder_api::the_scan_actually_finds_the_library_components`. A scan that finds *some* files but not the cdk has half-moved. |
| V3 | `material/animation.rs` exists and yields at least one `pub fn`. | The motion category is enumerated from one file; if it moves, C5 would hold vacuously over nothing. |
| V4 | `COMPONENTS` is non-empty and `MOTION` is non-empty. | A gallery emptied by a refactor must fail rather than agree with an empty library. |

V1's threshold is a floor, not a count: it is there so a moved directory fails, and it must not be
tightened into a number that has to be edited every time a component is added.

## §4 What the check does **not** do

- It does not render anything. It reads `const` data and source text, so it runs on every platform
  the crate compiles on ([research R14](../research.md#r14--what-cross-platform-means-here-and-where-the-gates-run)) and needs no display.
- It does not compare appearance. Image diffing is Out of Scope; this check holds the gallery
  *complete*, and a person holds it *correct*.
- It does not reach the three element-producing free functions that are neither a component nor an
  animation helper (`menu_panel`, `glyph::icon`, `glyph::icon_colored`). FR-014 widens the
  definition by exactly one category, and this is recorded as a known limit in
  [research R5](../research.md#r5--the-motion-category-and-the-one-thing-neither-category-reaches) rather than left to be discovered.
- It says nothing about density. `Entry::density` is empty for every entry at delivery because no
  component honours a density step (FR-003a); the rule that would hold it is added by 018 in the
  same change that introduces the axis.

## §5 Demonstrating both directions (SC-004)

SC-004 requires each failure direction to be *observed*, not assumed. Both are demonstrated by
tests that assert the check's own logic against a synthetic inventory, so the demonstration re-runs
on every build instead of living in a commit message:

- a component present in the inventory and absent from a stub catalogue → C1 fails, and the
  message contains the component's name;
- an entry naming a component absent from a stub inventory → C2 fails, naming the entry.

The rule functions therefore take their two sets as arguments rather than reading the world
directly; the tests that read the real library are thin wrappers over them. This is what makes the
check's own failure behaviour testable rather than only its success.
