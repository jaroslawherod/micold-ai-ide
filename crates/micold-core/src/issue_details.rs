//! One GitHub issue's relationships (feature 837): what blocks it and what it blocks, its parent
//! and sub-issues, labels, assignees, milestone, project fields and linked pull requests, read with
//! one `gh api graphql` query and held as a model whose invariants hold by type. Three contracts
//! govern it: `specs/837-issue-relationship-metadata/contracts/details-read.md` (the request, the
//! parse and the timeline paging), `contracts/tool-result.md` (the `create_worktree` result's
//! `issue` object) and `contracts/form-details.md` (the New worktree form's lines).
