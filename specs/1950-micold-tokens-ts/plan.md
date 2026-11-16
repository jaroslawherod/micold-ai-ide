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
| II. Concurrency | Pure function; binary does one write via temp file + rename. Pass |
| VI. Browser differences | Plain literals, no runtime API. Pass |
| VIII. Reusable components / no duplication | Family tables move from `css.rs` to a shared `tokens/tables.rs` used by both emitters. Pass |
| Test-first | Tests in `ts.rs` written before the emitter (tdd flow). Pass |

## Project Structure

```text
crates/micold-core/src/tokens/
  tables.rs          # NEW: COLOR_ROLES(&Roles), CORNER_RADII, DURATIONS, EASINGS, STATES, SPACING (moved out of css.rs)
  css.rs             # uses tables.rs; output byte-identical to today (existing tests guard it)
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
