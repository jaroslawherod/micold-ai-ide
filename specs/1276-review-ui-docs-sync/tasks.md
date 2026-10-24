---

description: "Task list for feature 1276: Review UI — docs, layout snapshot, visual pass and mockup sync"
---

# Tasks: Review UI — docs, layout snapshot, visual pass and mockup sync

**Input**: Design documents from `specs/1276-review-ui-docs-sync/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Blocked**: every task reads the build. Nothing here starts before its milestone's blockers
(#1269–#1275, #1128) have merged to `main`; each milestone below names the ones it waits on. Where a
slice merged differently from its issue, the build wins and the difference goes in T034 (spec
Context, FR-013).

**Tests**: Constitution Principle I (Test-First, NON-NEGOTIABLE): each test task is written first and
seen failing for the right reason before the task that makes it pass. This feature adds no
production code: its code is one `micold-core` test (FR-004), test-side layout states and one gate
(FR-005–FR-007). The visual pass (quickstart §B) and the mockup comparison (quickstart §C) are the
checks of how it looks; docs are checked by the quickstart §A greps and the docs CI.

**Documentation**: Principle VII: the user guide is US1's deliverable, in M1's `feat` PR.

**Cross-platform**: Principle VI: the fixture must pass on Linux, macOS and Windows CI (US2 sc.2);
the guide's key cells name the macOS form of every primary-modifier chord, checked by T005.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US5)

---

## Phase 1: Setup

**Purpose**: start from the merged build.

- [ ] T001 Rebase on `origin/main` (`scripts/autopilot/branch-start.sh`) and confirm the milestone's
  blockers are all `MERGED` on GitHub; record each blocker's merge PR in the ledger
  (`specs/1276-review-ui-docs-sync/autopilot.md`, input to T033). Re-run
  `grep -rn "Changes view" docs/` and keep the hit list in the scratchpad for T011–T013 (line numbers
  in [contracts/guide-outline.md](contracts/guide-outline.md) § *Inbound links* are pre-rebase).
- [ ] T002 Locate the review key table #1274 merged in `micold-core` (`workspaceSymbol` for
  `REVIEW_KEYS`, then `grep -rn` for the key `Ctrl+Shift+Enter` / `'?'` under `crates/micold-core/src`)
  and note in [contracts/guide-shortcut-table.md](contracts/guide-shortcut-table.md) its real path and
  fields (key, modifier set or primary flag, sheet text). If #1274 merged no `micold-core` table the
  client binds, file a blocker issue for it, link it from the ledger and the spec status, and stop
  T004–T006 until it merges (plan §1, research R1; this feature never moves the table).

---

## Phase 2: Foundational

**Purpose**: share the marker-table helpers before a second test uses them (plan §1, research R3).

- [ ] T003 Move `marked_section` and `rows` from `crates/micold-core/tests/terminal_only_doc.rs`
  into `crates/micold-core/tests/support/marked_table.rs`, generalised to take the page path and the
  marker name (`terminal-only`, `review-shortcuts`), declared in `crates/micold-core/tests/support/mod.rs`;
  `terminal_only_doc.rs` calls them. Moved, not copied (`mise run duplication`). Refactor under green:
  `cargo test -p micold-core --test terminal_only_doc` passes before and after.

**Checkpoint**: the helpers are shared; US1 can start.

---

## Phase 3: User Story 1 — The user guide describes the review tab as it is (P1) 🎯 MVP

**Goal**: `reviewing-changes.md`, `terminal-panes.md` and `worktrees-and-sessions.md` describe the
Review tab as built by R0–R6, and the guide's shortcut list is held equal to the bound keys by a test.

**Independent Test**: quickstart §A1–§A3 and §A6: `cargo test -p micold-core --test review_keys_doc`
passes and fails on a changed `Does` cell; the §A2 grep returns nothing from the five pages;
`site/checks/links.sh --sources` passes.

### Tests for US1 (write first)

- [ ] T004 [US1] Write `crates/micold-core/tests/review_keys_doc.rs` with
  `the_markers_are_there`, `the_guides_table_is_the_review_key_table` and
  `each_primary_chord_names_its_macos_form`, per [contracts/guide-shortcut-table.md](contracts/guide-shortcut-table.md):
  rows rendered from the T002 table in its order, key cell modifiers ordered `Ctrl+Alt+Shift+<key>`,
  ``(`Cmd+…` on macOS)`` suffix exactly on primary-modifier chords, `Does` = sheet text verbatim;
  failure messages list both row sets / name the missing marker. Uses T003's helpers. Run it: it
  fails on `the_markers_are_there` (no `review-shortcuts` markers in the page yet).
- [ ] T005 [US1] Only if T002 found the table *thinner* (contract § *If the table is thinner*): with no
  sheet text, narrow T004's row comparison to the `Key` cells and write
  `crates/micold-client/tests/review_sheet_doc.rs` comparing each `Does` cell with the `?` sheet's text
  for that key, with the same marker helpers (shared, not copied); with no primary flag, derive the
  suffix from the modifier set as the contract says. Seen failing before T007. Otherwise mark it
  "not needed" with the reason.
- [ ] T006 [US1] Add the commented `/docs/user-guide/reviewing-changes.md -micold-docs` block of the
  contract § *Attribute* to `.gitattributes` after `docs/**`, in the form of the
  `worktrees-and-sessions.md` entry; `cargo test -p micold-core --test documentation_is_not_read` and
  `scripts/tests/documentation-set.test.sh` pass.

### Implementation for US1

- [ ] T007 [US1] Restructure `docs/user-guide/reviewing-changes.md` to the headings of
  [contracts/guide-outline.md](contracts/guide-outline.md), each control a heading (SC-005), starting
  from the slices' own edits: *Opening a review*, *Keeping and closing the Review tab*, *Choosing what
  to review*, *The file tree*, *Marking files reviewed*, *Commenting on lines* (drag-to-comment,
  suggestion chips off by default), *Sending a review* (review bar, *Sending…*, send error, overflow),
  *The comments drawer*, *Changed since you looked*, *Keyboard* (with the `review-shortcuts` marker
  table, which makes T004/T005 pass), *Two worktrees, two windows* (per-worktree state; windows share
  marks and drafts, also during a send). Keep the text and heading ids of *Reading a diff*, *The
  review message*, *Statuses and replies*, *The base line*, *The Default entry*. State the
  PR-diff-cannot-be-read error and every `Ctrl`/`Cmd` difference (FR-001). Use the merged slices'
  wording of each control.
- [ ] T008 [US1] In the same page, rewrite *Pull requests* (heading and `#pull-requests` kept) for the
  source control's **Pull requests** entry per the outline, fold *The review panel* and *Committed and
  uncommitted* into their new sections, and remove every statement on the outline's *Must not say*
  list, including the *Changes view* hits at the page's top and its old line 132 (FR-002).
- [ ] T009 [P] [US1] `docs/user-guide/terminal-panes.md` tab strip section: the **Review** tab as a
  strip member with its badge and *changed since you looked* mark; pane chords and focus while it is
  shown, as R0 built them (FR-003).
- [ ] T010 [P] [US1] `docs/user-guide/worktrees-and-sessions.md`: row menu **Review changes** opens or
  focuses the Review tab (old line ~978); the header **Changes** button; the sidebar's *Review · n
  drafts* line if the build shows it; the `#pull-requests` link text at old line ~240 reworded to the
  source control's **Pull requests** entry (FR-003, FR-014).
- [ ] T011 [P] [US1] `docs/user-guide/settings.md` (old lines 653, 659, 660) and
  `docs/user-guide/agent-tools.md` (old lines 299, 300, 311): reword each *Changes view* mention to the
  Review tab / source control as the outline's § *Inbound links* lists; re-point `#sending-a-review`
  and `#statuses-and-replies` if T007 moved them (FR-014).
- [ ] T012 [P] [US1] `docs/user-guide/parallel-runs.md` (old lines 84, 91, 92) and `docs/README.md`
  (old line 45): reword *Changes view* to the Review tab per the outline's § *Inbound links*.
- [ ] T013 [US1] Check US1: quickstart §A2 grep over the five pages returns nothing, and
  `grep -rn "Changes view" docs/` returns only `docs/development/layout-snapshot.md` (T024) and
  `docs/development/review-redesign.md` (T034); read each `committed\|pull requests` hit; §A3 holds;
  `site/checks/links.sh --sources` passes; the mdBook build passes; `scripts/gate.sh` green.

**Checkpoint**: M1 deliverable observable.

---

## Phase 4: User Story 2 — The layout snapshot pins the finished review layout (P1)

**Goal**: one fixture recorded against the final layout covers every review state of FR-006, gated at
both windows (FR-007), and `layout-snapshot.md` names them (FR-008).

**Independent Test**: `cargo test -p micold-client --test layout_snapshot` and `--test
layout_text_overflow` pass on a clean checkout; +1 px on the review bar's padding fails naming
`review.bar` (quickstart §A7).

### Tests for US2 (write first)

- [ ] T014 [US2] Run `cargo test -p micold-client --test layout_snapshot` on the rebased tree and
  confirm it passes as merged (each slice re-recorded its own states; plan §2 step 1). A failure here
  is a blocker's defect: file it, do not re-record over it.
- [ ] T015 [US2] Register in `crates/micold-client/tests/support/covered_states.rs` every state of
  [contracts/review-states.md](contracts/review-states.md) no slice registered (a slice's own name for
  a state stands), reusing the slices' state builders (`grep -n` each slice's builder first), with the
  anchors the contract lists and an anchor on each conditional child; `review-source-open` as a press
  with `over` records. Run `layout_snapshot`: it fails `the_layout_matches_the_committed_fixture`
  naming the new states.
- [ ] T016 [US2] Redefine `session-area-changes-open` by changing `changes_open_under_header()` in
  `crates/micold-client/tests/support/covered_states.rs` in place (Review tab selected in the strip,
  header above, review body; research R5), and update its users:
  `crates/micold-client/tests/gates/session_area_fits.rs::the_header_stands_over_the_open_changes_view`
  (header, strip with the Review tab, then the review body) and
  `crates/micold-client/tests/session_area_stability.rs` (`rendered_presses`,
  `pressing_changes_with_the_view_open_on_the_shown_location_closes_it`, whose expected message becomes
  R0's for **Changes** pressed while the Review tab is shown). If R0 already changed them, follow R0.
- [ ] T017 [US2] Extract the status bar's invariants (28 dp high, flush with the window's bottom edge,
  full width) from
  `crates/micold-client/tests/gates/status_bar_fits.rs::at_the_smallest_window_the_bars_texts_stay_in_their_boxes`
  into a `pub(crate) fn` that test calls; make `assert_fits_under`, `assert_order`, `header_parts` and
  `strip_parts` in `crates/micold-client/tests/gates/session_area_fits.rs` `pub(crate)`; where the
  Review tab changes the strip's child count, take it from `STRIP_MEMBERS`. Existing gates stay green.
- [ ] T018 [US2] Write `crates/micold-client/tests/gates/review_tab_fits.rs`, registered in
  `crates/micold-client/tests/layout_snapshot.rs` with a `#[path]` module as `session_area_fits` is:
  for every `review-*` state and `session-area-changes-open`, both themes, at `MIN_WINDOW_SIZE` and
  1280×800 — no `text_overflows`, no `escapes` beyond declared exemptions, `review.bar` bottom ≤
  `status_bar` top and both inside the window, `review.drawer` inside `review.diff` with the diff's
  width unchanged, plus T017's status-bar fn and `assert_fits_under`. Only `tests/support/layout.rs`
  helpers; nothing copied from `layout_text_overflow.rs`, `gates/containment.rs`,
  `status_bar_fits.rs` or `session_area_fits.rs`. Seen failing on a state with a deliberately wrong
  anchor before it is accepted.

### Implementation for US2

- [ ] T019 [US2] Re-record the fixture once:
  `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot`, updating
  `crates/micold-client/tests/fixtures/layout_snapshot.txt` (FR-005); then `layout_snapshot` and
  `layout_text_overflow` pass without the variable, and `layout_snapshot.rs`'s
  coverage-narrowing test (that file's own T016, not this list's) passes.
- [ ] T020 [US2] Write the fixture diff's explanation, line group by line group (review-tab states;
  any other group, e.g. #1128 Overview drift, with its reason), into the M2 PR body draft in the
  scratchpad (FR-005, US2 sc.1).
- [ ] T021 [US2] Prove the gate is live: +1 px on the review bar's padding in the review bar's style
  (the file R1 merged) fails `layout_snapshot` naming `review.bar`; revert; record the output line in
  the PR body draft (US2 Independent Test).
- [ ] T022 [US2] Time each new covered state warm as `docs/development/layout-snapshot.md` § *Adding a
  covered state* asks.
- [ ] T023 [US2] `docs/development/layout-snapshot.md` § *What is covered*: one paragraph in the
  feature-paragraph form (features 1124–1126) naming each review state of the contract and what it
  anchors, and the warm timing from T022 in § *Adding a covered state* (FR-008).
- [ ] T024 [US2] Same file: replace the `session-area-changes-open` sentences (old line ~105, "header
  over the open Changes view, with no strip") with what T016 records (US2 sc.3); `grep -n "Changes view"
  docs/development/layout-snapshot.md` returns nothing.
- [ ] T025 [US2] `scripts/gate.sh` green; CI green on Linux, macOS and Windows (US2 sc.2).

**Checkpoint**: M2 deliverable observable.

---

## Phase 5: User Story 3 — A recorded visual pass shows the review tab looks right (P2)

**Goal**: 17 checks × 2 themes at 1280×800 with a verdict each (FR-009), no FAIL unrecorded (FR-010).

**Independent Test**: `specs/1276-review-ui-docs-sync/visual-pass/` holds a light and a dark crop per
check and `results.md` with a verdict per check; each FAIL has a fix commit or an issue link
(quickstart §B).

- [ ] T026 [US3] Build the world of [data-model.md](data-model.md) § *Visual-pass world* in a private
  data directory in the scratchpad: worktree A (≥ 12 files, nested, deep path, long name, binary; 2
  drafts; 2 sent, 1 replied; 3 reviewed, 1 reviewed then changed), worktree B with its own open Review
  tab and selection, worktree C with no changes, and the stub `gh` (`sh` script first on the daemon's
  `PATH`, on the pattern of `crates/micold-core/tests/github_gh_cli.rs`) listing one long-titled PR for
  A and exiting non-zero on its diff. All invented data.
- [ ] T027 [US3] Run the `visual-pass` skill (through an `autopilot-worker`) with the worktree path,
  the checks of [contracts/visual-pass-checks.md](contracts/visual-pass-checks.md), the T026 world,
  1280×800, light and dark, output to `specs/1276-review-ui-docs-sync/visual-pass/`
  (`NN-<check>-{light,dark}.png`).
- [ ] T028 [US3] Write `specs/1276-review-ui-docs-sync/visual-pass/results.md`: date, environment
  (Xvfb + lavapipe), verdict and one line per check, macOS / Windows recorded as not run with the
  reason unless a host was available.
- [ ] T029 [US3] For each FAIL: a styling or doc defect is fixed in the shared component or page (never
  a feature-local override) and its check re-shot; a behaviour defect is filed as an issue; the fix
  commit or issue link goes on the check's row in `results.md`, and the spec status names any filed
  issue (FR-010, US3 sc.2). A fix that changes geometry re-runs T019's re-record and says why in the
  PR. None: mark it "not needed".

**Checkpoint**: M3 deliverable observable.

---

## Phase 6: User Story 4 — The mockup canvas matches the build (P2)

**Goal**: every board and shared component of `https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu`
shows what the build draws, Overview board and `OverviewCard` included (FR-011, FR-012).

**Independent Test**: `specs/1276-review-ui-docs-sync/visual-pass/mockup-sync.md` lists every board
against its crop with 0 unlabelled differences (quickstart §C, SC-004).

- [ ] T030 [US4] Read the canvas with the Artifact tool (`action: read`); note whether it says
  *writer* and where the saved file is.
- [ ] T031 [US4] Compare each board of [data-model.md](data-model.md) § *Mockup board* with its M3 crop
  (labels, order and presence of controls, count formats, state colours), and the Overview board and
  `OverviewCard` with crop 17 (tiers, stripe on a neutral card, empty rows removed, title wrapping,
  cards per row at the board's width); write each difference in
  `specs/1276-review-ui-docs-sync/visual-pass/mockup-sync.md` (board, crop, difference, action).
- [ ] T032 [US4] With *writer*: fix each difference in the shared components (`TitleBar`, `Sidebar`,
  `StatusBar`, tab strip, review bar, drawer, source control, file tree row, comment card, shortcut
  sheet, `OverviewCard`) on the canvas's `tokens.css`; label "follow-up, not built" each element that
  `docs/development/review-redesign.md` or a slice's merged spec lists as a follow-up (the split
  option of section 0 at least), and remove every other unbuilt element (FR-012); republish to the same `url`. Without *writer*: hand the owner the
  `mockup-sync.md` list and note it in the spec status (spec Edge Cases). Record the action per row.

**Checkpoint**: M4 deliverable observable.

---

## Phase 7: User Story 5 — The redesign doc records what was built (P3)

**Goal**: `docs/development/review-redesign.md` is a record, not a proposal (FR-013).

**Independent Test**: quickstart §A5.

- [ ] T033 [US5] Gather from each R0–R6 slice's merged spec and PR (T001's list): the PR number, the
  answer the build took for open questions 0–5, and each difference from the proposal (renames, keys
  bound differently, parts cut); note them in the scratchpad.
- [ ] T034 [US5] Rewrite `docs/development/review-redesign.md` per plan §6: title and status *Review UI
  redesign: record* / built (epic #1277); the Tasks table gains a PR column (R0–R6 merged PRs, R7 this
  feature's PRs); each open question 0–5 answered; a *Differences from the proposal* list; a
  *Follow-ups not built* list (split view, section 0, and every element M4 labelled "follow-up, not
  built" in `visual-pass/mockup-sync.md`) with issue links where they exist; the "visual
  pass at 900 / 1280 / 1920" line in *Scope and files* corrected to 1280×800 (clarification 2). Its
  *Changes view* wording stays only where it describes the pre-redesign view. `site/checks/links.sh
  --sources` passes.

**Checkpoint**: M5 deliverable observable.

---

## Phase 8: Polish & Cross-Cutting (close unit; no code)

- [ ] T035 Run quickstart §A–§C end to end on `main` and record the results in the close PR.
- [ ] T036 Set spec status to built, naming any issue filed by T002 or T029 and the mockup route
  taken by T032; tick the tasks.

---

## Dependencies & Execution Order

- **Setup (T001–T002)** before everything; T002 gates T004–T007.
- **Foundational (T003)** before T004.
- **US1 (T004–T013)**: T004 → T005 (if needed) → T007 → T008; T006, T009–T012 in parallel after T001;
  T013 last.
- **US2 (T014–T025)**: needs #1128 as well, and M1 merged (T024 and T013 split the `layout-snapshot.md`
  grep). T014 → T015 → T016 → T017 → T018 → T019 → T020–T024 → T025.
- **US3 (T026–T029)**: needs M2 merged, so the pass sees the final tree.
- **US4 (T030–T032)**: needs M3's crops.
- **US5 (T033–T034)**: needs the blockers merged and M4 (T034 lists the follow-ups M4 labelled).
- **Polish (T035–T036)**: close unit, after M5.

## Parallel Example: US1

```text
T009 terminal-panes.md · T010 worktrees-and-sessions.md · T011 settings.md + agent-tools.md ·
T012 parallel-runs.md + docs/README.md   (different files, all after T001)
```

## Implementation Strategy

MVP is M1: the guide a user reads. Then the fixture (M2) once the last blocker, #1128, merges; then
the look (M3) on the final tree; the mockup (M4) from M3's crops; and the record (M5). Nothing starts
before its blockers merge.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). **None can be
implemented now**: every one waits on unmerged blockers, listed under **Depends on**.

### M1 — The user guide describes the Review tab, keys held by a test 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: the guide's *Reviewing a worktree's changes* chapter, `terminal-panes.md` and
  `worktrees-and-sessions.md` describe the Review tab as R0–R6 built it, with no pre-redesign
  statement and no *Changes view* left on user-guide pages, and `review_keys_doc` fails CI when a
  review key changes without the guide
- **Satisfies**: US1 acceptance scenarios 1–7; FR-001, FR-002, FR-003, FR-004, FR-014; SC-001, SC-005;
  edge cases empty review, failure states, multi-session, cross-platform keys (guide side)
- **Verify**: `cargo test -p micold-core --test review_keys_doc --test terminal_only_doc --test
  documentation_is_not_read`; `scripts/tests/documentation-set.test.sh`;
  `site/checks/links.sh --sources`; quickstart §A1–§A3, §A6; `scripts/gate.sh`
- **Depends on**: blockers #1269, #1270, #1271, #1272, #1273, #1274, #1275 merged (#1128 not needed);
  #1274 must ship the `micold-core` key table, else the T002 blocker issue
- **Tier**: full

### M2 — The layout fixture pins every review state at both windows

- **Tasks**: T014–T025
- **Deliverable**: `layout_snapshot` covers every review state of FR-006, re-recorded once against the
  final layout, and fails naming `review.bar` on a one-pixel padding change; `review_tab_fits` holds
  every FR-007 gate at the smallest window and 1280×800 in both themes; `layout-snapshot.md` names the
  states
- **Satisfies**: US2 acceptance scenarios 1–3; FR-005, FR-006, FR-007, FR-008; SC-002; edge case
  long names (snapshot side), cross-platform fixture
- **Verify**: `cargo test -p micold-client --test layout_snapshot --test layout_text_overflow --test
  session_area_stability`; quickstart §A4, §A7; `scripts/gate.sh`; CI on Linux, macOS, Windows
- **Depends on**: M1; blockers #1269–#1275 and #1128 all merged. Test and developer-doc only: if CI's
  user-guide gate fires on the `feat` title, the PR takes `docs-not-needed` (M1 shipped the guide)
- **Tier**: full

### M3 — A recorded visual pass of the finished review tab

- **Tasks**: T026–T029
- **Deliverable**: `specs/1276-review-ui-docs-sync/visual-pass/` holds a light and a dark crop at
  1280×800 for each of the 17 checks and `results.md` with a verdict each; every FAIL has a fix or a
  linked issue
- **Satisfies**: US3 acceptance scenarios 1–2; FR-009, FR-010; SC-003; edge cases empty review,
  failure states, multi-session (two-worktree switch), long names, cross-platform (recorded not run)
- **Verify**: quickstart §B; `ls specs/1276-review-ui-docs-sync/visual-pass/*-{light,dark}.png | wc -l`
  is 34; `results.md` has no FAIL row without a commit or issue link; `scripts/gate.sh` when T029
  changed code
- **Depends on**: M2 (the pass sees the final tree); blockers #1269–#1275 and #1128 merged
- **Tier**: full (a FAIL fix may change a shared component's style)

### M4 — The mockup canvas matches the build

- **Tasks**: T030–T032
- **Deliverable**: every board of the *Review tab redesign* canvas, the Overview board and
  `OverviewCard` included, matches its M3 crop or carries a "follow-up, not built" label, and
  `visual-pass/mockup-sync.md` records each comparison (or the hand-over list to the owner)
- **Satisfies**: US4 acceptance scenarios 1–3; FR-011, FR-012; SC-004; edge case mockup not editable
- **Verify**: quickstart §C; `mockup-sync.md` has no difference row without an action
- **Depends on**: M3 (its crops); blockers #1269–#1275 and #1128 merged
- **Tier**: docs

### M5 — The redesign doc is a record of what was built

- **Tasks**: T033–T034
- **Deliverable**: `docs/development/review-redesign.md` reads as a record: status built (epic #1277),
  every R0–R7 row with its PR, open questions 0–5 answered, differences and follow-ups listed
- **Satisfies**: US5 acceptance scenarios 1–2; FR-013; edge case a blocker lands differently
- **Verify**: quickstart §A5; `site/checks/links.sh --sources`
- **Depends on**: M1 (T001's PR list), M4 (its follow-up labels); blockers #1269–#1275 and #1128
  merged
- **Tier**: docs

Polish (T035–T036) changes no code: the close unit does it in the close PR.
