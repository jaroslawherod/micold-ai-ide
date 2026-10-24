---
feature: 1260-bundled-monospace-font
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 12d38521 # short SHA audited
behaviors: 14
proven: 4
likely: 2
test_after: 3
no_test: 1
high_smells: 0
criteria_total: 10
criteria_covered: 10
mutation_score: unmeasured # no mutation tool; 5 deliberate mutants run, 5 caught (see below)
mutants_survived: 0
suite: targeted only. lib mono+terminal 143 passed; layout_apparatus 12 passed; packaging 9+24+6 passed; macos-bundle.test.sh 25 cases 0 failures. Full workspace gate not re-run (green at 12d38521 per the M3 gate).
---

# TDD Verification: Bundled monospace typeface

**Verdict: FAIL.** The tests that exist are strong and assert behavior, but three behaviors
(T006, T007, T022) were written together with, or after, the code they cover; the cycle log
itself says their red was "proven afterwards by mutation". The rubric fails a `TEST_AFTER`
behavior outright.

This audit was not independent of the repository's own session (it read the log, history and
files cold, but ran in the same autopilot close unit). Another process mutated the shared working
tree while this audit ran (see "What was not audited"), so the mutation section is thin.

## Test-first evidence

The feature has no `tdd/test-list.md`; tasks.md test tasks are the list.

| Behavior | Class | Evidence |
| --- | --- | --- |
| T006 bytes guard (`mono.rs` tests) | TEST_AFTER | `3f7edb28` adds `mono.rs` and its tests together; cycle-log.md:8 "red proven afterwards by mutation" |
| T007 `pin_monospace` (`mono.rs` tests) | TEST_AFTER | same commit; cycle-log.md:9 |
| T009 global font system resolves monospace | PROVEN | cycle-log.md:10 red; test and `test_support::renderer` change in one commit `3f7edb28` |
| T013a terminal cell fonts resolve to bundled faces | LIKELY | log red (cycle-log.md:11), but the test lands in `e5048827`, after T010 landed in `3f7edb28`: history cannot show the order |
| T013b cell width = face advance | NOT_APPLICABLE | characterization, passes first run (cycle-log.md:11) |
| T016 layout guard, Regular and Bold | PROVEN | log red before T018 (cycle-log.md:15); the test and T018 (`layout.rs` switching to `install_monospace`) land together in `56efcebc` (`3f7edb28` only mentions it in its message) |
| T017 one copy of Fira Mono | LIKELY | passes on first run; red shown with a temporary stray copy (cycle-log.md:16), nothing in history |
| T019 layout snapshot unchanged | NOT_APPLICABLE | characterization |
| T021 licence set in four bundles | PROVEN | red output recorded (cycle-log.md:24); test and manifests in one commit `b3cc6f16` |
| T022 `.iss` licence allowance | TEST_AFTER | red by mutation after the code (cycle-log.md:25) |
| T023 macOS bundle script | PROVEN | red recorded (5 failures, cycle-log.md:26); test and script in `b3cc6f16` |
| T011/T012 startup and showcase call sites | NO_TEST | glue; see finding 3 |
| T020 packaging helpers refactor | NOT_APPLICABLE | refactor, tests green before and after |
| T027 `.gitattributes` | NOT_APPLICABLE | config |

Existing tests that the range touched: none weakened. `layout_apparatus.rs` monospace guard was
rewritten and is stricter (compares face bytes, adds Bold, adds messages naming Fira Mono);
`deb_ships_no_service_units.rs` and `packaging_excludes_showcase.rs` only moved helpers into
`support/packaging.rs` and gained a `licences` argument with every old assertion kept;
`macos-bundle.test.sh` is additive. No skips, exclusions or threshold changes.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH (process, FAIL condition) | T006, T007, T022 are `TEST_AFTER`: tests written in one pass with the code, red only shown later by mutation. Fix: nothing can restore the order; record the mutation red as the accepted evidence and note it, or re-prove each test by reverting its source in a scratch run and recording the failing output | `crates/micold-client/src/ui/material/mono.rs:62`; `specs/1260-bundled-monospace-font/tdd/cycle-log.md:8,9,25` |
| 2 | MED | Cycle log claims T013a failed before T010, but history shows T010 (`3f7edb28`) in an earlier commit than the test (`e5048827`). Order not corroborated. (T016 was checked and is fine: test and T018 share `56efcebc`) | `crates/micold-client/src/ui/terminal.rs:812` |
| 3 | MED | No test pins the real entry points: deleting the `install_monospace` call from the app or showcase startup passes every test; only the manual visual pass covers FR-002/FR-003 in the shipped binary | `crates/micold-client/src/shell/startup.rs:88`; `crates/micold-client/src/showcase/main.rs:59` |
| 4 | LOW | `missing()` tests the macOS source path and destination on different lines, so a script copying a licence to the wrong directory passes this test; only `scripts/tests/macos-bundle.test.sh` catches it | `crates/micold-client/tests/packaging_ships_font_licences.rs:62` |
| 5 | LOW | FR-004 (fallback for glyphs Fira Mono lacks) is asserted only as "a Roboto face is still in the database"; no test shapes a character outside Fira Mono | `crates/micold-client/src/ui/material/mono.rs:187` |
| 6 | LOW | Redundant at the same level: family, fixed-pitch and bytes checks appear in both `mono.rs` tests and the `layout_apparatus` monospace guard | `crates/micold-client/src/ui/material/mono.rs:69`; `crates/micold-client/tests/layout_apparatus.rs:169` |

No HIGH smells from the catalogue: no assertion-free, tautological, vacuous or conditional
tests; failure messages name `Fira Mono` and state the rule, matching the repository's style.
`support/packaging.rs` reuses the existing packaging readers instead of copying them. Tests are
deterministic and fast (all targeted runs under a second).

## Mutation results

No mutation tool in the profile. Deliberate mutants, sample of 5 (one at a time, each restored with
`git checkout --` and the touched tests re-run green; final `git diff --stat` over crates, scripts
and packaging empty). Mutant 1 was applied while two audit sessions shared the tree; its result was
re-read and is consistent.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `mono.rs:45` `db.remove_face(id)` replaced by a no-op | T007 host copy removal (FR-003) | No | `pin_monospace_drops_a_host_copy_...` failed at `mono.rs:148` |
| `mono.rs:47` `set_monospace_family` dropped | T007/T009/T013a monospace pin | No | 3 failed: `pin_monospace_...`, `the_global_font_system_resolves_...` (`mono.rs:205`), `cell_fonts_resolve_...` (`terminal.rs:814`) |
| `Cargo.toml:190` rpm `PROVENANCE.md` asset deleted | T021 licence set | No | `every_release_bundle_ships_the_font_licence_set` failed naming the rpm and the file |
| `macos-bundle.sh:129` licence `cp` replaced by a no-op | T023 | No | `macos-bundle.test.sh`: 4 failures of 25 |
| `mono.rs:58` Bold `load_font` removed from `install_monospace` | T016 Bold guard (FR-006, SC-001) | No | `the_monospace_face_parses_and_is_the_face_that_measures` failed: "Fira Mono Bold ... resolved ... at weight 400" |

Not mutated: the `.iss` allowance (T022), the one-copy walk (T017; the log shows a manual stray-copy
red), and the `startup.rs`/`showcase/main.rs` calls. By construction the last pair would survive:
no test reaches the binary's startup path (finding 3).

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1-1.3 (transcript, diff, inline code) / FR-002, FR-011 | `mono.rs` global-system test; pre-existing call-site asserts (`code_block.rs:372`) | Partly: resolution through the test renderer, not the app |
| US1.4-1.5 (OS defaults only, host copy) / FR-003, FR-004 | `pin_monospace_...` on a fabricated database | Unit only |
| US2.1-2.3 / FR-005, FR-006, SC-001, SC-003 | `layout_apparatus` guard and one-copy test | Yes, via `install_monospace` |
| US3.1-3.2 / FR-008, SC-004 | `packaging_ships_font_licences`, `packaging_excludes_showcase`, `macos-bundle.test.sh` | Yes (text of the real manifests) |
| FR-007 terminal | `cell_fonts_resolve_...`, `cell_width_equals_...` | In-crate |
| FR-009, SC-002, SC-005 | `layout_snapshot` unchanged | Yes |
| FR-010 user guide | none (documentation is not read by tests) | n/a |

Untested criteria: none. Tests tracing to nothing: none.

## What was not audited

- A second audit session ran in the same working tree at the same time (it wrote this file and
  the Phase 7 tasks first; this pass corrected finding 2 and completed the mutation section). The
  tree was clean of source changes at the end of this pass.
- The `.iss` allowance (T022) mutant was not run.
- Coverage and mutation tooling: absent from the profile.
- Full workspace suite, clippy, the sandbox acceptance suite, Windows and macOS CI runners.
- Visual pass evidence (`visual-pass/`) and T028/T029, ticked in the working tree by another actor.
- Performance beyond the observed sub-second runs.
