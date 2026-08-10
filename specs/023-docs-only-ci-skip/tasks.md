---

description: "Task list for 023-docs-only-ci-skip"
---

# Tasks: Documentation-Only Changes Skip the Build

**Input**: Design documents from `/specs/023-docs-only-ci-skip/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (NON-NEGOTIABLE), every unit of logic here lands Red first.
Three units have tests: the documentation-set declaration, the classifier, and the two
source-scanning gates. Workflow YAML carries no decision of its own — every branch it takes is
`needs.classify.outputs.docs_only`, produced by the tested script — so it is validated by the
recorded [quickstart.md](./quickstart.md) Part B procedure, per Principle I's glue exception.

**Documentation**: This feature is developer-facing, so Principle VII is satisfied by
`docs/development/ci-pipeline.md` shipping in the same change (FR-020), not by a user-guide page.

**Cross-platform**: The two gates run under `cargo test -p micold-core --all-targets`, which CI
already runs on all three platforms. The classifier is Linux-only by design — see plan.md's
Complexity Tracking.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US5)
- Include exact file paths in descriptions

## ⚠️ Read this before starting

**Ordering is a safety property here, not a preference.** The default branch's ruleset requires
four status checks by job name, with no bypass for anyone (research §R1). Two orderings will wedge
the repository:

1. Making `lint` / `test` skippable **before** the ruleset switch → a documentation-only pull
   request loses required checks and can never merge.
2. Switching the ruleset **before** a run has produced `ci complete` → *every* pull request waits
   forever on a context nothing emits.

Phase 2 therefore lands the gate and performs the switch **before** any job is allowed to skip.
Do not reorder it. If something goes wrong mid-phase, the rollback is one `PUT` (research §R13).

---

## Phase 1: Setup (baselines that cannot be captured later)

**Purpose**: Record the "before" state. Once the pipeline changes, these are unrecoverable.

- [X] T001 [P] Record the job list and wall-clock of the most recent code-affecting CI run into `specs/023-docs-only-ci-skip/ci-pass.md` under a "Baseline (pre-feature)" heading — this is the comparison SC-004 is measured against
- [X] T002 [P] Save the current ruleset to `specs/023-docs-only-ci-skip/ruleset.before.json` via `gh api repos/{owner}/{repo}/rulesets/19840981`, and confirm it lists the four legacy contexts — this file is the rollback source (FR-016, research §R13)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The single declaration, the aggregate gate, and the ruleset switch. Everything else
rests on these.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete, and the tasks inside it
are strictly ordered from T007 onward.

- [X] T003 Write failing `scripts/tests/documentation-set.test.sh` asserting `git check-attr micold-docs` verdicts for the paths in [quickstart §A1](./quickstart.md) — `set` for `docs/`, `specs/`, `README.md`, `CLAUDE.md`, `LICENSE`, `dialog-list.png`; `unset` for `CHANGELOG.md`; `unspecified` for `crates/`, `.github/`, `Cargo.toml`, `scripts/`, `assets/`. Confirm it FAILS (no `.gitattributes` yet)
- [X] T004 Create `.gitattributes` at the repository root with the declaration from [data-model](./data-model.md#entity-documentation-set), `/CHANGELOG.md -micold-docs` last. Confirm T003 passes (FR-003, FR-004)
- [X] T005 [P] Write failing `crates/micold-core/tests/ci_gate_covers_every_job.rs`: parse top-level job ids out of `.github/workflows/ci.yml` by indentation, assert every one except the gate appears in the gate's `needs:` list, fail naming any that does not. Confirm it FAILS (no gate job yet) (FR-015, research §R12)
- [X] T006 Add the `ci complete` job to `.github/workflows/ci.yml` per [contracts/required-checks.md](./contracts/required-checks.md): `name: ci complete`, `needs: [lint, test, assertions, docs]`, `if: always()`, failing on any covered result of `failure` or `cancelled` and ignoring the advisory `assertions`. Confirm T005 passes (FR-014)
- [X] T007 Add a "shell tests" step to the `lint` job in `.github/workflows/ci.yml` running every `scripts/tests/*.test.sh`, so the shell-side suites are enforced in CI and not only locally
- [X] T008 Push the branch, open the pull request, and confirm the new context reports: `gh pr checks --json name --jq '.[].name' | grep -Fx 'ci complete'`. **Do not proceed until this prints** (FR-016, quickstart §B0 step 2)
- [X] T009 Switch the ruleset — replace the four legacy contexts with the single `ci complete` (keeping `integration_id: 15368` and carrying every other rule through the `PUT`), using the exact commands in [research §R13](./research.md) (FR-013, FR-016)
- [X] T010 Verify the switch per [quickstart §A5](./quickstart.md): the ruleset lists exactly `ci complete`, and this pull request is still mergeable. If not, roll back with the saved `ruleset.before.json` before continuing

**Checkpoint**: The merge gate is decoupled from the pipeline's internal shape. Jobs may now be
skipped without making anything unmergeable.

---

## Phase 3: User Story 1 - A documentation-only pull request merges without a build (Priority: P1) 🎯 MVP

**Goal**: A pull request that touches only prose skips all lint, build and test work, settles green
in minutes, and merges — with the build jobs honestly reported as skipped.

**Independent Test**: Open a pull request whose changed files are all under `specs/` or `docs/`.
No compile step executes, no macOS or Windows runner starts, `ci complete` is green, and the pull
request is mergeable.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and observe them FAIL before implementing.

- [X] T011 [US1] Write failing `scripts/tests/classify-change.test.sh` covering every case in the [classifier contract](./contracts/classify-change.md#test-cases) — docs-only, specs-only, `README.md`, `CHANGELOG.md`, mixed, both commit orders, workflow-comment-only, deleted documentation file, deleted source file, path containing a space, empty diff, unresolvable base, and `FORCE_FULL_CI=1`. Each case builds its own throwaway repository. Confirm it FAILS (no script yet)

### Implementation for User Story 1

- [X] T012 [US1] Implement `scripts/classify-change.sh` per the [contract](./contracts/classify-change.md): `git diff --name-only -z "<base>...<head>"` with `core.quotePath=false`, classification by `git check-attr micold-docs --stdin`, `docs_only=` / `reason=` on stdout, offending paths on stderr, and every failure path landing on `docs_only=false`. Confirm T011 passes (FR-001, FR-002, FR-005, FR-006, FR-007)
- [X] T013 [US1] Add the `classify` job to `.github/workflows/ci.yml` — `ubuntu-latest`, `actions/checkout@v4` with `fetch-depth: 0`, then an explicit `git fetch --quiet origin "${{ github.base_ref }}"` before classifying (the `assertions` job at `ci.yml:100-104` does exactly this *despite* `fetch-depth: 0`, because checkout does not create `origin/<base>` on a `pull_request` run) — running `scripts/classify-change.sh` with the pull-request or push base as appropriate, exporting `docs_only` as a job output — and add `classify` to the `ci complete` job's `needs:` (T005's gate enforces this)
- [X] T014 [US1] Add `needs: classify` and `if: needs.classify.outputs.docs_only != 'true'` to the `lint`, `test` and `assertions` jobs in `.github/workflows/ci.yml`. Leave every step inside them untouched, leave `runs-on: ${{ matrix.os }}` alone, and leave `docs` unconditional (FR-008, FR-009, FR-011, FR-019)
- [X] T015 [US1] Make the `classify` job write the verdict, the reason, and any offending paths to `$GITHUB_STEP_SUMMARY`, so the run states which path it took without opening a job log (FR-018)
- [X] T016 [US1] Write `docs/development/ci-pipeline.md` covering the classification, the documentation set and where it is declared, the aggregate gate and why `if: always()` is load-bearing, and how to read a skipped run — including that `ci complete` green on a documentation-only run means "nothing needed building" (FR-020)
- [X] T017 [US1] Add `test -f docs/development/ci-pipeline.md` to the "Required developer docs exist" step of the `docs` job in `.github/workflows/ci.yml`, so Principle VII stays enforced for this feature's own document
- [X] T018 [US1] Run [quickstart §B1](./quickstart.md) live on a prose-only pull request and record run id, job list and wall-clock in `specs/023-docs-only-ci-skip/ci-pass.md`, confirming the `classify` reason is a path count and **not** `base ref unavailable` (SC-001, SC-002, SC-003, SC-006, FR-012, FR-017)

- [ ] T019 [US1] Verify the documentation-only **push to `main`** path live per [quickstart §B1a](./quickstart.md) (US1 scenario 4): land a prose-only commit on `main`, confirm the same three jobs run, and confirm the classifier resolved its base from `github.event.before` rather than a pull-request base. Check both degenerate cases the push path has and the pull-request path does not — an all-zero `before` (new branch) and a `before` that no longer exists (force push) — fall back to code-affecting. Record in `ci-pass.md` (FR-005, FR-006)

**Checkpoint**: MVP delivered. Documentation-only pull requests are cheap, honest, and mergeable.

---

## Phase 4: User Story 2 - A change that touches code still gets the full pipeline (Priority: P1)

**Goal**: Nothing was quietly lost. Every job and platform that ran before still runs, and the
aggregate gate fails when any of them fails.

**Independent Test**: Open a pull request that changes one `.rs` file and several documentation
files; diff its job list against the T001 baseline.

- [ ] T020 [US2] Run [quickstart §B2](./quickstart.md) live: a pull request changing one `.rs` file and several documentation files runs `lint`, three real `build + test` legs on their own operating systems, `assertions` and `docs`, plus `classify` and `ci complete`. Diff against T001's baseline — no job lost, no platform lost — and record in `ci-pass.md` (FR-011, SC-004)
- [ ] T021 [US2] Verify ordering-independence live: push the `.rs` change and the documentation change as separate commits, in each order, and confirm both classify as code-affecting (US2 scenario 2, FR-005)
- [ ] T022 [US2] Establish the fork-pull-request behaviour and record it: open a pull request from a fork of this repository and confirm the classifier either classifies correctly or falls back to code-affecting — it must never error the run. If a fork pull request is not practical to arrange, reason it through against the `classify` job's checkout and token, reach a stated conclusion, and record forks as out of scope in `docs/development/ci-pipeline.md` with the fail-safe rationale (spec Edge Cases, FR-006)
- [ ] T023 [US2] **On a scratch branch**, run [quickstart §B2a](./quickstart.md): break one platform deliberately (a `compile_error!` behind `#[cfg(target_os = "windows")]`), confirm `build + test (windows-latest)` fails, `ci complete` **fails naming it**, and the pull request is not mergeable. Delete the scratch branch afterwards — do not do this on the feature branch that carries the ruleset switch. This is the proof the gate is a gate (US2 scenario 5, FR-014)
- [ ] T024 [US2] **On a scratch branch**, cancel a run mid-flight (`gh run cancel`) and confirm `ci complete` reports failure, not success. `if: always()` exists for exactly this case, and a gate that goes green on a cancelled run is a gate anyone can bypass by cancelling (spec Edge Cases, FR-014)
- [ ] T025 [US2] **On a scratch branch**, verify the coverage gate bites: add a throwaway job to `.github/workflows/ci.yml`, leave it out of `ci complete`'s `needs:`, confirm `ci_gate_covers_every_job` fails naming it, then delete the branch (US2 scenario 6, FR-015)

**Checkpoint**: The fast path costs nothing on the safe path, and the gate cannot be papered over.

---

## Phase 5: User Story 3 - Documentation still has a gate of its own (Priority: P2)

**Goal**: Skipping the build did not skip the one check whose whole job is to notice a missing
document.

**Independent Test**: On a documentation-only branch, delete a required document; the run must fail.

- [X] T026 [US3] Confirm by inspection that the `docs` job in `.github/workflows/ci.yml` carries no `needs: classify` and no classification `if:` — it must run identically under both classifications (FR-010)
- [ ] T027 [US3] Run [quickstart §B3](./quickstart.md) live: on a documentation-only branch delete one required user-guide document, confirm the run fails and the pull request is not mergeable, restore the file, and record in `ci-pass.md` (US3 scenario 1, SC-005)

**Checkpoint**: The documentation gate survives the fast path.

---

## Phase 6: User Story 4 - Anyone can force the full pipeline (Priority: P3)

**Goal**: A misclassification is an inconvenience, not a blocker.

**Independent Test**: Apply the `full-ci` label to a documentation-only pull request and confirm the
full pipeline runs.

- [X] T028 [US4] Add `labeled` to the `pull_request` trigger types in `.github/workflows/ci.yml`, and set `FORCE_FULL_CI` on the `classify` step from `contains(github.event.pull_request.labels.*.name, 'full-ci')`. The script side is already covered by T011's `FORCE_FULL_CI` case (FR-021, research §R9)
- [X] T029 [P] [US4] Create the `full-ci` label in the repository with a description pointing at `docs/development/ci-pipeline.md`
- [X] T030 [P] [US4] Document the escape hatch in `docs/development/ci-pipeline.md` — how to apply it, and why removing the label is deliberately not a trigger (FR-020)
- [ ] T031 [US4] Run [quickstart §B4](./quickstart.md) live: apply the label to a documentation-only pull request, confirm a *fresh* run starts (not a re-run), the full pipeline executes on all three operating systems, and `ci complete` reports its real outcome. Record in `ci-pass.md` (FR-022)

**Checkpoint**: The classification can be overridden by anyone, without faking a code edit.

---

## Phase 7: User Story 5 - The exemption cannot quietly rot (Priority: P2)

**Goal**: The skip's precondition is checked by the build, and the constitution says what the
pipeline does.

**Independent Test**: Add a test that reads a file under `docs/`; the suite must fail. Read the
constitution's quality gates against the pipeline and find no disagreement.

### Tests for User Story 5 (MANDATORY — Constitution Principle I) ⚠️

- [X] T032 [US5] Write failing `crates/micold-core/tests/documentation_is_not_read.rs` with an **empty** allowlist: scan every `.rs` file under `crates/` (plus any `build.rs`) for string literals that resolve, relative to the repository root, to an existing path carrying `micold-docs` via `git check-attr`; fail naming file, literal and the exemption at stake. Confirm it FAILS on the two known fixture literals (FR-024, FR-025, research §R7)

### Implementation for User Story 5

- [X] T033 [US5] Add the two allowlist entries with written reasons — `crates/micold-core/tests/typeahead_corpus.rs` / `docs/user-guide` (fixture branch name) and `crates/micold-core/tests/submodule_failure_detail.rs` / `README.md` (file created inside a temporary repository the test builds) — and make a stale entry that matches nothing fail the gate too. Confirm T032 passes ([data-model](./data-model.md#entity-honesty-allowlist))
- [X] T034 [US5] Verify the gate bites per [quickstart §A3](./quickstart.md): add a temporary test that reads `docs/user-guide/settings.md`, confirm `mise run test-core` fails naming it, then revert (SC-008)
- [X] T035 [US5] Amend `.specify/memory/constitution.md` 1.5.0 → 1.6.0 in **three** places — Principle VI's CI bullet (L321), the TDD gate (L394), and the Cross-platform gate (L397) — scoping each to changes able to affect what is built, linted, packaged or tested, naming the documentation-only exemption once in full (declaration location plus the `documentation_is_not_read` check that enforces its precondition), and prepending a Sync Impact Report recording the MINOR rationale and why all three had to move together. **The exact replacement text is drafted in [constitution-amendment.md](./constitution-amendment.md)** (FR-023, FR-026, research §R11)
- [X] T036 [US5] Determine whether `.specify/templates/plan-template.md`'s Principle VI line ("CI covers all three") is left imprecise by T035 and update it if so — drafted replacement in [constitution-amendment.md](./constitution-amendment.md) — recording the finding either way in the Sync Impact Report
- [X] T037 [US5] Run [quickstart §A6](./quickstart.md): read the amended quality gates against [contracts/required-checks.md](./contracts/required-checks.md)'s job behaviour matrix and confirm no case the text forbids and the pipeline permits (SC-009)

**Checkpoint**: The exemption is enforced by the build and stated in the governance text.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T038 [P] Measure and record SC-007 per [quickstart §B5](./quickstart.md): the share of recent merged pull requests that would classify as documentation-only, excluding those whose only Markdown change is `CHANGELOG.md`. Put the number in `docs/development/ci-pipeline.md`
- [X] T039 [P] Link `docs/development/ci-pipeline.md` from `docs/README.md` so it is discoverable alongside the other developer documents
- [ ] T040 Run the whole of [quickstart Part A](./quickstart.md) (§A1–A6) on a clean checkout and record the results
- [ ] T041 Confirm the full workspace suite is green on Linux, macOS and Windows on a code-affecting run (Principle VI), and that both new gates ran on all three
- [ ] T042 After the feature has been live for a few pull requests, decide whether to retire `specs/023-docs-only-ci-skip/ruleset.before.json` or keep it as the recorded rollback; note the decision in `ci-pass.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies. Must happen before anything changes the pipeline — its
  whole purpose is capturing the "before".
- **Foundational (Phase 2)**: Depends on Setup. **BLOCKS every user story.** Internally ordered:
  T003 → T004, T005 → T006, then T007 → T008 → T009 → T010 strictly in sequence.
- **US1 (Phase 3)**: Depends on Phase 2. Cannot start earlier — T014 makes required-today jobs
  skippable, which is safe only after the ruleset switch (T009).
- **US2 (Phase 4)**: Depends on US1 (it verifies that US1 took nothing away). T025 additionally
  depends on T005/T006.
- **US3 (Phase 5)**: Depends on US1 (it verifies the `docs` job survived the skip machinery).
- **US4 (Phase 6)**: Depends on US1 (the label feeds the classifier US1 built).
- **US5 (Phase 7)**: Depends only on Phase 2 (T004's declaration). **Can run in parallel with
  US1–US4** — it touches `crates/` and `.specify/`, not `ci.yml`.
- **Polish (Phase 8)**: Depends on all stories.

### User Story Dependencies

- **US1 (P1)** — the MVP. Everything user-visible about this feature is here.
- **US2 (P1)** — verification-only; needs US1 to exist to verify it took nothing away.
- **US3 (P2)** — verification plus one inspection; needs US1.
- **US4 (P3)** — additive; needs US1's classifier.
- **US5 (P2)** — genuinely independent of US1–US4 after Phase 2.

### Within Each User Story

- Tests are written and observed failing before implementation (Principle I).
- `ci.yml` is edited by one task at a time — T013, T014, T017 and T028 all touch it and must be
  sequential, never `[P]`.
- Live verification tasks come last in their phase and are recorded in `ci-pass.md`.
- **T023, T024 and T025 deliberately break things** — a failing platform, a cancelled run, an
  uncovered job. All three run on **scratch branches**, never on the feature branch that carries
  the ruleset switch, and the branches are deleted afterwards.

### Parallel Opportunities

- T001 and T002 (different files, both read-only against the repository).
- T005 with T003/T004 — the coverage gate touches `crates/`, the declaration touches
  `.gitattributes`.
- All of Phase 7 (US5) with Phases 3–6, by a second person: different files entirely.
- T029 and T030 within US4; T038 and T039 within Polish.
- **Not parallel**: anything touching `.github/workflows/ci.yml` (T006, T007, T013, T014, T017,
  T028), and every task in the T007→T010 switch sequence.

---

## Parallel Example: US5 alongside the MVP

```bash
# Person A — Phase 3 (US1), editing ci.yml and scripts/
Task: "Implement scripts/classify-change.sh"
Task: "Add the classify job to .github/workflows/ci.yml"

# Person B — Phase 7 (US5), editing crates/ and .specify/
Task: "Write failing crates/micold-core/tests/documentation_is_not_read.rs with an empty allowlist"
Task: "Amend .specify/memory/constitution.md 1.5.0 -> 1.6.0"
```

---

## Implementation Strategy

### MVP First (Phases 1–3)

1. Phase 1 — capture the baselines while they still exist.
2. Phase 2 — land the gate, switch the ruleset. **This is the risky phase; do it in order.**
3. Phase 3 — the classifier and the skipping.
4. **STOP and VALIDATE**: quickstart §B1 on a real prose-only pull request.
5. At this point the feature delivers its whole point and can ship.

### Incremental Delivery

1. Phases 1–2 → merge gate decoupled, nothing else changed, nothing skipped yet. Safe to stop here.
2. Phase 3 → documentation-only pull requests are cheap. **MVP.**
3. Phase 4 → proof the safe path is untouched. Do not defer this past the first week.
4. Phase 5 → proof the documentation gate survived.
5. Phase 6 → the escape hatch.
6. Phase 7 → the exemption is enforced and the constitution agrees.

### If Something Goes Wrong

- **Any pull request becomes unmergeable**: roll the ruleset back with
  `specs/023-docs-only-ci-skip/ruleset.before.json` (research §R13). One `PUT`, and the four legacy
  contexts are required again — which is why T014 must never land before T009.
- **A documentation-only run skipped something it should not have**: the offending paths are in the
  `classify` job's summary. Widen nothing until you can say why the path cannot affect a build.

---

## Notes

- `[P]` tasks touch different files and have no ordering between them.
- Live verification tasks need a real pull request; they cannot be done locally, and they are not
  visual passes — the `visual-pass` skill does not apply to this feature.
- Record every Part B result in `specs/023-docs-only-ci-skip/ci-pass.md` as the repository records
  its other manual passes.
- Commit after each task or logical group; `ci.yml` edits are worth their own commits, since a
  bisect through this feature is a bisect through the merge gate.
