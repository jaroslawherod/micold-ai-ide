# Autopilot ledger — #943 structured-commands-fallback

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 943 — issue #943 "Structured session: slash commands, model switching and PTY fallback" (epic #945); labels: enhancement, flow:feature (in-progress added at start)
- **Kind**: feature
- **Effort**: default
- **Issue**: #943
- **Worktree branch**: claude/project-thread-k392yq
- **Started**: 2026-10-22
- **Phase**: M6 reviewed (A 2 rounds, B 1); full gate after rebase onto origin/main
- **Next step**: M6 (T043–T049): implement, gate, reviews, PR files. Cross-feature holds: M7 ← #941 M4, M8 ← #941 M6, M9 ← #942 M7/M8

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1237 | design (spec, plan, tasks) | merged (rebase) | f27e1182 |
| #1239 | M1 | merged (rebase) | 8304c937 |
| #1241 | M2 | merged (rebase) | 6b3b62c7 |
| #1244 | M3 | merged (rebase) | 399f6be3 |
| #1250 | M4 | merged (rebase) | 0b432628 |
| #1252 | M5 | merged (rebase) | e479ea8f |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | Core: commands, terminal-only, palette, model, control vs recorded fixtures | #1239 | merged |
| M2 | T019–T024 | full | Core transcript: chips, command output, compact/clear, model and terminal markers. **Needs #941 M1** (met) | #1241 | merged |
| M3 | T025–T031 | full | Daemon: command list at spawn; `/model` and terminal-only refused | #1244 | merged |
| M4 | T032–T038 | full | Daemon: model choice persisted, `--model`, live `set_model` | #1250 | merged |
| M5 | T039–T042 | full | Daemon: resume the post-`/clear` conversation — **provisional, decision 2** | #1252 | merged |
| M6 | T043–T049 | full | Daemon: switch kind. **Needs #941 M2** (met) | | in progress |
| M7 | T050–T054 | full | Client render-free palette, picker, switch offer. **Needs #941 M4** | | pending |
| M8 | T055–T060 | full | Window palette, picker, markers, guide. **Needs #941 M6** | | pending |
| M9 | T061–T065 | full | Window switching, offers, terminal-only table. **Needs #942 M7 (PromptCard) / M8 (SC-007)** | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify r1 | Terminal-period turns shown in the transcript? (US5 s2) | Only marked, not rebuilt from the CLI store; FR-023 updated | agent | 938 spec US7; 582 Out of Scope |
| 2 | clarify r1 | After `/clear`, which conversation do later launches resume? (US1 s5) | **Provisional A, pending jaro**: structured sessions follow the CLI (FR-005a); terminal keeps BUG-904 trade-off. Revert to B/C if he picks otherwise | user (provisional, via coordinator) | BUG-904 decision 1; 582 FR-017 |
| 3 | clarify r2 | Terminal launch after structured `/clear`; terminal `/clear` | Resume recorded handle; terminal `/clear` unseen | agent | 582 FR-017 |
| 4 | plan | FR-003: the CLI never writes a skill's expanded instructions to stdout; what goes behind the command? | The command's description; expansion only if a later CLI reports it; FR-003 and US1 s3 amended | agent | research.md R4, probe P-3 (2.1.307) |
| 5 | tasks | analyze I1/I2: does only M2's chip wait on #941 M1, and are `saved.rs`/`provider.rs` in the plan? | M2 is held whole on #941 M1; plan tree lists both files (plan.md updated) | agent | analyze report I1, I2 |
| 6 | implement M3 | T025/T030: stand-in answers `initialize` by default? Note appended by a task after `send_control`? | Stand-in answers only on an `initialize` directive (silent otherwise), so the 10 existing suites' exact event lists stay as they are; the stdout reader turns the answer into `CommandsReported` in the answer's place (deterministic order, raw answer not duplicated), and the request is written in `spawn` without waiting; `send_control` serves `set_permission_mode` and M4's `set_model` | agent | structured_* suites compare exact event lists (`events_of == fixture_events`) |
| 7 | implement M4 | T036 "invalid name → OperationError": a `ModelName` is checked on decode (`try_from`), so a frame with an invalid name never reaches the dispatch | Covered at the wire: `protocol_roundtrip` asserts such a `SetSessionModel` does not decode; no daemon-side refusal path exists to test | agent | M1 `ModelName` serde `try_from`; codec decodes control frames as JSON |
| 8 | implement M4 | T037: where does the busy pick wait, and how is its answer turned into a note? | The deferred choice lives in the `StructuredSession` (`defer_model`), written under the stdin mutex before the next user message or by the input driver's `turn_closed` after a `result` when idle; every `set_model` answer becomes a `ModelChanged` note in its place (a deferred refusal a `ModelRefused` note, an idle refusal only the requester's error); the state's `model_picks` keeps pick numbers (last wins) and the deferred pick's previous choice, put back on refusal via the `model_answers` feed; `PROTOCOL_VERSION` 66 | agent | data-model *PendingModel*; FR-015 "before any further user message" needs the write under the stdin mutex |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 67da24dc3a70beca179b57d029e7f152bc1a7ab6:dc6473291290b51830029c741e2fe6ab5630544e | CHANGES: 2 MAJOR, 3 MINOR — all fixed |
| Spec review | 2 (sonnet, fix diff) | 910550013b1158c635de912a8ee68881ce5de668:dc6473291290b51830029c741e2fe6ab5630544e | CLEAN (3 MINOR, fixed, prose only) |
| Plan review | 1 | bbe19f9619357fdbd70dce02a8e1225612cdd4e6:8915672b72d2ed2e420a78fd89db407984f5ce8f | CHANGES: 2 MAJOR, 2 MINOR — all fixed |
| Plan review | 2 (sonnet, fix diff) | cdb8aa85a308959029eb2b14aa15dfa62867367d:8915672b72d2ed2e420a78fd89db407984f5ce8f | CLEAN (1 MINOR, fixed, prose only) |
| speckit-analyze | 1 (sonnet) | — | 0 CRITICAL, 1 HIGH (C1), 4 MEDIUM, 5 LOW — C1, C2, G1, I1, I2, U1–U3 fixed (I1/I2 in plan.md); I3 left to jaro (decision 2), D1 informational |
| Tasks review | 1 | de7f53dab39c9aba91fd555cefc704e366ccdb3f:ee00a672e9c7ceefaa1edde46c12c37b7a0a103e | CHANGES: 1 MAJOR (Part B 3 / SC-007 moved M8 → M9), 3 MINOR — all fixed |
| Review A M1 (code-review high, claude -p) | 1 | b4e6c0e3cf0eba349e147e1a9bc80ace36b899ad:243190ac73d4e596d907aa24c2fa7b7d53e2a50c | CHANGES: 1 MAJOR (F1, declined: FR-009), 3 MINOR — F2, F4 fixed; F3 declined |
| Review B M1 (conformance, sonnet, claude -p) | 1 | 557f97e288a5a97b5363b42e3a56b2574406c567:cc1452181d4d44824e386a36d97cb82a8687e126 | CLEAN |
| Rebase review M1 (conflict resolution vs #941 M1, sonnet, claude -p) | 1 | daf7075aff3a43c20473739d35830934528dca79:40a159e175b97cc860d98471c79bd2068571abeb | CLEAN (2 MINOR, capture script: stray arg refused, interrupt id `micold-1` fixed — left) |
| Review A M2 (code-review high, claude -p) | 1 | befd7222c84571107707c3cbec98dd2188e17c53:eefffe5c2121cffe5fd1e92ea8ac54417a711184 | CHANGES (8, unranked): F1 synthetic error as output, F2 empty/stderr local-command lines, F3 compaction state, F4 switch mid-turn, F5 chip rewrote the typed text and took paths, F6 list rebuilt per message — fixed with tests; F7 rich arms untested — test added; F8 summary/description not behind a disclosure — declined |
| Review B M2 (conformance, sonnet, claude -p) | 1 | befd7222c84571107707c3cbec98dd2188e17c53:eefffe5c2121cffe5fd1e92ea8ac54417a711184 | CHANGES: F1 MAJOR T024 no red recorded — red observed by stubbing, logged; F2 MINOR stale counts — fixed |
| Review A M2 (scoped, sonnet, claude -p) | 2 | 14e2380da76348b1a29deee35ad3148d064bdc0d:a250b25ff50da64feb5e8bbc502918aaf4c20724 | CLEAN |
| Review B M2 (conformance, sonnet, claude -p) | 2 | 14e2380da76348b1a29deee35ad3148d064bdc0d:a250b25ff50da64feb5e8bbc502918aaf4c20724 | CLEAN; Verify: structured_transcript 71 passed, client lib transcript 7 passed |
| Tasks review | 2 (sonnet, fix diff) | 0c12e486473d3a8cc0937758fdd4473a3e424f23:ee00a672e9c7ceefaa1edde46c12c37b7a0a103e | CLEAN (1 MINOR, fixed, prose only) |
| Review A M3 (code-review high, claude -p) | 1 | 2105909dc24a3b40e9538ce502b9218cd6591a49:96f78747275b52e8fd55a3e2e2514b83d3e5d7ad | CHANGES: F1 MAJOR queue edit into `/model`/terminal-only reached the CLI — fixed (`edit_queue`, `QueueRefusal::Refused`, test); F2 MAJOR evicted note — declined (FR-009); F3 MINOR docs window before init — fixed, prose only; F4 MINOR history cloned per `/` message under the state lock — fixed (`EventStream::command_list`, test); F5 MINOR — declined |
| Review B M3 (conformance, sonnet, claude -p) | 1 | 2105909dc24a3b40e9538ce502b9218cd6591a49:96f78747275b52e8fd55a3e2e2514b83d3e5d7ad | CLEAN (1 MINOR: decisions rows 5/6 out of order — fixed) |
| Review A M3 (scoped, sonnet, claude -p) | 2 | 11d7edc6f025b98bcf13b1e3ff5818157a428ec4:1a67fe6b3749d9b242a82e3022438aa9a70db9a1 | CLEAN (1 MINOR: `iter` split `snapshot` from its doc — fixed); F1, F4 fixes hold, F2, F5 declines stand |
| Rebase review M3 (conflict resolution vs #941 M3, sonnet, claude -p) | 1 | 203ef720ba85b7e70a163cb969402b452c0d4b41:a4a9f93058c3628383b2401544c99c47b747d56a | CLEAN (3 MINOR: refusal before `Gone` in `change_queue_to`, queued `/x` not re-checked at delivery, failed initialize write only logged — left, none from the merge) |
| Review A M4 (code-review high, claude -p) | 1 | 2cdae125317a8dfe192b01ed66e048b47c557658:f3cef1e18cd81480526543bf7467c4e58e9d801f | CLEAN (3 MINOR: F1 `SpawnPlan` doc comment detached — fixed; F2 a stale idle-pick note can take the deferred slot, F3 a flushed-but-unanswered deferred pick's `previous` inherited by the next busy pick — left, edge cases of overlapping picks) |
| Review B M4 (conformance, sonnet, claude -p) | 1 | de30c437cac4be48233ba871b537a7524b9fc4eb:d5f63633b6dcdece9dd1c1272790e6b3b7e784b2 | CLEAN; Verify: structured_model 10, structured_session 19, session_store 44, protocol_roundtrip 22, schema_hash 10 passed (2 MINOR: an idle pick does not clear a still-unflushed deferred pick — left, same overlapping-picks family as A's F2/F3; decision 7 noted, no fix) |
| Rebase M4 onto origin/main f114ea2e (#941 M4) | — | — | clean rebase, no conflicts; full gate red once on `structured_model` `of_two_picks_the_last_one_wins` (test race: `answer(1)` skipped req 2's earlier answer) — fixed with `conn::answers` (both answers in either order), test-only; full gate green at 5081fdc1; rebased again onto origin/main 420b051e (#1161 M2, client settings only), full gate green at 010a8946 |

| Review A M5 (code-review high, claude -p) | 1 | 867788dfce74dede86870f3c81571eab6fae9772:7974aedb3455e248d3345bcefe9f870ddb4029e5 | CHANGES: F1 MAJOR superseded handle unknown after a second `/clear` — fixed (`set_conversation` marks it archived with the provider, test); F2 MAJOR report race — declined; F3 MINOR — :5120 wrong, :5075 commented; F4 MINOR archive marks only the id — fixed; F5–F8 MINOR — declined/left |
| Review B M5 (conformance, sonnet, claude -p) | 1 | 867788dfce74dede86870f3c81571eab6fae9772:7974aedb3455e248d3345bcefe9f870ddb4029e5 | CLEAN (1 MINOR: prune at state.rs:5075 asks about `id.0` — commented, same as A F3) |
| Review A M5 (scoped, sonnet, claude -p) | 2 | 3c4426a7072784195c6e004d47742eb1517c99e8:7974aedb3455e248d3345bcefe9f870ddb4029e5 | CLEAN (1 MINOR: F4 untested — test added, red with the handle marking disabled) |
| Review A M6 (code-review high, claude -p) | 1 | 80590d0f9c23e8957bda9f82aa3300084841e455:0f733011ca4071ab0e1728b7df887b204a52b1e3 | CHANGES (10 unranked): F1 `switching` stuck on a panic — fixed (drop guard `SwitchEnding`); F2 switch held through the command's readiness wait — fixed (switch ends once started, command typed on its own task); F3 already-terminal + command answered Ok untyped — fixed (`InvalidInput`, test, red with the guard off); F4 `permission_mode` doc — fixed, prose only; F5 misplaced doc — fixed; F8 `set_kind` copies `set_model` — fixed (`Catalog::change_session`, all three setters); F9 command-list lookup twice — fixed (`command_list_of`); F6, F7, F10 — declined/left |
| Review A M6 (scoped, sonnet, claude -p) | 2 | 76906799b8e13477c72e5e1fabc01cf0bc64c39e:7c8ba98e26bf692037db1647e8f9d3d81f38969a | CLEAN (2 MINOR: a delayed command task is not tied to the process it was for — left, rare and a structured run refuses a terminal-only command; F2's ordering and the drop guard untested — left) |
| Review B M6 (conformance, sonnet, claude -p) | 1 | 76906799b8e13477c72e5e1fabc01cf0bc64c39e:7c8ba98e26bf692037db1647e8f9d3d81f38969a | CLEAN |
| Full gate M6 run 1 | — | 7c8ba98e | red: `structured_switch` test race (late `initialize` line of the ended structured run read as terminal input) — fixed in the test's `typed` |

## Declined review findings

- Review A M1 F1 (MAJOR, command list and model lost when the history evicts `Init` and `CommandsReported`): declined — FR-009 says the list is the one reported at the latest start *whose events the session still holds*, and the fallback when they are no longer held; an `init` precedes every turn, so the latest one is recent.
- Review A M1 F3 (MINOR, a user skill named `login`/`config` is marked terminal-only): declined — FR-011: "a command is terminal-only if either source says so", by name; with only the `init` lists the kind of a name is not known.
- Review A M2 F8 (summary drawn inline, chip description and `era` unused in the window): declined — tasks.md T024 draws M2's arms with the existing pieces only; M8 T058 puts the command, compaction, clear and model markers in `DisclosureCard` (description and summary folded) and draws earlier eras as "before the clear"

- Review A M3 F2 (MAJOR, refusals read a list the history may have evicted): declined — same as Review A M1 F1: FR-009 makes the list the one the history still holds; an `init`, with the CLI's own terminal-only names, precedes every turn, and the documented terminal-only commands and `/model` are refused from the fallback too.
- Review A M3 F5 (MINOR, an empty `CommandsReported` hides an earlier non-empty one): declined — R1 as `CommandList::from_history` documents it: an empty latest note means that spawn's `initialize` was refused, and the latest `init`'s names count, not an older spawn's report.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M5 | A | F2 (MAJOR, discovery or a restart before the async `init` report is persisted) | discovery keeps the first session per location (FR-012), so a live or catalogued session's slot is never filled by its successor, and BUG-904's continuation check archives a `/clear` successor of an occupied slot; a restart inside the feed hop resumes the pre-clear conversation (option B's behaviour), nothing lost |
| M5 | A | F3 (MINOR, `id.0` at state.rs:5075/5120, client persist.rs) | :5120's closure already gets the resume handle from the catalog; :5075 prune: a session that followed `/clear` recorded its own conversation first (comment added); the client never sees the handle (not on the wire) |
| M5 | A | F5–F8 (MINOR) | F5 `set_conversation` keeps the own id as `None`; F6 persist under the lock is the catalog's pattern (`keep_permission_mode`, `set_model`); F7 duplication left to a later refactor; F8 the stand-in is one process per launch, so the flag is per launch |

- Review A M6 F6 (kind saved and broadcast before the old process stopped): R8 step 3 orders it so; the `switching` slot, not the kind, routes messages during the switch, and a model pick in that window is persisted either way (terminal branch keeps the choice).
- Review A M6 F7 (a `SessionStop` between the switch's stop and start is undone): left as a follow-up; holding the gate across both needs `start_session`'s internals split out, and a stop sent during a switch is rare.
- Review A M6 F10 (concurrency test counts starts, not stops; fixed sleeps in negative checks): MINOR, left — an extra stop without a start would show as a non-running session at the next iteration's start count.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- `micold-daemon --test attention_error_notice` `without_focus_the_first_reporting_window_gets_the_notice` is flaky (1 in 5 in isolation, M3 gate run 3); not this feature's, not touched by its diff.
- Overlapping model picks (Review A M4 F2/F3, Review B M4 F1, all MINOR): a stale idle-pick note can take the deferred slot; a flushed-but-unanswered deferred pick's `previous` is inherited by the next busy pick; an idle pick does not clear a still-unflushed deferred one. Correlate the deferred slot with its `set_model` request id when M5/M6 touches `pick_model`.
- `attention_error_notice` flake hit again in M4's full gate run 1 (passed on rerun).
