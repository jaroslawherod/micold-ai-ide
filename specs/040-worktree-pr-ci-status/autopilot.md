# Autopilot ledger — 040-worktree-pr-ci-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #486: https://github.com/jaroslawherod/micold-ai-ide/issues/486 — "Show pull request and CI status for each worktree". The run starts from GitHub issue #486. Follow .claude/skills/speckit-autopilot/references/issue.md: run `scripts/autopilot/issue.sh start 486 feat/worktree-pr-ci-status`, record **Issue** `#486` in the ledger, and end every PR body with `Refs #486` (never `Closes #486`). The issue's body is quoted in spec.md's **Input** line.
- **Kind**: feature
- **Issue**: #486
- **Worktree branch**: feat/worktree-pr-ci-status
- **Started**: 2026-10-02
- **Phase**: 3-design
- **Next step**: Continue the design unit from *Handover*: finish `speckit-plan` (data-model.md, three contracts, quickstart.md), then the plan review.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #529 | Spec | merged | e90a18f9969fe111c9aa6bfb666e31276f3ae38b |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which spec number? | 040. The spec was written as 039; `039-session-attention-notifications` reached `main` before PR #529 was pushed, and worktree `feat-terminal-scrollback-persistence` holds 041. 040 is on no worktree and no remote branch. PR #529 was renumbered before any review of it on GitHub. | agent-resolved | `ls specs` in every `git worktree list` entry and on remote branches, 2026-10-02 |
| D2 | spec | Where does the issue's "parsing lives in the render-free core, tested against recorded `gh` output" criterion go? | Under Assumptions as a constraint for the plan, not as a functional requirement: the spec rubric wants requirements to name observable outcomes. | agent-resolved | spec.md#Assumptions; review-rubrics.md, Spec rubric |
| D3 | spec | What do "modest interval" and "on demand" mean? | 5 minutes, fixed; on demand is the sidebar's existing refresh control. Failures are silent and a status older than two intervals is shown as stale. | agent-resolved | spec.md FR-018, FR-019; 029-refresh-worktrees-list |
| D4 | spec | What does "suggests removing" mean? | A passive mark and tooltip line that lead to the existing delete confirmation; only when the branch has no commits beyond the merged pull request; never automatic. | agent-resolved | spec.md FR-015 to FR-017 |
| D5 | clarify 1 | Does the Settings switch for pull request status start on or off? | Off until the user turns it on. Being signed in to `gh` is a precondition, not consent; the switch that states what is sent is the opt-in. Turning it on reads open projects at once. | agent-resolved | constitution.md#IV ("explicit, informed opt-in"); 034 spec.md#Assumptions (a choice in the application is the opt-in) and FR-003; spec.md FR-030, FR-018, story 4 scenarios 11 and 12 |
| D6 | clarify 1 | How does the user act on the removal suggestion? | With the existing **Delete** action in the row's right-click menu; the suggestion adds no control of its own. | agent-resolved | spec.md#Assumptions ("passive mark"); 008 spec.md#User Story 2; spec.md FR-016 |
| D7 | clarify 1 | How does the user open a worktree's pull request in the browser? (the issue asks for a link in the tooltip, which closes on unhover, 029 FR-010) | An **Open pull request** entry in the row's right-click menu, shown only when the row has an indicator. The tooltip stays as 029 defines it, with no link. Clicking the indicator and a tooltip that stays open are out of scope. | decided by user | Escalation of clarify round 1 (Q1, option A); spec.md FR-013, story 2 scenarios 5, 9 and 10, SC-009, Assumptions |
| D8 | clarify 1 | When the project's GitHub remote is a fork and the pull request lives in the upstream repository, is it shown? | No, out of scope. Only pull requests in the project's own GitHub repository are shown; such a worktree shows no indicator and the upstream repository is never contacted. | decided by user | Escalation of clarify round 1 (Q2, option A); spec.md FR-006, story 1 scenario 14, Edge Cases, Assumptions |
| D9 | design | The spec's "several windows" edge case and SC-007 assume two windows can show one project. Can they? | No: feature 010 lets one window hold a project; a second is refused or takes it over. The edge case and SC-007 were reworded to that; readings need no coordination between windows. | agent-resolved | `crates/micold-daemon/tests/exclusivity.rs` (010 FR-023 to FR-025); research.md R6 |
| D10 | design | Is the rate-limit pause of FR-024 shared between windows of one sign-in? | No, it is held by the window that received the answer; another window pauses when its own next reading is rejected (GitHub does not count a rejected request). FR-024 reworded. Sharing would need a daemon relay for one saved request per window per pause. | agent-resolved | research.md R9; spec.md FR-024, Edge Cases |
| D11 | design | How is "no commits beyond the merged pull request" decided, and what if it cannot be? | Locally, by the daemon: branch tip equal to, or an ancestor of, the pull request's head commit. When that head commit is not in the local repository (pushed from elsewhere, never fetched) no suggestion is shown; the feature does not fetch. Spec Edge Cases and Assumptions say so. | agent-resolved | research.md R11; spec.md FR-015, FR-017, FR-018a |

No `[NEEDS CLARIFICATION]` marker is open after clarify round 1. The round asked questions, so it is
not `CLEAN`.

Clarify round 2 (2026-10-02): coverage scan of all taxonomy categories found no critical ambiguity; no
questions asked, spec.md unchanged. `CLEAN`.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 1c5bb280062f08d77ec432de0fd32e95be915353:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 3 MAJOR (failed reading had two outcomes; what a reading covers vs the manual listing; no outcome above 50 worktrees), 3 MINOR. All six fixed. |
| Spec | 2 | efa54acf195e848c75585bb6eba28c60e3ae2785:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN: 1 MINOR (FR-022 vs FR-024 during a rate-limit pause), fixed. |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Design unit, step 1 (`speckit-plan`) partly done; handed over at the context cap (2026-10-02).

**Done** (committed, not pushed; no PR open): `branch-start.sh 529` (clarify commits rebased onto
`origin/main`); `setup-plan.sh`; `research.md` (R1 to R15, complete); `plan.md` (complete: summary,
technical context, constitution check, requirement map, test strategy, structure, delivery order,
risks); spec.md reworded per D9 to D11 (Edge Cases, FR-024, SC-007, Assumptions).

**Next steps, in order:**

1. Write the artifacts plan.md already links to. Their content is decided in research.md; do not
   redesign:
   - `data-model.md`: §1 `PullRequestStatus` (number, title, url, `PrState`, `CheckStatus` only for
     open/draft, `ReviewState`, `head` oid; no `Serialize`, redacting `Debug`); §2 `ReadingFailure`
     (`Unavailable` / `Passing` / `RateLimited { until }`); §3 the client state
     (`enabled`, `Phase::Idle | Reading { seq, again }`, `statuses: BTreeMap<branch, status>`,
     `removable: BTreeSet<branch>`, `read_at`, `pause_until`, keyed to the shown project); §4 the
     row projection (join by `Worktree.branch`; "Default" and branchless rows get nothing); §5
     `Settings.pr_status_enabled`; §6 `MergedBranchQuery` / `BranchContainment`.
   - `contracts/pull-request-source.md`: §1 trait `PullRequestSource` + `GhCli` impl + fake, in
     `micold_core::pull_request`; §2 the command and query (research R2; arguments are owner, name,
     `b0…`; `--include`); §3 `select_pull_request` (R3); §4 `reduce_checks` table (R4); §5
     `split_response`, `rate_limit_pause` (R9), `reading_failure` (R10); fixtures
     `crates/micold-core/tests/fixtures/gh/pr_*.txt` recorded with `--include`.
   - `contracts/reading-and-wire.md`: §1 start events (R7) and the source gate; §2 the schedule
     reducer and its invariants (R8), what a reading writes (FR-020); §3 `MergedBranchCheck` RPC
     (R11) and the 10 s bound per step; §4 the setting on `Settings`, `DaemonSettings`,
     `SettingsSet` (R12); §5 protocol 20 → 21.
   - `contracts/pull-request-ui.md`: §1 `PullRequestIndicator` builder, glyph and role table, stale
     form, showcase poses (R14); §2 row placement and the "can be removed" chip; §3 tooltip lines
     and their order (R15); §4 menu entry and opening; §5 the Settings control and its text (R12);
     §6 covered layout states.
   - `quickstart.md`: Part A automated (`mise run test-core`, `mise run gate`), Part B visual pass
     (showcase states in both themes; real `gh` against a repository with open/failing, merged and
     no pull request; **Open pull request**; removal suggestion then Delete confirmation; refresh;
     switch off; `gh` missing; no GitHub remote; desktop launch; sandbox placement).
2. Plan review: fresh `autopilot-reviewer`, Plan rubric, round 1 (`review-snapshot.sh` first). Tell
   it spec.md was reworded in this unit (D9 to D11) and to check those edits too.
3. `speckit-tasks`, milestones (`references/milestones.md`; the Settings switch and the wire change
   are foundational, so they go in M1 with US1; split M1 if over about 15 tasks), `speckit-analyze`,
   tasks review, close checklists, PR 2 (`Refs #486`).

**Code anchors found while planning** (so the next unit need not search again):
`crates/micold-core/src/github.rs` (`GhCli` :869, `classify` :804, `choose_remote` :113,
`locate_gh_on_host` :309); `crates/micold-client/src/shell/issues.rs` (the pattern for
`RemoteList` → `spawn_blocking` → `gh`); `shell/capabilities.rs` (`IssueTooling` :116);
`shell/daemon_sync.rs` (`PendingOp::RemoteList` :111, `RefreshFinished` :306/:835/:899, settings
mirror :566); `crates/micold-daemon/src/server.rs` (`RemoteList` arm :1252, `WorktreeRefresh` arm
:1552 — broadcast before ack, `SettingsSet` :977); `features/sidebar.rs::worktree_tooltip` :411;
`ui/sidebar.rs::build_items` :539 (trailing element :616, chips :564); `ui/mod.rs::worktree_menu_items`
:666; `ui/settings/github.rs` (section title "GitHub issues" also at `features/settings.rs` :272);
`ui/settings/environment.rs` :128 (`Checkbox` + `field_note`, the `tool_server_enabled` pattern);
`ui/material/tree_view.rs` (`TreeItem`); `showcase/catalogue.rs` :219 (`ActivityBadge` entry);
`shell/subscriptions.rs` :66; `protocol/version.rs` :78 (`PROTOCOL_VERSION` 20);
`tests/idle_subscriptions.rs`, `tests/refresh_is_only_on_demand.rs`,
`tests/issues_are_requested_only_on_named_events.rs` (gate patterns).

**Evidence**: the GraphQL query of R2 was run on 2026-10-02 with `gh` 2.54.0 against
`jaroslawherod/micold-ai-ide` (HTTP 200, 1.7 s, cost 1; an unknown repository answers `NOT_FOUND`
with headers on stdout and exit 1). The raw output is in the scratchpad as `probe1.txt` and
`probe2.txt`; it may be gone after a reboot and is not needed again before the fixtures task.

**Open findings**: none. No review has run in this unit.

## Open escalation

None.

## Token usage

## Follow-ups not done

