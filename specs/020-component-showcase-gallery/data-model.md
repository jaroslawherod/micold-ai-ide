# Data Model: Component Showcase Gallery

**Feature**: [020-component-showcase-gallery](./spec.md) | **Date**: 2026-07-28

Two kinds of data, kept apart on purpose:

- **The catalogue** — `const` declarations of what the gallery contains. Read by the view to render
  the page and by the completeness check to hold it complete. Immutable, no allocation, no clock.
- **The showcase's runtime state** — the scheme, and the triggers a developer presses. A small
  render-free reducer, unit-tested like `app::State` is.

Nothing here is persisted. The showcase reads and writes no file (FR-020).

---

## The catalogue (`micold_client::showcase::catalogue`)

### `Entry` — one component's place in the gallery

Corresponds to the spec's **Gallery section**.

| Field | Type | Meaning |
|---|---|---|
| `module` | `&'static str` | The library module the component is declared in, as the inventory scanner keys it — e.g. `material/button.rs`, `cdk/overlay.rs`. Half of the identity, because two modules each declare a `Surface`. |
| `component` | `&'static str` | The component's type name — e.g. `Button`. |
| `variants` | `&'static [&'static str]` | Named variants posed as separate instances (FR-003). Names must match the library's enum variants exactly (FR-013). |
| `density` | `&'static [&'static str]` | Density steps posed (FR-003a). **Empty for every entry at delivery** — no component honours a density step until 018 introduces the axis. |
| `posed` | `&'static [&'static str]` | Other states posed as separate instances: `"disabled"`, `"selected"`, `"unselected"`, `"empty"`, … (FR-003). |
| `live` | `&'static [&'static str]` | States that must be exercised with the pointer or keyboard rather than posed — typically `"hover"`, `"pressed"`, `"focus"`. Rendered as the section's caption so a state absent from the page reads as live, not missing (FR-005). |
| `interactive` | `bool` | Whether the entry's instances respond to a pointer or the keyboard. Drives FR-005's caption requirement, and is what makes it checkable: an interactive entry must declare a non-empty `live`, and a non-interactive one must declare an empty one — so the flag cannot be a shrug in either direction. |
| `section` | `Section` | `Components` for an ordinary row; `Motion` for a component whose appearance *is* an animation, whose instance lives in the motion section (FR-007a). |
| `layout` | `Layout` | `Inline` (chunked into rows with its siblings) or `FullWidth` (its own row), for a component whose natural size dwarfs its neighbours. |
| `render` | `for<'a> fn(&'a Showcase, Roles) -> Element<'a, Message>` | Builds every posed instance for this entry, live and interactive (FR-002). Carried by the entry so an entry cannot exist without an instance, nor an instance without an entry. |

**Invariants**

- `(module, component)` is unique across `COMPONENTS` and does not appear in `EXEMPTIONS`.
- Each `(module, component)` corresponds to a component the shared inventory finds (FR-011/FR-012).
- `variants` names exist in `module`'s `pub enum`s; every such variant is named by some entry for a
  component in `module` (FR-013).
- `render` resolves colours only through `Roles`; it never names a style value (FR-010, enforced by
  the widened boundary gate — see [research R13](./research.md#r13--the-showcase-is-bound-by-the-boundary-rule-fr-021-principle-viii)).
- `interactive` and `live` agree: non-empty `live` if and only if `interactive` (FR-005). At least
  one entry is `interactive`, so the rule cannot hold vacuously.

### `MotionEntry` — one animation's place in the motion section

Corresponds to the spec's **Motion entry**.

| Field | Type | Meaning |
|---|---|---|
| `animation` | `&'static str` | The helper's name as the library exposes it — `fade`, `expand`, `scale`, `scrim`. Matched against the `pub fn`s in `material/animation.rs` (FR-013a). |
| `label` | `&'static str` | What the entry is called on screen, so a developer comparing against a motion spec knows which one they are watching (FR-007c). |
| `render` | `for<'a> fn(&'a Showcase, Roles) -> Element<'a, Message>` | The demonstration, wired to this entry's replay trigger (FR-007b). |

**Invariants**

- `animation` is unique across `MOTION` and names a function that exists (FR-013a, both directions).
- Every animation helper the library provides has exactly one `MotionEntry`.

### `Exemption` — a recorded absence

Corresponds to the spec's **Exemption entry**.

| Field | Type | Meaning |
|---|---|---|
| `module` | `&'static str` | The module of the exempted component. |
| `component` | `&'static str` | Its type name. |
| `reason` | `&'static str` | Why it cannot be shown (FR-015). Mandatory; a blank reason is a check failure. |

**Invariants**

- Every entry names a component that **still exists** in the inventory (FR-015, on FR-012's
  reasoning): an exemption that outlives its component is a stale claim and fails the build.
- No component appears in both `EXEMPTIONS` and `COMPONENTS`.

Expected at delivery: the behaviour layer's host types, which decide *where* a floating surface
sits and have no appearance of their own (`cdk/overlay.rs::Overlay`, `cdk/overlay.rs::Surface`).
The exact set is whatever the inventory yields at implementation time; each carries its own reason.

### `Section` and `Layout`

```
Section = Components | Motion
Layout  = Inline | FullWidth
```

Both are plain enums with no behaviour. `Section` decides which of the page's two parts an entry
renders in; `Layout` decides whether it shares a row.

### Sample content (`micold_client::showcase::samples`)

Corresponds to the spec's **Sample content**. Fixed, invented data standing in for what a component
would display in the application (FR-006): labels and captions, a `TreeItem` list, `ProjectRow`s, a
menu-item list, and a `GridCache` built by applying one hand-written `GridFrame` so `TerminalPane`
renders real cells. All `const`/`static` or built by a pure function of no arguments — no clock, no
randomness, no environment, no filesystem (FR-022, guarded by the determinism gate).

Sample content belongs to the gallery, never to the component.

---

## Showcase runtime state (`micold_client::showcase::state`)

A render-free reducer: `Showcase` plus `Message` plus `update`. Unit-tested directly (Principle I);
the `view` that consumes it is the thin glue the GUI-wiring exception covers.

### `Showcase`

| Field | Type | Meaning |
|---|---|---|
| `scheme` | `ColorScheme` | The scheme every component on the page resolves against (FR-008). Starts `Light`; never read from the OS or from settings (FR-020). |
| `replays` | `Vec<u64>` | Per-entry generation counter. Bumping one replays that entry's transition via `.restart_on(key)` (FR-007b). |
| `running` | `Vec<bool>` | Per-entry run state, for a component whose appearance runs continuously (FR-023a). All `false` at rest; **no entry uses it at delivery** — nothing in the library runs continuously yet. |
| `shown` | `Vec<bool>` | Per-entry destination, so an exit transition can be watched as well as an entrance. |
| `open` | `Option<Floating>` | Which floating surface is open, if any (FR-007). One at a time, so the page is always reachable and can never deadlock. |
| `grid` | `GridCache` | The fabricated terminal grid `TerminalPane` renders from. Built once at boot from `samples`. |

All three are sized once at boot from `catalogue::COMPONENTS.len()` and indexed by catalogue
position, which is why the catalogue's order is fixed (FR-022). They are `Vec` rather than
`[_; COMPONENTS.len()]` deliberately: `Entry::render` names `&Showcase`, so an array length
const-evaluated from `COMPONENTS` would ask the compiler to resolve `COMPONENTS` and `Showcase`
through each other. A `Vec` sized from the same `const` costs nothing here — the length is still
derived from compile-time data, and nothing about determinism changes.

### `Message`

| Message | Effect |
|---|---|
| `SchemeToggled` | Flips `scheme`. Every component re-renders, including sections off screen (FR-009). |
| `Replayed(usize)` | `replays[i] += 1`, and sets `shown[i] = true` so the entrance is what plays. |
| `Reversed(usize)` | Flips `shown[i]`, so the exit can be watched. |
| `RunToggled(usize)` | Flips `running[i]` (FR-023a). |
| `Opened(Floating)` / `Dismissed` | Opens / closes a floating surface. `Opened` replaces whatever was open. |
| `NoOp` | For components whose messages the gallery does not act on — a button press in a catalogue has nowhere to go, and swallowing it keeps the instance genuinely interactive (FR-002) without inventing behaviour. |

### State transitions

There is no lifecycle to model. Every message is idempotent in shape (a toggle or a counter bump),
nothing is asynchronous, nothing can fail, and no state depends on another. That is a property worth
stating rather than a gap: it is what makes FR-022's "the same content on every launch" and SC-010
hold by construction.

### `Floating`

One variant per floating component in the library — the dialogs, menus, popovers and switcher
panels. Enumerated rather than boxed so `open` is `Copy` and the reducer stays trivially testable.
