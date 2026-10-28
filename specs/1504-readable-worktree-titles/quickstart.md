# Quickstart: validating #1504

Contract IDs (L1–L9) are in [contracts/location-title.md](contracts/location-title.md).

## Part A, automated

1. `mise run test-core` (or `cargo test -p micold-core --all-targets`): `location_title` (L1, L2,
   L4, L3 words), `jira_status` summaries (L6), `GitCli::own_commit_subject` against a temporary
   repository (L7: branch with own commits, fresh branch, detached HEAD, no default branch).
2. `cargo test -p micold-client --test features_overview --test overview --test features_sidebar
   --test features_jira_status --test jira_status_sidebar`: retry state machine (L5), facts stale
   drop (L7), card/row/header agree (FR-007), tooltip lines (L3), Jira status set unchanged (L6).
3. `cargo test -p micold-client --test issues_are_requested_only_on_named_events --test
   jira_is_requested_only_on_named_events`: caller counts unchanged (L8).
4. `mise run gate` (cloud: `scripts/gate.sh`): fmt, clippy, workspace tests, script tests.

## Part B, visual pass (real GUI or showcase; `visual-pass` skill)

Setup: a project on GitHub with `gh` signed in, GitHub issues on; optionally a Jira connection.
Worktrees (create with the app or `git worktree add`):

| Worktree | Make it so | Expected title / source words |
|---|---|---|
| A `feat-813_x` | folder names an issue | issue #813's title / *From GitHub issue #813* |
| B folder `wt-b`, branch `feat/1504-readable-titles` | ticket only in branch | issue #1504's title / *From GitHub issue #1504* |
| C folder `feat-ABC-42_login` | Jira connection for `ABC` | Jira summary / *From Jira ABC-42*; status chip shown |
| D folder `wt-d`, branch `feat/abc-43_x` | Jira key only in branch | summary of ABC-43 / *From Jira ABC-43*; **no** status chip |
| E branch `claude/e`, open PR "Add readable titles" | no ticket | "Add readable titles" / *From pull request #n* |
| F `.specify/feature.json` → `specs/900-x`, spec heading `# Feature Specification: Overview cards` | no ticket, no PR | "Overview cards" / *From the feature spec* |
| G main session titled "Fix the flaky resize test" | no ticket/PR/spec | that title / *From the session title* |
| H one commit "fix(overview): wrap long titles" | none of the above | that subject / *From the latest commit* |
| I fresh branch, nothing committed | none | the branch name; tooltip as today plus nothing new |

For each: the Overview card title and the sidebar row's first line read the same (FR-007); the
row tooltip shows the title, the source words, `Name:`/`Branch:`/`Folder:`; the session-area
header's title tooltip shows title, source words, display name; the expanded card (pointer rest)
shows the source line under the title. A renamed worktree shows its custom name in the sidebar.

Refresh: open a PR for H's branch, wait for the PR reading; H's title becomes the PR title on the
next refresh (FR-010). Archive G's session: G falls to the next source. Create a worktree with
the app, commit "feat: late commit" in it, close and reopen the Overview: card and row show that
subject / *From the latest commit* without reopening the project (FR-006).

Retry (US3): go offline, restart the app, open the Overview (A shows its next source's title,
never "loading"); go online; after 5 minutes, close and reopen the Overview: A shows its
issue title without a restart (SC-003).

Platforms: run the table on Linux, macOS and Windows (a branch with `\`-free non-ASCII name, e.g.
`feat/1504_überblick`, still yields ticket 1504; spec with CRLF and BOM still gives its title).

## Results (close unit, 2026-10-28)

**Part A**: passed. `scripts/gate.sh` (fmt, clippy core and workspace, `cargo test --workspace`,
script tests) exited 0 on the stacked tree (ledger, *Review rounds*, row "Close gate"). Its run
included `issues_are_requested_only_on_named_events` and `jira_is_requested_only_on_named_events`,
with caller counts unchanged (L8, SC-004). The close PR's own diff is docs and specs only; its
script tests and criteria check are in the same ledger row.

**Part B**: passed milestone by milestone, with screenshots in `/mnt/project-files/issue-1504/`:
M1 rows A, B and rename (light and dark), M2 rows C and D (the stub Jira request asks status and
summary, including ABC-43), M3 7 of 7 (prefix `m3-`), M4 rows F and H with Refresh (prefix `m4-`).
M5 needed no visual pass because it changed no UI; its Retry row is covered by the reducer and
mapping tests (`titles_to_fetch_*`, `title_answer_*`, `title_tests`).
