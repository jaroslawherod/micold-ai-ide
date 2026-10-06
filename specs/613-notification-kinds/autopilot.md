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
- **Next step**: M4 — see Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #618 | whole run (design + milestones), one PR from claude/project-thread-8kdqkn | draft, open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T020 | full | Awaiting-input kinds, notified by their defaults (short turns silent, permission and long turns titled by kind; SubagentStop ignored) | #618 | done (pushed; full gate green at b67fb7b7) |
| M2 | T021–T031 | full | Session error notifications (give-up, Copilot session.error) to one window | #618 | done (pushed 4cfe5bcc; gate green but for the 6 root-only permission tests) |
| M3 | T032–T045, T056–T066 | full | Per-kind switches in Settings with icons; service stores and applies them | #618 | done (pushed; full gate 2 at 312578d0 green but for the 6 root-only permission tests; reviews A and B clean; visual pass §B5/B6/B8/B9 PASS) |
| M4 | T046–T052 | full | Kind icons in desktop notifications (Linux image-data, Windows/macOS PNG) | #618 | implemented (T046–T052 ticked); gate and reviews A/B next |

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

- M4: the I4 distinctness gate compares each glyph drawn *at* 16×16 (ink box spanning the square, FR-017's wording); at the notification tile's 62.5 % span (10 px glyph) error and task_alt differ in only 13/256 px. The tile render keeps 62.5 %. Review B F1 (MINOR) raised it; kept.

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
| M2 A (code-review high) | 1 | 39cd0b1ee3295d1f961bdf985a1671a9b4525567:a65a5f3b7d62e3d93e51040a9950f0a5656a6b25 | CHANGES: 1 MAJOR (open point decided: suppress the give-up notice after a reported error, FR-007) — fixed + test (cycle 10), C7 reworded |
| M2 A (code-review, sonnet, fix diff) | 2 | a469edb7b5007610f3c6c56b9364d75a39bceb16:a65a5f3b7d62e3d93e51040a9950f0a5656a6b25 | CLEAN |
| M2 B (conformance, sonnet) | 1 | a469edb7b5007610f3c6c56b9364d75a39bceb16:a65a5f3b7d62e3d93e51040a9950f0a5656a6b25 | CLEAN |
| M3 A (code-review high) | 1 | c094ddd08c97045143ec3db066a19d8628a04c3c:905e9a64353ae30472a8442952a047e35993e20c | CHANGES: F1 MAJOR (disabled kind row: glyph dimmed, label not, FR-017) fixed + test; F2 MINOR (threshold validated while master off) not fixed |
| M3 A (code-review, sonnet, fix diff) | 2 | 3a6676f37bcd08d58963710634afc8c00978f696:4d33c7d194f63c72d725f95c90e14591bcae45f4 (fix diff from c094ddd0:905e9a64) | CLEAN (1 MINOR fixed in 1102688f: doc comment back on its own test) |
| M3 B (conformance, sonnet) | 1 | 1101f063f429302e86ec24f655cda678dda09680:1102688f75d350cb72b6d6dc05307398bd8b1445 | CHANGES: F1 MAJOR (no red run for client/UI tests, cycle 12) fixed by retroactive mutation in the cycle log (2 survivors, visual-only); F2 MAJOR (no visual-pass evidence) fixed: pass run, found disabled checked box lost its mark, fixed + test; F3 MINOR (gate not recorded) recorded below |
| M3 B (conformance, sonnet, fix diff) | 2 | 2fb93c3c00eead68f9d66492ed7be7962b6bad79:8872c8f330605106182f3c19dbb6f744987183a6 (fix diff from 1101f063:1102688f) | CLEAN (3 MINOR: §B8 crops predate the disabled-checked fix; ledger gate result; Verify not run by reviewer — gate 2 ran it) |
| M4 A (code-review high, claude -p) | 1 | 91e5bd7cfaa7f8100f9a4993640b60a0c5b3719a:2cb52cc722e2ccc8c7a103bd044f58379dacece6 | CHANGES: F1 MAJOR (macOS UNNotificationAttachment moves the attached file, so the run's shared icon is gone after the first banner) fixed: per-banner copy in temp dir (`attachable_copy`) + 2 tests; F2 MINOR (`png` swallowed encode errors) fixed: returns io::Result; F3 MINOR (premultiply round trip) fixed: shared `draw`; F4 MINOR (render per Linux notification) not fixed |
| M4 B (conformance, sonnet, claude -p) | 1 | 91e5bd7cfaa7f8100f9a4993640b60a0c5b3719a:2cb52cc722e2ccc8c7a103bd044f58379dacece6 | CLEAN (F1 MINOR distinctness mask span: kept, see Decisions; F2 MINOR guide block detached the bullet list: moved; F3 MINOR = A F2, fixed). Reviewer could not run Verify (sandbox) |

## CI fixes

- 4969e821 CI red: `activity_pipeline::a_copilot_session_is_watched_by_its_event_log_and_scanned_for_spinners_like_any_other` (Unknown ≠ Working). Root cause, pre-existing race not an M2 change: `drain_signals` took the spinner edge before reading the title, while the PTY thread stores the spinner before the title; a title landing in between was reported without its spinner. Reproduced every run with a 30 ms probe between the two reads; fixed by reading the title first (state.rs). Daemon fmt/clippy/tests green but for the root-only tests.

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

- **Done**: M4 T046–T052 implemented and ticked (2cb52cc7); review A round 1 fixes (macOS per-banner copy, `png` Result, shared `draw`) and review B MINOR (guide block moved) in the next commit; Linux tests green before the fixes (30 desktop_notify bin, 11 notification_icon); mac/win clippy --all-targets clean before the fixes.
- **Incident**: the first full gate ran with the disk full; `scripts/tests/autopilot.test.sh` then ran its git fixtures in this worktree: it checked out local `main`, committed `e1fd3291 ledger` onto local `main` (was 98f41a61 = origin/main at creation), wrote junk files, and tried (failed) pushes to origin. Restored by `git stash push -u` (stash@{0}, junk only) and checking the branch out again; local `main` still points at e1fd3291 (moving it back was refused by the permission classifier) — the orchestrator/user should `git branch -f main 98f41a61` and drop stash@{0}. Disk: stale test binaries pruned (~5 GB free).
- **Next**: (1) run the test suites at the fixes: `cargo test -p micold-client --test notification_icon --bin micold-ai-ide desktop_notify`, and `cargo clippy -p micold-client --all-targets --target aarch64-apple-darwin -- -D warnings` (+ `x86_64-pc-windows-msvc`, target installed) for the macOS change; (2) review A round 2 (sonnet) on the fix diff from 91e5bd7c:2cb52cc7; (3) full gate (CARGO_INCREMENTAL=0; check `df -h /` first: >10 GB free, or prune `target-shared/debug/deps` — never let scripts/tests run on a full disk); (4) visual pass: §B2–B4/B7 need a notification server and live AI sessions this container lacks — at most look at the four rendered icons on white and black; (5) push, write $SCRATCHPAD/pr-613-body-M4.md, return `PR: #618`.

## Open escalation

None.

## Follow-ups not done

- M1 review A F1: Claude Code turn that, after a granted permission, ends with PostToolUse then Stop (no further PreToolUse) stays `AwaitingInput` from the Notification, so its end is no attention event (010 FSM; pre-dates 613). Candidate: PostToolUse after AwaitingInput → Working.
