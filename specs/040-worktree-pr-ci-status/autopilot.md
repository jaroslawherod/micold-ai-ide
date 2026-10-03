# Autopilot ledger — 040-worktree-pr-ci-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #486: https://github.com/jaroslawherod/micold-ai-ide/issues/486 — "Show pull request and CI status for each worktree". The run starts from GitHub issue #486. Follow .claude/skills/speckit-autopilot/references/issue.md: run `scripts/autopilot/issue.sh start 486 feat/worktree-pr-ci-status`, record **Issue** `#486` in the ledger, and end every PR body with `Refs #486` (never `Closes #486`). The issue's body is quoted in spec.md's **Input** line.
- **Kind**: feature
- **Issue**: #486
- **Worktree branch**: feat/worktree-pr-ci-status
- **Started**: 2026-10-02
- **Phase**: 4-milestone
- **Next step**: M3 unit: implement T023–T029, gate, reviews A and B, open the PR.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #529 | Spec | merged | e90a18f9969fe111c9aa6bfb666e31276f3ae38b |
| #536 | Design | merged | 47f73eb184695cfcd1fb5cdc6129ee4c36dd8f4a |
| #541 | M1 | merged | e496e95c63b9a776ac22921f78ee100ea6b505b1 |
| #547 | M2 | merged | 2e688bf3d01bed2c3ae63681ecd05be8e6db3648 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | `micold-core` reads pull requests through `gh` and turns recorded answers into per-branch statuses and failure kinds (US1 core; no UI) | #541 | merged |
| M2 | T014–T022 | full | Protocol 22 (039 took 21): the daemon stores and broadcasts `pr_status_enabled` and answers `MergedBranchCheck` (no UI) | #547 | merged |
| M3 | T023–T029 | full | The holding window reads pull request status on the listing after `Attached` and on switch-on, and holds it in memory (no UI) | | in progress |
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

## Declined review findings

- Code A (M2) F4: `adopt_daemon_settings` ignores `pr_status_enabled` — T022 says the field is accepted and "stored nowhere yet"; holding it in memory is M3/M4 work.
- Code A (M2) F5: `ValidSettings::into_settings` writes `pr_status_enabled: false` — the one save path restores the stored value; the Settings switch that carries it through the draft is T038 (M4).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M3 unit 1 handed over at 150k (no PR yet). Branch reset onto origin/main 2e688bf3; #547 recorded merged.

**Done (WIP commit, not yet compiled, red not yet recorded):**
- `crates/micold-client/src/features/pr_status.rs`: `State`, `Phase { Idle, Reading { seq, again, started } }`, `Outcome`, `Msg { Held, ListingArrived, EnabledChanged, Finished, Released, RemotesTimedOut { seq, req } }`, `Effect`; `update` is a stub returning `Effect::None`. `RemotesTimedOut` is a shell-only variant (the 10 s bound on the reading's `RemoteList`); the reducer ignores it. `Trigger`/`Cause` are M7's, left out.
- registered in `features/mod.rs`.
- `crates/micold-client/tests/features_pr_status.rs` (T023, U70–U84) written against that API.

**Next step:** run `scripts/build-lock.sh cargo test -p micold-client --test features_pr_status`, record the red in `tdd/cycle-log.md` (style: one compact `### M3 cycle` entry per behaviour group, as M2's), implement `update` per RW §2 table + DM "State transitions", then T026 root wiring, T027, T028, T024, T025, T029.

**Design already settled (do not re-explore):**
- Feature-registration guard (`tests/feature_registration_cost.rs`): `features/pr_status.rs` has `pub enum Msg` but `update` returns `Effect`, so it is **shape B**: `shell/pr_status.rs` must declare `pub fn update(app: &mut App, msg: Msg) -> Task<Message>` and name `features::pr_status::Msg`; add `pub mod pr_status;` to `shell/mod.rs`. Only `app.rs` may call `features::pr_status::update(` → add `State::update_pr_status(&mut self, msg) -> Effect` in `app.rs` (like `update_session_for_effects`), add `pub pr_status: crate::features::pr_status::State` to root `State`, add `Message::PrStatus(crate::features::pr_status::Msg)` and decline it in `State::update` beside `Message::Connection(_) | Message::Sandbox(_)`. `main.rs` `update_inner`: `Message::PrStatus(msg) => shell::pr_status::update(app, msg)` (near line 595). Then run `feature_registration_cost`, `feature_write_isolation`, `root_state_is_shared`, `features_are_render_free`.
- T027: add `pull_requests: Arc<dyn Fn(PathBuf) -> Arc<dyn PullRequestSource + Send + Sync> + Send + Sync>` to `IssueTooling` (`shell/capabilities.rs`); `real()` builds `GhCli::new(gh)` (GhCli impls PullRequestSource, `micold-core/src/github.rs:1155`); `IssueTooling::none()` gets an unreachable factory; one existing literal `IssueTooling {` in `main_tests.rs:5157` needs the field. Check `no_concrete_implementations`.
- T028 `shell/pr_status.rs`: `update` calls `app.core.update_pr_status(msg)`; `Effect::Read{seq}` → `start(app, seq)` (the single `start(` call line). `RemotesTimedOut{seq,req}` handled before the reducer: if `app.pending_ops.remove(&req)` is `Some` → `update(app, Finished{seq, Err(Passing)})`. `start`: branches = `app.core.worktree.worktrees` with `branch: Some`, dedup in order; no daemon → Finished Passing directly (never `send_op`'s notice); else `send_op(PendingOp::PrStatusRemotes { project, seq, branches, started })` sending `ClientMsg::RemoteList`, and return `Task::perform(sleep(10 s), RemotesTimedOut{seq,req})`. New `PendingOp` arm in `daemon_sync.rs`: describe(); OperationResult (~line 771) → `pr_status::on_remotes(app, ..., Ok(remotes))`; OperationError (~887) → Passing; disconnect drain (~312) → nothing. `on_remotes`: drop unless `phase == Reading{seq}` and project still active; `micold_core::github::choose_remote` → `NoGithubRemote` ⇒ Finished Unavailable; else spawn_blocking: env-include PATH as `shell/issues.rs::start_issue_load` does, `(tooling.locate_gh)` None ⇒ Unavailable, else `(tooling.pull_requests)(gh).read(&repo, &branches, now)` ⇒ Finished{Ok{statuses, removable: empty, started_at: started}} or Err. One `crate::log_line` (no tracing dep; it is silent under cfg(test)) naming outcome kind + branch count, never title/url. `now` = SystemTime UNIX_EPOCH secs helper in shell/pr_status.rs.
- T029 wiring in `daemon_sync.rs`: `Attached` arm (~1028) → `Held` when `project == app.core.workspace.active`; `CatalogChanged` arm (~607) after `reconcile_catalog` → `follow_up = pr_status::update(app, ListingArrived{now})` (the one line); `Released` right after each of the two `app.displaced.insert` (~977, ~1007) when project is active, and in `on_disconnected` (~260); `SettingsChanged` arm (~624) and `on_connected` (~1080, after `adopt_daemon_settings`) → `EnabledChanged{settings.pr_status_enabled}` (capture the bool before `settings` moves). `workspace.rs`: `Released` before `switch_daemon_attachment` at ~216 and ~246 when `previous != Some(path)`; also on forget of the active project (`daemon_sync.rs` ~1262).
- T025 tests in `main_tests.rs`: copy `issue_rig` (~5151), `settle` (~5188), `remote_lists_sent`, `answer_remotes` (~5228), `messages()` (~2322, 10 s cap — so never run the 10 s timer Task: drop the Task returned by the `CatalogChanged` step, answer remotes, settle that; test the timeout by sending `Message::PrStatus(Msg::RemotesTimedOut{..})` directly). Fake: `micold_core::pull_request::FakePullRequestSource` (`with_answer`, `with_failure`, `calls()`).
- T024 gate: copy `tests/issues_are_requested_only_on_named_events.rs` (MARKERS + ALLOWED with reasons); also assert no `log_line` in `shell/pr_status.rs` formats `title`/`url`.

## Open escalation

None.

## Token usage

## Follow-ups not done

- M1 review A, F3 (MINOR, not confirmed against GitHub): a sign-in that may read pull requests but not checks could get a field-level `FORBIDDEN` error on `statusCheckRollup`. Contract PS §5 makes any `errors` entry a failure, and `github::classify` reads "resource not accessible" as no access, so the whole project would show nothing (`Unavailable`). Deciding whether such an error is tolerated (checks read as none) needs a recorded answer from a token of that kind and a contract change.
