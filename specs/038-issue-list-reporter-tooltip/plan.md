# Implementation Plan: Reporter, Labels and a Description Tooltip in the Issue List

**Branch**: `feat/issue-list-reporter-labels-tooltip` | **Date**: 2026-10-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/038-issue-list-reporter-tooltip/spec.md`

## Summary

Each row of feature 034's issue list becomes two wrapping lines: `#<number> <title>`, then the
reporter's login and the labels. Typing a login narrows the list. Resting the cursor on a row for
3 seconds opens a tooltip with the start of the issue's description as plain text, at most three
lines tall.

The design turns on five decisions, all recorded in [research.md](./research.md):

1. **The new data rides the requests 034 already makes** ([R1](./research.md)). The three GraphQL
   queries gain `author { login }` and `bodyText`. No new request, occasion or argument, so FR-013,
   FR-024 and FR-026 hold by construction. `bodyText` is GitHub's own plain-text rendering of the
   body, so FR-022 needs no Markdown parser and **no new crate** ([R2](./research.md)); core only
   folds whitespace and caps the text at 600 characters ([R3](./research.md)).
2. **One match text, mapped onto two lines** ([R4](./research.md)). `typeahead::rank` still reads
   one string per issue; a pure mapping in core splits its match spans between the title line and
   the details line. The matcher, its case rule and its budget are untouched, so the reporter is
   matched "under the same rule" (FR-009) because it is the same call.
3. **The two-line row is a mode of the shared picker row** ([R5](./research.md)):
   `material::picker::Row::details`. A row without details takes today's code path, so the branch
   picker and `Select` are unchanged (FR-029).
4. **The rest delay is a mode of the shared tooltip, built here** ([R7](./research.md), ledger D5):
   `material::Tooltip::after_rest`. Its rule is a pure state machine in core
   (`micold_core::tooltip::RestTimer`); the widget wakes once at the deadline through a second,
   gated door in `cdk::motion` ([R8](./research.md)), so waiting never redraws continuously
   (FR-018). Spec 036 is not a dependency.
5. **Keeping the highlighted row in view is new work** ([R6](./research.md), ledger D7): an
   operation modelled on `ui/focus.rs`'s `scroll_focused_into_view`, chained by the shell after a
   highlight move in the issue picker.

Everything with a decision in it is render-free and tested first: the description fold and cap, the
display lines and the emphasis mapping, the rest timer, the three-line clamp. The widgets supply
measurements and events.

## Technical Context

**Language/Version**: Rust, edition 2021, workspace MSRV (unchanged)

**Primary Dependencies**: `iced` 0.14 (`iced_core` `Paragraph::with_spans`,
`Shell::request_redraw_at`, `Overlay::overlay`), `serde_json`; `micold-core` modules `github`,
`typeahead`, and a new `tooltip`. External runtime tool: the GitHub CLI `gh`, as in 034. **No new
crate.**

**Storage**: none. Reporters and descriptions live in `WorktreeForm` only (FR-025); `Issue` stays
without `Serialize`.

**Testing**: `mise run test-core` for the render-free rules; `mise run gate` for the workspace;
client reducer, source-gate and geometry-gate tests under `crates/micold-client/tests/`;
`quickstart.md` §B (the `visual-pass` skill) for render glue, the real `gh`, timing and SC-008.

**Target Platform**: Linux, macOS, Windows desktop. No OS-specific code: no `cfg` arm is added.

**Project Type**: desktop application (three-crate Cargo workspace).

**Performance Goals**: ranking 1,000 two-part match texts stays under the existing 50 ms release
budget (`typeahead_budget.rs`, SC-002); the tooltip opens 3.0–3.5 s after rest (SC-003); the issue
list loads in at most 1.5× today's time on 1,000 issues (SC-008, measured in §B).

**Constraints**: no request added or changed beyond two fields in the node selection (FR-013,
FR-024, FR-026); no frame request outside `cdk/motion.rs` (`idle_requests_no_frames.rs`); no
reporter or description in files or logs (FR-025); the single-line picker row byte-for-byte as today
(FR-029).

**Scale/Scope**: one new core module (`tooltip`), one extended (`github`); the shared picker row,
its list overlay and the shared tooltip each gain a mode; one new UI operation; the issue picker's
view and one shell route change; two showcase poses; one user-guide section and two development
pages. Six milestones.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Each rule is in `micold-core` with its tests
  written first: `description_from`, `Issue::title_line` / `details_line` / `emphasis`, the parsing
  of `author` and `bodyText`, `RestTimer`, `clamp_to_lines`. The client's logic-bearing glue is
  held by tests that fail before the change: the reducer (`issue_source_state.rs`), the frame gate
  (`idle_requests_no_frames.rs`), geometry gates for wrapping, the highlight in view and the
  tooltip against its row. The GUI exception is claimed only for `src/ui/` composition and pixels
  (quickstart §B). The test list is derived by `speckit.tdd.plan` into `tdd/test-list.md`.
- [x] **II. Multi-Session Support**: PASS. No session state. The list, highlight and tooltip state
  belong to one open form in one window: the timer is widget state in that window's tree, and the
  fields are in that form's `IssueList`. Nothing is shared between windows (spec, Edge Cases).
- [x] **III. Worktree Integration**: PASS. Nothing about creating a worktree changes (FR-008); a
  pick runs 034's path.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing is stored. No new contact with
  GitHub: the same requests, on the same occasions, after the same opt-in, with two more fields in
  the answer (FR-024, FR-026). A hover makes no request (SC-006).
- [x] **V. Rust + iced Stack**: PASS. Rust and iced only. Types narrow the states: `RestTimer` is an
  enum (`Away`, `Waiting`, `Open`, `Spent`), so "open while waiting" cannot be written;
  `RowEmphasis` separates the two lines' spans; `Issue` holds only the capped description and has a
  redacting `Debug`.
- [x] **VI. Cross-Platform Parity**: PASS. No `cfg` arm and no OS call. The rest tolerance is in
  logical pixels, the time source is `Instant`, the wake is iced's `request_redraw_at` on every
  backend. The core rules run on all three OSes in CI.
- [x] **VII. Documentation First-Class**: PASS. The user guide's "From a GitHub issue" section
  gains the two lines with US1, searching by reporter with US2 and the description tooltip with the
  milestone that ships it on issue rows (FR-031). `component-library.md` and
  `component-showcase.md` describe the two-line row with M1 and the tooltip's modes with M4, the
  milestones that add them.
- [x] **VIII. Reusable UI Component Foundation**: PASS. No new component: the two-line row is
  `material::picker::Row::details`, the tooltip is `material::Tooltip` with `after_rest`,
  `max_lines` and `subject` (FR-028). Appearance stays in `material/`, behaviour in `cdk/`
  (`material_boundary.rs`, `cdk_no_appearance.rs`). The showcase poses both.

### Re-check after Phase 1 design

All eight still **PASS**. Three things recorded rather than waved through:

- **A second frame-request door.** `idle_requests_no_frames.rs` allows one `request_redraw` in the
  rendering layer. This plan adds one `request_redraw_at` beside it, in the same file, behind
  `RestTimer::Waiting`, and extends the gate to hold exactly those two
  ([contracts/rest-tooltip.md §3](./contracts/rest-tooltip.md)). A redraw at one instant lets the
  runtime sleep until then; it is not a held-awake loop.
- **FR-007 is new work, not existing behaviour** (ledger D7). The operation is generic; only the
  issue picker chains it. The same defect in the branch picker and `Select` is a recorded
  follow-up, outside this spec (FR-029).
- **Shared-file overlap with spec 036.** Both features change `cdk/tooltip.rs`. They add separate
  builder methods and can coexist; the flow that merges second rebases (ledger D5).

No entry in Complexity Tracking.

## Requirement → design map

| Requirement | Where |
|---|---|
| FR-001 | `Issue::title_line()`; the row's label ([issue-fields §3](./contracts/issue-fields.md), [picker-row §1](./contracts/picker-row.md)) |
| FR-002 | `author { login }` in all three queries; `Issue::reporter`, `GHOST_LOGIN` ([issue-fields §1–2](./contracts/issue-fields.md)) |
| FR-003 | `Issue::details_line()`: reporter, then `"  ·  "` and the labels only when there are labels ([issue-fields §3](./contracts/issue-fields.md)) |
| FR-004 | `Row::details`; `EmphasisedLabel` wrapping mode, `Wrapping::WordOrGlyph`, `Shrink` height ([picker-row §1–2](./contracts/picker-row.md)) |
| FR-005 | Details at `TypeRole::Caption` in `on_surface_variant`; label at `TypeRole::Body` in `on_surface` ([picker-row §1](./contracts/picker-row.md)) |
| FR-006 | One `issue_from_node` for list, search and typed-number nodes; one row builder in `ui/worktree_form.rs` |
| FR-007 | `picker_highlight_into_view()` chained after `IssueHighlightMoved`; marker on the first line ([picker-row §3](./contracts/picker-row.md)) |
| FR-008 | `IssuePicked` reducer untouched; `issue_source_state.rs` pick tests stay green |
| FR-009 | Reporter added to `Issue::row_text` (the match text); `typeahead::rank` unchanged ([issue-fields §4](./contracts/issue-fields.md)) |
| FR-010 | `Issue::emphasis` maps reporter spans to the details line ([issue-fields §4](./contracts/issue-fields.md)) |
| FR-011 | Placeholder `"Search by number, title, label or reporter"` in `ui/worktree_form.rs` |
| FR-012 | Searched issues are ranked by the same `rank` call over the same match text ([data-model §4](./data-model.md)) |
| FR-013 | `search_args` and `list_args` unchanged; `github_parse.rs` argument tests extended to say so |
| FR-014 | The description is not part of `row_text` ([issue-fields §4](./contracts/issue-fields.md)) |
| FR-015, FR-016 | `micold_core::tooltip::RestTimer`, `REST_TOLERANCE`; `Tooltip::after_rest(ROW_TOOLTIP_REST)` ([rest-tooltip §1–2](./contracts/rest-tooltip.md)) |
| FR-017 | `RestTimer` transitions `Away` and `Spent`; `Tooltip::subject` ([rest-tooltip §1, §4](./contracts/rest-tooltip.md)) |
| FR-018 | `cdk::motion::wake_at`; `idle_requests_no_frames.rs` ([rest-tooltip §3](./contracts/rest-tooltip.md)) |
| FR-019 | `Row::tooltip(issue.description())`: the text is the description only ([picker-row §4](./contracts/picker-row.md)) |
| FR-020 | `description_from` trims; an empty description passes no tooltip ([issue-fields §2](./contracts/issue-fields.md)) |
| FR-021 | `clamp_to_lines`; `Tooltip::max_lines(ROW_TOOLTIP_LINES)` ([rest-tooltip §5](./contracts/rest-tooltip.md)) |
| FR-022 | `bodyText` + `description_from` ([issue-fields §1–2](./contracts/issue-fields.md)) |
| FR-023 | `TooltipPosition::Bottom` with the existing `place` flip; `Menu` forwards `Overlay::overlay`; the panel takes no input ([picker-row §5](./contracts/picker-row.md)) |
| FR-024 | Description held on `Issue`; no call to `IssueSource` from the view or the tooltip; `issues_are_requested_only_on_named_events.rs` |
| FR-025 | No `Serialize` on `Issue`; redacting `Debug`; no log macro over issues ([issue-fields §5](./contracts/issue-fields.md)) |
| FR-026 | Query text differs only in the node selection; arguments, page size, cap and timeout unchanged ([issue-fields §1](./contracts/issue-fields.md)) |
| FR-027 | `IssueList`, `SearchState` and their views untouched; existing covered states stay in the layout snapshot |
| FR-028 | `material::picker::Row::{details, tooltip, key}`, `material::Tooltip::{after_rest, max_lines, subject}`; showcase poses ([picker-row §6](./contracts/picker-row.md), [rest-tooltip §6](./contracts/rest-tooltip.md)) |
| FR-029 | A row without details is built by today's code; `picker_parity.rs`, `menu_anatomy.rs` and the branch picker's layout records unchanged |
| FR-030 | No `cfg` arm; core rules tested on all three OSes in CI |
| FR-031 | `docs/user-guide/worktrees-and-sessions.md` § "From a GitHub issue", one edit per shipping milestone |

## Test strategy by layer

| Layer | What it holds | Requirements |
|---|---|---|
| `micold-core` unit (`mise run test-core`) | `github_parse.rs`: `author` (login, `null` → `ghost`, bot), `bodyText`, same node shape in all three queries, unchanged arguments. The three queries share one node selection, and a list, a search and a typed-number node parse to the same `Issue` (FR-006). NEW `github_issue_lines.rs`: `title_line`, `details_line`, `emphasis` (split, clip, rebase, separators unemphasised), match text with and without the reporter, description absent from it. NEW `github_description.rs`: fold, trim, cap at 600 with `…`, empty results. NEW `github_privacy.rs`: `Debug` redacts; a malformed page's error carries no body. NEW `tooltip_rest.rs`: every row of the R7 table, tolerance edge, subject reset. NEW `tooltip_clamp.rs`: fits, cuts at word boundary, no doubled `…`, never above `max_lines` | FR-001–003, 006, 009, 010, 012–017, 020–022, 025, 026 |
| `micold-core` release budget | `typeahead_budget.rs`: 1,000 issue rows with the reporter in the match text < 50 ms | SC-002 |
| `micold-client` reducer | `issue_source_state.rs`: a login narrows the list; a searched issue matching only by reporter is kept; description-only text matches nothing; a pick is unchanged; 034's loading, empty, failure and retry cases stay green, unedited (FR-027) | FR-008, 009, 012, 014, 027 |
| `micold-client` source gates | `idle_requests_no_frames.rs` (two doors, both in `motion.rs`; a waiting tooltip asks for one timed wake at its deadline, others for no timed wake); `issues_are_requested_only_on_named_events.rs`; `material_builder_api.rs`; `material_boundary.rs`; `cdk_no_appearance.rs`; `typeahead_is_generic.rs`; `showcase_completeness.rs` | FR-018, 024, 028 |
| `micold-client` widget tests | `material/picker.rs` unit tests: a row with details is taller than `MENU_ITEM_BASE` when it wraps and never clips; a row without details matches today's. NEW `tests/picker_highlight_into_view.rs`: Down through rows of one to five lines keeps the highlighted row inside the list's viewport at every step. NEW `tests/tooltip_rest_glue.rs`: the widget feeds `RestTimer` cursor and redraw events; a press closes; a changed subject resets; the panel is at most three lines. NEW `tests/issue_picker_rows.rs`: the view's rows for a listed, a searched and a typed-number issue carry the same two lines (FR-006); the placeholder names the reporter (FR-011); a row gets a tooltip only for a non-empty description (FR-020), and the panel's text equals the text passed to `Row::tooltip` (FR-019) | FR-004, 006, 007, 011, 015–017, 019, 020; SC-004, SC-007 |
| Layout/geometry gates | `tests/support/covered_states.rs`: NEW states with the issue list **open** (`form.issue_list_open = true`, so the list is recorded in the overlay layer, as `worktree-menu-open` is; today's `add-worktree-dialog-issue-*` states leave it closed and record no row): a 256-character title, 20 labels, no labels and mixed heights, at the default window and a narrow one, each state small enough that the rows under test are inside the list's eight-row viewport. NEW `gates/issue_rows_show_all_text.rs` holds SC-001: it reads each state's text with an overlay-drawing pass (`support::layout::painted_text_settled`; `painted_text` and `layout_text_overflow.rs` run `Before::Mounted`, which draws no overlay, so they cannot see a row) and asserts, for every row, that the row is wholly inside the list's viewport (a state that outgrows the cap fails rather than passing on unpainted rows), that its text records, joined, hold the whole title, the reporter and every label, and that each record lies inside its row. `gates/containment.rs` holds the new states' overlay layout records (each row inside the list); they stay under the cap, so `PICKER_LIST_CONTENT` gains no entry. NEW `gates/picker_row_tooltip_clears_its_row.rs`: first row, last row, a row at the list's lower edge; the panel is inside the window, does not intersect its row and is at most three lines tall. Both new gates are `#[path]` modules of `tests/layout_snapshot.rs`, as every gate under `tests/gates/` is. `fixtures/layout_snapshot.txt` regenerated; its records for the branch picker, `Select` and 034's loading, empty and failure states do not change, and `material/picker_parity.rs` and `material/menu_anatomy.rs` stay green (FR-027, FR-029) | FR-004, 021, 023, 027, 029; SC-001, SC-005 |
| quickstart §B (visual-pass) | emphasis and hierarchy in both themes; the 3 s timing by stopwatch over 20 trials; a 10 s sweep; a real `gh` against a real repository; a template body's text; idle CPU with a tooltip waiting and open (FR-018); SC-008 before and after; the guide read against the app (FR-031) | FR-005, 018, 022, 030, 031; SC-003, 004, 006, 008 |
| CI's user-guide gate (`scripts/check-user-guide-updated.sh`) | each `feat` PR that ships user-facing behaviour edits the guide | FR-031 |

## Project Structure

### Documentation (this feature)

```text
specs/038-issue-list-reporter-tooltip/
├── plan.md              # This file
├── research.md          # Phase 0 — R1–R15
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1 — §A automated, §B recorded pass
├── contracts/
│   ├── issue-fields.md  # queries, parsing, description, display lines, match text, privacy
│   ├── picker-row.md    # Row::details/tooltip/key, wrapping, highlight in view, nested overlay
│   └── rest-tooltip.md  # RestTimer, wake_at, subject, clamp_to_lines, Tooltip builder methods
├── checklists/requirements.md
├── tasks.md             # Phase 2 (/speckit-tasks)
├── tdd/test-list.md     # /speckit-tdd-plan
└── evidence/README.md   # §B's record, screenshots beside it (written by the milestones)
```

### Source Code (repository root)

```text
crates/micold-core/
├── src/
│   ├── lib.rs               # + pub mod tooltip
│   ├── github.rs            # Issue: reporter, description, title_line, details_line, emphasis,
│   │                        #   RowEmphasis, GHOST_LOGIN, DESCRIPTION_MAX_CHARS, description_from,
│   │                        #   redacting Debug; queries + issue_from_node read author, bodyText
│   └── tooltip.rs           # NEW — RestTimer, REST_TOLERANCE, clamp_to_lines
└── tests/
    ├── github_parse.rs      # extended
    ├── github_issue_lines.rs  github_description.rs  github_privacy.rs   # NEW
    ├── tooltip_rest.rs  tooltip_clamp.rs                                 # NEW
    ├── typeahead_budget.rs  # issue rows carry a reporter
    └── fixtures/gh/         # + a captured node with author and bodyText

crates/micold-client/
├── src/
│   ├── ui/cdk/tooltip.rs        # rest mode: RestTimer in State, subject, press closes
│   ├── ui/cdk/motion.rs         # + wake_at
│   ├── ui/cdk/picker.rs         # Menu forwards Overlay::overlay
│   ├── ui/material/mod.rs       # Tooltip::after_rest, max_lines, subject
│   ├── ui/material/line_clamp.rs# NEW — the measuring label behind max_lines (as ellipsized.rs)
│   ├── ui/material/picker.rs    # Row::details, tooltip, key; wrapping EmphasisedLabel;
│   │                            #   ROW_TOOLTIP_REST, ROW_TOOLTIP_LINES; highlighted-row Id
│   ├── ui/material/typeahead.rs # passes the row modes through (TypeaheadRow)
│   ├── ui/picker_scroll.rs      # NEW — picker_highlight_into_view (beside focus.rs)
│   ├── ui/focus.rs              # delta_into_view becomes pub(super)
│   ├── ui/worktree_form.rs      # issue rows: two lines, emphasis, key, tooltip; placeholder
│   ├── shell/issues.rs, main.rs # chain the operation after IssueHighlightMoved
│   └── showcase/                # two-line rows of differing height; a rest-delay tooltip
└── tests/
    ├── issue_source_state.rs  idle_requests_no_frames.rs               # extended
    ├── picker_highlight_into_view.rs  tooltip_rest_glue.rs  issue_picker_rows.rs   # NEW
    ├── gates/issue_rows_show_all_text.rs  gates/picker_row_tooltip_clears_its_row.rs   # NEW
    ├── layout_snapshot.rs                                              # + a #[path] mod for each new gate
    ├── support/covered_states.rs                                       # + open issue lists, mixed heights
    └── fixtures/layout_snapshot.txt                                    # regenerated

docs/
├── user-guide/worktrees-and-sessions.md   # From a GitHub issue: two lines, reporter search, tooltip
├── development/component-library.md       # Row::details; Tooltip rest mode; the second frame door
└── development/component-showcase.md      # the new poses
```

**Structure Decision**: the existing three-crate workspace, unchanged. The daemon is not touched:
`gh` runs in the client (034 R4) and nothing new crosses the protocol, so `PROTOCOL_VERSION` and
`SCHEMA_HASH` stay.

## Delivery order

Sliced by story priority; each slice ships something observable (milestones in `tasks.md`).

| Slice | Milestone | Delivers | Gate |
|---|---|---|---|
| **1a (US1, P1)** | M1 | `author` read and held; the two display lines and the emphasis mapping (match text unchanged); `Row::details` and the wrapping label; the issue picker's rows; showcase pose; redacting `Debug`; guide: two lines; `component-library.md` and `component-showcase.md`: the two-line row | `mise run gate`; quickstart §B1, B2 |
| **1b (US1 AS6)** | M2 | The highlighted row kept wholly in view: `picker_highlight_into_view`, chained by the shell | §B3 |
| **2 (US2, P2)** | M3 | Reporter in the match text; reporter emphasis; the hint; guide: search by reporter | §B4 |
| **3a (US3, component)** | M4 | `RestTimer`, `clamp_to_lines`; `Tooltip::after_rest`, `max_lines`, `subject`; `wake_at` and the extended frame gate; showcase pose; `component-library.md` and `component-showcase.md`: the tooltip's modes | §B5 |
| **3b (US3, issue rows)** | M5 | `bodyText` read, `description_from`; `Menu` forwards overlays; `Row::tooltip`, `Row::key`; the issue picker passes both; the row-tooltip geometry gate; guide: the tooltip | §B6–B11 |
| **Polish** | M6 | §B recorded in full, SC-008 measured before and after | quickstart §B complete |

US1 is split along acceptance scenario 6: 1a ships the rows (scenarios 1–5 and 7–9), 1b the
scrolling that keeps a tall highlighted row in view. Until 1b merges, a highlight moved past the
list's visible rows is out of view, as it is on `main` today for a list of more than eight issues
(research R6); 1a makes it no worse per issue and ships no half-wired control.

US3 is split along its acceptance scenarios: 3a ships the component (observable in the showcase),
3b ships scenarios 1–13 on issue rows. 3a changes nothing a user of the app sees, so its PR carries
the `docs-not-needed` label; 3b carries the guide.

## Risks

| Risk | Handling |
|---|---|
| A page of 100 issues with very long bodies slows the list (SC-008) | Measured in §B on a 1,000-issue repository before (`main`) and after: measured in M5 before its PR, recorded again in M6. Above 1.5×: SC-008 conflicts with FR-024, and the flow escalates with the measurement; no fallback is decided in advance ([R14](./research.md)). |
| A tooltip inside the picker's overlay is dropped or misplaced by a wrapper | `Menu` forwards `overlay`; the geometry gate drives a real hover and fails on a missing or displaced panel ([R10](./research.md)). |
| The wrapping label changes the single-line row | Wrapping is a separate mode; a row without details keeps today's builder. `picker_parity.rs`, `menu_anatomy.rs` and the layout snapshot's branch-picker records must not change. |
| The timed wake becomes a redraw loop | `wake_at` is called only in `Waiting`, with one deadline; the gate's behavioural half asserts one request while waiting and none when open, away or spent. |
| 036 merges first and changes `cdk/tooltip.rs` | Separate builder methods; `branch-start.sh` rebases, and conflicts are in files this flow owns. |
| `bodyText` differs from the spec's example for some Markdown | Scenario 12's body is a captured fixture of a real node, and §B checks a template body on a real repository. A gap is a spec question, not a parser of ours. |
| The 3 s timing cannot be asserted by a unit test | The rule takes `now` as an argument and is tested exactly; §B times the real widget over 20 trials (SC-003). |

## Complexity Tracking

No constitutional violation requires justification. Left empty deliberately.
