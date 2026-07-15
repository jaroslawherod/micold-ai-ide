# Quickstart & Validation: Material Design Icons

**Feature**: `004-material-icons` | **Date**: 2026-07-15

How to validate the feature end-to-end. References the contract in
[`contracts/icon-api.md`](contracts/icon-api.md) and the [`data-model.md`](data-model.md).

## Prerequisites

- Rust stable toolchain (via `mise`).
- The vendored icon font present under `assets/fonts/` with its `LICENSE`/provenance.

## 1. Render-free core stays green (no GUI)

Proves the icon vocabulary is pure and testable without iced (FR-008, SC-006):

```bash
mise run test        # cargo test --no-default-features --all-targets
```

**Expect**: all tests pass, including the new `icons` tests:
- every `Icon` variant maps to its pinned codepoint,
- `Icon::ALL` has no duplicates and covers every variant.

## 2. Font/asset integrity

Proves no runtime "tofu" (SC-005):

```bash
cargo test --features gui icons   # asset-backed tests
```

**Expect**: every `Icon::glyph()` codepoint resolves to a real glyph in the shipped font,
and the pinned `MATERIAL_SYMBOLS` family name matches the file.

## 3. Visual walkthrough — all surfaces (FR-005, SC-001/002/003)

```bash
mise run run         # cargo run --features gui
```

Walk through every surface and confirm the icon **and** unchanged behavior:

| Step | Action | Expect |
|------|--------|--------|
| 1 | Launch with no project | Empty-state button shows the `folder_open` icon; pressing it still opens the selector |
| 2 | App bar | Help action shows its icon; opening Help reveals About with the `info` icon; both still act |
| 3 | Open the selector, browse | "Up" shows the `arrow_upward` icon; git folders show the `commit` git badge; open still works |
| 4 | With known projects listed | Each item shows Open (`folder_open`) and Rename (`edit`) icons; active item shows `check_circle`; unavailable shows `error` and stays un-openable |
| 5 | Same-concept check | "Open" uses the same glyph on the empty state and in the list (SC-002) |

## 4. Theme correctness (US2, FR-007, SC-004)

| Step | Action | Expect |
|------|--------|--------|
| 1 | Set OS to dark, launch | Every icon legible; icon color matches its surface's foreground role |
| 2 | Set OS to light | Same, in light |
| 3 | Toggle OS theme while running | Icon colors switch live with the rest of the UI; none left mismatched or invisible |
| 4 | Inspect a disabled control | Its icon shows the disabled visual state consistently with the control |

## 5. Cross-platform (Principle VI, SC-007)

CI builds and tests on Linux, macOS, and Windows. **Expect**: the icon tests pass on all
three and the embedded-font glyphs render identically (no per-platform substitution).

## 6. Documentation (Principle VII, FR-013)

**Expect**: the user guide is updated in the same change to describe the shared icon
vocabulary and how surfaces use it, and the docs check passes in CI.

## Done when

All of §1–§6 pass: core green without GUI, no tofu, every listed surface shows its icon
with unchanged behavior, both themes correct, cross-platform green, and docs updated.
