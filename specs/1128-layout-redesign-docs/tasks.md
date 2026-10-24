---

description: "Tasks for #1128: layout redesign — user guide, layout snapshot and visual pass"
---

# Tasks: Layout redesign — user guide, layout snapshot and visual pass

**Input**: Design documents from `specs/1128-layout-redesign-docs/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Constitution I. US1, US2 and US4 change no code: their checks are the pattern sweep
(quickstart A2), the link check (A1) and the record count (§B), each run as the story's last task.
US3's only code is test code over a layout that already ships, so its gate passes when written;
its red is shown by the mutation proof (T027, quickstart A3, research R6 *Test-first*).

**Documentation**: Constitution VII. The user guide is US1's and US2's deliverable; US3 updates
`docs/development/layout-snapshot.md` (T028) and changes no user-guide page.

**Cross-platform**: Constitution VI. Every chord a touched sentence names carries its macOS form
(FR-018, R11). The visual pass is Linux only and says so (spec Edge cases).

**Staging**: Phases 1–3 (M1) run now. Phases 4–6 (M2–M4) start only after **#1125 and #1127 have
merged on main** (spec Assumptions, research R1). If either is closed unmerged, its region's tasks
are dropped and the ledger says so (spec Edge cases).

**Scope rule for every docs task** (FR-011, FR-020): change only the sentence, table row or alt
text a pattern hit; nothing else on the page is rewritten; nothing under `crates/*/src` changes.
Line numbers in [contracts/doc-edits.md](contracts/doc-edits.md) are from plan time; each task
works from the fresh pattern run of its phase's first task.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US4 from spec.md

---

## Phase 1: Setup

**Purpose**: the stage-1 hit list, fresh from main.

- [x] T001 Run the five stage-1 patterns of [doc-edits §Patterns](contracts/doc-edits.md#patterns)
  (`grep -E -i -n`) over `docs/user-guide/*.md site/media.toml` on the branch rebased on main;
  match each hit to E1–E12 of [doc-edits §Stage 1](contracts/doc-edits.md#stage-1--inventory-at-plan-time-m1)
  and list any hit not in the inventory, with its kind (stale, ambiguous bar, unlinked, checked,
  historical, or stage 2 such as `appearance-theming.md:58` and `sandboxed-daemon.md:505`). The list
  lives in the session scratchpad, not the tree.

---

## Phase 2: Foundational

**Purpose**: the link targets every stage-1 edit points at exist and answer SC-007's stage-1
questions.

- [x] T002 Confirm the owning sections of [research R2](research.md#r2--owning-sections) exist with
  these anchors: `docs/user-guide/appearance-theming.md#status-bar`,
  `docs/user-guide/terminal-panes.md#the-header-above-the-terminal`,
  `docs/user-guide/terminal-panes.md#the-tab-strip-above-the-terminal`,
  `docs/user-guide/worktrees-and-sessions.md#claude-plan-usage`. Check SC-007's stage-1 answers:
  "see the connection state" is in the first paragraph of `appearance-theming.md#status-bar`, and
  "switch to a shell" in the first paragraph of `terminal-panes.md#the-tab-strip-above-the-terminal`;
  if either is not, add it to that first paragraph (one sentence, chords in both forms, FR-018).

---

## Phase 3: User Story 1 — The status bar and session area read the same on every page (P1) 🎯 MVP

**Goal**: the four pages place the connection, sandbox, CLI and plan usage in the status bar, a
session's state, restart and instances in the context header and tab strip, and never name the bar
under the terminal or an unnamed "bar".

**Independent test**: quickstart A2, stage-1 rows: every hit is an owning section, a linked
mention, or historical; no hit for the bar under the terminal; every "bar" in the four pages names
which bar. A1 passes.

- [x] T003 [US1] E1 in `docs/user-guide/worktrees-and-sessions.md` (~1195): "the bar in the error
  colour" → "the meter's bar in the error colour" (FR-003)
- [x] T004 [US1] E3–E5 in `docs/user-guide/worktrees-and-sessions.md` (~1583, ~1586, ~1603): the
  restart control and the `failed` status are placed in the tab strip, the first of them linking to
  `./terminal-panes.md#the-tab-strip-above-the-terminal`, per the *Change* column of E3–E5
  (FR-002, FR-003)
- [x] T005 [US1] E6 in `docs/user-guide/worktrees-and-sessions.md` `### The tab strip` (~1796–1801):
  the opening paragraph's list of the strip's members becomes one sentence linking to
  `./terminal-panes.md#the-tab-strip-above-the-terminal`; the instance rules (exactly one marked
  tab, the AI tab's rules, right-click) stay; the heading, and so `#the-tab-strip`, stays (FR-004)
- [x] T006 [US1] E7 in `docs/user-guide/worktrees-and-sessions.md` (~1494, ~1698, ~1748, ~1778,
  ~1788): the first tab-strip mention in each section links to the owning section; wording
  otherwise unchanged (FR-004)
- [x] T007 [US1] E2 and E8 in `docs/user-guide/worktrees-and-sessions.md` (~1228, ~2052): the
  status-bar mentions link to `./appearance-theming.md#status-bar` (FR-004)
- [x] T008 [P] [US1] E9 in `docs/user-guide/overview.md` (~263): "the status bar at the bottom"
  links to `./appearance-theming.md#status-bar` (FR-004)
- [x] T009 [P] [US1] E10 in `docs/user-guide/settings.md` (~292) and
  `docs/user-guide/sandboxed-daemon.md` (~504): the status-bar mention links to
  `./appearance-theming.md#status-bar` if it does not already; the sentence is otherwise unchanged
  (FR-011)
- [x] T010 [US1] E11, E12 and T001's extra hits: confirm E11 (`worktrees-and-sessions.md` ~1561,
  error banner) and E12 (stored-sessions banner, ~409–416) stay unchanged (US1 scenario 2, spec
  Terms); fix each extra stage-1 hit from T001 by FR-001–FR-004 (outside the four pages, FR-011:
  that sentence only), leaving stage-2 hits for T012
- [x] T011 [US1] Verify stage 1: re-run the stage-1 patterns (quickstart A2) and confirm every hit
  is an owning section, a linked mention, historical or stage 2; every chord in a touched sentence
  has its macOS form (FR-018); `site/checks/links.sh --sources` passes, or record that lychee is
  missing and CI runs it (A1); `git diff --stat origin/main -- 'crates/*/src'` is empty (FR-020)

**Checkpoint**: M1 deliverable observable on main.

---

## Phase 4: User Story 2 — The sidebar and title bar read the same on every page (P1)

**Blocked until #1125 and #1127 have merged on main.**

**Goal**: the four pages describe the shipped title bar (breadcrumb, jump-to field, Settings,
overflow menu) and sidebar (Overview row, chip bar, state groups, two-line rows), each element once,
linked elsewhere; `worktrees-and-sessions.md` opens with the four regions.

**Independent test**: quickstart A2 stage-2 rows, A1, and the five SC-007 answers each in the first
paragraph of its owning section.

- [ ] T012 [US2] Read #1125's and #1127's merged specs and user-guide changes: write down the final
  removed-element list (spec Terms) and the patterns for anything they removed beyond
  [doc-edits §Patterns](contracts/doc-edits.md#patterns) stage 2; record the title bar's and the
  sidebar's owning sections (research R2). If #1127 wrote no title-bar section, add `### Title bar`
  to `docs/user-guide/appearance-theming.md` beside `### Status bar` (breadcrumb, jump-to field,
  Settings, overflow menu; any platform difference such as macOS window controls). If #1127 put plan
  usage in the title bar, update FR-001 and FR-010 in `specs/1128-layout-redesign-docs/spec.md` to
  match (spec Edge cases) and say so in the ledger. Run all stage-2 patterns over
  `docs/user-guide/*.md site/media.toml` and match the hits to S1–S15 in the scratchpad list
- [ ] T013 [US2] S2, S3 in `docs/user-guide/overview.md` (~20, ~36, ~43): open and close the
  Overview by the sidebar's Overview row and Ctrl+Shift+O (Cmd+Shift+O on macOS); which regions
  stay while it is open and while it is full screen, as shipped (FR-006, US2 scenario 1)
- [ ] T014 [P] [US2] S4, S5, S14 in `docs/user-guide/project-selection.md`: `## Switching projects
  from the top bar` (~70) gets a heading naming the breadcrumb, and its text says where the panel
  opens, without "top bar" or "next to the menu button" (FR-007); ~155 names the breadcrumb's
  project part; `#opening-a-project` says the empty screen shows the title bar and the status bar
  only, and where the open-project actions are. Re-point every inbound link to
  `#switching-projects-from-the-top-bar` (`grep -rn` over `docs/ site/`) (FR-019)
- [ ] T015 [US2] S1, S6, S14, S15 in `docs/user-guide/worktrees-and-sessions.md`: **Open another
  project** (~13) where #1127 put it, linked; tag filtering (~320) describes the chip bar, with no
  filter button or tag-filter panel (US2 scenario 4); `#opening-a-project-git-repositories-only`
  describes the empty screen; where the groups are described: a project with no worktrees shows the
  Overview row, Default and no group, and a row moves between groups as its state changes (spec
  Edge cases, Principle II)
- [ ] T016 [US2] S12 in `docs/user-guide/worktrees-and-sessions.md`: a short opening section before
  `## Opening a project (git repositories only)` naming the regions top to bottom (title bar,
  sidebar beside the session area, status bar), each linked to its owning section (FR-009, US2
  scenario 5)
- [ ] T017 [US2] S13 across the four pages: wherever the quick switcher is opened, name the jump-to
  field in the title bar and Ctrl+Shift+P (Cmd+Shift+P on macOS) (FR-008, US2 scenario 3)
- [ ] T018 [P] [US2] S7–S10 outside the four pages, those sentences and rows only (FR-011):
  `docs/user-guide/appearance-theming.md` (~4, ~12, ~58), `docs/user-guide/icons.md` (~12, ~30,
  ~41, ~86), `docs/user-guide/settings.md` (~4, ~52), `docs/user-guide/sandboxed-daemon.md` (~505):
  each removed element becomes the shipped one and links to its owning section; a hit that says the
  element is gone stays (historical)
- [ ] T019 [P] [US2] S11 in `site/media.toml`: the alt text and captions of the four pages' media
  that name a removed element (`worktree-sidebar-light`: "each tagged with the kind of branch it
  holds") describe what shipped (research R4)
- [ ] T020 [US2] Verify stage 2: all patterns (stage 1 and 2, plus T012's) return only owning
  sections, linked mentions and historical hits (SC-001, SC-002); the five SC-007 answers are each in
  the first paragraph of their owning section; every chord has its macOS form (FR-018); A1 passes;
  `scripts/tests/media-references.test.sh` and `site/checks/media-references.sh` pass after T019's
  `site/media.toml` edit; `git diff --stat origin/main -- 'crates/*/src'` is empty (FR-020)

---

## Phase 5: User Story 3 — The layout snapshot holds the whole redesigned window (P1)

**Blocked until #1125 and #1127 have merged on main** (the title bar's anchors and height and the
Overview row's selection come from them, research R5, R7).

**Goal**: two whole-window covered states, one per mockup board, held by a region gate and a
regenerated fixture.

**Independent test**: quickstart A3: `layout_snapshot` and `layout_text_overflow` pass, the two
states are in the fixture, gate W1–W5 pass in both schemes, a regeneration leaves the fixture
unchanged, and the five mutations fail as listed.

- [ ] T021 [US3] Extend `strip_with` in `crates/micold-client/tests/support/covered_states.rs`
  (today `pub fn strip_with(shells: usize, helpers: usize) -> StateUnderTest`, ~2578) with a
  variant that takes the base `State` and the location; `strip_with` itself calls it with its
  current base and `feat-short`, so its callers and their fixture sections stay byte-identical. At a
  location that already has a session, the variant replaces that session with its main session,
  keeping its `ActivitySignal` (research R5). Reuse, do not copy, `with_every_card_state()` (~486)
  and `running_sandbox()` (~2690)
- [ ] T022 [US3] Register `window-session-view` in
  `crates/micold-client/tests/support/covered_states.rs` after the status-bar states, one entry and
  nothing else (`tests/layout_coverage_registry.rs`): `with_every_card_state()` with the session
  shown at `feat-working` holding the main session, 2 shell tabs and 1 helper tab (the T021
  variant, `ActivitySignal::Working` kept), `running_sandbox()`, connected, 1280×800; anchors
  `shell.root`, `title_bar` (the bar, not its container), `shell.body`, `sidebar`, `session.area`,
  `status_bar` ([contracts/covered-states.md](contracts/covered-states.md))
- [ ] T023 [US3] Register `window-overview` in the same file: the same base with the Overview open
  (not full screen) and the sidebar's Overview row selected, through the field or accessor #1125 M3
  shipped; same anchors, `session.area` being the Overview in its place
- [ ] T024 [US3] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` with
  `UPDATE_LAYOUT_SNAPSHOT=1 mise exec -- cargo test -p micold-client --test layout_snapshot`;
  confirm the diff only adds the two states (every existing state's section unchanged); copy the
  anchor paths from the fixture into T022/T023's entries (never counted from the source,
  `layout-snapshot.md#anchors`); re-run without the variable and confirm it passes (FR-013)
- [ ] T025 [US3] Add gate W in `crates/micold-client/tests/gates/window_regions_fit.rs` (new),
  included from `crates/micold-client/tests/layout_snapshot.rs` with one `#[path =
  "gates/window_regions_fit.rs"] mod` as `gates/status_bar_fits.rs` is, reading the cached records
  of both states in both schemes: W1–W4 on the records, W5 on the `window-overview` `State`, exactly
  as [contracts/covered-states.md §Gate W](contracts/covered-states.md#gate-w--testsgateswindow_regions_fitrs);
  heights from `micold_core::tokens::anatomy::app_bar::{HEIGHT, BOTTOM_EDGE}` and
  `anatomy::status_bar::HEIGHT`, never literals (if #1127 dropped the divider, W3 follows what
  shipped). Each failure message names the state, the anchor and both numbers
- [ ] T026 [US3] Run `mise exec -- cargo test -p micold-client --test layout_snapshot` and `--test
  layout_text_overflow`: both pass for the two new states in both colour schemes, containment
  included (US3 scenario 4, FR-013). Measure the two states' cost per
  `docs/development/layout-snapshot.md#adding-a-covered-state`: time a warm `layout_snapshot` run
  (a second run after the build) with the two entries and with them removed, and note the
  difference for T028
- [ ] T027 [US3] Mutation proof: apply each of the five edits of
  [covered-states §Mutation proof](contracts/covered-states.md#mutation-proof-quickstart-a3) in
  turn, run `layout_snapshot`, confirm the failure names the state and the listed W assertion,
  revert; record the five outcomes for the PR description and the ledger (SC-005; the red of
  Constitution I)
- [ ] T028 [US3] In `docs/development/layout-snapshot.md` *What is covered*, one passage listing each
  region's states with the issue that added them (#1124 `status-bar-*`, #1125
  `main-shell-sidebar-grouped` and its later states, #1126 `session-area-*`, #1127's states) and
  the two whole-window states, with their measured warm cost (FR-014, research R7)
- [ ] T029 [US3] Verify: quickstart A3 in full (pass, regeneration leaves `git diff --exit-code
  crates/micold-client/tests/fixtures/layout_snapshot.txt` clean); A1 for `layout-snapshot.md`;
  `git diff --stat origin/main -- 'crates/*/src'` empty (FR-020)

---

## Phase 6: User Story 4 — A visual pass checks the finished window against the mockup (P2)

**Blocked until M3 has merged** (it draws M3's states).

**Goal**: four frames at 1280×800, 20 verdicts with reasons, contrast per region.

**Independent test**: quickstart §B: 4 screenshots, 20 verdicts, every non-*matches* verdict with
its reason (SC-006).

- [ ] T030 [US4] Draw `window-session-view` and `window-overview` at 1280×800 in light and dark
  through `ui::view` and the tiny-skia renderer from a harness in the session scratchpad (not
  committed), or with the `visual-pass` skill on Xvfb (research R8); save
  `specs/1128-layout-redesign-docs/visual-pass/{session-view,overview}-{light,dark}-1280.png`
- [ ] T031 [US4] Write the header of `specs/1128-layout-redesign-docs/visual-pass.md` per
  [contracts/visual-pass-record.md §Header](contracts/visual-pass-record.md#header): full commit
  SHA, how it was run, states, size, renderer; not checked (macOS, Windows, other sizes, failure
  states); the mockup boards
- [ ] T032 [US4] One table per screenshot in `specs/1128-layout-redesign-docs/visual-pass.md`: five
  verdicts (title bar, sidebar, session area or Overview, status bar, whole window), each
  *matches*, *differs on purpose* (the decision and its spec) or *differs by defect* (the region's
  issue and the follow-up) (FR-015, FR-017)
- [ ] T033 [US4] Contrast column per region and theme: cite the composition-contrast test of
  research R9, or measure with `micold_core::tokens::contrast` in a scratchpad probe and record the
  ratio; body text under 4.5:1 is *differs by defect* (FR-016)
- [ ] T034 [US4] Each *differs by defect* gets a line in the ledger's *Follow-ups not done* (region,
  issue, what is wrong, screenshot); no code change (D1, FR-020). Check quickstart §B's expected
  counts

---

## Phase 7: Polish & cross-cutting

Changes no code; left to the close unit's PR (milestones rule 4).

- [ ] T035 Quickstart A4 (`mise run gate`) and the FR-020 check over the whole feature's diff;
  tick the tasks the milestones completed

---

## Dependencies & execution order

- Phase 1 → Phase 2 → Phase 3 (M1): now.
- Phase 4 (M2) and Phase 5 (M3): after **#1125 and #1127 merge** (external) and after M1. They touch
  disjoint files and may run side by side, but merge in order M2, M3.
- Phase 6 (M4): after M3.
- Phase 7: the close unit.
- Within US1: T003–T007 edit `worktrees-and-sessions.md` in order; T008, T009 are other files
  and may run beside them. T010 after T001; T011 last.
- Within US2: T012 first; T014, T018, T019 are other files; T013, T015–T017 follow T012; T020 last.
- Within US3: T021 → T022 → T023 → T024 → T025 → T026 → T027; T028 after T024; T029 last.

## Parallel example: User Story 1

```text
T003–T007 worktrees-and-sessions.md, in order
T008 overview.md:263
T009 settings.md, sandboxed-daemon.md
```

## Implementation strategy

### MVP first

M1 alone corrects the two shipped regions' mentions on main now; nothing in it waits for #1125 or
#1127. Stage 2 then lands M2 (pages), M3 (snapshot) and M4 (visual pass) as #1125 and #1127 merge.

### Incremental delivery

Each milestone merges on its own and leaves main green. CI skips the test suite only for a PR whose
every path is marked `micold-docs` in `.gitattributes`: M1 and M4 do so unless they touch another
path (M1's T010 may edit `site/media.toml`, which is not `micold-docs`); M2 edits `site/media.toml`
(T019), which `site/checks/media-references.sh`, `site/build.sh` and `site/stage.sh` read, so its
PR runs the pipeline; M3 runs it.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Status bar and session area in the user guide 🎯 MVP

- **Tasks**: T001–T011
- **Deliverable**: on main, the user guide places connection, sandbox, CLI and plan usage in the
  status bar and a session's state, restart and instances in the context header and tab strip, each
  linked to its owning section, with no bar under the terminal and no unnamed "bar".
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001–FR-004, FR-011 (stage 1), FR-017–FR-020;
  SC-001 (stage 1), SC-002 (stage 1), SC-003, SC-007 (connection state, switch to a shell)
- **Verify**: quickstart A1 and A2 (stage-1 patterns); `git diff --stat origin/main -- 'crates/*/src'`
  empty
- **Depends on**: — (#1124 and #1126 already merged)
- **Tier**: full

### M2 — Sidebar and title bar in the user guide

- **Tasks**: T012–T020
- **Deliverable**: on main, the user guide opens `worktrees-and-sessions.md` with the four regions
  and describes the shipped title bar and sidebar, each element once and linked, with no removed
  element named as present on any page or in `site/media.toml`.
- **Satisfies**: US2 acceptance scenarios 1–5; FR-005–FR-011, FR-017–FR-020; SC-001, SC-002,
  SC-003, SC-007; spec Edge cases (no project, no worktrees, multi-session, cross-platform)
- **Verify**: quickstart A1 and A2 (all patterns); SC-007 first-paragraph check;
  `scripts/tests/media-references.test.sh`
- **Depends on**: M1; **external: #1125 and #1127 merged on main** (do not start before)
- **Tier**: full (T019 edits `site/media.toml`, not a docs path; 9 tasks)

### M3 — Whole-window covered states and gate W

- **Tasks**: T021–T029
- **Deliverable**: on main, the layout snapshot covers `window-session-view` and `window-overview`
  at 1280×800, gate W holds their regions against each other, and a regeneration reproduces the
  committed fixture.
- **Satisfies**: US3 acceptance scenarios 1–5; FR-012–FR-014, FR-019 (`layout-snapshot.md`),
  FR-020; SC-004, SC-005
- **Verify**: quickstart A3 (tests, regeneration diff, mutation proof); A1
- **Depends on**: M1, M2 (merge order); **external: #1125 (Overview row selection, its M3) and
  #1127 (title bar) merged on main**
- **Tier**: full
- **User-guide gate**: no user-guide page changes, so the PR takes the `docs-not-needed` label.

### M4 — Visual pass against the mockup boards

- **Tasks**: T030–T034
- **Deliverable**: `specs/1128-layout-redesign-docs/visual-pass.md` on main holds 4 frames at
  1280×800 (2 boards × 2 themes) and 20 verdicts with reasons and contrast.
- **Satisfies**: US4 acceptance scenarios 1–4; FR-015, FR-016, FR-017, FR-020; SC-006
- **Verify**: quickstart §B
- **Depends on**: M3; external: as M3
- **Tier**: full (committed paths are specs only, but the render harness in the scratchpad is Rust)
