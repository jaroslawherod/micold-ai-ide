# Autopilot ledger — #941 structured-prompt-input

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 941 — issue #941 "Structured session view: prompt input, interrupt and queued messages"; labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #941
- **Worktree branch**: claude/project-thread-ik47pq
- **Started**: 2026-10-21
- **Phase**: M6
- **Next step**: M6 implement T055–T061

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1209 | Design: spec, clarifications, plan, research, contracts, tasks | merged (rebase) | 01c64cee744edf4036b2edd018dbd0677c015dba |
| #1223 | M1: core decisions, fixtures, wire types | merged (rebase) | cdfafa66da143c85003745205c980ee0736c97db |
| #1240 | M2: daemon delivers, queues, input driver, StructuredInput | merged (rebase) | 1781310b95438ce48d8006762ee0fc949f877343 |
| #1243 | M3: daemon interrupt, overdue, hold on interrupt, saved queue | merged (rebase) | 203ef720ba85b7e70a163cb969402b452c0d4b41 |
| #1246 | M4: client composer state, sends, drops | merged (rebase) | f114ea2ee0eb7e97c7e0ada228b26af6a870a383 |
| #1249 | M5: shared components TextArea limits, AttachmentChip, QueueRow, BusyIndicator | merged (rebase) | 74907e8f7c158ee992f119e223772f52628c09a8 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T022 | full | Core decisions (input, attachment, turn ids/asks, encoders, UserSent/InterruptAsked, transcript, saved queue), recorded fixtures, every wire type (one bump); daemon refuses new requests "Not available yet" | #1223 | merged |
| M2 | T023–T033 | full | Daemon delivers, queues, input driver, StructuredInput broadcast, hold on exit/start, activity from the turn | #1240 | merged |
| M3 | T034–T038 | full | Daemon interrupt, overdue, hold on interrupt; queue saved/restored under Save terminal history | #1243 | merged |
| M4 | T039–T047 | full | Client render-free composer state, sends, drops (no draw code); may merge beside M2–M3 | #1246 | merged |
| M5 | T048–T054 | full | Shared components TextArea limits, AttachmentChip, QueueRow, BusyIndicator; iced image feature | #1249 | merged |
| M6 | T055–T061 | full | Composer under the structured transcript, transcript lines, user guide, Part B | — | in progress |
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
| 11 | M3 implement | Reuse #942's control plumbing for the interrupt? | Yes, its encoder, ids and stdin mutex: `encode_interrupt(next_request_id())` written through the same `write_whole` under the stdin mutex `set_permission_mode` uses. `ControlWaiters` are not used (plan I-2): the answer is not awaited, the `result` ends the turn; its `control_response` resolves no waiter and is dropped there. The turn check, the `InterruptAsked` note and the write are made under the stdin mutex, so a request never reaches a later turn | agent | plan I-2; contracts/claude-input.md *Interrupt* |
| 12 | M3 implement | Where is the interrupt asked, and how is the 10 s tested? | Asked in the connection's handler under the state lock (arrival order: a spawned ask let an interrupt sent before a message land after it); the write and the overdue timer run on a spawned task (`input_driver::interrupt`). The bound is `INTERRUPT_OVERDUE`, shortened by `DaemonState::set_interrupt_overdue` in tests, as `set_details_bound` is (no paused clock: the stand-in is a real process) | agent | T035 "shortens or pauses the clock" |
| 13 | M3 implement | Which thread writes the saved queue? | A dedicated `queue-writer` thread started with the history store, fed by a channel: the history saver's task wakes on `SAVER_TICK`, which would delay a queue save, and tests would have to drive it. Snapshots are taken under the state lock with a global generation, only when `SessionInput::revision` changed and saving is on; the writer drops a generation older than one it wrote. A damaged file is replaced by the empty queue's save (removed) | agent | research R9; T038 |
| 14 | M3 verify | The interrupt-vs-message race of the M2 handover: real? | Real and wider than noted: `deliver` runs on `spawn_blocking` after the turn opens, so an interrupt asked in between was written to the idle CLI before its message (red test). `StructuredSession::written` (the last turn whose message was written, set under the stdin mutex) replaces the wait on the pending note: `write_interrupt` waits until its turn's message is out | agent | tdd cycle 7 |
| 10 | M2 implement | When does the activity follow the turn? | `TurnBegan` in the lock section that opens a turn; `TurnEnded` when the driver handles a `result` (or a failed write closes the turn) and the turn is idle in that same section, so a turn opened meanwhile is never shown idle; a process end leaves activity to supervision's `Ended` | agent | research R5, FR-011 |
| 15 | M4 implement | A send (or a `Taken` queued message) comes back while the user has typed a new draft: what does the composer hold? | The returned message goes back unchanged (text and pictures, FR-005/SC-005) with nothing typed lost: into an empty draft as it was; otherwise first, with the typed text after it on a new line, its pictures after the returned ones, and the caret kept in the typed text. The spec says only "unchanged"; displacing or dropping either would lose input | agent | spec.md FR-005, SC-005, US4 AS3 ("no draft is displaced") |
| 16 | M4 verify | The M4 handover left `composers.forget` unwired when a session closes: M4 or later? | M4: the composer state is M4's (T041–T047), and closing or removing a session already forgets its transcript in `app::State::forget_structured`; that now forgets the composer too and the shell drops its editor. Nothing drawn depends on it | agent | `shell/daemon_sync.rs` close/remove paths; test `closing_a_session_forgets_its_composer` |
| 17 | M5 implement | T049 says add iced's `image` feature; it turns on `image/default` (AVIF via rav1e, EXR, TIFF, a dozen formats): build cost and new crates nothing reads | Plan deviation: the workspace `iced` gains `image-without-codecs` and a workspace `image` dependency (0.25, no default features, `png`, `jpeg`, `gif`, `webp`) names the four codecs FR-027 accepts; Cargo unifies them into iced's decoder. Cargo.lock gains `gif`, `image-webp`, `kamadak-exif` (iced_graphics' image feature), `color_quant`, `mutate_once`; plan's "No new crate" is false either way | agent | research R8, FR-027; iced 0.14 Cargo.toml `image = ["image-without-codecs", "image/default"]` |
| 18 | M5 implement | How do the new components draw and take input? | `TextArea::max_height` is enforced by its chrome (laid out in the limit's room, node cut to it), not iced's `text_editor::max_height`, which stops at whole lines below the limit and adds its padding on top; `key_binding` replaces the default keys and `on_submit`. `BusyIndicator`'s indeterminate circular indicator is a ring of eight dots in quads turned through `Progress` (no canvas), idle keeps the ring's room so the words stay put. `QueueRow`'s actions are text buttons (each names itself and is a keyboard stop); `AttachmentChip` takes an `image::Handle` the caller makes once from the held bytes (a handle per frame is a new image to decode), its remove control a focusable icon button labelled `Remove <name>` by tooltip | agent | research R10 *Shared components*; Constitution VIII |
| 19 | M6 implement | The plan keeps the composer editors in the binary (`App.composer_editors`); the view sees only the lib `State` | Plan deviation: each `features::composer::Composer` holds its `app::Editor` (precedent `features/settings.rs`) and its pictures' `app::Preview` handles, rebuilt whenever the composer replaces the draft (`show_draft`) or a picture is removed, so the editor is never stale and no generation is needed; `State::editor` gives an empty one before a composer exists. The shell acts on `composers.editor_mut` | agent | handover M6 unit 1; `ui::terminal::transcript_body` draws from `&State` |
| 20 | M6 implement | Where do the composer's keys and the transcript's Escape come from? | `TextArea::key_binding` turns a focused press into a `KeyInput` and sends `Msg::Key` unless `composer_key` says `Edit`; `composing` is always false (winit delivers an IME composition as input-method events, not key presses). The transcript has focus when the window's keyboard is the pane's (`terminal_focused`) and the composer's editor did not capture the key: then `ui::subscription` listens for keys while the active session shows its transcript and is busy, and sends `Msg::TranscriptKey` | agent | FR-012; iced `keyboard::listen` skips captured events |
| 21 | M6 implement | The view cannot see whether the session's service is connected (the outbox is the shell's) | The composer shows *not running* from the phase; *not connected* is said when a send finds no connection (`returned` with `NOT_CONNECTED`), as M4's reducer already does. **Stop session** (FR-015) is `Msg::StopSessionRequested`, sent as `SessionStop` only while `offers_stop_session` | agent | FR-006, FR-015 |

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
| Review A M3 (code-review high) | 1 | d8da36895b5b8421117dd41244048463003010f1:3deb5c3b0611bd889e9c071452b7e17e263882fd | CHANGES: 1 MAJOR (written turn read after the write; fixed: `Delivery::turn`), 6 MINOR declined |
| Review A M3 (scoped, sonnet) | 2 | 9c9b872e65da00c2b1e90c75b1d26be4e3552a96:e5bbe8ae9b6a356903a0e6f6ae1b63b8a942cf53 | CLEAN (fix holds; no ordering counterexample; declines stand) |
| Review B M3 (conformance, sonnet) | 1 | b93566e90abb3dae86a4dcb4978023a93178aece:f346cb10855ed26bff69f4cfaaf8f5cb05b9d6e5 | CLEAN (Verify: structured_input 18 passed, structured_input_saved 5 passed) |
| Full gate M3 | — | — | green at f346cb10 (`scripts/gate.sh`, after rebase onto origin/main 6b3b62c7; no wire change in M3). Scoped gate as root fails only the root-only unreadable-file tests |
| Review A M4 (code-review high) | 1 | a6645123811153f70ca4ef415fdb1dc8847c539e:956e72ed7046e521afcf31de43eedcf96de89c11 | CHANGES: 1 MAJOR (session.rs doc comment split; fixed), 2 MINOR (late answer recreates a closed composer: fixed; capped image read: declined). Judged the composer forget on close right and M4's |
| Review A M4 (scoped, sonnet) | 2 | 07e58e0efa5d4f8ed8f0c23d1bb72e3975f39b5b:c13b30c5bbe342cfc32c3fab540b1b133312bd27 | CLEAN: 1 MINOR (forget_composer under another fn's orphan doc block; fixed) |
| Review B M4 (conformance, sonnet) | 1 | a6645123811153f70ca4ef415fdb1dc8847c539e:956e72ed7046e521afcf31de43eedcf96de89c11 | CHANGES: 1 MAJOR (no M4 cycle log / red evidence; fixed: Cycle 8 with a mutant set), 2 MINOR (shell editor removal untested: fixed; sync_editor on a forgotten session: declined). Verify: features_composer 29, pane_drop_target 5 passed. Judged the composer forget right and M4's |
| Review B M4 (conformance, sonnet) | 2 | eb42ba55d87556ba0162a8114c2cf52b3174ce42:6066b32c5054a4ca635f77998f2416d85e190c53 | CLEAN: 1 MINOR (cycle-log case count; fixed, prose only) |
| Full gate M4 | — | — | 1st red: clippy `large_enum_variant` on `composer::Msg::Dropped` (boxed); 2nd red: `root_vocabulary_is_cross_cutting` pin 26 → 27 (`Composer`); 3rd red: daemon `attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice` under load (no M4 change to the daemon; 5/5 green alone); green at 6066b32c, and at ea0ac0e8 after the rebase onto origin/main d16d74c1 (#1161 M1; no M4 wire change, no pin to move) |
| Review A M5 (code-review high) | 1 | f8f897dadf089779ffa9a29b5fc7acc5450d0825:86313219a4781610fe3a448bf9b94a08b9a9bc74 | CHANGES: 2 MAJOR (no explicit builder test in material_builder_api: fixed, `the_composer_parts_are_built_and_chained`; showcase limited area cannot scroll: fixed, live claim dropped and `a_scrolled_held_area_draws_other_lines_at_the_same_height`), 3 MINOR (T049 wording: fixed; reduce-motion, key_binding/on_submit, editor max_height, PhantomData, image dep: declined) |
| Review A M5 (scoped, sonnet) | 2 | 6c771212742f11c48eb8f63a78a4286ebebd2ce9:8e745e91a432afdd3a2f601874f796efdd0753d8 | CLEAN (fixes hold; declines stand) |
| Review B M5 (conformance, sonnet) | 1 | 5846601b6efdf6a3d667ce97a94e1e4496aa816e:9400acbe1e9bc55c16f122f2a08859413afb2685 | CLEAN: 2 MINOR (unit tests test-after: recorded in the cycle log, mutant set; plan/research still said `image` and "No new crate": fixed, prose only). Verify: layout_snapshot 83, material_builder_api 12, showcase_completeness 45 passed |
| Full gate M5 | — | — | rebased onto origin/main 420b051e (#1161 M2; no wire change, no pin to move). 1st red: daemon `attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice` under load (the M4 flake; review B's Verify build ran beside it; no M5 change to the daemon); green at 9400acbe (`scripts/gate.sh`). Visual pass not run: the change is showcase-only (nothing in the app draws the parts until M6); the geometry gate and contrast cases cover both schemes |

## Declined review findings

- Review B M4 r1 F3 (MINOR, `sync_editor` recreates a forgotten session's editor on a late answer): `forget_composer` drops that session's composer requests in flight, so no late answer reaches it; `editor()` needs `sync_editor` to create a fresh session's editor before it has a composer.
- Review A M4 r1 F3 (MINOR, drops read a whole picture before the size check): a read capped at the limit would make the TooLarge refusal name a wrong size; a drop is the user's own and is read off the UI thread.

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
| M3 | A r1 F2 (MINOR) | `write_interrupt` polls 1 ms with no bound | Every `Delivery` goes to `deliver`, whose write either records its turn or closes it, which ends the wait; a write blocked on the pipe is exactly what the interrupt must wait for |
| M3 | A r1 F3 (MINOR) | a failed interrupt write leaves `Asked` with no timer | The write fails only when stdin is gone or broken: the process's end follows, and its EOF ends the turn and the ask |
| M3 | A r1 F4 (MINOR) | a failed queue save is not retried | Same as the history store's one-shot event saves on a failed disk; the next queue change saves again; logged |
| M3 | A r1 F5 (MINOR) | a saved queue is dropped when input already exists | The queue is loaded with the event stream before the session can start or take a message (load precedes start); no path creates the input first |
| M3 | A r1 F6–F7 (MINOR) | snapshot clones under the lock; small per-session maps | Taste/efficiency: queues are a few messages; not fixed now |
- Review A M5 r1 F3 (MINOR, `BusyIndicator` ignores reduce-motion; copies `Bar`'s loop): `StageProgress`'s indeterminate bar, the precedent, ignores it too, and an indeterminate indicator that stops says nothing is happening; the loop is five lines, not worth a shared helper.
- Review A M5 r1 F4 (MINOR, `key_binding` silences `on_submit`; clamp the node instead of `TextEditor::max_height`): documented — `key_binding` replaces the default keys and `on_submit` by design (the composer's Enter is its own); `TextEditor::max_height` held 40 lines at 172 for a 200 limit (Decision 18).
- Review A M5 r1 F5 (MINOR, `image` dependency unused; `PhantomData<&'a ()>`): the dependency is how the four codecs reach iced's decoder (commented in both Cargo.toml files, Decision 17); the marker is `IconButton`'s pattern for a builder whose `From` names `'a`.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Flaky under a full gate's load (not M4's): daemon `attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice` saw no `SessionErrorNotice` once; 5/5 green alone.
- M6 (draw): focus the composer after a drop (M4 has no draw code; the reducer half is T042).
- Cloud container: `gh` unauthenticated; no gh steps run in the spec unit (none needed: no PR). Reviewers ran via `claude -p --agent autopilot-reviewer` (no Agent tool).
