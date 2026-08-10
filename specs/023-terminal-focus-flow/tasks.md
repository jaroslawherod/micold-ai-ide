---

description: "Task list for 023 — Natural Terminal Focus Flow"
---

# Tasks: Natural Terminal Focus Flow

**Input**: Design documents from `/specs/023-terminal-focus-flow/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/focus-model.md](./contracts/focus-model.md),
[quickstart.md](./quickstart.md)

**Tests**: Mandatory (Constitution Principle I). Every story writes its failing tests first. The
`src/ui/` edits fall under the GUI-wiring exception and are validated by `quickstart.md` §B, run
headlessly with the repo's `visual-pass` skill — *and* the exception's precondition gets its own
source gate (`tests/terminal_bar_stability.rs`). Anything in `src/ui/` that would be a rule of its
own is extracted into a pure `pub(crate)` function with inline tests instead of relying on the
exception (T010, T014).

**Documentation**: Each story updates `docs/user-guide/worktrees-and-sessions.md` in the same change
(Principle VII).

**Cross-platform**: No `cfg(target_os)` is added. CI runs the suite on Linux, macOS and Windows
(Principle VI).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1–US4 from spec.md
- Paths are repo-relative; `crates/micold-client/` is the only crate touched

---

## Phase 1: Setup

**Purpose**: Capture the evidence that the bug exists, and retire the contract this one supersedes.

- [ ] T001 Record the pre-fix baseline with the `visual-pass` skill and write it to `specs/023-terminal-focus-flow/visual-pass-baseline.md`: with the terminal focused, press the mode toggle **once** and screenshot the result — it must show the press doing nothing. This is the "down from two" half of SC-002 and cannot be reconstructed after T012/T013 land.
- [ ] T002 [P] Add a superseded-by pointer at the top of `specs/006-real-terminal-emulator/contracts/focus-model.md` naming `specs/023-terminal-focus-flow/contracts/focus-model.md` (v2) and stating that 006's routing rule and write gate survive verbatim.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Replace the stored flag with the derived predicate. Every user story reads it.

**⚠️ CRITICAL**: No story work can begin until T005–T008 are complete — after T005 the codebase does
not compile until T007 and T008 land.

- [ ] T003 [P] Write failing tests for the predicate in `crates/micold-client/tests/terminal_focus.rs`, covering: its first four terms (`active_session.is_some()`, `!terminal_released`, `focused_field.is_none()`, `overlay == Overlay::None`), one test per term flipping the answer, plus the all-clear case; that only the **displayed** session's terminal is ever eligible — background sessions present with `active_session: None` reads false (FR-020); and that `focus_terminal()` clears both `terminal_released` and `focused_field`, so a `TerminalFocused` press wins over a field that held the keyboard (FR-008b, FR-018). The fifth term, `any_menu_open()`, belongs to US4/T027.
- [ ] T004 [P] Write the failing source gate `no_scattered_release_writes` in the new file `crates/micold-client/tests/terminal_bar_stability.rs`: walk `crates/micold-client/src/**/*.rs` and fail if `terminal_released` is assigned outside the bodies of `focus_terminal()` and `release_terminal()`. The whole tree, not just `app.rs` — the helpers are `pub(crate)` and `features/session.rs` calls them. Follow the shape of `crates/micold-client/tests/showcase_glue.rs`.
- [ ] T005 In `crates/micold-client/src/app.rs`: replace `pub terminal_focused: bool` with `pub terminal_released: bool` (default `false`); add `pub fn terminal_focused(&self) -> bool` as in [data-model.md](./data-model.md); add `fn any_menu_open(&self) -> bool { false }` as a **stub**, marked `// filled in by US4/T028`, so US4's tests are observed failing; and add the two writers `pub(crate) fn focus_terminal(&mut self)` (clears `terminal_released` **and** `focused_field`) / `pub(crate) fn release_terminal(&mut self)`. `pub(crate)` because T024 calls the first from `features/session.rs`. Point the `TerminalFocused` / `TerminalFocusReleased` arms at those writers.
- [ ] T006 In `crates/micold-client/src/app.rs`, delete the now-redundant `self.terminal_focused = false;` lines from the session-close arm and the `SessionRemoveConfirmed` arm — `active_session = None` already makes the predicate false (FR-012, FR-016).
- [ ] T007 [P] Point the read sites at the predicate: `crates/micold-client/src/ui/terminal.rs` (`.focused(state.terminal_focused())`) and `crates/micold-client/src/ui/mod.rs` (`subscription()`'s early return). `route_key`'s signature is unchanged — only its argument's provenance.
- [ ] T008 [P] Migrate every `State { terminal_focused: … }` construction in `crates/micold-client/tests/` to set `terminal_released` or drive the message instead. A test that still names the field will not compile; that is the migration working (research R3).

**Checkpoint**: `mise run test` is green, focus is derived, and no story task is blocked.

---

## Phase 3: User Story 1 — One press does what you pressed (Priority: P1) 🎯 MVP

**Goal**: A single press on any control activates it, whatever the terminal was holding — and a press
into an unfocused pane both takes the keyboard and reaches the program.

**Independent Test**: With the terminal focused, one press on the mode toggle switches the mode and
typing immediately afterwards reaches the terminal (quickstart §B1). Screenshots show no focus-ring
blink (§B2). A press into an unfocused pane running `vim` reaches `vim` (§B6).

### Tests for User Story 1 (MANDATORY) ⚠️

> Write these first; all three must fail before T012–T015.

- [ ] T009 [P] [US1] Add the gate `bar_does_not_branch_on_focus` to `crates/micold-client/tests/terminal_bar_stability.rs`: read `crates/micold-client/src/ui/terminal.rs` and fail if the bottom bar's construction adds or removes a child as a function of terminal focus (research R1's structural precondition, FR-008a).
- [ ] T010 [P] [US1] Extend the inline `mod tests` in `crates/micold-client/src/ui/material/terminal_pane.rs` with the granting press: the truth table of the new `press_grants_focus(focused, is_left_press, over_bounds)` (all eight rows), and `press_routing(focused_now = true, …)` across mouse-mode on/off and shift on/off (FR-008b, research R5). Inline because both functions are `pub(crate)`.
- [ ] T011 [P] [US1] Add `no_input_from_presses_outside_bounds` to the same inline `mod tests`: `TerminalPane::update` with the cursor outside its bounds produces no `TerminalAction(Write(..))`, for any button, mouse mode, or modifier combination (FR-003, SC-008). This is SC-008's only mechanical evidence.

### Implementation for User Story 1

- [ ] T012 [US1] Delete the click-outside release block from `TerminalPane::update` in `crates/micold-client/src/ui/material/terminal_pane.rs` — the `if self.focused { … shell.publish(Message::TerminalFocusReleased) }` guarded on `!cursor.is_over(bounds)`. A press on a control that types nothing must not touch focus (FR-005, FR-006).
- [ ] T013 [US1] In `crates/micold-client/src/ui/terminal.rs`, push the release-focus `IconButton` **unconditionally** into the bottom bar and gate only its `on_press` on `state.terminal_focused()`. The bar's child list must not depend on focus (research R1); this is what T009 checks.
- [ ] T014 [US1] In `crates/micold-client/src/ui/material/terminal_pane.rs`, add `pub(crate) fn press_grants_focus(focused: bool, is_left_press: bool, over_bounds: bool) -> bool` (`!focused && is_left_press && over_bounds`) beside `press_routing`, and use it in `Widget::update`: publish `Message::TerminalFocused` when it is true, and pass `self.focused || grants` to `press_routing` at **both** call sites (FR-008b). The decision is a tested pure function, not a branch in `update` — Principle I's GUI-wiring exception does not cover code with a rule of its own.
- [ ] T015 [US1] Delete both `Task::done(Message::TerminalFocused)` re-assertions from `crates/micold-client/src/main.rs` (the BUG-001 workarounds). The race they won no longer exists, and re-asserting is the intermediate-holder shape FR-008a forbids.
- [ ] T016 [P] [US1] Update `docs/user-guide/worktrees-and-sessions.md`: one press activates any control regardless of what the terminal holds; a press into an unfocused terminal both focuses it and reaches the program, even if a field held the keyboard.
- [ ] T017 [US1] Run quickstart §B1, §B2 and §B6 with the `visual-pass` skill — including §B1's rapid-alternation sequence and §B6's press-into-pane-from-a-focused-field case — and record the pass (command, what was driven, screenshots, observation) in `specs/023-terminal-focus-flow/visual-pass.md`. §B2 and §B6 must show pixels.

**Checkpoint**: The reported bug is fixed and demonstrable. This is a shippable MVP on its own.

---

## Phase 4: User Story 2 — Coming back to the app resumes typing (Priority: P1)

**Goal**: Leaving the window and returning changes nothing about who holds the keyboard.

**Independent Test**: Focus the terminal, switch windows, switch back, type — the characters reach
the process with no click. Release the terminal first and the release survives the round trip
(quickstart §B3).

### Tests for User Story 2 (MANDATORY) ⚠️

- [ ] T018 [P] [US2] Add `window_focus_changes_no_focus_term` to `crates/micold-client/tests/terminal_focus.rs`: applying `Message::WindowFocusChanged(false)` then `(true)` leaves `terminal_released`, `focused_field`, `overlay`, the menu flags and `active_session` all unchanged, so `terminal_focused()` reads the same before and after — both for a focused terminal and for a released one (FR-013–FR-015).

### Implementation for User Story 2

- [ ] T019 [US2] Confirm the `Message::WindowFocusChanged` arm in `crates/micold-client/src/main.rs` still only re-detects the OS theme, and add a one-line comment there naming FR-013–FR-015: this story is satisfied by writing **nothing**, and a future "helpful" restore would break it. No other production change.
- [ ] T020 [P] [US2] Update `docs/user-guide/worktrees-and-sessions.md`: switching away and back leaves the keyboard where you left it, including an explicit release.
- [ ] T021 [US2] Run quickstart §B3 with the `visual-pass` skill — including §B3.4, which releases and re-acquires focus while `yes | nl` prints and confirms the line numbers are unbroken (FR-025) — and append the pass to `specs/023-terminal-focus-flow/visual-pass.md`.

**Checkpoint**: US1 and US2 both hold independently.

---

## Phase 5: User Story 3 — Landing on a session leaves you ready to type (Priority: P2)

**Goal**: Every navigation that puts a terminal in front of the user clears an explicit release, so
launch, session switch, mode toggle and instance switch all land ready to type.

**Independent Test**: From a released terminal, do each of: select a session, start one, toggle the
mode, open/close/switch a Regular Terminal instance, switch project, relaunch the app — and type
straight away with zero presses (quickstart §B4).

### Tests for User Story 3 (MANDATORY) ⚠️

- [ ] T022 [P] [US3] Add failing navigation tests to `crates/micold-client/tests/terminal_focus.rs`: starting from `terminal_released: true`, each of `SessionStarted`, `SessionSelected`, `TerminalModeToggled`, `ShellInstanceOpenRequested`, `ShellInstanceSelected`, `ShellInstanceCloseRequested` clears the release (FR-011, FR-021a); and `State::default()` with a restored `active_session` is focused (FR-012a).

### Implementation for User Story 3

- [ ] T023 [US3] In `crates/micold-client/src/app.rs`, call `self.focus_terminal()` from the six navigation arms named in T022. No arm may assign `terminal_released` directly — T004's gate enforces it.
- [ ] T024 [US3] In `crates/micold-client/src/features/session.rs`, change `restore_after_activation` to call `self.focus_terminal()` where it set `self.terminal_focused = false;` — a project switch onto a restored session lands focused (FR-011). This is why T005 makes the helper `pub(crate)`.
- [ ] T025 [P] [US3] Update `docs/user-guide/worktrees-and-sessions.md`: what counts as navigation, and that navigating to a terminal clears a release you made earlier.
- [ ] T026 [US3] Run quickstart §B4 with the `visual-pass` skill and append the pass to `specs/023-terminal-focus-flow/visual-pass.md`.

**Checkpoint**: US1–US3 hold independently.

---

## Phase 6: User Story 4 — Focus is never taken while you are typing somewhere else (Priority: P2)

**Goal**: A field, dialog or menu holds the keyboard for as long as it is open, and gives it back
when it finishes — with no restore stack and no interference from terminal output.

**Independent Test**: Type into the Add Worktree form while a background session floods output —
every character lands in the field, none reaches a terminal; dismissing the form returns the keyboard
unless it was explicitly released (quickstart §B5).

### Tests for User Story 4 (MANDATORY) ⚠️

- [ ] T027 [P] [US4] Add failing bounds tests to `crates/micold-client/tests/terminal_focus.rs`: each of the six menu flags makes the predicate false and restores it on close; an open overlay makes it false (FR-017); `FieldFocusChanged(id, true)` then `(id, false)` round-trips (FR-018, FR-010); terminal output and lifecycle changes flip nothing (FR-019); and `terminal_context_menu` being open is **deliberately not** a term — the terminal keeps the keyboard (FR-007, research R4). The menu cases fail against T005's stub, which is the point.

### Implementation for User Story 4

- [ ] T028 [US4] In `crates/micold-client/src/app.rs`, replace T005's `any_menu_open()` stub with the real predicate over exactly `help_menu_open`, `project_switcher_open`, `sidebar_filter_open`, `project_menu_open`, `worktree_menu_open`, `session_menu_open` — and not `terminal_context_menu`. Add the comment naming FR-007/research R4 so the omission reads as a decision.
- [ ] T029 [P] [US4] Update `docs/user-guide/worktrees-and-sessions.md`: what takes the keyboard from the terminal (fields, dialogs, menus), what gives it back, and that the terminal's own right-click menu does not.
- [ ] T030 [US4] Run quickstart §B5 with the `visual-pass` skill and append the pass to `specs/023-terminal-focus-flow/visual-pass.md`.

**Checkpoint**: All four stories hold independently.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T031 [P] Re-point the header comment of `crates/micold-client/tests/terminal_focus.rs` at `specs/023-terminal-focus-flow/contracts/focus-model.md` (v2) instead of 006's.
- [ ] T032 [P] Cross-cutting documentation review in `docs/`: the focus rules must read consistently wherever they appear, and nothing may still describe the click-outside release.
- [ ] T033 Verify `mise run test`, `cargo fmt --check` and clippy are green locally, and that every job in `.github/workflows/ci.yml` passes on Linux, macOS and Windows (Principle VI). No `cfg(target_os)` was added, so a platform-only failure is a real finding.
- [ ] T034 Run the whole of `quickstart.md` — §A end to end and §B1–§B6 in one sitting — and finalize `specs/023-terminal-focus-flow/visual-pass.md` against `visual-pass-baseline.md` from T001, so SC-002's "one press, down from two" is evidenced by both frames.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies. T001 must run **before** T012/T013 or its evidence is gone.
- **Foundational (Phase 2)**: blocks every story. T005 breaks the build until T007 and T008 land, so
  T005 → T006 → {T007, T008} is one unit of work, not four commits to leave half-done.
- **US1 (Phase 3)**: after Phase 2. Independent of US2–US4.
- **US2 (Phase 4)**: after Phase 2. Independent — it asserts an absence, so it needs no US1 code.
- **US3 (Phase 5)**: after Phase 2. Independent of US1/US2/US4.
- **US4 (Phase 6)**: after Phase 2. Independent of US1–US3, and the only story that touches
  `any_menu_open()` — T005 leaves it a stub precisely so T027 goes Red.
- **Polish (Phase 7)**: after every story you intend to ship.

### Task-Level Graph

```text
T001 ──┐
T002 ──┴─→ T003 ─┐
           T004 ─┴─→ T005 → T006 → T007 ─┐
                                  T008 ─┴─→ ┬─ US1: T009,T010,T011 → T012 → T014 ─┐
                                            │                   T013 ────────────┤
                                            │                   T015 ────────────┼→ T016 → T017
                                            ├─ US2: T018 → T019 → T020 → T021
                                            ├─ US3: T022 → T023 → T024 → T025 → T026
                                            └─ US4: T027 → T028 → T029 → T030
                                                                            ↓
                                                     T031,T032 → T033 → T034
```

T010, T011, T012 and T014 all live in `terminal_pane.rs`; do them in that order in one pass. T013
(`ui/terminal.rs`) and T015 (`main.rs`) are independent of them and of each other.

### Riskiest task first

**T013** is the one that has to be right. It is the structural fix for research R1 — a
focus-conditional child in a row of pressable siblings, which fails *silently*: a swallowed press
looks like a slow app, not like a bug. Its gate (T009) is a source-level regex, not a behavioural
test, so read the diff as well as the green.

**T014** is second. It is the one place this feature adds logic to `src/ui/`, and the reason it is a
pure `pub(crate)` function rather than an expression inside `Widget::update` is that the constitution
does not exempt rules — only wiring. Keep it that way.

### Parallel Opportunities

- Phase 1: T002 alongside T001.
- Phase 2: T003 ∥ T004 (different files, both failing-first). Then T007 ∥ T008.
- Phase 3: T009 ∥ T010 ∥ T011 (T010 and T011 share a file — same pass, one commit); later T016
  alongside T014/T015.
- After Phase 2 completes, **US1, US2, US3 and US4 can be worked in parallel by four people** — they
  share only `app.rs` (T023 and T028) and `tests/terminal_focus.rs` (T018, T022, T027), each in a
  distinct region.
- Phase 7: T031 ∥ T032.

---

## Parallel Example: Foundational Phase

```bash
# The two failing-first gates, together:
Task: "Predicate truth-table tests in crates/micold-client/tests/terminal_focus.rs"
Task: "no_scattered_release_writes gate in crates/micold-client/tests/terminal_bar_stability.rs"

# After T005/T006, the two migrations, together:
Task: "Point read sites at the predicate in src/ui/terminal.rs and src/ui/mod.rs"
Task: "Migrate State { terminal_focused: … } constructions in crates/micold-client/tests/"
```

---

## Implementation Strategy

### MVP First (Setup + Foundational + US1)

1. Phase 1 — **T001 first**, before any code changes erase the baseline.
2. Phase 2 — the predicate. This alone delivers FR-009, FR-010, FR-012, FR-012a, FR-013–FR-020 as
   consequences (research R3); the stories below mostly *prove* them.
3. Phase 3 — the two defects the predicate does not fix: the swallowed press and the granting press.
4. **STOP and VALIDATE**: quickstart §B1/§B2/§B6. The reported bug is gone. Ship here if you want to.

### Incremental Delivery

Phase 2 + US1 → demo. Then US2 (pure verification, cheapest), US3, US4 in any order — each is a
handful of tests plus at most one small production edit, and each adds a §B section to the recorded
visual pass.

---

## Notes

- **T008 is not optional cleanup.** After T005 the crate does not compile until every test stops
  naming `terminal_focused` as a field. That is deliberate: the state you can set is the decision the
  user makes, not the answer the application derives.
- **`focus_terminal()` clears `focused_field` as well as the release.** Without that second line a
  press into the pane made while the sidebar filter held the keyboard leaves the predicate false, and
  FR-008b depends on iced's blur happening to arrive first. T003 pins it.
- **US2 has one production task and it changes no behaviour.** Resist the urge to make it do
  something. FR-013–FR-015 are satisfied by *not* writing anything on window blur; T018 exists to
  make a future restore fail loudly.
- **The spec's "Suspended holder" entity has no runtime existence** (data-model.md). FR-013's wording
  describes a restore; the design meets its outcome by writing nothing at all. T019's comment is what
  stops a future reader from adding the mechanism the entity implies.
- Commit after each task or logical group, except T005–T008, which must land together.
- Verify each test fails before implementing it (Principle I, non-negotiable).
- The `visual-pass` skill runs §B headlessly (Xvfb + `xdotool` + `import`). Do not ask a human.
