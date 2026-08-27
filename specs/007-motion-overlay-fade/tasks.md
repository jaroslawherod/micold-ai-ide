---

description: "Task list for Generic Motion Library & Overlay Fade In/Out"
---

# Tasks: Generic Motion Library & Overlay Fade In/Out

**Input**: Design documents from `/specs/007-motion-overlay-fade/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/animation-api.md, quickstart.md

**Tests**: Per Constitution Principle I, the render-free animation **core** is built test-first
(Red-Green-Refactor). The GUI render helpers and overlay wiring have no meaningful headless
unit surface (rendering) — per plan.md's Constitution Check they are validated via
`quickstart.md`, not unit tests.

**Cross-platform**: Pure Rust + iced, no OS branching (Principle VI).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1 / US2 / US3 (Setup & Foundational carry no story label)

## Path Conventions

Single project; paths are repo-root-relative (`src/`, `docs/`), matching plan.md.

---

## Phase 1: Setup

**Purpose**: Register the new module.

- [X] T001 Add `pub mod motion;` to `src/lib.rs` and create an empty `src/motion.rs` with a module doc comment.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Build the extractable, framework-agnostic animation core (the reusable
mechanism) and replace the app's bespoke per-animation system with it, migrating the four
existing animations. Both user stories build on this. Placed in Foundational because
introducing the shared `Animator` requires retiring the old `Anim`/`*_anim` system in one
step — a partial migration would leave a broken dual-system intermediate state.

**⚠️ CRITICAL**: No user story work begins until this phase is complete.

### Core library (test-first — Constitution Principle I)

- [X] T002 Write failing unit tests in `src/motion.rs` (`#[cfg(test)]`) for `Animator<K>` covering contracts C1–C7 from contracts/animation-api.md: converge-and-stop without overshoot (C1), faster `speed` converges in fewer `tick`s (C2), `animating()` true→false as tracks settle (C3), independent keys (C4), `get` default 0.0 (C5), new-key-settled-at-target (C6). Run and confirm they FAIL.
- [X] T003 Implement `Track` and `Animator<K: Copy + Eq + Hash>` (`new`/`set`/`to`/`get`/`tick`/`animating` + `Default`) in `src/motion.rs` to pass T002; std-only, no iced, no app types; document the public API with rustdoc (FR-015/FR-017).
- [X] T004 Verify the core is render-free and app-agnostic: `cargo test --no-default-features motion` passes (contract C7 / SC-008).

### App integration + migrate existing animations

- [X] T005 Add the timing helper `step(d: Duration) -> f32` and duration constants (`OVERLAY_ENTER`=300ms, `OVERLAY_EXIT`=240ms, `MENU_FADE`≈90ms, `MAIN_FADE`≈90ms, `SIDEBAR_SLIDE`≈114ms, `HANDLE_HOVER`≈800ms) in `src/main.rs`, replacing the raw `FADE_STEP`/`SLIDE_STEP`/`HOVER_STEP` floats (FR-013).
- [X] T006 Add `MotionKey { Menu, Sidebar, Main, HandleHover, Overlay }` (Copy+Eq+Hash) in `src/ui/mod.rs` (FR-016).
- [X] T007 Replace App's `menu_anim`/`sidebar_anim`/`main_anim`/`handle_hover_anim` fields with a single `motion: Animator<MotionKey>` in `src/main.rs`; initialize it in `boot()` (snap Sidebar to hidden/shown, Main to 1.0, others to 0.0).
- [X] T008 Add `apply_motion_targets(app)` in `src/main.rs` that sets targets+speeds for Menu/Sidebar/Main/HandleHover from state; call it in `Message::AnimationTick` then `app.motion.tick()`, deleting the four `approach()` lines (FR-007).
- [X] T009 Rewire `subscription()` in `src/main.rs` to run the `ANIM_TICK` clock while `app.motion.animating()` (targets applied via a shared pure helper), removing the four-way target check (FR-014).
- [X] T010 Delete `ui::Anim`; change `ui::view` to take `&Animator<MotionKey>` and read menu/sidebar/main/handle_hover via `motion.get(..)` in `src/ui/mod.rs`.
- [X] T011 Update `view()` in `src/main.rs` to pass `&app.motion` (remove the `Anim` construction).

**Checkpoint**: App builds; the four existing animations now run through the shared `Animator` and behave as before; the extractable core is complete and tested.

---

## Phase 3: User Story 1 - Modal overlays ease in and out (Priority: P1) 🎯 MVP

**Goal**: Every modal overlay fades in on open and fades out on close (Cancel, Esc, successful submit), revealing the app beneath during exit.

**Independent Test**: Open/close each of the five overlays via every dismissal path and observe a visible fade in on open and fade out on close, with the app behind re-appearing during exit (quickstart §3, §5).

> **Tests**: Overlay transitions are rendering; there is no headless unit surface. Validated via `quickstart.md` (Constitution Check, plan.md).

- [X] T012 [P] [US1] Add a `scale(content, progress)` transform primitive (scale-about-center via `with_transformation`, passthrough widget like `slide`) in `src/ui/material/animation.rs`, and export it from `src/ui/material/mod.rs`.
- [X] T013 [US1] Add the reusable `Modal` builder in `src/ui/material/modal.rs` (`Modal::new(base, dialog, roles).progress(p).into()`; renders `base` at `p<=0.001`, else `base` + animated scrim `fill_quad` alpha=`p*SCRIM_ALPHA` + centered `scale(dialog, p)` + `opaque` capture); export from `src/ui/material/mod.rs` (FR-001/FR-003/FR-011, depends on T012).
- [X] T014 [US1] Define `ClosingOverlay` enum (About / Selector / Rename / Worktree(form, error) / Settings) and add `dismissing: Option<ClosingOverlay>` to `App` (init `None` in `boot()`) in `src/main.rs`.
- [X] T015 [US1] Implement the overlay motion lifecycle in the `update` wrapper in `src/main.rs`: snapshot the open overlay + its draft **before** `update_inner`; on `Some(X)→None` set `dismissing` + `motion.to(Overlay, 0.0, step(OVERLAY_EXIT))`; on open set `dismissing=None`, `motion.set(Overlay,0.0)`, `motion.to(Overlay,1.0, step(OVERLAY_ENTER))`; in `AnimationTick`, drop `dismissing` once `motion.get(Overlay) <= 0.001` (FR-002/FR-006).
- [X] T016 [US1] Extend `apply_motion_targets`/`subscription` in `src/main.rs` so the animation clock runs while the `Overlay` track is in flight (FR-014).
- [X] T017 [P] [US1] Refactor `about::modal` in `src/ui/about.rs` to build only its dialog body and return `Modal::new(base, dialog, roles).progress(p).into()`; add a `progress: f32` parameter.
- [X] T018 [P] [US1] Refactor `project_selector::modal` in `src/ui/project_selector.rs` the same way (accept `progress`).
- [X] T019 [P] [US1] Refactor `rename::modal` in `src/ui/rename.rs` the same way (accept `progress`).
- [X] T020 [P] [US1] Refactor `worktree_form::modal` in `src/ui/worktree_form.rs` the same way (accept `progress`).
- [X] T021 [P] [US1] Refactor `settings_form::modal` in `src/ui/settings_form.rs` the same way (accept `progress`).
- [X] T022 [US1] Wire overlay rendering in `src/ui/mod.rs` (and `view()` in `src/main.rs` to pass the `dismissing` snapshot): when an overlay is open render it live with `progress = motion.get(Overlay)`; else if `dismissing` is set render the snapshot via the same modal fns with the same progress; else render `base` (FR-001/FR-002/FR-012).
- [X] T023 [US1] Add a "Motion & animations" note to `docs/user-guide/appearance-theming.md` describing overlay fade in/out (Constitution Principle VII).
- [X] T024 [US1] Validate US1 per quickstart §3 & §5: fade in/out for all five overlays across Cancel/Esc/submit, invalid-submit stays open, reveal-beneath, reopen-during-exit, rapid-toggle, quit-mid-animation.
      (2026-08-25 — **ran, and it failed**. Recorded at 60 fps against the shipped binaries;
      evidence: `evidence/T024-quickstart-pass.md`. Overlays **enter** with a visible transition
      (About grows 530×256 → 560×278 over ~16 rendered steps, scrim dimming with it) and **exit in
      a single frame** — About and Settings, via Cancel, via Esc and via a successful submit, four
      for four. Filed as `bugs/BUG-001.md`; FR-002/FR-003 are not met. Passing here: invalid-submit
      keeps the overlay open with its error, rapid-toggling never wedges one part-way, and
      reopening straight after a dismissal renders correctly. Reveal-beneath and reopen-during-exit
      are unanswerable while BUG-001 stands — the interval they describe does not exist.
      Quit-mid-animation is unreachable on a WM-less Xvfb: `xdotool windowclose` panics inside
      `Surface::configure` under lavapipe **at rest as well**, so it is a harness artefact and the
      control run says so. The task is ticked as *run*, with the failure carried by the bug rather
      than by leaving the pass permanently open — the 005 T058 / 001 T033 precedent.)

**Checkpoint**: Overlays fade in and out; MVP is shippable.

---

## Phase 4: User Story 2 - One reusable, extractable animation library (Priority: P2)

**Goal**: The animation core is proven reusable across components and extractable for use outside this project; existing animations verified unchanged.

**Independent Test**: The `motion` core compiles/tests with no app-specific code (`--no-default-features`); adding a new animated element is single-site; the four migrated animations behave as before (quickstart §4, §7).

- [X] T025 [P] [US2] Add an explicit extractability regression test in `src/motion.rs` asserting the core has no app/iced dependency (compiles + passes under `--no-default-features`), formalizing contract C7 / SC-008.
- [X] T026 [US2] Write module-level rustdoc in `src/motion.rs` documenting the public API for external consumers, with a self-contained usage example independent of this app (FR-017).
- [X] T027 [US2] Confirm single-site registration (FR-008/SC-004): verify no residual `*_anim` fields or per-animation `approach` calls remain in `src/main.rs`, and record (module docs/comment) the one-place recipe for adding an animated element.
- [X] T028 [US2] Regression-verify the four migrated animations (overflow menu, sidebar, main view, resize-handle hover) behave identically to before, per quickstart §4 (SC-005).
      (2026-08-25 — measured at 60 fps, evidence as T024. Sidebar collapse 25 frames / expand 26
      frames, a new value every frame; resize-handle hover ramps over ≥34 frames; the button ripple
      runs 8 frames. Those three are intact. The overflow menu **opens** in 3 frames but **closes in
      one**, and the main-view switch is a single frame — both are the same
      leaving-does-not-animate defect as T024, and are covered by `bugs/BUG-001.md` rather than by a
      second report.)

**Checkpoint**: Reusable/extractable core proven; existing animations preserved.

---

## Phase 5: User Story 3 - Perceptible, consistent, tunable timing (Priority: P3)

**Goal**: Overlay motion is clearly perceptible and its timing is legible/tunable.

**Independent Test**: Each overlay open (~300ms) and close (~240ms) is plainly visible (not a flash); durations are expressed as time units in one place (quickstart §3 timing check).

- [X] T029 [US3] Confirm overlay timing is `OVERLAY_ENTER`=300ms / `OVERLAY_EXIT`=240ms and that all timings are expressed as legible `Duration` constants (not opaque per-frame steps) in `src/main.rs` (FR-005/FR-013).
- [X] T030 [US3] Validate perceptibility per quickstart §3 (each transition within the 0.15–0.5s band, clearly visible); tune the duration constants if needed (FR-004/SC-002/SC-003).
      (2026-08-25 — the **enter** is clearly perceptible and in band; no tuning needed. The exit's
      nominal 200 ms is in band too and is simply never drawn (`bugs/BUG-001.md`), so SC-002 is
      vacuously failed on the way out and no constant would fix it. Note also that T029's wording is
      stale: `OVERLAY_ENTER`/`OVERLAY_EXIT` moved out of `main.rs` into `ui/material/modal.rs` with
      017 and now read `duration::MEDIUM_2` (300 ms) and `duration::SHORT_4` (200 ms) — still legible
      named `Duration`s in one place, but the exit is 200 ms, not the 240 ms recorded there.)

**Checkpoint**: Motion is clearly perceptible and tunable.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T031 [P] Cross-cutting docs review: ensure the appearance-theming note is linked from the user-guide index in `docs/`.
- [X] T032 Run the full `quickstart.md` end-to-end (all sections).
      (2026-08-25 — `evidence/T024-quickstart-pass.md` covers §3, §4, §5 and §6. §6's idle-cost
      claim reads 28.9–31.0% CPU here, but the thread breakdown puts 29.3% of that in the llvmpipe
      software-rasteriser workers against 1.0% on the app's own main thread, so the application is
      idle and the figure is the rasteriser; it is not filed as a defect, and the
      renderer-independent form of the claim is already held by the green `idle_requests_no_frames.rs`
      and `idle_subscriptions.rs`. Not covered, and stated as such in the evidence: the rename,
      add-worktree and project-switcher overlays individually — About and Settings were exercised on
      all three dismissal paths and all five share `material::Modal`, so they should be re-checked
      when BUG-001 is fixed.)
- [X] T033 Verify `cargo fmt --check`, `cargo clippy` (no warnings), `cargo test`, and `cargo test --no-default-features` all pass.
- [X] T034 Confirm the change builds and tests on Linux, macOS, and Windows in CI (Constitution Principle VI). *(2026-08-20: satisfied by the three-OS CI matrix added in `10a1fe7` (2026-07-20) — `.github/workflows/ci.yml` builds the whole workspace and runs the render-free core suite plus the component gates on ubuntu/macos/windows for every code-affecting change, and has been green on all three since. Latest run: [32302430171](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/32302430171). The full GUI suite and clippy stay Linux-only by design — that is the only runner with the iced system deps.)*

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: none.
- **Foundational (Phase 2)**: depends on Setup; BLOCKS all user stories.
- **US1 (Phase 3)**: depends on Foundational. The MVP.
- **US2 (Phase 4)**: depends on Foundational (verification/hardening of the core built there); independent of US1.
- **US3 (Phase 5)**: depends on Foundational (+ US1 present to observe overlay timing).
- **Polish (Phase 6)**: depends on all desired stories.

### Within Foundational

- T002 (failing tests) → T003 (impl) → T004 (verify) — strict Red-Green (Principle I).
- T005–T011 are the migration; T007→T008→T009→T011 all touch `src/main.rs` (sequential); T006 & T010 touch `src/ui/mod.rs` (sequential to each other).

### Within US1

- T012 → T013 (Modal uses `scale`). T014 → T015 → T016 (`src/main.rs`, sequential). T017–T021 are different files ([P]). T022 depends on T013 + T017–T021. T023 [P]. T024 last (validates all).

### Parallel Opportunities

- US1 overlay refactors T017, T018, T019, T020, T021 — different files, run in parallel.
- T012 (animation.rs) parallel with the T014/T017–T021 group (different files) but before T013.
- US2 and US3 verification tasks can proceed once Foundational + US1 exist.

## Parallel Example: User Story 1 overlay refactors

```bash
Task: "Refactor about::modal to use Modal in src/ui/about.rs"
Task: "Refactor project_selector::modal to use Modal in src/ui/project_selector.rs"
Task: "Refactor rename::modal to use Modal in src/ui/rename.rs"
Task: "Refactor worktree_form::modal to use Modal in src/ui/worktree_form.rs"
Task: "Refactor settings_form::modal to use Modal in src/ui/settings_form.rs"
```

## Implementation Strategy

### MVP First

1. Phase 1 Setup → 2. Phase 2 Foundational (core + migration) → 3. Phase 3 US1 (overlay fade) → **STOP & VALIDATE** (quickstart §3/§5) → demo.

### Incremental Delivery

Foundational (reusable core + existing animations migrated) → US1 (overlay fade MVP) → US2 (extractability proof + docs) → US3 (timing polish) → Polish. Each step is independently valuable and non-breaking.
