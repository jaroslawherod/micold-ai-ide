# TDD cycle log — 482 worktree changes review (M1)

No `tdd/test-list.md` was derived at design time; as in 575 and 582, the test-first task pairs of
tasks.md are the test list. Each red was run against the stubs of commit `959df6e6` (they compile
and return the "nothing" answer: empty lists, `None`, zero limits, a nil id) and failed on its
assertion, not on a build error.

| Behaviour | Tasks | Red (stub) | Green |
|---|---|---|---|
| `RelPath`, `LineRange`, `limits` | T003 → T004 | `review::tests`: 5 of 5 FAILED (e.g. `a_line_range_refuses_line_zero_and_a_reversed_range`, `the_limits_are_those_of_research_r8` left `0` right `5000`) | `cargo test -p micold-core --lib review` ok |
| Comment JSON shape, round trip, refused reversed range, v4 ids | T005 → T006 | `review::comment::tests`: 4 of 4 FAILED (stub derives: `"state": "Pending"`, `range` nested, nil id) | ok |
| Default branch, toggles, `DiffRange::for_view` | T011 → T016 | `review::base::tests`: 5 of 5 FAILED (stub `None`, toggles off) | ok |
| numstat / raw parsers, assemble, untracked merge, content, large | T012 → T017 | `review::changes::tests`: 8 of 8 FAILED (stub empty vectors, `large` always false) | ok |
| Real-git lists and base | T013 → T018 | `review_git`: 8 of 8 FAILED at their assertion (e.g. `the_base_is_the_merge_base_with_local_main` left `Unavailable(NoDefaultBranch)`; `committed_uncommitted_and_both_lists_follow_the_toggles` left `[]`) | 8 passed |
