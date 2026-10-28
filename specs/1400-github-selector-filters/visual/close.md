# Quickstart §B close pass (T061)

Every step ran headless with the `visual-pass` recipe: Xvfb with lavapipe (software Vulkan), the
client and daemon pinned from the milestone's worktree build, the dev-only `fake_gh`
(`crates/micold-client/examples/fake_gh.rs`) first on `PATH` as `gh`, a private data dir and a
scratch git repo whose remote is `https://github.com/acme/app`. B1–B16 ran with their milestones
(M2, M3, M4, M5, M7); B11's reopen half, which M6 added and no milestone pass ran, and B5's restart half ran at
close (2026-10-28, light theme, binary built at the M7 tip `c573fbbdc`). Screenshots are in the project files under
`issue-1400/`; each milestone's verdict and findings are in the ledger's *Review rounds*.

| # | Verdict | When | Evidence / note |
|---|---|---|---|
| B1 | PASS | M2 | `m2-b1-form-light.png`, `m2-b1-form-dark.png`, `m2-b1-list-light.png`: today's list, no active line, 034's statement. The Basic/Advanced switch came in M5: `m5-1-basic-light.png`, `m5-1-basic-dark.png` |
| B2 | PASS | M2 | `m2-b2-panel-chips-{light,dark}.png`, `m2-b2-filtered-list-{light,dark}.png`, `m2-b2-active-line-{light,dark}.png`; round 2 `m2r2-b2-chip-*`, `m2r2-b2-active-line-*`. Fake logged `-label:"in-progress" -label:"blocked"` |
| B3 | PASS | M2 | `m2-b3-search-local-light.png`: the filtered list narrows with no request (complete list) |
| B4 | PASS | M2 round 2 | `m2-b4-panel-include-exclude-light.png`, `m2r2-b4-moved-{light,dark}.png`: `blocked` moves from Exclude to Include. Round 1 failed this row (`blocked` not offered in Include; fixed, see *Review rounds*) |
| B5 | PASS | M3, close | `m3-b5-light.png`, `m3-b5-dark.png`: same filters and active line after closing and reopening the form, one listing. Restart half re-run at close: after the client was stopped and relaunched, the form reopened in the remembered mode with one listing and Basic still excluded `in-progress` (`close-b5-b11-after-restart-advanced-light.png`, `close-b5-after-restart-basic-light.png`; `close-fake-gh.log`) |
| B6 | PASS | M4 | `m4-2-closed-chosen-{light,dark}.png`, `m4-3-closed-rows-{light,dark}.png`, `m4-3-all-rows-light.png`; picking closed issues `m4-5-b6-pick-closed7-light.png`, `m4-5-b6-pick-closed14-light.png` (ticket, name and type filled) |
| B7 | PASS | M2, M3 | `m2-clear-filters-light.png`, `m2r2-clear-filters-{light,dark}.png`; reopen on the default `m3-b7-light.png`, `m3-b7-dark.png` |
| B8 | PASS | M5 | `m5-3-b8-light.png`, `m5-3-b8-dark.png`: Query and Run replace Filters and the active line; fake logged `repo:acme/app is:issue (-label:in-progress) sort:created-asc` |
| B9 | PASS | M5 | `m5-4-b9-light.png`, `m5-4-b9-dark.png`, `m5-4-b9-three-light.png`: each refusal's text; fake logged nothing |
| B10 | PASS | M5 | `m5-5-b10-light.png`, `m5-5-b10-dark.png`: both of GitHub's messages in order, the query kept; fixed and run, the list returned (`FAKE_GH_MODE=reject`) |
| B11 | PASS | M5, close | Switch half (M5): `m5-6-draft-kept-light.png` (the draft kept on the switch to Basic). Reopen half (close), with `in-progress` excluded in Basic: ran `label:bug` (one request), typed the draft `label:bug label:ui` without running (no request), switched to Basic: the basic list again (`close-b11-1-run-then-draft-light.png`, `close-b11-2-back-to-basic-light.png`). Reopened: Basic, the filter applied, one listing (`close-b11-4-reopened-basic-light.png`); the file held `"query":"label:bug","ran":true` and no draft (`close-b11-selector.json`). Switching to Advanced showed `label:bug`, not the draft, and sent nothing (`close-b11-6-switch-shows-last-run-light.png`); a reopen while in Advanced ran `label:bug` once (`close-b11-5-reopened-advanced-light.png`) |
| B12 | PARTIAL | M2 | `m2-b12-back-from-new-light.png`: source switched to New branch and back keeps the filter, clears the search text and reloads. Not run with Jira: the pass had no Jira site, so the Jira source was not offered. The Jira-and-back case is covered by `github_filter_state.rs` (source to Jira and back, both modes) |
| B13 | PASS | M2, M4, M7 | `m2-b13-down-enter-adds-dark.png`, `m2-b13-enter-does-not-add-dark.png`; state chips `m4-4-keyboard-*.png`; `m7-keyboard-filters-button-ring-light.png`, `m7-keyboard-state-chip-ring-light.png`, `m7-keyboard-closed-chip-ring-dark.png`, `m7-keyboard-author-focus-light.png`, `m7-keyboard-milestone-focus-dark.png`. Accepted: the clear (x) inside Assignee/Author/Milestone is not a Tab stop (ledger M7-D2) |
| B14 | PASS | M2 | `m2-b14-github-off-dark.png`: no filter control, fake logged no request |
| B15 | PASS | M3 | `m3-b15-dark.png`: after Forget, `<data_dir>/github/<id>.selector.json` gone; `m3-iso-dark.png`: one file per project |
| B16 | PASS | M7 | `m7-assignee-choices-dark.png`, `m7-assignee-me-list-light.png`, `m7-assignee-typed-person-light.png`, `m7-author-use-login-{light,dark}.png`, `m7-author-invalid-login-{light,dark}.png`, `m7-milestone-choices-{light,dark}.png`, `m7-milestone-v24-list-light.png`, `m7-no-milestone-list-light.png`, `m7-panel-all-six-set-light.png`; reopen `m7-remembered-reopen-{light,dark}.png`. Fake logged each term |
| B17 | NOT RUN | — | Needs a signed-in `gh` and a real repository to compare with github.com's search (SC-002); the cloud container has no signed-in `gh`. Same gap as T018's `ISSUE_ADVANCED` probe (ledger *Follow-ups*) |
| B18 | PASS (Linux) | all | Light and dark in every milestone pass above, and narrow windows: `m2-layout-800-dark.png`, `m4-1-panel-narrow800-dark.png`, `m5-1-advanced-800-light.png`, `m5-7-narrow-{light,dark}.png`, `m7-panel-{light,dark}-*.png`. macOS and Windows: no such host here, not run (CI runs the gates on all three) |

Found by the passes and not this feature's defects, kept as follow-ups in the ledger: the dialog is
clipped at a 700 px window (M7-D1, `m7-form-dark-700-scrolled-down.png`); the Basic/Advanced chips
move as the dialog re-centres (M5-D2); Enter in the form re-clicks the sidebar button behind the
scrim (predates 1400).
