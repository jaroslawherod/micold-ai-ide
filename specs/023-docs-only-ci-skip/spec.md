# Feature Specification: Documentation-Only Changes Skip the Build

**Feature Branch**: `ci/the-spec-only-changes-should-not-trigger-full-ci-build`

**Created**: 2026-08-09

**Status**: Closed

**Input**: User description: "the changes done only to spec or docs should not trigger the full CI"

## Context

Every pull request in this repository runs the same pipeline: formatting and two clippy passes on
Linux, then a build-and-test matrix on Linux, macOS and Windows, then an advisory assertion-freeze
check, then a documentation presence check. The three matrix legs each compile the whole workspace
from a cold-ish cache; they are the pipeline's cost and its wall-clock.

A large share of this project's pull requests change no code at all. The specification workflow
files feature specs, bug reports, plans, task lists and pass records under `specs/`; the
documentation principle requires user-guide and developer-guide updates under `docs/`; and the
repository's own guidance lives in root Markdown. A pull request that only adds a bug report or
records a visual pass currently pays for three operating systems' worth of Rust compilation to
learn what was already known before it started: nothing it touched can change what the compiler or
the test suite sees.

That is not a guess about the codebase — it is checkable. No test or build step reads the contents
of `docs/` or `specs/`. The only appearances of those paths in the suite are a doc comment pointing
a reader at a design note, and fixture strings that merely look like branch names.

The wrinkle is that the pipeline is not only slow, it is also load-bearing. The default branch's
ruleset requires four checks by name — `fmt + clippy`, and `build + test` on each of the three
runners — before a pull request may merge, with no bypass available to anyone. A workflow that
simply declines to run on documentation paths does not produce a fast green pull request; it
produces one that waits forever for checks that will never report.

Requiring individual job names is what creates that trap: it welds the merge gate to the pipeline's
internal shape, so no job can be skipped, renamed, or re-platformed without making pull requests
unmergeable. This feature replaces those four names with a single aggregate gate that reports the
outcome of the run as a whole. The jobs beneath it become free to skip — which is what makes a
documentation-only run cheap *and* honest: the build jobs report as skipped, rather than reporting
success for work they did not do.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A documentation-only pull request merges without a build (Priority: P1)

Someone files a bug report under a feature's `specs/` directory, or records a visual pass, or fixes
a sentence in the user guide, and opens a pull request. The pull request's check list settles green
within a couple of minutes. No macOS runner starts, no Windows runner starts, nothing compiles. The
build and lint jobs show as skipped — honestly, not as passes they did not earn — and the one check
the default branch requires reports success, so the merge button is live without anyone touching
branch protection, re-running a job, or pushing a throwaway code commit to shake the pipeline
loose.

**Why this priority**: This is the entire request. Delivered alone it removes the cost from the
majority of this repository's pull requests.

**Independent Test**: Open a pull request whose changed files are all under `specs/` or `docs/`.
Confirm the run finishes in minutes, that no compile or test step executed, that the aggregate gate
reports success, and that the pull request is mergeable.

**Acceptance Scenarios**:

1. **Given** a pull request whose changed files are all documentation or specification material,
   **When** its checks run, **Then** no formatting, lint, build or test step executes.
2. **Given** that same pull request, **When** its check list is read, **Then** the aggregate gate
   is present with a successful conclusion, and the lint and build jobs are shown as skipped rather
   than as successful.
3. **Given** that same pull request, **When** a maintainer opens it, **Then** it can be merged by
   the repository's normal merge path, with no administrative override.
4. **Given** a documentation-only push that lands on the default branch, **When** its checks run,
   **Then** they behave the same way as on the pull request — no compilation, checks green.

---

### User Story 2 - A change that touches code still gets the full pipeline (Priority: P1)

Someone changes a Rust source file, a manifest, a build script, or a workflow file — on its own or
alongside a pile of documentation. The pipeline behaves exactly as it does today: formatting,
clippy on the core and the workspace, build and test on all three platforms, the assertion-freeze
check, the docs check. Nothing is skipped, and nothing is skipped merely because most of the
change was prose.

**Why this priority**: Equal to the first. A fast pipeline that occasionally skips a real build is
worse than a slow one, and this repository's principles put the full three-platform suite on every
code change. This story is what stops the feature from becoming a hole in that gate.

**Independent Test**: Open a pull request that changes one Rust file and twenty documentation
files. Confirm every job that runs today still runs.

**Acceptance Scenarios**:

1. **Given** a pull request that changes at least one non-documentation file, **When** its checks
   run, **Then** every job that runs today runs, on every platform it runs on today.
2. **Given** a pull request that changes a code file in one commit and only documentation in a
   later commit, **When** its checks run, **Then** it is treated as a code change — the decision is
   made from everything the pull request changes, not from its most recent commit.
3. **Given** a pull request that changes only comments inside a workflow, manifest, or build
   script, **When** its checks run, **Then** it is treated as a code change.
4. **Given** a pull request that renames or deletes a code file and nothing else, **When** its
   checks run, **Then** it is treated as a code change.
5. **Given** a code-affecting pull request in which one platform's build or test fails, **When** the
   run finishes, **Then** the aggregate gate fails and the pull request is not mergeable — the gate
   summarises the run, it does not paper over it.
6. **Given** a job added to the pipeline that the aggregate gate does not cover, **When** the suite
   runs, **Then** it fails, naming the uncovered job.

---

### User Story 3 - Documentation still has a gate of its own (Priority: P2)

A documentation-only pull request removes or renames one of the documents the project requires to
exist. Its checks fail, and the pull request cannot merge. Skipping the build did not also skip the
one check whose whole job is to notice this.

**Why this priority**: Documentation is a first-class deliverable here, and the check that guards
it is cheap. Losing it would trade a real gate for seconds. It is P2 only because it protects the
feature rather than delivering it.

**Independent Test**: On a documentation-only branch, delete a required user-guide document and
open a pull request. The run must fail.

**Acceptance Scenarios**:

1. **Given** a documentation-only pull request that deletes a required document, **When** its
   checks run, **Then** the run fails and the pull request is not mergeable.
2. **Given** a documentation-only pull request that leaves every required document in place,
   **When** its checks run, **Then** the documentation check passes.

---

### User Story 4 - Anyone can force the full pipeline (Priority: P3)

A contributor changes only documentation but wants the full suite anyway — they are validating the
pipeline itself, or they suspect the classification is wrong, or a dependency moved underneath
them. They can demand a full run on that pull request without inventing a code edit to trigger it.

**Why this priority**: An escape hatch turns a misclassification from a blocker into an
inconvenience. Valuable, but the feature is useful before it exists.

**Independent Test**: On a documentation-only pull request, invoke the escape hatch and confirm the
full pipeline runs.

**Acceptance Scenarios**:

1. **Given** a documentation-only pull request, **When** a contributor asks for a full run through
   the documented mechanism, **Then** every job runs as it would for a code change.
2. **Given** the escape hatch was used, **When** the run finishes, **Then** the aggregate gate
   reports the real result of the full pipeline, not a skipped one.

---

### User Story 5 - The exemption cannot quietly rot (Priority: P2)

A maintainer reading the project's constitution finds a quality gate that matches what the pipeline
actually does, with the documentation-only exemption named rather than implied. And the condition
the exemption stands on — that nothing under test reads project prose — is not a claim in a
document: a contributor who later writes a test that reads a file under `docs/` or `specs/` finds
the build red, with the reason spelled out.

**Why this priority**: The skip is only sound while its precondition holds, and preconditions
recorded solely in prose are the ones that quietly stop holding. This mirrors how the project
already handles a widened exemption — the constitution's showcase-glue carve-out ships with an
automated check that its own precondition still holds.

**Independent Test**: Add a test that reads a file under `docs/`, run the suite, and confirm it
fails with a message naming the exemption it would break. Separately, read the constitution's
quality gates against the pipeline and find no disagreement.

**Acceptance Scenarios**:

1. **Given** a change that adds a test reading content from the documentation set, **When** the
   suite runs, **Then** it fails, and the failure names the documentation-only exemption as what is
   at stake.
2. **Given** the constitution after this feature, **When** its quality gates are read against the
   pipeline's behaviour, **Then** the documentation-only exemption is stated explicitly and the
   full-suite requirement is scoped to changes that can affect built or tested artifacts.
3. **Given** a path newly added to the declared documentation set, **When** the suite runs,
   **Then** tests reading that path fail too — without a second list needing to be edited.

---

### Edge Cases

- **A pull request that changes nothing** (empty diff, or only merge commits): treated as
  documentation-only — there is no code to test — and must still settle green rather than hang.
- **A pull request touching a documentation path and a code path that share a prefix** (for
  example a new top-level file whose name begins with `doc`): only the paths the project has
  declared documentation count; anything else is code.
- **A very large pull request** (hundreds of changed files): the classification must consider every
  changed file, not a truncated first page of them. A pull request too large to classify
  confidently must fall back to the full pipeline.
- **A pull request from a fork**: classification must work without privileges a fork's run does not
  have; if it cannot, the fallback is the full pipeline.
- **The base branch moves while the pull request is open**: the classification is made against the
  pull request's own changed set, so a code change landing on the base branch does not retroactively
  reclassify an open documentation-only pull request. Re-running the checks after a rebase
  reclassifies from the new changed set.
- **A re-run of a skipped run**: re-running a documentation-only run must produce the same
  green result, not an error about jobs that never existed.
- **A job is added to the pipeline and nobody wires it into the aggregate gate**: its failures stop
  blocking merges, silently — the classic failure mode of an aggregate gate, and the reason FR-015
  demands a check rather than a convention.
- **The aggregate gate's own job fails to start** (a syntax error above it, a cancelled run): the
  gate must be absent or failing, never green. A gate that reports success when the run did not
  happen is worse than no gate.
- **The window during which the ruleset is being switched over**: the new gate's name must already
  be reporting on a real run before it is required, or every open pull request blocks on a check
  nothing produces (FR-016).
- **A future test starts reading `docs/` or `specs/` content**: the assumption the whole feature
  rests on stops holding. This must fail the build (FR-024) rather than quietly make the skip
  unsound — a test that reads project prose and a pipeline that skips tests when prose changes are
  individually reasonable and jointly a hole.
- **Someone widens the documentation set later**: the newly-added path becomes both skippable and
  unreadable-by-tests at the same moment, because the classification and the honesty check read the
  same declaration (FR-025).

## Requirements *(mandatory)*

### Functional Requirements

**Classifying a change**

- **FR-001**: The continuous-integration pipeline MUST classify every pull request and every push
  it runs on as either *documentation-only* or *code-affecting*.
- **FR-002**: A change MUST be classified documentation-only only when **every** path it changes
  belongs to the declared documentation set; a single path outside that set MUST make it
  code-affecting.
- **FR-003**: The declared documentation set MUST be recorded in one place in the repository, and
  MUST cover the specification tree (`specs/`), the documentation tree (`docs/`), Markdown files at
  the repository root, the licence text, and images referenced only by documentation.
- **FR-004**: Files that describe how the project is built, tested, packaged, released or linted —
  manifests, lockfiles, toolchain and tool configuration, build and helper scripts, and workflow
  definitions — MUST NOT belong to the documentation set, even when only their comments change.
- **FR-005**: For a pull request, the classification MUST be made from the complete set of files
  the pull request changes relative to its merge base, not from its latest commit alone.
- **FR-006**: When the changed set cannot be determined reliably, the pipeline MUST fall back to
  treating the change as code-affecting.
- **FR-007**: A change that alters no files MUST be classified documentation-only.

**What a documentation-only run does**

- **FR-008**: On a documentation-only change, the pipeline MUST NOT execute any formatting, lint,
  compilation, or test step.
- **FR-009**: On a documentation-only change, the pipeline MUST NOT start a macOS or Windows
  runner.
- **FR-010**: On a documentation-only change, the pipeline MUST still run the documentation
  presence check, and that check MUST be able to fail the run.
- **FR-011**: On a code-affecting change, the pipeline MUST run exactly the jobs, on exactly the
  platforms, that it runs today — this feature MUST NOT remove, reorder, or weaken any existing
  gate.

**Staying mergeable**

- **FR-012**: On every change, under both classifications, every status check the default branch's
  protection rules require MUST report a conclusion, under the exact name the rules demand, without
  human intervention.
- **FR-013**: The merge gate MUST NOT be welded to the pipeline's internal shape. The set of
  required status checks MUST be reduced to a single aggregate gate reporting the outcome of the
  run as a whole, so that the jobs beneath it can be skipped, renamed, or moved between platforms
  without any pull request becoming unmergeable.
- **FR-014**: The aggregate gate MUST fail when any job it covers failed or was cancelled, MUST
  treat a skipped job as satisfied, and MUST NOT be satisfiable by a run in which a covered job
  failed.
- **FR-015**: Every job in the pipeline MUST be covered by the aggregate gate, and an automated
  check MUST fail when one is not — a job added later and forgotten is otherwise a job whose
  failures stop blocking merges silently.
- **FR-016**: The one-time change to the repository's branch-protection rules MUST be applied only
  once a run producing the aggregate gate's check name exists, MUST be recorded with the exact
  command used, and MUST have a recorded rollback.
- **FR-017**: A documentation-only pull request MUST be mergeable through the repository's normal
  merge path, with no administrative override and no re-run of any check.

**Being legible**

- **FR-018**: A reader of a pull request's check list MUST be able to tell, without opening a log,
  whether the run was a full pipeline or a documentation-only one.
- **FR-019**: On a documentation-only change, the jobs that would have compiled, linted or tested
  MUST report as skipped rather than as successful. No check may report success for work it did not
  do.
- **FR-020**: The repository's developer documentation MUST describe the classification, the
  documentation set, the aggregate gate, and the escape hatch, in the same change that introduces
  them.

**Escape hatch**

- **FR-021**: Contributors MUST be able to force the full pipeline on a change that would otherwise
  be classified documentation-only, without adding an unrelated code edit.
- **FR-022**: When the full pipeline is forced, the aggregate gate MUST report its real outcome.

**Governance**

- **FR-023**: The project's governance text MUST NOT be left contradicting the pipeline's
  behaviour. The constitution mandates all-three-platform CI in three separate places — Principle
  VI's CI bullet, the Development Workflow section's TDD gate, and its Cross-platform gate. **All
  three** MUST be amended in the same change as the pipeline, each scoped to changes able to affect
  what is built, linted, packaged or tested, with the documentation-only exemption named. Amending
  fewer than all three leaves the text forbidding what the pipeline does.
- **FR-024**: The amendment MUST NOT rest on its own wording. The condition it depends on — that
  no test or build step reads the contents of the documentation set — MUST be asserted by an
  automated check that fails the build when it stops holding, so the exemption's precondition is
  verified on every code change rather than left to review.
- **FR-025**: The check in FR-024 MUST read the same declared documentation set as the
  classification (FR-003), so the two cannot drift: a path added to the set immediately becomes a
  path no test may read.
- **FR-026**: The amendment MUST be filed as a version bump of the constitution with a recorded
  rationale, consistent with how the project has recorded previous narrowings and exemptions.

### Key Entities

- **Documentation set**: the declared collection of path patterns whose contents cannot affect what
  is compiled, linted, packaged or tested. Single source of truth for the classification; anything
  not in it is code.
- **Change classification**: the verdict — documentation-only or code-affecting — derived from a
  change's full set of touched paths, and the input to every skip decision in the pipeline.
- **Aggregate gate**: the single status check the default branch requires, summarising the whole
  run. It replaces the four per-job checks required today, and is what decouples the merge gate from
  the pipeline's internal shape.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A pull request that changes only documentation or specification material reaches a
  complete, green, mergeable state in under 3 minutes from the moment its checks start.
- **SC-002**: A documentation-only pull request consumes zero macOS and zero Windows runner
  minutes, and zero compilation time on any platform.
- **SC-003**: After the one-time ruleset switch, 100% of documentation-only pull requests merge
  without a maintainer touching branch protection, bypassing a rule, re-running a check, or pushing
  a commit whose only purpose is to trigger the pipeline.
- **SC-003a**: The branch-protection switch requires exactly one edit, applied once, with a
  recorded rollback command — and no pull request is left unmergeable at any point during it.
- **SC-004**: Every change whose touched paths include at least one non-documentation file runs the
  identical set of jobs and platforms it runs today — measured by comparing a code-touching run's
  job list before and after this feature: zero differences.
- **SC-005**: A documentation-only pull request that removes a required document fails its checks —
  demonstrated deliberately, not assumed.
- **SC-006**: A reader can determine which of the two paths a run took from the pull request's
  check list alone, without opening a job log.
- **SC-007**: Across the repository's recent history, the share of pull requests that qualify as
  documentation-only is measured and reported once, so the saving this feature delivers is a known
  number rather than a claim.
- **SC-008**: Introducing a test that reads documentation-set content fails the build —
  demonstrated deliberately on a throwaway change, not assumed from the check's existence.
- **SC-009**: The governance text and the pipeline agree: a reader comparing the constitution's
  quality gates against the pipeline's behaviour finds no case the text forbids and the pipeline
  permits.

## Assumptions

- **The documentation set as drawn**: `specs/`, `docs/`, root-level Markdown *except*
  `CHANGELOG.md` (so `README.md` and `CLAUDE.md`), the licence text, and documentation images at
  the repository root. Everything else — including `assets/` (shipped with the application),
  `scripts/`, `packaging/`, `.github/`, `.cargo/`, and every manifest and tool configuration — is
  code.
- **`CHANGELOG.md` is a build input, not documentation** (corrected during planning; it was
  assumed to be documentation when this spec was first written). It is compiled into the
  application at build time so the app can show a "what's new" view offline, which puts it squarely
  under FR-004: changing it changes the built artifact, so it must take the full pipeline. This is
  the first thing the honesty check of FR-024 would have caught, and it was found by looking for
  exactly what that check looks for.
- **Nothing under test reads documentation**: verified for the suite as it stands — no test or
  build step reads the contents of `docs/` or `specs/`. This is what makes the skip safe, and per
  FR-024 it stops being an assumption at all: it becomes something the build checks.
- **The ruleset is edited once, by hand, at the right moment** (authorised by the repository owner
  during planning; the spec originally forbade it). The four per-job contexts are replaced by the
  single aggregate gate. Because the rules live outside the repository, no pull request can carry
  this change — it is applied manually, and only after a run has already produced the gate's check
  name, so there is never a moment when a required context has no producer.
- **Individual job names stop being load-bearing** once the switch is done. This is the point of
  the aggregate gate: `fmt + clippy` and the three `build + test` legs can then be skipped, renamed,
  or moved between runners without any pull request becoming unmergeable — which is what lets a
  documentation-only run skip them outright instead of faking a pass.
- **The advisory assertion-freeze check counts as code CI** and is skipped on documentation-only
  changes; it compares assertions in the test suite, which such a change cannot touch.
- **Release automation is unaffected**: the release workflow keeps its current triggers, and
  documentation commits keep flowing to it as they do now.
- **Both triggers are in scope**: pull requests and pushes to the default branch, since the
  pipeline runs on both today.

## Out of Scope

- Making the full pipeline itself faster (caching, job splitting, reducing the matrix).
- Any change to the release, packaging, or version-bumping automation.
- Skipping work for other categories of "cheap" change — asset-only, comment-only in source, or
  test-only changes. Only documentation and specification material is in scope here.
- Introducing a merge queue, or changing which checks the default branch requires.
