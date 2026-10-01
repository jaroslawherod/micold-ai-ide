---
feature: 037-explain-hidden-cli
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 20 # US1 AS1–AS7 with AS4a, US2 AS1–AS6, US3 AS1, AS1a, AS1b, AS2–AS4
planned_at: f490b395
updated_at: f490b395
suite_baseline: green # 4098 passed, 0 failed, 9 ignored, 374 binaries at f490b395
---

# Test List: Explain Why an AI CLI Is Not Offered

Derived from `spec.md` (the acceptance scenarios, FR-001 to FR-017, Edge Cases), `plan.md` (test
strategy), `research.md` R1–R9, `data-model.md` and the two contracts. The code was read only to
place behaviours and to find existing tests. Twelve behaviours are already held by an existing test
and are recorded `DONE` with it.

Trace ids: `US<n>-AS<m>` is acceptance scenario *m* of user story *n* in spec.md (`US1-AS4a`,
`US3-AS1a` and `US3-AS1b` are the lettered ones). `FR-0xx` and `SC-00x` are spec.md's. Contract row
ids are cited beside the spec trace for placement, never instead of it: `A1`–`A5`, `S1`–`S7` and
`C1`–`C4` are rows of `contracts/availability-answer.md`; `W1`–`W7` are sections of
`contracts/reason-wording.md`.

**Two series share the letter U, and one shares the letter A.** `contracts/reason-wording.md` W4
numbers its six surfaces U1–U6. This list always writes them as **surface U1** to **surface U6**.
A bare `U<n>` here, and every `[U<n>]` marker in tasks.md, is a behaviour id of this list. In the
same way a bare `A<n>` is an acceptance behaviour of this list, and the contract's wire rows are
written **contract A1** to **contract A5**.

## Outer loop: acceptance behaviors

**Entry point.** No end-to-end GUI runner exists (the profile's `acceptance` runner is the sandbox
suite, which has nothing to do with this feature). Two entry points are used, by who owns the
behaviour:

- **The client's `App`** in `crates/micold-client/src/main_tests.rs`, for the surfaces the client
  writes (surfaces U1, U2, U3 and U6). Each test feeds
  `DaemonMsg::AiCliAvailability { req, available, env }` through the shell arm (`update_inner`),
  as the existing `availability_*` tests do, and then reads what a surface would draw:
  `missing_cli_notice(app.core.session.availability.home())` for the Settings note, the
  notification queue for the missing-default message, `app.core.session.start_menu_note(dir)` and
  `offered_providers(Some(dir))` for a row's list, and the outbox for what was sent to the service.
- **The session service** in `crates/micold-daemon/tests/session_start.rs` and
  `tests/mcp_create_session.rs`, for the two surfaces the service writes (surfaces U4 and U5). The
  tests start a real `DaemonState` with the script fixtures those files have and read
  `WireLifecycle::Failed { reason, .. }`, which is the text the pane and the banner show unchanged
  (research R9), or the tool call's error.

What is actually rendered is the quickstart §B visual pass (T017, T027, T035, T036).

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| A1  | Host placement, home answered with Pi missing and `env: Some(IncludeOff)`: the note under **Default AI CLI** names Pi Coding Agent, says sessions get only the login PATH because "Source a script before each session" is off, and says to turn it on if the startup file puts it on the PATH, or install it on the login PATH | US1-AS1, FR-001, FR-006, SC-001 | example | PENDING | |
| A2  | From A1, a second home answer with every CLI available: the note is `None` and the home answer the selector is drawn from offers Pi, with no other message and no restart | US1-AS2, FR-013, SC-002 | example | PENDING | |
| A3  | Home answered with `env: Some(ScriptTimedOut)`: the note names the CLI, says the startup script timed out for your home directory so its PATH additions are not applied, and names "Script path" and "Timeout". It is the same string whatever `app.env_include_last_outcome` holds | US1-AS3, FR-004a, FR-007 | example | PENDING | |
| A4  | Home answered with `env: Some(ScriptFailed)`, then with `Some(ScriptNotFound)`: the note says the script exited with an error, then was not found, each for your home directory | US1-AS4, FR-001, FR-004a | example | PENDING | |
| A5  | Home answered with `env: Some(NoScriptPath)`: the note says no script is sourced because "Script path" is empty and says to set it. It does not say to turn anything on | US1-AS4a, FR-001 | example | PENDING | |
| A6  | Host placement, home answered with `env: Some(Applied)` and Pi missing: the note says Pi Coding Agent was not found on the PATH sessions get for your home directory (the login PATH plus what the startup script adds), and names both installing it and making the script add its directory | US1-AS5, FR-001, FR-002 | example | PENDING | |
| A7  | Home answered with every supported CLI available: no note, in any `env` state | US1-AS6, FR-011, SC-005 | example | PENDING | |
| A8  | Before any answer is filed the note is `None`. After an answer with a CLI missing and `env: Some(IncludeOff)` arrives, the note names the missing CLIs with that reason | US1-AS7, FR-011, SC-005 | example | PENDING | |
| A9  | A row whose directory's answer lacks the stored default with `env: Some(IncludeOff)`: the primary press posts one notification equal to `start_refusal(cli, IncludeOff, ThisComputer, .., Fresh)`, opens the list of available CLIs, sends no start to the service and leaves the stored default unchanged | US2-AS1, FR-008, FR-015 | example | PENDING | |
| A10 | Service: a `Resume` start of a session whose directory's script timed out fails with a reason that names the CLI, says the script timed out for that directory, says to fix the script or raise "Timeout" and then restart this session, and contains neither "install" nor "another AI CLI" | US2-AS2, FR-009, FR-004a | example | PENDING | |
| A11 | Service, host: a `Fresh` start in a directory whose script succeeded without the CLI fails with the `Applied` host reason (not found on the PATH sessions get for that directory), keeps the remedy (install it, or another AI CLI) and adds making the script add its directory | US2-AS3, FR-009 | example | PENDING | |
| A12 | Service, `MICOLD_IMAGE_REFERENCE` set, script succeeded without the CLI: the `Fresh` and `Resume` reasons are today's `missing_cli_reason` sentences byte for byte. Written before T023 and green against the untouched gate, then `BASELINE` | US2-AS4, FR-005 | characterization | PENDING | |
| A13 | Service, `MICOLD_IMAGE_REFERENCE` set, environment-include off: the reason is the `IncludeOff` image sentence, names the image as where sessions run, and contains neither "isn't in" nor "isn't installed" | US2-AS5, FR-005, FR-002, SC-003 | example | PENDING | |
| A14 | Service: `create_session` for a CLI the target directory's environment lacks is refused with `explain(&[cli], env, place, Dir(cwd))`'s `{reason} {action}`, leaves no session record, and with environment-include off does not contain "is not installed" | US2-AS6, FR-009a, FR-002 | example | PENDING | |
| A15 | A row whose own answer offers two CLIs, lacks Pi and carries `env: Some(ScriptFailed)`: with its list open, `start_menu_note(dir)` names Pi Coding Agent with that reason and action and names the row's directory, and the two CLIs are still offered | US3-AS1, FR-010, SC-007 | example | PENDING | |
| A16 | A row whose own answer offers one CLI: it offers no choice (no chevron) and `start_menu_note(dir)` is `None` | US3-AS1a, FR-010, SC-007 | example | PENDING | |
| A17 | A row whose own answer offers one CLI that is not the stored default: the primary press opens the list with A9's message, and `start_menu_note(dir)` is `None` | US3-AS1b, FR-010, FR-008 | example | PENDING | |
| A18 | A row whose own answer offers every supported CLI: with its list open, `start_menu_note(dir)` is `None` | US3-AS2, FR-011, SC-005 | example | PENDING | |
| A19 | Two rows whose answers differ: each row's note is its own directory's, and opening, answering and closing one row's list leaves the other row's note and offer as they were | US3-AS3, FR-012 | example | PENDING | |
| A20 | With the list open on a row that lacks Pi: the CLIs the list offers for that row do not include Pi although the note names it, and opening the list sent no start to the service | US3-AS4, FR-010, FR-015 | example | PENDING | |

Notes on the outer loop:

- A7, A16, A17 (its note half) and A18 assert an absence and may pass once they compile. T039 and
  T044 record a deliberate mutant as their red evidence.
- A12 is a characterization test: US2-AS4 is "unchanged from today". It must be green before T023
  touches the gate.
- A20 holds the half of US3-AS4 a cargo test reaches at the `App`: the list has no item for the
  missing CLI. "Pressing the note does nothing" is geometry, held by U89 in `menu_anatomy` and seen
  in quickstart §B B11.
- A16's chevron half is already held by U86 and by
  `main_tests.rs` `availability_a_project_without_the_script_offers_no_choice`.

## Inner loop: unit behaviors

### `crates/micold-core/src/cli_reason.rs`: `SpawnEnv::classify`, `script_applied`

Tests in `crates/micold-core/tests/cli_reason.rs` (new file).

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U1  | `enabled == false` gives `Some(IncludeOff)` whatever the path (blank, set, a path that names no file) and whatever the attempt (`None`, `Success`, `MissingScript`, `TimedOut`) | FR-001 row 1, Edge Cases off with a missing path (R2) | example | PENDING | |
| U2  | Enabled with `""` or `"   "` gives `Some(NoScriptPath)` whatever the attempt, `MissingScript` included: never `ScriptNotFound` and never `IncludeOff` | FR-001 row 2, Edge Cases blank path (R2) | example | PENDING | |
| U3  | Enabled, path set, attempt `MissingScript` gives `Some(ScriptNotFound)` | FR-001 row 3 (R2) | example | PENDING | |
| U4  | Attempt `NonZeroExit { .. }` gives `Some(ScriptFailed)` | FR-001 row 4 (R2) | example | PENDING | |
| U5  | Attempt `TimedOut { .. }` gives `Some(ScriptTimedOut)` | FR-001 row 5 (R2) | example | PENDING | |
| U6  | Attempt `Success` gives `Some(Applied)` | FR-001 row 6, Edge Cases succeeded but changed nothing (R2) | example | PENDING | |
| U7  | Enabled, path set, attempt `None` or `Disabled` gives `None`: the other side of U3–U6 | FR-011 (R2, R3) | example | PENDING | |
| U8  | `script_applied()` is true for `Applied` and false for each of the other five states | FR-002 | example | PENDING | |

### `crates/micold-core/src/cli_reason.rs`: `name_list`, `explain`

Tests in `crates/micold-core/tests/cli_reason.rs`. Every sentence is asserted as the exact string of
`contracts/reason-wording.md` W1 and W2.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U9  | `name_list` gives "A", "A and B", "A, B and C" in display names, and `None` for an empty slice | FR-003, FR-004 (W1) | example | PENDING | |
| U10 | `explain(&[], ..)` is `None` in every state and place | FR-011 (W2) | example | PENDING | |
| U11 | `IncludeOff` on this computer: the exact reason and action of W2's first row, with no directory | FR-001 row 1, US1-AS1 (W2) | example | PENDING | |
| U12 | `NoScriptPath` on this computer: the exact row, and the action does not contain "Turn it on" | FR-001 row 2, US1-AS4a (W2) | example | PENDING | |
| U13 | `ScriptNotFound` on this computer: the exact row, action `Correct "Script path".` | FR-001 row 3, US1-AS4 (W2) | example | PENDING | |
| U14 | `ScriptFailed` on this computer: the exact row, action `Fix the script named in "Script path".` | FR-001 row 4, US1-AS4 (W2) | example | PENDING | |
| U15 | `ScriptTimedOut` on this computer: the exact row, action naming "Script path" and "Timeout" | FR-001 row 5, US1-AS3 (W2) | example | PENDING | |
| U16 | `Applied` on this computer, one CLI: the exact row ("was not found on the PATH sessions get for {dir}", "Install it, or make the script add its directory.") | FR-001 row 6, US1-AS5 (W2) | example | PENDING | |
| U17 | In an image, the five states in which no script was applied use the lead `A session in {image} would not find {names}:`, `the image's PATH`, and `use an image that puts {them} on its PATH` where the host rows say install | FR-005, US2-AS5 (W1, W2) | example | PENDING | |
| U18 | `Applied` in an image: `{names} isn't in {image}.` and the obligation sentence. Joined by a space they are today's note byte for byte, and they name no directory | FR-005, FR-004a (W2d) | example | PENDING | |
| U19 | Two and three missing CLIs are named together in one reason, with "them", "were", "aren't" and "their directories" where one CLI gives "it", "was", "isn't" and "its directory" | FR-004, Edge Cases several missing (W1, W2) | example | PENDING | |
| U20 | Over all states and both places: where `!env.script_applied()` neither string contains `isn't installed`, `not installed`, `isn't in` or `aren't in`, and the action is never only an instruction to install. In `Applied` the reason may say not found: the other side | FR-002, SC-003 (W2a) | example | PENDING | |
| U21 | Over all states and both places: every double-quoted label in a sentence is `LABEL_ENABLED`, `LABEL_SCRIPT_PATH` or `LABEL_TIMEOUT` | FR-003 (W2b) | example | PENDING | |
| U22 | The directory appears in `ScriptNotFound`, `ScriptFailed` and `ScriptTimedOut` in both places and in `Applied` on this computer, and in no sentence of `IncludeOff`, `NoScriptPath` or `Applied` in an image | FR-004a (W2c) | example | PENDING | |
| U23 | `AttemptDir::Home` is written "your home directory" and `AttemptDir::Dir(p)` is written as `p` displayed | FR-004a (W1) | example | PENDING | |
| U24 | Over all states and both places: no sentence contains `.bashrc`, `.zshrc`, `.profile` or `$PROFILE` | FR-016 (W2e) | example | PENDING | |
| U25 | Over all states and both places: no sentence contains "see below" | FR-007 (W2f) | example | PENDING | |

### `crates/micold-core/src/protocol/`: the wire field

Tests in `crates/micold-core/tests/schema_hash.rs` and `tests/protocol_roundtrip.rs`.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U26 | `PROTOCOL_VERSION` is 18 and the pinned schema hash is the one of the message set with `env` | FR-012 (contract A1, A5) | example | PENDING | |
| U27 | `DaemonMsg::AiCliAvailability` round-trips with `env: Some(SpawnEnv::ScriptTimedOut)` and with `env: None`, and the two are distinct after the round trip | FR-012, FR-011 (contract A1) | example | PENDING | |

### `crates/micold-daemon/src/state.rs`, `server.rs`: what the service answers

Tests in `crates/micold-daemon/tests/ai_cli_availability.rs`, with the bash and PowerShell fixtures
the file has. In every row `available` stays what the file's existing tests assert (contract A2).

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U28 | Environment-include off: the answer carries `env: Some(IncludeOff)` | FR-001 row 1, FR-012 (S1) | example | PENDING | |
| U29 | On with a blank script path: `Some(NoScriptPath)` | FR-001 row 2 (S2) | example | PENDING | |
| U30 | On with a script path that names no file: `Some(ScriptNotFound)` | FR-001 row 3 (S3) | example | PENDING | |
| U31 | On with a script that exits 3: `Some(ScriptFailed)` | FR-001 row 4 (S4) | example | PENDING | |
| U32 | `cfg(unix)`: on with a script path that names a directory: `Some(ScriptFailed)`, not `ScriptNotFound` | FR-001 (the note on a path that cannot be sourced) (S4) | example | PENDING | |
| U33 | On with a script that sleeps past a 1 s timeout: `Some(ScriptTimedOut)` | FR-001 row 5 (S5) | example | PENDING | |
| U34 | On with a script that adds a directory holding a fake CLI: `Some(Applied)`, and the CLI is in `available` | FR-001 row 6, FR-012 (S6) | example | PENDING | |
| U35 | On with a script that succeeds and leaves the PATH alone: `Some(Applied)`, and the CLI is not in `available`: the other side of U34 | Edge Cases succeeded but changed nothing (S6) | example | PENDING | |
| U36 | No `cwd` and no home directory: `env` is `Some(IncludeOff)` or `Some(NoScriptPath)` for those settings, and `None` when the settings call for an attempt | FR-011 (S7) | example | PENDING | |
| U37 | A second answer for a directory does not run the script again: one run in the counting script's log | FR-014, SC-006 (contract A3) | example | DONE | `crates/micold-daemon/tests/ai_cli_availability.rs::a_second_answer_for_a_directory_does_not_run_the_script_again` |
| U38 | Reading `env` adds no run: two answers through `availability_in` carry the same state and the log still holds one run | FR-014, SC-006 (contract A3) | example | PENDING | (extends U37's test) |
| U39 | After `set_env_include` turns environment-include on, the next answer for the same directory carries the new state, with no restart of the service | FR-013 (contract A3) | example | PENDING | |

### `crates/micold-client/src/features/session.rs`, `shell/daemon_sync.rs`: the client's copy

Tests in `crates/micold-client/tests/directory_availability.rs`, and one in
`crates/micold-client/src/main_tests.rs` (U45).

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U40 | `answered` stamps `asked_for` with `Home` for a request that named no directory | FR-004a (C2) | example | PENDING | |
| U41 | `answered` stamps `asked_for` with `Dir(path)` for a request that named one | FR-004a (C2) | example | PENDING | |
| U42 | `for_dir` for a row without its own answer returns the home answer with `asked_for == Home`, and returns the row's own with `asked_for == Dir(path)` and its own `env` once that is filed | FR-004a, FR-012, Edge Cases row drawn from the home answer (C3) | example | PENDING | |
| U43 | A newer answer for a key replaces `env` together with `available` | FR-012, FR-013 (C2) | example | PENDING | |
| U44 | An answer older than the newest request for its key is dropped: `env` and `available` stay the newer answer's | FR-012 (C2) | example | PENDING | |
| U45 | `DaemonMsg::AiCliAvailability { env: Some(SpawnEnv::IncludeOff), .. }` through the shell arm files a `CliAvailability` whose `env` is that state and whose `source` is the boot plan's | FR-012, FR-013 (C1) | example | PENDING | |
| U46 | No new availability request is sent: every site that asks is one of the named events | FR-014 (C4) | example | DONE | `crates/micold-client/tests/availability_is_asked_only_on_named_events.rs::nothing_but_the_contracts_named_events_asks_for_availability` |

### `crates/micold-client/src/features/settings.rs`: `missing_cli_notice` (surfaces U1 and U2)

Tests in `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs`.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U47 | On this computer, in each of the six states, the note is `explain`'s `{reason} {action}` for the missing CLIs with `AttemptDir::Home` | FR-006, FR-001, SC-001 (W4) | example | PENDING | |
| U48 | With an image as the source, in each of the five states in which no script was applied, the note is `explain`'s for `Place::Image(reference)` and does not contain "isn't in" | FR-005, FR-002 (W4) | example | PENDING | |
| U49 | With an image as the source and the script applied, the note names the missing CLI, the image and the obligation, with "isn't" for one and "aren't" for several | FR-005 (W2d) | example | DONE | `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs::an_image_missing_one_names_it_the_image_and_the_obligation`, `::an_image_with_no_cli_at_all_names_every_one`, `::one_missing_cli_and_two_agree_with_their_verbs` |
| U50 | The note under *Image reference* is the same string as the note under **Default AI CLI** for one answer | FR-005 (W4 surface U2) | example | PENDING | |
| U51 | With no answer the note is `None` | FR-011, US1-AS7 (W5) | example | DONE | `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs::an_unanswered_service_says_nothing` |
| U52 | With nothing missing the note is `None` | FR-011, US1-AS6 (W5) | example | DONE | `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs::an_image_with_every_cli_says_nothing_either` |
| U53 | With a CLI missing and `env: None` the note is `None`: the other side of U47 | FR-011, FR-001 (W5) | example | PENDING | |
| U54 | With every supported CLI missing the note names all three in one sentence and gives the reason once | FR-004, Edge Cases nothing is available | example | PENDING | |
| U55 | Asking for the note leaves the availability answer equal to what it was (the function takes it by shared reference) | FR-015 | example | PENDING | |

### `crates/micold-core/src/cli_reason.rs`: `start_refusal`, `start_refusal_unknown`

Tests in `crates/micold-core/tests/cli_reason.rs`.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U56 | `Fresh`, every state and place except `Applied` in an image: `{reason} {action} Or start this session on another AI CLI.` | FR-008, FR-009 (W3) | example | PENDING | |
| U57 | `Resume`, the same rows: `{reason} {action} Then restart this session: its conversation can only continue in {name}.` | FR-009, US2-AS2 (W3) | example | PENDING | |
| U58 | `Applied` in an image: the `Fresh` and `Resume` forms are today's `missing_cli_reason` sentences byte for byte | FR-005, US2-AS4 (W3b) | example | PENDING | |
| U59 | The `Resume` form contains "another AI CLI" in no state and no place | FR-009 (W3a) | example | PENDING | |
| U60 | In `ScriptNotFound`, `ScriptFailed` and `ScriptTimedOut` no form contains "install". In `IncludeOff` and `NoScriptPath` installing is named beside another action, never alone: the other side | FR-009, SC-003, US2-AS2 (W3c, R7) | example | PENDING | |
| U61 | In every state and place except `Applied` in an image, both forms begin with `explain(&[cli], ..)`'s `{reason} {action}` | FR-012, SC-004 (W3d) | example | PENDING | |
| U62 | `start_refusal_unknown(cli)` is `{name} would not be found by a session here. Start this session on another AI CLI.` | FR-008, FR-002 (W5, R8) | example | PENDING | |
| U63 | U20's rule holds for both forms of `start_refusal` and for `start_refusal_unknown` | FR-002, SC-003 (W3e) | example | PENDING | |

### `crates/micold-daemon/src/state.rs`: the launch gate (surface U4)

Tests in `crates/micold-daemon/tests/session_start.rs`. A10–A13 are this component's outer
behaviours.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U64 | On the host with environment-include off, a `Fresh` start of a missing CLI fails with exactly `start_refusal(cli, IncludeOff, ThisComputer, Dir(cwd), Fresh)` | FR-009, FR-002 (W4) | example | PENDING | |
| U65 | With a script that exits with an error, a `Resume` start fails with exactly `start_refusal(cli, ScriptFailed, .., Dir(cwd), Resume)` | FR-009 (W4) | example | PENDING | |
| U66 | A failed start and an availability answer for the same directory name the same state | FR-012, SC-004 (contract A3) | example | PENDING | |
| U67 | A start refused for a missing CLI names it by its display name and not its command, spawns nothing and spends no restart attempt | FR-003, FR-015 | example | DONE | `crates/micold-daemon/tests/session_start.rs::starting_a_session_whose_cli_is_absent_reports_it_and_spends_no_restart_budget` |

### `crates/micold-client/src/catalog_sync.rs`: the banner (surface U4, client side)

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U68 | A failure's reason is shown as the service sent it and said once, however many snapshots carry it: it is not rewritten afterwards | FR-009, FR-012 (W6, R9) | example | DONE | `crates/micold-client/tests/start_failure_notice.rs::an_unchanged_failure_is_said_once_however_many_snapshots_carry_it` |

### `crates/micold-client/src/features/session.rs`: `start_menu_toggled` (surface U3)

Tests in `crates/micold-client/tests/unavailable_default_says_so.rs`, and U75 in
`tests/directory_availability.rs`.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U69 | In each of the six states, the press on a row whose stored default is missing posts exactly one notification equal to `start_refusal(cli, env, place, asked_for, Fresh)` | FR-008, FR-012, US3-AS1b (W4) | example | PENDING | |
| U70 | The primary press with a missing default publishes the list opening and no start | FR-008, US2-AS1 | example | DONE | `crates/micold-client/tests/session_start_press.rs::pressing_start_with_an_uninstalled_default_offers_the_choice_and_starts_nothing` |
| U71 | After that press the list is open and the stored default is the one the user stored | FR-008, FR-015 | example | PENDING | |
| U72 | With `env: None` on the answer in use, the notification is `start_refusal_unknown(cli)` | FR-008, FR-002 (W5, R8) | example | PENDING | |
| U73 | In an attempt state a row reading the home answer says "your home directory", and a row with its own answer names its own directory | FR-004a, Edge Cases row drawn from the home answer | example | PENDING | |
| U74 | With an image as the answer's source the notification is the image form of the sentence | FR-005, FR-008 | example | PENDING | |
| U75 | A newer answer filed after the notification posts no second one and leaves the first as it was said | FR-012, Edge Cases a message was shown and the state then changed (W6) | example | PENDING | |
| U76 | A press whose default the row's answer now offers posts nothing | FR-011 (W5) | example | DONE | `crates/micold-client/tests/unavailable_default_says_so.rs::a_default_that_turned_out_to_be_installed_says_nothing` |
| U77 | The chevron opens the same list and posts nothing | FR-008 (said only when a start is refused) | example | DONE | `crates/micold-client/tests/unavailable_default_says_so.rs::the_chevron_opens_the_same_list_and_says_nothing` |

### `crates/micold-client/src/features/session.rs`: `State::start_menu_note` (surface U6)

Tests in `crates/micold-client/tests/directory_availability.rs`.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U78 | An answer offering two of three CLIs with `env: Some(..)` gives `Some` of `explain`'s `{reason} {action}` for the missing one and the answer's `asked_for` | FR-010, US3-AS1, SC-007 (R6) | example | PENDING | |
| U79 | An answer offering all three gives `None` | FR-011, US3-AS2 (W5) | example | PENDING | |
| U80 | An answer offering one CLI gives `None`: the other side of U78's "two or more" | FR-010, US3-AS1a, US3-AS1b (D5, D6) | example | PENDING | |
| U81 | An answer offering none gives `None` | FR-010, Edge Cases nothing is available | example | PENDING | |
| U82 | An answer offering two with `env: None` gives `None` | FR-011 (W5) | example | PENDING | |
| U83 | With no answer in use (neither the row's nor home's) the note is `None` | FR-011, Edge Cases not answered yet | example | PENDING | |
| U84 | A row reading the home answer says "your home directory", and names its own directory once its own answer is filed | FR-004a, FR-012 (D7) | example | PENDING | |
| U85 | Two rows with different answers each get their own note, and filing one row's answer does not change the other's | US3-AS3, FR-012, Edge Cases several rows at once | example | PENDING | |
| U86 | A row whose own answer offers one CLI offers no choice (no chevron), whatever home holds | FR-010, US3-AS1a, SC-007 (026 FR-006) | example | DONE | `crates/micold-client/tests/features_session.rs::the_chevron_follows_the_rows_own_answer` |

### `crates/micold-client/src/ui/material/menu.rs`: `MenuOverlay::note`, `menu_panel_size_with_note`

Tests in `crates/micold-client/src/ui/material/menu_anatomy.rs` (a geometry gate inside the client
binary).

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U87 | A menu with a note lays out its items, then a `Divider`, then the note | FR-010 (W7) | example | PENDING | |
| U88 | The note wraps at the panel's width less the item padding at both sides | FR-010 (W7) | example | PENDING | |
| U89 | The note has no pressable region | US3-AS4, FR-010 (W7) | example | PENDING | |
| U90 | `menu_panel_size_with_note(items, note)` equals the laid-out panel's size for a one-line note and for a three-line note | FR-010 (W7) | example | PENDING | |
| U91 | A menu without a note is laid out as `menu_panel_size` says today | FR-010 (W7, "laid out exactly as before") | example | DONE | `crates/micold-client/src/ui/material/menu_anatomy.rs` `the_clamping_estimate_matches_the_panel_it_estimates` |

## Existing tests this feature rewrites

These pass today and encode the rule this feature replaces. Each is rewritten by the task named,
under the behaviour named, and is not credited as `DONE`:

- `missing_cli_is_reported_where_it_is_chosen.rs::the_host_placement_gets_a_different_sentence_and_no_image`
  and `::a_missing_pi_is_named_as_pi_coding_agent_and_never_as_its_command` assert 027's host
  sentence ("this computer", "Pi Coding Agent isn't"). T012, U47.
- `unavailable_default_says_so.rs::an_unavailable_default_is_named_when_the_list_opens_in_its_place`
  and `::an_unavailable_pi_default_is_named_as_pi_coding_agent` pin "isn't installed. Install
  it…". T021, U69.
- `session_start.rs::a_missing_cli_is_advised_on_where_sessions_run_and_on_what_is_being_started`
  pins the host `Fresh` sentence. T019, U64 and A12.
- `mcp_create_session.rs::a_cli_that_is_not_installed_fails_naming_it_and_leaves_no_record`
  asserts the binary name `pi`. T020, A14.
- The fixtures in `features_settings.rs` and `start_failure_notice.rs` quote the old text as an
  opaque string (T012, T021). U68's test does not depend on the wording.

## Invariants and edge cases still to place

None. Every Edge Case in spec.md has a line above: nothing available (U54, U81), a row drawn from
the home answer (U42, U73, U84), not answered yet (A8, U51, U83), the answer cannot be obtained
(the same state as "no answer in use": U51, U83; `env: None` is U53, U72, U82), several CLIs missing
(U19, U54), off with a path that names no file (U1), on with a blank path (U2, U29, A5), a relative
script path and per-directory outcomes (the state is each directory's own attempt: U66, A19, U85),
the script succeeded but changed nothing (U6, U35), the environment-include group shows another
directory (A3), the state changes while Settings is open (A2, U43), a message was shown and the
state then changed (U68, U75), several sessions and rows at once (A19, U85, U67), container
placement (U17, U18, U48, A12, A13), cross-platform (U24, and the bash and PowerShell fixtures of
U28–U39).

## Out of scope

- Rendering: the note's wrapping inside the Settings column, theming, and the start list passing
  its note (`ui/mod.rs`, T032) are GUI glue under Constitution I's exception, validated by
  quickstart §B (T017, T027, T035, T036), not by a cargo test.
- The labels on the Settings page (T015): the page renders the same three constants U21 holds the
  sentences to, so there is no second copy to test (plan, FR-003).
- A row with fewer than two available CLIs gets no chevron and nothing new (026 FR-006, decided by
  the user). No behaviour here adds a control to that row. A16 and U80 hold the absence.
- Pressing the note at the `App` entry point (US3-AS4's press): geometry, held by U89 and B11.
- `mcp_create_session.rs` is `#![cfg(unix)]`, so A14 runs on unix only. The sentence is held on
  every platform by U11–U25.
- The user guide (FR-017, T016, T026, T034) and the wording cross-check (T037): documentation.
- SC-002's count of five interactions: a manual measure, seen in quickstart §B B1–B2. A2 holds the
  part a test reaches (the next answer lists the CLI with no restart).
- Detecting where a CLI is installed, reporting one that is found but unusable, rewording 011's and
  035's lines, following an unsaved environment-include change: spec Out of Scope.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`, matches CI)
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets` (`mise run test-core`)
- Coverage: none (no cargo-llvm-cov, no cargo-tarpaulin)
- Mutation: none (no cargo-mutants)
- Property: none (no proptest). The rules over "all states and both places" (U20–U25, U59–U63) are
  enumerated in full: six states × two places is the whole input space.

Tests inside `crates/micold-client/src/main_tests.rs` and `src/ui/material/menu_anatomy.rs` are
`#[cfg(test)]` modules of the client binary. Run one with
`scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide {name}`.
