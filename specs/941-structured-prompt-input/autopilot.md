# Autopilot ledger — #941 structured-prompt-input

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 941 — issue #941 "Structured session view: prompt input, interrupt and queued messages"; labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #941
- **Worktree branch**: claude/project-thread-ik47pq
- **Started**: 2026-10-21
- **Phase**: M2
- **Next step**: M2 PR to open (branch pushed; title/body in the session scratchpad)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1209 | Design: spec, clarifications, plan, research, contracts, tasks | merged (rebase) | 01c64cee744edf4036b2edd018dbd0677c015dba |
| #1223 | M1: core decisions, fixtures, wire types | merged (rebase) | cdfafa66da143c85003745205c980ee0736c97db |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T022 | full | Core decisions (input, attachment, turn ids/asks, encoders, UserSent/InterruptAsked, transcript, saved queue), recorded fixtures, every wire type (one bump); daemon refuses new requests "Not available yet" | #1223 | merged |
| M2 | T023–T033 | full | Daemon delivers, queues, input driver, StructuredInput broadcast, hold on exit/start, activity from the turn | — (to open) | gate green at 00df9bc2, pushed |
| M3 | T034–T038 | full | Daemon interrupt, overdue, hold on interrupt; queue saved/restored under Save terminal history | — | pending |
| M4 | T039–T047 | full | Client render-free composer state, sends, drops (no draw code); may merge beside M2–M3 | — | pending |
| M5 | T048–T054 | full | Shared components TextArea limits, AttachmentChip, QueueRow, BusyIndicator; iced image feature | — | pending |
| M6 | T055–T061 | full | Composer under the structured transcript, transcript lines, user guide, Part B | — | pending |
| M7 | T063 | light | Interrupt withdraws a pending #942 request (daemon test); waits on #942 M3 | — | pending |

T062 (Polish, no code) is left to the close unit; T063 is M7, waiting on #942 M3 (I-3).
M1 has 22 tasks (over rule 3's ~10) and is kept whole: it is no user story but the foundation every
story and every later milestone builds on, and the wire bump must be one (I-6); there is no
acceptance-scenario split. Milestones follow plan *Delivery* (layers), not one per story, because
each story needs core, daemon and window and no UI may be half-wired on `main` (rule 6): nothing
is visible before M6.

speckit-analyze (sonnet, fresh session): 3 HIGH, 6 MEDIUM, 5 LOW, all fixed in tasks.md. Plan fixed
(analyze I2): *Superseded assertions* now keeps the core `a_message_mid_turn_is_refused…` test (accept
still answers `TurnInProgress` internally) and moves `a_blank_message_is_refused…` to
`SessionInput`; data-model gained `InTurn.asks` (the ask epoch) and the feed message is `TurnClosed`.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify r1 | Is the queue the session's or each window's? | The session's, held by the service, shared by every window, kept when a window closes | agent | specs/938-structured-session-mode/spec.md#Edge Cases; spec.md#Constraints, #FR-009; constitution II |
| 3 | clarify r1 | After a restart, does queue delivery resume on its own? | No: queue on hold until one "Send queued" control | decided by user (jaro, 2026-10-21) | ledger escalation r1; #938 restores sessions unattended |
| 4 | clarify r1 | What does an interrupt do to the queue? | Kept unchanged, on hold until "Send queued" | decided by user (jaro, 2026-10-21) | queue shared by every window (Decision 1) |
| 5 | clarify r2 | Send while idle with the queue on hold? | Delivered at once, ahead of the held queue, which stays on hold | agent | spec.md#User Story 3 AS2, Clarifications Q2 |
| 2 | clarify r1 | Does the queue survive a service restart? | As the event history: saved under Save terminal history, memory only otherwise | agent | specs/938-structured-session-mode/spec.md#Clarifications Q3 |
| 6 | M1 implement | The plan assumed postcard control frames; they are JSON (`protocol::codec`). How do 12 MiB of pictures fit 16 MiB frames? | `ImageAttachment` bytes are base64 in JSON, plain bytes in postcard; `MAX_MESSAGE_IMAGE_BYTES` 12 → 9 MiB (base64 12 MiB + text). No requirement changes (the limit is plan-level); research R8, data-model, wire.md, plan, tasks T005/T019 corrected | agent | crates/micold-core/src/protocol/codec.rs "Control messages … are always JSON" |
| 7 | M2 implement | How does the driver learn a process ended, and tell a late end of an earlier process from the current one's? | The stdout reader reports its end too (`TurnClosed { by: Exit, turn: Option<TurnId>, process: serial }`); the driver holds the queue (`on_process_end`) only when the current primary is that process or none runs. `TurnClosed.turn` is `Option` (the plan's had none for an end between turns) | agent | plan R2/R6 name no exit signal; restart order: teardown → spawn → swap |
| 8 | M2 implement | `UserSent` must precede the turn's lines but not be shown for a failed write | A note slot (`Emit.sending`) under the lock the reader holds across ending a turn and passing the event on: set before the write, passed on by the reader before the next line or by the writer after a good write, dropped on a failed one; the lock is never held across the write (no pipe deadlock) | agent | research R3; P-3 |
| 9 | M2 implement | Reuse #942's control plumbing? | Not needed in M2: no control request is written (M3's interrupt uses `encode_interrupt`/`next_request_id`, and waits on the `result`, not `ControlWaiters`, per I-2). The input driver is spawned like #942's `spawn_mode_reports` (`server::spawn_input_driver`; tests spawn it) | agent | plan I-2 |
| 10 | M2 implement | When does the activity follow the turn? | `TurnBegan` in the lock section that opens a turn; `TurnEnded` when the driver handles a `result` (or a failed write closes the turn) and the turn is idle in that same section, so a turn opened meanwhile is never shown idle; a process end leaves activity to supervision's `Ended` | agent | research R5, FR-011 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 124c8bf01142dc504b1089c95d75b80d67980571:65cefbbe38d05d3054bce51ba4a2c4b3ab37a9e9 | CHANGES: 3 MAJOR, 2 MINOR (all fixed) |
| Review spec | 2 | 7c8c2f791befe95ba7718b63d4cde6c4a169439f:65cefbbe38d05d3054bce51ba4a2c4b3ab37a9e9 | CLEAN (sonnet) |
| Review plan | 1 | 5ed319d71185c5d5919a6e5458d96e8a71871984:125e33e3d05fbd606512f1185a0c45153db7f0bd | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Review plan | 2 | 5c526288a3d957a05bf42cfd41fc885f04683309:cc3ecef13d97dbe7bade5a8f198321d26f929441 | CLEAN (sonnet) |
| Review tasks | 1 | 98c81d7650a1aaa75d3488756d2a6b9f04034a02:0909b8aa08b8a364302b36b8fdb71a48509ffa94 | CHANGES: 2 MAJOR, 1 MINOR (all fixed); checklist item confirmed |
| Review tasks | 2 | 22c56cfa4d83ea22a4f99578a543af65950291e4:0909b8aa08b8a364302b36b8fdb71a48509ffa94 | CLEAN (design PR #1209 merged) |
| Review A M1 (code-review high) | 1 | b061491b31b1e8adfe7a819ba51179e7e88ac6bb:96a0defe7106c1484b328d9175dc173b7b440a05 | CHANGES: 2 MAJOR (Debug prints picture bytes; wire types outside the hashed source), 7 MINOR; fixed 2 MAJOR + 2 MINOR, 5 declined |
| Review A M1 (scoped, sonnet) | 2 | 5f610c5dfb06d3b71341f75aa694d4278dc805d0:ca17d07da3d3242ae0a73a894ff08cf80dbcf877 | CLEAN: 2 MINOR (capture temp file fixed; picture-name check declined) |
| Review B M1 (conformance, sonnet) | 1 | 2c163570b179fbb1922c81e07643980c76793802:ca17d07da3d3242ae0a73a894ff08cf80dbcf877 | CLEAN (Verify: daemon structured_session 16 passed; core green except the root-only permission test, which gate.sh runs unprivileged) |
| Review A M2 (code-review high) | 1 | 29e7b03bf3c25a809d773ea3cf7e34f6ae868450:37dd08531ecdfbd87a01762e2b401eacfd59af71 | CLEAN: 2 MINOR (both declined) |
| Review B M2 (conformance, sonnet) | 1 | c036df0cd4e6f0f20bded9f400e77d2d8016736a:f647684b709efa9bc318e217ab35f37fc8438d29 | CHANGES: 1 MAJOR (mid-turn case of the superseded #938 test raced: the second turn had no pause; fixed, it holds its result too) |
| Review B M2 (conformance, sonnet) | 2 | a3e725b524fc4ac20b51b32f9b8d9e8fa78c63e7:4ff69c045598722443a8f666d7b37f26db3dc7d2 | CLEAN (Verify 3×: structured_input 12 passed, structured_session 16 passed) |
| Full gate M2 | — | — | 1st red: `structured_input.rs` lacked its `// unix-only:` reason (fixed); 2nd red: `a_user_message_is_one_line…` dropped `UserSent` (seq 0) in the sender's `answer` under load — test sends from a non-viewing connection now (test-only); green after rebase onto origin/main (#943 M1, PROTOCOL_VERSION 64; no M2 wire change) |

## Declined review findings

- Review A M1 r1 F2 (an asked interrupt closes the turn `Interrupted` whatever the result): spec
  research R4 and T013 decide it ("a turn holding the `InterruptAsked` note whose `result` is
  `success` … also closes `Interrupted`").
- Review A M1 r1 F3 (per-process turn ids reset at 1, a stale interrupt could hit a new turn 1): not
  reachable in M1 (the daemon ignores `TurnInterrupt`); T008 keeps the per-process counter until M2
  moves ids to `SessionInput::next_turn`.
- Review A M1 r1 F4 (queue unbounded, 1 GiB file cap): research R9 "The queue has no length cap
  (the spec sets none)" and the 1 GiB read bound.
- Review A M1 r1 F8 (`submit` takes `busy`/`running` bools): data-model *SessionInput* signature;
  the daemon computes both under one lock (M2). MINOR.
- Review A M1 r1 F9 (blank check written in three places): the two daemon copies are M1's
  compile-only bridge that M2 removes (T008); MINOR.
- Review A M1 r2 F1 (picture name not checked for `/`, `\` or empty): MINOR; the name is display text only, never joined to a path (pictures are held in memory and in the saved queue file, not as files).
- Review A M2 r1 F1 (`Emit.sending` is one slot: a stdout line arriving while a write is in flight
  can pass the note on before a write that then fails): MINOR; Decision 8 — the reader passes the
  note on before the turn's next line by design, and a CLI writes no line for a turn it was never
  sent; a write that fails after a line arrived means the CLI died, and the process end follows.
- Review A M2 r1 F2 (one driver task serialises every session's delivery write, so a CLI that
  stops reading stdin stalls others): MINOR; research R2 specifies one driver handling reports in
  order, and a full 64 KiB pipe needs a CLI that has stopped reading mid-session; revisit with
  M3's overdue handling if it shows.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Cloud container: `gh` unauthenticated; no gh steps run in the spec unit (none needed: no PR). Reviewers ran via `claude -p --agent autopilot-reviewer` (no Agent tool).
