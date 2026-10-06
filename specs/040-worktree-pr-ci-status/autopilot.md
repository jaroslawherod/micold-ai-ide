# Autopilot ledger — 040-worktree-pr-ci-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #486: https://github.com/jaroslawherod/micold-ai-ide/issues/486 — "Show pull request and CI status for each worktree". The run starts from GitHub issue #486. Follow .claude/skills/speckit-autopilot/references/issue.md: run `scripts/autopilot/issue.sh start 486 feat/worktree-pr-ci-status`, record **Issue** `#486` in the ledger, and end every PR body with `Refs #486` (never `Closes #486`). The issue's body is quoted in spec.md's **Input** line.
- **Kind**: feature
- **Issue**: #486
- **Worktree branch**: feat/worktree-pr-ci-status
- **Started**: 2026-10-02
- **Phase**: 4-milestone
- **Next step**: M4 in progress (T030–T040).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #529 | Spec | merged | e90a18f9969fe111c9aa6bfb666e31276f3ae38b |
| #536 | Design | merged | 47f73eb184695cfcd1fb5cdc6129ee4c36dd8f4a |
| #541 | M1 | merged | e496e95c63b9a776ac22921f78ee100ea6b505b1 |
| #547 | M2 | merged | 2e688bf3d01bed2c3ae63681ecd05be8e6db3648 |
| #611 | M3 | merged | ef56a970339f2a77a0f8642a6c4dcc848a00b1ce |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | `micold-core` reads pull requests through `gh` and turns recorded answers into per-branch statuses and failure kinds (US1 core; no UI) | #541 | merged |
| M2 | T014–T022 | full | Protocol 22 (039 took 21): the daemon stores and broadcasts `pr_status_enabled` and answers `MergedBranchCheck` (no UI) | #547 | merged |
| M3 | T023–T029 | full | The holding window reads pull request status on the listing after `Attached` and on switch-on, and holds it in memory (no UI) | #611 | merged |
| M4 | T030–T040 | full | MVP: the Settings switch, and the indicator on every worktree row with a pull request | | pending |
| M5 | T041–T047 | full | Pull request lines in the tooltip; **Open pull request** in the row menu | | pending |
| M6 | T048–T054 | full | "can be removed" chip and `Cleanup:` line for a merged pull request with nothing newer | | pending |
| M7 | T055–T065 | full | 5-minute interval, refresh trigger, one further reading, rate-limit pause, stale form | | pending |
| M8 | T066–T068 | full | Architecture page and the recorded quickstart §B pass | | pending |

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
| D9 | design | The spec's "several windows" edge case and SC-007 asked for every window on a project to show the same status. What happens with two windows on one project? | Feature 010 lets one window hold a project; a refused or displaced window stays on it read-only. Only the holding window reads and shows pull request status; a read-only window shows no indicator and sends nothing; a take-over reads once. Edge case, SC-007, FR-018 and FR-022 reworded. (First wording, "a project is shown in one window at a time", was false; corrected after plan review round 1, F1.) | agent-resolved | `crates/micold-daemon/tests/exclusivity.rs`; `shell/daemon_sync.rs` `Displaced`/`Refused`/`Attached` arms; research.md R6 |
| D10 | design | Is the rate-limit pause of FR-024 shared between windows of one sign-in? | No, it is held by the window that received the answer; another window pauses when its own next reading is rejected (GitHub does not count a rejected request). FR-024 reworded. The pause outlives a project switch in that window (the limit belongs to the sign-in; plan review round 1, F3). Sharing would need a daemon relay for one saved request per window per pause. | agent-resolved | research.md R9; spec.md FR-024, Edge Cases |
| D11 | design | How is "no commits beyond the merged pull request" decided, and what if it cannot be? | Locally, by the daemon: branch tip equal to, or an ancestor of, the pull request's head commit. When that head commit is not in the local repository (pushed from elsewhere, never fetched) no suggestion is shown; the feature does not fetch. Spec Edge Cases, Assumptions, FR-015 and FR-017 say so. | agent-resolved | research.md R11; spec.md FR-015, FR-017, FR-018a |
| D12 | design | Story 1 (P1) has 40 tasks. How is it cut, and why do M1 to M3 ship no UI? | Along its layers, by milestones.md rule 6: no subset of its scenarios can be shown in the application in 15 tasks or fewer, since every scenario needs the core rules (M1), the setting on the wire (M2) and the reading (M3) before a row draws anything. Each of M1 to M3 has a Verify that observes `main`; M4 completes the story and is the MVP. M2 also ships `MergedBranchCheck`, which only M6 uses, because the repository takes one protocol bump per feature. M5 and M8 are `full`: M5 adds the `https://github.com/` guard (a security check), M8's visual pass may need code fixes. | agent-resolved | tasks.md#Milestones; research.md R11; tasks review round 1, F3, F4, F6 |

No `[NEEDS CLARIFICATION]` marker is open after clarify round 1. The round asked questions, so it is
not `CLEAN`.

Clarify round 2 (2026-10-02): coverage scan of all taxonomy categories found no critical ambiguity; no
questions asked, spec.md unchanged. `CLEAN`.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 1c5bb280062f08d77ec432de0fd32e95be915353:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 3 MAJOR (failed reading had two outcomes; what a reading covers vs the manual listing; no outcome above 50 worktrees), 3 MINOR. All six fixed. |
| Spec | 2 | efa54acf195e848c75585bb6eba28c60e3ae2785:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN: 1 MINOR (FR-022 vs FR-024 during a rate-limit pause), fixed. |
| Plan | 1 | a5247dd1cedaf09ec0215b7430bff32fc45a08c0:d1ff3b449ac89dff2650dcfb466239e06ecce20b | CHANGES: 4 MAJOR (a refused or displaced window still shows the project, so "one window per project" was false; start event S1 named `WorktreeMsg::Loaded`, which production never sends; the rate-limit pause was dropped on a project switch; `X-RateLimit-Reset` used without `Remaining: 0`), 3 MINOR (names and signatures across artifacts; FR-015/FR-017/FR-018/FR-022 not following D9 and D11; SC-004, SC-007, SC-010, FR-028, FR-035 in no test layer, request cost figure). All seven fixed. |
| Plan | 2 | 44432235aa3ef419836025515a694e389a5e0538:68bb925ba96bb73e2b5697fe2572270e6bdc867c | CLEAN: 3 MINOR (`Released` must skip the self-collision early returns; no reducer row for `Held` / `ListingArrived` during a reading; another window's `CatalogChanged` can arrive between `Attached` and the fresh listing), all fixed or recorded as an accepted bound. |
| Tasks | 1 | 9a09604522b5326d1486e431c38f1de03af866ea:9501106dd083d16e187bf2851014bfa64d139318 | CHANGES: 3 MAJOR (T043's test of the open handler sat under `tests/`, which cannot reach the binary's `shell`, and `worktree_menu_items` is private; the row's stale threshold was a comparison in `ui/` that no test saw; M5 was `light` with a security check), 3 MINOR (rule 6 not rule 3 for the story 1 split; Verify lines missing suites; M8 `light` with open-ended fixes, T068's checklist clause). All six fixed. Checklist: 16 of 16 ticked items confirmed against the edited spec. `speckit-analyze` before it: 0 CRITICAL, 0 HIGH; I1, E2, E3, I2, I3 applied; E1 answered by this round's F1 and F2. |
| Tasks | 2 | 86514bb7cfe3a10a9d00e54449743e74d39c8730:5ce9f7ffd4d719ce1de292d1a8b91df286732994 | CLEAN: 1 MINOR (T059 and U146 keyed the `Read:` line on `age_secs` while the data model keys it on `stale`), fixed. |
| Code A (M1) | 1 | be2c688facf99c1a98ccb77330add3bf207b3410:9436237451f36aaa08f6690dd91f5c17d8d14005 | CLEAN: 3 MINOR. F1 (`reading_failure` lost the `errors` when the data beside them could not be read) fixed with a test; F2 (`GhCli::read` gave every chunk the reading's start time) fixed; F3 not fixed, see *Follow-ups not done*. |
| Code B (M1) | 1 | f50cb67fdfa4082ea4415240e94f795d0dbc1d5d:e288d6cc75625160e5702584b919fe23603d8aa9 | CLEAN: no findings. Verify `mise run test-core` exit 0, 143 `test result: ok`, 0 FAILED. |
| Code A (M2) | 1 | aeae95f9d6bc9aff627bf0e545263609cd31eabf:22338024d8f1b6ada75d7eddd74425bf9698b587 | CHANGES: 2 MAJOR, 7 MINOR. F1 (arm awaited its blocking task in the connection loop, BUG-009) fixed by spawning; F2 (no `GIT_NO_LAZY_FETCH` on `merge-base`) fixed; F3 (branch name with revision suffix resolved another commit) fixed with a test; F7 (`task_failed`) fixed; F4, F5 declined (see *Declined review findings*); F6, F8, F9 MINOR, not fixed. |
| Code A (M2) | 2 | 45a61c0003058242a677266ac843d5a10759bc5b:2940ceb865da2b715f1304838e7348e45d57a27f | CLEAN: 9 MINOR. F2 (`branch_tip` without the lazy-fetch guard), F4 (guard belonged in `branch_tip`), F5 (duplicate of `naming::is_valid_branch`), F6, F9 fixed; F3 noted in the comment (git before 2.44 ignores `GIT_NO_LAZY_FETCH`); F7 `@{` case added; F1 (no limit on concurrent checks: one trusted local client sends one per reading) and F8 (other arms awaited, outside this flow) not fixed. |
| Code B (M2) | 1 | e371f2bb8f7a3ad63b23b8a4f5d0873f01a88410:773e98e6d3f7f8ce13e4b78e6db0880903639f91 | CHANGES: 1 MAJOR, 2 MINOR. F1 (the Settings save's preservation of `pr_status_enabled` had no test) fixed with a test shown red; F2 (`local_only` split `run_git` from its doc) fixed; F3 (cycle-log hashes from the preparation branch) fixed. Verify: `mise run test-core` 1618 passed; `merged_branch_check` 9, `pr_status_setting` 3 passed. |
| Code B (M2) | 2 | e3935a77c458a870b9dd33178afd00a054fe746d:64239328d1ceb837f428dd93f1b6950baa3c2163 | CLEAN: F1–F3 fixes hold. Verify: `mise run test-core` exit 0; `merged_branch_check` 9, `pr_status_setting` 3 passed. |
| Code A (M3) | 1 | aec34873e2ccf720b22ee7bc0510cecd9d26c1fc:7f3b013ddc67b34868efe9154df411bda4671b68 | CLEAN: 1 MINOR (`Held` while a reading runs), not fixed. |
| Code B (M3) | 1 | aec34873e2ccf720b22ee7bc0510cecd9d26c1fc:7f3b013ddc67b34868efe9154df411bda4671b68 | CHANGES: 2 MAJOR, 1 MINOR. F1 (stray `}` from the rebase merge in main_tests.rs) fixed; F2, F3 declined (see *Declined review findings*). Verify: `pr_status` 16, `features_pr_status` 16, `pr_status_is_read_only_on_named_events` 4 passed. |

## Declined review findings

- Code A (M2) F4: `adopt_daemon_settings` ignores `pr_status_enabled` — T022 says the field is accepted and "stored nowhere yet"; holding it in memory is M3/M4 work.
- Code A (M2) F5: `ValidSettings::into_settings` writes `pr_status_enabled: false` — the one save path restores the stored value; the Settings switch that carries it through the draft is T038 (M4).

- Code B (M3) F2: red before green for T024/T025 behaviours beyond the two mutants — the cycle log already records that these tests were written after the wiring; every behaviour is pinned by a passing test and two were shown red by mutant; a retrofit of ~15 mutants buys no behaviour. Recorded as a known Constitution I deviation for the reviewer of the PR.
- Code B (M3) F3: `crate::log_line` is the client's only logging facility (no level exists).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M4 unit 1 stopped at 136k context. Branch is `feat/worktree-pr-ci-status` reset to `origin/main` (M3 merged, #611 recorded). Nothing pushed, no PR yet.

**Done (committed as one WIP commit `test(040): M4 red tests (T030-T033 part)`)**: tests only, all RED (do not compile until the code exists):
- `tests/icons.rs`, `tests/icons_font.rs` (T031): seven new `Icon` variants `PrOpen e0b6`, `PrDraft e745` (`edit_note`; contract says `edit` but `edit` is `Rename`'s codepoint f097 family and no two icons share one), `PrMerged e0b3`, `PrClosed e14b`, `ChecksPassing e5ca`, `ChecksPending e8b5`, `ChecksFailing e5c9` (`cancel`; contract says `close`, which is `Icon::Close`'s). All these codepoints were checked present in the shipped font's cmap (`e14b`/`e8b5` alias f08c/efd6, same glyph). `Icon::ALL.len()` expected 42. Update `contracts/pull-request-ui.md` §1 table for the two glyph changes (and record as a decision D13 in this ledger).
- `tests/features_sidebar.rs` (T030): expects `micold_client::features::sidebar::{row_pull_request, RowPullRequest}`: `pub struct RowPullRequest<'a> { pub status: &'a PullRequestStatus, pub age_secs: u64 }` (no `stale`/`removable` yet; M5/M6/M7 add them) and `pub fn row_pull_request<'a>(entry: &SidebarEntry, statuses: &'a BTreeMap<String, PullRequestStatus>, read_at: Option<u64>, now: u64) -> Option<RowPullRequest<'a>>` (Worktree with `branch: Some(b)` -> lookup; detached, Default -> None; age = now.saturating_sub(read_at.unwrap_or(now))).
- `tests/features_settings.rs` (T033): `SettingsSection::GithubIssues.label() == "GitHub"`; `GithubDraft { entries, pr_status_enabled: bool }` (Default false, seeded from `Settings.pr_status_enabled` in `from_settings`); `Msg::PrStatusToggled(bool)` via `edit(...)`; `ValidSettings.pr_status_enabled` written by `validate()` and `into_settings()` (drop the `pr_status_enabled: false` stub in `into_settings`).
- `tests/settings_sections.rs`: `DEFERRED` now empty; `ui/settings/github.rs` must declare `SETTINGS` with `("pr_status_enabled", "PrStatusToggled")` and call `page(\n        "GitHub",` (title "GitHub"; keep the issue-mapping intro text sensible).
- `src/main_tests.rs` (U105/U106): `PrStatusToggled(true)` + `Saved` sends `SettingsSet { pr_status_enabled: Some(true) }`; unchanged draft vs the live `app.core.pr_status.enabled` sends `Some(None)`... i.e. `pr_status_enabled: None`; off from on sends `Some(false)`; an open draft follows `SettingsChanged` (do it in `shell/pr_status.rs::enabled_changed`: if `app.core.settings.settings_draft` is Some set `draft.github.pr_status_enabled = enabled`). In `shell/persist.rs`: the open path (line ~269) keeps `pr_status_enabled: stored.pr_status_enabled`; replace the "keeps the document's value" closure (~line 401) with plain `*stored = settings.clone()`; send `pr_status_enabled: (valid.pr_status_enabled != app.core.pr_status.enabled).then_some(valid.pr_status_enabled)` in `SettingsSet` (~line 433). Update `crates/micold-client/src/shell/persist.rs` `ValidSettings` use accordingly.
- `tests/showcase_completeness.rs` (T032): catalogue entry `component: "PullRequestIndicator"`, `module: "material/pull_request_indicator.rs"`, `interactive: false`, 12 `posed` names exactly: "open", "draft", "merged", "closed", "open, checks passing", "open, checks pending", "open, checks failing", "draft, checks passing", "draft, checks pending", "draft, checks failing", "open, checks passing, stale", "merged, stale"; `variants: &[]` (check whether the inventory's enum scan wants `PrMark`/`CheckMark` variants posed; run `cargo test -p micold-client --test showcase_completeness`).

**Next (in order)**: (1) T034: add covered states in `tests/support/covered_states.rs` (registry; patterns: `with_project()`, `state.pr_status.statuses.insert("feat/short".into(), PullRequestStatus{..})`, `read_at`, `enabled`, `held`; Settings state `settings-view-github-issues` needs the new `GithubDraft.pr_status_enabled` field and a new state for the switch; narrowest sidebar width via `state.sidebar.width`; showcase entry state if a showcase state kind exists); (2) T035 icons; (3) T036 `PullRequestIndicator` (model on `ui/material/unread_mark.rs`: builder `new(PrMark, &Roles)`, `.checks(CheckMark)`, `.stale(bool)`, `Into<Element>`; 16 px glyphs, 2 px apart, 16 wide / 34 wide with check, fixed height; roles per contract §1; stale = `outline` role; export in `ui/material/mod.rs`; catalogue entry in `src/showcase/catalogue.rs` + render fn in `src/showcase/sections/atoms.rs`; add a unit test for width 16/34 like `unread_mark.rs` tests; `docs/development/component-library.md` section); (4) T037: in `ui/sidebar.rs::build_items` pass `state.pr_status.statuses`, `read_at`, `now` (a local `now()` helper in the view glue, `SystemTime`, like `shell/pr_status.rs:45`) and put the indicator first in the row's trailing element before `row_actions_cluster` (a `row![indicator, cluster]` only when `Some`; `None` must build exactly today's tree); map `PrState`/`CheckStatus` to `PrMark`/`CheckMark` in the sidebar; (5) T038 as above plus the `Checkbox` + `field_note` in `ui/settings/github.rs` with the contract §5 wording (see `ui/settings/environment.rs` for `Checkbox::new(...).on_toggle(...)` and `field_note`); (6) T039 regenerate `tests/fixtures/layout_snapshot.txt` (see `tests/layout_snapshot_regeneration.rs` for the command); (7) T040 docs: `docs/user-guide/worktrees-and-sessions.md`, `docs/user-guide/settings.md` (rename "GitHub issues" heading to "GitHub" too), `docs/development/component-library.md`; (8) tick T030-T040 in tasks.md, append cycle-log entries to `tdd/cycle-log.md` and U98-U110 states in `tdd/test-list.md`; (9) verify.md: scoped gate + review A (high) + review B + visual pass (quickstart §B1-§B3; also ask it to confirm the seven glyphs look right in the rendered showcase), full gate with `MICOLD_SKIP_GH_LAUNCH_TEST=1`, PR titled `feat(040): the pull request indicator and its Settings switch (#486)`, body ends `Refs #486`.

## Open escalation

None.

## Token usage

## Follow-ups not done

- M1 review A, F3 (MINOR, not confirmed against GitHub): a sign-in that may read pull requests but not checks could get a field-level `FORBIDDEN` error on `statusCheckRollup`. Contract PS §5 makes any `errors` entry a failure, and `github::classify` reads "resource not accessible" as no access, so the whole project would show nothing (`Unavailable`). Deciding whether such an error is tolerated (checks read as none) needs a recorded answer from a token of that kind and a contract change.
