# Autopilot ledger — #813 worktree-overview

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #813 "feat: worktree overview with state-aware cards (sessions, PRs, task
  progress)" (title and body as given to the spec unit); labels: enhancement, effort:high,
  flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #813
- **Worktree branch**: claude/project-thread-2lmupk
- **Started**: 2026-10-17
- **Phase**: plan
- **Next step**: plan unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Definition of "main session" (issue open question) | Feature 776's main session: the un-archived session marked main, else the most recently active | agent-resolved | crates/micold-core/src/occupancy.rs `main_session`; specs/776-one-session-per-worktree |
| D2 | spec | Is the LLM summary (option 3) in the first iteration? | No: out of scope; options 1 and 2 ship (issue's recommendation) | agent-resolved | issue #813 "Recommended: 1 as the baseline and 2 where hooks are configured" |
| D3 | spec | Card state vs feature 905's location state | Reuse 905's state; Blocked: needs permission → Needs action, Blocked: failed/resumable → Needs attention | agent-resolved | crates/micold-core/src/location_state.rs |
| D4 | clarify | FR-023: Overview in the main pane or as a window overlay? | Main pane, like Changes/Compare (482/483); sidebar and top bar stay; a sidebar selection closes it | agent-resolved | crates/micold-client/src/ui/mod.rs `view`; spec FR-009/FR-012 need the top bar visible |
| D5 | clarify | Overview shortcut while Settings is open | Deferred to plan: follow how Settings yields to other main-pane views | agent-resolved | crates/micold-client/src/ui/mod.rs; feature 027 FR-026 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c54be34533424e39665e73004ef6ce8f0cadc4e2:992e129808513fd43ef6b1982b4cb1881cdb0921 | CLEAN (3 MINOR: F1 F2 F3 fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Plan unit 1 hit the 150k cap during research; nothing of plan.md written yet. Next: a fresh plan
unit runs `speckit-plan` (setup-plan.sh copies the template) using these findings, then review.

Findings (all paths under `crates/`):

- **Main pane**: `micold-client/src/ui/mod.rs` `view_with` picks Settings (`settings.settings_draft`,
  whole content area incl. sidebar) → Changes (`changes.open`) → Compare (`runs.compare`) →
  terminal → `shell::view`. The same list is repeated in `main_content_key` (sentinels
  `u64::MAX..MAX-2`), `app.rs` `terminal_focused` and `view_facts.main_area_taken`. Sidebar
  selection closes Changes/Compare in `main.rs` `Message::Session(SessionMsg::Selected)` (sends
  `ChangesMsg::SessionSelected`, `runs::Msg::CompareClosed`). Escape: `app.rs` `on_escape`
  (registry first, then Settings cancel); `ui/mod.rs` `subscription` holds Escape/Tab listeners
  only while the terminal is not focused.
- **D5 evidence**: nothing yields Settings to another main-pane view; Settings closes only by
  save/cancel (`features/settings.rs` `saved`, `cancelled`), and Changes/Compare are reached from
  the sidebar, hidden while Settings is open. Proposed answer: the shortcut and button do nothing
  while Settings is open (the draft is not discarded); record in research.
- **Reserved chords**: `micold-client/src/keymap.rs` `encode` tiers 1/1a/1b
  (`is_primary_shift_chord(key, mods, letter)`), `KeyOutput::{ReleaseFocus,NewTerminalInstance,
  Pane}`; `app.rs` `route_key`/`KeyRouting`; `ui/material/terminal_pane.rs` ~L1724 publishes the
  message and `capture_event()`. Unfocused: no global shortcut listener exists yet; a new
  `keyboard::listen().filter_map(named fn)` in `ui/mod.rs subscription` is needed (TypeId rule).
- **Top bar**: `micold-client/src/ui/toolbar.rs` `view`: `Toolbar::new(..).action(usage)
  .action(switcher).action(menu)`; switcher is `Button::text(..).leading(Icon::OpenProject)
  .trailing_mark(..)`. Shared parts in `ui/material/`: `IconButton`, `Button`, `UnreadMark`,
  `StateIcon` (905), `Tag`, `Surface`, `StageProgress` (indeterminate only; no determinate bar),
  `ReviewCommentCard` (no generic card). Need new shared `OverviewCard`/`AttentionBadge` +
  determinate progress; showcase in `showcase/catalogue.rs`, `tests/showcase_completeness.rs`.
- **Locations**: `features/sidebar.rs` `State::sidebar_entries()` (Default + filtered worktrees +
  run `Group`s with `RunRow::Worktree`), `WorktreeNode`/`DefaultNode` with `main_session()`,
  `state()` (905 `micold-core/src/location_state.rs` `location_state`), `worktree_tags`, `daemon`.
  Main session: `micold-core/src/occupancy.rs` `main_session`. Default never has a PR
  (`row_pull_request`).
- **PRs**: `micold-core/src/pull_request.rs` `status_query` already asks per branch `o{i}` (open,
  newest 10) and `r{i}` (newest 10 any state); `parse_status` → `BTreeMap<branch,
  PullRequestStatus>` (only branches with a PR); `PullRequestStatus{state: PrState{Open/Draft
  {checks}, Merged, Closed}, review}`. No counts, no `mergeable`. Plan: add `mergeable` to
  FRAGMENT, a merged alias `m{i}` (nodes `isCrossRepository`, first 100) and per-branch counts
  returned beside statuses; client `features/pr_status.rs` `State{enabled, statuses, removable,
  read_at}` gains counts; `Outcome::Ok`, `Msg::StatusesRead`, `read_outcome` signatures change;
  `shell/pr_status.rs` carries them. Unknown counts when disabled/Unavailable/unread.
- **Agent requests**: `features/agent_confirm.rs` `Prompt` drops the wire's `caller: SessionId`
  (`DaemonMsg::ConfirmationRequested` in `micold-core/src/protocol/messages.rs` ~L1093 has it);
  add it to `Prompt` (client only, no protocol bump). Protocol is 48.
- **Hooks**: `micold-daemon/src/hooks.rs` `handle_connection` → `classify_hook(body)` reads only
  `hook_event_name`; body never logged; `PostToolUse` carries `tool_name`/`tool_input` (TodoWrite
  todos), `UserPromptSubmit` carries `prompt`. Story 5 needs a new extraction there, a per-session
  activity summary in `DaemonState` (`state.rs` `note_activity`, `overlay_live_summaries`), a
  `SessionSummary` field (`serde(default)`, protocol 48→49) adopted in `micold-client/src/
  catalog_sync.rs` like `paused_on_user`.
- **Not yet looked at**: issue-title lookup (feature 532, `shell/issues.rs`,
  `micold-core/src/github.rs`/`issue_details.rs`), worktree naming parse (`micold-core/src/
  naming.rs`, `worktree.rs`), spec-kit feature record (`.specify/feature.json`) and a disk reader
  for tasks.md / ledger off the UI thread (see `shell/changes_watch.rs`, `shell/runs_watch.rs`
  for watcher patterns), start-session offer from the sidebar (`SessionMsg::StartMenuOpened`).
- **Docs**: no shortcut list in Settings; shortcuts are in `docs/user-guide/worktrees-and-sessions.md`
  (~L1590 terminal focus) and `docs/user-guide/terminal-panes.md` §Keyboard shortcuts; new page goes
  in `docs/user-guide/` + `docs/SUMMARY.md`. Plan format model: `specs/905-worktree-state-icon/`.

## Open escalation

None.

## Follow-ups not done

None yet.
