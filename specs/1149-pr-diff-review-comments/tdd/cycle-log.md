# Cycle log: PR diff review comments (#1149)

No per-behavior red runs were recorded while the milestones were built (M1 to M3 were delivered through
milestone PRs with reviews A and B, not the red-green loop). `verification.md` graded 13 of the 14
behaviors `TEST_AFTER`; the full suite passed (8770) and 6 hand-made mutants on the changed files were
all caught.

This log and tdd/test-list.md were written at close, after verification.md (which says they were missing). Accepted, per behavior (T001, T002, T003, T004, T006, T008, T012, T014, T017, T019, T020, T023, T025):
written together with or after the source in the milestone commit; red evidence not recoverable; the
mutants above show the tests bind the behavior. T015 is a declared characterization test.
