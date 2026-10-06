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

M1 unit 1 handed over at 150k context (commit "feat(613): M1 core kinds, turn clock, grant kind (WIP)").

- **Done, micold-core green** (`cargo test -p micold-core --all-targets`: 1441 passed, 1 failed =
  `settings_refuses_save_over_failed_read`, root-only, fails on the unchanged tree too): T001, T003,
  T005, T006, T007, T012, T014 ticked. `NotificationKind`, `NotificationKinds`,
  `LONG_TASK_THRESHOLD`, `TurnChange`, `TurnClock::change(change, now, threshold)` (threshold is an
  argument, ready for D4=B), `notification_text(kind, …)`, `Settings::notification_kinds` (both
  forms + conversions), `AttentionGranted.kind`, `PROTOCOL_VERSION` 30. Cycle evidence:
  `tdd/cycle-log.md` cycles 1–3 (no `tdd/test-list.md`: driven from tasks.md order, noted there).
- **The workspace does not build yet**: daemon `state.rs` (grant/AttentionGranted) and client
  `app.rs:471` (`notification_text` without kind) are the callers left for T013/T018/T019.
- **In progress, T009/T017**: `crates/micold-daemon/src/attention.rs` has the new `Views` tests
  (U54–U68 rewritten to note a kind before a claim, plus C10–C12 and Principle II tests) and
  `todo!()` stubs for `grant(session, seq, current, notify: impl Fn(kind)->bool) -> Option<kind>`
  and `note_event(session, seq, kind, notify)`; field `pending` added. Red not yet run (daemon does
  not compile until state.rs is updated). Implement: note_event → notify ? push pending : granted =
  max(seq) and prune pending ≤ granted; grant → None if seq > current or ≤ granted, or no pending
  kind for seq, or !notify(k) (record nothing); else granted = seq, prune ≤ seq, Some(k);
  forget_session also clears pending.
- **Next**: T002/T004 catalog part (`persist_service_settings` carries `notification_kinds`;
  `notification_kinds()`, `notify(kind)`; tests: load a Catalog from a temp settings file —
  `JsonFileStore::at`, `JsonFileSettingsStore::at`), T008/T015 (hooks.rs: SubagentStop → Ignored,
  drop `HooksMap.subagent_stop`; update `classifies_hook_event_names` and `settings_json` tests;
  010 contracts/hooks.md), T008/T016 (`turn_change` in activity.rs), T010/T018 (state.rs),
  T011/T019 (client), T020 (guide). Then T013 can be ticked. Then verify.md (gate, review A/B).
- Open findings: none. PR: #618 (draft, open). D4 updated to B (user).

## Open escalation

None.

## Follow-ups not done

None.
