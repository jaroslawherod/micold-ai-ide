# Feature Specification: Layout Snapshot Parity Gate

**Feature Branch**: `feat/019-layout-snapshot-parity`

**Created**: 2026-07-27

**Status**: Closed

**Input**: User description: "Layout snapshot parity gate. Feature 017 established that CI can verify colour but not layout: `style_snapshot` pins all 116 resolved styles byte-for-byte across both schemes, so a colour regression fails the build, but nothing catches a spacing, sizing or structural regression. That gap is why the sidebar-name-overlapping-its-close-button defect reached a human instead of a test, and why feature 017's parity tasks T001b/T048/T049/T050 could only be closed on manual inspection. This feature closes it: walk the widget tree at fixed window sizes, resolve the layout, and pin the resulting bounds as a committed fixture asserted byte-for-byte in CI, the same shape as the style snapshot — so a layout drift names the widget that moved instead of relying on someone noticing."

## Why this exists

Feature 017 ended with an honest gap, recorded in its own task notes. Every colour the application resolves is pinned byte-for-byte in both schemes and checked on every build. Nothing checks where anything *is*.

That asymmetry has already cost something real. A long session name overlapping its close button shipped, was noticed by a person looking at the running application, and was traced to a layout assumption no test could see. The parity gate that should have caught it (T050) could only be signed off by eye, because the baseline it needed (T001b) was never captured and stopped being capturable once the feature shipped.

This feature closes the gap in the shape that already works here: record the resolved output, commit it, and let a diff name what changed.

## Clarifications

### Session 2026-07-28

- Q: Should every covered state be recorded in both colour schemes, or is scheme-independence better asserted than duplicated? → A: Record one scheme (light) in the fixture, plus an assertion that the dark-scheme walk produces byte-identical geometry.
- Q: One canonical window size, a narrow variant for some states, or a full size matrix? → A: One canonical window size for every state; width-sensitivity is exercised through state data instead.
- Q: Feature 020's showcase gallery claims to be "a better subject for such a fixture" — should 019 cover it? → A: Application states only, as specified. The gallery is recorded as a future extension via FR-016; no dependency and no schedule coupling.
- Q: Are the empty and error states the Edge Cases name required coverage, or implementer's discretion? → A: Required minimum — no project open, an unavailable project, and a disconnected daemon join FR-008's mandated set.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A layout regression fails the build and names what moved (Priority: P1)

A developer changes a component's padding, swaps a container for a different one, or alters how a row distributes its space. They did not intend to move anything else. The automated check fails and tells them precisely which elements shifted, in which state, and by how much — before the change reaches review, let alone a user.

**Why this priority**: This is the entire feature. Everything else is convenience around it. Delivered alone, it converts the class of defect that reached a person in feature 017 into a build failure.

**Independent Test**: Introduce a deliberate one-off spacing change in any covered state, run the check, and confirm it fails naming that element. Revert, and confirm it passes.

**Acceptance Scenarios**:

1. **Given** the committed fixture matches the current application, **When** the check runs with no changes, **Then** it passes.
2. **Given** a developer increases the padding of one component, **When** the check runs, **Then** it fails and the message identifies the affected element, the covered state, and the recorded versus observed geometry.
3. **Given** a developer restructures a widget tree without changing any resolved position or size, **When** the check runs, **Then** it passes — the record is of geometry, not of tree shape for its own sake.
4. **Given** a covered state is removed from the application, **When** the check runs, **Then** it fails rather than silently narrowing coverage.

---

### User Story 2 - An intended layout change is easy to accept and to review (Priority: P2)

A developer deliberately changes a layout. They regenerate the fixture with one documented command, and the resulting diff shows exactly which elements moved and by how much — reviewable in the pull request without building or running the application.

**Why this priority**: A gate that is painful to satisfy gets bypassed or deleted. It also turns the fixture into review evidence: "this change moves these four things and nothing else" is a stronger claim than any prose description.

**Independent Test**: Make an intentional layout change, run the documented regeneration command, and confirm the diff is human-readable and limited to the affected elements.

**Acceptance Scenarios**:

1. **Given** an intentional layout change, **When** the developer runs the documented regeneration command, **Then** the fixture updates and the check passes.
2. **Given** a regenerated fixture, **When** a reviewer reads the diff, **Then** each changed line identifies an element and its state without requiring the application to be run.
3. **Given** the regeneration command is not explicitly invoked, **When** the check runs, **Then** it never rewrites the fixture as a side effect.

---

### User Story 3 - Coverage is visible and cheap to extend (Priority: P3)

A developer adds a new screen or dialog. Bringing it under the gate is a small, obvious step. A developer reading the check can tell what it does *not* cover, so nobody assumes more protection than exists.

**Why this priority**: Feature 017's real failure was not a missing test — it was an unclear boundary between what CI verified and what a human still had to. This story keeps the new gate from acquiring the same ambiguity.

**Independent Test**: Add a new covered state, confirm it takes a single registration step, and confirm the documented coverage boundaries match reality.

**Acceptance Scenarios**:

1. **Given** a new dialog exists, **When** a developer registers it as a covered state, **Then** the fixture gains its layout with no other change required.
2. **Given** a developer reads the check's documentation, **When** they ask "would this catch X?", **Then** the answer is stated explicitly for text-dependent, animated and scroll-dependent geometry.

---

### Edge Cases

- **Text-dependent geometry.** Widths derived from measuring a string depend on the typeface resolved at run time. Until feature 018 ships Roboto with the application, the platform's default sans-serif is used — a different typeface on each operating system, and potentially between machines running the same one. See the Dependencies section; this is the constraint that shapes the whole feature.
- **Variable data.** A state built from the developer's real workspace records their worktree names and counts. Covered states must be constructed from fixed data.
- **Animation.** Components that animate own a progress value, so their geometry differs mid-transition. What is recorded must be a defined, reproducible moment rather than "whenever the walk happened".
- **Overlays.** Dialogs and menus are positioned in a layer above the base tree. Recording the base tree alone would leave the surfaces most likely to be repositioned uncovered.
- **Scrolling.** Content taller than its viewport has geometry that depends on scroll offset.
- **Sub-pixel values.** Resolved geometry is fractional. Differences in the final decimal places are noise, not regressions, and would make the fixture flap.
- **Empty and error states.** No project open, an unavailable project, a disconnected daemon — these are layouts too, and are the ones least often looked at by eye.
- **Window size.** Geometry is a function of the space available, so a record is meaningless without the size it was taken at. One canonical size is used throughout (FR-008b); what varies between covered states is their data, not the window.
- **A defect present when the fixture is generated.** The gate pins what *is*, not what is *correct*. An existing layout bug is baked in silently.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST resolve the layout of a defined set of application states without a display, a GPU, or a window manager, so the check runs identically in CI and on a developer machine.
- **FR-002**: For each covered state, the system MUST record every laid-out element's identity, position and size in a stable, deterministic order.
- **FR-003**: The recorded output MUST be asserted against a committed fixture, and any difference MUST fail the check.
- **FR-004**: A failure MUST name the covered state and the specific element(s) that differ, together with the recorded and observed geometry. A failure that only reports "the layout changed" does not satisfy this requirement.
- **FR-005**: The check MUST be deterministic — repeated runs on the same commit and machine MUST produce identical output.
- **FR-006**: The check MUST produce identical output on Linux, macOS and Windows for all geometry it covers. Any category it cannot cover identically MUST be excluded from the fixture rather than tolerated as a difference.
- **FR-007**: Covered states MUST be constructed from fixed test data and MUST NOT read the developer's real workspace, configuration, or session store.
- **FR-008**: Covered states MUST include, at minimum, the reduced parity set feature 017's T001b named: the main shell with the sidebar expanded and collapsed, the add-worktree dialog in both branch-source modes, and one open menu — each at a recorded window size.
- **FR-008a**: The fixture MUST record a single colour scheme. The system MUST separately assert that resolving every covered state in the other scheme yields byte-identical geometry, and MUST fail naming the state if it does not.
- **FR-008b**: All covered states MUST be resolved at one canonical window size, recorded in the fixture. Layout behaviour that depends on content exceeding its container MUST be exercised through a covered state's fixed data — a constrained panel width, a deliberately over-long label — rather than by adding window sizes.
- **FR-008c**: Covered states MUST additionally include the empty and error layouts: no project open, an unavailable project, and a disconnected daemon. These are the screens least often inspected by eye, which is where an automated gate earns most over the human inspection feature 017 closed on.
- **FR-008d**: Covered states MUST exercise every **nesting level** a covered screen can display, not the collapsed form alone. Concretely: at least one state MUST have a worktree expanded, so the sidebar tree contains a row below depth 0. *(Added 2026-08-07 by [018](../018-material3-visual-system/spec.md)'s BUG-005.)* Every one of the sixteen states registered before that date held only depth-0 tree rows, and the sidebar's row height had come to depend on a row's depth — so the fixture was byte-identical across a change that took a third of the height off every session row in the application, and the byte-identity was then cited as evidence that nothing had moved. A screen's collapsed form is not a sample of its expanded one.
- **FR-009**: Coverage MUST extend to overlay surfaces (dialogs, menus, and other floating layers), not the base widget tree alone.
- **FR-010**: Elements whose geometry depends on an animation MUST be recorded at a defined, reproducible point in that animation.
- **FR-011**: Elements whose geometry depends on scroll position MUST be recorded at a defined, reproducible scroll offset.
- **FR-012**: Recorded values MUST be normalised to a fixed precision sufficient to absorb floating-point noise while still distinguishing any difference a person could see.
- **FR-013**: The system MUST provide a single documented command that regenerates the fixture deliberately, and MUST NOT regenerate it as a side effect of a normal run. *(Annotated 2026-08-16 by BUG-001: the second clause always held; the first was violated in effect for the feature's whole life, because the command every artefact documented was missing `--test` and therefore ran nothing. The mechanism was correct throughout — only the written invocation was wrong, which is why nothing failed. See FR-013a.)*
- **FR-013a** (bugfix BUG-001): The documented regeneration command MUST be **verified to select this gate**, not merely written down. A command that names the target as a bare test-name filter matches no test, reports `0 passed`, and exits 0 — so a reader following it accepts no baseline while being told it worked. Every place the command is printed or committed — the failure message, the fixture's own header line, the module doc, the contract, and the user-facing docs — MUST carry the same verified form, and that agreement MUST be asserted mechanically rather than maintained by review.
- **FR-014**: The check MUST fail when a covered state can no longer be constructed, so removing a screen is a visible event rather than a silent loss of coverage.
- **FR-015**: The check MUST document what it covers and — explicitly — what it does not, including every category of geometry excluded under FR-006.
- **FR-016**: Registering an additional covered state MUST require changes in one place only.
- **FR-017**: The check MUST run as part of the standard test suite on every change, on all three supported platforms, consistent with the existing cross-platform gate.
- **FR-018**: The feature MUST be demonstrated against the defect class that motivated it: reintroducing an over-long label overlapping its adjacent control MUST cause the check to fail.
- **FR-018a**: A text-overflow finding MUST be attributed to the node that **drew** the text, established from the clip the painter actually passed, and MUST NOT be attributed to a node merely because it contains the text's origin and sits deeper in the tree. Where no node matches the clip — a widget that clips to nothing of its own, which is the FR-018 defect — the deepest containing node stands, so that case is still caught. *(Added 2026-08-07 by [018](../018-material3-visual-system/spec.md)'s BUG-005.)* Two recorded overflows were false positives under the previous rule: the sidebar's `"Short"` row label, drawn at (32, 146.8) and correctly clipped to its own 164dp box, was measured against a 24.65dp filter-panel chip node that the collapsed panel had left lying at the same coordinates. Attribution by geometry alone cannot tell a container from a stranger standing on the same spot, and this gate had already changed the rule once — narrowest containing node to deepest — against the same class, which altered which stranger wins rather than whether one can.
- **FR-019**: Generating the fixture MUST NOT change the application's appearance. If generation surfaces an existing layout defect, that is recorded as a finding and addressed separately, not fixed silently while building the gate.

### Key Entities

- **Covered state**: A named, reproducible configuration of the application — the data it holds, the overlay it has open, the window size it is laid out in, and the colour scheme — from which a layout can be resolved.
- **Layout record**: One element's resolved geometry within a covered state: what it is, where it is, and how large it is.
- **Fixture**: The committed, human-readable collection of layout records across all covered states; the reference the check asserts against, and the artefact a reviewer reads.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Any change that moves or resizes a covered element is caught automatically, with no human inspection required, and the failure identifies the element.
- **SC-002**: The check yields identical results across all three supported platforms and across repeated runs, with zero spurious failures over a full release cycle.
- **SC-003**: The specific defect that motivated this feature — an over-long label overlapping its adjacent control — is reproduced and caught by the check, demonstrated as part of delivery.
- **SC-004**: Every screen named in feature 017's reduced parity set is covered, so the manual walkthrough that feature could only close by eye becomes automated — together with the empty and error layouts of FR-008c, which no walkthrough reliably reached at all.
- **SC-005**: Accepting an intended layout change takes one documented command, and the resulting diff is understandable by a reviewer who has not run the application. *(Annotated 2026-08-16 by BUG-001: the diff half held; the one-documented-command half did not, since the documented command accepted nothing. Measure this by the fixture changing, never by the command's exit code — the broken form exited 0.)*
- **SC-007a** (bugfix BUG-001): Following the printed regeneration instruction verbatim, with no prior knowledge of cargo's argument forms, accepts the pending change — demonstrated by the fixture actually changing, not by the command succeeding.
- **SC-006**: The gates stay cheap enough that nobody is tempted to skip the suite: `mise run test` completes in **under 60 seconds** locally on a developer machine.
- **SC-006a**: That cost grows with coverage and with nothing else: **one additional covered state adds no more than 3 seconds** to the suite, across both schemes.

  > **Amended 2026-07-29, after measurement.** The original read: *"The check completes fast enough
  > to stay in the default suite — under 10 seconds locally, and adding no more than 10% to total
  > suite runtime."* Both halves were wrong in ways worth recording, because both were written
  > before anyone had measured what this work costs.
  >
  > **The 10 seconds named a test binary, and that boundary turned out to be an implementation
  > detail.** The gates share binaries — the containment gate lives inside `layout_snapshot`
  > precisely so it can reuse that binary's resolved records instead of recomputing them — so which
  > binary a gate sits in changes the measured number without changing the work done or the time
  > anyone waits. Moving a gate between files must not be able to pass or fail a criterion. The
  > suite is what a developer actually waits on, so the suite is what is budgeted.
  >
  > **The 10% share was perverse.** A ceiling stated as a fraction of the suite tightens when the
  > rest of the suite gets faster: making some unrelated test quicker could fail this criterion with
  > the gates untouched, and the cheapest way to satisfy it would be to slow something else down.
  > An absolute budget plus a growth rule says the intended thing directly.
  >
  > **The number was set against a prediction that proved wrong.** R9 reasoned that layout is a pure
  > tree walk with cached shaping and that the dominant cost would be one-time font-system
  > construction, so a few dozen states would not approach 10s. The dominant cost is in fact shaping
  > real text across nine screens in two schemes — about 12s, and irreducible without giving
  > something up: fewer covered states weakens FR-008 and SC-004, dropping the dark pass violates
  > FR-008a, and faster shaping is not ours to write.
  >
  > SC-006a is the load-bearing half now. A fixed total would have to be raised every time coverage
  > grew, which would make it a record of what happened rather than a budget; a per-state ceiling
  > keeps the gates honest as FR-016 invites more states to be added.
- **SC-007**: A developer can state, from the documentation alone and without reading the implementation, whether a given category of visual regression would be caught.

## Dependencies

- **D1 — Text-derived geometry depends on feature 018.** FR-006 requires identical output on all three platforms, and the application currently requests the platform's default sans-serif, so the same string measures differently on each. Feature 018 (`018-material3-visual-system`) already resolves this as a product decision, not a testing workaround: its FR-008 requires text to render in a typeface shipped with the application, and FR-008a names Roboto in two static weights, precisely so rendering is identical regardless of installed system fonts.

  Feature 018 is specified but not implemented. Two orderings are therefore viable, and the choice belongs to planning rather than to this spec:

  - **Sequence this feature after 018.** Full coverage including text-derived widths, one fixture, no compromise. Costs waiting.
  - **Deliver structural coverage first, extend after 018.** Cover containers, spacing, structure and fixed sizes now — excluding text-derived widths under FR-006 and documenting the exclusion loudly under FR-015 — then widen once the typeface ships. Note that the motivating defect is still catchable this way: what failed was a container not constraining its label, which is structural.

  What is *not* viable is shipping a fixture containing system-font measurements: it would pass only on the machine that generated it.

  **Resolved in planning** (see `plan.md` and `research.md` R2): neither ordering was taken. The
  snapshot constructs its own headless renderer and therefore chooses its own default font, and the
  application's text layer sets no font of its own — so pinning a committed typeface as the
  measuring basis makes metrics identical everywhere immediately, with text-derived geometry fully
  in scope. **This feature does not depend on feature 018.**

- **D2 — No dependency on feature 020.** Feature 020 (`020-component-showcase-gallery`) states that
  its gallery would be "a better subject for such a fixture" because its content is fixed. That
  advantage does not apply here: FR-007 already requires covered states built from fixed test data,
  so this feature's determinism does not come from the gallery. Nor could the gallery substitute —
  FR-008's minimum set is *application screens*, and FR-018's demonstration is a composition defect
  a component shown in isolation cannot exhibit. Covering the gallery is recorded as a possible
  future extension through FR-016's one-place path, once 020 exists. **Neither feature blocks the
  other, and they share no files.**

## Assumptions

- **This gate is forward-looking, not retroactive.** It cannot recover the pre-017 baseline T001b was meant to capture; that comparison is permanently unavailable. Its value is guarding every change from now on. Feature 017's parity claim stands as recorded — closed on human inspection — and is not reopened here.
- **The fixture records the application as it is on the day it is generated.** Generating it asserts that the layout is *known*, not that it is *correct*. The gate prevents drift; it does not audit design.
- **Mirroring the style snapshot is deliberate.** Committed fixture, byte-for-byte assertion, explicit regeneration — already proven in this codebase, and reusing its shape keeps both gates understandable as one idea rather than two.
- **Coverage is a curated set of states, not every reachable state.** The application's state space is unbounded; the set covers each distinct screen and its meaningful variants, and is expected to grow via FR-016.
- **Scheme-independence is asserted, not duplicated.** The fixture records the light scheme only; a separate assertion walks the dark scheme and requires byte-identical geometry. Layout is expected to be scheme-independent, and this makes that expectation *checked* — a scheme-dependent layout fails naming the state — without doubling an artefact a reviewer has to read.
- **No change to the application's appearance is in scope** (FR-019). This feature adds a check.

**Bugfix**: 2026-08-07 — [feature 018](../018-material3-visual-system/bugs/BUG-002.md)'s BUG-002 is the "known, not correct" assumption above arriving in practice, and is worth recording here rather than only there. The fixture held a 499.9 × 64.0 icon button in the app bar — §7.3 states 48 × 48 — as its expected value from the day it was generated, and was green on it throughout. Nothing about the gate misbehaved; a snapshot records what it is shown, so a defect older than the fixture becomes the baseline. What the gate *did* do is prove the fix, in a diff that moves the trigger to the trailing edge it belongs on. Two consequences: 018 added SC-008b, an assertion about laid-out size that a snapshot structurally cannot make, and the covered set gained `session-terminal-bottom-bar` (FR-016, one registration) — the terminal screen had no geometry coverage at all, which is why only half of that defect was recorded anywhere.

**Bugfix**: 2026-08-16 — BUG-001 Every documented invocation of this gate omitted `--test`, so
`cargo test -p micold-client layout_snapshot` read the target's name as a **test-name filter**,
matched nothing, and exited 0 having run nothing. Eleven occurrences across five files. Two classes:
the regeneration hints (the failure message, the fixture's own header line, the module doc, the
contract, the user-facing docs), where following one accepted no baseline while reporting success;
and the quickstart's *run the gate* commands, which do not regenerate anything and exist purely to
exercise the check — Part A's headless "must still pass" therefore passed vacuously. FR-013
annotated and FR-013a added (the documented command must be verified to select the gate, asserted
mechanically); SC-005 annotated and SC-007a added (measured by the fixture changing, never by an exit
code). The requirements themselves needed no correction and the plan needed no change: the mechanism
behind FR-013 was right the whole time. What was missing is that a string printed to a human sat
outside everything verifying a feature whose subject is that unverified claims decay —
`layout_snapshot_regeneration.rs` already stops the gate from rewriting its baseline and *reporting
success*, and this was that file's mirror image. `the_regenerate_hint_selects_this_target` now reads
every printed command and fails if it lacks `--test`. Fixed in PR #176 before the report was filed;
the fixture's only change was its header line, and no geometry moved.
