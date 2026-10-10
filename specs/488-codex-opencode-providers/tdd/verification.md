---
feature: 488-codex-opencode-providers
verdict: FAIL
verdict_after_close: CONDITIONAL — blocking findings TDD-1 and TDD-2 fixed in the close PR (not re-audited from cold); mutation score unmeasured (T044 open)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved: no overrides/ or presets/ in this repo
verified_at: aac20577
range: 96f85d69^..HEAD, 488 commits only (040/041 commits eefb2f51, 75691338, 82437f91 excluded as other features)
behaviors: 10 # no tdd/test-list.md exists; behavior groups derived from tasks.md milestones
proven: 0
likely: 2
test_after: 6
no_test: 0
not_applicable: 2
high_smells: 0
weakened_existing_tests: 1
criteria_total: 18 # US1-US6 acceptance scenarios
criteria_covered: 12 # through a real entry point; 6 only partly (see Traceability)
mutation_score: unmeasured # cargo-mutants 27.1.0 ran out of disk, see Mutation results
mutants_survived: unmeasured
suite: 6197 passed, 6 failed, 9 ignored (all 6 failures are root-user permission tests unrelated to 488); wall time not separable from a cold build
independence: the auditor did not write these tests; every cited file was re-read in this session
---

# TDD Verification: Codex CLI and OpenCode as session providers

**Verdict: FAIL.** Six of ten behavior groups landed test-after (no recorded red, or a log that says "not red-first"), and an existing sandbox test was loosened to match new behavior that contradicts the spec.

Stage that failed: question 1 (did the tests come first) and question 3 (strength, unmeasured). Question 2 (assertions) is mostly sound.

## Test-first evidence

No `tdd/test-list.md` exists (feature not planned through the extension), and `cycle-log.md` has sections for M3 to M6 only. Groups below come from `tasks.md`.

| Group (tasks) | Class | Evidence |
|---|---|---|
| M1 provider surface, start, remember (T003, T005-T007, T010) | TEST_AFTER | No M1 section in the cycle log. `a06f7f64` adds `codex_provider.rs`, `opencode_provider.rs`, `codex_opencode_start.rs` and the source in one commit. |
| M1 store round trip, protocol 38 to 39 (T004, T008) | TEST_AFTER | Same commit, no red recorded. |
| M1 chooser, Settings, MCP lists (T009) | TEST_AFTER | Same commit, no red recorded. |
| M2 unavailable providers (T012-T015) | NOT_APPLICABLE | Characterization. `aba051d3` changes tests only; no baseline run recorded. |
| M3 seam additions on old providers (T016-T017) | TEST_AFTER | Log: "Not broken retrospectively (covered by the same passes, green)". `44a49385` has tests and source together. |
| M3 Codex bind, resume, candidates (T018-T021) | TEST_AFTER | Log opens "tests and code were written in one pass, not red-first". Retrospective breaks exist for 3 behaviors only; they prove the test can fail, not that it came first. |
| M3 Codex naming, bounded prefix, archive marker (T018-T019) | TEST_AFTER | Listed under "Not broken retrospectively". |
| M4 OpenCode resume, bind, naming (T023-T025) | LIKELY | Log records a red run (7 failed, 8 passed) before the code, but `4a23c592` commits tests and source together, so order is not corroborated. |
| M5 activity, tool-server log, trust (T027-T029) | NOT_APPLICABLE | Characterization, no code change. Retrospective breaks for 2 of 3; the badge behavior cannot be broken (see finding 3). |
| M6 per-provider sign-in file (T031-T032) | LIKELY | Red is compile errors only (missing API), recorded; `d0b03628` combines tests and source. |

Discrepancies between sources: none contradicted the log, but the log is the only evidence for M4 and M6.

## Existing tests changed by the range

Mechanical, intent preserved: `ai_cli_registry.rs` (ALL grows to 5), `cli_reason.rs`, `directory_availability.rs`, `missing_cli_is_reported_where_it_is_chosen.rs`, `main_tests.rs` (provider lists), `sandbox_argv.rs` mapped-volume counts 4 to 6, `store_roundtrip.rs` unknown provider `"Codex"` to `"Gemini"` (same intent, still a load error), `mcp_tools_catalog.rs` rename three to five (an added assertion, not a loosening). SC-007 ("existing providers' tests pass unchanged") is therefore not literally true; the edits above were needed.

Loosened: see finding 1.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Existing test `only_the_ai_cli_sign_in_is_mounted_writable` was changed from "only Claude Code's sign-in is rw, every other credential ro" to a path-substring predicate that also accepts Codex and OpenCode `auth.json` as rw. Before: `let expected = if host == sign_in { "rw" } else { "ro" };`. After: `host == sign_in \|\| host.ends_with("auth.json") && (host.contains(".codex") \|\| host.contains("opencode"))`. The predicate re-implements the mount rule by string matching instead of asking `AiCli::sandbox_auth_file`. It also contradicts `spec.md` US5 independent test and `tasks.md` M6 deliverable, which say the sign-in is read-only; the writable choice is recorded only in `autopilot.md` and the cycle log, and the spec was never amended. A writable mount of a host credential file is a security-relevant change. | `crates/micold-core/tests/sandbox_argv.rs:301-309` (commit `04f977e2`/`d0b03628`); `spec.md:92`, `tasks.md:135` |
| 2 | HIGH | Test-after for 6 of 10 behavior groups (M1 and M3), including the protocol bump and the Codex binding rules that Principle II (concurrent sessions never cross-resume) depends on. Retrospective breaks cover 3 behaviors; 8 M3 behaviors are explicitly unbroken. | `cycle-log.md:3-11`; commits `a06f7f64`, `44a49385` |
| 3 | MED | `the_badge_stays_unknown_through_output_and_silence` samples a badge whose projection default is `Unknown`, so the 3 s loop cannot fail on any activity regression; the log admits no cheap break exists. Only the `activity_source` asserts (lines 424-432) can fail, and `codex_provider.rs:56-59` and `opencode_provider.rs:59-62` already pin the same two values (redundant). The AS2 half (a provider with a source does follow busy/idle) is not pinned by this feature at all. | `crates/micold-daemon/tests/codex_opencode_start.rs:418-446`; `cycle-log.md:36` |
| 4 | MED | US3 AS3 (named from first turn) has no test through the daemon. The stand-ins write a first turn (`hello codex`, `hello opencode`) but no assertion reads the session's name; naming is proven only by `read_title` unit calls. The wiring at `state.rs:3076` and `:3231` is unexercised for minted providers. | `codex_resume.rs:77,108`; `codex_provider.rs:286`; `opencode_provider.rs:318` |
| 5 | MED | `the_app_start_path_gives_the_same_reason_before_any_terminal` is named for the app start path and for "before any terminal", but only calls `ops::cli_unavailable` and checks the text contains the command. It never calls `start_session` and never checks that no terminal exists. US2 AS2 "from the app" is covered only by the MCP test at lines 295-326. | `crates/micold-daemon/tests/codex_opencode_start.rs:328-338` |
| 6 | MED | Sleepy and shared-state tests. Fixed sleeps: 6 s (twice, via `never_cross_resume`), 2 s, and a 3 s polling loop; the OpenCode hang case waits out a real 2 s timeout. `terminal_backend.rs` sets and restores process-wide `CODEX_HOME` with no lock in a multithreaded test binary (`opencode_provider.rs` and the daemon files do lock). | `codex_resume.rs:313`; `codex_opencode_start.rs:440-444,518`; `opencode_provider.rs:304-311`; `terminal_backend.rs:143-148` |
| 7 | MED | Duplicated setup. The `Env` guard (`set`/`Drop`) is copied byte for byte between `codex_resume.rs:33-63` and `codex_opencode_start.rs:34-64`, on top of five existing copies (`mcp_create_session.rs`, `mcp_binding_spawn.rs`, `mcp_cross_session.rs`, `pi_launch_wiring.rs`, `review_send.rs`). The new files added two more instead of extracting into `tests/support/`. Counts against the duplication budget. | as cited |
| 8 | MED | Likely unpinned logic (read, not mutated). In `codex_first_turn` the fallback filter `!starts_with('<') && !starts_with("# AGENTS.md")` is never reached with a context item present: the test that includes `<environment_context>` also has an `event_msg` turn, which wins first, and the fallback test has no context item. Constants `CLOCK_ALLOWANCE` (2 s, both providers), `RECENT_DAYS` (3), `OUTPUT_CAP`, `LIST_LIMIT` have no boundary test: the "older than spawn" case uses 3600 s against a 5 s window. | `codex_provider.rs:286-332,165-179`; `first_turn.rs` `codex_first_turn`; `provider.rs:1707-1711,1957-1963` |
| 9 | MED | Windows and platform edges untested. The OpenCode resume module is `#[cfg(unix)]`; no `.cmd` or `.exe` availability case exists though T003 and the spec's platform edge case ask for one. | `opencode_provider.rs:95`; `spec.md:120` |
| 10 | LOW | No `tdd/test-list.md` and no M1 or M2 cycle-log sections: the extension's evidence trail is incomplete. Strictly the rubric's BLOCKED condition ("no test list") also applies; FAIL was chosen because the evidence still supports a grade. | `specs/488-codex-opencode-providers/tdd/` |
| 11 | LOW | Stale comments: `CodexProvider` docs say "every store read answers nothing recorded" and the two test file headers say "fresh start only", all false since M3/M4. | `provider.rs:1698-1701`; `codex_provider.rs:1`; `opencode_provider.rs:1` |
| 12 | LOW | `folder_trust.rs` Codex test is eager (six behaviors in one test, though each assertion is distinct). | `crates/micold-core/tests/folder_trust.rs` (added `codex_trusts_what_its_config_lists_and_everything_below`) |

Smell pass summary: no assertion-free, tautological, doubled-subject or empty tests found. The `if delivered { .. } else { .. }` in `a_first_prompt_is_never_typed_into_a_trust_question` (`codex_opencode_start.rs:503-520`) asserts in both branches, so it is not the HIGH conditional smell. Doubles are real stand-in executables on a private `PATH`, not mocks, which is the opposite of over-mocking. Secrets: none found.

## Mutation results

**Unmeasured.** cargo-mutants 27.1.0 was installed (`cargo install`) and invoked as instructed (`--in-place`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`, `-f provider.rs -f first_turn.rs`, `-F` restricted to the Codex/OpenCode and binding functions, `-p micold-core --test-package micold-core`; 190 candidate mutants listed). `df -h /` showed 27 GB free before the full-suite build, 2.8 GB after it (the cold workspace build left `target-shared` at 25 GB), and 0 bytes within two minutes of starting the mutation baseline, which rebuilt dependencies under a different feature set. The run died with ENOSPC before executing any mutant. Per instruction it was not retried. No deliberate-mutant fallback was done (it also needs a build). The source files were never mutated: `git status` shows no change under `crates/`.

Also note for any rerun: the baseline would be red in this environment because six tests fail when run as root (see Suite), so a rerun needs a non-root user or `-- --skip` of those four micold-core tests. That skip is a test-filter change and must be reported if used.

Predicted survivors from reading (not measured): finding 8 items.

## Suite

`CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 scripts/build-lock.sh cargo test --workspace --no-fail-fast`: 6197 passed, 6 failed, 9 ignored. Failures: `settings_refuses_save_over_failed_read`, `settings_write_is_logged`, `worktree_leftovers` (2), `mutation_semantics` (1), `settings_service_write_refuses_failed_read`. All rely on chmod-denied files, which root bypasses (`id -u` is 0). None touch Codex, OpenCode or the provider seam, so they are environment failures, not feature failures. The profile's recorded baseline was green (5071) on a non-root run.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| US1 AS1, AS2 start in worktree, labelled | `mcp_create_session_starts_each_provider_in_its_worktree` | Yes (MCP entry point) |
| US1 AS3 survives restart | `the_provider_survives_a_restart_of_the_service`, `every_provider_survives_a_save_and_load` | Yes |
| US1 AS4 Settings default | `the_settings_default_applies_without_an_override` | Yes |
| US1 AS5 MCP `ai_cli` | `create_session_accepts_exactly_the_five_ai_clis`, start test above | Yes |
| US2 AS1 unavailable with reason | `a_provider_off_the_path_is_listed_unavailable` (no reason asserted), `missing_cli_is_reported_where_it_is_chosen` | Yes |
| US2 AS2 refused, no terminal | MCP refusal test (record count and no process asserted); app path see finding 5 | Yes (MCP), partial (app) |
| US2 AS3 available on next listing | `a_provider_installed_afterwards_is_available_on_the_next_listing` | Yes |
| US3 AS1, AS2 resume and fresh | `restart_resumes`, `never_cross_resume`, provider `resume` modules | Yes |
| US3 AS3 named from first turn | provider unit tests only | No (finding 4) |
| US3 AS4 guide says no resume | none (docs) | No |
| US4 AS1 badge `Unknown` | finding 3 test | Yes, weak |
| US4 AS2 busy then idle | pre-existing `activity_pipeline::hooks_drive_the_projected_activity_signal` | Yes (not this feature's test) |
| US5 AS1 CLI in image | `sandbox_real_ai_cli` (iterates `AiCli::ALL`), not run | Not verified |
| US5 AS2 sign-in shared | `sandbox_credentials.rs` mount tests, `sandbox_real_ai_cli_sessions.rs` not run | Partial (finding 1) |
| US5 AS3 absent files pruned | client `only_the_sign_in_files_this_host_has_are_shared` | Yes |
| US6 AS1 guide lists providers | none; T035 unticked, SC-006 unchecked | No |

Criteria with no test through a real entry point: US3 AS3, US3 AS4, US6 AS1, US5 AS1, partly US5 AS2. Tests tracing to nothing: none found (the test list does not exist, so tracing was by task id and test name).

## What was not audited

- Mutation: unmeasured (disk). No survivor triage, no deliberate mutants.
- `crates/micold-daemon/src/state.rs` bind loop (133 added lines) and `ops.rs`, `sandbox/mod.rs`, `client/shell/sandbox.rs` source were read only in diff headers, not graded line by line.
- Real-runtime sandbox suite (`sandbox_real_*`, `mise run image`, `mise run test-sandbox`): not run, so SC-005 is unverified.
- `mise run gate` (fmt, clippy, scripts/tests) and `cargo check` for Windows and macOS targets: not run.
- Docs (T011, T015, T022, T026, T030, T034) content and the `docs/user-guide` edits sitting uncommitted in the working tree: not read.
- Visual pass evidence (`visual-pass.md`) and quickstart Parts A and C: not re-run.
- Performance of the suite: only the 6 s, 3 s and 2 s sleeps were noted; no timing was taken for the new files.
- Coverage: no tool in the profile.

## Close addendum (2026-10-10)

Mutation testing was not measured: cargo-mutants hit ENOSPC on a cold workspace build (25 GB) in the container; T044 stays open for a host with disk and a non-root user. Findings TDD-1 (spec amended: sign-in mount is writable; test predicate now derived from `sandbox_auth_file`), TDD-2 (test-list.md and retrospective reds for the core behaviours), TDD-3, 4, 5, 8, 9, 10, 11 are fixed in the close PR. TDD-6/7 partly: the `Env` guard is shared; two fixed sleeps stay because they assert absence. Quickstart Part C (real CLIs) is not run: no Codex/OpenCode installed here.

Windows: the Windows-only availability path (`.cmd`/`.exe` via PATHEXT) is covered by an un-gated test; the `#!/bin/sh` stand-in resume tests are unix-only because no shell script runs on Windows, and `cargo check --target x86_64-pc-windows-msvc --tests` is left to CI (target not installed here). `mise run duplication` was not run (no mise, npx offline).
