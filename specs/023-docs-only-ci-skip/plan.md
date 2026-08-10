# Implementation Plan: Documentation-Only Changes Skip the Build

**Branch**: `ci/the-spec-only-changes-should-not-trigger-full-ci-build` | **Date**: 2026-08-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/023-docs-only-ci-skip/spec.md`

## Summary

A pull request that touches only `docs/`, `specs/`, or the repository's own prose currently pays
for three platforms' worth of Rust compilation to learn nothing. This feature classifies every run
by the paths it changes and, on a documentation-only change, skips all formatting, lint, build and
test work.

What made that hard was the merge gate, not the skipping. The default branch requires four status
checks *by job name*, with no bypass for anyone — which welds the gate to the pipeline's internal
shape, so no job can be skipped without making pull requests unmergeable. The repository owner
authorised replacing those four contexts with a single aggregate gate, `ci complete`, that states
the run's outcome. Everything beneath it is then free to skip, and a documentation-only run reports
its build jobs as *skipped* rather than faking a pass for work it did not do.

The classification is one declaration (`.gitattributes`, attribute `micold-docs`) read through one
matcher (`git check-attr`) from two places: the CI classifier and a Rust gate that fails the build
if any test or build step ever starts reading project prose. That gate is what keeps the exemption
honest rather than merely written down — and it found its first real case during research: the
changelog is `include_str!`'d into the binary, so it is a build input and not documentation.

An aggregate gate has one classic failure mode — a job added later and left out of its `needs:`
stops blocking merges silently — so a second source-scanning gate asserts the coverage on every
build.

## Technical Context

**Language/Version**: GitHub Actions workflow YAML; POSIX shell (`bash`) for the classifier; Rust
(stable, pinned by `rust-toolchain.toml`) for the honesty gate

**Primary Dependencies**: `git` (already a hard dependency of this project — Principle III);
`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, all already in
use. **No new third-party action or crate.**

**Storage**: N/A — no application state is touched

**Testing**: `cargo test -p micold-core --all-targets` (picks up both new gates automatically on all
three platforms); a shell-driven test of the classifier against crafted temporary repositories

**Target Platform**: GitHub-hosted runners (`ubuntu-latest` for the classifier; the gates themselves
run on all three)

**Project Type**: Repository infrastructure — CI pipeline, one shell script, two source-scanning
gates, one manual ruleset edit, one governance amendment, one developer document. No application
code changes.

**Performance Goals**: documentation-only run complete and green in under 3 minutes (SC-001), zero
macOS/Windows runner minutes (SC-002)

**Constraints**: the ruleset switch must land only after a run has produced the `ci complete`
context, with a recorded rollback (FR-016) — a required context with no producer blocks every pull
request permanently; the gate must never be satisfiable by a run in which a covered job failed
(FR-014); no code-affecting change may lose a single job it runs today (FR-011, SC-004)

**Scale/Scope**: one workflow file, one new script, three new tests, one new `.gitattributes`, one
manual ruleset edit, one constitution amendment, one developer doc

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Each unit of logic lands Red first. The honesty gate
      goes Red on its first run against the real tree, on the two fixture literals that resolve to
      real documentation paths, and Green once each is allowlisted with a written reason. (Its
      other natural Red — `metadata.rs`'s `include_str!("../../../CHANGELOG.md")` — was consumed
      during planning: the declaration must carve the changelog out from the start, because a
      changelog-only push that skipped the build would skip building the very file it embeds.) The
      coverage gate has one too: written against today's `ci.yml` it fails, because no `ci complete`
      job exists yet. The classifier is driven by a test harness
      that feeds it crafted changed-sets (documentation-only, mixed, empty, deleted-file, spaced
      path) before the script exists. Workflow YAML needs no exception at all: Principle I governs
      *production code*, and `.github/workflows/ci.yml` is neither compiled into nor shipped with
      the application. It is deliberately **not** claimed under Principle I's GUI/process-spawn
      carve-out — that carve-out's path list (`src/main.rs`, `src/ui/`, `src/showcase/`) is
      constitutive rather than illustrative, as the constitution's own 1.5.0 report insists, and
      quietly reading `.github/` into it is precisely the erosion that report warns against. The
      workflow gets the same *discipline* regardless: it carries no decision of its own — every
      branch it takes is `needs.classify.outputs.docs_only`, produced by the tested script — and it
      is validated by the recorded [quickstart.md](./quickstart.md) Part B procedure on real pull
      requests.
- [x] **II. Multi-Session Support**: PASS (not engaged). No session state, no application state.
- [x] **III. Worktree Integration**: PASS (not engaged). No worktree or VCS operation on a user's
      behalf; the classifier reads a diff and writes nothing.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS (not engaged). No application data, no
      network dependency added to the product. Nothing about the app's offline behaviour changes.
- [x] **V. Rust + iced Stack**: PASS. No GUI framework involved; the one piece of logic that lives
      in the repository's own language (the honesty gate) is Rust.
- [x] **VI. Cross-Platform Parity**: PASS. Every code-affecting change still builds and tests on
      Linux, macOS and Windows, unchanged (FR-011, SC-004). The gate itself runs on all three. The
      classifier script runs only on `ubuntu-latest`, and is CI infrastructure rather than product
      code — see Complexity Tracking.
- [x] **VII. Documentation First-Class**: PASS. FR-020 puts the developer document
      (`docs/development/ci-pipeline.md`) in the same change, and the `docs` job — which keeps
      running on documentation-only changes precisely so this stays enforceable — is extended to
      require it.
- [x] **VIII. Reusable UI Component Foundation**: PASS (not engaged). No UI.

**Post-Phase-1 re-check**: unchanged. The design added no application code, no dependency, and no
UI; the only movement was Principle I gaining a concrete Red for each unit, recorded above.

**Governance note**: this feature *amends* the Development Workflow & Quality Gates section (FR-023,
research §R11 — 1.5.0 → 1.6.0, MINOR, with a Sync Impact Report). The check above is against the
constitution as it stands today; the amendment is part of the work, not a way around the gate.

## Project Structure

### Documentation (this feature)

```text
specs/023-docs-only-ci-skip/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── classify-change.md
│   └── required-checks.md
├── checklists/
│   └── requirements.md
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
.gitattributes                                        # NEW — the single declaration of the
                                                      #   documentation set (attribute micold-docs)

.github/workflows/
└── ci.yml                                            # MODIFIED — adds `classify` at the front and
                                                      #   `ci complete` at the back; job-level `if:`
                                                      #   skips lint/test/assertions on a docs-only
                                                      #   change; leaves `docs` unconditional

scripts/
├── classify-change.sh                                # NEW — base..head -> docs_only=true|false
└── tests/
    └── classify-change.test.sh                       # NEW — Red-first harness over temp repos

crates/micold-core/tests/
├── documentation_is_not_read.rs                      # NEW — FR-024/FR-025 honesty gate
└── ci_gate_covers_every_job.rs                       # NEW — FR-015; every job is in the gate's
                                                      #   `needs:`, asserted not assumed

docs/development/
└── ci-pipeline.md                                    # NEW — classification, the set, the escape
                                                      #   hatch, and how to read a skipped run

.specify/memory/constitution.md                       # MODIFIED — 1.5.0 -> 1.6.0 (FR-023, FR-026)
```

**Structure Decision**: The repository is a three-crate Cargo workspace (`micold-core`,
`micold-client`, `micold-daemon`) whose application code this feature does not touch at all. The
work lands in four places instead: the workflow that runs CI, a script beside the repository's
existing CI helper (`scripts/check-assertions-frozen.sh` is the precedent for a shell script on the
merge path), a test in `micold-core` — chosen because its CI step is `--all-targets` and therefore
picks up new tests on all three platforms with no workflow edit (research §R10) — and the
governance and developer documents that FR-020 and FR-023 require.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| The classifier is a shell script exercised only on Linux, not on all three platforms | It runs exclusively in the `classify` job on `ubuntu-latest`; there is no second platform for it to behave differently on | Reimplementing it in Rust so `cargo test` covers it on three platforms would put a compile in front of the very decision whose purpose is to avoid compiling. Principle VI governs user-facing feature parity, which this does not touch |
| A `classify` job now sits on the critical path of every code pull request | The build jobs need its verdict before they can decide whether to run | Each job computing the diff itself means redundant full-history checkouts and several places for the logic to drift, to save one short job |
| Part of this feature lives outside the repository, in a manually-applied ruleset edit | The merge gate is repository *settings*; no pull request can carry a change to it (research §R1) | Leaving the four job names required is what forced the previous design's dishonest green checks. The edit is one command, ordered after the gate is observed reporting, with its rollback recorded (research §R13) |

**Removed by the aggregate gate** — two entries this table carried under the previous design are
simply gone: three `build + test (<os>)` jobs reporting success without having built anything, and
a test matrix collapsed onto Linux to keep names alive. Neither is a trade-off any more, because
those names are no longer what the merge gate reads.
