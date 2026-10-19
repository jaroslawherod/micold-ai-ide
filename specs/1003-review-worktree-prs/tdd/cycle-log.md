# TDD cycle log — 1003 review-worktree-prs

No `tdd/test-list.md`: tasks.md's test-first order is the list (ledger *Handover*).

| Task | Phase | Command | Result |
|---|---|---|---|
| T002 | red | `cargo test -p micold-core --lib worktree_branches` (stubs) | 13 failed, 3 passed (the 3 "is dropped" cases hold on an empty stub) |
| T003 | green | same | 16 passed |
| T004 | red | not run apart: `read_branch_set` (T005) was spliced in with T003; the stub returned `[]`, which fails every T004 assertion | — |
| T005 | green | `cargo test -p micold-core --test worktree_branch_set` | 7 passed |
| T006 | red | compile: `list_in_chunks`, `LIST_BRANCHES_PER_REQUEST`, `head_branch`, `list_branches` missing (test written in part 1 against the API in the ledger) | does not compile |
| T007 | green | `cargo test -p micold-core --test pull_request_list --test pull_request_is_never_stored` | 15 + 3 passed |
| T009 | red | `cargo check -p micold-client --all-targets` | E0026: `ReadPullRequests` has no fields `entry`, `current` |
| T011 | green | `cargo test -p micold-client --test features_pr_changes` (with T012, one build) | passed |
| T010 | red | `cargo test -p micold-client --bin micold-ai-ide pr_list_branch_set`, T012 read swapped back to `[current]` | 2 failed: `left: [{"C"}]`, `right: [{"A", "B", "C"}]`; no-local-git case passed (it pins the old behaviour) |
| T012 | green | `cargo test -p micold-client --bin micold-ai-ide pr_list_` | 16 passed |
| T015 | red | not run apart: tests (fixture `pr_list_branches_checks.txt`, `checks` field) were written before `list_query`/`BranchPullRequest.checks` (T016) in one edit; against the old code they do not compile (`CheckStatus` field missing) | does not compile |
| T016 | green | `cargo test -p micold-core --test pull_request_list --test pull_request_is_never_stored` | 17 + 3 passed |
| T017 | red | not run apart: `check_mark` and `PullRequestRow::checks` did not exist when their tests were written | does not compile |
| T018 | green | `cargo test -p micold-client --test features_pr_changes`; `cargo test -p micold-client --lib -- check_mark pull_request_row` | 28 passed; 2 passed |
