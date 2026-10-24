# Autopilot ledger — #943 structured-commands-fallback

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 943 — issue #943 "Structured session: slash commands, model switching and PTY fallback" (epic #945); labels: enhancement, flow:feature (in-progress added at start)
- **Kind**: feature
- **Effort**: default
- **Issue**: #943
- **Worktree branch**: claude/project-thread-k392yq
- **Started**: 2026-10-22
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1237 | design (spec, plan, tasks) | merged (rebase) | f27e1182 |
| #1239 | M1 | merged (rebase) | 8304c937 |
| #1241 | M2 | merged (rebase) | 6b3b62c7 |
| #1244 | M3 | merged (rebase) | 399f6be3 |
| #1250 | M4 | merged (rebase) | 0b432628 |
| #1252 | M5 | merged (rebase) | e479ea8f |
| #1257 | M6 | merged (rebase) | 319e8706 |
| #1265 | M7 | merged (rebase) | f462ddf5 |
| #1313 | M8 | merged (rebase) | 3c7db2eb |
| #1321 | M9 | merged (rebase) | 4910aeee |
| (to open) | close: quickstart results, tdd-verify, spec Closed | open (branch `claude/project-thread-k392yq`) | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | Core: commands, terminal-only, palette, model, control vs recorded fixtures | #1239 | merged |
| M2 | T019–T024 | full | Core transcript: chips, command output, compact/clear, model and terminal markers. **Needs #941 M1** (met) | #1241 | merged |
| M3 | T025–T031 | full | Daemon: command list at spawn; `/model` and terminal-only refused | #1244 | merged |
| M4 | T032–T038 | full | Daemon: model choice persisted, `--model`, live `set_model` | #1250 | merged |
| M5 | T039–T042 | full | Daemon: resume the post-`/clear` conversation (decision 2: A, confirmed) | #1252 | merged |
| M6 | T043–T049 | full | Daemon: switch kind. **Needs #941 M2** (met) | #1257 | merged |
| M7 | T050–T054 | full | Client render-free palette, picker, switch offer. **Needs #941 M4** (met) | #1265 | merged |
| M8 | T055–T060 | full | Window palette, picker, markers, guide. **Needs #941 M6** (met, e65993bc) | #1313 | merged |
| M9 | T061–T065 | full | Window switching, offers, terminal-only table. **Needs #942 M7 (PromptCard) / M8 (SC-007)** (met) | #1321 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify r1 | Terminal-period turns shown in the transcript? (US5 s2) | Only marked, not rebuilt from the CLI store; FR-023 updated | agent | 938 spec US7; 582 Out of Scope |
| 2 | clarify r1 | After `/clear`, which conversation do later launches resume? (US1 s5) | **A** (confirmed by jaro 2026-10-22, no longer provisional): structured sessions follow the CLI (FR-005a); terminal keeps BUG-904 trade-off. M5 stands as merged | user (jaro, 2026-10-22) | BUG-904 decision 1; 582 FR-017 |
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
| Full gate M6 run 2 | — | af86e20c | green, after rebase onto origin/main 2dddbef6 (no conflicts, `PROTOCOL_VERSION` still 66 there → 67) |
| Review A M7 (code-review high, claude -p) | 1 | 680a6287dbd32baea23edcbb6743a94f45d274a1:f69e3ab27d5d0953fac3b519cfac593e5a2eff1c | CHANGES: F1 MAJOR `/login` offered for ever after one auth failure — fixed (latest answer decides, test); F2 draft lost on a confirmation or refused switch — fixed (draft used up only on the switch's `OperationOk`, `switch_completed`, tests); F3 `/model <name>` draft lost when not connected — fixed (cleared only once sent); F4 connect-path reconcile left kind changes queued — fixed (`follow_kind_changes` there too); scoped check red: `terminal_size_reporting` built `session::State` with `..Default` over a private field — fixed (`kind_changes` pub) |
| Review A M7 (scoped, sonnet, claude -p) | 2 | e54e773ad3b63812f8aa40d85774109a6b42e1c4:60e69c6bc6f588c804c40a0973558addfcdfb039 | CLEAN |
| Review B M7 (conformance, sonnet, claude -p) | 1 | e54e773ad3b63812f8aa40d85774109a6b42e1c4:60e69c6bc6f588c804c40a0973558addfcdfb039 | CHANGES: F1 MAJOR a click on a terminal-only row reopened the palette beside the offer — fixed (pick dismisses over the draft; red test); F2 MINOR T052/T053 red evidence thin — fixed (mutation recorded). Verify: `features_command_palette` 14, `features_session` 69 passed |
| Scoped check M7 run 2 | — | 60e69c6b | red: `feature_write_isolation` (unclassified methods; `command_palette` writing `composers`) — fixed: the glue moved to `features/composer.rs`, palette state and switch offer held in `composers` |
| Review B M7 (scoped, sonnet, claude -p) | 2 | c421317b74053b04c2d9df1f5163cf8d58f430b3:0e7b9b19e14e0569a4a849fa0490b69514ca2f02 | CLEAN |
| Full gate M7 run 1 | — | 0e7b9b19 | red: `micold-daemon attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice` (no daemon change in M7; passed 5 of 5 reruns alone — timing flake under the gate's load) |
| Full gate M7 run 2 | — | 0e7b9b19 | green (split-gate.sh, ALL_GREEN) |
| Full gate M7 run 3 | — | 972fd90d | green (split-gate.sh, ALL_GREEN) after rebase onto origin/main b7f1b704 (#942 M6; conflicts in `app.rs` and `daemon_sync.rs` PendingOp arms resolved keeping both; no wire change in M7) |
| Full gate M7 run 4 | — | 32b8657f | green (split-gate.sh) after rebase onto origin/main 79db5768 (#944 M6; no conflicts) |
| T060 visual pass (live claude 2.1.310, Xvfb + tiny-skia) | — | a2a7cd87 | PASS B1, B2, B4 (restart-memory check not run), B5 except the picker's typed name: FAIL, its field took no keys (`KeyboardElsewhere`, `terminal_focused()` true once the composer blurred) — fixed at f3eb98e6 and re-checked live in dark (typed name → *Model 'no-such-model' not found*). Evidence and findings: `visual-pass/m8-record.md` (31 crops, both themes) |
| Review of the T060 fix (fresh context, sonnet, claude -p) | 1 | f3eb98e6 | CLEAN of BLOCKER/MAJOR; 5 MINOR: keep the derived term (agreed); a `model_picker_open()` helper (declined, one use); a picker left open across a session switch holds neither terminal nor field focus until a click (pre-existing, follow-up); another-session test case and a `terminal_focus.rs` case (declined: `composer_fits` covers the term and its release); the focus contract does not enumerate later terms (nothing to add) |
| Full gate M8 run 2 | — | 46b3dc76 | `scripts/gate.sh` ran out of disk while linking tests (fmt and clippy had passed); `target-shared/debug` wiped, then `split-gate.sh`: green (ALL_GREEN) |
| Review A M9 (code-review high, claude -p) | 1 | 93ada85ac8d9e844fa3deb61df00368047c27a1d:98efa87d6743299fed0e836d90632d74e9954087 | CHANGES (8, unranked): F1 the /login card held the palette's slot — fixed (offer → palette → login card); F2 the login card replaced another open offer — fixed (not over an open offer); F3 confirm/dismiss ignored their session — fixed (`switch_offer_is`, test); F5 the question's wording in two places — fixed (`switch_question`); F4, F6, F7, F8 declined |
| Review B M9 (conformance, sonnet, claude -p) | 1 | 93ada85ac8d9e844fa3deb61df00368047c27a1d:98efa87d6743299fed0e836d90632d74e9954087 | CLEAN (1 MINOR: T063/T064 red by mutation, disclosed); Verify: `terminal_only_doc` 2 passed; `check-user-guide-updated.sh` ok (9 user-facing paths, 1 page) with `CHANGE_TITLE` set |
| T065 visual pass (live claude 2.1.310, Xvfb + tiny-skia) | — | 9e21bda4 | PASS B3 (both commands to their first permission request, SC-007), B6 (offer, terminal with the held-queue line, back with the marker and the queue on hold, draft kept, same conversation), B7 (confirmation names the work; two windows follow); FAIL → fixed: a switch's command went out without its `/` and the daemon refused it (9e21bda4); after the fix `/mcp` was not typed because Claude Code asked to trust the scratch folder (938's first-prompt rule) — not seen typed. NOT RUN: B8 Copilot (no CLI; unit tests), B8 sandbox (no container runtime), B9 (Linux only), the `/login` card (needs a signed-out CLI). `visual-pass/m9-record.md`, 16 crops |
| Review A M9 (scoped, sonnet, claude -p) | 2 | b135d83a39982b5f3ad479473826238fdfb3b04c:751150ca9977f7708fea767b9fd1c09919e52359 | CLEAN (F1–F3, F5 fixes hold; F4, F6–F8 declines stand; the `wire_command` fix checked) |
| Full gate M9 run 1 | — | 751150ca + fmt | `split-gate.sh` (disk): fmt, clippy, core and daemon green; client chunk 160 red: `root_is_routing_only` (the root reducer's new `SwitchDismissed` arm decided) — moved into the feature (f38301f3) |
| Full gate M9 run 2 | — | f38301f3 | `split-gate.sh`: green (ALL_GREEN) |
| speckit-converge (close, claude -p) | 1 | 4910aeee | NONE: no unbuilt behaviour; only T066 open (done in the close PR). Run because M8 has no review B row |
| speckit-tdd-verify (close, sonnet, claude -p) | 1 | 4910aeee | FAIL: F1 (T063/T064 red only by mutation), F2–F3 HIGH, F4–F9 MED, F10–F11 LOW; 10 core mutants, 9 caught, 1 equivalent (F10). F3 fixed (the SC-002 helper counts the names whose unique prefix it checked and fails at none); the rest declined or follow-ups (below). `tdd/verification.md` |
| Close review (fresh, sonnet, claude -p) | 1 | 27a69438e9f442555728def0f83cc56886248014:4910aeee90b59dd49d50e93a89c7713366196eb5 | CHANGES: F1 MAJOR the first F3 fix (an else-branch) restated its own condition — replaced by a count of names checked; F2/F3 MAJOR the quickstart row and ledger claimed that fix — reworded |
| Close review (scoped, sonnet, claude -p) | 2 | f35757fab971788190dfbd93809766d4c1a68b05:4910aeee90b59dd49d50e93a89c7713366196eb5 | CLEAN (2 MINOR: one checked name passes the count guard — left; commit the edits together — done) |
| Full gate close run 1 | — | f35757fa | `split-gate.sh` (disk, `target-shared/debug` wiped first): green (ALL_GREEN) |

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
- Review A M9 F4 (the terminal's tree path differs between a single pane, several panes and a transcript): declined — those switches already swap the whole body (a transcript for a terminal, `panes::view` for one pane); the strip keeps the terminal's place within the single-pane terminal branch, which is what a state change there needs.
- Review A M9 F6 (`view_full`'s eighth argument and `too_many_arguments`): MINOR, taste; left.
- Review A M9 F7 (`SwitchDismissed` handled by the lib reducer and the shell): declined — the registry closes a dialog through the lib reducer (another dialog opening, `close_dialogs`), the shell receives the window's own presses; both are needed, as for #942's `ModeCancelled`.
- Review A M9 F8 (the table parser splits on every `|`): MINOR; a reason with a `|` fails the test loudly, not silently; left.
- tdd-verify F1 (T063/T064 have no recorded red): disclosed in M9's PR and `cycle-log.md`; the red was shown by mutation after green and cannot be recorded first after the fact.
- tdd-verify F2 (`MODEL_SHORTCUTS`, `CHECKED_AGAINST` asserted equal to literals): declined — both mirror facts of the external CLI; the pin makes a change to them a reviewed one, and `terminal_only_doc` checks the version against the guide.
- tdd-verify F4 (no `tdd/test-list.md`): process artefact this flow never used; `cycle-log.md` holds the traces.
- tdd-verify F5 (SC-007 has no automated test): SC-007 is a manual check by the spec's GUI exception, passed live in T065; T066 done in this PR.
- tdd-verify F8 (terminal-period marker assertions removed in 65a62038): false — the marker moved to `features::session::terminal_period_marker`, asserted by `features_session` `the_terminal_period_marker_says_where_the_turns_are`.
- tdd-verify F9 (serde test round-trips a test-local struct): the real record is covered by `session_store` `a_named_model_round_trips_and_default_is_not_written` and `a_record_written_before_model_choices_loads_default`.
- tdd-verify F11 (`len() == 70` tied to the fixture): declined — the fixture is recorded and fixed; the count is the check that parsing drops nothing.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M8 T060: before a structured session's first turn its skills are grouped as **Custom commands** (the CLI sends `init` with `skills` only after the first message; `CommandList::from_history`).
- M8 T060: after `/compact` a second **Command output** card repeats the earlier `/context` output; a model pick during a turn leaves a "Set model to …" **Command output** card beside the marker; the collapsed **Command output** preview shows markdown's `##`.
- M8 T060: newer CLI `system` events (hook started/response, `post_turn_summary`, `active_goal`, `autocompact_state`, `rate_limit_event`) draw as *Raw event* rows (938/940's mapping).
- M8 T060 fix review: a model picker left open across a switch of the displayed session holds neither the terminal's nor the field's keys until a click; close the picker on a session switch.

- `micold-daemon --test attention_error_notice` `without_focus_the_first_reporting_window_gets_the_notice` is flaky (1 in 5 in isolation, M3 gate run 3); not this feature's, not touched by its diff.
- Overlapping model picks (Review A M4 F2/F3, Review B M4 F1, all MINOR): a stale idle-pick note can take the deferred slot; a flushed-but-unanswered deferred pick's `previous` is inherited by the next busy pick; an idle pick does not clear a still-unflushed deferred one. Correlate the deferred slot with its `set_model` request id when M5/M6 touches `pick_model`.
- `attention_error_notice` flake hit again in M4's full gate run 1 (passed on rerun), and again in M8's full gate (passed alone on rerun).
- Review A M6 F7: a `SessionStop` that arrives between a switch's stop and its start is undone by the start; hold the session gate across both (split `start_session`'s gated part) if it matters.
- Review A M6 round 2 MINORs: a switch's delayed command task is not tied to the process it was for; the switch-ends-before-the-command ordering and the `SwitchEnding` guard have no test.
- M9 T065: the command of a switch is not typed when Claude Code first asks to trust the folder (938's first-prompt rule); the daemon logs why, the window says nothing.
- M9 T065 gaps: `/mcp` typed into the terminal not seen live (folder trust), B8's sandboxed switch (no container runtime here), B8's Copilot session (no `copilot`), B9 (macOS, Windows), the `/login` card (needs a signed-out CLI) — for a human pass.
- tdd-verify F6: fixed sleeps in the daemon tests' "nothing happened" checks (`structured_switch.rs:873,892,930`, `structured_commands.rs:363`, `structured_model.rs:374`); replace with an event or a barrier.
- tdd-verify F7: the SC-003 palette test (`command_palette.rs:234`) asserts 100 ms on the wall clock; can flake on a loaded runner.
- tdd-verify F10: the equal-name tier in `palette.rs:39` `filter` is dead (the equivalent mutant); fold it into the prefix tier.
