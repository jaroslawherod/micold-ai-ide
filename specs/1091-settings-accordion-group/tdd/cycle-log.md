# Cycle log: Settings accordions form a group (1091)

**Honesty note.** Behaviors 1-9, 12-14 of `test-list.md` were written test-after: each test landed in the same commit as its source (`7db0b4f17`, `ca70ba535`, `b91d13a03`), so no genuine red-before-green was recorded. Their "red" below is retroactive and mutant-based only: the audit's deliberate mutants (verification.md, Mutation results) were applied to the finished source and the suite failed, which shows the test can fail, not that it was written first. Behaviors 10, 11, 15, 16 were written after the audit against the surviving mutants; their red is the mutant run, also not a pre-implementation red. No behavior in this feature has a true test-first cycle.

Mutants were applied one at a time with `sed`, the suite run, and the file restored with `git checkout -- <file>` (tree checked clean of source changes afterwards).

| Behavior | Mutant applied | Suite | Failing test | Result |
| --- | --- | --- | --- | --- |
| 10 editing a second daemon | A: `daemons.rs:862` reset removed in `begin_edit` | `daemons_feature` | `editing_a_second_daemon_resets_the_open_section` | red (66 passed, 1 failed) |
| 10 (with B) | A+B: also `daemons.rs:890` reset removed in `discard_draft` | `daemons_feature` | same | red |
| 11 stale press | F: `daemons.rs:1904` `kind.filter(..)` replaced by `kind` | `daemons_feature` | `a_stale_press_for_a_section_outside_the_kind_changes_nothing` | red |
| 15 independent toggle | K: `state.rs` `*open = !*open` to `*open = true` | `showcase_state` | `independent_sections_close_and_reopen_on_their_own` | red (26 passed, 1 failed) |
| 16 grouped pose | L: `surfaces.rs:278` `.in_group(..)` to `.open(true)` | `showcase_accordion_group` | both tests | red (2 failed) |

Restored runs: `daemons_feature` 67, `showcase_state` 27, `showcase_completeness` 42, `showcase_accordion_group` 2, lib `accordion` 21, all passed.

Findings 6-10 are test-quality refactors of existing tests (not new behaviors): self-comparing block removed, eager test split, literal expectations, `held.len() >= 2` (now two truly held sections), named keys `FIRST`/`SECOND`/`LONE`. They were not mutant-checked individually, except that the rewritten held test fails if a held section is dropped from `section_open`'s hold check by construction (not run).

Not done: behaviors 1-9, 12-14 have no mutant re-run in this pass beyond the audit's own table; behavior 17 has no test.
