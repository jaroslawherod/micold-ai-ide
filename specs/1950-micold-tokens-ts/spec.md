# Feature Specification: `micold-tokens-ts` — the design tokens as a generated TypeScript theme

**Feature Branch**: `claude/project-thread-rib1g7`
**Created**: 2026-11-16
**Status**: Draft
**Input**: Issue #1950 (part of #1722, milestone M4a, covers FR-034): a pure `tokens::ts::module()` beside `tokens::css` and a `micold-tokens-ts` binary writing `apps/client/src/theme/tokens.generated.ts` (colour roles light and dark, type scale, shape, elevation, motion, state layers); unit tests like `tokens/css.rs`; drift check in the gate.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The client reads the desktop app's tokens (Priority: P1)

A client developer builds the React Native (Expo) theme from one generated TypeScript module that carries every design token the desktop app uses, in light and dark, so the web client looks like the desktop app without a hand-copied palette.

**Why this priority**: Without the module there is no source for the client's Material 3 theme (FR-034); every later client milestone depends on it.

**Independent Test**: Generate the module and check, by unit test over the returned text, that it names every colour role in both schemes with the same values the desktop renderer uses, plus the type scale, shape, elevation, motion and state-layer tokens.

**Acceptance Scenarios**:

1. **Given** the token set in `crates/micold-core/src/tokens/`, **When** the module is generated, **Then** it declares every colour role for the light scheme and for the dark scheme, each as the same `#rrggbb` value the CSS sheet emits.
2. **Given** the generated module, **When** a developer reads the type scale, shape, elevation, motion and state-layer tokens, **Then** each carries the value the desktop tokens define.
3. **Given** the same token set, **When** the module is generated twice, **Then** the two outputs are byte-identical.

---

### User Story 2 - The build fails when the generated file drifts (Priority: P1)

A maintainer who changes a token in `micold-core` without regenerating the client file sees the gate fail, naming the file and the command that regenerates it.

**Why this priority**: FR-034 requires CI to fail on drift; otherwise the client theme silently diverges from the desktop app.

**Independent Test**: Change a token value (or hand-edit the committed file) and run the gate: it fails; regenerate and it passes.

**Acceptance Scenarios**:

1. **Given** the committed `apps/client/src/theme/tokens.generated.ts` equals the generator's output, **When** the gate runs, **Then** the drift check passes.
2. **Given** a token changed or the committed file edited by hand, **When** the gate runs, **Then** the drift check fails and prints the regeneration command.
3. **Given** the file is missing, **When** the gate runs, **Then** the drift check fails rather than passing vacuously.

---

### User Story 3 - A generated file nobody edits by mistake (Priority: P2)

Whoever opens the generated file is told on its first line that it is generated, by which binary, and from which directory.

**Why this priority**: The only warning an editor gets before changing a file the build overwrites.

**Independent Test**: Assert the first line of the output names `micold-tokens-ts` and `crates/micold-core/src/tokens/`.

**Acceptance Scenarios**:

1. **Given** the generated module, **When** its first line is read, **Then** it is a header comment saying the file is generated and must not be edited.

### Edge Cases

- A token name that is not a valid TypeScript identifier or is a reserved word: names are emitted as object keys in a fixed casing rule; the unit test asserts every emitted key is a valid identifier.
- Scheme-independent families (type scale, shape, elevation, motion, state opacity) are exported once; only colour roles are split into `light` and `dark`; elevation is exported once, with its per-scheme shadow alpha nested as `{ light, dark }`. A test asserts both colour schemes have the same role keys.
- Empty input: with no path argument the binary prints the module to stdout. An empty token family still emits an empty object (`{} as const`), so every export always exists.
- Floating-point values (radii, opacities, elevations) print identically on every platform: output has no locale, no trailing-zero or exponent variation.
- Concurrency (Principle II): the generator is a pure function with no file, environment or clock input, so concurrent sessions or CI jobs produce the same bytes.
- Browser differences (Principle VI): the module uses only plain `as const` literals and no runtime API, so it behaves the same in every browser and in the Expo web build.
- Failure: the binary writes only to its output path argument or stdout; an unwritable path exits non-zero with a message, never a partial file.
- `tracking` and component anatomy and density are not emitted, as in the CSS sheet.

## Clarifications

### Session 2026-11-16

- Q: `apps/client/` does not exist yet; who creates `apps/client/src/theme/`? → A: This feature does (it depends on nothing in M4a); it commits only `tokens.generated.ts`, and the Expo app (#1735's successor) is built around it later. _(agent-resolved: specs/1722-rn-client-k8s/issues.md#N2, plan.md#M4a)_
- Q: Where does the drift check live? → A: A `scripts/tests/*.test.sh` script, which `scripts/gate.sh` already runs in CI's order; the plan names it. _(agent-resolved: scripts/gate.sh header)_
- Q: Does the TS module follow the CSS sheet in repeating scheme-independent tokens per scheme? → A: No: FR-003 exports them once; repetition in CSS exists only because CSS fails silently on a missing name, which `as const` types do not. _(agent-resolved: crates/micold-core/src/tokens/css.rs#stylesheet doc)_

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `micold_core::tokens::ts::module()` MUST return the complete TypeScript module text as a pure function of the token set: no environment, file or clock input.
- **FR-002**: The module MUST contain every colour role in light and dark as `#rrggbb` strings, equal to the values `tokens::css::stylesheet()` emits for the same roles.
- **FR-003**: The module MUST contain the type scale, corner radii, elevation levels, motion durations and easings, and state-layer opacities, each equal to the corresponding desktop token, each exported once; only colour roles are split by scheme, and elevation nests its per-scheme shadow alpha as `{ light, dark }`.
- **FR-004**: The module MUST open with a header comment naming `micold-tokens-ts` and `crates/micold-core/src/tokens/` and saying not to edit it.
- **FR-005**: The `micold-tokens-ts` binary MUST output exactly what `module()` returns, deciding nothing itself: `micold-tokens-ts <path>` writes the file at `<path>`, and `micold-tokens-ts` with no argument writes to stdout. The client file is regenerated with `cargo run -p micold-core --bin micold-tokens-ts -- apps/client/src/theme/tokens.generated.ts`, the command FR-007 prints.
- **FR-006**: The committed `apps/client/src/theme/tokens.generated.ts` MUST equal the generator's output.
- **FR-007**: The gate MUST fail, naming the file and the regeneration command, when the committed file differs from the generator's output or is missing (FR-034 of #1722).
- **FR-008**: Unit tests in the style of `tokens/css.rs` MUST assert FR-002 to FR-004 and determinism, and run in `mise run test-core`.

### Key Entities

- **Token module**: the generated TypeScript text; one export per token family (colour roles split `light`/`dark`; all other families scheme-independent, exported once: type scale, shape, elevation, motion, state layers).
- **Drift check**: the gate step comparing the committed file with fresh generator output.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of colour roles the CSS sheet emits appear in the TypeScript module for both schemes with identical values.
- **SC-002**: Two generator runs produce byte-identical output.
- **SC-003**: A one-token change in `micold-core` makes the gate fail until the file is regenerated, and a regenerated file makes it pass.
- **SC-004**: Regenerating the file produces an empty `git diff` on an unchanged token set.

## Out of scope

- Mapping the tokens into a React Native Paper `MD3Theme` and the Expo app itself (later M4a issues).
- Fonts (`assets/fonts/`), component anatomy, density and letter-spacing (`tracking`).
- Changes to the CSS emitter or the docs site.

## Assumptions

- `apps/client/` does not exist yet; this feature creates only `apps/client/src/theme/tokens.generated.ts`.
- The module is plain TypeScript (`as const` objects), needing no Node toolchain to generate or to check drift.
- Drift is checked with `scripts/gate.sh` running the binary and diffing, so no Node is needed in the gate.
