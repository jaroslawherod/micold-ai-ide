---
description: "Task list for feature 038 — reporter, labels and a description tooltip in the issue list"
---

# Tasks: Reporter, Labels and a Description Tooltip in the Issue List

**Input**: Design documents from `/specs/038-issue-list-reporter-tooltip/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY and must be observed failing before their implementation. Behavior ids (`[A#]`, `[U#]`)
refer to [tdd/test-list.md](./tdd/test-list.md); `/speckit.tdd.run` ticks a task from them. Every
phase lists its failing tests first. The GUI exception is claimed only for `src/ui/` composition and
pixels, verified by the recorded quickstart §B pass.

**Documentation**: Per Constitution Principle VII, each user-facing story carries its own user-guide
task in the milestone that ships it (CI's user-guide gate).

**Cross-platform**: Per Constitution Principle VI, this feature adds no `cfg` arm. Every rule is in
`micold-core`, whose suite CI runs on Linux, macOS and Windows.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story the task belongs to (US1–US3)
- Every description carries an exact file path

## Path Conventions

Three-crate workspace: `crates/micold-core/` (render-free rules), `crates/micold-client/` (form,
shell, UI, showcase). The daemon is not touched. Build and test through `mise run <task>`
(CLAUDE.md). "NEW" marks a file that does not exist yet.

---

## Phase 1: Setup (Shared Infrastructure)

No setup tasks: no new crate, dependency, binary or configuration (plan, Structure Decision).

---

## Phase 2: Foundational (Blocking Prerequisites)

No foundational tasks. Every prerequisite serves one story and ships in that story's first slice.

---

## Phase 3: User Story 1, slice A — two wrapping lines with the reporter (Priority: P1) 🎯 MVP

**Goal**: Each issue row shows `#<number> <title>` and, below it, the reporter and the labels; both
lines wrap and nothing is cut. The single-line row of the branch picker and `Select` is unchanged.

**Independent Test**: quickstart §B1, B2; automated: `github_parse.rs`, `github_issue_lines.rs`,
`github_privacy.rs`, `issue_picker_rows.rs`, `gates/issue_rows_show_all_text.rs`.

### Tests for User Story 1, slice A (MANDATORY — Constitution Principle I) ⚠️

- [X] T001 [P] [US1] [A15] [U1] [U2] [U3] [U4] [U5] [U6] Extend `crates/micold-core/tests/github_parse.rs` and add a captured node to `crates/micold-core/tests/fixtures/gh/` (contracts/issue-fields.md §1–2):
  - A node with `"author": {"login": "octocat"}` parses to `reporter() == "octocat"`.
  - `"author": null` and a node with no `author` key parse to `reporter() == "ghost"`; the issue is still listed.
  - A bot's `login` is kept as reported.
  - `LIST_QUERY`, `SEARCH_QUERY` and `SEARCH_WITH_NUMBER_QUERY` each contain the one shared node selection, and it contains `author { login }`; a list node, a search node and a typed-number node with the same fields parse to equal `Issue`s (FR-006).
  - `list_args_send_only_the_repository` and `search_args_send_only_the_query` also assert that no argument contains `author:` and that the arguments are the ones sent today (FR-013, FR-026).
- [X] T002 [P] [US1] [U7] [U8] [U9] [U10] [U11] [U12] [U13] [U14] [U15] Write `crates/micold-core/tests/github_issue_lines.rs` (NEW; contracts/issue-fields.md §3–4, data-model §3–4):
  - `title_line()` is `#<number> <title>`; `details_line()` is `<reporter>`, then `"  ·  "` + `labels.join(", ")` "only when `labels` is not empty" (the three rows of the contract's table, including `ghost`).
  - `row_text()` is unchanged from today: `#N title` + (`"  ·  "` + labels); it does not contain the reporter in this slice (slice A only; T024 replaces this case when the reporter joins the match text).
  - `emphasis(spans)`: a span in the title part maps to the same range of the title line; a span in the labels part maps to the details line after the reporter and its 6-byte separator; a span crossing the separator is split and "the separator's bytes carry no emphasis"; a span outside every part is dropped; an issue without labels yields no details emphasis.
  - Ranges returned are "sorted, non-overlapping, and lie on character boundaries" for a title and labels with multi-byte characters.
- [X] T003 [P] [US1] [U28] [U29] Write `crates/micold-core/tests/github_privacy.rs` (NEW; contracts/issue-fields.md §5, FR-025):
  - `format!("{:?}", issue)` contains the number and the title and does not contain the reporter's login.
  - A source check over `crates/micold-core/src/github.rs`, `crates/micold-client/src/shell/issues.rs` and `crates/micold-client/src/features/worktree_form.rs`, and also `crates/micold-client/src/main.rs` and `crates/micold-client/src/ui/`: no `tracing`/`log` macro call names an issue, a reporter or a description.
- [X] T004 [P] [US1] [A7] [U31] [U32] [U33] [U34] [U35] Add unit tests to `crates/micold-client/src/ui/material/picker.rs` (contracts/picker-row.md §1–2):
  - The wrapping label, at a width narrower than its text, lays out more than one line high and at most the bound wide; a 256-character title without spaces stays inside the bound; emphasised and plain runs concatenated equal the input.
  - A row with details is at least `density::MENU_ITEM_BASE` high, and higher when either line wraps.
  - A row without details has today's fixed `MENU_ITEM_BASE` height and today's single-line label (FR-029).
  - A picked row with details lays its marker out beside the first line, whatever the row's height (US1 scenario 7).
- [X] T005 [P] [US1] [A1] [A2] [A3] [A8] [U36] Write `crates/micold-client/tests/issue_picker_rows.rs` (NEW; contracts/picker-row.md §4): the rows the issue picker's view builds for a listed issue, an issue from the search beyond the cap and an issue from a typed number carry `title_line()` as label and `details_line()` as details, with the emphasis `Issue::emphasis` gives for the match's spans (FR-006).
- [X] T006 [US1] [A1] [A2] [A4] [A5] [A9] [U44] Add issue-list states to `crates/micold-client/tests/support/covered_states.rs` (contracts/picker-row.md §6) with the list open (`form.issue_list_open = true`, so the rows are recorded in the overlay layer): a 256-character title, an issue with 20 labels, an issue without labels and rows of one to five lines, each state small enough that its rows are inside the list's eight-row viewport. Write `crates/micold-client/tests/gates/issue_rows_show_all_text.rs` (NEW, registered as a `#[path]` module in `crates/micold-client/tests/layout_snapshot.rs` as the other gates are): it reads each state's text with `support::layout::painted_text_settled` (a pass that draws the overlay; `painted_text` does not) at the canonical `WINDOW`, and again at a narrow window whose size the gate declares, through a size-taking variant of that pass added to `crates/micold-client/tests/support/layout.rs` (the existing function calls it with `WINDOW`; a covered state carries no window size), and asserts in both for every row that the row is wholly inside the list's viewport, that its text records, joined, hold the whole title, the reporter and every label, and that every record lies inside its row (SC-001, FR-004). `gates/containment.rs` runs over the new states unedited.

### Implementation for User Story 1, slice A

- [X] T007 [US1] [U1] [U2] [U3] [U4] [U5] [U6] In `crates/micold-core/src/github.rs`: add `Issue::reporter` ("Never empty"; `GHOST_LOGIN` = `"ghost"` "when the node's `author` is `null` or absent"), the chainable `.reported_by(login)`, and the accessor `reporter()`; `Issue::new` keeps its four arguments and yields `reporter = "ghost"`. Move the node fields of the three queries into one shared selection, add `author { login }` to it, and read it in `issue_from_node` (T001).
- [X] T008 [US1] [U7] [U8] [U9] [U10] [U11] [U12] [U13] [U14] [U15] In `crates/micold-core/src/github.rs`: add `title_line()`, `details_line()`, `RowEmphasis { title, details }` and `Issue::emphasis(&[Range<usize>])` with slice A's mapping (title part → title line, labels part → details line); `row_text()` stays as today (T002).
- [X] T009 [US1] [U28] [U29] In `crates/micold-core/src/github.rs`: replace the derived `Debug` of `Issue` with a hand-written one that prints `reporter` as `<redacted>` (T003).
- [X] T010 [US1] [U31] [U32] [U33] In `crates/micold-client/src/ui/material/picker.rs`: add the wrapping mode of `EmphasisedLabel`, shaped as one paragraph with `Paragraph::with_spans` and `Wrapping::WordOrGlyph`, reporting the paragraph's height (T004; research R5).
- [X] T011 [US1] [U34] [U35] In `crates/micold-client/src/ui/material/picker.rs` and `crates/micold-client/src/ui/material/typeahead.rs`: add `Row::details(text, spans)`; a row with details renders the label at `TypeRole::Body` in `on_surface` over the details at `TypeRole::Caption` in `on_surface_variant`, height `Shrink` with `density::MENU_ITEM_BASE` as the minimum and `spacing::XS` vertical padding, the picked-row marker aligned to the first line; a row without details takes today's code path (T004; contracts/picker-row.md §1).
- [X] T012 [US1] [U36] [U37] [U44] In `crates/micold-client/src/ui/worktree_form.rs`: build the issue picker's rows in one function (`issue_rows`) used for every issue, as `TypeaheadRow::new(issue.title_line(), e.title).details(issue.details_line(), e.details)` with `e = issue.emphasis(&matched.spans)` (T005).
- [x] T013 [US1] Pose the two-line rows in the showcase's `Typeahead` entry: `crates/micold-client/src/showcase/sections/controls.rs` (and `showcase/samples.rs`, `showcase/catalogue.rs` as the entry needs): a short row, a row with a long title and a row with many labels, one highlighted and one picked; `showcase_completeness.rs` and `showcase_captions.rs` stay green (FR-028).
- [X] T014 [US1] [U44] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`UPDATE_LAYOUT_SNAPSHOT=1`, docs/development/layout-snapshot.md) and confirm in the diff that the records of the branch picker, `Select` and 034's loading, empty and failure states did not change (FR-027, FR-029); `material/picker_parity.rs` and `material/menu_anatomy.rs` pass unedited.
- [X] T015 [P] [US1] Update `docs/user-guide/worktrees-and-sessions.md` § "From a GitHub issue": each issue takes two lines, the number and title, then who reported it and its labels; long text wraps; `ghost` for a deleted account (FR-031).
- [X] T016 [P] [US1] Update `docs/development/component-library.md` § "Pickers" (`Row::details`, the wrapping label, and that a row without details is unchanged) and `docs/development/component-showcase.md` (the two-line `Typeahead` pose).
- [x] T017 [US1] [A1] [A2] [A3] [A4] [A5] [A7] [A8] [A9] Run `mise run gate`; run quickstart §B1 and §B2 (B2 ends in the showcase, `mise run showcase`) with the `visual-pass` skill in the light and dark themes and save the screenshots under `specs/038-issue-list-reporter-tooltip/evidence/`.

**Checkpoint**: US1 scenarios 1–5 and 7–9 work; `mise run gate` passes.

---

## Phase 4: User Story 1, slice B — the highlighted row stays in view (Priority: P1)

**Goal**: With rows of differing height, Up and Down keep the highlighted row wholly visible.

**Independent Test**: quickstart §B3; automated: `picker_highlight_into_view.rs`.

### Tests for User Story 1, slice B (MANDATORY — Constitution Principle I) ⚠️

- [X] T018 [US1] [A6] [U39] [U40] [U41] [U42] [U43] Write `crates/micold-client/tests/picker_highlight_into_view.rs` (NEW; contracts/picker-row.md §3, modelled on `tests/focus_scroll.rs`):
  - A list of 12 issues with rows one to five lines tall, in a list eight base rows high: after each of 11 Down presses and 11 Up presses, with the operation applied, the highlighted row's bounds lie inside the scrollable's viewport (SC-007).
  - A row already wholly visible causes no scroll.
  - Each press moves the highlight by exactly one issue, and Enter picks the highlighted issue (FR-007).
  - The shell's arm for `FormMsg::IssueHighlightMoved` chains `picker_highlight_into_view()` (a source check over `crates/micold-client/src/main.rs` and `crates/micold-client/src/shell/issues.rs`), and no other picker's message does (FR-029).
  - A highlighted row taller than the viewport is aligned to the viewport's top (FR-007).

### Implementation for User Story 1, slice B

- [X] T019 [US1] [U39] [U40] In `crates/micold-client/src/ui/material/picker.rs`: give the highlighted row, and only it, the widget `Id` `PICKER_HIGHLIGHT` in `menu_element`.
- [X] T020 [US1] [U39] [U40] [U41] [U42] Add `crates/micold-client/src/ui/picker_scroll.rs` (NEW) with `picker_highlight_into_view<M>() -> Task<M>`: two passes as `focus::into_view`, reusing `focus::delta_into_view` (in `crates/micold-client/src/ui/focus.rs`: make it `pub(super)` and give it a `margin` parameter, since it pads the target by `MARGIN` (16) today; the focus caller and the unit tests in `focus.rs` pass `MARGIN` and keep their expectations, the picker passes `0.0`, so a wholly visible row does not scroll and a too-tall row lands on the viewport's top); "a row taller than the viewport is aligned to its top". Export it from `crates/micold-client/src/ui/mod.rs` (T018).
- [X] T021 [US1] [U39] [U40] [U43] In `crates/micold-client/src/main.rs` and `crates/micold-client/src/shell/issues.rs`: chain `picker_highlight_into_view()` after `FormMsg::IssueHighlightMoved`; the reducer is not changed (T018).
- [X] T022 [P] [US1] Update `docs/user-guide/worktrees-and-sessions.md` § "From a GitHub issue" (the list follows the highlight when moving with Up and Down) and `docs/development/component-library.md` § "Pickers" (the operation, and that only the issue picker chains it).
- [X] T023 [US1] [A6] Run `mise run gate`; run quickstart §B3 with the `visual-pass` skill and save the screenshots under `specs/038-issue-list-reporter-tooltip/evidence/`.

**Checkpoint**: US1 complete (scenarios 1–9).

---

## Phase 5: User Story 2 — find issues by who reported them (Priority: P2)

**Goal**: Typing a login narrows the list to that reporter's issues, with the match emphasised in
the reporter; the hint says so. No request is added or changed.

**Independent Test**: quickstart §B4; automated: `github_issue_lines.rs`, the reporter cases of
`issue_source_state.rs`, `issue_picker_rows.rs`, `typeahead_budget.rs`.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T024 [P] [US2] [A10] [A12] [A13] [U13] [U16] [U17] [U18] [U19] [U20] Extend `crates/micold-core/tests/github_issue_lines.rs` (contracts/issue-fields.md §4):
  - `row_text()` is `#N title` + `"  ·  "` + reporter + (`"  ·  "` + labels); T002's "unchanged from today" case is replaced by this one.
  - The five rows of the contract's emphasis table for issue #7 (`fix`, `ana`, `ui`, a span crossing title, separator and reporter, a span over a separator only).
  - `typeahead::rank` over issues matches a reporter by a part of the login and in a different letter case, and emphasises both a title match and a reporter match in one row (US2 scenarios 1–4).
  - An issue whose description alone contains the typed text is not matched (FR-014).
- [X] T025 [P] [US2] [A10] [A15] [U45] [U46] [U47] [U48] Extend `crates/micold-client/tests/issue_source_state.rs`:
  - Typing a login narrows a loaded list to that reporter's issues plus other matches (FR-009).
  - A searched issue that matches only by its reporter is kept; one that matches by nothing is dropped (FR-012).
  - Typing a login causes the same search request as any other text: one search with the typed text and no author filter (FR-013, US2 scenario 6).
  - 034's pick, loading, empty, failure and retry cases pass unedited (FR-008, FR-027).
- [X] T026 [P] [US2] [A11] [A12] [A14] [U37] [U38] Extend `crates/micold-client/tests/issue_picker_rows.rs`: the placeholder of the issue search field is `"Search by number, title, label or reporter"` (FR-011); a row matched by its reporter carries the emphasis in its details at the reporter's range (FR-010).
- [X] T027 [P] [US2] [U21] Extend `crates/micold-core/tests/typeahead_budget.rs`: the 1,000 issue rows carry reporters, under the existing 50 ms release budget (SC-002).

### Implementation for User Story 2

- [X] T028 [US2] [U13] [U16] [U17] [U18] [U19] [U20] [U21] [U37] [U45] [U46] [U47] [U48] In `crates/micold-core/src/github.rs`: add the reporter to the match text built for `row_text()` and map the reporter part to the start of the details line in `Issue::emphasis` (T024, T025, T027).
- [X] T029 [US2] [U38] In `crates/micold-client/src/ui/worktree_form.rs`: change the issue picker's placeholder to `"Search by number, title, label or reporter"` (T026).
- [X] T030 [P] [US2] Update `docs/user-guide/worktrees-and-sessions.md` § "From a GitHub issue" and § "Searching beyond the 1,000 loaded issues": the search also matches the reporter's login; beyond the loaded issues a reporter is found only when GitHub's search for the typed text returns the issue, as for a label (FR-031, FR-013).
- [X] T031 [US2] [A10] [A11] [A12] [A13] [A14] [A15] Run `mise run gate` and `scripts/build-lock.sh cargo test --release -p micold-core --test typeahead_budget`; run quickstart §B4 with the `visual-pass` skill and save the screenshots under `specs/038-issue-list-reporter-tooltip/evidence/`.

**Checkpoint**: US1 and US2 work; `mise run gate` passes.

---

## Phase 6: User Story 3, slice A — the rest-delay tooltip in the component library (Priority: P3)

**Goal**: The shared tooltip can open only after the cursor has rested, wake once at its deadline,
limit its text to a number of lines, and reset when its subject changes. The showcase shows it. No
screen of the app uses it yet; slice B puts it on issue rows.

**Independent Test**: quickstart §B5; automated: `tooltip_rest.rs`, `tooltip_clamp.rs`,
`tooltip_rest_glue.rs`, `idle_requests_no_frames.rs`.

### Tests for User Story 3, slice A (MANDATORY — Constitution Principle I) ⚠️

- [ ] T032 [P] [US3] [A16] [A17] [A18] [A19] [A20] [U49] [U50] [U51] [U52] [U53] [U54] [U55] [U56] [U57] [U58] Write `crates/micold-core/tests/tooltip_rest.rs` (NEW; contracts/rest-tooltip.md §1, data-model §5): the nine behaviours of the contract's table, among them:
  - Still for `delay` opens; at `delay − 1 ms` it is not open (FR-015).
  - A move of more than `REST_TOLERANCE` (4.0) restarts the wait; "a distance of exactly 4.0 is within tolerance"; "the anchor does not drift".
  - Moving 10 px every 100 ms for 10 s never opens (SC-004).
  - Leaving closes; `press()` closes until the cursor has left; `reset()` closes and waits the full delay again (FR-017).
  - `wake_at` is `Some(since + delay)` "only in `Waiting`" (FR-018).
- [ ] T033 [P] [US3] [A23] [A24] [U59] [U60] [U61] [U62] Write `crates/micold-core/tests/tooltip_clamp.rs` (NEW; contracts/rest-tooltip.md §5, data-model §6): the six rows of the contract's table with the fake measure, among them a text that fits is returned borrowed and unchanged, a cut ends in one `…` after a whole word, and `lines_of(output) <= max_lines` for generated lengths (FR-021, SC-005).
- [ ] T034 [P] [US3] [U63] [U64] Extend `crates/micold-client/tests/idle_requests_no_frames.rs` (contracts/rest-tooltip.md §3):
  - Source half: frame requests in `src/ui/` exist only in `cdk/motion.rs`, exactly two (`request_redraw` behind `animating()`, `request_redraw_at` in `wake_at`); `wake_at` is called outside `motion.rs` only from `cdk/tooltip.rs`.
  - Behaviour half: a rest-mode tooltip with a cursor at rest asks for exactly one timed wake, at `since + delay`, and none once open; away, spent and a tooltip without `after_rest` ask for none (FR-018).
- [ ] T035 [P] [US3] [A16] [A17] [A18] [A19] [A23] [A24] [A25] [U65] [U66] [U67] [U68] [U69] [U70] [U78] [U79] [U80] Write `crates/micold-client/tests/tooltip_rest_glue.rs` (NEW; contracts/rest-tooltip.md §2, §4, §6):
  - Driven with cursor and redraw events, a rest-mode tooltip shows its panel only after the delay at rest and not while the cursor moves.
  - A press over the trigger closes it and the trigger still receives the press.
  - A changed `subject` closes an open tooltip and starts the wait again.
  - With `max_lines(3)` and a long text, the panel is at most three `Caption` lines plus its padding tall and its text ends in `…`; a short text is shown whole.
  - A tooltip without `after_rest` opens at once, as today.
  - `crates/micold-client/tests/material_builder_api.rs` lists `after_rest`, `max_lines` and `subject` as chainable builder methods of `material::Tooltip`.
  - A redraw with the cursor still but the trigger's bounds no longer under it (the list scrolled or narrowed) closes an open panel; a trigger with another `subject` arriving under the still cursor waits the full delay again (FR-017).
  - With no cursor (`Cursor::Unavailable`) no panel opens, and the trigger still takes keyboard input.
  - Two widget trees built from the same view keep separate rest state: resting in one opens nothing in the other (Principle II).

### Implementation for User Story 3, slice A

- [ ] T036 [US3] [U49] [U50] [U51] [U52] [U53] [U54] [U55] [U56] [U57] [U58] Add `crates/micold-core/src/tooltip.rs` (NEW) and `pub mod tooltip` in `crates/micold-core/src/lib.rs`: `RestTimer` (`Away`, `Waiting { anchor, since }`, `Open`, `Spent`), `Rest`, `REST_TOLERANCE = 4.0`, `observe`, `press`, `reset` (T032).
- [ ] T037 [US3] [U59] [U60] [U61] [U62] In `crates/micold-core/src/tooltip.rs`: add `clamp_to_lines(text, max_lines, lines_of)` (T033).
- [ ] T038 [US3] [U63] [U64] In `crates/micold-client/src/ui/cdk/motion.rs`: add `wake_at(shell, Instant)`, the one `request_redraw_at` (T034).
- [ ] T039 [US3] [U64] [U65] [U66] [U67] [U69] In `crates/micold-client/src/ui/cdk/tooltip.rs`: add the rest mode — `after_rest(Duration)`, `subject(u64)`, a `RestTimer` in the widget state fed by cursor, redraw and press events, the existing `shown` track aimed on every open and close, `motion::wake_at` while waiting; without `after_rest` the widget behaves as today (T034, T035).
- [ ] T040 [US3] [U68] [U70] Add `crates/micold-client/src/ui/material/line_clamp.rs` (NEW, modelled on `ui/material/ellipsized.rs`) and, in `crates/micold-client/src/ui/material/mod.rs`, `Tooltip::after_rest`, `Tooltip::max_lines` and `Tooltip::subject`; `max_lines` measures a `Caption` paragraph at `TOOLTIP_MAX_WIDTH` less the panel's padding and calls `clamp_to_lines` (T035).
- [ ] T041 [US3] Pose it in the showcase's `Tooltip` entry, `crates/micold-client/src/showcase/sections/floating.rs`: an instance with `after_rest(3 s)` and `max_lines(3)` over a long text, captioned to hold the cursor still (FR-028).
- [ ] T042 [P] [US3] Update `docs/development/component-library.md` (the tooltip's rest mode, line limit and subject, and the second frame door in `cdk::motion` with the gate that holds it) and `docs/development/component-showcase.md` (the rest-delay `Tooltip` instance).
- [ ] T043 [US3] Run `mise run gate`; run quickstart §B5 with the `visual-pass` skill and save the screenshots under `specs/038-issue-list-reporter-tooltip/evidence/`.

**Checkpoint**: the showcase shows a rest-delay tooltip; every existing tooltip behaves as before.

---

## Phase 7: User Story 3, slice B — an issue's description on its row (Priority: P3)

**Goal**: Resting the cursor on an issue row for 3 seconds opens a tooltip with the start of the
description as plain text, at most three lines; it closes on another row, on leaving, on a pick and
when the list changes under the cursor. No request is made for it.

**Independent Test**: quickstart §B6–B11; automated: `github_description.rs`,
`gates/picker_row_tooltip_clears_its_row.rs`, `issue_picker_rows.rs`.

### Tests for User Story 3, slice B (MANDATORY — Constitution Principle I) ⚠️

- [ ] T044 [P] [US3] [A22] [A27] [A28] [U6] [U20] [U22] [U23] [U24] [U25] [U26] [U27] Write `crates/micold-core/tests/github_description.rs` (NEW) and add two captured nodes to `crates/micold-core/tests/fixtures/gh/` (contracts/issue-fields.md §2, data-model §2):
  - `description_from`: "every run of Unicode whitespace becomes one space; leading and trailing space is removed"; more than `DESCRIPTION_MAX_CHARS` (600) characters is "cut at a character boundary to 600, trailing space is removed, and `…` is appended"; "an input with no non-whitespace character gives `""`".
  - The node whose body is US3 scenario 12's parses to `description() == "Problem The list cuts long titles off."`; the node whose body is only an HTML comment parses to `""` (scenario 13).
  - `bodyText` that is `null` or absent gives `""` and the issue is still listed; a 65,536-character `bodyText` gives 601 characters.
  - The shared node selection contains `bodyText`; the request arguments are unchanged (FR-024, FR-026).
  - `row_text()` does not contain the description (FR-014).
- [ ] T045 [P] [US3] [U28] [U30] [U81] Extend `crates/micold-core/tests/github_privacy.rs`: `{:?}` of an issue does not contain its description; `parse_list_page` on a malformed page that contains a body returns an error whose `Display` and `Debug` contain no part of the body (FR-025). `Issue` derives or implements no `Serialize` (a source check over `crates/micold-core/src/github.rs`) (FR-025).
- [ ] T046 [P] [US3] [A21] [A22] [A26] [A28] [U71] [U72] [U73] [U74] [U82] [U83] Extend `crates/micold-client/tests/issue_picker_rows.rs` and the unit tests in `crates/micold-client/src/ui/material/picker.rs` (contracts/picker-row.md §4):
  - A row built for an issue with a description carries that description as its tooltip text and the issue number as its key; an issue with an empty description gets no tooltip (FR-020); listed, searched and typed-number issues alike (US3 scenario 11).
  - The tooltip's text is exactly the text passed to `Row::tooltip` (FR-019).
  - `menu_element` wraps a row with a tooltip in `Tooltip` with `ROW_TOOLTIP_REST` (3 s), `ROW_TOOLTIP_LINES` (3) and the key as subject; a row without one is not wrapped.
  - The view of a form on another source, and of a form whose list is `IssueList::Loading` after a newer load started, builds no row and so no row tooltip: a wait in progress is dropped with its row.
  - A highlighted row that the cursor is not over shows no panel: the highlight passes nothing to the row's tooltip.
- [ ] T047 [P] [US3] [A16] [A23] [A25] [U75] [U76] [U84] [U85] Write `crates/micold-client/tests/gates/picker_row_tooltip_clears_its_row.rs` (NEW, beside `tooltip_clears_its_row.rs` and registered like it in `crates/micold-client/tests/layout_snapshot.rs`; contracts/picker-row.md §5): with a hover held past the rest delay on the first row, the last row and a row at the list's lower edge, exactly one panel opens; it lies inside the window, does not intersect its row, and is at most three `Caption` lines plus the panel's padding tall; a click on the row under an open panel picks the issue (FR-021, FR-023, US3 scenario 10). With a panel open on one row, moving the cursor onto the adjacent row by less than `REST_TOLERANCE` closes it, and the adjacent row's panel opens only after the full delay (FR-016). With a panel open, the search field keeps keyboard focus and Up, Down and Enter move the highlight and pick as without it (FR-023).
- [ ] T048 [P] [US3] [U77] Extend `crates/micold-client/tests/issues_are_requested_only_on_named_events.rs`: no code under `crates/micold-client/src/ui/` calls the issue source, so a resting cursor cannot cause a request (FR-024, SC-006). It passes today and must keep passing: a characterization test, no red phase.

### Implementation for User Story 3, slice B

- [ ] T049 [US3] [U6] [U20] [U22] [U23] [U24] [U25] [U26] [U27] [U28] [U30] In `crates/micold-core/src/github.rs`: add `DESCRIPTION_MAX_CHARS = 600`, `description_from`, `Issue::description`, the chainable `.described(body_text)` and the accessor `description()`; add `bodyText` to the shared node selection and read it in `issue_from_node`; print `description` as `<redacted>` in `Debug` (T044, T045).
- [ ] T050 [US3] [U75] [U76] In `crates/micold-client/src/ui/cdk/picker.rs`: implement `Overlay::overlay` for `Menu`, forwarding to its content's `Widget::overlay` (T047; research R10).
- [ ] T051 [US3] [U71] [U73] [U74] [U75] [U76] In `crates/micold-client/src/ui/material/picker.rs` and `crates/micold-client/src/ui/material/typeahead.rs`: add `Row::tooltip(text)`, `Row::key(u64)`, `ROW_TOOLTIP_REST`, `ROW_TOOLTIP_LINES`, and wrap a row that has a non-empty tooltip text in `material::Tooltip` with `after_rest`, `max_lines`, `subject` and `TooltipPosition::Bottom` (T046, T047).
- [ ] T052 [US3] [U71] [U72] In `crates/micold-client/src/ui/worktree_form.rs`: `issue_rows` passes `.key(issue.number)` and, when the description is not empty, `.tooltip(issue.description())` (T046).
- [ ] T053 [US3] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` if the wrapped rows changed its records, and confirm the branch picker, `Select` and 034's states did not change.
- [ ] T054 [P] [US3] Update `docs/user-guide/worktrees-and-sessions.md` § "From a GitHub issue": resting the cursor on an issue for 3 seconds shows the start of its description, at most three lines, as plain text; it needs a pointer; an issue without a description shows nothing; the description is not searched (FR-031). Update `docs/development/component-library.md` § "Pickers": `Row::tooltip`, `Row::key`.
- [ ] T055 [US3] [A16] [A17] [A18] [A19] [A20] [A21] [A22] [A23] [A24] [A25] [A26] [A27] [A28] Run `mise run gate`; run quickstart §B6–B9 and §B11 with the `visual-pass` skill and save the results and screenshots under `specs/038-issue-list-reporter-tooltip/evidence/`. Also run §B10 (load time: five runs before on `main` and five after, alternating) and stop and escalate above 1.5× (research R14): M5 merges `bodyText`, the only change that can slow loading.

**Checkpoint**: All three stories work; `mise run gate` passes.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T056 Repeat quickstart §B10 on the merged result, five runs each, alternating: "before" is the commit on `main` just before M1's merge (record its SHA), "after" is `main` after M5; and record the medians and the ratio in `specs/038-issue-list-reporter-tooltip/evidence/README.md`; above 1.5×, stop and escalate (research R14).
- [ ] T057 Run quickstart §A (`mise run test-core`, `mise run gate`, the release rank budget) and §B1–B9 and §B11 (light and dark themes; §B10 is T056) with the `visual-pass` skill on the merged result, and record each step's outcome in `specs/038-issue-list-reporter-tooltip/evidence/README.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **US1 slice A (Phase 3)** has no prerequisite.
- **US1 slice B (Phase 4)** depends on slice A (rows of differing height).
- **US2 (Phase 5)** depends on slice A (`details_line`, `emphasis`); not on slice B.
- **US3 slice A (Phase 6)** has no prerequisite in this feature; it is ordered after US2 by priority.
- **US3 slice B (Phase 7)** depends on US3 slice A (the tooltip modes) and US1 slice A (`issue_rows`, the shared node selection).
- **Polish (Phase 8)** depends on all stories.

### Within Each Phase

- Tests first, observed failing; then implementation in the listed order.
- Core (`micold-core`) before the component library, the component library before the form's view.
- The user-guide task lands in the same phase as the behaviour it describes.

### Parallel Opportunities

- Slice A of US1: tests T001–T005 touch different files; T015 and T016 run beside the code tasks.
- US2: tests T024–T027 touch different files.
- US3 slice A: tests T032–T035; T036 and T038 touch different files.
- US3 slice B: tests T044–T048.

## Parallel Example: User Story 1, slice A

```text
T001 github_parse.rs   T002 github_issue_lines.rs   T003 github_privacy.rs
T004 material/picker.rs (unit tests)   T005 issue_picker_rows.rs
```

## Implementation Strategy

MVP first: US1 slice A is the core of the request (two lines, the reporter, nothing cut off) and
merges alone. Slice B completes US1 for long lists. US2 makes the new text searchable. US3 lands in
two steps, the component and then its use, so the timing rule and the frame gate are reviewed
without the issue list in the diff. Each step merges on its own with `mise run gate` green.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Two wrapping lines with the reporter 🎯 MVP

- **Tasks**: T001–T017
- **Deliverable**: On `main`, each row of the create-worktree form's issue list shows `#<number>
  <title>` and, below it in smaller, lower-emphasis text, the reporter's login and the labels; long
  titles and many labels wrap instead of being cut; the showcase's `Typeahead` entry shows rows of
  differing height; the user guide describes the two lines.
- **Satisfies**: US1 acceptance scenarios 1–5, 7–9; FR-001–FR-006, FR-008, FR-025 (reporter),
  FR-026, FR-027, FR-028 (row), FR-029, FR-030, FR-031 (two lines); SC-001
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test issue_picker_rows`; `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot issue_rows_show_all_text`; `mise run test-core`; quickstart §B1, B2 (the client, then `mise run showcase` for the `Typeahead` pose)
- **Depends on**: —
- **Tier**: full

### M2 — The highlighted row stays in view

- **Tasks**: T018–T023
- **Deliverable**: On `main`, moving the highlight with Up and Down through the issue list scrolls
  the list so the highlighted row is always wholly visible, whatever its height.
- **Satisfies**: US1 acceptance scenario 6; FR-007; FR-031 (the list follows the highlight); SC-007
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test picker_highlight_into_view`; quickstart §B3
- **Depends on**: M1
- **Tier**: full

### M3 — Search by reporter

- **Tasks**: T024–T031
- **Deliverable**: On `main`, typing a login (any letter case, or part of it) in the issue search
  field narrows the list to that reporter's issues with the match emphasised in the reporter; the
  hint reads "Search by number, title, label or reporter"; the user guide says so.
- **Satisfies**: US2 acceptance scenarios 1–6; FR-009–FR-014, FR-031 (reporter search); SC-002
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --test github_issue_lines`; `scripts/build-lock.sh cargo test -p micold-client --test issue_source_state --test issue_picker_rows`; quickstart §B4
- **Depends on**: M1
- **Tier**: full

### M4 — The rest-delay tooltip in the component library

- **Tasks**: T032–T043
- **Deliverable**: On `main`, the component showcase's `Tooltip` entry has an instance that opens
  only after the cursor has rested on it for 3 seconds, never while the cursor moves, and shows at
  most three lines ending in an ellipsis; every existing tooltip behaves as before. No screen of
  the app uses it yet (milestones.md rule 6); M5 puts it on issue rows. The PR carries the
  `docs-not-needed` label: nothing a reader of the user guide would notice changes.
- **Satisfies**: FR-015–FR-018 and FR-021 as component behaviour; FR-028 (tooltip); SC-004, SC-005 as rules
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test tooltip_rest_glue --test idle_requests_no_frames`; quickstart §B5 (`mise run showcase`)
- **Depends on**: —
- **Tier**: full

### M5 — An issue's description on its row

- **Tasks**: T044–T055
- **Deliverable**: On `main`, resting the cursor on an issue row for 3 seconds opens a tooltip with
  the start of the issue's description as plain text, at most three lines; it closes on another
  row, on leaving the list and on a pick; an issue without a description shows none; the user
  guide describes it.
- **Satisfies**: US3 acceptance scenarios 1–13; FR-015–FR-024, FR-025 (description), FR-026,
  FR-031 (tooltip); SC-003, SC-004, SC-005, SC-006, SC-008
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --test github_description`; `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot picker_row_tooltip_clears_its_row`; quickstart §B6–B11
- **Depends on**: M1, M4
- **Tier**: full

### M6 — Recorded pass and load-time measurement

- **Tasks**: T056–T057
- **Deliverable**: On `main`, `specs/038-issue-list-reporter-tooltip/evidence/README.md` records
  quickstart §B in full, with the issue list's load time before and after on a repository of more
  than 1,000 open issues.
- **Satisfies**: SC-003, SC-008; FR-030 (recorded on Linux; the other OSes by CI's core suite)
- **Verify**: read `specs/038-issue-list-reporter-tooltip/evidence/README.md` (every §B step has an outcome; the SC-008 ratio is at most 1.5); `mise run gate`
- **Depends on**: M1–M5
- **Tier**: light
