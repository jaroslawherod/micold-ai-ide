# Quickstart: Validating "Reporter, Labels and a Description Tooltip in the Issue List"

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Contracts**:
[issue-fields.md](./contracts/issue-fields.md), [picker-row.md](./contracts/picker-row.md),
[rest-tooltip.md](./contracts/rest-tooltip.md)

**§A** is the automated suite (what CI runs). **§B** is the recorded pass for what no test reaches:
render glue in `src/ui/`, real timing, the real `gh`. It runs headless with the repo's `visual-pass`
skill. Every rule with a decision in it is in §A.

## Prerequisites

```bash
mise trust                      # once per fresh worktree
gh --version && gh auth status  # §B only: a signed-in GitHub CLI
```

Projects for §B:

```bash
git clone --depth 1 https://github.com/cli/cli "$SCRATCH/issue-demo"      # > 1,000 open issues: the cap, SC-008
git clone --depth 1 https://github.com/jaroslawherod/micold-ai-ide "$SCRATCH/small"   # template bodies, few issues
```

`$SCRATCH` is any scratch directory outside the repository.

## §A — Automated

```bash
mise run test-core     # micold-core: all pure rules
mise run gate          # fmt, clippy, workspace tests, script tests
scripts/build-lock.sh cargo test --release -p micold-core --test typeahead_budget   # SC-002 budget
```

| Area | Test file | Covers |
|---|---|---|
| Parsing | `micold-core/tests/github_parse.rs` | `author`, `ghost`, `bodyText`, unchanged arguments (FR-002, 013, 026) |
| Description pass | `micold-core/tests/github_description_pass.rs`, `micold-client/tests/issue_source_state.rs` | Two-pass load: FR-024, FR-026, SC-008 |
| Display lines, emphasis, match text | `micold-core/tests/github_issue_lines.rs` | FR-001, 003, 009, 010, 014 |
| Description | `micold-core/tests/github_description.rs` | FR-020, 022; scenarios 12, 13 |
| Privacy | `micold-core/tests/github_privacy.rs` | FR-025 |
| Rest timer | `micold-core/tests/tooltip_rest.rs` | FR-015–017; SC-003, SC-004 as a rule |
| Line clamp | `micold-core/tests/tooltip_clamp.rs` | FR-021, SC-005 as a rule |
| Rank budget | `micold-core/tests/typeahead_budget.rs` (release) | SC-002 |
| Form reducer | `micold-client/tests/issue_source_state.rs` | FR-008, 009, 012, 014 |
| Frame requests | `micold-client/tests/idle_requests_no_frames.rs` | FR-018 |
| No request on hover | `micold-client/tests/issues_are_requested_only_on_named_events.rs` | FR-024, SC-006 |
| Highlight in view | `micold-client/tests/picker_highlight_into_view.rs` | FR-007, SC-007 |
| Tooltip glue | `micold-client/tests/tooltip_rest_glue.rs` | FR-015–017, 021 |
| Issue rows in the view | `micold-client/tests/issue_picker_rows.rs` | FR-006, 010, 011, 019, 020 |
| 034's states unchanged | `micold-client/tests/issue_source_state.rs` (existing cases), `layout_snapshot.rs` | FR-027 |
| Row tooltip geometry | `micold-client/tests/gates/picker_row_tooltip_clears_its_row.rs` | FR-021, 023; SC-005 |
| Wrapping, nothing clipped | `gates/issue_rows_show_all_text.rs` (text, read with the overlay drawn) and `gates/containment.rs` (layout records) over the new covered states with the list open | FR-004; SC-001 |
| Single-line rows unchanged | `material/picker_parity.rs`, `material/menu_anatomy.rs`, `layout_snapshot.rs` | FR-029 |
| Component API and showcase | `material_builder_api.rs`, `showcase_completeness.rs`, `typeahead_is_generic.rs` | FR-028 |

## §B — Recorded pass

Run with the `visual-pass` skill on a private display. Record each step's result and screenshot in
`specs/038-issue-list-reporter-tooltip/evidence/README.md`, screenshots beside it. Steps B1–B4 and B6–B11 use the client
(`mise run run`) with the project `$SCRATCH/issue-demo` unless stated; B5 and the end of B2 use the showcase
(`mise run showcase`).

| Step | Do | Expect | Covers |
|---|---|---|---|
| B1 | Open the create-worktree form, choose **GitHub issue**. Then type a term that returns issues from beyond the 1,000 loaded, and clear it. | Every row: `#<number> <title>` on the first line; below it the reporter's login, then ` · ` and the labels in smaller, lower-emphasis text. A row without labels shows the login alone. A searched issue's row has the same two lines. Check in the light and the dark theme. | US1 1–3, 8; FR-005 |
| B2 | Find rows with a long title and with many labels; narrow the window to its minimum width, then widen it. Click a row that wraps; reopen the list. Then `mise run showcase` → Typeahead: look at the two-line pose. | Titles and details wrap inside the row at every width; nothing is cut, ellipsized or outside the list. The click picks the issue: ticket, name and type are filled as before, and its row carries the picked marker beside its first line. The showcase pose shows rows of differing height, one highlighted, one picked. | US1 4, 5, 7, 9; FR-008, FR-028; SC-001 |
| B3 | Press Down 15 times, then Up 15 times; press Enter on a tall row; reopen the list. | Each press moves one issue; the highlighted row is always wholly visible; Enter picks it; its row carries the picked marker. | US1 6; FR-007; SC-007 |
| B4 | Read the empty search field's hint. Type a login seen in B1, then the same login in capitals. Then type a term that returns issues from beyond the 1,000 loaded. | The hint names the reporter. The list narrows to that reporter's issues (plus other matches); the login is emphasised in the details line; capitals match too. Searched issues have the same two lines, with the login emphasised. | US2 1–5 |
| B5 | Showcase → Tooltip: rest the cursor on the rest-delay instance; then keep the cursor moving over it for 10 s. | A tooltip of at most three lines ending in `…` opens after about 3 s at rest, and never while moving. | FR-028; SC-004 |
| B6 | Rest the cursor on a row with a description. Time it with a stopwatch from rest to open, 20 trials. | The tooltip opens between 3.0 and 3.5 s in every trial, never before 3.0 s. It shows only description text. | US3 1, 2, 6; SC-003 |
| B7 | Sweep the cursor over the list for 10 s. Then open a tooltip and move to the next row; then leave the list; then open one and click the row. | No tooltip during the sweep. Moving to another row closes it and the next opens only after 3 s; leaving closes it; the click picks the issue and closes it. | US3 3–5, 10; SC-004 |
| B8 | Rest on a row with a long description, a short one, and an issue with an empty body. Rest on the first row and on the last visible row. Type to narrow the list while a tooltip is open. | Long: three lines, ends in `…`. Short: whole, no `…`. Empty: no tooltip. The panel is inside the window and does not cover its row in either position. When a different row comes under the cursor the tooltip closes. | US3 7–9; FR-017, 023; SC-005 |
| B9 | Project `$SCRATCH/small`: rest on an issue created from a template (a heading, checkboxes, an HTML comment). | Words only, in one paragraph: no `#`, `*`, `[ ]`, brackets, addresses or comment text. | US3 12, 13; FR-022 |
| B10 | Load time. On `main` (before) and on the branch (after), with `$SCRATCH/issue-demo`: choose **GitHub issue** and time from the click to the list appearing, five runs each, alternating builds. | Median after ≤ 1.5 × median before; no run shows 034's timeout failure (each request answered within 10 s). Above 1.5×: stop and escalate (research R14). Also time, on the branch, from the list appearing to the first row's tooltip being available and note it: descriptions follow the list (FR-024), the first page within a few seconds. | SC-008 |
| B11 | Idle cost and requests. With the list open, read the client's CPU time (`utime + stime` in `/proc/<pid>/stat`) over 30 s three times: (a) cursor at rest beside the list, (b) cursor at rest on a row with a description, from before the tooltip opens until 30 s later, (c) the same on a row with no description. During (b) and B6–B8, run `pgrep -x gh` once a second. | (b) and (c) use no more CPU time than (a) plus 10%: waiting for and showing a tooltip adds no steady redraw (the search field's caret blink is in all three). Once the description pass has ended (no `gh` process for 5 s after the list appeared; up to about 20 s on 1,000 issues), `pgrep` finds no `gh` process while the cursor only rests. | FR-018; SC-006 |

**Not runnable on this host**: macOS and Windows. FR-030 rests on the absence of any `cfg` arm in
this feature and on `micold-core`'s suite running on all three OSes in CI; B1–B11 are recorded on
Linux.
