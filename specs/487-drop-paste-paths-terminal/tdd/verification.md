---
feature: 487-drop-paste-paths-terminal
verdict: BLOCKED
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved from the extension copy; no override or preset exists
verified_at: 474f5937 # short SHA audited (base before feature a42cf255)
behaviors: 14 # no test-list.md; the 14 "write failing tests" tasks of tasks.md stand in
proven: 0
likely: 0
test_after: 14
no_test: 0
high_smells: 6
criteria_total: 19 # acceptance scenarios: US1 x5, US2 x5, US3 x5, US4 x4 (SC rows in Traceability are extra)
criteria_covered: 19 # each has at least one test; none through the real OS entry point, 3 only via a pure function or a reducer fed a ready value
mutation_score: null # no mutation tool (profile: mutation null); 12 deliberate mutants instead, 9 caught of 12 (see below)
mutants_survived: 3 # M1, M3, M10; all real gaps, none equivalent
suite: not run in full (disk full, see What was not audited); feature targets 98 passed, 0 failed
---

# TDD Verification: Drop files or paste images into a terminal to insert their paths

**Verdict: BLOCKED.** The feature has no `tdd/test-list.md` and no `tdd/cycle-log.md`, so no behavior has a recorded red. The audit ran anyway against `spec.md` and `tasks.md`: on that evidence alone it would be `FAIL` (all 14 test tasks `TEST_AFTER`, 6 `HIGH` smells, 2 surviving mutants inside ticked tasks).

Not an independent audit of authorship: this report was written by a fresh session that did not write the feature, and every file cited was re-read at `474f5937`. No fresh-context subagent was used for the smell pass; every finding below was verified by opening the cited line.

## Test-first evidence

Evidence for ordering: `autopilot.md` (declined finding "B M1 F4") states "no `tdd/cycle-log.md`: tests were written beside the code; red-first was not recorded and is not invented now." `tasks.md` says tests come first and are "seen to fail"; nothing records that. Git history cannot corroborate: every `feat(487)` commit adds tests and source together (`4576ac69`, `9a5fea98`, `480d5bbe`, `08160c13`), and the later `fix(487)` commits (`18fea954`, `87350d91`, `01c9a4af`, `c6a6b3e8`) change source with little or no new test. Fail closed: no red recorded means test-after.

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| T004 quote tables | TEST_AFTER | `4576ac69` adds `quote.rs` source and tests together; no red |
| T006 `ShellKind::detect` | TEST_AFTER | same commit; no red |
| T008 real-shell round trip | TEST_AFTER | same commit; no red |
| T009 `plan_insertion` host | TEST_AFTER | same commit; no red |
| T011 drop reducer | TEST_AFTER | same commit; no red |
| T012 coalescing / pointer | TEST_AFTER | same commit; no red |
| T013 pane hit-test | TEST_AFTER | same commit; no red |
| T018 `PastedLayout` | TEST_AFTER | `9a5fea98`; no red |
| T020 image writer | TEST_AFTER | `9a5fea98`; no red |
| T022 `paste_source` + reducer | TEST_AFTER | `9a5fea98`; no red |
| T025 sandbox `plan_insertion` | TEST_AFTER | `480d5bbe`; no red |
| T027 sandbox reducer + writer | TEST_AFTER | `480d5bbe`; no red |
| T030 `orphans` | TEST_AFTER | `08160c13`; no red |
| T032 daemon cleanup | TEST_AFTER | `08160c13`; no red |

**Existing tests weakened: none found.** Across the 12 test-touching commits of the feature (`4576ac69` through `54750b12`) the only removed line inside a test file is `#[allow(dead_code)]` in the feature's own new `pasted_cleanup.rs` (`54750b12`). No `#[ignore]`, skip, filter rename, config exclusion, or lowered threshold was added. Added `#[cfg(unix)]` gates (`mod.rs:370,444,462`, `quote.rs`, `path_insert_roundtrip.rs`) apply only to new tests, each with a cross-platform twin or a stated reason.

**tasks.md vs evidence.** All T001 to T034 are ticked (`[X]` for T001 to T017, `[x]` after). With no test list, a tick cannot be checked against a `DONE` behavior, so every behavioral tick is a completion claim without the lifecycle's evidence. Two tick texts no longer match what was built: T012 says "pointer state follows `CursorMoved`" (the design changed, `autopilot.md` Decisions M1) and no test covers pointer state; T022 says reducer tests for "text on the clipboard ... no file is created", but only the pure `paste_source` is tested (`clipboard.rs:278`).

## Findings

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | No test list, no cycle log, no recorded red for any behavior; ordering not provable from history | `autopilot.md` (B M1 F4); `git show --stat` of the four feat commits |
| 2 | HIGH | Mutant M3 survived: sandbox forcing the shell to `Bash` is not pinned. Both tests that claim it use fish with a name whose fish and bash quoting are identical (no `\` or `'`), so the assertion cannot tell the shells apart. Assert with a name containing a backslash (bash `'a\b'`, fish `'a\\b'`) | `micold-core/src/path_insert/mod.rs:372-388` (comment at 383 claims "single quotes, not fish's"); `micold-client/tests/features_session.rs:1021` |
| 3 | HIGH | Tautological: two different input paths are fed in and the test asserts the outputs differ. Uniqueness is owned by `next_file`/`save`, not the reducer. Assert nothing here, or drive the real `save` twice | `micold-client/tests/features_session.rs:1128` |
| 4 | HIGH | Vacuous plus conditional assertion: `q.len() >= name.len() + 2` passes for any quoting that adds two characters; the `Err` arm is accepted for Cmd for every name without checking which. Name says "refuses it by name" but checks neither. Mitigated for sh and bash by the round trip, not for fish, PowerShell, cmd | `micold-core/src/path_insert/quote.rs:177-186` (assertion at 181) |
| 5 | HIGH | Silent skip: the zsh, fish, and non-UTF-8 bash round trips pass when the shell is missing (`if installed(..)` / early `return`). In this environment only `sh` and `bash` exist, so SC-003 ("100% round-trip for each supported shell") was exercised for 2 of 5 shells; fish and zsh quoting and PowerShell (Windows-only test) ran nothing and reported green | `micold-core/tests/path_insert_roundtrip.rs:48,57,72,80` |
| 6 | HIGH | Vacuous: asserts that source text of two files contains substrings (`.on_file_drop(`, `FileDropped`, `.pane_at(`). A comment containing the string satisfies it, and wiring commented out elsewhere would not fail. The widget path from OS drop event to `PaneMsg::FileDropped` has no behavioral test (declined as B M1 F2) | `micold-client/tests/pane_drop_target.rs:61-70` |
| 7 | HIGH | Mutant M10 survived: the unbracketed control-character refusal (review MAJOR fixed in `18fea954`, guards FR-003 and SC-004: a newline in a name would run the command) is wired only in `drops.rs:104`; just the predicate `has_control_character` is tested. Deleting the refusal fails no test | `micold-client/src/shell/drops.rs:100-116` vs test at `:131` |
| 8 | MED | Mutant M1 survived: "most specific project wins when one sits inside another" (`min_by_key` instead of `max_by_key`) has no test; the sandbox then maps a nested project's file through the outer mount | `micold-core/src/path_insert/mod.rs:220-231` |
| 9 | MED | FR-006 and FR-009 reach no test through their entry point: `on_paste_requested`, `on_image_pasted`, the `arboard` read, and the text-wins-over-image rule are only covered by the pure `paste_source` table and by reducers fed a ready `Result<PathBuf,_>`. The M2 chord/middle-click routing bug (`87350d91`) has a unit test (`terminal_pane.rs:1923`, feature `gui`; not run here) | `clipboard.rs:101,278-302`; `features_session.rs:1103-1143` |
| 10 | MED | Eager and echoing: `a_drop_inserts_the_quoted_paths_in_order_and_submits_nothing` checks quoting, order, bracketing, and no leading space in one test with unlabeled `bytes[6..len-6]` magic bounds; `the_target_is_the_pane_dropped_on...` passes the `DropTarget` in and asserts it comes out, so it does not test the unfocused-pane claim (that lives in `pane_drop_target.rs` and `main_tests.rs`) | `features_session.rs:857-882,885-912` |
| 11 | MED | Bypassed test utility / duplicated setup: the same hand-built `MountSet` (`StateMount`, `HomeMount`, `SecretMount`) appears three times instead of one factory beside the `support` helpers | `mod.rs:339-357`, `pasted_image.rs:288-307`, `features_session.rs:985-1010` |
| 12 | MED | No real-entry-point test for any criterion: no OS drag-and-drop, no real clipboard, no `sandbox_real_*` acceptance test for US3, and the visual passes (quickstart Part B) were skipped for all four milestones (`autopilot.md` Decisions). Closest are `main_tests.rs` (drives `update()` with `PaneMsg::FileDropped`) and the daemon `pasted_cleanup.rs` (calls `delete_session` and `sweep_pasted_images` on a real service) | `autopilot.md` Decisions M1-M4 |
| 13 | MED | Conformance gaps: T012 pointer-state test absent; T022 "text on the clipboard creates no file" reducer test absent; T027 "if neither location is possible" is tested only as a reducer receiving `Err` | `tasks.md` T012, T022, T027 |
| 14 | LOW | Test of a constant: `the_name_set_is_at_least_twenty_awkward_names` asserts the length of a `const`; it pins the SC-003 floor, not behavior | `quote.rs:115` |

Properties: tests are isolated (tempdirs, no network), deterministic, and fast (feature targets finish in under 0.2 s of test time). `a_worktree_that_cannot_be_written...` uses a file-as-worktree so it works as root (good). The Windows-only and root-only variability the ledger mentions ("six root-only tests", M4 gate) is outside this feature's files.

## Mutation results

No mutation tool in the profile (`mutation: null`), so this is a deliberate-mutant sample, not a score. 12 mutants, one at a time, each restored with `git checkout -- <file>`; tree clean afterwards (`git status --short` empty) and the affected targets re-run green (below). The first run of M7 failed to apply and M9 used a filter that matched no tests; both were re-run with a correct command and are reported as the re-run.

| Mutant | File | Behavior | Survived | Judgment |
| --- | --- | --- | --- | --- |
| M1 `max_by_key` to `min_by_key` | `path_insert/mod.rs:229` | T025 nested projects | Yes | Real gap, finding 8 |
| M2 skip the `NotMounted` check | `path_insert/mod.rs:233` | T025 | No (2 failing) | pinned |
| M3 sandbox keeps the host shell instead of Bash | `path_insert/mod.rs:172` | T025, T027 | Yes | Real gap, finding 2 |
| M4 invert orphan liveness test | `pasted.rs` `orphans` | T030 | No (2 failing) | pinned |
| M5 drop `seq` from the file name | `pasted.rs` `next_file` | T018 | No (2 failing) | pinned |
| M6 join paths with `""` not `" "` | `path_insert/mod.rs` `text` | T009 | No (2 failing) | pinned |
| M7 do not escape `'` for POSIX | `quote.rs:24` | T004, T008 | No (3 failing) | pinned (also by the real-shell round trip) |
| M8 cmd accepts `%` | `quote.rs` | T004 | No (3 failing) | pinned |
| M9 `paste_source` ignores non-empty text | `clipboard.rs:102` | T022 | No (2 failing) | pinned |
| M10 disable control-character refusal | `shell/drops.rs:104` | T011 / SC-004 | Yes | Real gap, finding 7 |
| M11 drop the "process is running" gate | `features/session.rs:1880` | T011 | No (3 failing) | pinned |
| M12 daemon removes no pasted image on delete | `daemon/src/state.rs` `remove_pasted_images` | T032 | No (2 failing) | pinned |

Sampled: 12 mutants over 10 behaviors (T004, T009, T011, T018, T022, T025, T027, T030, T032, plus the review-fix guard). Highest-risk picks were data loss (cleanup deletes), the sandbox visibility boundary (FR-011), and the "nothing runs until Enter" rule (FR-003). Not exhaustive; coverage was unavailable (`coverage: null`).

Restore check after the last mutant (all green, `CARGO_INCREMENTAL=0`): `micold-core --lib path_insert` 28 passed; `path_insert_roundtrip` 5 passed; `micold-daemon --test pasted_cleanup` 3 passed; `micold-client --test features_session --test pane_drop_target` 47 + 4 passed; `--bin micold-ai-ide clipboard::` 4 passed; `drops` 11 passed (includes `main_tests` drop tests); `pasted_image` 8 passed.

## Traceability

No `traces` column exists, so criteria were mapped to tests by reading them.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1 one file, no newline | `features_session.rs:857`, `mod.rs` host tests | Partial: reducer level only |
| US1.2 three files in order | `mod.rs:273`, `main_tests.rs` (coalescing) | Partial |
| US1.3 awkward names | `quote.rs` tables, `path_insert_roundtrip.rs` (sh, bash only here) | Partial: finding 5 |
| US1.4 text already typed untouched | `features_session.rs:857` (no leading space) | Partial |
| US1.5 pane dropped on, focused or not | `pane_drop_target.rs`, `main_tests.rs` | Partial: widget event path untested (finding 6) |
| US2.1 image saved, path inserted | `pasted_image.rs:159+`, `features_session.rs:1103` | No: clipboard read untested (finding 9) |
| US2.2 text unchanged | `clipboard.rs:278` (pure) | No |
| US2.3 text and image: text wins | `clipboard.rs:283` (pure) | No |
| US2.4 unreadable/unsaveable: message | `features_session.rs`, `pasted_image.rs` | Partial |
| US2.5 distinct paths | `pasted.rs`, `pasted_image.rs` `two_saves_never_share_a_file` | Yes at unit level; `features_session.rs:1128` is a tautology |
| US3.1 container path | `mod.rs:372,391`, `features_session.rs:1021` | Partial; shell choice unpinned (finding 2) |
| US3.2 outside refused with message | `mod.rs:410`, `features_session.rs:1021,1055` | Partial |
| US3.3 mixed drop | `mod.rs:522`, `features_session.rs:1021` | Partial |
| US3.4 pasted image in container | `features_session.rs:1078`, `pasted_image.rs:276` | Partial |
| US3.5 worktree unwritable: state dir | `pasted_image.rs:276` | Partial |
| US4.1 delete removes images | `pasted_cleanup.rs` | Yes (real service) |
| US4.2 no untracked status | `pasted_image.rs` (git status clean) | Yes |
| US4.3 dropped file never deleted | `pasted_cleanup.rs` | Yes |
| US4.4 start sweep | `pasted_cleanup.rs` | Yes |
| SC-003 / SC-004 | round trip (2 of 5 shells ran), `features_session.rs:857`, `main_tests.rs`; SC-004 guard for unbracketed text untested (finding 7) | Partial |

Untested or weakly tested: all 19 scenarios have some test; US2.2 and US2.3 only through a pure function; SC-001 (path within 1 s) has no test. Tests tracing to nothing: `quote.rs:115` (constant length). Claimed tests that do not exist: T012's pointer-state test and T022's reducer no-file test (finding 13).

## What was not audited

- The full workspace suite was not run. The disk went to 0 free bytes (linker `Bus error`, `No space left on device`) and the profile's 343 s suite could not be built. To get running I deleted `target-shared/debug/incremental` (860 MB regenerable cache, the recovery the profile names), which other worktrees share. Only the feature's targets were run (counts in Mutation results). Baseline `suite_baseline: green` at `cdc473ab` is the profile's, not re-observed.
- Mutation testing: no tool; the 12 deliberate mutants are a sample, not a score. Client-only mutants were run on `clipboard.rs`, `drops.rs`, and `session.rs`; none on `split_view.rs`, `daemon_sync.rs`, `terminal_pane.rs`, or `sandbox.rs` (`share`, fail-closed without a running sandbox).
- Coverage: none available.
- The `--features gui` test at `terminal_pane.rs:1923`, the `sandbox_real_*` acceptance suite, and anything needing a container runtime, display, real clipboard, zsh, fish, PowerShell, or Windows/macOS: not run.
- Git history is not squashed but bundles test and source, so ordering is unprovable, not disproven.
- Docs (`docs/user-guide/*`), `Cargo.lock`, and other features' commits in the PR range (040, 041, 550) were out of scope.
- Performance and SC-001's 1 s bound: no test, not assessed.
- No secrets or credentials were found in the tests read.

## Remediation (close unit, 2026-10-09)

The verdict above stands as audited: the feature was built test-after and no red was recorded at the time. T035–T042 then fixed the test-strength findings (no production change); `tdd/cycle-log.md` holds the evidence: a red and a green, proven with mutants, for T035, T036, T037, T039 (env var) and T040, and for the nested-project part of T041. T038 has none, and the text-wins and pointer-state parts of T041 are not built (T041 stays open).

Not proven: T038 (per-shell expected strings) has no mutant run, and the fish, zsh and PowerShell expectations were checked by hand (only bash is installed here). `MICOLD_REQUIRE_SHELLS=1` is not yet set in CI. The T012 pointer state has no separate test; the T040 move-then-drop test covers it. Finding 12 (no real OS drop/clipboard entry point, visual passes skipped) remains a known gap.
