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
- **Next step**: M2 handover: review A + scoped gate, review B, full gate, push (see Handover)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #618 | whole run (design + milestones), one PR from claude/project-thread-8kdqkn | draft, open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T020 | full | Awaiting-input kinds, notified by their defaults (short turns silent, permission and long turns titled by kind; SubagentStop ignored) | #618 | done (pushed; full gate green at b67fb7b7) |
| M2 | T021–T031 | full | Session error notifications (give-up, Copilot session.error) to one window | #618 | implemented (reviews pending) |
| M3 | T032–T045, T056–T066 | full | Per-kind switches in Settings with icons; service stores and applies them | — | todo |
| M4 | T046–T052 | full | Kind icons in desktop notifications (Linux image-data, Windows/macOS PNG) | — | todo |

M1 (16 story tasks) and M3 (25, with the D4 threshold tasks) exceed ~10 tasks: kept whole. M1's TurnClock, Views pending kinds and the client kind only deliver together; US1 was already split along its scenarios (error endings = M2). M3's service switches without the Settings rows would leave nothing a user can observe. Phase 7 (T053–T055) changes no code: close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Do disabled kinds change unread marks? | No: unread stays as 039 (FR-018) | agent-resolved | issue #613 asks about notifications only; 039 FR-017 |
| D2 | spec | Defaults per kind | Needs permission, Session error, Long task finished on; Turn finished off | agent-resolved | issue #613 "Expected" bullet 1 |
| D3 | spec | Keep 039's Desktop notifications switch? | Yes, as master switch above per-kind switches | agent-resolved | 039 FR-026; stored value carries over |
| D5 | plan | Claude's `SubagentStop` (mapped to `Stop` by 010): keep, or stop treating it as a turn end? | Ignore it (`HookClass::Ignored`, no longer registered); spec FR-024 + edge case, FR-018 exception | agent-resolved (plan review round 1 F1–F4) | it marked working sessions waiting/unread mid-turn and lost the real turn end; mapping was incidental (010 BUG-001) |
| D4 | clarify | Long-task threshold fixed at 60 s or adjustable in Settings? | B: adjustable in Settings, whole seconds, default 60 s, next to the Long task finished switch — decided by user (2026-10-06). Amended: spec (Clarifications, Terms, US2.10–13, FR-012–FR-014, FR-022, FR-023, new FR-025/FR-026, SC-006, new SC-008, Assumptions), plan, research R2, data-model "Long-task threshold", contracts C2/C6, W5.6, S5–S7, quickstart §B9, tasks T056–T066 in M3. Bounds 10–3600 s, clamp on read/service, refuse on save, after `env_include_timeout_secs`. M1 code untouched; `LONG_TASK_THRESHOLD` stays the default | user | orchestrator relay 2026-10-06 |
| D6 | tasks | speckit-analyze F1–F15: spec/plan fixes | Spec: FR-024 carve-out in intro and Out of Scope, US1.11 (subagent stop), FR-004/Terms (abnormal exit counts only when not restarted), FR-015 same glyph, FR-017 enabled rows only, edge cases (refused permission; event noted before a service restart is not notified), assumption (only Copilot reports errors). Contract C2 threshold field (test seam), C3 TurnFinished fallback. Plan wire 30–32. F11 (FR numbering), F12 declined as taste/accepted | agent-resolved | analyze report; data-model pending-kind rule; state.rs note_activity |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c48e430ee5881189ede316fafd185dd594ab7342:f613369477c59a5fef3b3b56f9085d05eb440ff2 | CHANGES: 1 MAJOR, 2 MINOR (all fixed) |
| Plan | 1 | 50bb81b82acbb6a541a083394cf52b15c2fc0de0:718bb460f44042a7afb6a55cba3acd2e0b94dc62 | CHANGES: 4 MAJOR, 1 MINOR (all fixed: R3 rewritten, SubagentStop → Ignored, FR-024, C17) |
| Plan | 2 | 56cef6625431cd768db0299b038eb5ab912f5dd0:6db8b02a59ca25fb6b5d67f0be14a3bdd497bf16 | CLEAN (1 MINOR, fixed, prose only: plan names HooksMap.subagent_stop) |
| Spec | 2 | 4e5c412acfa49fdd6f0f43625e21fd7c3ebc035a:f613369477c59a5fef3b3b56f9085d05eb440ff2 | CLEAN (1 MINOR, fixed: FR-017 3:1 vs white and black) |
| Tasks | 1 | 9a5fcef479387b7f6b0609e566bbd79d671c39aa:14028ac91c0fbb74f7f5b7af762b1d64d48359b8 | CHANGES: 1 MAJOR (T006 [P] on a shared file), 2 MINOR — all fixed, prose only (task markers, Verify line, checklist tick); review done |
| M1 A (code-review high) | 1 | not taken (tree before 3a651085; HEAD 6a9d2397 + T010/T020 work) | 10 findings: F2 spinner-lifted turn never started the clock (MAJOR, fixed + test); F1, F3–F10 declined (below) |
| M1 A (code-review, sonnet, fix diff) | 2 | 6a9d2397..b67fb7b7 | CLEAN after triage: 7 findings, none holds as BLOCKER/MAJOR (declined below) |
| M1 B (conformance, sonnet) | 1 | 89404f970ba2ec63f2c289d34bcb7bd0b433a7ef:b67fb7b7987edfecaa9cb8ac3db962a33c9d5acf | CHANGES: 1 MAJOR (no red runs for cycles 4–7): fixed with stub red runs logged in cycle-log; MINOR (010 hooks.md edit, required by T015) noted. Verify output all green. Fix touches only the TDD log: no re-run |
| D4 amendment (spec+plan+tasks) | 1 | 4bd5df0c073ae15298790294ebe96cd38024d535:ec36c53871c732e3d4854660c23eee980c818a81 | CHANGES: 1 MAJOR (T066 must reword M1's "a minute or more" in settings.md:157), 2 MINOR (plan supporting text; name `effective_long_task_threshold()`) — all fixed, prose only (task/plan wording); review done |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A1 | F1 activity.rs:96 turn resumed after a permission with only PostToolUse ends silently | data-model "Mapping" sets PostToolUse → no change and US1.7 defines the resume by PreToolUse; the FSM (010) is unchanged by 613. Recorded under Follow-ups |
| M1 | A1 | F3 state.rs fallback to TurnFinished | contract C3 names that fallback |
| M1 | A1 | F4 attention.rs pending grows with unclaimed events | bounded by events per session, pruned by any grant or forget; MINOR |
| M1 | A1 | F5 per-kind off then on delivers a pending event | per-kind switches and C15 are M3 (T033, T038–T040) |
| M1 | A1 | F6 serde defaults beside `default_on()` | MINOR, taste; defaults covered by core settings tests |
| M1 | A1 | F7 SessionError never produced | M2 (T021–T031) |
| M1 | A1 | F8 `pub set_long_task_threshold` | plan C2 test seam; D4=B turns it into a setting (M3) |
| M1 | A1 | F9 docs describe kinds without switches | text describes behaviour, names no switch; switches and Session error documented by T045/T031 |
| M1 | A1 | F10 test re-implements description formatting | MINOR |
| M1 | A2 | F1 a startup spinner starts the clock | every CLI's prompt event (UserPromptSubmit, Copilot `user.message`, Pi `turn_start`) is PromptSubmitted and resets `since`; timing from a spinner applies only when no prompt was seen (restart mid-turn), the case the fix is for |
| M1 | A2 | F2 `turn_change(SpinnerObserved, true)` is constant | kept: one mapping source (data-model "Mapping"); MINOR |
| M1 | A2 | F3 attention_support duplicates attention_claims harness | MINOR; attention_claims left as 039 wrote it |
| M1 | A2 | F4 short-turn tests weak / 200 ms margin | MINOR; same margin as attention_claims' US1.1 test, the turn is two in-memory calls |
| M1 | A2 | F5 Copilot multi-round `assistant.turn_end` | the CLI mapping is 010's, checked against the captured fixture (`a_whole_captured_turn_ends_awaiting_input`); 613 changes no CLI mapping |
| M1 | A2 | F6 spinner test has no short counterpart; F7 fmt churn in the diff | MINOR |

## Handover

M2 unit hit the 150k context cap after implementing. **Done**: T021–T031 implemented and ticked
in dc6ac923 (protocol 31 `SessionErrorNotice`; `ActivityEvent::Ended { error }`;
`Views::error_notice_target`; `DaemonState::error_notice` called from the GiveUp arm and from
`note_activity`; client `State::session_error_notice(_ification)` + daemon_sync arm; user guide
settings.md). TDD cycle 9 logged with red and green runs; targeted Verify tests green (only the
known root-only permission tests fail locally). **Next**: scoped gate detached with review A
(`code-review` high on origin/main...HEAD) in its shadow; then review B (conformance, sonnet);
no visual pass (no GUI change, only a desktop notification through the existing path); full gate
(raw mise.toml `gate` commands, CARGO_INCREMENTAL=0); push to #618; write
$SCRATCHPAD/pr-613-body-M2.md. **Open point for review**: C7 is applied literally, so a Copilot
session whose CLI reported an error and later crash-loops to give-up gets a second notice. Not
pushed: no green full gate yet.

## Open escalation

None.

## Follow-ups not done

- M1 review A F1: Claude Code turn that, after a granted permission, ends with PostToolUse then Stop (no further PreToolUse) stays `AwaitingInput` from the Notification, so its end is no attention event (010 FSM; pre-dates 613). Candidate: PostToolUse after AwaitingInput → Working.
