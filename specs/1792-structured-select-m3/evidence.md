# Evidence: 1792 quickstart (T022)

## Part A (automated), run by the close unit on the merged tree plus the close PR's tests

`scripts/build-lock.sh cargo test -p micold-client --lib -- <filter>`: `menu_select` 10 passed, `picker_parity` 7, `composer` 11, `expand` 40; `--test main -- motion_tokens ui_glyph_literals one_overlay_implementation` 15 passed (SC-003); `--test material_boundary` 6 passed; `--test layout_snapshot -- the_model_and_permission_selectors` passed. `mise run duplication` was not run (no `mise` in the cloud thread); CI is the gate of record.

## Part B (visual pass), recorded per milestone

- Steps 1 and 4 (trigger states, menus against the showcase; SC-001, FR-009): `visual-pass-M1.md`, `visual-M1.md`. Focus and pressed were NOT CHECKED there; the close PR pins both in `menu_select_anatomy.rs` instead (`keyboard_focus_on_the_trigger_draws_a_ring`, `pressing_the_trigger_paints_a_layer_other_than_hover`).
- Steps 2, 3, 5, 6 and D1 (expand and collapse, reversal, Reduce motion, two sessions; SC-002, FR-007, FR-008, FR-013): `visual-pass-M3.md`. Step 5 passed on a clean release build after the first run hit a mutated binary (see the ledger's follow-ups).
- Step 7 (narrow window, light and dark): `at_a_narrow_width_the_row_ends_at_its_wrapped_height` and the layout gates cover the wrap; no separate visual run was recorded.
