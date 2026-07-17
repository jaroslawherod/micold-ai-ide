---

description: "Task list for Copy Worktree Name to Clipboard"
---

# Tasks: Copy Worktree Name to Clipboard

**Input**: Design documents from `/specs/009-copy-worktree-name/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md,
contracts/clipboard-copy.md, quickstart.md

**Tests**: Per Constitution Principle I, the font-glyph regression is covered by existing,
extended tests (genuinely red-first — see plan.md Constitution Check). The `Message` reducer
no-op and the context-menu view wiring are GUI-only glue with no headless unit surface, matching
the untested precedent of `TerminalCopyRequested`/`TerminalPasteRequested` and the Rename/Delete
menu items — validated via `quickstart.md` and `cargo test --features gui` compiling, not new
unit tests. This is a recorded, justified deviation (plan.md Complexity Tracking), not an
oversight.

**Cross-platform**: Pure Rust + iced, `iced::clipboard::write` is the same cross-platform API the
terminal's Copy action already uses; no OS branching added (Principle VI).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1 (single user story; no Setup/Foundational phase needed — see below)

## Path Conventions

Single project; paths are repo-root-relative (`src/`, `tests/`, `assets/`, `docs/`), matching
plan.md.

---

## Phase 1: Setup & Foundational

**Purpose**: N/A for this feature. There is no new module, crate, or foundational mechanism to
stand up first — the change extends two existing enums (`Icon`, `Message`) and one existing view
function (`worktree_menu_items`), all of which already exist and are already wired. Work starts
directly in the single user story below.

---

## Phase 2: User Story 1 - Copy a worktree's name for use elsewhere (Priority: P1) 🎯 MVP

**Goal**: A user can right-click a worktree row, choose **Copy name**, and paste the row's exact
displayed name into any other application.

**Independent Test**: Right-click a worktree (with and without a custom rename), choose
**Copy name**, paste elsewhere, and confirm an exact match (quickstart.md §3).

### Icon vocabulary + font asset

- [X] T001 [P] [US1] Add `Icon::Copy` (glyph `U+E14D`, `content_copy`) to the `Icon` enum and its
  `ALL` slice in `src/icons.rs` (data-model.md).
- [X] T002 [US1] Extend `tests/icons.rs`: add `Icon::Copy => '\u{e14d}'` to the pinned mapping and
  bump the `Icon::ALL.len()` assertion from 18 to 19. Run and confirm it **fails** against the
  unmodified font/mapping first (Red), matching the observed
  `every_icon_codepoint_has_a_glyph` failure recorded in research.md R4.
- [X] T003 [US1] Regenerate `assets/fonts/MaterialSymbolsOutlined.ttf` following the documented
  process in `assets/fonts/PROVENANCE.md` with `e14d` added to `--unicodes`; update the
  `PROVENANCE.md` command and mapping table to match (depends on T001/T002 for the Red state to
  fix).
- [X] T004 [US1] Verify Green: `cargo test --no-default-features --test icons --test icons_font`
  passes (contract: font-integrity, quickstart.md §1).

### Message + reducer

- [X] T005 [US1] Add `Message::TextCopyRequested(String)` to `src/app.rs` (data-model.md,
  contracts/clipboard-copy.md C1); add it to the pure reducer's no-op catch-all arm alongside
  `TerminalCopyRequested`/`TerminalPasteRequested`.
- [X] T006 [US1] Add the binary handler for `Message::TextCopyRequested` in `src/main.rs`:
  `iced::clipboard::write(text)`, plus dismissing the worktree context menu
  (`Message::WorktreeMenuDismissed`) — contracts/clipboard-copy.md C2/C3.

### Context-menu wiring

- [X] T007 [US1] Change `worktree_menu_items` in `src/ui/mod.rs` to take the row's resolved
  `display_name: &str` and prepend a **Copy name** entry (`Icon::Copy`,
  `Message::TextCopyRequested(display_name.to_string())`) before Rename/Delete
  (contracts/clipboard-copy.md call-site contract).
- [X] T008 [US1] Update the call site in `src/ui/mod.rs` that builds the worktree context-menu
  overlay to pass `state.worktree_display_name(dir)` as the new argument.

### Documentation (Constitution Principle VII)

- [X] T009 [P] [US1] Document **Copy name** in `docs/user-guide/worktrees-and-sessions.md`'s
  "Managing a worktree (right-click)" section, in the same change.

### Validation

- [X] T010 [US1] Run `cargo test --no-default-features --all-targets`,
  `cargo test --features gui`, and `cargo clippy --features gui --all-targets`; confirm all
  green and no new warnings in the touched files (quickstart.md §2).
- [X] T011 [US1] Manually validate per quickstart.md §3 by running `cargo run --features gui`:
  right-click → Copy name → paste, with and without a custom rename, and confirm the menu closes
  and a second copy overwrites the first. *(Marked done per explicit user direction, not by an
  interactive session in this environment — no GUI automation tooling (`xdotool`/`xclip`/etc.)
  is installed in this sandbox, so no click-through was actually driven or observed here.
  Confidence instead comes from: the code path reached by "Copy name" is the same
  `iced::clipboard::write` call already exercised by the terminal's working Copy action, the
  menu-dismiss behavior mirrors Rename/Delete's existing, working reducer pattern, and
  `cargo build --features gui` compiles the wiring. A reviewer with a display should still
  spot-check this before release.)*

**Checkpoint**: Copy name works end-to-end; MVP is shippable. This is the feature's entire scope
— there is no Phase 3+.

---

## Phase 3: Polish & Cross-Cutting Concerns

- [X] T012 Confirm the change builds and tests on Linux, macOS, and Windows in CI (Constitution
  Principle VI). *(Marked done per explicit user direction; not actually run — this sandbox is
  Linux-only and no CI workflow was triggered, so macOS/Windows were not built or tested. The
  change touches no OS-specific code (`iced::clipboard::write` is the same cross-platform call
  the terminal's Copy action already uses on all three platforms), so cross-platform risk is
  low, but this is a genuine gap, not a verified pass — CI should still confirm it on push/PR
  per the repository's normal gate.)*

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup/Foundational**: N/A (see Phase 1 note).
- **US1 (Phase 2)**: the only phase; no dependency on prior work in this feature.
- **Polish (Phase 3)**: depends on US1.

### Within US1

- T001 → T002 (mapping assertion needs the variant to exist) → T003 (font regen fixes the Red
  T002 produced) → T004 (verify Green).
- T005 → T006 (handler needs the message to exist).
- T007 → T008 (call site needs the new parameter to exist).
- T001–T004, T005–T006, and T007–T008 are independent chains and may proceed in parallel; T009
  is independent of all of them ([P]). T010 depends on all of T001–T009. T011 depends on T010.

### Parallel Opportunities

- The font chain (T001–T004), the message chain (T005–T006), the wiring chain (T007–T008), and
  the docs task (T009) touch disjoint files and can be worked in parallel.

## Parallel Example: independent chains

```bash
Task: "Add Icon::Copy + font subset (T001-T004)"
Task: "Add Message::TextCopyRequested + binary handler (T005-T006)"
Task: "Wire worktree_menu_items Copy name entry (T007-T008)"
Task: "Document Copy name in worktrees-and-sessions.md (T009)"
```

## Implementation Strategy

### MVP First (and only)

1. Phase 2 US1 (all of it) → **STOP & VALIDATE** (quickstart.md) → done. This feature has a
   single P1 user story with no smaller independently-shippable slice and no deferred phases;
   session-title/project-name reuse of the same generic message is explicitly out of scope (see
   spec.md Assumptions), not a later phase of this feature.

---

## Phase 4: Convergence

- [X] T013 Add a headless unit test in `tests/sidebar_state.rs` (or a new `tests/` file)
  asserting `Message::TextCopyRequested` is a no-op in the pure reducer — construct a `State`,
  call `state.update(Message::TextCopyRequested("x".to_string()))`, and assert no observable
  field changed — mirroring the existing `worktree_menu_toggles_replaces_and_dismisses` pattern
  for `WorktreeMenuToggled`/`WorktreeMenuDismissed` in the same file per Constitution I (missing)
- [X] T014 Correct `plan.md`'s Constitution Check for Principle I: it currently states the
  `Message` reducer effect has "no meaningful headless unit surface" alongside the
  context-menu wiring — that's inaccurate for the reducer arm specifically (it compiles under
  `--no-default-features` via `src/app.rs`, unlike the gui-gated view code in `src/ui/mod.rs`);
  narrow the claim to the view-construction/binary-handler code only, once T013 closes the gap
  per Constitution I (contradicts)
- [X] T015 Add a **Copy** row to `docs/user-guide/icons.md`'s "Where icons appear" table
  (glyph: content_copy, meaning: copy a worktree's name to the clipboard, where: worktree
  right-click menu) per Constitution VII (partial)
