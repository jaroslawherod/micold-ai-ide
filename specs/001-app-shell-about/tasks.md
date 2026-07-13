---

description: "Task list for Application Shell with Help / About"
---

# Tasks: Application Shell with Help / About

**Input**: Design documents from `/specs/001-app-shell-about/`

**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅

**Tests**: MANDATORY per Constitution Principle I (Test-First, NON-NEGOTIABLE). Every user story writes failing, reviewed tests BEFORE its implementation (Red-Green-Refactor).

**Documentation**: MANDATORY per Constitution Principle VII. Each user-facing story ships its user-guide docs in the same change.

**Cross-platform**: Per Constitution Principle VI, build + tests MUST pass on Linux, macOS, and Windows. Core logic stays OS-agnostic.

**Organization**: Tasks grouped by user story for independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1 / US2 / US3 (maps to spec.md user stories)
- Exact file paths included in every task

## Path Conventions

Single-project desktop app (per plan.md). Rust **lib + bin** layout: `src/lib.rs` exposes the
render-free core so integration tests in `tests/` can drive it; `src/main.rs` is a thin
binary. Paths are repo-relative.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Bootstrap the Rust + iced project — this is the repository's first code.

- [X] T001 Initialize the Cargo project at repo root: create `Cargo.toml` with package metadata (`name = "micold-ai-ide"`, `version`, a one-line `description`, `license`), an `iced` dependency (latest stable 0.13+ line, pinned), and both `[lib]` and `[[bin]]` targets, per plan.md and research.md R1/R2.
- [X] T002 Add the Rust stable toolchain to `mise.toml` alongside the existing `uv` entry (closes the constitution follow-up TODO; satisfies Technology Constraints).
- [X] T003 [P] Add `rustfmt.toml` and clippy configuration so `cargo fmt --check` and `cargo clippy -- -D warnings` run clean.
- [X] T004 [P] Add the root `Apache-2.0` `LICENSE` file (full Apache License 2.0 text) and set `Cargo.toml` `license = "Apache-2.0"` (SPDX). Decision confirmed by project owner; closes the second constitution follow-up TODO. Required for a correct About display (FR-008).
- [X] T005 [P] Create `.github/workflows/ci.yml`: build + `cargo test` matrix on `ubuntu-latest`, `macos-latest`, `windows-latest`, plus `cargo fmt --check`, `cargo clippy`, and a docs check (Principle VI + TDD gate).
- [X] T006 [P] Create the `docs/user-guide/` directory and a top-level `docs/` index entry so the docs pipeline exists (Principle VII).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core type scaffolding and the iced bootstrap that every user story builds on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T007 Define core domain types in `src/app.rs`: `State` (holds `metadata` + `overlay`), the `Message` enum (`HelpMenuToggled`, `AboutOpened`, `AboutClosed`), and `enum Overlay { None, About }`, per data-model.md.
- [X] T008 Create `src/lib.rs` exposing the render-free core (`app`, `metadata`, `ui` modules) as the crate's public API so `tests/` integration tests can drive `update`.
- [X] T009 Implement the iced application bootstrap: thin `src/main.rs` that calls the lib to run the app, plus `update()`/`view()` wiring in `src/app.rs` that launches a single bare main window (no toolbar content yet).
- [X] T010 [P] Create `src/ui/mod.rs` module wiring (declares `toolbar` and `about` submodules).

**Checkpoint**: App compiles and launches an empty window on all platforms — stories can begin.

---

## Phase 3: User Story 1 - Launch to a working application window (Priority: P1) 🎯 MVP

**Goal**: Launching the app shows a single main window with a top toolbar whose only entry is "Help".

**Independent Test**: Run `cargo run` → one window with a top toolbar showing exactly "Help" and no other entries.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST; ensure they FAIL before implementation.

- [X] T011 [P] [US1] Integration test in `tests/toolbar.rs`: the toolbar entry list equals exactly `["Help"]` — no other entries (FR-002, FR-003).

### Implementation for User Story 1

- [X] T012 [US1] Implement the toolbar in `src/ui/toolbar.rs`, exposing a render-free `toolbar_entries() -> [&str]` returning `["Help"]` (testable core) and a `view` rendering a top toolbar with the single "Help" entry (FR-001, FR-002, FR-003).
- [X] T013 [US1] Wire the toolbar into `app.rs` `view()` so launching shows the window with the toolbar across the top (FR-001).
- [X] T014 [US1] Document the main window + toolbar in `docs/user-guide/help-about.md` (Principle VII).

**Checkpoint**: US1 fully functional and independently testable — this is the MVP.

---

## Phase 4: User Story 2 - View application information via Help → About (Priority: P1)

**Goal**: Selecting "Help" reveals "About"; activating it opens a modal overlay showing name, version, license, and description.

**Independent Test**: With the window open, Help → About opens the dialog and all four fields show correct, non-empty values.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST; ensure they FAIL before implementation.

- [X] T015 [P] [US2] Test in `tests/metadata.rs`: `AppMetadata` resolves `name == "Micold AI IDE"` and `version` from `CARGO_PKG_VERSION`; empty `license`/`description` → `"unknown"` fallback (FR-006, FR-007, FR-008, FR-009, FR-016).
- [X] T016 [P] [US2] Test in `tests/about_open.rs`: `update(AboutOpened)` moves `Overlay::None → About`; a second `AboutOpened` stays `About` (idempotent, single instance) (FR-005, FR-015).

### Implementation for User Story 2

- [X] T017 [P] [US2] Implement `AppMetadata` in `src/metadata.rs`: read `CARGO_PKG_VERSION` / `CARGO_PKG_DESCRIPTION` / `CARGO_PKG_LICENSE` via `env!`, `const APP_NAME = "Micold AI IDE"`, and the empty-string → `"unknown"` fallback rule (FR-006/007/008/009/016).
- [X] T018 [P] [US2] Implement the Help menu in `src/ui/toolbar.rs`: selecting "Help" reveals only the "About" action; activating it emits `Message::AboutOpened` (FR-003, FR-004).
- [X] T019 [P] [US2] Implement the About modal overlay in `src/ui/about.rs`: an in-window `stack` overlay with a dimmed, input-blocking backdrop, rendering name/version/license/description (FR-013; contract C5).
- [X] T020 [US2] Add the `AboutOpened` arm to `app.rs` `update()` — idempotent set to `Overlay::About` and move focus into the dialog (FR-005, FR-014, FR-015). (edits `app.rs`)
- [X] T021 [US2] Render the overlay in `app.rs` `view()` when `Overlay::About`, with focus landing on the Close button (FR-013, FR-014). (edits `app.rs`; depends on T019, T020)
- [X] T022 [US2] Add the "About" section to `docs/user-guide/help-about.md` (Principle VII).

**Checkpoint**: US1 + US2 both work independently; the About dialog displays all four fields.

---

## Phase 5: User Story 3 - Dismiss the About dialog and return (Priority: P2)

**Goal**: Close the dialog via the Close button or Esc, returning to the unchanged main window.

**Independent Test**: With the dialog open, dismiss via Close (one run) and via Esc (another); dialog disappears and the window is focused and unchanged both times.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST; ensure they FAIL before implementation.

- [X] T023 [P] [US3] Test in `tests/about_dismiss.rs`: `update(AboutClosed)` moves `Overlay::About → None`; `AboutClosed` while `None` is a no-op (FR-010, FR-011, FR-012, edge case).
- [X] T024 [P] [US3] Test in `tests/keyboard.rs`: Esc maps to `AboutClosed` only while `Overlay::About`; Esc while `None` produces no About message (FR-011, edge case).

### Implementation for User Story 3

- [X] T025 [US3] Add the `AboutClosed` arm to `app.rs` `update()`: `Overlay::About → None`, no-op when `None`, return focus to the main window (FR-010/011/012/014). (edits `app.rs`)
- [X] T026 [P] [US3] Add a "Close" button to `src/ui/about.rs` emitting `Message::AboutClosed` (FR-010).
- [X] T027 [US3] Add the keyboard subscription in `app.rs` mapping Esc → `AboutClosed`, gated on `Overlay::About` (FR-011, edge case). (edits `app.rs`; depends on T025)
- [X] T028 [US3] Add the dismissal section to `docs/user-guide/help-about.md` (Principle VII).

**Checkpoint**: All three stories independently functional; full launch → Help → About → dismiss round trip works.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Cross-cutting quality, docs review, and the cross-platform verification gate.

- [X] T029 [P] Cross-cutting docs review and `docs/` index/navigation update (no per-feature docs deferred here — those shipped in their stories).
- [X] T030 [P] Add inline unit tests for remaining edge cases in `src/ui/about.rs` (long license/description wraps without hiding Close) and repeated-open behavior.
- [X] T031 Run `cargo fmt --check` and `cargo clippy -- -D warnings` clean across the crate.
- [ ] T032 Verify `cargo build` and `cargo test` pass on Linux, macOS, and Windows via CI (Principle VI, SC-005).
- [ ] T033 Run the `quickstart.md` manual walkthrough (steps 1–9) on each platform and confirm parity.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories.
- **User Stories (Phase 3–5)**: All depend on Foundational. US1 → US2 → US3 in priority order; US2 renders inside the toolbar/window US1 establishes, US3 dismisses the dialog US2 opens.
- **Polish (Phase 6)**: Depends on all targeted stories being complete.

### User Story Dependencies

- **US1 (P1)**: After Foundational. No dependency on other stories. MVP.
- **US2 (P1)**: After Foundational. Uses the toolbar from US1 (Help menu attaches to it) but the About open transition + metadata are independently testable.
- **US3 (P2)**: After Foundational. Dismisses the US2 dialog; its close/Esc transitions are independently testable via `update`.

### Within Each User Story

- Tests written and FAILING before implementation (Principle I).
- Render-free core (metadata, `update` arms) before/independently of `view` wiring.
- `app.rs`-editing tasks within a story are sequential (same file): US2 T020 → T021; US3 T025 → T027.
- User-guide docs ship with the story (Principle VII).

### Parallel Opportunities

- Setup: T003, T004, T005, T006 in parallel (T001 first, T002 depends on nothing but pairs with T001).
- Foundational: T010 parallel with T007–T009 wiring once types exist.
- US2 tests T015 + T016 parallel; impl T017 + T018 + T019 parallel (distinct files: `metadata.rs`, `toolbar.rs`, `about.rs`).
- US3 tests T023 + T024 parallel; T026 parallel with T025 (distinct files).

---

## Parallel Example: User Story 2

```bash
# Tests first (different files) — write failing, then review:
Task: "Test AppMetadata resolution + fallback in tests/metadata.rs"
Task: "Test AboutOpened transition + idempotency in tests/about_open.rs"

# Then implementation across distinct files in parallel:
Task: "Implement AppMetadata in src/metadata.rs"
Task: "Implement Help menu (reveal About) in src/ui/toolbar.rs"
Task: "Implement About overlay rendering in src/ui/about.rs"
# T020/T021 (both edit src/app.rs) run sequentially after these.
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 Setup → 2. Phase 2 Foundational → 3. Phase 3 US1 → **STOP & VALIDATE**: `cargo run` shows a window with a "Help"-only toolbar; `cargo test` green. Demo the shell.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. US1 → runnable window + toolbar (MVP).
3. US2 → About dialog with identity/version/license/description.
4. US3 → dismissal round trip.
Each story adds value without breaking the previous.

---

## Notes

- License value (T004): **Apache-2.0**, confirmed by project owner (SPDX `Apache-2.0`). Set in `Cargo.toml` `license` and shipped as the root `LICENSE` file; the About dialog reads it via `CARGO_PKG_LICENSE`.
- `[P]` = different files, no incomplete-task dependency.
- Verify each story's tests FAIL before implementing (Principle I).
- A story is "done" only when its tests pass, its docs exist, and it works on Linux, macOS, and Windows (Principles I, VI, VII).
- Commit after each task or logical group.

## Implementation status (2026-07-13)

- **Completed (T001–T031)**: Rust + iced project bootstrapped; render-free logic core with
  18 passing tests (`cargo test --no-default-features`); iced GUI (toolbar, Help→About,
  modal About overlay, Close button, Esc subscription) builds clean (clippy + fmt) and the
  binary launches a real 800×600 window on Linux.
- **Architecture note**: iced is an **optional `gui` feature** used only by the binary; the
  lib core (state, `update`, metadata) is iced-free so logic tests run without building the
  GUI stack. The pure `toolbar_entries()`/`help_actions()`/`on_escape` live in the lib; the
  `ui` module (iced rendering) is bin-only — a refinement of the plan's structure.
- **Known gap — FR-014 (focus management)**: Esc-to-close and click-to-close both work, but
  moving keyboard focus *into* the dialog (onto Close) and back to the window on close is
  **not implemented**: iced 0.13 `button` is not focusable in the default focus chain, so
  there is no supported way to focus it. Deferred; the primary flows are unaffected.
- **T032 (open)**: build + tests verified on **Linux** locally; the `.github/workflows/ci.yml`
  matrix runs macOS + Windows but has not executed (requires a push).
- **T033 (open)**: app launch + window creation verified on Linux; the full 9-step manual
  click-through and macOS/Windows parity remain to be run (no headless UI driving here).
