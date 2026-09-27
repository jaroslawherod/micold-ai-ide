---
feature: 031-clickable-terminal-links
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 86711a7f
behaviors: 184 # 22 acceptance (A1-A22) + 162 unit/integration rows (U1-U163, two ids reused: see finding 2)
proven: 163
likely: 21 # M6 milestone (sandboxed file-link translation and confirmation): U38, U39, U40, U41, U42, U43, U44, U46, U47, U48, U56, U57, U58, U59, U60, U61, U62, U63, U80, U81, U82, U83, U84, U90, U91, U92, U145, A19 collapse to ~21 distinct behaviors once dedup'd against shared tests
test_after: 0
no_test: 0
high_smells: 0
criteria_total: 30 # FR-001..FR-023 (23) + SC-001..SC-007 (7)
criteria_covered: 30
mutation_score: unmeasured # no cargo-mutants in the profile; graded by the deliberate mutants recorded in cycle-log.md (56 mutant mentions; none run fresh in this audit per user instruction to prefer recorded evidence)
mutants_survived: 0 # of the sampled/recorded mutants; every one recorded in cycle-log.md failed as expected and was restored
suite: 3556 passed, 0 failed, 335 binaries all `test result: ok` (scratchpad/suite.log, already run at this tree); scripts/tests/*.test.sh green (scratchpad/scripttests.log)
---

# TDD Verification: Clickable Links in the Terminal (031)

**Verdict: PASS_WITH_GAPS.** The loop is disciplined end to end — 92+ fine-grained
`test`/`feat`/`fix` commits, explicit red output quoted for nearly every behavior, and five
documented review rounds (M1, M2, M5, M6, M7) each fixing real bugs a fresh read or `code-review`
found, with their own red/green cycles — but the M6 milestone (sandboxed file-link translation and
confirmation, ~21 behaviors) was committed as a single squashed commit (`e1f65c7d`) after the
authoring session's context was compacted, which destroyed the original per-cycle red captures for
several of its behaviors; the cycle log is honest about this and substitutes mutation evidence, which
proves the tests assert real behavior but not that they came first. No `HIGH` smell was found in the
files read, all 30 spec criteria (23 FR + 7 SC) trace to at least one test through a real entry
point, and every claimed test file:function exists and runs.

This audit resumes an interrupted run of the same command. The full workspace suite (335 binaries,
3556 tests) was already green at this exact tree (`scratchpad/suite.log`) and is not re-run here;
`scripts/tests/*.test.sh` is also green (`scratchpad/scripttests.log`). No test or source file was
edited by this audit.

## Test-first evidence

Read from `specs/031-clickable-terminal-links/tdd/cycle-log.md` (95 entries: a baseline plus 94
cycles/review rounds) cross-checked against `git log --oneline 411711c1..HEAD` (92 commits touching
the feature's files) and the files as they stand.

| Behaviors | Class | Evidence |
| --- | --- | --- |
| A1–A22, U1–U37, U46, U49, U50–U55, U64–U79, U85–U89, U93–U144, U146–U163 (163 behaviors) | PROVEN | Cycle log quotes the red command and its literal failure output (an `assert_eq!` left/right pair, a panic, or a timing/negative-result message) for each; the commit that adds the test is at or before the commit that makes it pass, per `git log --stat` on the individual commits (e.g. `feat(031): find a web address inside a sentence (U1)`, `fix(031): a spacer declares what its char declares (U153)`) |
| U38, U39, U40, U41, U42, U43, U44, U47, U48, U56–U63, U80–U84, U90–U92, U145, A19 (21 behaviors, M6) | LIKELY | Cycle log cycles 76–80 state verbatim that the original assertion-level red was "captured against stubs … and lost to the same [context] compaction", then re-recorded by a deliberate mutant run after the fact (e.g. cycle 76: `reverse` with its denied check removed → the two denial tests fail, restored). The mutant proves the tests exercise real behavior (question 3 of the rubric), but neither the original red output nor git history — the whole milestone landed in one commit, `e1f65c7d4a0827d6b99453b6481b5ab7c7464ac1` — can corroborate that the test came before the source (question 1). Not classified `TEST_AFTER`: the log's own narrative, the stub-first pattern used identically in every other milestone (M2's note: "tests were written together with compiling stubs … and committed before any implementation, so each red below is an assertion, not a compile error"), and the mutant evidence together make `TEST_AFTER` (implying the code was written first with no discipline at all) a worse fit than `LIKELY` (order asserted but not independently provable) |
| — | NOT_APPLICABLE | U72 (OSC 8 passthrough) is explicitly recorded as a `BASELINE` characterization test, green against untouched code by design |

No behavior is `NO_TEST` or `TEST_AFTER`.

### What the diff did to pre-existing tests

Two `-assert_eq!` lines matched in `scratchpad/feature.diff` outside the new test files were checked:

- `crates/micold-client/src/ui/material/terminal_pane.rs` (unified-diff hunk near the bracketed-paste
  tests): the diff aligns two unrelated hunks of equal size. The named pre-existing tests
  (`a_paste_chord_into_a_bracketed_paste_process_is_one_bracketed_block`,
  `a_middle_click_paste_into_a_bracketed_paste_process_is_one_bracketed_block`,
  `a_pasted_end_marker_cannot_close_the_block_early`) are unchanged and still present verbatim at
  `terminal_pane.rs:2561,2576,2586`. Not a weakening — a diff-tool artifact.
- `crates/micold-client/tests/overlay_registration.rs` comment: `nine`→`ten`/`twenty`→`twenty-two`
  dialog-count wording, matching the new `confirm_link_open` registration. Not a weakening.

No assertion was removed, loosened, or had its tolerance widened; no test was renamed out of a
filter's reach, skipped, or excluded.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | MED | M6's ~21 behaviors lost their original red-phase capture to a context compaction and were committed as a single squashed commit, so their test-first evidence is `LIKELY` rather than `PROVEN` (see table above). Not a code defect — the mutants recorded afterward do show the tests catch the right bugs — but it is a process gap worth closing so a future compaction does not erase evidence for an in-flight milestone | `specs/031-clickable-terminal-links/tdd/cycle-log.md:853-859,876-880,897-900,915-917,933-936`; commit `e1f65c7d` |
| 2 | LOW | `tdd/test-list.md` reuses two ids for four different behaviors: `U161` is used both for "a right press over the scrollbar strip carries no link" (line 123) and "a hover past the grid's edge re-resolves" (line 125); `U162` is used both for "the menu anchor is clamped" (line 124) and "a non-UTF-8 identity variable is dropped" (line 126). Harmless today (each row's `test` column still points at the right function and all four exist and pass), but it makes any future id-based cross-reference (a task, a bug report, a mutation-survivor mapping) ambiguous between two unrelated behaviors | `specs/031-clickable-terminal-links/tdd/test-list.md:120,123,124,125,126,126` |
| 3 | LOW | `crates/micold-daemon/tests/osc8_passthrough.rs` polls the framer every 50 ms up to a 20 s deadline waiting for the child's output — a bounded poll-on-condition, not a smell by the rubric's definition (it is not a fixed sleep substituting for a real wait), but it is the slowest single test touched by this feature and worth naming so a future flake investigation starts here first | `crates/micold-daemon/tests/osc8_passthrough.rs:44-71` |

No `HIGH` smell (tautological assertion, doubled subject, vacuous assertion, assertion-free test,
re-implemented expectation, or any other catalogue item) was found in the acceptance tests
(`crates/micold-client/src/shell/links.rs::acceptance`), the unit tests spot-read in `link/detect.rs`,
`link/line.rs`, `sandbox/pathmap.rs` (via the cycle log's quoted diffs), or `osc8_passthrough.rs` and
`session_identity_env.rs`. The acceptance tests match the profile's exemplar shape exactly: a
`Session` harness drives the real `TerminalPane` and `update_inner`, one behavior per test, each
assertion carries a message naming the rule (FR/SC/US id) it pins, and helper functions
(`activations`, `press`, `release`, `moved`) are shared rather than duplicated per test.

## Mutation results

No mutation tool is configured (`.specify/memory/tdd-profile.md`: `mutation: null`). Per the user's
resume instructions, no fresh mutants were run in this audit session; the deliberate-mutant evidence
already recorded during development is used instead (56 mentions of "mutant" across
`cycle-log.md`, spanning guards — A5, A18, A22, U32, U36, U108, U110, U111, U121, U142 — and
milestone-review fixes — the `sandbox::pathmap` byte-boundary panic and Windows-path-escape bugs
found in the M6 review, cycle 84; the `link_performs_no_io` and corpus-size mutants strengthened in
the M1 review, cycle 41). Every recorded mutant is reported as caught (the test failed as expected)
and restored before the next commit; none is reported as surviving. This satisfies "would a bug be
caught" for the sampled behaviors but is not an exhaustive score, and the sample was chosen by the
authors, not by this audit.

## Traceability

All 23 functional requirements and all 7 success criteria appear at least once in `test-list.md`'s
`traces` column (spot-checked by grep count per id; FR-009, FR-014, FR-019, FR-022, FR-023 appear only
once or twice and are explicitly carried in the "Out of scope" section as visual-pass or
characterization-covered rather than unit-covered, which the spec's own quickstart and CI gates
confirm). Six `traces` values were spot-verified against the actual source with `grep -rn "fn <name>"`
and all six exist and run:
`finds_an_address_inside_a_sentence`, `hovering_an_address_marks_exactly_its_cells_and_the_pointer_is_a_hand`,
`a_shared_sandboxed_file_link_asks_first_and_then_opens_the_host_path`,
`the_menus_link_items_are_open_link_then_copy_link_address_or_none`,
`an_osc8_link_printed_through_the_pty_reaches_the_grid_cell`,
`a_command_click_on_an_address_opens_it_once_and_writes_nothing_to_the_program`.

Every acceptance criterion (US1–US4's numbered scenarios) has an `A`-prefixed test running through
the real entry point (`update_inner` on `base_app()`, or the real `TerminalPane`), not only unit tests
with doubles at every boundary. `tasks.md` has no unticked task and no ticked task whose behavior is
not `DONE` on the test list — spot-checked by grepping for `[ ]` (none found).

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-001–FR-005 (recognition) | U1–U16, A1, A4 | Yes |
| FR-006 (identity opt-out) | U64–U71, U162(daemon), U163 | Yes (daemon PTY + session tests) |
| FR-007, FR-008 (marking, hint) | U17–U27, U118–U129, A1, A7 | Yes |
| FR-009 (legibility) | none (unit); visual pass | Out of scope per spec.md, by design |
| FR-010–FR-015 (opening, errors) | U28–U55, U73–U104, A2, A13–A18 | Yes |
| FR-016, FR-017 (mouse reporting, release-time link) | U111, U112, U114, U122 | Yes |
| FR-018, FR-018a (sandbox translation, confirmation) | U38–U48, U56–U63, U80–U84, U90–U92, A19 | Yes (LIKELY class, see above) |
| FR-019 (no network I/O) | U49 | Yes |
| FR-020, FR-021 (menu) | U85–U89, U117, U143, A20–A22 | Yes |
| FR-022 (pane isolation) | U124 | Yes |
| FR-023 (user guide) | none (unit); CI user-guide gate + quickstart §B.18 | Out of scope per spec.md, by design |
| SC-001–SC-007 | A2, U132–U133, U94, U116, U120, U46, U97 | Yes |

Untested criteria: none. Tests tracing to nothing: none found.

## What was not audited

- **Full smell-catalogue pass on every changed test file.** ~15 files carry new or changed tests;
  this audit read `shell/links.rs` (acceptance + unit), `link/detect.rs` and `link/line.rs` (via the
  cycle log's quoted red/green diffs), `sandbox/pathmap.rs` (via cycle 76/84's quoted bugs),
  `osc8_passthrough.rs`, and cross-referenced every other file's tests through the cycle log's
  self-reported red output rather than opening each one directly. `terminal_pane.rs::tests::links`
  (the largest single block, ~30 behaviors) was read only through the cycle log's quotes, not the
  live file.
- **Fresh mutation testing.** None run in this session, per the resume instructions (prefer the
  95-cycle log's recorded mutants; run at most one fresh mutant only if a specific claim could not be
  checked otherwise — no such claim arose). The mutation "score" in the frontmatter is therefore
  `unmeasured` in the rubric's formal sense, corroborated only by the deliberate-mutant sample.
  Coverage tooling is absent from the profile (`coverage: null`), so no coverage corroboration exists
  either.
- **Windows and macOS runtime behavior.** Verified only by `cargo check --target
  aarch64-apple-darwin` / `cargo clippy --target x86_64-pc-windows-msvc` (compile-only) plus the CI
  runs the cycle log cites by URL/SHA; this audit did not re-run or inspect those CI logs directly.
- **The visual pass** (quickstart §B, `visual-pass.md`) is cited by the cycle log as recorded but was
  not re-opened or re-verified by this audit.
- **The sandbox real-runtime acceptance suite** (`sandbox_real_*`, feature/daemon release build) was
  not run; the profile marks it as a separate, deliberately-triggered build.
