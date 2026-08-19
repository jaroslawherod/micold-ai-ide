---

description: "Task list for feature 026 — The AI Session as a Tab"
---

# Tasks: The AI Session as a Tab

**Input**: Design documents from `/specs/026-ai-session-tab/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/ai-session-tab-ui.md](./contracts/ai-session-tab-ui.md)

**Tests**: Per Constitution Principle I (NON-NEGOTIABLE), every implementation task is preceded by a
failing test. This feature makes that unusually cheap: `data-model.md` lists five derived values,
each a pure function of one session record, so almost every decision here is reachable from
`cargo test` without a renderer.

**Documentation**: Per Principle VII, `docs/user-guide/worktrees-and-sessions.md` is updated inside
the story that changes what a user sees, not deferred to Polish.

**Cross-platform**: Per Principle VI, nothing here is platform-specific; wheel scrolling comes from
the rendering stack's scrollable, which the sidebar already uses on all three.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1, US2, US3 — from `spec.md`
- Exact file paths in every description

## Path conventions

A three-crate Cargo workspace. Client code is `crates/micold-client/src/`, its gates are
`crates/micold-client/tests/`, render-free model code is `crates/micold-core/src/`.

---

## Phase 1: Setup — the shared primitives this feature extends

**Purpose**: Two components in the shared library need widening before anything can use them.
Principle VIII: extend, never fork. Both are independent of each other and of every user story.

- [ ] T001 [P] Failing test in `crates/micold-client/src/ui/material/scrollable.rs`'s `mod tests`:
  a `Scrollable` built with a horizontal direction reports that direction, and one built without it
  is still vertical. The default matters as much as the new value — two call sites (the sidebar list
  and the folder browser) depend on it and must not move
- [ ] T002 [P] Failing test in `crates/micold-client/src/ui/material/activity_badge.rs`'s
  `mod tests`: a badge built from a `BadgeEmphasis` directly draws the same element as one built
  from the `ActivitySignal` that maps to it, and a badge built from `None` reserves the slot and
  draws nothing. The reserved-empty case is the one T024 depends on
- [ ] T003 Add the direction to `crates/micold-client/src/ui/material/scrollable.rs` as a chainable
  builder step defaulting to `Direction::Vertical` (depends on T001; research R5). The wrapper is
  where the design system's 4px themed scrollbar lives, and where dismiss-on-scroll is reported from
  — a hand-rolled horizontal scroller would reintroduce exactly the divergence this component was
  created to end, and would silently drop the scroll-dismissal the tab menu needs
- [ ] T004 Add `ActivityBadge::for_emphasis(Option<BadgeEmphasis>, roles)` to
  `crates/micold-client/src/ui/material/activity_badge.rs`, and make `new(signal, roles)` sugar over
  it (depends on T002; research R3). Do **not** reach the new use through a contrived
  `ActivitySignal`: that vocabulary means daemon activity, not process lifecycle, and
  `tests/showcase_completeness.rs` poses variants by name, so the lie would be on the gallery page
- [ ] T005 [P] Pose the new variants in `crates/micold-client/src/showcase/catalogue.rs` and the
  section that renders them under `crates/micold-client/src/showcase/sections/` — the horizontal
  scrollable beside the vertical one, and whichever `BadgeEmphasis` the stopped mark uses. Gate C3
  in `tests/showcase_completeness.rs` requires every library variant to have an instance, and C4
  requires every posed variant to still exist

**Checkpoint**: `cargo test -p micold-client` green; the two primitives can carry the feature.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: The bar must be able to hold a growing strip, and the three derived values every story
reads must exist. No user story can start until this phase is done.

**⚠️ The first three tasks fix a defect that is live on `main` today** (research R7), independent of
this feature: past about five instances the bar's trailing controls — the "+" and the mode toggle —
are laid out narrower, or at zero, silently. That is feature 012's BUG-005 one level out. This
feature meets it sooner by making the strip always visible and adding a tab, so it is fixed first,
where it can be verified on its own.

- [ ] T006 Register a covered state in `crates/micold-client/tests/support/covered_states.rs` with
  **enough Regular Terminal instances to overflow the bar** (six or more at the fixture's 1280dp
  window). Feature 019 FR-016 makes this file the single registration site. Without this state the
  next task's gate inspects nothing, which is the "a pass that records nothing looks like a pass
  that found nothing" shape 019 keeps meeting
- [ ] T007 Failing gate in `crates/micold-client/tests/gates/` (new `bar_controls_hold_their_size.rs`,
  compiled into the `layout_snapshot` binary beside `tab_children_fit` so it shares the record
  cache): in every covered state that draws a bottom bar, **no control in that bar is laid out
  narrower than the width it asks for** — the "+", the mode toggle, the status and the title
  (FR-002c). Must **fail on today's `main`** in the T006 state, naming the squeezed controls and
  their widths. This is the same question `tab_children_fit` asks one level in, and the reason it is
  a second gate rather than a widened first one is that the bar's children are not tabs and are not
  recognised by that gate's structural rule
- [ ] T008 Bound the strip in `crates/micold-client/src/ui/terminal.rs::pane` so its growth cannot
  take width from its siblings (depends on T007; FR-002c). The strip becomes the bar's flexible
  member and every other control keeps its measured size
- [ ] T009 [P] Failing test beside `restart_message` in `crates/micold-client/src/ui/terminal.rs`'s
  `mod tests`: a `StripTab` names either an instance or the AI process, and `marked_tab(session)`
  returns exactly one of them for every combination of `mode` and `active_shell` — including a
  `Regular` session whose `active_shell` is `None`. FR-005's "never zero, never two" is a claim about
  totality; this is where it is proved
- [ ] T010 Add `StripTab` and `marked_tab` to `crates/micold-client/src/ui/terminal.rs` (depends on
  T009; `data-model.md`). A closed two-variant enum, **not** an `Option<ShellInstanceId>`: `None`
  already means "this session has no active instance" in this file, and overloading it to also mean
  "the AI tab" gives one value two meanings and makes the marked tab unanswerable in the one case
  that matters (Principle V)
- [ ] T011 [P] Failing test in `crates/micold-client/src/ui/terminal.rs`'s `mod tests`: one
  predicate answers "this process is stopped" for **both** vocabularies — `Idle | Failed |
  InterruptedResumable` for the AI process, `NotStarted | Exited` for an instance — and answers
  `false` for `Starting` and `Restarting { .. }` in both (research R1, FR-012d, FR-012e). Assert it
  for every variant of both enums by name, so a variant added later fails here rather than silently
  defaulting
- [ ] T012 Generalise `attached_process_restartable` in `crates/micold-client/src/ui/terminal.rs`
  into that predicate, taking a `StripTab` instead of implying the attached one, and re-express
  `attached_process_restartable` as a call into it (depends on T010, T011; research R2). This is the
  phase's load-bearing task: FR-012d asks the mark and the menu to agree, and deriving both from one
  function is what makes that structural. This file has already paid for the alternative twice —
  `empty_terminal_message` and the bar disagreed "for exactly as long as they were two readings of
  one fact", and BUG-004 was `restart_message` re-deriving a fact the predicate beside it already had

**Checkpoint**: the bar survives an overflowing strip, and every story's decisions are pure functions
with tests. User stories can begin.

---

## Phase 3: User Story 1 — The strip says what the pane is showing, always (Priority: P1) 🎯 MVP

**Goal**: The AI CLI process is a right-anchored, unclosable, icon-labelled tab; the strip is always
visible; exactly one tab is marked in every state; and the strip stays honest once it overflows.

**Independent Test**: Open a session with no Regular Terminal instances — the strip shows the AI tab
alone, marked. Open an instance and switch to it — that tab is marked and the AI tab is not. Open
instances until they overflow — the AI tab, the "+" and the toggle are all still at full size.

**Sub-increment**: T013–T022 deliver the strip itself and are a coherent, shippable MVP on their own.
T023–T028 add overflow. Phase 2 already made the bar safe at any instance count, so the split is a
delivery choice rather than a correctness one.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T013 [P] [US1] Failing test in `crates/micold-client/tests/terminal_tabs.rs`: the strip is
  built for a session with **zero** and with **one** Regular Terminal instance (FR-003), it contains
  one member per instance plus the AI tab (FR-001), and the AI tab is last (FR-002). Read out of the
  built element the way that file's existing call-site tests are
- [ ] T014 [P] [US1] Failing test in `crates/micold-client/src/ui/terminal.rs`'s `mod tests`: the AI
  tab measures `TAB_WIDTH` — the same figure a terminal tab measures — and its leading and trailing
  slots are equal, which is what puts the icon on the tab's midline once the trailing slot carries no
  close control (FR-010a)
- [ ] T015 [P] [US1] Failing test in `crates/micold-client/tests/terminal_bar_stability.rs`: the
  bar's child list does not vary with the number of instances, now that the strip is unconditional
  (feature 023 FR-008a). The `if let Some(switcher)` this feature deletes was such a variation; the
  test is what stops a later one being added

### Implementation for User Story 1

- [ ] T016 [US1] Render the AI tab in `crates/micold-client/src/ui/terminal.rs::instance_switcher_row`
  (depends on T013, T014): `Icon::AiCli` as the label — the glyph already exists, so FR-009 needs no
  font work — the same fixed width, the same indicator treatment, and a **reserved, empty trailing
  slot** where a terminal tab draws its close control (FR-004, FR-009, FR-010, FR-010a)
- [ ] T017 [US1] Delete the `session.shells.len() <= 1` early return in
  `crates/micold-client/src/ui/terminal.rs::instance_switcher_row` so the strip is drawn whenever a session is displayed (FR-003, superseding feature 012 FR-005), and
  rename the function to say what it now builds
- [ ] T018 [US1] Mark the tab `marked_tab` names, rather than comparing against `active_shell`, in
  `crates/micold-client/src/ui/terminal.rs::instance_switcher_row` (depends on T010, T016; FR-005). The AI tab takes the indicator in `AiCli` mode and a terminal tab
  takes it otherwise, from one source, so FR-008's "the toggle and the tab cannot disagree" holds
  because there is nothing to keep in step
- [ ] T019 [US1] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` with
  `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot` (depends on
  T016–T018). **Two covered states must move** (research R9): `session-terminal-instance-tabs` gains
  the AI tab, and `session-terminal-bottom-bar` — which drew no strip at all until FR-003 — gains an
  entire strip. The second is easy to miss and is where the single-instance user's whole visible
  change lands
- [ ] T020 [US1] Add the AI tab's anchors to `crates/micold-client/tests/support/covered_states.rs`
  so `tests/gates/tab_children_fit.rs` runs against it by name (depends on T019). Both of that
  gate's assertions are meaningful here: the touch-target one catches the AI tab squeezed by the
  scrolling viewport, and `a_tabs_content_sits_on_its_tabs_midline` is what actually holds FR-010a's
  centred icon — the property that failed at 4.6dp the morning before this feature was planned
- [ ] T021 [P] [US1] Document the AI tab in `docs/user-guide/worktrees-and-sessions.md` (Principle
  VII): the strip is always there, the rightmost tab is the AI conversation, and it has no close
  control because a session has exactly one
- [ ] T022 [US1] Run `quickstart.md` §1–§3 and §7 with the `visual-pass` skill and record it in a new
  `specs/026-ai-session-tab/visual-pass.md` (depends on T016–T020). §1's zero-instance state is the
  one to judge rather than merely observe: it is the state feature 012 deliberately rendered nothing
  in, so a single tab there is the most likely thing to read as a stray control instead of a
  deliberate strip

### Overflow (still User Story 1 — spec scenarios 5–7)

- [ ] T023 [P] [US1] Failing test in `crates/micold-client/src/ui/terminal.rs`'s `mod tests`:
  "content lies beyond this edge" is a pure function of the viewport offset and the content width,
  answered for the leading edge, the trailing edge, both and neither (FR-002e). Research R6 — the
  *fade* is appearance and cannot be gated, but the fact behind it can, and this is it
- [ ] T024 [P] [US1] Failing test in `crates/micold-client/src/ui/terminal.rs`'s `mod tests`:
  changing the marked tab yields a scroll-into-view request for it, and only for it (FR-002d)
- [ ] T025 [US1] Put the terminal tabs in a horizontal `material::Scrollable` inside
  `crates/micold-client/src/ui/terminal.rs::instance_switcher_row`, with the AI tab **outside** it (depends on T003, T016; FR-002a, FR-002b).
  Tabs keep their fixed width — no shrinking, no ellipsis, no dropping — and the AI tab keeps the
  right-hand end at any instance count
- [ ] T026 [US1] Scroll the marked tab into view when it changes, in
  `crates/micold-client/src/ui/terminal.rs` (depends on T024; FR-002d)
- [ ] T027 [US1] Draw the edge fade in `crates/micold-client/src/ui/terminal.rs` on any edge with
  content beyond it, distinctly when the content beyond it is the marked tab (depends on T023;
  FR-002e). Wheel scrolling comes from the scrollable;
  **no scroll-arrow controls** (FR-002f) — they would spend an interactive target's width at each end
  of the bar T008 just finished protecting
- [ ] T028 [US1] Run `quickstart.md` §6 with the `visual-pass` skill and append it to
  `specs/026-ai-session-tab/visual-pass.md` (depends on T025–T027). The fade is drawn, not laid out, so this is the only check
  that can see it at all; §6's first expectation is also the regression check for T008

**Checkpoint**: US1 complete. The strip is always present, always marked, and honest under overflow.

---

## Phase 4: User Story 2 — Reaching the AI CLI by pressing its tab (Priority: P1)

**Goal**: The AI tab is pressable — primary press selects, secondary press offers the terminal tab's
menu minus Close, and nothing about either disturbs a process.

**Independent Test**: From a displayed terminal instance, press the AI tab — the pane shows the AI
conversation and the terminal instance is untouched. Press it again — nothing happens. Right-click it
while running — nothing. Right-click it while stopped — Restart, and no Close.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T029 [P] [US2] Failing test in `crates/micold-client/tests/app_state.rs`: a primary press on
  the AI tab sets `mode = AiCli` and changes no lifecycle, no `active_shell` and no other session
  (FR-006, FR-011); pressing it while already displayed changes nothing at all (FR-007)
- [ ] T030 [P] [US2] Failing test in `crates/micold-client/src/ui/terminal.rs`'s `mod tests`: the
  menu for a `StripTab::Ai` is the menu for an instance **minus Close**, in the same order, for every
  lifecycle — and is **empty** whenever the AI process is running, so no menu opens (FR-004, FR-006a,
  FR-006b). Derived from T012's predicate, so a restart item can never appear where the mark does not
- [ ] T031 [P] [US2] Failing test in `crates/micold-client/tests/app_state.rs`: the menu records
  **which tab** it was opened on; opening it for another replaces rather than stacks; the "close every
  menu" path clears it; and an action dispatched from it targets that tab, not the marked one
  (FR-006a, extending feature 012's BUG-005 test to the AI tab)

### Implementation for User Story 2

- [ ] T032 [US2] Widen `State::shell_instance_menu` in `crates/micold-client/src/app.rs` from
  `Option<(ShellInstanceId, u16, u16)>` to `Option<(StripTab, u16, u16)>`, and the message that opens
  it likewise (depends on T010, T031; research R8). **One** surface, not a second one: FR-006a
  defines the AI tab's menu as the terminal tab's menu with an item filtered, and two surfaces is the
  shape that lets them drift — which is the thing FR-006a is worded to prevent
- [ ] T033 [US2] In `crates/micold-client/src/ui/terminal.rs`, give the AI tab a primary `on_press`
  selecting the AI CLI and wrap it in the existing
  `crates/micold-client/src/ui/cdk/context_area.rs` primitive for the secondary press (depends on
  T016, T029). The
  wrapper already lets the child answer first, so the primary press keeps working through it — the
  property feature 012's T069 asserts and this reuses rather than re-establishes
- [ ] T034 [US2] Build the menu's items from `StripTab` in
  `crates/micold-client/src/ui/mod.rs::shell_instance_menu_items`, filtering Close for the AI tab and
  taking restart from T012's predicate (depends on T012, T030, T032; FR-004, FR-006a, FR-006b)
- [ ] T035 [US2] Update the surface registration in `crates/micold-client/src/features/session.rs`
  and the entry in `crates/micold-client/tests/overlay_registration.rs`'s `POPOVERS` for the widened
  state (depends on T032). The popover count assertion in that file is what catches a
  popover-shaped field that nobody registered — it caught this exact omission during feature 012
- [ ] T036 [P] [US2] Document pressing the AI tab and its menu in
  `docs/user-guide/worktrees-and-sessions.md` (Principle VII), beside the terminal tab's menu already
  described there
- [ ] T037 [US2] Run `quickstart.md` §5 with the `visual-pass` skill and append it to
  `specs/026-ai-session-tab/visual-pass.md` (depends on T033–T035). The check worth the setup is the **silence**: a secondary
  press on a running AI tab must produce nothing at all, not an empty panel

**Checkpoint**: US1 and US2 both work independently. The strip is complete and pressable.

---

## Phase 5: User Story 3 — The strip reports which processes are not running (Priority: P2)

**Goal**: A tab whose process is stopped says so, by the same mark on both kinds of tab, and that
mark is what makes US2's menu findable.

**Independent Test**: With two instances open, `exit` the one that is not displayed — its tab gains
the mark without being selected, and its sibling is untouched. Stop the AI CLI — the AI tab wears the
same mark in the same place. Restart either from its menu — the mark clears.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T038 [P] [US3] Failing test in `crates/micold-client/src/ui/terminal.rs`'s `mod tests`: a tab
  wears the mark for exactly the states T012's predicate calls stopped, for both an instance and the
  AI process, and never for `Starting` or `Restarting { .. }` (FR-012, FR-012d, FR-012e). Assert it
  **through the predicate**, so the mark cannot be given its own lifecycle match later
- [ ] T039 [P] [US3] Failing test in `crates/micold-client/src/ui/terminal.rs`'s `mod tests`: every
  tab builds the **same children** in the same order whether or not its process is stopped — the mark's slot is reserved and drawn empty,
  never pushed or omitted (research R4, feature 023 FR-008a). Without this the mark is a conditional
  child inside a pressable tab, and iced's positional `Tree::diff_children` hands the pressed control
  its neighbour's node and drops the press
- [ ] T040 [P] [US3] Failing test in `crates/micold-client/tests/terminal_tabs.rs`: a tab that is
  both marked-active and stopped carries both cues, and the mark is not drawn in the indicator's role
  (FR-012a). Colour identity, not geometry — the composited result is §8's business, but "these two
  are not the same role" is assertable here

### Implementation for User Story 3

- [ ] T041 [US3] Add the reserved leading slot to every tab in
  `crates/micold-client/src/ui/terminal.rs::instance_switcher_row`, drawing
  `ActivityBadge::for_emphasis` when stopped and an empty space of the same size otherwise (depends
  on T004, T012, T038, T039; FR-012c). The slot goes in the **leading spacer the tab already
  reserves** — it exists only to balance the trailing close control and is empty today, so no tab
  grows and `TAB_WIDTH` does not move
- [ ] T042 [US3] Apply the same slot to the AI tab in
  `crates/micold-client/src/ui/terminal.rs::instance_switcher_row` (depends on T016, T041;
  FR-012). The mark must sit
  in the same place on both, or FR-010's "consistent with the tabs it sits beside" is false in the
  one state that matters
- [ ] T043 [US3] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` (depends on
  T041, T042). **Every tab in every covered state gains a child.** The diff is the artefact: no tab
  changes width, no tab's label leaves its midline, and no control drops under its touch target —
  `tab_children_fit` is the gate that says so, and it now runs on the AI tab too (T020)
- [ ] T044 [P] [US3] Document the mark in `docs/user-guide/worktrees-and-sessions.md` (Principle
  VII): what it means, that it appears on a background tab you have not selected, and that it is how
  you know which tab's menu has a restart in it
- [ ] T045 [US3] Run `quickstart.md` §4 with the `visual-pass` skill and append it to
  `specs/026-ai-session-tab/visual-pass.md` (depends on T041–T043). This story's substance is appearance and this is where it
  is judged: the mark against the accent an active tab wears **and** the muted tint an inactive one
  wears, in **both** schemes, and not mistakable for the indicator. FR-012a is the requirement a
  tone-only cue would have failed, and it was the reason the mark is a mark

**Checkpoint**: all three stories independently functional.

---

## Phase 6: Polish & cross-cutting concerns

> Per-story user-guide docs shipped inside their stories (Principle VII). This phase is
> cross-cutting only.

- [ ] T046 [P] Re-run **feature 012's `quickstart.md` §8** and record it in
  `specs/012-multiple-regular-terminals/visual-pass.md`. This feature changes 012's terminal tabs —
  every one of them gains a slot (T041) and the strip they live in now scrolls — and §8 is that
  strip's appearance section. Recorded there rather than here because the control is 012's
- [ ] T047 [P] Note in `specs/012-multiple-regular-terminals/spec.md` that FR-005 is superseded by
  026 FR-003 and that its tabs gained a stopped mark, so a reader of the older spec is not misled by
  a requirement this feature reversed
- [ ] T048 Run the whole of `specs/026-ai-session-tab/quickstart.md`'s automated section and
  `mise run test`; confirm `cargo fmt --check` and `cargo clippy --workspace --all-targets --
  -D warnings` are clean
- [ ] T049 Verify the build and full suite on Linux, macOS and Windows (Principle VI) — the
  three-platform matrix in `.github/workflows/ci.yml`, run on the pull request, is the record
- [ ] T050 [P] Cross-cutting documentation review in `docs/`: the user guide now describes a strip
  that is always present, scrolls, carries a state mark and offers two menus. Read it as a whole
  rather than as four appended paragraphs

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (Phase 1)**: no dependencies; start immediately
- **Foundational (Phase 2)**: independent of Phase 1 except that T025 needs T003 — but it **blocks
  every user story**, and its first three tasks fix a live defect, so it goes first
- **US1 (Phase 3)**: after Phase 2
- **US2 (Phase 4)**: after Phase 2; T033 needs US1's T016 to have a tab to press
- **US3 (Phase 5)**: after Phase 2; T042 needs US1's T016
- **Polish (Phase 6)**: after the stories it reviews

### User story dependencies

- **US1 (P1)** — independent. The MVP.
- **US2 (P1)** — needs a rendered AI tab (T016) to attach presses to; otherwise independent.
- **US3 (P2)** — needs T016 for the AI tab's slot; otherwise independent. Genuinely severable: the
  strip is correct and useful without it, which is why it is not P1. It is what makes US2's menu
  *findable*, so shipping US2 without it leaves an action discoverable only by trial.

### Within each story

- Tests written and failing before implementation (Principle I)
- Pure functions before the render that reads them
- Fixture regeneration after the render changes, never before
- User-guide docs in the same story (Principle VII)
- The visual pass last in the story, against the finished build

### Parallel opportunities

- **Phase 1**: T001 ∥ T002, then T003 ∥ T004, then T005
- **Phase 2**: T009 ∥ T011 (different concerns, same file — coordinate the edit); T006–T008 are a
  strict chain
- **Phase 3**: T013 ∥ T014 ∥ T015; T023 ∥ T024; T021 ∥ any implementation task
- **Phase 4**: T029 ∥ T030 ∥ T031; T036 ∥ any
- **Phase 5**: T038 ∥ T039 ∥ T040; T044 ∥ any
- **Phase 6**: T046 ∥ T047 ∥ T050

Two whole stories can run in parallel after Phase 2 and T016 — US2 and US3 touch different concerns
(presses and menus versus the tab's leading slot) and meet only in `instance_switcher_row`.

---

## Implementation strategy

**MVP** is Phase 2 + Phase 3's T013–T022: the bar stops squeezing its controls, and the strip is
always visible with the AI CLI in it and exactly one tab marked. That is User Story 1's headline
claim — "the strip says what the pane is showing, always" — and it is shippable without scrolling,
without the tab being pressable, and without the mark. The mode toggle still reaches the AI pane, so
nothing is unreachable in the meantime.

**Then, in order of what each unlocks**: T023–T028 (overflow) completes US1 and is what makes the
feature safe at the instance counts 012 encourages; Phase 4 makes the tab pressable, which is what
turns a status display into a control; Phase 5 makes Phase 4's menu findable.

**Do not reorder Phase 5 before Phase 4.** The mark's stated purpose (FR-012b) is to point at a tab
whose menu has something in it. Shipping the mark first would put a cue on the strip pointing at an
action that does not exist yet.
