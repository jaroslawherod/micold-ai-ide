---
feature: 943-structured-commands-fallback
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override in .specify/templates/overrides or presets
verified_at: 4910aee
behaviors: 22 # task pairs of cycle-log.md (no tdd/test-list.md); support-code and docs rows excluded
proven: 12
likely: 6
test_after: 1 # T063/T064 (one cycle-log row, five behaviors)
no_test: 0
not_applicable: 3 # T025, T035, T040: stand-in support code, exercised by later tests
high_smells: 3
criteria_total: 38 # FR-001..FR-027 plus 005a, 008a, 022a (30) and SC-001..SC-008
criteria_covered: 37 # SC-007 has no automated test
mutation_score: 90 # 9 of 10 deliberate mutants caught, the survivor equivalent (100% of non-equivalent). Scope: commands/{mod,palette,terminal_only}.rs and model.rs of micold-core, sampled; no tool (cargo-mutants absent)
mutants_survived: 1 # equivalent, see Mutation results
suite: "micold-core --all-targets --no-fail-fast: 3908 passed, 4 failed (uid 0, permission tests, not this feature), 10 ignored, 217 binaries, 3m25s"
---

# TDD Verification: Structured commands and fallback (#943)

**Verdict: FAIL.** The T063/T064 behaviors (held-queue line, terminal-period marker, switch menu entries, switch confirmation dialog) have no recorded red: `cycle-log.md:96` says the red build ran after the first edits had landed and shows the red "by mutation after green", which is `TEST_AFTER` under the rubric. Three `HIGH` smells (two tautological constant assertions, one conditional assertion) also stand, and the feature has no test list.

The audit was independent: the auditing session did not write these tests, and every file cited was read cold at `4910aeee`. Everything outside the sampled core files rests on the cycle log, the history and a read of the tests, not on a mutation run.

## Preflight

- Profile `.specify/memory/tdd-profile.md` read: conventions, exemplars (`git.rs` unit, `workspace_lookup.rs`, `sandbox_real_session_start.rs`) and `helpers`.
- `tdd/test-list.md` is absent. `cycle-log.md:3` says so on purpose (as 482, 837, 946, 972): tasks.md's test tasks are the list. This audit used the cycle-log rows as the behavior list. No `traces` column exists, so the traceability section below is built by hand from `tasks.md` and the test files.
- `mise` is not installed in this environment. The `test-core` task's own command (`scripts/build-lock.sh cargo test -p micold-core --all-targets`) was run instead, with `--no-fail-fast`. Without that flag cargo stopped at the first failing binary (`settings_refuses_save_over_failed_read`, 162 of 217 binaries), so the first run was discarded.
- The 4 failures are `a_save_over_an_unreadable_file_is_refused_and_leaves_it_untouched`, `a_refused_write_is_logged_as_a_failure_with_the_reason`, `a_directory_that_cannot_be_emptied_names_what_survived` and `the_leftover_report_is_capped`. The audit ran as uid 0, which ignores file permissions. None touches the feature; they predate it (the files are not in the 943 diff).
- Range audited: the 50 commits matching `(943)` between `f27e1182` and `4910aeee` (the range also holds rebased commits of #941, #942, #944, not audited).

## Test-first evidence

History shows tests and source landing together in each `feat(943)` commit (per-cycle commits), with a few test-only follow-ups (`421ff59d`, `75f3b021`). Order inside a commit cannot be seen, so a row is `PROVEN` only where the log carries a red command with its failure output, and `LIKELY` where the log claims tests written first but the red is a stub or a statement, not output.

| Task(s) | Class | Evidence |
|---|---|---|
| T001–T002 | PROVEN | bash test red `12 cases, 1 failed` |
| T004–T005 | PROVEN | did not compile, variants listed |
| T006–T007 | PROVEN | did not compile, `CommandsReported` etc. |
| T008 | PROVEN | `schema_hash` `left: 62 right: 63` |
| T009–T012 | PROVEN | did not compile, `terminal_only::*` unresolved |
| T013–T014 | PROVEN | did not compile, `palette::*` unresolved |
| T015–T018 | PROVEN | did not compile, `ModelChoice`, `encode_*` |
| T019–T023 | PROVEN | 19 compile errors recorded |
| T024 | LIKELY | red by stubbing the functions to empty strings, 3 failed; draw glue declared a GUI exception |
| T026–T027 | PROVEN | did not compile, `send_control` |
| T028–T030 | PROVEN | `0 passed; 6 failed` |
| T032–T034 | PROVEN | did not compile; `schema_hash` failed 65 → 66 |
| T036–T037 | LIKELY | red by stubbing the dispatch to `OperationOk`: `0 passed; 10 failed` (stub, not absence) |
| T039–T042 | PROVEN | did not compile; `4 passed; 1 failed` with the feeder stubbed |
| T043–T044 | PROVEN | did not compile, `SessionSwitchKind` |
| T045–T049 | LIKELY | stub answering `OperationOk`: `1 passed; 10 failed` |
| T050–T054 | LIKELY | "written before the module existed" with no output kept; evidence is a mutation after green (`13 passed; 1 failed`) |
| T055–T057 | PROVEN | `unresolved import … CommandPalette`; `layout_snapshot` red |
| T058 | LIKELY | unit tests "written with the functions"; mutation after green |
| T061 | PROVEN | `0 passed; 2 failed` before the guide had the table |
| T063–T064 | TEST_AFTER | `cycle-log.md:96`: the red build failed on earlier edits, not on the missing names; red shown only by mutation after green |
| T065 visual-pass fix | PROVEN | `0 passed; 1 failed`, `left: "mcp" right: "/mcp"` |
| T025, T035, T040 | NOT_APPLICABLE | stand-in support code, exercised by T028–T029, T036, T041 |
| T003, T031, T038, T049 docs, T059, T062 | NOT_APPLICABLE | recording or docs; T059 and T062 are checked by scripts/`terminal_only_doc` |

Review-round fixes (A M2, M3, M5, M6, M7, B M7, M9) each record a red against the pre-fix code and are `PROVEN`; they are not rows above.

### Tests that already existed

Diffed the 943 commits for removed or loosened assertions, `#[ignore]`, renamed tests and config changes. No weakening found.

- `PROTOCOL_VERSION` pin bumps (`protocol_roundtrip.rs`, 62 → 63 → 65 → 66 → 67 and `schema_hash.rs`): the value is re-pinned with each wire change, as the feature's own T008 prescribes.
- `27e8bc53`: about a dozen `assert!(…is_none())` lines changed only in their path (`command_palette` → `composers.palette`); content kept.
- `92e16552`: `user_message_text` and its test deleted with the function, replaced by drawing the typed text (FR-003 is pinned in `core/tests/structured_transcript.rs`).
- `65a62038`: the `TERMINAL_PERIOD` constant and its two `contains` assertions replaced by `terminal_period_marker()`; the new marker is tested in `features_session.rs` (words "not shown here", "scrollback" are in the drawn marker per the log; I did not find a test that pins those words. See F8).
- `#[ignore = "timing…"]` in the range is #941's (`composer_large_draft.rs`, commit `8cc50e17`), not 943's.
- `mise.toml`, `Cargo.toml`: no threshold or exclusion changes by 943 commits.

`tasks.md`: T001–T065 are ticked and each has evidence in the cycle log. T066 (quickstart Part A) is correctly unticked: it was not run. No ticked task lacks evidence; no behavioral task is unticked with its behavior done.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| F1 | HIGH | `TEST_AFTER`: the held-queue line, terminal-period marker, switch-menu entries and switch dialog (T063/T064) have no recorded red. Their tests exist and pass (`features_session switching` 16, `held_queue_text` at `features_session.rs:1770`) and three mutants were caught after the fact, but the rubric fails a behavior without a red. Needs a red recorded by reverting the code under the existing tests, or an explicit waiver in the ledger. | `cycle-log.md:96`; `crates/micold-client/src/features/session.rs:342,359,442,477` |
| F2 | HIGH | Tautological assertions: `assert_eq!(MODEL_SHORTCUTS, ["sonnet","opus","haiku"])` and `assert_eq!(CHECKED_AGAINST, "2.1.307")` compare a constant with its own literal. They fail on any edit of the constant and prove no behavior. `CHECKED_AGAINST` is already pinned against the guide by `terminal_only_doc.rs`. Should assert a consequence (the picker offers the shortcuts when no models are reported; already in `features_command_palette`) or be deleted. | `crates/micold-core/src/structured/model.rs:176`; `crates/micold-core/src/structured/commands/terminal_only.rs:83` |
| F3 | HIGH | Conditional logic in a test: `if let Some(n) = unique` asserts the unique-prefix rule only for names that have a unique prefix; a list where none has one asserts nothing, so SC-002's prefix half can pass vacuously. Should assert that `unique` is `Some` for every name not a prefix of another, or count the prefixes checked and assert the count is positive. | `crates/micold-core/tests/command_palette.rs:162` |
| F4 | MED | No `tdd/test-list.md` and no `traces`: coverage of criteria cannot be checked mechanically and relies on tasks.md prose. Fail-closed: recorded as a process gap, and the reason a `BLOCKED` was weighed (the cycle log is detailed enough to audit, so the audit ran). | `specs/943-structured-commands-fallback/tdd/` |
| F5 | MED | No automated test reaches a real Claude Code: SC-007 (`/speckit-specify`, `/speckit-autopilot` in a real session) and the sandboxed/Copilot switch (B8) rest on the visual pass only (`autopilot.md` M9 T065: B8 sandbox, B8 Copilot, B9, `/login` card NOT RUN). No `sandbox_real_*` acceptance test exists for 943. T066 (Part A) is unrun. | `autopilot.md` T065 row; `tasks.md:400` |
| F6 | MED | Sleepy tests: fixed sleeps stand in for "nothing happened" checks (300 to 500 ms), already noted as a left MINOR in Review A M6 F10. | `crates/micold-daemon/tests/structured_switch.rs:873,892,930`; `structured_commands.rs:363`; `structured_model.rs:374` |
| F7 | MED | Non-deterministic: SC-003 is asserted against the wall clock (`slowest < 100 ms` over 1,000 runs); load on a shared runner can fail it. The `ignore`-in-release pattern used by #941's equivalent test is not followed. | `crates/micold-core/tests/command_palette.rs:234-258` |
| F8 | MED | Words of the terminal-period marker are not pinned: after `65a62038` the two `contains` assertions ("not shown", "scrollback") were removed with the constant, and I found no replacement asserting them (US5 scenario 2 asks for them). Confirm with the T063/T064 tests. | `crates/micold-client/src/ui/transcript/entries.rs` (removed), `features/session.rs:359` |
| F9 | MED | Framework under test: `a_default_choice_is_not_stored…` round-trips a test-local `Stored` struct that repeats the serde attributes instead of the real `StoredSession`; a change to the real attributes would not fail it. `session_store.rs` covers the real record, so this is redundant, not unsafe. | `crates/micold-core/src/structured/model.rs:229-261` |
| F10 | LOW | The equal-name tier (tier 0) in `filter` is dead: an equal name sorts first among prefix matches anyway (survivor M3). The FR-008 order stays right either way. | `crates/micold-core/src/structured/commands/palette.rs:39` |
| F11 | LOW | Magic value: `assert_eq!(commands.len(), 70)` depends on the recorded fixture's size, with the reason in the message only. | `crates/micold-core/tests/structured_commands.rs:41` |

Smell catalogue pass on the new or changed files read in full (`core/tests/structured_commands.rs`, `core/tests/command_palette.rs`, `core/tests/terminal_only_doc.rs`, the `#[cfg(test)]` blocks of `commands/mod.rs`, `terminal_only.rs`, `model.rs`) and skimmed with grep for sleeps, loops and `ignore` in the daemon and client suites (`structured_{commands,model,switch,resume}.rs`, `features_command_palette.rs`; 3,417 lines): no assertion-free tests, no doubled subjects, no self-approving snapshots (the layout fixture re-recording is recorded with its cause in the log), no skipped tests. Style matches the exemplars (unit blocks at the file bottom, integration tests per concern, `mod support`/`stream_json` helpers, stock asserts with rule messages). Daemon tests drive the real server over the protocol with the recorded stand-in CLI, which is the repo's accepted boundary double.

The daemon and client suites were not read line by line; their smell status is by sample and by the log. See "What was not audited".

## Mutation results

No mutation tool in the profile (`mutation: null`, `cargo-mutants` absent). Ten deliberate mutants, one at a time, each restored with `git checkout` and the suite re-run (3908 passed, 4 failed, 10 ignored, the same as the baseline). Run set: core lib plus `structured_commands`, `command_palette`, `structured_transcript`, `session_store`, `protocol_roundtrip`, `terminal_only_doc`.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `commands/mod.rs` `else if c.builtin` → `!c.builtin` | T011 kind rules | No | caught by 4 tests |
| `palette.rs` `caret <= name_end` → `<` | T013 palette_query | No | `the_palette_is_open_while_…` |
| `palette.rs` `name == query` → `false` (tier 0) | T013 filter | Yes | equivalent: the equal name still sorts first within the prefix tier (F10) |
| `commands/mod.rs` `MAX_DESCRIPTION_BYTES` 512 → 513 | T011 parse_initialize | No | the 300 × `é` case |
| `model.rs` drop `!name.starts_with('-')` | T015 ModelName | No | `--resume`, `-m` cases |
| `palette.rs` `accept` caret `len()` → `len() - 1` | T013 accept | No | `accepting_fills_…` |
| `commands/mod.rs` drop `!commands.is_empty()` filter | T011 from_history | No | `a_later_spawn_that_reports_nothing_…` |
| `terminal_only.rs` `/config` alias `settings` dropped | T009 | No | lib classify test and `terminal_only_doc` |
| `palette.rs` drop the `/ ` leading-whitespace guard | T013 palette_query | No | caught |
| `model.rs` `cli_value` `"default"` → `""` | T017 | No | caught by `control::tests::initialize_and_set_model_…` (not a model.rs test) |

Score: 9 of 10 caught; the one survivor is equivalent. No survivor sits inside a `DONE` behavior, so the rubric's mutation `FAIL` condition does not apply. The cycle log records further mutations by the feature's own loop (T050, T052, T058, T061, T063/T064) that I did not repeat.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| FR-001, FR-002, SC-001 | `structured_commands` (daemon) T028; `core/tests/structured_commands.rs` | Yes: daemon protocol with stand-in CLI |
| FR-003 | `structured_transcript` T022; `transcript_rich` | Yes (core builder); drawing by the visual pass |
| FR-004, FR-005, FR-006 | `structured_transcript` T019–T021; `features_transcript_rich` | Yes (core builder) |
| FR-005a | `structured_resume` T041; `session_store` T039 | Yes |
| FR-007, FR-008, FR-008a, SC-002, SC-003 | `command_palette` (core), `features_command_palette` (client reducer) | Yes at the client reducer; SC-003 by wall clock (F7) |
| FR-009, FR-010 | `core/tests/structured_commands.rs`; palette tests | Yes |
| FR-011, FR-012, SC-006 | `terminal_only`, `terminal_only_doc`, `structured_commands` (refusals), `features_session` | Yes |
| FR-013 to FR-018, SC-004 | `structured_model` (10), `features_command_palette` | Yes |
| FR-019 to FR-025, SC-005 | `structured_switch` (11), `features_session` (75) | Yes (restarts, concurrency, held queue) |
| FR-022a | `features_session` (login offer from the latest answer) | Reducer only; the `/login` card was not run live |
| FR-026 | gates and `structured_*` suites unchanged | Yes |
| FR-027 | `terminal_only_doc`, `check-user-guide-updated.sh` | Yes (doc check) |
| SC-007 | none automated | No: visual pass only (F5) |
| SC-008 | the full gate runs the pre-existing suites; `structured_session` assertions unchanged | Yes, by the gate |

Every test name cited in the cycle log that I looked up exists (16 sampled: e.g. `a_compaction_keeps_its_first_summary`, `the_command_list_is_the_one_the_history_holds`, `the_open_model_picker_keeps_the_keyboard_from_the_pane`). Untested criteria: SC-007 (no automated test; covered by a live visual pass). Tests tracing to nothing: the two constant tests of F2.

## What was not audited

- Mutation was a 10-mutant sample over four core files (`commands/mod.rs`, `palette.rs`, `terminal_only.rs`, `model.rs`), not the files of the daemon (`state.rs`, `catalog.rs`, `structured.rs`, `ops.rs`) or the client (`features/*`, `ui/*`), and not `control.rs`, `transcript/build.rs` or `session.rs`.
- The daemon and client test files were checked by grep (sleeps, loops, ignores) and by the log, not read line by line; a full smell pass over about 6,000 lines of them is open.
- Only `micold-core` was run. The daemon, client and `scripts/tests/*.test.sh` suites were not re-run; the ledger records a green `split-gate.sh` at `f38301f3`.
- The 4 core failures are permission tests failing under uid 0, not examined further.
- The range includes rebased commits of #941, #942 and #944; only commits matching `(943)` were audited.
- Visual-pass evidence (`visual-pass/*.md`, PNGs), quickstart Part A (T066), sandbox and Copilot switching, macOS and Windows: not assessed.
- Coverage: the profile has no tool (`coverage: null`).
- Performance beyond SC-003 and test-suite speed: not assessed.
- Remediation tasks were not appended to `tasks.md`: the request restricted writes to this file.
