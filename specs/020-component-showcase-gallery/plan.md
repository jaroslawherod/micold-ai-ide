# Implementation Plan: Component Showcase Gallery

**Branch**: `020-component-showcase-gallery` | **Date**: 2026-07-28 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/020-component-showcase-gallery/spec.md`

## Summary

Ship a second, development-only binary in `micold-client` that renders the entire shared component
library on one scrolling page — every component, every posed state, both schemes, plus a motion
section with a replay control per animation — with no daemon, no repository and no saved state, and
hold that page complete with a build-time check rather than with anyone's memory.

The technical shape follows from one decision: **the gallery is a `const` catalogue in the crate's
library, and each entry carries the function that renders its own instances.** That makes the page a
traversal of the catalogue, so the completeness check can read the same data the renderer does and
neither can drift from the other. The check reuses 017's definition of "a component" by *sharing its
code* — the scanner moves into `tests/inventory/mod.rs` and both gates include it — so the two
cannot disagree about what the library contains (FR-014).

Three existing 017 gates widen to cover the new directory, because a showcase the gates cannot see
would be exempt from exactly the rules that make it trustworthy: the boundary gate (so the gallery
composes components instead of styling widgets), the idle-frames gate (so the showcase honours the
single sanctioned frame-request path), and the builder-API gate (which now shares its scanner). Four
new gates land alongside: packaging exclusion, determinism, isolation from the application's state,
and the glue check Principle I's exception requires.

Nothing in the application changes except two lines of its manifest — a `[[bin]]` and the
`default-run` that keeps `mise run run` working.

## Technical Context

**Language/Version**: Rust, edition 2021, MSRV 1.97 (workspace-inherited)

**Primary Dependencies**: `iced` 0.14 and `micold-core` (`tokens`, `theme`) — both already
dependencies of `micold-client`. **No new dependency** ([research R16](./research.md#r16--no-new-dependency)).

**Storage**: none. The showcase reads and writes no file, and creates no state directory (FR-020).

**Testing**: `cargo test` via `mise run test`; integration tests in `crates/micold-client/tests/`,
plus the render-free reducer's own tests. The source-scanning / catalogue-reading gates also run on
macOS and Windows in CI, per the list in `.github/workflows/ci.yml` ([research R14](./research.md#r14--what-cross-platform-means-here-and-where-the-gates-run)).

**Target Platform**: Linux, macOS, Windows desktop. For the showcase specifically: **a build target
only** — it must compile everywhere, and no claim is made about its appearance on any platform (spec,
Assumptions).

**Project Type**: desktop application (Rust + iced workspace), gaining a second, development-only
binary in an existing crate.

**Performance Goals**: zero frames requested and no measurable CPU at rest, with every replay and run
control stopped (SC-009) — the same guarantee the application already carries, not a weaker one.

**Constraints**: the application's appearance and behaviour unchanged (FR-019, SC-007, verified by
the existing suite and the untouched style-parity fixture); no daemon, git repository or saved state
(FR-020); no second implementation of anything the library already provides (FR-021); fixed content
and ordering on every launch (FR-022); never installed (FR-018/FR-018a).

**Scale/Scope**: **38** distinct components across `src/ui/material/` and `src/ui/cdk/` — counted by
module + name, after collapsing `animation.rs`'s private widget-tree tags and separating the two
`Surface`s, and excluding the three records the builder gate already partitions out (`MenuItem`,
`ProjectRow`, `TreeItem`). Five variant enums (`button::Variant`, `text::TypeRole`,
`activity_badge::BadgeEmphasis`, `surface::Kind`, `overlay::Anchor`), four animation helpers, two or
three exemptions, one scrolling page. Roughly: eleven new source files, eleven test files (seven new,
three widened, one shared scanner extracted), two manifest lines, one `mise` task, one developer
document — **54 tasks** ([tasks.md](./tasks.md)).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: Every gate and the showcase reducer are written Red-first —
  each gate is observed failing before the code that satisfies it exists, by a vacuity or synthetic
  Red stated per task — and the reducer's `update` is driven directly. The `view` glue
  (`src/showcase/gallery.rs`, `src/showcase/main.rs`) is thin GUI wiring with no decision logic of
  its own, which Principle I's named exception covers via the recorded
  [quickstart.md](./quickstart.md) procedure. **Every state transition lives in the tested reducer**,
  not in the view — the same split `app.rs` versus `ui/` already has.
  Two things make that claim hold rather than merely be asserted, both added after this feature's
  `/speckit-analyze` pass. First, the exception now covers a development-only binary's own render
  glue — constitution **1.5.0**, a **MINOR** amendment: it previously named only `src/main.rs` and
  `src/ui/`, so `src/showcase/` fell outside it, and widening the covered set of a NON-NEGOTIABLE
  principle is a material expansion rather than a wording fix (the same reasoning that made 1.3.0's
  Default-session exception MINOR). Second, `tests/showcase_glue.rs` (T054) asserts the two glue
  files hold no branch on showcase state, so the exception's precondition is checked on every build
  rather than trusted — which is what an amendment widening an exemption ought to arrive with.
- [x] **II. Multi-Session Support**: Not applicable, and provably so — the showcase creates no
  session, holds no session state, and FR-020 forbids it from touching the ones that exist. No new
  state is introduced that could leak between sessions.
- [x] **III. Worktree Integration**: Not applicable. FR-020 forbids the showcase from creating,
  reading or modifying a git worktree, and its import list is the statement of that
  ([contracts/showcase-launch.md §2](./contracts/showcase-launch.md)).
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: Stronger than required — the showcase stores
  nothing at all and is fully functional offline. Nothing leaves the device.
- [x] **V. Rust + iced Stack**: Rust and iced only. Invalid states are made unrepresentable where it
  matters: `Showcase::open` is an `Option<Floating>`, so two floating surfaces cannot be open at once
  and the deadlock the spec's Edge Cases name is not a state the type admits.
- [x] **VI. Cross-Platform Parity**: The existing `cargo build --workspace` step covers all three
  platforms and now builds the second binary; the feature's own gates additionally *run* on all
  three. Scoped to compilation by the spec's Assumptions, with the reasoning recorded rather than
  assumed — see Complexity Tracking.
- [x] **VII. Documentation First-Class**: A new `docs/development/component-showcase.md` ships in the
  same change, linked from `docs/README.md` and from the "Adding a component" steps in
  `docs/development/component-library.md`, and verified by CI's `docs` job. The user guide is
  deliberately **not** extended (FR-024): the showcase is not a user-facing capability, and the
  spec's Assumptions record that Principle VII's obligation is met by developer documentation for
  this audience.
- [x] **VIII. Reusable UI Component Foundation**: The showcase adds no component and forks nothing —
  it composes the library exactly as a feature module does, which the widened boundary gate now
  enforces at the same zero budgets. Where the gallery reveals something missing from the library,
  FR-021's answer is to add it to the library, and the gate is what forces that conversation instead
  of permitting a gallery-local workaround.

**Re-checked after Phase 1 design**: all eight still hold. The design added no state to persist, no
platform branch, no dependency, and no component; the one thing it added to the application is two
manifest lines, one of which (`default-run`) exists specifically to keep the application's documented
launch command working.

## Project Structure

### Documentation (this feature)

```text
specs/020-component-showcase-gallery/
├── plan.md                            # This file
├── research.md                        # Phase 0 — 16 decisions
├── data-model.md                      # Phase 1 — the catalogue and the reducer's state
├── quickstart.md                      # Phase 1 — §A automated, §B the recorded walkthrough
├── contracts/
│   ├── gallery-catalogue.md           # the gallery's own surface
│   ├── completeness-check.md          # the nine rules and four vacuity guards
│   └── showcase-launch.md             # the command, and everything it must not touch
├── checklists/requirements.md         # 16/16, from /speckit-specify
└── tasks.md                           # Phase 2 — 54 tasks, from /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-client/
├── Cargo.toml                         # + [[bin]] micold-showcase, + default-run  (R1, R1a)
├── src/
│   ├── lib.rs                         # + pub mod showcase;
│   ├── showcase/
│   │   ├── mod.rs                     # the module's own docs + re-exports
│   │   ├── catalogue.rs               # COMPONENTS / MOTION / EXEMPTIONS — one list  (const)
│   │   ├── state.rs                   # render-free: Showcase, Message, update  (tested directly)
│   │   ├── samples.rs                 # fixed invented content, incl. the fabricated GridCache
│   │   ├── gallery.rs                 # view: iterates the catalogue; one cdk overlay host
│   │   ├── sections/                  # the entries' render fns, grouped so they can be split
│   │   │   ├── atoms.rs               #   Text, Ellipsized, Glyph, Divider, Tag, ActivityBadge
│   │   │   ├── controls.rs            #   Button, IconButton, Checkbox, TextField, Select, …
│   │   │   ├── surfaces.rs            #   Surface, Scrollable, Accordion, Toolbar, TreeView, …
│   │   │   ├── floating.rs            #   Modal, menus, switcher, Tooltip
│   │   │   ├── terminal.rs            #   TerminalPane, from the fabricated grid
│   │   │   └── motion.rs              #   the four helpers + the six animating components
│   │   └── main.rs                    # the binary — iced::application glue only
│   └── ui/                            # UNCHANGED
└── tests/
    ├── inventory/mod.rs               # EXTRACTED from material_builder_api.rs  (FR-014)
    ├── material_builder_api.rs        # widened: now `mod inventory;`
    ├── material_boundary.rs           # widened: scans src/showcase/ too  (FR-021)
    ├── idle_requests_no_frames.rs     # widened: scans src/showcase/ too  (FR-023)
    ├── showcase_state.rs              # NEW — the reducer, driven directly  (Principle I)
    ├── showcase_glue.rs               # NEW — the view holds no decision logic  (Principle I)
    ├── showcase_completeness.rs       # NEW — the nine rules + four guards + §5 demonstrations
    ├── showcase_determinism.rs        # NEW — no clock, randomness, env or filesystem  (SC-010)
    ├── showcase_isolation.rs          # NEW — names no store/settings/daemon/git  (FR-017, FR-020)
    ├── showcase_captions.rs           # NEW — every interactive entry names its live states (FR-005)
    └── packaging_excludes_showcase.rs # NEW — the manifest and the desktop entry  (FR-018a)

docs/
├── README.md                          # + Development index entry
└── development/
    ├── component-library.md           # + a pointer from "Adding a component"
    └── component-showcase.md          # NEW  (FR-024)

mise.toml                              # + [tasks.showcase]
.github/workflows/ci.yml               # + the gate list on all three platforms; + docs test -f
```

**Structure Decision**: the gallery lives in `micold-client`'s **library** at `src/showcase/`, with
the binary declared as `[[bin]] name = "micold-showcase", path = "src/showcase/main.rs"`. The library
placement is what lets an integration test read the catalogue; a `main.rs` is invisible to `tests/`.
`src/ui/` is untouched — putting the showcase under it would either make it a library layer (exempt
from the boundary rules, the wrong direction) or break `material_boundary`'s assertion that every
directory under `ui/` is a known library layer. A separate crate was rejected: it would require
extracting the component library, which is Out of Scope and would break the path-based gates 017
relies on. See [research R1](./research.md#r1--where-the-showcase-lives).

## Phase 0 — Research

Complete: [research.md](./research.md). Sixteen decisions, each with its rationale and the
alternatives rejected. The ones that shape everything else:

- **R1/R1a** — a second binary in `micold-client`, gallery in the library; `default-run` is not
  optional or `mise run run` breaks the day this lands.
- **R2** — one component definition, shared by extracting 017's scanner (FR-014). Two wrinkles it
  must handle: component names are not unique (two `Surface`s; `Fade` appears as both a wrapper and a
  private widget-tree tag), so the inventory is keyed by module *and* name.
- **R3** — the catalogue is data, and every entry carries its own `render` function, so an entry
  cannot exist without an instance nor an instance without an entry.
- **R5** — the motion category is enumerated from `material/animation.rs`'s `pub fn`s; the three
  element-producing free functions that fall outside both categories are recorded as a known limit
  rather than quietly uncovered.
- **R6** — replay is a generation counter feeding `.restart_on(key)`. No clock anywhere. FR-023a's
  run control ships with zero users because nothing in the library runs continuously yet.
- **R11–R14** — the new gates, the widened ones, and where they all run.

No `NEEDS CLARIFICATION` remains. The spec raised none, and the five items its clarification session
resolved are already encoded in FR-003a, FR-007a–c, FR-013a, FR-018a and FR-023a.

## Phase 1 — Design & Contracts

Complete. [data-model.md](./data-model.md) covers the catalogue's three `const` slices, the entry /
motion-entry / exemption shapes and their invariants, the sample-content rule, and the render-free
reducer's state and messages. Three contracts:

- **[gallery-catalogue.md](./contracts/gallery-catalogue.md)** — the gallery's own surface: what an
  entry declares, what its `render` may and may not do, how triggers work, and the five-step
  procedure for adding a component.
- **[completeness-check.md](./contracts/completeness-check.md)** — nine rules (C1–C9), four vacuity
  guards (V1–V4), what the check deliberately does not do, and how SC-004's two failure directions
  are demonstrated on every run rather than once by hand.
- **[showcase-launch.md](./contracts/showcase-launch.md)** — the command, the five things launching it
  must not do, the two functions it must share with the application, and the packaging exclusion.

[quickstart.md](./quickstart.md) is the validation guide: §A maps each automated success criterion to
the test that answers it; §B is the recorded walkthrough for the only three that need a person —
SC-001's timing, SC-005's hover/press pass, SC-006's scheme comparison — with the tables to fill in.

## Implementation sequencing (for `/speckit-tasks`)

Not tasks, but the order the user stories imply and the dependencies between them:

1. **Foundation** — the `[[bin]]`, `default-run`, `pub mod showcase`, the `mise` task, and an empty
   catalogue that compiles and opens a window. Nothing else can be tested until the binary exists.
2. **US1 (P1)** — sections rendering from the catalogue, the samples module, the widened boundary and
   idle-frames gates. This is the premise; everything else refines it.
3. **US2 (P2)** — posed variants and states per entry, captions naming what is live.
4. **US3 (P3)** — the scheme control.
5. **US4 (P4)** — the shared inventory extraction and the completeness check. Deliberately last: the
   check needs a gallery to hold complete. Its **rule functions take their two sets as arguments**, so
   SC-004's demonstrations are unit tests over synthetic inventories rather than a manual break-and-fix.
6. **Cross-cutting** — the packaging gate, the determinism gate, CI's three-platform step, the
   developer document, and the SC-007 confirmation that the application's suite and style fixture are
   untouched.

Two of the new gates (packaging, determinism) have no dependency on the gallery's content and can land
at any point after step 1; the packaging one is worth landing early, because it is the requirement
with the worst failure mode in the feature.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Principle VI scoped to *compilation* for the showcase, with no per-platform appearance claim | The showcase is a development tool no user installs; parity of the components it displays is owned by the features that introduce them. The spec's Assumptions record this as the deliberate scope. | Requiring a recorded launch on three machines protects nothing — there is no user-facing surface to protect — and CI cannot launch a GUI headlessly, so the "verification" would be an unverifiable ritual. Mitigated by running the feature's gates on all three platforms rather than on Linux alone. |
| Three existing 017 gates are modified rather than left alone | FR-014 requires one shared definition of a component; FR-021 and FR-023 explicitly bind the showcase to the boundary and frame-request rules, which the current gates cannot see because they scan `src/ui/` only. | Writing parallel gates for `src/showcase/` would produce exactly the two-scanners-that-happen-to-agree arrangement FR-014 exists to prevent, and a second copy of the boundary rule would drift from the first at the first budget question. |
| A second binary in an existing crate, rather than a new crate | The spec's own framing, and the library must stay at `src/ui/material` for 017's path-based gates. | A separate crate needs the component library extracted into a package — explicitly Out of Scope, and it would break every path-based gate in one commit. |
| `default-run` added to the application's manifest | Without it, two binaries make `cargo run -p micold-client` ambiguous and `mise run run` fails. | There is no alternative; the only choice is whether it lands in this change or breaks the workflow. Recorded here because it is the one edit to the application FR-019 could otherwise be read to forbid, and it exists solely to *preserve* existing behaviour. |
