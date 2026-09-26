---
feature: 032-untitled-session-labels
verdict: FAIL # 27 of 83 behaviors have no recorded red (see "Test-first evidence")
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: be954fb0
behaviors: 83 # 72 unit (U1-U72) + 11 acceptance (A1-A11)
proven: 0
likely: 49
test_after: 27
no_test: 0
not_applicable: 7
high_smells: 0
criteria_total: 24 # 11 US scenarios + 16 FRs - FR-002/FR-003/FR-012 are elaborated by multiple U-rows, counted once each
criteria_covered: 23 # FR-015 (user guide wording) has no automated test
mutation_score: unmeasured this run # scope would have been the feature's changed files; see "What was not audited"
mutants_survived: 0 observed # cycle-log's own self-reported deliberate mutants: ~14 applied across cycles 1, 3, 4; all reported killed
suite: 3438 passed, 0 failed, 6 ignored, workspace-wide (scripts/build-lock.sh cargo test --workspace)
---

# TDD Verification: A session the AI CLI never titled still gets a label (032)

**Verdict: FAIL.** 27 of the 83 listed behaviors — most of the `claude` first-turn parsing rules
(U24–U36, U46–U49) and four of the eleven acceptance scenarios (A1–A4), plus two more (A7, A8) —
have **no recorded red**, by the cycle log's own admission for the first group and by its narrative
for the second. The rubric's `TEST_AFTER` class and `FAIL` verdict condition both fire on "no red
recorded" regardless of order, and they fire here even though the team disclosed the gap themselves
and closed it with mutation testing afterward (Phase 4 evidence, not Phase 1/2 evidence).

This audit corrects the feature-commit range implied by the task: `41463b4a..HEAD` in two-dot git
syntax **excludes** `41463b4a`, but that commit ("derived session label kind, store, wire and
catalog seam") is itself feature 032's Phase 2 foundational commit (T001–T016) and had to be
included for the test-first analysis below to mean anything. The full commit range audited is
`41463b4a~1..HEAD` filtered to `(032)` subjects (18 commits, `41463b4a` through `be954fb0`).

## Test-first evidence

Every commit in this feature groups several behaviors' tests and implementation into one commit
("cycles grouped per task pair... not one behavior per build" — cycle-log.md, Cycle 1 notes,
explaining the shared-build-lock wait time). Git history therefore **never** independently
corroborates test-before-source ordering for any behavior here — the most it can show is
same-commit. Per the rubric, that caps every behavior with recorded red at `LIKELY`, not `PROVEN`,
which is why `proven: 0` above is not itself a defect: it is the ceiling this delivery style
imposes, and the cycle log is unusually rigorous about supplying the actual assertion failures
inside that ceiling.

| Class | Count | Behaviors |
| --- | --- | --- |
| `PROVEN` | 0 | — (see above: no commit in this feature separates a test's addition from its source) |
| `LIKELY` | 49 | Cycle 1: U1–U23, U44, U45, U56–U59 (29). Cycle 3: U63, A9 (2). Cycle 4: A5, A6, U37–U43, U50–U55 (15, incl. U53/U55 — see note below). Cycle 5: U69, A10 (2). Cycle 5a: U72 (1) |
| `TEST_AFTER` | 27 | Cycle 2, red lost entirely: A1–A4, U24–U36, U46–U49, U61, U62, U66, U67 (25). Cycle 3, no dedicated red despite being `example` kind: A7, A8 (2) |
| `NOT_APPLICABLE` | 7 | Formal `guard` rows: A11, U60, U64, U65, U68, U70, U71 |
| `NO_TEST` | 0 | — |

**The decisive finding (cycle-log.md, Cycle 2, lines 52–58, self-reported):**

> the per-behaviour red output of this cycle was lost. The session that ran it was killed by a rate
> limit mid-T022 and never wrote this entry; its tests and implementation arrived in one commit
> (`feat(032): label untitled claude sessions with their first typed turn`), so git history does
> not separate them either. ... Treat cycle 2 as **verified by mutation, not by recorded red** — a
> Principle I deviation, recorded here rather than papered over.

That commit is `98ebcd6f`. It adds `claude_first_turn` (the whole command/argument/follower parsing
rule, C3.1–C3.7), `ClaudeProvider::read_label`, the daemon's `discover_external_sessions` /
`record_recovered_names` label branches, and the four daemon acceptance tests A1–A4, in one commit
with no cycle-log red entry. Cycle 3's deliberate mutants (two, applied against the already-merged
code, restored with `git checkout --`) later demonstrated that `ai_cli_provider.rs` and
`untitled_session_labels.rs` would catch a "always return `None`" mutant and that
`first_turn_label.rs` would catch a "skip the record classifier" mutant — real evidence that these
tests discriminate **today**, but not evidence that they existed and failed before the
implementation, which is what `TEST_AFTER` is for. Per the mutant list (cycle-log.md, Cycle 3,
lines 100–105), four behaviors in this same commit — U30, U32, U33, U34 — were **not** killed by
either mutant, so even the retroactive mutation coverage of this commit is partial; those four
remain unverified by any mechanism beyond passing today.

**The second finding, smaller but structurally the same defect:** Cycle 3's own header calls
`A7–A9` "guards" (cycle-log.md line 67: "US2: a title still wins, and replaces a label (U63; U64,
U65, U71, A7–A9 as guards)"), but `test-list.md` marks only `U64`, `U65`, `U71` with `kind: guard` —
`A7`, `A8`, `A9` are all `kind: example`, which by the rubric's own convention implies a red is
expected. The cycle log's body text confirms no red was demanded for `A7`/`A8`
(`a_titled_session_is_never_given_a_label`, `an_observed_terminal_title_replaces_a_label_and_is_persisted`):
"Guards, passing on arrival ... A7, A8, U65: `record_session_name` / `record_session_label` already
enforce the precedence." `A9` is the exception inside that same sentence group — it is the *same
test function* as `U63` (`a_labelled_session_reads_the_title_its_records_gained`), and `U63` **does**
have a dedicated, quoted red (`left: 0` / `right: 1`, `untitled_session_labels.rs:588`) two lines
above. So `A9`/`U63` is correctly `LIKELY`; `A7` and `A8` are `TEST_AFTER` because nothing shows
them ever failing, and the discovery/recovery passes they exercise are new code in this feature
(not pre-existing behavior), so `NOT_APPLICABLE`'s "characterization of untouched code" carve-out
does not apply to them either.

**A defensible judgment call, stated so it can be checked:** three tests in Cycle 4
(`a_block_summary_and_a_file_with_neither_key_are_no_title` = U53,
`a_first_turn_past_the_read_bound_is_no_label` = U55, and an untracked test — see Traceability)
"passed on their first run" with no red, but were each immediately given a deliberate mutant in the
*same* cycle, killed, and restored (cycle-log.md, Cycle 4, lines 152–161). That is materially
different from Cycle 2's total evidence loss — it is a same-cycle substitute for red, not an
after-the-fact repair — so this report counts U53 and U55 as `LIKELY` rather than `TEST_AFTER`.
A stricter reading of the rubric (red literally required, no substitute) would move these two to
`TEST_AFTER` and drop `criteria_covered`/`likely` by nothing (both traced criteria still have other
`LIKELY` tests), only worsening `test_after` from 27 to 29.

## Findings

Ordered by severity, so finding 6 (MED) sits above finding 5 (LOW); the ids are the order they were
found in.

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | 25 behaviors (A1–A4, U24–U36, U46–U49, U61, U62, U66, U67) have no recorded red at all; the cycle log admits the evidence was lost to a rate-limit kill and the commit that carries them (`98ebcd6f`) does not separate test from source | `specs/032-untitled-session-labels/tdd/cycle-log.md:52-58`; commit `98ebcd6f` |
| 2 | HIGH | Of that same commit's mutation-based retroactive check, 4 behaviors (U30, U32, U33, U34) were not killed by either deliberate mutant applied — they have neither red evidence nor a demonstrated kill | `specs/032-untitled-session-labels/tdd/cycle-log.md:104-105` |
| 3 | HIGH | `A7`, `A8` are `kind: example` in `test-list.md` but received no dedicated red; the cycle log's own header mislabels them "guards" although the code path they exercise (the widened `recover_session_names` candidate filter) is new in this feature, not characterization of pre-existing behavior | `specs/032-untitled-session-labels/tdd/cycle-log.md:67,82-86`; `specs/032-untitled-session-labels/tdd/test-list.md` rows A7, A8 |
| 4 | MED | Three tests exist but are not referenced by any `test-list.md` row, so the list understates coverage in this direction: `a_last_record_still_being_written_is_not_read` and `a_copilot_log_with_nothing_typed_in_it_has_no_label` in `crates/micold-core/tests/first_turn_label.rs`, and `a_labelled_copilot_session_reads_the_name_its_workspace_file_gained` in `crates/micold-daemon/tests/untitled_session_labels.rs` | `crates/micold-core/tests/first_turn_label.rs:308,406`; `crates/micold-daemon/tests/untitled_session_labels.rs:905` |
| 6 | MED | The read-layer guard in `record_recovered_names` (`if !unlabelled { return None; }`) is untested: deleting it **survives** U64, because U64 asserts only the final label and a recovery count of 0, and `Catalog::record_session_label` refuses the second label on its own. The guard exists to avoid the `read_label` disk read per already-`Derived` session on every pass (its own doc comment; SC-006), and nothing observes the read | `crates/micold-daemon/src/state.rs:1355` (guard), `:1258` (its stated purpose); `crates/micold-daemon/tests/untitled_session_labels.rs:613` (U64) |
| 5 | LOW | FR-015 (the user guide must say what an untitled row reads) has no test tracing to it in `test-list.md`; it is a docs-only requirement satisfied by prose edits in commits `98ebcd6f`, `a725b721`, `76964d24`/`e2f479e9`, but nothing pins the wording against drift | `specs/032-untitled-session-labels/spec.md` FR-015; `docs/user-guide/worktrees-and-sessions.md` |

No test smells (assertion-free, tautological, vacuous, doubled-subject, over-mocked, or any other
`HIGH`/`MED` item from the rubric's catalogue) were found in any of the nine new/changed test files
read in full: `untitled_session_labels.rs`, `session_label_kinds.rs`, `session_name_round_trip.rs`,
`first_turn_label.rs`, `ai_cli_provider.rs`, `ai_cli_provider_seam.rs`, `copilot_provider.rs`,
`protocol_roundtrip.rs`, `session_title_sync.rs`. Every assertion carries a message stating the rule
being pinned, doubles are the profile's own `Fake*`/`Minimal*` helpers (never hand-rolled ad hoc),
fixtures are read from `tests/fixtures/first_turn/*`, and every test that touches process-global
environment (`CLAUDE_CONFIG_DIR`, `COPILOT_HOME`, `PATH`) holds an explicit mutex or a scoped
restore-on-drop guard. The live-PTY tests in `untitled_session_labels.rs` (US3) poll a condition
with `wait_until` rather than a fixed sleep, which is the right pattern for a real child process,
though they do make that file's wall time and occasional-flakiness risk higher than the rest of the
suite (real `cat`/`sh`/`powershell` processes, a 10 s poll ceiling per test, `kill()` cleanup).

## Mutation results

**Independent re-verification was not possible this run.** A deliberate mutant applied to
`crates/micold-core/src/first_turn.rs` (swapping the `args`/`name` priority in the command-turn
branch of `claude_first_turn`) was reverted on disk between the edit and the test run — `git status`
and `git diff` on the file showed no change afterward — and a follow-up `git status` was then denied
by the harness's auto-mode classifier as "Modify Shared Resources," confirming this worktree's
`target-shared` and git state are treated as shared with other concurrent sessions/builds (per this
repository's own documented build-lock sharing). Repeating the attempt risked colliding with
whatever process reverted the first one, so Phase 4 falls back to the cycle log's self-reported
mutants rather than fresh ones.

Self-reported mutants (cycle-log.md), all restored and the suite re-confirmed green per its own
account — treat these as `LIKELY`-tier evidence (self-reported, not independently re-run this
audit), same caveat as the red evidence above:

| Mutant | Behavior(s) | Survived | Judgment |
| --- | --- | --- | --- |
| `set_derived_label` without the `Pending` guard | U3, U4 | No | Killed, as reported |
| No empty-string check / label-first precedence / `label` without `skip_serializing_if` | U8, U9, U10 | No | Killed, as reported |
| Pi `read_label` returns `read_title` | U45 | No | Killed, as reported |
| `claude_first_turn` returns `None` for every prefix | A1–A4, U66, U67, U65 (7 of 25 Cycle-2 behaviors) | No | Killed, as reported — but only 7 of the 25 `TEST_AFTER` behaviors were exercised by this one mutant |
| Record classifier skipped (`ClaudeTurn::of` → always `Prompt`) | U24, U25, U26, U27, U28, U29, U35, U31, U36 (9 of 25) | No | Killed, as reported — combined with the mutant above, 16 of the 25 `TEST_AFTER` behaviors have *some* mutation evidence; U30, U32, U33, U34, U46–U49, U61, U62 (9 behaviors) have none |
| `read_yaml_scalar`'s `\|`/`>` guard dropped | U53 | No | Killed, as reported |
| `read_prefix`'s bound replaced with `.take(u64::MAX)` | U55 | No | Killed, as reported |
| `data.source` filter dropped in Copilot turn text | (untracked test, Finding 4) | No | Killed, as reported |

Scope: none of this was re-run by this audit; it is transcribed and cross-checked against the test
names for consistency (all named tests and behaviors exist and match `test-list.md`'s ids), not
independently executed.

## Close-phase addendum (2026-09-26)

Written after the report above, in the close PR that acts on it. Three things changed:

- **Findings 1–3 are NOT closed, and the verdict stands at FAIL.** The close phase tried twice to
  supply the missing mutation evidence and got none: the first attempt was lost to two concurrent
  instances of this audit colliding over mutants in one worktree, the second to a build that never
  came out from behind another worktree's lock before the file was restored. `cycle-log.md` *Cycle 6*
  records the attempt, the designed mutant set, and the ids still unproven — U30, U32, U33, U34,
  U46–U49, U61, U62, A7, A8 — so the gap is written down rather than inferred from a missing section.
  `tasks.md` T048–T050 stay open and are carried as ledger follow-ups. Cycle 6 did produce three
  results from reading the code: A7 is defended by three independent layers, so the prescribed single
  mutant would survive it; U47 is not killable by one small mutation; and finding 6 below is
  confirmed.
- **Finding 5 is not closed either, and now has a reason rather than an omission.** A gate pinning
  the guide's FR-015 wording was written and then removed: `documentation_is_not_read.rs` refuses any
  test that reads a page declared `micold-docs`, because CI skips the build for docs-only changes and
  such a test makes that skip unsound. The `.gitattributes` carve-out that would allow it — the repo
  carries six of them, five because a test reads the file — was declined — it would make every prose edit to the project's most-edited guide page run the full
  pipeline, for a LOW finding, and a gate over six sentences of prose fires on rewording. FR-015 is
  covered by `scripts/check-user-guide-updated.sh` (omission, not drift); the residual risk is a
  ledger follow-up. `test-list.md` records the conclusion where the U76 row briefly was.

- **Finding 6 is not closed, deliberately.** It is a genuine test-strength gap with no behavior
  behind it: the guard is present and correct, and the only way to observe the read it prevents is a
  read-counting provider. `DaemonState` builds its providers from the environment and has no
  injection seam (`tasks.md` *Daemon tests*), so closing finding 6 means adding a production seam
  for a test — new structure, no new requirement. It is recorded in `autopilot.md` under
  *Follow-ups not done* instead, and U64's row in `test-list.md` now says what it does not cover.
  Finding 4 is the only one closed in this PR.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1–US1.6 | A1–A6 | Yes (real `DaemonState`+`Catalog` over temp dirs, real provider file parsing) |
| US2.1–US2.3 | A7–A9 | Yes |
| US3.1–US3.2 | A10, A11 | Yes |
| FR-001 | A1, A2, U2, U61, U62 | Yes |
| FR-002 | A1, A5, U16–U43 (via first-turn parsing), U46, U59 | Yes |
| FR-003 | U16–U20 | No (unit-only; shaping has no acceptance-level assertion of the 80-char bound, though A2's fixtures are all under it) |
| FR-004 | A4, U5, U36, U67 | Yes |
| FR-005 | A7, U3, U8, U49, U57, U61, U63, U65 | Yes |
| FR-006 | A8, A9, A11, U6, U14, U60, U63 | Yes |
| FR-007 | A3, U4, U7, U56, U64 | Yes |
| FR-008 | U7 | No (unit-only) |
| FR-009 | A3, U10 | Yes |
| FR-010 | A10, U68, U69, U72 | Yes |
| FR-011 | U21–U23, U34, U35, U47, U58, U66 | Yes |
| FR-012 | A5, A6, U24–U43 | Yes |
| FR-013 | U71 | No (unit-only; a call-site scan, not an acceptance path) |
| FR-014 | U21, U43, U48, U55 | Yes |
| FR-015 | *(none)* | **Untested** — docs-only, no automated test |
| FR-016 | A6, U50–U53 | Yes |
| SC-001–SC-002, SC-004–SC-005 | A1, A2 | Yes (fixtures shaped on the reporter's data; the corpus probe is manual, see quickstart) |
| SC-003 | A7 | Yes |
| SC-006 | U70 | No (unit-only bound; SC-006's "no perceptible delay" is a quickstart eyeball check, not automated — `test-list.md` says so explicitly) |
| SC-007 | A10 | Yes |
| SC-008, SC-009 | A6, A5, U43, U55 | Yes |

Every `traces` reference in `test-list.md` was checked against the actual test files: all 72 named
functions (and the `derived_labels` submodule for U13–U15) exist verbatim, none are `#[ignore]`d.

**Untested criteria:** FR-015 (Finding 5).
**Tests tracing to nothing:** the three tests in Finding 4.

## What was not audited

- **Independent mutation testing.** See "Mutation results" above — an attempted deliberate mutant
  was reverted by something outside this session before it could be observed, and this worktree's
  git state was then flagged as a shared resource. This audit reports only the cycle log's
  self-reported mutants, unverified by a fresh run.
- **Windows- and macOS-specific code paths** (`register_titler`'s `#[cfg(windows)]` arm in
  `untitled_session_labels.rs`) were not exercised: this host is Linux.
- **The acceptance/sandbox-real-runtime suite** and the release-mode daemon half of it were not run;
  this feature does not touch sandboxed session spawning, so it was judged out of scope for this
  audit, not merely skipped.
- **SC-001, SC-005, SC-006 on the reporter's real project** are quickstart §B checks
  (`specs/032-untitled-session-labels/evidence/quickstart-b.md`), not automated tests; this audit did
  not re-run quickstart, only confirmed the evidence file exists and `tasks.md` marks T046/T040 done.
- **Coverage** was not measured (no `cargo-llvm-cov` in this workspace, per the stack profile).
- **Performance** (SC-006's "no perceptible delay") was not independently measured; it rests on the
  bound argument in `test-list.md`'s own scope note, not a timing test.
