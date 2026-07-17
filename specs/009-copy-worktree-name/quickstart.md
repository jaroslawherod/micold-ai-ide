# Quickstart / Validation Guide: Copy Worktree Name to Clipboard

Proves the feature end-to-end. See [contracts/clipboard-copy.md](./contracts/clipboard-copy.md)
and [data-model.md](./data-model.md) for the interfaces referenced here.

## 1. Font-integrity regression (FR-005, SC-002) — headless, no GUI needed

```bash
cargo test --no-default-features --test icons --test icons_font
```

Expected: both suites pass —
- `tests/icons.rs`: `Icon::Copy` maps to its pinned codepoint (`U+E14D`) and `Icon::ALL` has no
  duplicate variant or glyph (now 19 icons).
- `tests/icons_font.rs`: every `Icon` codepoint, including `Icon::Copy`, resolves to a real glyph
  in the shipped, subsetted font — no "tofu" can reach the running UI.

## 2. Full logic-core + gui-parity suites (CI parity)

```bash
cargo test --no-default-features --all-targets
cargo test --features gui
cargo clippy --features gui --all-targets
```

Expected: all green, no new clippy warnings in `src/icons.rs`, `src/app.rs`, `src/main.rs`, or
`src/ui/mod.rs`.

## 3. Manual check (US1: FR-001..005, SC-001..003)

The context-menu wiring itself is GUI-only presentation glue with no headless unit surface (see
plan.md's Constitution Check) — the following was validated by code review and by the app
building/linting clean, per this workspace's general preference for headless verification over
launching the full GUI. If you want to confirm the end-to-end UX yourself:

```bash
cargo run --features gui
```

| Action | Expected |
|--------|----------|
| Right-click any worktree row | Context menu shows **Copy name**, **Rename**, **Delete**, in that order, each with its own icon. |
| Choose **Copy name** | The menu closes; pasting elsewhere (another app, or back into a rename dialog's text field) shows exactly the row's displayed name. |
| Rename a worktree, then right-click it and choose **Copy name** | The pasted text is the custom name, not the derived one (FR-002 / Acceptance Scenario 2). |
| Right-click a different worktree and choose **Copy name** | The clipboard now holds the second worktree's name, overwriting the first (Edge Cases). |

## 4. Confirm the font subset process is reproducible (contributor check)

`assets/fonts/PROVENANCE.md`'s subset command includes `e14d` in `--unicodes`; re-running it from
the upstream variable font (`fonttools varLib.instancer` → `pyftsubset`) reproduces a font that
passes step 1 above bit-for-bit in coverage (glyph count 19, no missing codepoints).
