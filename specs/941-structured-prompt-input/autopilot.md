# Autopilot ledger — #941 structured-prompt-input

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 941 — issue #941 "Structured session view: prompt input, interrupt and queued messages"; labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #941
- **Worktree branch**: claude/project-thread-ik47pq
- **Started**: 2026-10-21
- **Phase**: M1
- **Next step**: M1 implemented (T001–T022 ticked); review A r2 (fix diff) + scoped gate, then review B, full gate, PR files

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1209 | Design: spec, clarifications, plan, research, contracts, tasks | merged (rebase) | 01c64cee744edf4036b2edd018dbd0677c015dba |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T022 | full | Core decisions (input, attachment, turn ids/asks, encoders, UserSent/InterruptAsked, transcript, saved queue), recorded fixtures, every wire type (one bump); daemon refuses new requests "Not available yet" | — | pending |
| M2 | T023–T033 | full | Daemon delivers, queues, input driver, StructuredInput broadcast, hold on exit/start, activity from the turn | — | pending |
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
| 3 | clarify r1 | After a restart, does queue delivery resume on its own? | No: queue on hold until one "Send queued" control | recommended default, pending user confirmation | ledger escalation r1; #938 restores sessions unattended |
| 4 | clarify r1 | What does an interrupt do to the queue? | Kept unchanged, on hold until "Send queued" | recommended default, pending user confirmation | queue shared by every window (Decision 1) |
| 5 | clarify r2 | Send while idle with the queue on hold? | Delivered at once, ahead of the held queue, which stays on hold | agent | spec.md#User Story 3 AS2, Clarifications Q2 |
| 2 | clarify r1 | Does the queue survive a service restart? | As the event history: saved under Save terminal history, memory only otherwise | agent | specs/938-structured-session-mode/spec.md#Clarifications Q3 |
| 6 | M1 implement | The plan assumed postcard control frames; they are JSON (`protocol::codec`). How do 12 MiB of pictures fit 16 MiB frames? | `ImageAttachment` bytes are base64 in JSON, plain bytes in postcard; `MAX_MESSAGE_IMAGE_BYTES` 12 → 9 MiB (base64 12 MiB + text). No requirement changes (the limit is plan-level); research R8, data-model, wire.md, plan, tasks T005/T019 corrected | agent | crates/micold-core/src/protocol/codec.rs "Control messages … are always JSON" |

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

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. (Clarify r1 Q1/Q2 are with the user as decision cards; the recommended defaults are applied
as Decisions 3–4 until they answer.)

## Follow-ups not done

- Cloud container: `gh` unauthenticated; no gh steps run in the spec unit (none needed: no PR). Reviewers ran via `claude -p --agent autopilot-reviewer` (no Agent tool).
