# TDD cycle log — 937 required-field markers

No `tdd/test-list.md` exists for this feature (as for 813/905): the `tdd.run` hook had no list, so
M1's tasks were driven test-first by hand. Commands ran with `CARGO_INCREMENTAL=0`.

| Cycle | Behaviour (tasks) | Red | Green |
|---|---|---|---|
| 1 | `Requirement`, `RequiredMarks`, constants (T001, T002) | `cargo test -p micold-core --lib required::` on the stub: 7 failed | same: 9/9 passed; 10/10 after `state_in` was added for T012 |
| 2 | `.required(..)` appends `*` to the label of `TextField`, `Select`, `Typeahead` (T003, T004) | `cargo test -p micold-client --lib -- required`: 4 failed | same: 4 passed |
| 3 | `WorktreeForm::required_fields()` mirrors `preview()` (T005, T009) | `--test features_worktree_form -- required` on the `Vec::new()` stub: 4 failed | 4 passed |
| 4 | `ParallelDialog::required_fields()` mirrors `validate` (T006, T010) | `--test features_runs -- required` on the stub: 2 failed | 2 passed |
| 5 | Rename drafts' one required field (T007, T011) | written before the stub's body; its run was filtered out by the `required` filter (test names say `requires`), so red was not observed by a run — the stub's empty list fails its `fields.len() == 1` assertion | `-- requir`: 2 passed |
| 6 | No `.label("…*")` outside `ui/material/` (T013) | guard: the scanner's own fixture test finds the two planted starred labels | 3 passed |
| 7 | Showcase poses the required field (T014) | `the_required_field_is_posed` | passed with `showcase_completeness` |
| 8 | Settings `required_fields(section)`, daemon `Draft::required_fields()`, `State::repository_field` (T016–T020) | tests were written after the code; red observed afterwards by stubbing the lists (both `required_fields` returning `Vec::new()`; the `repository_field` stub did not apply, rustfmt had split its line, so that test stayed green and was not observed red) and running `--no-fail-fast -- required` on `features_daemons` and `features_settings`: 3 of 4 and 7 of 10 failed, each on its exact-list assertion (the 4 that stayed green: the repository test above, and tests that assert an empty list or the mark-forgetting reducer); stub reverted | same tests on the real code: passed |
