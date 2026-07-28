# Contract: the gallery catalogue

**Feature**: [020-component-showcase-gallery](../spec.md) | Covers FR-001–FR-007c, FR-021–FR-023a

The catalogue is the gallery's public surface: the one list a developer edits when adding a
component, and the one the completeness check reads. Its shape is a contract because two things
depend on it — the page and the gate — and they must not be able to disagree.

Declared in `crates/micold-client/src/showcase/catalogue.rs`, exposed as
`micold_client::showcase::catalogue`. Field-by-field meanings are in
[data-model.md](../data-model.md); this states the rules a catalogue must obey and the API it
presents.

---

## §1 The shape

```rust
pub const COMPONENTS: &[Entry];
pub const MOTION:     &[MotionEntry];
pub const EXEMPTIONS: &[Exemption];
```

Three `const` slices. No lazy initialisation, no builder, no registration at startup: the catalogue
is fully known at compile time, which is what makes FR-022's "the same components, the same
ordering, on every launch" structural rather than arranged.

## §2 An entry carries its own instance

```rust
pub render: for<'a> fn(&'a Showcase, Roles) -> Element<'a, Message>,
```

Every `Entry` and `MotionEntry` holds a function that builds its instances. The page is produced by
iterating the catalogue and calling them.

This is the load-bearing decision. It means:

- an entry cannot be declared without something to show (FR-011 would otherwise be satisfied by a
  name alone);
- an instance cannot be shown without being declared (FR-012's direction, arriving through the
  renderer instead of the list);
- the check reads names while the page reads renderers, and neither can drift from the other,
  because they are fields of the same value.

`render` receives the whole `Showcase` so an entry can reach the replay counter it owns and the
sample grid it draws from, and `Roles` so it resolves the active scheme's tokens — never a resolved
colour, and never a style value (FR-010).

The three `const` slices live in one file, `catalogue.rs` — the one list a developer reads. The
`render` functions they point at live in
`sections/{atoms,controls,surfaces,floating,terminal,motion}.rs`, grouped so section work can be
split across commits without splitting the list.

## §3 What a `render` may and may not do

**Must**: build the real component from `micold_client::ui::material` / `::cdk`, live and
interactive, with every posed state as its own instance side by side (FR-002, FR-003).

**Must not**:

- fake hover, pressed or focus with a static approximation — those are exercised on the instance
  itself (FR-004);
- style a widget, call `.style(...)`, name a raw text size, or construct a rendering-stack widget
  the library wraps (FR-021 — enforced by the widened boundary gate, so this is a build failure and
  not a review note);
- read the clock, the environment, the filesystem, or a random source (FR-022 — enforced by the
  determinism gate);
- ask the runtime for a frame. Only `cdk::motion::Progress` may, and it only does so while moving
  (FR-023 — enforced by the widened idle-frames gate).

## §4 Captions are part of the entry, not the prose (FR-005)

`live` names the states a developer has to produce with a pointer or a keyboard. The gallery renders
it as the section's caption — "hover, pressed and focus are live: point at an instance" — so that a
state absent from the page is read as live rather than missing.

`interactive` is what makes that checkable. **Two rules, enforced by
`crates/micold-client/tests/showcase_captions.rs`** — the catalogue's own test, which reads no
inventory and is therefore not part of the completeness check:

| Rule | Statement | Failure names |
|---|---|---|
| **Agreement** | An `Entry` has a non-empty `live` if and only if it is `interactive`. | the entry, and which half is wrong |
| **Non-vacuity** | At least one `Entry` is `interactive`. | that the whole catalogue claims to be static |

An interactive entry with an empty `live` is a caption bug that tells a developer nothing is expected
where something is. A non-interactive entry with a populated `live` promises a response that never
comes. And a catalogue in which nothing is interactive would satisfy the agreement rule while saying
nothing at all — the same vacuous pass FR-016 guards against elsewhere, in miniature.

## §5 Triggers (FR-007b, FR-023a)

An entry that animates owns an index into `Showcase::replays` and renders its own control:

- **Replay** bumps the generation counter; the wrapper sees a changed identity through
  `.restart_on(key)` and plays the transition again, from zero, as many times as asked.
- **Reverse** flips `shown`, so the exit is watchable too — Material exits are quicker than
  entrances, and an entry that could only be entered would hide half the motion spec.
- **Run / Stop** (FR-023a) drives a component whose appearance runs continuously. At rest it is
  stopped and asks for no frames; the developer's press is the operation the indication reports on.
  **No entry uses this at delivery** — nothing in the library runs continuously yet — and the
  mechanism is here so 018's indeterminate indicator plugs in without the catalogue changing shape.

There is no timer, no subscription and no animation clock anywhere in the showcase. A trigger is a
value change; the component does the rest.

## §6 Ordering

The page's order is the catalogue's order, and it is fixed (FR-022, SC-010). Entries are grouped by
`section` — components first, motion second — and within a section they appear in declaration order.
`Showcase`'s per-entry arrays are indexed by catalogue position, so reordering the catalogue is a
deliberate edit rather than an incidental one.

## §7 Adding a component to the gallery

The whole procedure, which the developer documentation (FR-024) restates for its own audience:

1. Add an `Entry` to `COMPONENTS` naming the component's module and type.
2. List its named variants, and any other posed state (`disabled`, `selected`, …).
3. List what has to be exercised live.
4. Write `render` from the real component and `samples`.
5. If it has no appearance of its own, add an `Exemption` with the reason instead of an `Entry`.

The build tells you when you have not: the completeness check names what is missing, in either
direction.
