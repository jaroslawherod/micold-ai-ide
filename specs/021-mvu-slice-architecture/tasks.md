---

description: "Task list for feature 021 — Feature-Module MVU Architecture"
---

# Tasks: Feature-Module MVU Architecture

**Input**: Design documents from `/specs/021-mvu-slice-architecture/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md — all present

**Tests**: MANDATORY per Constitution Principle I. This feature has an unusual test posture worth
stating once: the **existing 71-file suite is the specification** (spec assumption "Test suite is the
behavior specification"), and FR-027 freezes its assertions. So for *extraction* tasks the Red state
already exists — any behavior drift turns the suite red. New **invariants** (the three guard tests)
follow ordinary Red-Green-Refactor: the guard is written and observed failing first.

**Documentation**: Not user-facing, so Principle VII is satisfied by architectural documentation
(`docs/development/architecture.md`), written per-story rather than deferred to polish.

**Cross-platform**: Principle VI. Only one platform branch is touched (the OS-theme probe), and
porting it improves parity. SC-006 requires all three platforms green.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1–US4 from spec.md
- Every task names exact file paths

## Path Conventions

Three-crate Rust workspace. `crates/micold-core/` (render-free domain + ports),
`crates/micold-client/` (iced GUI + app state), `crates/micold-daemon/` (**out of scope**, Q1).

## Story-to-tier map, and why the order differs from priority

US1 and US2 are **both P1**. The spec breaks the tie explicitly: US2 "is the outcome the other three
depend on — without per-feature boundaries there is nothing for the overlay registry to register
into". So Tier 1 (US2's first half) goes first, and it is the MVP.

US2 is delivered across **two** phases, because the spec assigns it both feature modules (Tier 1)
and per-feature reducer modules (Tier 3). This is not a numbering accident.

| Phase | Tier | Story | research.md §6 steps |
|---|---|---|---|
| 3 | Tier 1 | US2 (part 1) 🎯 MVP | 1–7 |
| 4 | Tier 2 | US1 | 8–11 |
| 5 | Shell split | US3 | 12–16 |
| 6 | Tier 3 | US2 (part 2) + US4 | 17–20 |

**Every task is its own commit** — SC-009 is verified from git history, not just the endpoint.
A task that needs a later task to compile is a planning error, not an acceptable intermediate.

---

## Phase 1: Setup

**Purpose**: Scaffolding and a baseline to measure against

- [X] T001 Create `crates/micold-client/src/features/mod.rs` with an empty module tree and declare `mod features;` in `crates/micold-client/src/lib.rs`
- [X] T002 [P] Record the pre-change baseline in `specs/021-mvu-slice-architecture/baseline.md`: per-file line counts from `find crates -name '*.rs' -exec wc -l {} + | sort -rn | head -10`, `State` field count, `Message` variant count, and the current commit SHA
- [X] T003 [P] Create `docs/development/architecture.md` with section headings only — tier structure, where a feature lives, adding a floating surface, adding a capability, the read/write asymmetry

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The safety nets every later phase relies on

**⚠️ CRITICAL**: No extraction may begin until T004 exists — it is what makes FR-027 enforceable
rather than aspirational

- [X] T004 Add an assertion-freeze check to `scripts/check-assertions-frozen.sh` that fails when `git diff <base>...HEAD -- crates/*/tests/` removes or alters a line matching `assert`, allowing pure relocation (the identical assertion re-added elsewhere in the same diff), per FR-027
- [X] T005 [P] Wire T004 into CI in `.github/workflows/ci.yml` as a non-blocking advisory job first, so its false-positive rate is known before it gates merges — **it earned its keep on PR #98**: two false positives on this feature's own diff (a relocated constant's import path changing, and rustfmt rewrapping an assertion past the width limit). Both fixed by comparing whole balanced assertion statements instead of lines; four probes (delete / weaken / rewrap / no-op) verify it still flags what it should. Had this job been blocking from the start, T023 would have been stuck behind a check that was wrong
- [X] T006 [P] Confirm the baseline suite is green on Linux, macOS and Windows via CI before any change lands — discharged on PR #98, run 31261847685: `build + test` passed on ubuntu, macos and windows. Needed a pull request to exist at all, since `.github/workflows/ci.yml` triggers only on `push: main` or `pull_request`; a feature-branch push runs nothing. The same run caught `fmt + clippy` failing on drift the local loop had not been checking

**Checkpoint**: Behavior drift and assertion tampering are both detectable. Extraction can start.

---

## Phase 3: User Story 2 (part 1) — Feature modules (Priority: P1) 🎯 MVP

**Goal**: Every custom type in the monolithic state file moves into a module named for its feature,
together with the helper functions over it. Tier 1 of research.md §6, steps 1–7.

**Independent Test**: Pick any feature; write a test constructing only that feature's types,
exercising only its own operations, with no reference to any unrelated feature's types. It must
compile and pass without the application shell (SC-004).

**Method for every extraction task below**: move the type *with* its helpers (FR-001), `pub use` it
back from `app.rs` in the same commit so no call site changes, and keep the whole suite green. Never
split a feature across parallel state/update/view files (FR-001a).

**Commit shape — corrected during T016.** The isolation test and its extraction go in **one**
commit. Splitting them, as T007/T015 did, leaves the test commit importing a module that does not
exist yet: `809e7ae` does not compile, which violates SC-009 ("verified by the step's own commit").
The test is still written and observed failing first — that ordering is what TDD asks for — but
Red is recorded in the commit message rather than in the history. Run `cargo fmt --all` before
committing; CI's `fmt + clippy` job checks it and the first extractions did not.

### Tests for User Story 2 — write first, observe failing ⚠️

Each test constructs only its own feature's types, so each fails to compile until its module exists.
All are separate files, so all are parallelizable.

- [X] T007 [P] [US2] Isolation test for the worktree-creation form in `crates/micold-client/tests/features_worktree_form.rs`
- [X] T008 [P] [US2] Isolation test for sidebar types in `crates/micold-client/tests/features_sidebar.rs`
- [X] T009 [P] [US2] Isolation test for project/workspace types in `crates/micold-client/tests/features_project.rs`
- [X] T010 [P] [US2] Isolation test for settings types in `crates/micold-client/tests/features_settings.rs`
- [X] T011 [P] [US2] Isolation test for worktree types in `crates/micold-client/tests/features_worktree.rs`
- [X] T012 [P] [US2] Isolation test for notification types in `crates/micold-client/tests/features_notifications.rs`
- [X] T013 [P] [US2] Isolation test for session types in `crates/micold-client/tests/features_session.rs`
- [X] T014 [P] [US2] Isolation test for daemon-connection types in `crates/micold-client/tests/features_connection.rs`
- [X] T014a [P] [US2] Render-free guard in `crates/micold-client/tests/features_are_render_free.rs` asserting no module under `crates/micold-client/src/features/` names the rendering framework in code — comments excepted, matching the existing convention in `app.rs` (FR-006). Follows the mechanism of `crates/micold-client/tests/{material_boundary,cdk_no_appearance}.rs`. **This is a regression lock, not a migration**: the property holds today and the guard exists to keep it holding across eight extractions. Q2's decision to site feature modules in the client rests entirely on it

### Implementation for User Story 2 (part 1)

Sequential — every task edits `app.rs`, so none of these are parallelizable against each other.

- [X] T015 [US2] Move `WorktreeForm`, `WorktreeFormStatus`, `BranchSource`, `ResolutionState` and their impls from `crates/micold-client/src/app.rs:86–326` to `crates/micold-client/src/features/worktree_form.rs` (~240 lines)
- [X] T016 [US2] Move `SidebarEntry`, `DefaultNode`, `WorktreeNode`, `TagFilter`, `matches_filters`, `worktree_location_label` from `crates/micold-client/src/app.rs:372–456` to `crates/micold-client/src/features/sidebar.rs` (~85 lines) — `DEFAULT_LOCATION_LABEL` travelled with them: it is the project-root half of the same location tooltip, and leaving it behind would split the feature (FR-001)
- [X] T017 [US2] Move `ProjectMenu`, `clamp_menu_anchor`, `SwitcherEntry`, `RenameDraft`, ~~`SelectKind`~~ from `crates/micold-client/src/app.rs:327–371, 457–497` to `crates/micold-client/src/features/project.rs` (~85 lines) — **`SelectKind` did not travel**: it is terminal text selection (feature 006, FR-013) and was swept into this task by its line range, not by its feature. Grouping it under "project" would be the exact mistake FR-001 exists to correct. It goes to `features/session.rs` in T021 instead
- [X] T018 [US2] Move `SettingsDraft` from `crates/micold-client/src/app.rs:469–484` to `crates/micold-client/src/features/settings.rs` (~16 lines) — **the feature stays split after this task**: the draft's validation (range checks and their error messages) lives in `main.rs`'s `Message::SettingsSaved` arm, not beside the type it validates. That is reducer code returning a `Task`, so moving it is Tier 3 work, not Tier 1's. Recorded in the module's own doc comment so the split is visible from the code
- [X] T019 [US2] Move `WorktreeRenameDraft` and the worktree helpers `worktree_tree`, `filtered_worktree_tree`, `visible_worktrees`, `has_visible_worktrees`, `worktree_tags`, `worktree_display_name`, `available_tag_filters` from `crates/micold-client/src/app.rs:498–510, 2156–2245, 2276+` to `crates/micold-client/src/features/worktree.rs` (~105 lines) — **split across two modules, not one.** `WorktreeRenameDraft`, `visible_worktrees`, `has_visible_worktrees`, `worktree_display_name` and `worktree_tags` are worktree-owned and went to `features/worktree.rs`. `worktree_tree`, `filtered_worktree_tree` and `available_tag_filters` went to `features/sidebar.rs`: they are named for worktrees but typed for the sidebar — they return `WorktreeNode`/`TagFilter`, read `sidebar_filters`, and `worktree_tree`'s doc comment opens "Build the sidebar tree". Filing them by name would group against the feature, which is what FR-001 argues against and what SC-010 measures. `sidebar_entries` moved with them (T016 had left it behind). `worktree_tags` widened from private to `pub(crate)` to cross the boundary — revisit in T062
- [X] T020 [US2] Move `NoticeLevel` and ~~`Notification`~~ from `crates/micold-client/src/app.rs:923–944` to `crates/micold-client/src/features/notifications.rs`, reconciling against the existing `micold_core::notify` queue rather than duplicating it (~22 lines) — **the reconciliation was a deletion.** `app::Notification` was never constructed anywhere: every real notification is a `micold_core::notify::Notification` on the queue, which is what the snackbar renders. The duplication the task warned against was already present, not about to be introduced. `NoticeLevel` is not a duplicate and stays — it is the banner's fill vocabulary, while `notify::Level` also decides how long a message lingers (FR-032c). The inline `match` that translated between them became `NoticeLevel::to_queue_level`
- [X] T021 [US2] Move the session helpers `sessions_in_worktree`, `active_sessions`, `switch_active`, `record_foreground`, `restore_after_activation`, `restore_foreground`, `arm_notice`, `note_background_restart`, `session_mut` from `crates/micold-client/src/app.rs:2014–2155` to `crates/micold-client/src/features/session.rs` (~142 lines), **plus `SelectKind`**, reassigned here from T017 — `session_mut` widened from private to `pub(crate)`: seven reducer arms call it and the reducer does not move until Tier 3 (T062)
- [X] T022 [US2] Extract the daemon-connection types and the `connection_status` projection from `crates/micold-client/src/app.rs` and `crates/micold-client/src/main.rs:2106` to `crates/micold-client/src/features/connection.rs` — **this step is absent from research.md §6's Tier 1 table, which lists seven steps for eight features; the gap was found during task generation** (FR-001, as amended). `ConnectionStatus` came from `ui/mod.rs`, not `app.rs` — it sat beside the banner that draws it. `connection_status` now takes the four facts instead of the shell's `App`, so the precedence is testable without a window (Principle I); the active-project-to-displacement lookup stays in `main.rs`, being plumbing rather than a decision
- [X] T023 [US2] Remove the transitional `pub use` re-exports from `crates/micold-client/src/app.rs` and update every call site to import from `crate::features::*` — also removed the one in `crates/micold-client/src/ui/mod.rs` that T022 left. The `app::` references that remain across the suite are `State`, `Message`, `Overlay`, `ClosingOverlay` and `on_escape`, which Tier 1 does not move
- [X] T024 [US2] Write the "where a feature lives" and "tier structure" sections of `docs/development/architecture.md`, listing all nine modules — the ninth is `worktree_form`, not an overlay module; overlays are listed as still living in `app.rs` until Tier 2
- [X] T025 [US2] Verify SC-010 by review — name the single module for each feature in FR-001 — and record the intermediate `app.rs` line count against T002's baseline — **SC-010 does not yet pass, and Tier 1 could never have made it pass**: FR-001 names nine features and `overlays` is one of them, which Tier 2 gives a module. Settings is partial (validation in `main.rs`). The review table is in `baseline.md`; `app.rs` 2,434 → 1,689 (−31%)

**Checkpoint**: `app.rs` should be roughly 1,700 lines (types out, both reducers still in). Every
feature answers "where does it live?" with one module. Full suite green on all three platforms.

---

## Phase 4: User Story 1 — Overlay registry (Priority: P1)

**Goal**: Adding a floating surface costs its own module plus at most one registration line, and
zero edits to any central match statement. Tier 2 of research.md §6, steps 8–11.

**Independent Test**: Add a throwaway overlay end-to-end and count changed files with
`git diff --stat`. It must be its own module plus ≤1 registration line, with zero central match
edits (SC-001). Then revert.

**⚠️ Highest-risk phase in the feature.** The exit-animation snapshot (FR-011) renders a *copy* of a
surface whose live state has been cleared, and `ClosingOverlay` exists solely to serve it. Steps
land as four separate commits so a bisect finds one of them, not a monolithic overlay rewrite.

### Tests for User Story 1 — write first, observe failing ⚠️

- [X] T026 [P] [US1] Registration guard in `crates/micold-client/tests/overlay_registration.rs` — a surface that exists but is not registered MUST fail the build or this test, never be discovered by hand at runtime (FR-010, contract R2) — **guards the seven popovers, not the `Overlay` enum.** R2's premise ("forgetting one of eight edit sites") does not hold for the enum: those sites are matches over a closed enum, so `rustc` already enforces coverage — verified by removing an arm three ways, each of which failed to compile before any test ran. The popovers are loose `State` fields with no such protection, and the two dismissal paths clear different subsets of them (4 of 7 and 6 of 7). The guard makes every combination a recorded decision; three probes confirm it fires
- [X] T027 [P] [US1] Dismissal-ordering test in `crates/micold-client/tests/overlay_dispatch_ordering.rs` covering contract obligations D1 (~~popover closes before modal on Escape~~ — **the contract had D1 backwards**; the popover branch is guarded on no modal being open, so the modal wins whenever both are, and FR-012 says preserve what exists. Contract corrected, test asserts the code), D2 (opening a modal closes popovers) and D3 (closing the filter panel leaves filters intact), asserted against the public entry points rather than the special-case match, so the migration must leave it passing unmodified

### Implementation for User Story 1

- [X] T028 [US1] Introduce the uniform `FloatingSurface` type and ~~`StackBand`~~/`DismissalRules` in `crates/micold-client/src/overlay/mod.rs`, built on feature 017's existing `Layer`/`Surface`/`Trigger` vocabulary in `crates/micold-core/src/overlay.rs` — not a parallel one (FR-014) — **`StackBand` was not introduced**: it is a second name for `micold_core::overlay::Layer`, which already declares the bands bottom-to-top and derives `Ord` as the z-order, and a synonym is exactly the parallel vocabulary FR-014 forbids. `DismissalRules` exists but decides nothing — it records the surface kind and the cancel message and forwards every trigger question to `dismisses`. Also note **`ui::cdk::overlay::Surface` already is a uniform floating-surface type** (feature 017): it owns render-time concerns (panel, anchor, scrim), while this owns state-time ones (identity, which is open, what closes it, and at T036 the snapshot). Three layers, one rule — recorded in the module doc so `one_overlay_implementation.rs` is not read as having been quietly widened. The trait has **no `view` method**, contrary to the contract sketch: FR-006 forbids feature modules naming the renderer, and Tier 1 already sited views in `crate::ui`; T029's registration line names a surface and its view together so FR-009's one-registration-point still holds
- [X] T029 [US1] Add the registry and its `register!` macro in `crates/micold-client/src/overlay/registry.rs`, with `Overlay`/`ClosingOverlay` still present and deriving into it so both representations coexist green — **the derive is `Overlay::as_surface`**, one function on the enum returning each variant's `SurfaceId` and cancel message. Both readers call it: the registry, and `on_escape`, whose nine-arm match is now that one call. So the two representations cannot answer differently while they coexist, and the equivalence is exhaustively tested (`overlay_registry.rs`, twenty states = ten variants x filter panel open/closed). **Two registrations, not sixteen**: `ModalSurface` (the transitional bridge standing for whichever `Overlay` variant is open — T032 splits it into nine) and `features::sidebar::SidebarFilterPanel` (a genuine feature-owned surface, costing one line). Two is enough to exercise both bands and both dispatch shapes; more would have pre-empted T031/T032. **Dispatch has two shapes on purpose**: `escape` goes to the topmost surface only (contract D1), `scroll_beneath` to every surface it reaches (what `dismiss_on_scroll_beneath` does today). A single `dismiss` entry point would have had to pick one and silently change the other. `probes()` is public so contract R3 can be tested by reordering, which is the only way to test it. Two probes confirm the guard fires: demoting the modal to the popover band, and registering a modal with no cancel message
- [X] T030 [US1] Implement the builder API for `FloatingSurface` terminating in `.into()` per Principle VIII and FR-030, and confirm `crates/micold-client/tests/material_builder_api.rs` still passes — it does (10 tests, unmodified), as does `showcase_completeness.rs` (34). **The `.into()` is `Open`, not an `Element`.** Principle VIII's terminator is conversion into an `iced::Element`, and nothing under `src/overlay/` becomes one — T028 removed the `view` method the contract sketch gave `FloatingSurface` precisely so it could not (FR-006). A surface's element terminator is `ui::cdk::overlay::Surface`, already a Principle VIII component, reached at T035. What this task did apply is the *shape*: `Open::of` became `impl<S: FloatingSurface> From<&S> for Open`, so erasure reads `(&surface).into()` and there is exactly one door out; and `tests/overlay_builder_api.rs` now holds the layer to no public fields and no `&mut self` setters, so the sixteen surfaces T031–T036 add cannot introduce a second way to configure one. The scan is `tests/inventory/mod.rs`'s, pointed at another directory via a new `declarations_in` — restating "what a constructor is" in a second scanner is the drift FR-014 objects to. Two probes confirm it fires. **The rule is stated differently here on purpose**: in the library a non-builder public method needs a place on a sanctioned list, but here readers (`kind`, `id`, `layer`, `on`) are the norm, since asking is what generic dispatch does. **Found by the new guard**: `inventory::struct_body` mis-parsed tuple structs — it walked to the next `{`, which for `SurfaceId(&'static str)` is the `impl` block, so every method in it read as a public field. Fixed for all three struct forms with a unit test; the library has no tuple structs, so its gates are unaffected
- [X] T031 [US1] Migrate the 7 ad-hoc popovers off their loose `State` fields onto the registry — `help_menu_open`, `project_switcher_open`, `sidebar_filter_open`, `worktree_menu_open`, `project_menu_open`, `terminal_context_menu`, `session_menu_open` in `crates/micold-client/src/app.rs` (FR-007) — all seven now have a surface type in the feature module that owns them and one `register!` line each. **The fields stay, and that is the migration finishing rather than stalling**: four of them carry payload (which worktree, which session, where the cursor was), which is feature state, not overlay state. What migrated is *openness* — `Registered::open_in` derives it from whatever the feature already stores — so the three pure bools are the only ones the registry could have absorbed, and absorbing them means moving `State` ownership into the feature modules, which is Tier 3 (T062). `open_overlay` and `dismiss_on_scroll_beneath` are now two calls into the registry instead of ten field assignments across two different subsets. **A tenth feature module, `features/help.rs`**: the overflow menu had no home — two constants at the top of `app.rs` and a `bool` two hundred lines below — and FR-001 asks where a feature lives, not how big it is. `HELP_ACTIONS`/`help_actions` moved with it (import-only updates in `ui/toolbar.rs` and `tests/toolbar.rs`; no assertion touched). Architecture doc's module table needs the tenth row at T038. **Three behaviour changes, each argued and pinned by a test:** (1) *Escape now closes every popover*, where it previously closed only a modal or the filter panel. No widget handles Escape — `cdk::overlay::Surface` observes an outside click and nothing else — and feature 017 added `Surface::dismisses_on` precisely so "callers that own such a trigger consult the same rule rather than re-deciding it"; the keyboard subscription was simply never wired to it. FR-012 preserves the *priority* between simultaneously-open surfaces and the modal-closes-popovers rule, both of which still hold and are asserted; it does not require that a surface Escape never reached keeps not being reached. Pinned by `escape_now_reaches_every_popover`. (2) *Opening a modal now closes all seven popovers, not four.* The three it missed were cleared by hand in the reducer arms that open a modal from them — the fragile arrangement T026 recorded — or, for the terminal context menu, by nothing. (3) *A scroll beneath now closes the terminal context menu too*, the seventh of seven; it is a non-modal surface and the core rule has always said so. **A trap worth recording**: several cancellations are toggles whose reducer arms close their neighbours, so a batch of messages collected up front hands a toggle to an already-closed surface and *reopens* it. `close_each` re-asks which surfaces are open after each close, bounded by the registration count. Probed: the batch version fails feature 017's own frozen `overlay_dismissal_delta`. Two further probes confirm R2 fires — an `open_in` reading its neighbour's field, and a missing `register!` line. `overlay_registration.rs` was rewritten to the subject its own T026 doc said it would take: registration, not two lists of field assignments
- [X] T032 [US1] Migrate the 9 real `Overlay` variants onto the registry, preserving each surface's dismissal rules (the 10th variant, `None`, becomes "nothing open" rather than a surface) — nine surface types, each in the feature module that owns the dialog: `AboutDialog` (help — "About" is the Help menu's single action, so the menu and the dialog it opens are one feature), `ProjectSelectorDialog`/`RenameProjectDialog`/`ConfirmForgetProjectDialog` (project), `AddWorktreeDialog` (worktree_form), `SettingsDialog` (settings), `ConfirmWorktreeDeleteDialog`/`RenameWorktreeDialog` (worktree), `ConfirmSessionRemoveDialog` (session). `ModalSurface` — T029's transitional bridge, one registration standing for nine — is deleted. Sixteen registrations now, and `src/overlay/` holds no surface type at all: the layer describes surfaces without containing any, which is the shape SC-001 is measuring. **The enum survives as storage, not as a description**: each dialog's `open_in` is `state.overlay == Overlay::X`, and `on_escape` still matches on it. Those are two independent statements of the same nine facts — the drift this feature exists to end — so `overlay_registry.rs` holds them equal over every state either can express, on *both* the cancel message and the surface identity, until T034 deletes the second. The identity half is new here and is not redundant: T035 keys the view and the exit animation on identity, so a mistyped id would move a dialog's transition rather than break its dismissal, which the message equivalence cannot see. Both probes fire — a typo'd id, and an `open_in` reading its neighbour's variant. **R2 caught a real omission during this task**, not a hypothetical: rewriting the `register!` list dropped `WorktreeContextMenu`, and `every_popover_is_registered` failed on the next build. That is precisely the "opens but will not close, discovered by hand" failure the guard exists for, found by the guard instead
- [X] T033 [US1] Collapse central match site 1 of 6 — the `Overlay` enum at `crates/micold-client/src/app.rs:55` — onto generic dispatch — **site 1's match is `Overlay::as_surface`, and it is deleted**. The enum declaration itself is not a match statement; the match that lives at site 1 is the nine-arm `impl Overlay` block T029 added as the bridge, holding the enum's own account of each dialog's identity and cancellation. That account is now stated once, in the feature module that owns the dialog, so the enum describes nothing: the variants say only *which slot is filled*, and each dialog's `Registered::open_in` reads that. **Site 2's body went with it, necessarily**: `as_surface` had exactly one caller left after T032, `on_escape`, and the two were one match split over two functions — `on_escape` is now `registry::escape(state)`, one line. T034 keeps site 3 (the keyboard subscription's hand-written mirror) and the priority question, which is where the remaining work in that pair actually is; the `on_escape` wrapper survives until then as the name the scrim and ~30 existing assertions call. **The sequencing in the task text is the one thing that could not be honoured**: site 1 is the storage every other site still matches on, so the enum cannot be removed first — that is T037, and the four-commit chain runs 1(bridge) → 3 → 4,5 → 6 → 1(slot). **The equivalence test lost its subject and gained a better one.** With `as_surface` gone there is no second answer to compare against, so `overlay_registry.rs` now states the nine facts itself, in an exhaustive `expected` match written independently of the code under test. Strictly stronger than the equality it replaces: an equality can only catch the two sides *disagreeing*, never both being wrong. Three probes fire — a dialog cancelling with its neighbour's message (caught by the cancellation test only), a typo'd surface id (caught by the identity test only, confirming that half is not redundant), and a tenth variant added with no expectation (fails to compile). The third currently fails at the *library's* remaining match sites before the test file is reached; the test's own exhaustiveness only becomes the load-bearing one at T037
- [X] T034 [US1] Collapse sites 2 and 3 — `on_escape` at `crates/micold-client/src/app.rs:2322` and its keyboard-subscription mirror at `crates/micold-client/src/ui/mod.rs:519` — preserving the popover-before-modal priority currently hand-written at `ui/mod.rs:554` — site 2 went with site 1 at T033; **this is site 3, and it is where T031's Escape change actually reaches the user.** The registry has answered for every popover since T031, but the live keyboard path never asked it: it was a nine-arm match over the enum with the filter panel hand-checked above it. It now emits `Message::EscapePressed` and the reducer asks the registry — *what happened*, not *what should close*, exactly as `ScrolledBeneathOverlay` has worked since feature 017. The two triggers are now reported the same way, and `State::dismiss_topmost` sits beside `dismiss_on_scroll_beneath` as its deliberate opposite shape (one surface vs. every surface it reaches). **The macro is gone, and its reason with it**: `Subscription::filter_map` needs a zero-sized closure and takes the subscription's identity from its `TypeId`, so naming the message per overlay required one distinct closure *expression* each — otherwise iced kept the previous overlay's recipe alive across a switch and Esc emitted the wrong message. A message that does not name its target cannot be stale, so one shared closure is now correct. What stays in the view layer is the only part that belongs to it: whether to hold a listener open at all, which it does only while Escape has something to close, so Esc with nothing open is as inert as before. **The priority is preserved but not "popover-before-modal"** — the task text has it backwards. The hand-written guard read `overlay == None && sidebar_filter_open`, i.e. the popover is consulted *only when no dialog is open*: dialog outranks popover, contract D1. That is now the `Layer` ordering rather than a guard above a match, and `overlay_dispatch_ordering.rs` (which exists to catch exactly this being reversed "to match the contract's prose") passes unmodified. **A `Subscription` is opaque, so the wiring is held from both ends**: `pressing_escape_closes_the_topmost_surface` drives the message the subscription emits and asserts the topmost surface closed *and nothing else moved*; `the_keyboard_subscription_names_no_surface` reads the function and fails if any overlay variant or any per-surface message reappears in it. Both probes fire — reinstating the filter-panel special case, and making Escape close every open surface instead of one. **`on_escape` survives as a name, not a site**: it is one line delegating to the registry, it is what the scrim and ~30 existing assertions across nine test files call, and moving them onto `overlay::registry` would churn frozen assertions to no benefit — `overlay_dispatch_ordering.rs` says in its own header that it "deliberately drives only the public entry points"
- [X] T035 [US1] Collapse sites 4 and 5 — the view match at `crates/micold-client/src/ui/mod.rs:337` and `capture_overlay` at `crates/micold-client/src/main.rs:727` — **site 4 is done; site 5 moves to T036, and could not have been done here.** `capture_overlay` builds a `ClosingOverlay`, and the shape of that snapshot is precisely what T036 decides (`FloatingSurface::snapshot`, contract A1–A3). Collapsing site 5 first would have meant designing T036's type inside T035 and then rewriting it — the two are one decision, so T036 takes both. Same shape of correction as T033/T034: the site numbering in the contract is a checklist, not a dependency order. **Site 4, the ten-arm view match, is gone.** Each dialog's state lookup moved into the `ui` module that draws it, behind a uniform `dialog(&State, ColorScheme, &EnvIncludeOutcome) -> Option<Element>`, and `ui::view` is now `registry::open_dialog(state)` plus a call through the view it was registered with. **The view is named on the registration line, not on the surface** — this is what T028 promised when it removed `FloatingSurface::view`: FR-006 forbids a feature module naming a rendering framework, and Tier 1 sited views in `crate::ui`, so the two halves cannot live in the same module. They can still be named in one *place*, which is what FR-009 actually asks. The `register!` macro grew an optional `=> view` per line (`$(.drawn_by($view))?`), so a dialog is one line and a popover is still one line — popovers register no view, because their panel is pushed by `ui::view` whether or not they are open, since it owns its own fade and must outlive the flag that opened it. The nine `ui` dialog modules became `pub(crate)` so the registration line can name them. **`Open`'s `PartialEq` is now hand-written**, comparing identity/band/dismissal and ignoring the view: deriving it would compare function pointers, whose addresses the compiler does not promise are unique (rustc warns, and clippy's `-D warnings` rejects it) — and which view draws a surface is not part of what makes it that surface. **The identity keying stays on `overlay_key(Overlay)` for now**, contrary to T032's note that T035 would move it: the live dialog and its fading-out snapshot must share one key space or the exit animation restarts, and the snapshot branch is `ClosingOverlay`, i.e. T036. It is a cast, not a match. **Three guards, all probed**: every dialog is registered *with* a view (a dialog registered without one opens and draws nothing, and dismissal, stacking and identity would all still look right); no popover is; and — the failure the first two cannot see — `a_dialog_draws_from_its_own_state` opens each dialog through the reducer and asserts its registered view actually produces a body, so a line pairing a dialog with *another* dialog's view fails here rather than as a modal that renders empty. Seven of nine are covered that way; the project selector's listing and a session's record come from the binary, not the pure core, so a `State` built in a test has neither. Probes fired for a mispaired view and a missing one
- [X] T036 [US1] Collapse sites 5 and 6 — `capture_overlay` at `crates/micold-client/src/main.rs:727` (deferred here by T035) and the `ClosingOverlay` enum and its impl in `crates/micold-client/src/app.rs` — **but the snapshot did not move onto `FloatingSurface::snapshot`, and that is the finding, not a shortcut.** Written out, every surface's `snapshot` would have been the same line: clone the state, remember which surface was open. A trait method whose every implementation is identical is a hook with nothing per-surface in it, and adding one would have made "add a surface" cost a method body again — the opposite of FR-009. So the snapshot is a single type, `registry::Closing`, holding a `SurfaceId` and a boxed `State` clone. **Three per-surface lists collapse to none**: the enum's nine variants, `capture_overlay`'s ten arms, and `ui::dismissing_dialog`'s nine — one idea ("the state as it was") had been spelled out three times, once per site, which is why sites 5 and 6 were always one decision. **The exit now draws through the same registration as the entrance**: `Closing::surface()` runs `open_dialog` over the snapshotted state, so the fading dialog is rendered by the very function pointer the live path calls, rather than by a second renderer that resembles it. `restart_on` moved off `overlay_key(Overlay)` onto the surface's own identity (`surface_key`, FNV-1a over the `SurfaceId` name) — the keying T035 deliberately left behind because the live dialog and its snapshot must share one key space, and they now do by construction. **Three deliberate behaviour changes, all in the exit animation, all improvements the old shape could not express**: (1) a confirm-worktree-delete fading out now shows its branch line, its keep-branch checkbox and any rename override — `ClosingOverlay::ConfirmDelete(String)` carried only the directory and its arm reconstructed a stripped-down dialog, with a comment saying so; (2) confirm-session-remove reads the session record, still present in the snapshot, instead of a separately captured label; (3) a focused field keeps its focus ring through the fade, where `dismissing_dialog` passed `None` for focus everywhere and the ring vanished a frame before the dialog did. **Cost, stated plainly**: a whole-`State` clone before every message *while a dialog is open*, where it used to be a draft clone — the same gate as before, so nothing is cloned in the common case, and the heaviest thing in `State` is the `WorktreeForm` the old enum already boxed for being several times the size of its siblings. **`overlay_transition_identity.rs` had to be restated, and T037 says it must pass unmodified** — see the note against T037. Four probes, each firing on its own property and nothing else: every snapshot reporting one name (3 fail — faithful, injective, and the new draws-itself check); a dialog whose snapshot is never taken (5 fail); a snapshot that keeps no state (2 fail — contract A1 and A3, and *only* those); and a dialog registered without its view (1 fail here, 2 in `overlay_registry.rs`). Full workspace suite green (182 binaries), clippy `-D warnings` clean
- [ ] T037 [US1] Delete the `Overlay` enum and confirm `crates/micold-client/tests/{one_overlay_implementation,overlay_dismissal_delta,overlay_stacking,overlay_transition_identity}.rs` all pass **unmodified** — **`overlay_transition_identity.rs` is already modified, at T036, and could not not have been** (FR-027). Its subject was `ClosingOverlay`, the type T036 deletes; a test of a type that no longer exists cannot be preserved by keeping its text. The four properties were restated one for one over `registry::Closing`, under the same four test names — faithful, never-nothing, injective, covers every dialog — with nothing weakened: the subject is now the identity the renderer actually keys on rather than an `Overlay` it was translated into, and every snapshot is produced by the real `Closing::of` from a real state instead of being hand-constructed. Two properties were **added**, both of which the old shape could not state: contract A1 (the snapshot still knows how to draw itself, through its own registration) and A3 (it holds no live reference — it goes on saying what it said after the state moves on). The freeze check flags the file; the file's own header comment is the explanation it is flagged against. The other three protected tests are untouched
- [ ] T038 [US1] Write the "adding a floating surface" section of `docs/development/architecture.md`
- [ ] T039 [US1] Add the **permanent** SC-001 guard in `crates/micold-client/tests/surface_registration_cost.rs`, failing if any registered surface is reachable from anywhere beyond its own module and the single registration point (SC-001, SC-002a — clarified 2026-08-07: a permanent guard replaces the one-time file count, which proves the property only on the day it is taken)
- [ ] T040 [US1] Perform quickstart.md procedure M2 (six manual overlay behaviors) and record the result

**Checkpoint**: 19 enum variants and 7 loose fields gone. Six central match statements reduced to
zero. Every pre-existing overlay test passes unmodified.

---

## Phase 5: User Story 3 — Capabilities and shell split (Priority: P2)

**Goal**: Every I/O concern is a narrow declared capability; the binary is the single place real
implementations are chosen; the shell divides by external system. research.md §6, steps 12–16.

**Independent Test**: For each capability, run the behavior depending on it against a fake and
assert the outcome, with no real filesystem, repository, clipboard or OS query involved (SC-005).

**Scoping note**: FR-017 already largely holds — `app.rs` constructs no concrete implementation, and
all nine construction sites are already inside the shell. The real work is FR-018 (single assembly
point); T048's guard is a regression lock on an existing property, not a migration.

### Tests for User Story 3 — write first, observe failing ⚠️

- [ ] T041 [P] [US3] Guard in `crates/micold-client/tests/no_concrete_implementations.rs` asserting non-shell code names no concrete implementation — `GitCli`, `JsonFileStore`, `JsonFileSettingsStore`, `StdFolderScanner` — and that they are constructed in exactly one place (FR-017, FR-018)
- [ ] T042 [P] [US3] Fake-coverage test in `crates/micold-client/tests/service_capability_fakes.rs` asserting every declared capability has a fake and at least one test exercising real behavior through it (FR-019, SC-005), **and** that each capability is narrow enough that no consumer must implement an operation it does not exercise — the spec's own narrowness test, applied per capability (FR-016). A capability failing the narrowness check MUST be split rather than the check relaxed
- [ ] T043 [P] [US3] Behavior test for env-include resolution through its fake in `crates/micold-core/tests/env_include.rs`
- [ ] T044 [P] [US3] Behavior test for the OS theme probe through its fake in `crates/micold-core/tests/os_theme.rs`
- [ ] T045 [P] [US3] Test asserting a feature emits `Outcome::ClipboardWrite` with zero real clipboard access in `crates/micold-client/tests/clipboard_request.rs` (FR-015a, contract C2)

### Implementation for User Story 3

- [ ] T046 [US3] Declare `EnvIncludeResolver` in `crates/micold-core/src/env_include.rs` with a real implementation moved from `crates/micold-client/src/main.rs:397–450` and a fake
- [ ] T047 [US3] Declare `OsThemeProbe` in `crates/micold-core/src/os_theme.rs` wrapping the `dark_light` call at `crates/micold-client/src/main.rs:2678` with a fake — this is the codebase's only direct OS branch, so this also serves Principle VI
- [ ] T048 [US3] Add fakes for any of the seven existing ports lacking one — `ProjectStore`, `SettingsStore`, `FolderScanner`, `TerminalBackend`, `TerminalHandle`, `AiCliProvider` — in `crates/micold-core/src/` beside each capability, as ordinary public items matching `FakeGit` at `crates/micold-core/src/git.rs:467`. **Not** behind a `cfg` feature and **not** in a separate crate (FR-019, clarified 2026-08-07)
- [ ] T049 [US3] Create the `Capabilities` struct in `crates/micold-client/src/shell/capabilities.rs`, assembled once at boot, replacing all nine inline construction sites in `crates/micold-client/src/main.rs` (523, 532, 649, 1295, 1310, 1330, 1924, 2604, 2709) — four of which are inside `update_inner` (FR-018)
- [ ] T050 [US3] Split `crates/micold-client/src/main.rs` startup into `crates/micold-client/src/shell/startup.rs` — `boot`, `window_settings`, `main` — with inline `#[cfg(test)]` tests relocated alongside (FR-019a, FR-027's relocation clause)
- [ ] T051 [US3] Split persistence into `crates/micold-client/src/shell/persist.rs` — `persist`, `persist_settings`, `prune_empty_sessions` — with its tests
- [ ] T052 [US3] Split daemon synchronisation into `crates/micold-client/src/shell/daemon_sync.rs` — `send_op`, `switch_daemon_attachment`, `reconcile_catalog`, `PendingOp` — with its tests
- [ ] T053 [US3] Split subscriptions into `crates/micold-client/src/shell/subscriptions.rs` — `subscription`, `cursor_move_events`, `window_focus_events`, `os_theme_poll` — with its tests
- [ ] T054 [US3] Split the remaining two systems into `crates/micold-client/src/shell/env_include.rs` and `crates/micold-client/src/shell/os_theme.rs` with their tests
- [ ] T055 [US3] Move `update_inner`'s effectful arms from `crates/micold-client/src/main.rs:775–2028` to the shell module addressing each arm's external system
- [ ] T056 [US3] Route clipboard through `Outcome::ClipboardWrite` interpreted by the shell, replacing the three direct `iced::clipboard` calls at `crates/micold-client/src/main.rs:1840, 1847, 1856` (FR-015a)
- [ ] T057 [US3] Write the "adding a capability" section of `docs/development/architecture.md`

**Checkpoint — SC-004b**: Tiers 1 and 2 and the shell split are all merged with **zero Tier 3 work**.
Demonstrating green here is the criterion. Do not start Phase 6 before recording it.

- [ ] T058 [US3] Record the SC-004b demonstration: full suite green on all three platforms with no part of Tier 3 merged, noted in `specs/021-mvu-slice-architecture/baseline.md`

---

## Phase 6: User Story 2 (part 2) + User Story 4 — Reducer split and outcomes (Priority: P1/P2)

**Goal**: The reducer becomes per-feature modules over the shared state, and cross-feature effects
become explicit returned outcomes. research.md §6, steps 17–20.

**Independent Test (US2)**: The root state, messages and reducer contain composition and routing
only — no feature's decision logic (FR-002).
**Independent Test (US4)**: Run the worktree-delete path in isolation; assert it returns outcomes
describing the session and overlay consequences while touching only worktree data. The existing
`worktree_delete.rs` must pass unchanged.

**Applies to both reducers.** Per FR-004a as amended, a feature's pure arms go to its reducer module
and its effectful arms to the shell module for their external system. `app.rs::update` is 778 lines;
`main.rs::update_inner` was 1,253 before Phase 5 reduced it.

### Tests for Users Stories 2 and 4 — write first, observe failing ⚠️

- [ ] T059 [P] [US4] Cross-feature write guard in `crates/micold-client/tests/feature_write_isolation.rs` asserting no feature reducer mutates another feature's data, and **naming the offending path** in its failure message (FR-020, FR-024a, SC-007, contract O6)
- [ ] T060 [P] [US4] Termination test in `crates/micold-client/tests/outcome_termination.rs` asserting outcome interpretation terminates under a cycle and does not depend on composition order (FR-024, contract O4/O5)
- [ ] T061 [P] [US2] Root-routing test in `crates/micold-client/tests/root_is_routing_only.rs` asserting the root reducer contains no feature decision logic (FR-002)

### Implementation

- [ ] T062 [US2] Split `State::update` at `crates/micold-client/src/app.rs:1165–1942` into per-feature reducer modules, one per feature module from Phase 3, each operating on the shared state (FR-004a)
- [ ] T063 [US2] Reduce the root reducer in `crates/micold-client/src/app.rs` to routing only, dispatching to the per-feature reducer modules (FR-002)
- [ ] T064 [US2] Promote the worktree-creation form to a nested unit in `crates/micold-client/src/features/worktree_form.rs` with its own message type absorbing the 22 root variants — 18 `AddWorktree*` and 4 `WorktreeCreate*` — routed through one wrapping variant (FR-003; the sole nested unit per research.md §5)
- [ ] T065 [US4] Introduce the `Outcome` enum in `crates/micold-client/src/features/mod.rs` with the four known variants — `SessionsClosed`, `OverlayDismissed`, `ClipboardWrite`, `NotificationRaised` — and the root's draining interpreter with a fixed iteration bound (FR-021, FR-022, FR-024)
- [ ] T066 [US4] Convert the worktree-delete path to mutate only worktree data and return `SessionsClosed` + `OverlayDismissed`, and confirm `crates/micold-client/tests/worktree_delete.rs` passes **unmodified** (FR-023)
- [ ] T067 [US4] **Discovery**: run T059's guard against the full codebase, enumerate every cross-feature write it names, and record the list in `specs/021-mvu-slice-architecture/cross-feature-writes.md` with a proposed outcome variant for each. This task is complete when the list exists — it converts nothing
- [ ] T067a [US4] **Conversion**: for each write enumerated by T067, convert it to return an outcome, one commit per write. Expand this into one concrete task per entry once T067's list exists — its size is unknowable until then, which is why it is not estimated here (FR-020, FR-021)
- [ ] T068 [US2] Write the "read/write asymmetry" section of `docs/development/architecture.md`, including why guard tests hold the line rather than the type system (plan.md Complexity Tracking)

**Checkpoint**: Zero direct cross-feature reducer writes. Root is routing only. 22 variants left the
root message enum.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T069 Measure SC-003: run `find crates -name '*.rs' -exec wc -l {} + | sort -rn | head -10` and confirm neither `app.rs` nor `main.rs` is near the top and neither holds more than one feature. **FR-005 governs; the 500-line figure is indicative** (clarified 2026-08-07). Record the count as a progress signal — do NOT split a single-feature module to cross a threshold
- [ ] T070 [P] Add the **permanent** SC-002 guard in `crates/micold-client/tests/feature_registration_cost.rs`, failing if adding a feature would require edits beyond its own module and one registration point (SC-002, SC-002a)
- [ ] T071 [P] Verify SC-004 — each of the **eight** feature modules has an isolation test (T007–T014). The overlay registry is a ninth module but not a feature module, and is covered by its own guards instead
- [ ] T072 Perform quickstart.md procedure M1 (persisted state written by the pre-change build loads and behaves identically) and record the result (SC-008, FR-026)
- [ ] T073 Run the FR-027 check across the whole feature branch: `git diff main...HEAD -- crates/*/tests/ | grep -E '^-.*assert'` must show nothing but pure relocations
- [ ] T074 [P] Promote T004's assertion-freeze job from advisory to blocking in `.github/workflows/ci.yml` now its false-positive rate is known
- [ ] T075 [P] Review `docs/development/architecture.md` end to end for coherence and update `docs/README.md` navigation
- [ ] T076 Confirm SC-009 from history: every task above is its own commit, and each commit builds, runs and passes
- [ ] T077 Verify the full suite green on Linux, macOS and Windows (SC-006, Principle VI)
- [ ] T078 Re-measure the baseline table in `specs/021-mvu-slice-architecture/spec.md` against the final tree and record the delta — the figures have moved four times during this feature's life

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies
- **Phase 2 (Foundational)**: Depends on Phase 1. **Blocks everything** — T004 is what makes FR-027 enforceable
- **Phase 3 (US2 part 1, Tier 1)**: Depends on Phase 2. The MVP
- **Phase 4 (US1, Tier 2)**: Depends on Phase 3 — the registry needs feature modules to register into (spec, US2's "Why this priority")
- **Phase 5 (US3, shell)**: Depends on Phase 2 only. **Orthogonal to Tiers 1–3** (FR-019a) — can run parallel to Phases 3–4 with a second developer
- **Phase 6 (US2 part 2 + US4, Tier 3)**: Depends on Phases 3, 4 and 5. Last, because §5's nesting evidence is only trustworthy once boundaries are visible
- **Phase 7 (Polish)**: Depends on all above

### Critical constraint

**Phase 5 must complete and be recorded green (T058) before Phase 6 begins.** SC-004b requires
demonstrating Tiers 1, 2 and the shell split with zero Tier 3 merged. Starting Phase 6 early makes
that criterion unverifiable — it is the one ordering rule that cannot be relaxed for convenience.

### Within each phase

- Tests written and observed failing before implementation (Principle I)
- Extraction tasks are sequential where they touch the same file (all of Phase 3 edits `app.rs`)
- Architectural docs ship inside their own phase, not deferred (Principle VII)

### Parallel Opportunities

- T002, T003 in Setup
- T005, T006 in Foundational
- All eight isolation tests T007–T014 plus the render-free guard T014a (separate files)
- All five capability tests T041–T045 (separate files)
- T059–T061 (separate files)
- **Phase 5 against Phases 3–4** — the shell split is orthogonal by FR-019a, the single largest parallelization win here
- T070, T071, T074, T075 in Polish

**Not parallelizable**: T015–T023 all edit `app.rs`. T028–T037 form a deliberate serial chain so a
bisect lands on one overlay change.

---

## Parallel Example: Phase 3 tests

```bash
# All eight isolation tests are separate files with no shared state.
# Each fails to compile until its module exists — that is the Red state.
# T014a joins them: a regression lock that passes from the start and must keep passing.
Task: "Isolation test for the worktree-creation form in crates/micold-client/tests/features_worktree_form.rs"
Task: "Isolation test for sidebar types in crates/micold-client/tests/features_sidebar.rs"
Task: "Isolation test for project/workspace types in crates/micold-client/tests/features_project.rs"
Task: "Isolation test for settings types in crates/micold-client/tests/features_settings.rs"
Task: "Isolation test for worktree types in crates/micold-client/tests/features_worktree.rs"
Task: "Isolation test for notification types in crates/micold-client/tests/features_notifications.rs"
Task: "Isolation test for session types in crates/micold-client/tests/features_session.rs"
Task: "Isolation test for daemon-connection types in crates/micold-client/tests/features_connection.rs"
```

---

## Implementation Strategy

### MVP (Phase 3 only)

1. Phase 1 Setup → Phase 2 Foundational
2. Phase 3 — Tier 1 feature modules
3. **STOP and VALIDATE**: every feature answers "where does it live?" with one module (SC-010);
   `app.rs` roughly 1,700 lines; suite green on three platforms
4. This is a complete, shippable improvement. Tiers 2 and 3 need not follow immediately.

### Incremental Delivery

Each phase is independently shippable (FR-004c), which is unusual and worth exploiting:

1. Setup + Foundational → safety nets in place
2. Phase 3 → feature modules → **ship** (MVP)
3. Phase 4 → overlay registry → **ship** (removes the largest source of "I forgot a site" bugs)
4. Phase 5 → capabilities and shell split → **ship** (SC-004b checkpoint)
5. Phase 6 → reducer split and outcomes → **ship**
6. Phase 7 → verification and measurement

### Parallel Team Strategy

Two developers, after Phase 2:

- Developer A: Phase 3 → Phase 4 (Tiers 1 and 2, both touching `app.rs`)
- Developer B: Phase 5 (shell split, touching `main.rs`)

The split is clean because FR-019a makes the shell orthogonal to the tiers, and the two developers
touch different files. They converge for Phase 6, which needs both.

---

## Notes

- **Every task is its own commit.** SC-009 is verified from `git log`, not from the endpoint
- Verify tests fail before implementing — for extraction, the existing suite already provides Red
- The existing 71-file suite is frozen (FR-027). Additions and relocations are fine; rewrites and
  deletions are defects
- Re-measure rather than trusting any figure here — the baseline has moved four times in ten days
- **T022 covers a gap**: research.md §6's Tier 1 table lists seven steps for eight features. The
  daemon-connection extraction was missing and is added here
