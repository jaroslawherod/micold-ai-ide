# Contract: Design Tokens — Tag Colors & Sidebar Typography

**Module**: `src/tokens.rs` (pure). Test: `tests/tokens.rs`. GUI mapping: `src/ui/style.rs`.

## Tag color role pairs

`Roles` gains a `(fill, on_fill)` `Rgb` pair per conventional type plus one for the issue tag,
defined in both `LIGHT` and `DARK` consts:

```rust
// on Roles:
pub tag_feat: Rgb,      pub on_tag_feat: Rgb,
pub tag_fix: Rgb,       pub on_tag_fix: Rgb,
pub tag_chore: Rgb,     pub on_tag_chore: Rgb,
pub tag_docs: Rgb,      pub on_tag_docs: Rgb,
pub tag_refactor: Rgb,  pub on_tag_refactor: Rgb,
pub tag_test: Rgb,      pub on_tag_test: Rgb,
pub tag_build: Rgb,     pub on_tag_build: Rgb,
pub tag_ci: Rgb,        pub on_tag_ci: Rgb,
pub tag_perf: Rgb,      pub on_tag_perf: Rgb,
pub tag_style: Rgb,     pub on_tag_style: Rgb,
pub tag_issue: Rgb,     pub on_tag_issue: Rgb,
```

- A helper maps `ConventionalType → (fill, on_fill)` so the GUI and tests share one lookup.
- The **status** tag (missing/invalid) reuses the existing `error` / `on_error` pair — no new
  role.
- Colors follow the Material palette style already used in `LIGHT`/`DARK`; each type visually
  distinct (FR-005).

## Contrast enforcement (the key test hook)

`tests/tokens.rs` `pairs()` returns a FIXED-SIZE array checked for AA (≥ 4.5) in both schemes.
Extend it: change the return type length and add one entry per new pair, e.g.
`("tag_feat", roles.on_tag_feat, roles.tag_feat)`, for all 11 tag pairs. The existing
`light_scheme_meets_aa_contrast` / `dark_scheme_meets_aa_contrast` loops then enforce AA for
every tag automatically (FR-006, SC-007). **A tag color is only guaranteed AA if it is added to
this array** — every new pair MUST be listed.

## Sidebar typography (80%) — FR-012

Add sidebar-scoped size constants (consumed only by the sidebar / `tree_view`), 80% of the
current sizes:

```rust
pub mod sidebar {
    pub const NAME: u16 = 11;  // 80% of type_scale::BODY (14) ≈ 11.2 → 11
    pub const TAG: u16  = 10;  // 80% of type_scale::LABEL (12) = 9.6 → 10
    pub const SESSION: u16 = 11; // session labels
}
```

- App-wide `type_scale` constants are NOT modified (reduction is sidebar-only).
- A unit test asserts these equal `round(0.8 * base)` so the "80%" intent is auditable.

## Sidebar spacing (minimal padding) — FR-009

- Outer sidebar content column horizontal padding: `spacing::XS` (4px) instead of `spacing::MD`
  (16px).
- `tree_view` per-depth indent step reduced (target ~`spacing::XS`), keeping child sessions
  visually nested but tight.
- Vertical spacing unchanged. Exact values validated in `quickstart.md`.

## Invariants

- `tokens.rs` stays pure (no iced dependency); runs under `--no-default-features`.
- Every tag `(on_fill, fill)` pair present in `pairs()`; both `LIGHT` and `DARK` populated.
