# TDD cycle log: 1272 (M1)

The feature has no `tdd/test-list.md`: tasks.md's test tasks are the list, and each row names the
task pair it covers.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | wording constants moved to `review::source_menu`; client constants re-exported | structural, behaviour-preserving | client `features_changes`, `features_pr_changes` green unchanged |
| T002, T005 | `rows()`: one test per FR-005 state (Absent, Loading, Unavailable, Empty, List, stale List), input order, row invariants over every input, unlisted number marks nothing, chips and Default-entry note | bodies stubbed with `todo!()`: `cargo test -p micold-core --lib review::source_menu` 16 FAILED (0 passed) | 16 passed |
| T003, T006 | `stops()` order and enablement, heading/note/stale no stops, stale-independent, `keeps_open` only for `Kind(_)` | same run (stubs) | same run |
| T004, T007 | `label()` for every data-model §6 row (incl. Default entry), `Label::full()`, `tooltip()` with and without the base line | same run (stubs) | same run |
| T008 | client `PrSection` is `pub use …::PrRows as PrSection` | structural, behaviour-preserving | `features_pr_changes` green unmodified (SC-004) |
