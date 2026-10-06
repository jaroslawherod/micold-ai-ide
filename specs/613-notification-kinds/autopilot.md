# Autopilot ledger — #613 notification-kinds

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #613 "Notifications: only notify when a session needs attention; add per-kind settings and icons" (notify by default only when a session needs real attention — waiting for input or permission, errored, finished a long task; per-kind on/off settings; a distinct icon per kind; builds on specs/039 and 575). labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #613
- **Worktree branch**: claude/project-thread-8kdqkn
- **Started**: 2026-10-06
- **Phase**: implement
- **Next step**: milestone M1 continue from *Handover*

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #618 | whole run (design + milestones), one PR from claude/project-thread-8kdqkn | draft, open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T020 | full | Awaiting-input kinds, notified by their defaults (short turns silent, permission and long turns titled by kind; SubagentStop ignored) | #618 | in progress |
| M2 | T021–T031 | full | Session error notifications (give-up, Copilot session.error) to one window | — | todo |
| M3 | T032–T045 | full | Per-kind switches in Settings with icons; service stores and applies them | — | todo |
| M4 | T046–T052 | full | Kind icons in desktop notifications (Linux image-data, Windows/macOS PNG) | — | todo |

M1 (16 story tasks) and M3 (14) exceed ~10 tasks: kept whole. M1's TurnClock, Views pending kinds and the client kind only deliver together; US1 was already split along its scenarios (error endings = M2). M3's service switches without the Settings rows would leave nothing a user can observe. Phase 7 (T053–T055) changes no code: close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Do disabled kinds change unread marks? | No: unread stays as 039 (FR-018) | agent-resolved | issue #613 asks about notifications only; 039 FR-017 |
| D2 | spec | Defaults per kind | Needs permission, Session error, Long task finished on; Turn finished off | agent-resolved | issue #613 "Expected" bullet 1 |
| D3 | spec | Keep 039's Desktop notifications switch? | Yes, as master switch above per-kind switches | agent-resolved | 039 FR-026; stored value carries over |
| D5 | plan | Claude's `SubagentStop` (mapped to `Stop` by 010): keep, or stop treating it as a turn end? | Ignore it (`HookClass::Ignored`, no longer registered); spec FR-024 + edge case, FR-018 exception | agent-resolved (plan review round 1 F1–F4) | it marked working sessions waiting/unread mid-turn and lost the real turn end; mapping was incidental (010 BUG-001) |
| D4 | clarify | Long-task threshold fixed at 60 s or adjustable in Settings? | B: adjustable in Settings, default 60 s — decided by user (2026-10-06); spec/plan/tasks to be amended before M3. M1 keeps `LONG_TASK_THRESHOLD` as the default, passed to `TurnClock::change` as an argument so a setting can feed it | user | orchestrator relay 2026-10-06 |
| D6 | tasks | speckit-analyze F1–F15: spec/plan fixes | Spec: FR-024 carve-out in intro and Out of Scope, US1.11 (subagent stop), FR-004/Terms (abnormal exit counts only when not restarted), FR-015 same glyph, FR-017 enabled rows only, edge cases (refused permission; event noted before a service restart is not notified), assumption (only Copilot reports errors). Contract C2 threshold field (test seam), C3 TurnFinished fallback. Plan wire 30–32. F11 (FR numbering), F12 declined as taste/accepted | agent-resolved | analyze report; data-model pending-kind rule; state.rs note_activity |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c48e430ee5881189ede316fafd185dd594ab7342:f613369477c59a5fef3b3b56f9085d05eb440ff2 | CHANGES: 1 MAJOR, 2 MINOR (all fixed) |
| Plan | 1 | 50bb81b82acbb6a541a083394cf52b15c2fc0de0:718bb460f44042a7afb6a55cba3acd2e0b94dc62 | CHANGES: 4 MAJOR, 1 MINOR (all fixed: R3 rewritten, SubagentStop → Ignored, FR-024, C17) |
| Plan | 2 | 56cef6625431cd768db0299b038eb5ab912f5dd0:6db8b02a59ca25fb6b5d67f0be14a3bdd497bf16 | CLEAN (1 MINOR, fixed, prose only: plan names HooksMap.subagent_stop) |
| Spec | 2 | 4e5c412acfa49fdd6f0f43625e21fd7c3ebc035a:f613369477c59a5fef3b3b56f9085d05eb440ff2 | CLEAN (1 MINOR, fixed: FR-017 3:1 vs white and black) |
| Tasks | 1 | 9a5fcef479387b7f6b0609e566bbd79d671c39aa:14028ac91c0fbb74f7f5b7af762b1d64d48359b8 | CHANGES: 1 MAJOR (T006 [P] on a shared file), 2 MINOR — all fixed, prose only (task markers, Verify line, checklist tick); review done |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 2 handed over at 150k context (commit "feat(613): M1 daemon and client kinds (WIP)").

- **Done since unit 1** (ticked): T002/T004 (catalog `notification_kinds()`, `notify(kind)`,
  `persist_service_settings` carries kinds; `catalog.rs` `notify_tests`), T008/T015 (SubagentStop
  ignored + unregistered; 010 contracts/hooks.md updated), T008/T016 (`turn_change`), T009/T017
  (`Views::grant/note_event`, pending pruning), T011/T013/T019 (client kind, title by kind; client
  save keeps stored `notification_kinds` like `pr_status_enabled`). T018 code done in state.rs
  (threshold lives in `Inner.long_task_threshold`, setter `set_long_task_threshold`).
- **Green**: workspace builds all targets, no warnings; daemon lib attention/activity/hooks/catalog
  47 ok; client attention_notify 14 ok; client bin desktop_notify/persist ok. The 039 harnesses
  (`attention_claims`, `settings_desktop_notifications`, `unread_state`) use `state_on()` =
  threshold ZERO so every finished turn is a long task.
- **Not yet re-run**: `attention_claims.rs` after fixing its two failures (supervision test now
  expects refusal per C12; SC-001 test claims after each turn as a window does — it uses an async
  closure, may need reworking to compile). Then run `--test settings_desktop_notifications
  --test unread_state --test hooks_receiver --test attention_events` (cargo stopped before them).
- **Next**: re-run those; T010 copilot/pi long-turn tests (`copilot_activity.rs`, `pi_activity.rs`:
  set threshold, long turn → claim granted `LongTaskFinished`), tick T010/T018; T020 user guide;
  record cycles 4+ in `tdd/cycle-log.md`; then verify.md (scoped gate + review A high, review B,
  full gate, push, `$SCRATCHPAD/pr-613-body-M1.md`).
- Open findings: none. PR: #618 (draft, open). D4 = B (user).

## Open escalation

None.

## Follow-ups not done

None.
