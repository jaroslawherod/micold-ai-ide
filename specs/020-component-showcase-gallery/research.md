# Research: Component Showcase Gallery

**Feature**: [020-component-showcase-gallery](./spec.md) | **Date**: 2026-07-28

Every decision below was taken against the library **as it stands today**, not against the library
feature 018 will produce. That is deliberate: this feature lands first, and its whole value is that
018's visual walkthrough happens against one page rather than by navigating the application.

The spec raised **no** `[NEEDS CLARIFICATION]` markers, and its five clarifications are already
encoded in the requirements. What follows resolves the *technical* unknowns planning surfaced:
where the showcase lives, how it declares itself to a check, how the check reuses 017's definition
of a component, and how three existing gates have to widen so the showcase is bound by the same
rules the application is.

---

## R1 — Where the showcase lives

**Decision**: a second binary in `micold-client`, with its gallery code in the crate's **library**:

- `crates/micold-client/src/showcase/mod.rs` — `pub mod showcase;` from `src/lib.rs`
- `crates/micold-client/src/showcase/{catalogue,state,gallery,samples}.rs`
- `crates/micold-client/src/showcase/main.rs` — the binary, declared explicitly as
  `[[bin]] name = "micold-showcase", path = "src/showcase/main.rs"`

**Rationale**: the spec's own framing ("shipped as a second binary in the `micold-client` crate")
settles the packaging question. Putting the *gallery* in the library rather than in the binary is
what makes the completeness check possible at all: an integration test can call
`micold_client::showcase::catalogue::COMPONENTS`, but it can never see inside a `main.rs`. The
binary is then thin glue — `iced::application(...)` wired to the showcase's own `update`/`view` —
which is exactly the shape Principle I's GUI-wiring exception covers.

The main lives at `src/showcase/main.rs` rather than under `src/bin/` so that cargo's binary
auto-discovery is never in play: an explicit `[[bin]]` path cannot collide with an auto-discovered
target for the same file, and there is no second inferred name (`showcase` vs `micold-showcase`) to
explain later.

**Alternatives considered**:

- *A separate `crates/micold-showcase` crate.* Rejected: it would require the component library to
  be consumable across a crate boundary, which means extracting it — explicitly Out of Scope, and
  it would break the path-based gates 017 relies on (`src/ui/material`, `src/ui/cdk`).
- *`src/bin/micold-showcase.rs` by auto-discovery.* Rejected: the manifest would then not name the
  showcase at all, and the launch command's provenance would be implicit.

## R1a — `default-run` is not optional

**Decision**: add `default-run = "micold-ai-ide"` to `crates/micold-client/[package]` in the same
change that adds the second `[[bin]]`.

**Rationale**: with two binaries and no `default-run`, `cargo run -p micold-client` fails with
"could not determine which binary to run". That command *is* `mise run run` — the documented way to
launch the application (CLAUDE.md). Breaking it would be a behavioural change to the application's
development interface on the day this feature lands, which FR-019 forbids in spirit and which
nobody would thank us for. This is the one line of the change that is easy to forget and impossible
to miss once forgotten.

## R2 — One definition of "a component", shared by both gates (FR-014)

**Decision**: extract the component scanner out of `tests/material_builder_api.rs` into a shared
test-support module, `crates/micold-client/tests/inventory/mod.rs`, and have both the builder-API
gate and the new completeness gate `mod inventory;` it. Neither keeps a copy.

`tests/inventory/` is a *directory*, so cargo does not compile it as its own test binary; it is
included by the test files that need it, the same way `tests/support/mod.rs` already is.

**Rationale**: FR-014 requires that a change to the definition takes effect in both gates *at
once*. Two scanners that happen to agree today is the arrangement the requirement exists to
prevent — the failure mode is silent (the completeness check keeps passing while its idea of the
library drifts from the builder gate's). Sharing the code makes agreement structural.

The definition inherited is `Declared::is_component()`: a `pub struct` declared under
`src/ui/material/` or `src/ui/cdk/` that either has a `From<Self> for …` conversion, or is one of
the documented `TERMINAL_TYPES` (`Surface`). Nothing about it changes here.

**Two properties of the existing scanner the shared module must handle explicitly**, because both
are already true and neither matters until something keys off the result:

1. **Names are not unique.** `material/animation.rs`'s private `mod tags` declares
   `pub struct Fade; Expand; Scale; Scrim;` as widget-tree tags, so the scanner yields two
   `Declared`s named `Fade` — the wrapper and its tag — and both look convertible, because
   convertibility is tested against the whole module's text. Separately, `material/surface.rs` and
   `cdk/overlay.rs` each declare a `Surface`, and they are different components.
2. Therefore the inventory is keyed by **(module, component)**, and duplicates within one module
   are collapsed. A gallery entry names both halves of the key.

**Alternatives considered**: keying by name alone (rejected — the two `Surface`s would silently
satisfy each other's requirement, which is precisely the "passes while wrong" shape FR-016 guards
against); teaching the scanner to skip `mod tags` (rejected — a private tag module is not the
scanner's business, and a rule that reaches inside one would be the first special case).

## R3 — The gallery declares itself as data, and each entry carries its own instance

**Decision**: `showcase::catalogue` exposes `const COMPONENTS: &[Entry]`, `const MOTION:
&[MotionEntry]` and `const EXEMPTIONS: &[Exemption]`. Every `Entry` carries, alongside its names, a
**render function pointer**:

```rust
pub render: for<'a> fn(&'a Showcase, Roles) -> Element<'a, Message>,
```

The gallery's `view` builds each section by iterating the catalogue and calling that pointer. The
completeness check reads the same constants as data.

**Rationale**: this closes the gap between "the gallery lists it" and "the gallery shows it". If
the catalogue were data and the renderer a `match` on component names, an entry with no matching
arm would render nothing and still pass the check — a catalogue that claims coverage it does not
have, which is the failure FR-012 exists to prevent, arriving through the back door. Because the
view *is* the catalogue traversal, an entry cannot exist without an instance and an instance cannot
appear without an entry.

Reading the catalogue as **data through the library's public API**, rather than by scanning the
gallery's source, follows from the same reasoning: a source scan can only ever approximate what the
gallery contains, and this check is the feature's load-bearing claim.

**Alternatives considered**: scanning `src/showcase/*.rs` for component names (rejected: brittle,
and it cannot distinguish a rendered instance from a mention in a comment); a procedural macro
generating both the section and the entry (rejected: no new dependency is expected, and a macro
would hide the one list a developer adding a component needs to read).

## R4 — What "every named variant" means, mechanically (FR-013)

**Decision**: the variant set is every name declared by every `pub enum` in the library, keyed by
name. The check requires each to be named by some entry's `variants`, from any module, and requires
every name an entry lists to still exist. Two-way, like everything else here. Variant identity is
the name; a payload-carrying variant (`Kind::Notification(NoticeLevel)`, `Anchor::Point(Point)`) is
satisfied by one instance with a representative payload of the entry's choosing.

**Rationale**: a component's named variants are, in this library, an enum next to it —
`button::Variant`, `surface::Kind`, `text::TypeRole`, `activity_badge::BadgeEmphasis`,
`overlay::Anchor`. Attribution deliberately does **not** follow the module: `cdk/overlay.rs`
declares `Anchor` and both of its components are exempted as behaviour-layer hosts, so a
module-scoped rule would be unsatisfiable there and would have to be weakened during
implementation — the worst moment to weaken a gate. Keying by name states what the spec asks for
directly ("every variant has an instance") and lets a variant be posed wherever it is actually
visible: `Anchor` in the floating section, because every floating component converts into a
`cdk::Surface` with one.

## R5 — The motion category, and the one thing neither category reaches

**Decision**: the motion category is enumerated as the **`pub fn`s declared in
`src/ui/material/animation.rs`** — today `fade`, `expand`, `scale`, `scrim`. The check fails if that
file is missing or yields no functions (FR-016's shape, applied to the second category).

Components whose appearance *is* an animation (`Fade`, `Expand`, `Scale`, `Scrim`, `ViewFade`,
`HoverReveal`) remain components under R2 and are therefore still required to appear — but their
entry declares `section: Section::Motion`, so their instance is a replayable motion entry rather
than a static row. FR-007a asks for exactly that, and it means a wrapper is never posed as a
picture of itself.

**Known limit, recorded rather than left implicit**: the library also exposes element-producing
free functions that are neither a `pub struct` nor an animation — `material::menu_panel`,
`glyph::icon`, `glyph::icon_colored`. FR-014 widens the check by exactly one category (motion) and
says so; these three stay outside both. They are not invisible in practice — `Glyph` is a component
and the popover panels are rendered by the overlay entries that use `menu_panel` — but no check
holds them, and pretending otherwise would be the kind of vacuous coverage claim this feature is
against. If that gap matters later, it is a third category, added deliberately.

## R6 — Replay and run controls, without a clock (FR-007b, FR-023a)

**Decision**: showcase state holds a per-entry `u64` generation counter and a per-entry `bool`. A
"Replay" press bumps the counter, which reaches the wrapper as `.restart_on(key)` and replays the
transition from zero; the boolean drives `.shown(…)` so an exit can be watched too. No timer, no
subscription, no animation clock.

**Rationale**: the wrappers already own their own progress and already ask for the next frame only
while moving (017's `Progress`). Handing them a changed identity is the whole of "play it again",
and it costs the showcase nothing at rest — which is what keeps FR-023 and SC-009 literally true
rather than approximately true.

**FR-023a's run control has no users today.** No component in the library runs continuously:
`StageProgress`'s fill is a fixed, non-animated `0.4` precisely because it makes no claim about
completion. So the mechanism is built (it is the same trigger as replay, applied to appearance) and
the catalogue's shape carries it, with zero entries using it at delivery. 018's indeterminate
indicator is the first, and it plugs in without the gallery changing shape — which is the point of
building the trigger now rather than when it is needed.

## R7 — Floating surfaces reuse the application's overlay host (FR-007, Edge Cases)

**Decision**: the showcase's `view` ends in one `cdk::overlay::Overlay`, exactly as
`ui::view` does. Each floating component is pushed onto it with its own open flag and its own
`on_dismiss`, and the showcase subscribes to Escape the same way the application does.

**Rationale**: FR-021 forbids a second implementation of anything, and the two Edge Cases about
floating surfaces (the page must stay reachable; two open at once must not deadlock) are properties
the cdk overlay already has and is already tested for (`tests/overlay_stacking.rs`,
`tests/overlay_dismissal_delta.rs`). Building a gallery-local "show one dialog" mechanism would be
both duplication and a weaker guarantee.

## R8 — Scheme switching (FR-008–FR-010)

**Decision**: the showcase's state holds a `ColorScheme`; the control toggles it; `view` resolves
`tokens::roles(scheme)` per render and hands the window `micold_client::ui::theme(scheme)`. The
showcase never reads the OS preference, the settings file, or `dark_light`.

**Rationale**: re-resolving roles per render is what makes FR-009's "every component, no restart,
including sections off screen at the time" fall out rather than being arranged. Not reading the OS
preference is FR-020 (no saved application state) and FR-009 (no host theme change) at once.

## R9 — Laying out a page of unequal components (Edge Cases)

**Decision**: one `material::Scrollable` holding a column of sections. Within a section, posed
instances are laid out as a column of `row`s **chunked at a fixed count**, with an entry able to
declare itself full-width so an oversized component (a banner, the terminal pane) gets its own row.
The page never scrolls horizontally.

**Rationale**: iced has no wrapping row, and measuring to decide would make the layout depend on
the window — which FR-022 rules out. A fixed chunk is deterministic and reflows by scrolling
vertically when the window narrows.

**Alternative rejected**: a horizontal scrollable per section. A clipped instance reads as a
missing one (the spec says so), and a gallery whose instances hide behind a scrollbar is exactly
the "consulted as though complete" failure User Story 4 is about.

## R10 — Sample content and determinism (FR-006, FR-022, SC-010)

**Decision**: all sample content is `const`/`static` in `showcase::samples` — invented labels, a
fixed `TreeItem` list, fixed `ProjectRow`s, and a `GridCache` built by applying one hand-written
`GridFrame` for `TerminalPane`. Nothing is read from disk, the environment, the clock, or a random
source.

A **determinism gate** scans `src/showcase/` for the vocabulary that would break SC-010:
`Instant::now`, `SystemTime`, `Utc::now`, `rand`, `uuid::new_v4`, `std::env::var`,
`current_dir`, `home_dir`, `read_to_string`. Comment-stripped, like every other gate here.

**Rationale**: SC-010 ("two consecutive launches render the same content in the same order") is on
the automated list, and the only honest way to automate it without screenshots is to forbid the
inputs that could vary. The gate is cheap and names the offending line.

## R11 — Packaging exclusion as a gate (FR-018a, SC-008)

**Decision**: a test reads `crates/micold-client/Cargo.toml` and
`packaging/micold-ai-ide.desktop` as text and fails when either names the showcase binary or its
path. It also asserts that both files exist and that the manifest's `[package.metadata.deb] assets`
list is present and non-empty, so a relocation or a deleted assets list fails rather than passing
over nothing.

**Rationale**: cargo-deb ships *only* the listed assets when `assets` is present — which is why the
showcase is excluded by default today. That is a property of a declarative list nobody re-reads,
and the spec is right that it is both the cheapest thing to automate and the least safe thing to
leave to a person. The vacuity guards matter as much as the scan: an `assets` list that has been
emptied or moved would make the check pass while cargo-deb reverted to shipping every binary.

The release workflow is deliberately not scanned: it invokes `cargo deb -p micold-client` and adds
no asset list of its own, so the manifest is the single place the decision is recorded. FR-018a
names two files, and adding a third would spread the rule rather than enforce it.

## R12 — The showcase is bound by the frame-request rule (FR-023)

**Decision**: extend `tests/idle_requests_no_frames.rs` to scan `src/showcase/` in addition to
`src/ui/`, keeping `SANCTIONED = "ui/cdk/motion.rs"` and the "exactly one request across the
scanned layer" assertion.

**Rationale**: today that scan covers `src/ui/` only. A showcase outside it could call
`shell.request_redraw()` and spin at 60fps forever with every test green — the exact failure the
gate's own module doc describes, reintroduced by a new directory the gate does not know about.
FR-023 says the showcase is not exempt; this is what makes that true.

## R13 — The showcase is bound by the boundary rule (FR-021, Principle VIII)

**Decision**: extend `tests/material_boundary.rs` so `src/showcase/*.rs` is scanned as feature-
module source, at the same budgets (0 / 0 / 0) as `src/ui/`'s feature modules.

**Rationale**: FR-021 says the showcase must not become a second implementation of anything, and
the boundary gate is the existing statement of that rule. A gallery is *the* place where the
temptation to "just style this one wrapper so it reads better" is strongest, and where a hand-styled
copy would be most damaging — because a developer would then be comparing the showcase's button to
the application's. The gate makes the showcase's copy provably the same component.

Note this constrains the gallery in a useful way: headings and captions are `material::Text` with a
`TypeRole`, the page's scroll is `material::Scrollable`, and controls are `material::Button` /
`ToggleChip`. Where the gallery finds it *needs* something the library lacks, FR-021's answer is to
add it to the library — and the boundary gate is what forces that conversation instead of allowing
a local workaround.

## R14 — What "cross-platform" means here, and where the gates run

**Decision**: compile parity only, per the spec's Assumption. The existing
`cargo build --workspace` step already covers Linux, macOS and Windows and now builds the second
binary too. In addition, the CI test job gains a step that runs **this feature's gates plus the 017
gates it widens** on all three platforms. The step in `.github/workflows/ci.yml` is the single
authoritative list; prose elsewhere refers to it rather than restating a count that then goes stale:

```
cargo test -p micold-client --test showcase_completeness --test showcase_determinism \
  --test showcase_isolation --test showcase_captions --test showcase_state --test showcase_glue \
  --test packaging_excludes_showcase --test material_boundary --test material_builder_api \
  --test idle_requests_no_frames
```

**Rationale**: the client's full suite runs on Linux only today, because iced's system
dependencies are installed there. These tests open no window — they read source files, one `const`
slice and a reducer — so they run anywhere the crate compiles. The spec's Assumption says this feature's
checks run on all three, and path handling (`\` vs `/` in the scanners' display keys) is exactly the
kind of difference a Linux-only run would miss.

**Alternative rejected**: leaving the gates Linux-only and softening the Assumption. The gates are
the feature's deliverable; a gate that has only ever run on one platform is the weaker claim, and
the cost here is seconds.

## R15 — Documentation (FR-024, Principle VII)

**Decision**: a new `docs/development/component-showcase.md` (what it is for, how to launch it, how
to add a component to it, how the completeness check fails and what each failure means), linked
from `docs/README.md`'s **Development** section, plus a pointer from
`docs/development/component-library.md`'s "Adding a component" list — which is where a developer
adding a component actually looks. `docs/user-guide/` is untouched. CI's `docs` job gains a
`test -f` for the new file, matching how the user-guide docs are verified.

**Rationale**: the audience is developers on this repository, so Principle VII's obligation is met
by developer documentation (the spec's own Assumption). Adding the pointer to the component-library
doc's step list is what stops the gallery being the thing everyone forgets — the same reasoning as
gating it.

## R16 — No new dependency

**Confirmed**: the showcase needs `iced` (already), `micold-core` for `tokens`/`theme` (already),
and `alacritty_terminal`'s types transitively through `GridCache` (already, as a client
dependency). Nothing is added to `Cargo.toml` beyond the `[[bin]]` and `default-run` lines.
