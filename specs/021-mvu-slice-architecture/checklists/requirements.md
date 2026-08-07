# Specification Quality Checklist: Feature-Module MVU Architecture

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-07-28
**Last validated**: 2026-08-07 (iteration 4, pre-merge)
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

### Validation findings (iteration 1)

**Stakeholder framing — resolved.** The template's "non-technical stakeholder" criterion is
adapted, not waived: this is an internal restructuring with no end-user-visible change, so the
stakeholder *is* the maintainer. The spec states this explicitly in its Context section and frames
every story and success criterion as cost-of-change rather than product behavior. Requirements
avoid naming languages, frameworks, types or APIs throughout — "the render-free core", "the
binary", "a declared capability", "the mandated model-view-update shape" — so the no-implementation-
details criterion holds on its own terms.

**Baseline verified, not assumed.** The original description cited `src/app.rs` (~1,640 lines) and
`src/main.rs` (~1,940 lines). Those paths no longer exist. Every figure in the spec's baseline
table was measured against the current workspace on 2026-07-28 and the paths corrected to
`crates/micold-client/`. Three premises were corrected as a result:

1. Sizes are larger than described (2,245 / 2,914 lines), and counts are higher (36 state fields,
   124 message variants).
2. Desired outcome #2 is **partly delivered** — feature 017 already unified overlay rendering,
   dismissal and stacking behind a shared vocabulary. The spec scopes this feature to the
   state-and-routing remainder (FR-014 requires building on the existing abstraction, not a
   parallel one).
3. Desired outcome #3 is **partly delivered** — seven ports already exist in the render-free core,
   and process/PTY I/O has left the client for the session daemon. The description's PTY port is
   therefore superseded and deliberately absent from FR-015.

**Two open clarifications remained (Q1, Q2).** Both were genuine scope forks with no safe default,
both created by the client/core/daemon split that postdates the feature description. They were
recorded in the spec's Open Questions section and blocked `/speckit-plan`. All other gaps were
closed with documented assumptions rather than markers, per the 3-marker limit. **Both are resolved
in iteration 2** — see below.

### Validation findings (iteration 2)

**Structural premise made explicit and narrowed.** Iteration 1 took the original description's
"distributed, component-based MVU" at face value: seven features, each owning state, a message
vocabulary, a reducer and a view-model projection, with cross-feature state made unrepresentable.
Checked against established guidance on structuring model-view-update applications, that shape is
the anti-pattern the guidance names — per-component nesting is warranted at the granularity of a
*page*, and this application is a single screen with no pages. The spec now carries a **Structural
stance** section stating the premise, and organizes the work into three tiers plus a shell split.
Two consequences for requirement quality:

1. **Testability improved.** "Each feature is a self-contained MVU unit" was an outcome that could
   only be judged holistically. The tiers are separately checkable: FR-001/FR-001a is a file-layout
   assertion, FR-004a a reducer-location assertion, FR-003 a per-feature recorded decision with
   evidence, and FR-004c an independently-demonstrable green build per tier (SC-004b).
2. **A false requirement was removed.** The former FR-003 — cross-feature state unrepresentable —
   contradicted an edge case the spec itself lists (a view legitimately reading two features' data).
   That contradiction is now resolved explicitly: isolation is enforced on *writes* by guard test
   (FR-020, FR-024a) and reads for display remain available (FR-003a).

**"Zero nested units" is now a stated success condition, not a shortfall.** FR-004b and SC-004a
make the conditional explicit, so the plan cannot be forced into nesting features that have no
independent lifecycle merely to satisfy the spec.

**Q1 resolved — daemon out of scope.** The session daemon is not a model-view-update application,
so no requirement in this spec describes it. Restructuring it is a separate feature. Scope is now
the client and the render-free core.

**Q2 resolved — feature modules live in the client.** The render-free obligation is already met by
the current state file, and core residence would separate a type from its own operations, which
FR-001a forbids. The core keeps the domain model and the declared service capabilities.

**Traceability preserved.** All iteration-1 FR and SC numbers are unchanged; iteration 2 adds only
suffixed identifiers (FR-001a, FR-003a, FR-004a–c, FR-019a, FR-024a, SC-004a, SC-004b). The FR-014
and FR-015 references in iteration 1's findings above still point at the same requirements.

### Validation findings (iteration 3, 2026-08-06 — after rebase onto `main`)

The branch was rebased onto `main`, pulling in 49 commits that postdate the 2026-07-28 baseline.
Every measured figure in the spec was re-taken against the rebased tree. Four corrections resulted:

1. **Baseline figures were stale and are now dual-column.** The shell file grew 2,914 → 3,467
   lines (+19%) and the state file 2,245 → 2,358 (+5%); the message enum went 124 → 128 variants.
   `State` field count (36) and `Overlay` variant count (10) are unchanged. The
   `ClosingOverlay` enum (9 variants) was measured and added to the table — the spec described it
   in prose but never sized it. The baseline table now shows both measurements side by side,
   because the drift itself supports the feature's premise and SC-003's absolute line target.
2. **FR-007 undercounted the popovers.** It named four ad-hoc popovers; there are now seven — a
   project context menu, a terminal context menu and a session context menu were each added as
   another loose state field during the interim. FR-007 now names all seven and points at the
   accretion as the thing FR-009 exists to stop.
3. **A second binary now exists and is now explicitly scoped out.** Feature 020 added a
   development-only component gallery with its own state, message vocabulary and reducer. The spec
   gains a short section placing it out of scope while citing it twice as precedent: it is a worked
   example of FR-003's nesting condition (a separate screen with an independent lifecycle), and its
   isolation is already held by a guard test rather than by unrepresentable state, which is exactly
   FR-024a's mechanism.
4. **The Q2 evidence got stronger, and the claim was corrected.** Iteration 2 said the state file
   "references the rendering framework in only four places". Re-measured: three references, all of
   them in comments, none in code. The claim is now stated precisely, and the client test suite it
   cites has grown from 41 files to 62.

Re-confirmed unchanged: the two named files are still the largest and second-largest source files
in the repository (so SC-003's premise holds); exactly seven service ports exist in the render-free
core; and the three I/O concerns FR-015 calls out as unported — clipboard, OS theme probe,
environment-include resolution — are still unported.

### Validation findings (iteration 4, 2026-08-07 — pre-merge, after rebase onto `origin/main`)

Iteration 3 had been validated against a tree that was already 23 commits behind `main`. The branch
was rebased onto the current `origin/main` and every measured figure re-taken a third time. No
requirement changed; four figures did.

1. **The baseline drifted again, in one day.** The shell file went 3,467 → 3,567 lines and the
   state file 2,358 → 2,434; the message enum gained two more variants (128 → 130) and `State` its
   thirty-seventh field. Measured against the original 2026-07-28 anchor that is +22% and +8% over
   ten days. The spec's drift paragraph now says so explicitly, including that both files grew
   between the 08-06 and 08-07 readings alone — the argument for SC-003's absolute target is
   stronger, not weaker, for having been re-checked.
2. **The daemon figures in Q1 were stale.** The server module is 1,483 lines (was cited as 1,317)
   and the state module 1,317 (was 1,215). Q1's reasoning is unaffected — the daemon is still not a
   model-view-update application — but the parenthetical "both still growing" is now demonstrated
   rather than asserted.
3. **The Q2 rendering-framework count moved 3 → 4, and the test suite 62 → 71 files.** All four
   references remain comments; none is code. The claim is restated at the new figures and the
   conclusion is unchanged.
4. **User Story 2's inline counts were corrected** from 36 fields / 124 variants to 37 / 130, so no
   figure in the spec now contradicts the baseline table.

Re-confirmed unchanged: the two named files are still the largest and second-largest source files
in the repository; exactly seven service ports; the same three unported I/O concerns; seven ad-hoc
popover fields (`help_menu_open`, `project_switcher_open`, `sidebar_filter_open`,
`worktree_menu_open`, `project_menu_open`, `terminal_context_menu`, `session_menu_open`), matching
FR-007's list exactly; `Overlay` at 10 variants and `ClosingOverlay` at 9; and the showcase binary
still under 300 lines behind its packaging guard test.

### Blocking status

**Not blocking, and ready to merge.** All checklist items pass. The branch carries two docs-only
commits against the current `origin/main`, touches no code, and is ready for `/speckit-plan`;
`/speckit-clarify` is no longer required, as the two open questions are resolved in the spec's
Resolved Decisions section.

One caveat the merge should carry: the baseline figures are a snapshot of a moving target, and
have now moved three times in ten days. They are dated in the spec for exactly that reason. Re-take
them at plan time rather than trusting the table; the *ratios* and the argument hold, the absolute
numbers will be low again by then.

Two items for the planning phase to carry, neither a spec defect:

- FR-003 requires a per-feature record of nested-unit versus reducer-module, with evidence. That
  record is a planning artifact and does not exist yet.
- FR-028 requires an incremental migration sequence. The tier table fixes the coarse ordering; the
  step-by-step sequence is still to be produced.
