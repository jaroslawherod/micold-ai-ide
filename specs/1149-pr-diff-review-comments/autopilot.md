# Autopilot ledger — #1149 pr-diff-review-comments

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1149 "Review comments on a pull request's diff, sent to the agent session to apply and push to the PR" (verbatim in spec.md Input); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1149
- **Worktree branch**: feat/1149_review-comments-on-a-pull-request-s-diff-sent-to
- **Started**: 2026-10-20
- **Phase**: implement (M1)
- **Next step**: M1 (after design PR merges)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1153 | Design (spec, clarify, plan, tasks) | merged | 0ebf14ba355194296f2d6d1b0c51597140a601d3 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Write, list, tidy PR comments on a PR's diff (whole wire delta, Send for PR not yet offered) | | pending |
| M2 | T012–T022 | full | Send review on a PR: build_pr batch, branch/commit reads, daemon delivery | | pending |
| M3 | T023–T027 | full | Anchoring on hunks, head-follow, outdated | | pending |

## Decisions

- Clarify: wrong branch on Send review delivers with a switch-branch preamble; merged/closed PR allows Send review and states the state in the batch (decided by user).

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | bfa4381fe14264233731c90e5798d8826424308b:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CLEAN (3 MINOR, fixed, prose only) |
| Plan | 1 | 9da7410d0fbbfa3b1efa48b7b2efc964b3c3010b:dc70782b6bf0193f8b7575e71e273abb07ee59f6 | CHANGES (2 MAJOR, 3 MINOR, all fixed) |
| Plan | 2 | c3f8e55734895dfd9d4f21aa89c678c63359b917:dc70782b6bf0193f8b7575e71e273abb07ee59f6 | CLEAN (2 MINOR, fixed, prose only) |
| Tasks | 1 | 2e2a21d914c701e16939ae8cc0f2572fcee721b1:18eb11b47a1f2d64bcfddf0c3eb39eeee17c4324 | CHANGES (2 MAJOR, 2 MINOR, all fixed: test-first order of T020/T021, W21 test into T006) |
| Tasks | 2 | diff HEAD~2 | CLEAN (2 MINOR, no action) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Unit M1 handed over at 150k (no PR yet; WIP committed locally, not pushed; gate not run; no review run).

Done (compiles and passes `cargo test -p micold-core --all-targets`): T001-T005 in `crates/micold-core`
(`BranchPrState` serde, `PrAnchor`/`Scope`/`ReviewComment.pr`/`Draft.pr`, scoped
`begin_send(entry, scope, pr, outdated)`/`clear_sent(scope)`/`discard_pending(scope)`, `PrContext` in
`review/prompt.rs`, store v3 written only with PR comments, wire `Add.pr`, `ClearSent{scope}`,
`DiscardPending{scope}`, `ReviewSend.pr: Option<PrSend>`, `PrSend`, PROTOCOL_VERSION 57 and pins updated).
Daemon source done (T007): `crates/micold-daemon/src/review.rs` (W18 anchor check, W19 scoped tidy with
no-op `Ok(None)`, `relocate` skips PR comments), `state.rs` `review_send(.., pr)` answers InvalidInput for
`Some`, `server.rs` passes `pr`. Daemon tests (T006) written in `tests/review_edit.rs` (W18, W19, edit/delete/
forget, W21), existing tests in review_send/review_reports/mcp_audit_log/review_edit patched for the new
fields; NOT yet compiled or run because `micold-daemon` tests depend on `micold-client`, which does not
compile yet.

Next (client, T008-T011; `crates/micold-client/src/features/changes.rs` has 3 compile errors at the
`ReviewEditOp::ClearSent` (~1369), `DiscardPending` (~1393), `Add` (~1638) sites):
- `PullRequestView.head: String` set in `read_pull_request_diff` from `view.pull_requests.shown()` entry
  `.head` for `number` (R3: head when the read started); `view_scope(view) -> Scope`.
- `can_pick`: true for `Source::PullRequest` with `pr_file_diff` = `FileDiff::Text` (use `diff_body(view)`),
  and for `Source::Changes` as now. `GutterPressed` already uses `can_pick`.
- `composer_saved`: file = `selected_file(view)`, diff = `diff_body(view)`; for a PR the diff has
  `old: None, new: None`, so take the quote from the hunks' `DiffLine`s (`line.new`/`line.old` + `text`),
  return `Effect::None` if a picked line is not in a hunk; set `pr: Some(PrAnchor{number, head_branch
  (from listed PR), head_commit: pr.head})`.
- Scope everything by `view_scope`: `file_comments` (drop the `Source::Changes` early return; filter
  `comment.scope()==view_scope`), `pending_counts`, `status_counts`/`review_toolbar_label` (shown scope),
  `jump_target`, `tidy_actions` + `ClearSentPressed`/`DiscardConfirmed` emit `ClearSent/DiscardPending
  { scope: view_scope }`, `send_action` count the shown scope and be `None`/disabled for a PR until M2,
  `Effect::ReviewSend` and `shell/changes.rs` (~202-211) add `pr: None`; `judge_anchors`: PR comments keep
  their stored range and are never outdated or moved in M1 (M3 does the hunk work); `reanchor_effect`
  unchanged (working tree only).
- `panel_rows`: `PanelRow.scope`, ordered working tree then PRs by number; add `PanelGroup { scope,
  title: Option<String>, listed: bool }` (title from `view.pull_requests.shown()`; `listed:false` when the
  number is not in the list) and a function giving the groups; a not-listed scope has `send_action` None.
- `ui/changes.rs`: group heading per PR in the panel (existing `Text` styles, "not in the list" flag).
- Tests-first T008 in `crates/micold-client/tests/features_changes.rs` (existing fixtures `comment()`,
  `quoting()` construct `ReviewComment` and `ReviewEditOp::Add`/`ClearSent`/`DiscardPending`: add `pr:
  None` / `scope: Scope::WorkingTree`); also fix `tests/support/modals.rs` (~188, ~219) and
  `src/showcase` if they build `ReviewComment`.
- T011 user guide `docs/user-guide/reviewing-changes.md`.
- Then: tick T001-T011 in `specs/1149-pr-diff-review-comments/tasks.md`, run the daemon tests
  (`cargo test -p micold-daemon --test review_edit --test review_send --test review_reports --test
  mcp_audit_log` via `scripts/build-lock.sh`), then follow verify.md (scoped gate, review A high, review B
  once, full gate) and pr.md. Run builds detached with `scripts/autopilot/hold.sh` (foreground cap 250 s).
  `cargo fmt --all` before the gate.


## Open escalation

None.

## Follow-ups not done

None.
