# Implementation Plan: `micold-tokens-ts` — the design tokens as a generated TypeScript theme

**Branch**: `claude/project-thread-rib1g7` | **Date**: 2026-11-16 | **Spec**: [spec.md](spec.md)

## Summary

Add `micold_core::tokens::ts::module()`, a pure function beside `tokens::css::stylesheet()` that renders the token set as one TypeScript module of `as const` objects; a trivial `micold-tokens-ts` binary that writes it to a path argument or stdout; the committed `apps/client/src/theme/tokens.generated.ts`; and a `scripts/tests/tokens-ts-drift.test.sh` the gate runs. The colour-role list, corner radii, motion and state tables `css.rs` already holds are shared, not copied (one table per family, both emitters iterate it).

## Technical Context

**Language/Version**: Rust (workspace toolchain); emitted TypeScript 5 (`as const` literals only)
**Primary Dependencies**: none new; `std::fmt::Write`, `std::fs`
**Storage**: one committed generated file
**Testing**: unit tests in `tokens/ts.rs` (`mise run test-core`); a shell test under `scripts/tests/`
**Target Platform**: generator on Linux/macOS CI; output consumed by Expo web (later milestone)
**Project Type**: library function + CLI binary
**Performance Goals / Constraints**: deterministic bytes (SC-002); no locale-dependent float formatting
**Scale/Scope**: ~46 colour roles x 2 schemes, 15 type roles, 7 radii, 6 elevation levels, 12 durations, 6 easings, 7 state opacities, spacing scale

## Constitution Check

| Principle | Status |
|---|---|
| I. Test-First | `ts.rs` tests and the drift test are written red before the emitter and the file (Test strategy). Pass |
| II. Multi-Session | Pure function; the binary writes via temp file + rename, so concurrent runs never see a partial file. Pass |
| III. Worktree Integration | Not affected: no worktree code touched. N/A |
| IV. Local-First Storage | Build-time generator writing a committed file; no remote service, no runtime storage. Pass |
| V. TS + RN Client, Rust Core | The client's TS theme is generated from the Rust core's tokens, the direction this principle sets. Pass |
| VI. Web Client Reach | Plain `as const` literals, no runtime API: same in every browser and Expo web. Pass |
| VII. Documentation | Module and binary doc comments as in `css.rs`; `docs/development/` note on regenerating the file added. Pass |
| VIII. Reusable UI Foundation | One table per token family in `tables.rs`, iterated by both emitters; no copy. Pass |

## Test strategy

| Requirement | Test | Where |
|---|---|---|
| FR-001 purity, SC-002 determinism | two calls byte-equal; no env/file input (signature) | `tokens/ts.rs` unit (`mise run test-core`) |
| FR-002, SC-001 colours | every `tables::color_roles` entry, both schemes, equals the `stylesheet()` value; `light`/`dark` same keys | `tokens/ts.rs` unit |
| FR-003 families | type scale, shape, motion, state, spacing values per table; elevation: every `level`, `surface` key, `shadow` null-or-`offsetY`/`blur`/`alpha.light`/`alpha.dark` against `elevation::LEVELS` | `tokens/ts.rs` unit |
| FR-004 header | first line exact | `tokens/ts.rs` unit |
| FR-005 binary | `<path>` writes `module()` bytes; no arg prints them; unwritable path exits 1, no file left | `crates/micold-core/tests/tokens_ts_bin.rs` (consolidated test target) |
| FR-006, FR-007, SC-003, SC-004 drift | equal passes; edited fails and prints file + command; missing fails | `scripts/tests/tokens-ts-drift.test.sh`, run by `scripts/gate.sh` with every `scripts/tests/*.test.sh` |
| FR-008 | the unit rows above run in `test-core` | - |

No client test, geometry gate or quickstart Part B visual pass applies: nothing renders, and `apps/client/` has no app yet.

## Project Structure

```text
crates/micold-core/src/tokens/
  tables.rs          # NEW: CORNER_RADII (moved, the only table css.rs has today) plus new extractions of the
                     # inline lists: color_roles(&Roles), DURATIONS, EASINGS, STATES, SPACING
  css.rs             # uses tables.rs; output byte-identical to today (existing tests guard it); its
                     # test-only 46-name list (css.rs:331) is replaced by tables::color_roles
  ts.rs              # NEW: module() + unit tests
  mod.rs             # pub mod tables; pub mod ts;
crates/micold-core/src/bin/micold-tokens-ts.rs   # NEW: arg path -> atomic write, else stdout
apps/client/src/theme/tokens.generated.ts        # NEW, generated, committed
scripts/tests/tokens-ts-drift.test.sh            # NEW: regenerate to a temp file, cmp with committed
```

**Structure Decision**: mirror `micold-tokens-css`; contract in [contracts/tokens-module.md](contracts/tokens-module.md), decisions in [research.md](research.md), shapes in [data-model.md](data-model.md).

## Phases

1. Extract tables into `tables.rs`; `css.rs` tests stay green unchanged (refactor under green).
2. `ts.rs` tests (FR-002 values equal CSS for every role in both schemes; FR-003 families; FR-004 header line; keys are valid identifiers; determinism; `light`/`dark` same keys), then `module()`.
3. Binary + atomic write; failure exits non-zero, no partial file.
4. Generate and commit `tokens.generated.ts`; drift test (passes, fails on edit, fails when missing, prints the FR-005 command).

## Complexity Tracking

None.
