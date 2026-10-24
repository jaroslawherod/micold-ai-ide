# Implementation Plan: Review UI — docs, layout snapshot, visual pass and mockup sync

**Branch**: `claude/project-thread-kxels1` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1276-review-ui-docs-sync/spec.md`

## Summary

Slice R7 closes the review UI redesign (epic #1277). It adds no behaviour. It brings four things to
the as-built state once R0–R6 (#1269–#1275) and #1128 have merged: the user guide's three pages, the
layout fixture with its review states and gates, a recorded visual pass at 1280×800 in both themes,
and the *Review tab redesign* mockup canvas (with the Overview board and `OverviewCard`); and it turns
`docs/development/review-redesign.md` from a proposal into a record.

The one piece of code that is not a test fixture or a doc is a `micold-core` integration test,
`crates/micold-core/tests/review_keys_doc.rs`, that holds the guide's shortcut table equal to the
review key table the client binds (FR-004), on the pattern of
`crates/micold-core/tests/terminal_only_doc.rs`. Layout work is test-side only: new and redefined
entries in `crates/micold-client/tests/support/covered_states.rs`, one new gate
`crates/micold-client/tests/gates/review_tab_fits.rs`, and one re-record of
`crates/micold-client/tests/fixtures/layout_snapshot.txt`. Decisions and rejected alternatives:
[research.md](research.md).

## Technical Context

**Language/Version**: Rust (workspace toolchain, `rust-toolchain.toml`), Markdown (mdBook user guide).

**Primary Dependencies**: `micold-core` (review key table, new in #1274), `micold-client` test
support (`tests/support/layout.rs`: `records_for`, `text_overflows`, `escapes`, `StateUnderTest`),
the `visual-pass` skill (Xvfb + lavapipe + xdotool), the docs pipeline (`site/checks/links.sh`,
mdBook). No new crate.

**Storage**: N/A. Nothing new is stored. The reviewed set (R4) is stored by the session service; this
feature only documents it.

**Testing**: `cargo test -p micold-core --test review_keys_doc` (FR-004);
`cargo test -p micold-client --test layout_snapshot`, `layout_text_overflow` (the gates under `tests/gates/` compile into `layout_snapshot` through
`#[path]` modules) (FR-005–FR-007); `scripts/tests/documentation-set.test.sh` and
`crates/micold-core/tests/documentation_is_not_read.rs` (the `.gitattributes` exception);
`site/checks/links.sh --sources` (FR-014); the visual pass by the `visual-pass` skill (FR-009).
`mise run gate` before every push (`mise` is missing in this cloud container: run the steps of
`scripts/gate.sh` directly, as the mise task does).

**Target Platform**: Linux, macOS, Windows desktop (CI on all three). The visual pass runs on Linux.

**Project Type**: desktop application (iced) with a daemon; this feature touches docs and tests.

**Performance Goals**: Each new covered state is timed warm as `layout-snapshot.md` § *Adding a
covered state* asks; the figure goes in that section. No runtime goal: no runtime code changes.

**Constraints**: The fixture is re-recorded **once**, after every blocker has merged, against the
bundled monospace face (#1260). The fixture is never re-recorded for another issue's layout. No
review behaviour, layout or key changes (spec Out of Scope); a defect that needs one is filed.

**Scale/Scope**: 3 user-guide pages, 1 development doc rewritten to a record, 1 development doc
section, about 12 covered states added or redefined, 1 gate, 1 core test, ~17 visual-pass checks × 2
themes, ~10 mockup boards and components.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Verdict | How |
|---|---|---|
| **I. Test-First** | PASS | The only non-test code is docs. FR-004's test is written first and fails (no marker table in the guide) before the guide's table is written. Each new covered state is registered and fails `the_layout_matches_the_committed_fixture` before the re-record; the new gate is written against the states and seen to fail with a deliberate one-pixel padding change (spec US2 Independent Test) before it is accepted. |
| **II. Multi-Session** | PASS | No new state. The guide states per-worktree Review tab state and cross-window agreement (FR-001); the visual pass has the two-worktree switch check (FR-009); a covered state holds two worktrees' tabs ([contracts/review-states.md](contracts/review-states.md)). |
| **III. Worktree Integration** | PASS | Nothing touches files or VCS. The guide describes the Review tab as one per worktree and dropped on worktree removal, as R0 builds it. |
| **IV. Local-First** | PASS | Nothing leaves the device. The fixture states use fixed invented data; the visual pass uses a private data directory (`visual-pass` §4). The mockup sync publishes to the owner's own claude.ai artifact, which is the design tool, not app state. |
| **V. Rust + iced** | PASS | Test code in Rust against the iced widget tree; no new types in production code. |
| **VI. Cross-Platform** | PASS | The fixture must pass on all three CI hosts (US2 sc.2); the guide names `Ctrl` / `Cmd` differences from the key table, and the FR-004 test checks both renderings ([contracts/guide-shortcut-table.md](contracts/guide-shortcut-table.md)). macOS / Windows screens: checked or recorded as not run (spec Edge Cases). |
| **VII. Documentation** | PASS | This feature is the documentation. Docs CI (`links.sh --sources`, mdBook build) must pass (FR-014). |
| **VIII. Reusable UI** | PASS | No widget added. The mockup's shared components (`TitleBar`, `Sidebar`, `StatusBar`, `OverviewCard`, review bar, drawer, …) are synced in place on the shared `tokens.css`, not forked per board. Small styling fixes from a visual-pass FAIL (FR-010) go in the shared component, never a feature-local override. |

No violations; Complexity Tracking is empty.

## Requirement map

| FR | Where it is done | Tested by |
|---|---|---|
| FR-001 | `docs/user-guide/reviewing-changes.md`, sections per [contracts/guide-outline.md](contracts/guide-outline.md) | Read-through against the visual-pass screenshots (quickstart §A); FR-004 test for the keys |
| FR-002 | Same page: remove every statement on the outline's *Must not say* list | `grep` list in quickstart §A2 |
| FR-003 | `docs/user-guide/terminal-panes.md` (tab strip), `docs/user-guide/worktrees-and-sessions.md` (row menu, header, sidebar line) | quickstart §A3 |
| FR-004 | `crates/micold-core/tests/review_keys_doc.rs` (new); marker table in `reviewing-changes.md`; `.gitattributes` `-micold-docs` line | Core test; `documentation-set.test.sh`; `documentation_is_not_read.rs` |
| FR-005 | One re-record of `layout_snapshot.txt`, line groups explained in the PR body | `layout_snapshot` binary; PR review |
| FR-006 | `covered_states.rs`: states in [contracts/review-states.md](contracts/review-states.md); `session-area-changes-open` redefined | `layout_snapshot`, coverage-narrowing test (`layout_snapshot.rs` T016) |
| FR-007 | `gates/review_tab_fits.rs` (new): every `review-*` state at `MIN_WINDOW_SIZE` and 1280×800, both themes; existing gates already cover 1280×800 | `layout_snapshot` binary (gate modules) |
| FR-008 | `docs/development/layout-snapshot.md` § *What is covered*: one paragraph for the review states | quickstart §A4 |
| FR-009 | `specs/1276-review-ui-docs-sync/visual-pass/` with `results.md`; checks in [contracts/visual-pass-checks.md](contracts/visual-pass-checks.md) | `visual-pass` skill (quickstart §B) |
| FR-010 | Fix in the shared component / doc, or an issue linked in `results.md` | `results.md` has no FAIL without a fix or link |
| FR-011 | Mockup canvas artifact `MviqD4uqBNaZtsCKH7yuTu`: boards and components per [data-model.md](data-model.md) § Mockup board | quickstart §C (board-by-screenshot comparison) |
| FR-012 | Same: follow-ups labelled "follow-up, not built", other unbuilt elements removed | quickstart §C |
| FR-013 | `docs/development/review-redesign.md` rewritten as a record (status, PRs, answers, differences, follow-ups) | quickstart §A5 |
| FR-014 | Inbound links to `reviewing-changes.md#…` (today: `settings.md:660`, `worktrees-and-sessions.md:240`, `agent-tools.md:300`, `agent-tools.md:311`) updated when a heading moves | `site/checks/links.sh --sources`; docs build |

## Design

### 1. Guide-equals-bindings test (FR-004)

`crates/micold-core/tests/review_keys_doc.rs` reads the text between
`<!-- review-shortcuts:start -->` and `<!-- review-shortcuts:end -->` in
`docs/user-guide/reviewing-changes.md`, parses the Markdown table's body rows, and compares them in
order with rows built from the review key table that R5 (#1274) puts in `micold-core` and the client
binds. The row format, the platform rendering and the failure messages are in
[contracts/guide-shortcut-table.md](contracts/guide-shortcut-table.md). The marker-section and
row-parsing helpers are the same as `terminal_only_doc.rs`'s; they move to a shared
`crates/micold-core/tests/support/marked_table.rs` used by both tests rather than being copied
(`mise run duplication` budget).

The table's name and shape come from the merged #1274. The plan names it
`micold_core::review::keys::REVIEW_KEYS` (new in #1274); the implementer uses whatever #1274
merged and updates the contract's names. A `micold-core` review key table that the client binds and
the `?` sheet reads is a requirement on #1274, passed to that thread at plan time. This feature
never moves the table itself (no production module here, spec Out of Scope): if #1274 merges
without one, it files a blocker issue for the table, links it from the ledger and the spec status,
and M1's key test waits on it (research R1). If the table has no sheet text or no modifier flag,
the fallbacks in [contracts/guide-shortcut-table.md](contracts/guide-shortcut-table.md) § *If the
table is thinner* apply.

`.gitattributes` gains, after `docs/**`, a commented
`/docs/user-guide/reviewing-changes.md -micold-docs` line in the form of the
`worktrees-and-sessions.md` entry, so `documentation_is_not_read.rs` accepts the read and
`scripts/classify-change.sh` runs the pipeline when the page changes.

### 2. Layout fixture and gates (FR-005–FR-008)

Order of work, all after the last blocker merges (spec Edge Cases):

1. Rebase on `origin/main`; run `layout_snapshot` and confirm it passes as merged (each slice
   re-recorded its own states).
2. Register the states of [contracts/review-states.md](contracts/review-states.md) that no slice
   registered, reusing the slices' state builders in `covered_states.rs` (`grep -n` for each
   slice's builder before writing one). Redefine `session-area-changes-open` as the Review tab
   shown in the strip by changing its builder `changes_open_under_header()` in place, and update
   both users of that builder: `gates/session_area_fits.rs::the_header_stands_over_the_open_changes_view`
   (header, strip with the Review tab, then the review body) and the press tests in
   `session_area_stability.rs` (`rendered_presses`,
   `pressing_changes_with_the_view_open_on_the_shown_location_closes_it`), whose expected message
   becomes what R0 built for **Changes** pressed while the Review tab is shown. If R0 already
   changed them, follow R0.
3. Write `gates/review_tab_fits.rs`, registered in `layout_snapshot.rs` with a `#[path]` module as
   `session_area_fits` is: for every state whose name starts `review-`, in both themes, at
   `MIN_WINDOW_SIZE` and 1280×800, assert no `text_overflows`, no `escapes` beyond the declared
   exemptions, the review bar's bottom edge at or above `status_bar`'s top edge, and the drawer's
   rectangle inside the diff's (overlay, not a narrower diff). For the same states, windows and
   themes it also runs the two existing gates' invariants, so every FR-007 gate holds at both
   windows: the status bar's (28 dp high, flush with the window's bottom edge, full width), moved
   out of `status_bar_fits.rs::at_the_smallest_window_the_bars_texts_stay_in_their_boxes` into a
   `pub(crate) fn` that test also calls; and the session area's header and strip checks, by
   calling `session_area_fits.rs`'s `assert_fits_under` (with `assert_order`, `header_parts`,
   `strip_parts`) made `pub(crate)`. Where the Review tab changes the strip's child count that
   function asserts, the count comes from `STRIP_MEMBERS`, not a second constant. It calls the
   existing `tests/support/layout.rs` helpers; nothing is copied from `layout_text_overflow.rs`,
   `gates/containment.rs`, `status_bar_fits.rs` or `session_area_fits.rs`.
4. Re-record once (`UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot`),
   then explain the diff line group by line group in the PR body (FR-005).
5. Prove the gate is live: a one-pixel change to the review bar's padding fails `layout_snapshot`
   naming `review.bar`; revert.
6. Write the review paragraph in `docs/development/layout-snapshot.md` § *What is covered* and
   replace the `session-area-changes-open` sentences (FR-008, US2 sc.3); record the warm timing in
   § *Adding a covered state*.

### 3. User guide (FR-001–FR-003, FR-014)

`reviewing-changes.md` is restructured to the outline in
[contracts/guide-outline.md](contracts/guide-outline.md): each R0–R6 control is a heading (SC-005).
Sections that describe unchanged behaviour (anchoring, the review message, statuses, the base line,
the Default entry) keep their text and heading ids, so inbound links still resolve. *Pull requests*
keeps its heading (and `#pull-requests`) but is rewritten for the source control's **Pull
requests** entry: today it describes the left-column section FR-002 forbids. The headings that go
(*The review panel*, *Committed and uncommitted* as a list toggle) are folded into their new
sections, and every inbound link is re-pointed (FR-014); the link text at `settings.md:660` and
`worktrees-and-sessions.md:240` drops "Changes view" for the Review tab's source control. Each slice's own guide edits are
the starting text; this feature reconciles them into one chapter and removes leftovers (FR-002).

### 4. Visual pass (FR-009, FR-010)

Run by the `visual-pass` skill (forked, Sonnet) against the real client and daemon on Xvfb with a
private data directory seeded per [data-model.md](data-model.md) § Visual-pass world. One crop per
check per theme in `specs/1276-review-ui-docs-sync/visual-pass/NN-<check>-{light,dark}.png`, and
`visual-pass/results.md` with date, environment (Xvfb + lavapipe), verdict per check, and for each
FAIL the fix commit or issue link. macOS / Windows: recorded as not run, with the reason, unless a
host is available.

### 5. Mockup canvas (FR-011, FR-012)

The canvas is the claude.ai artifact `https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu`. The
implementer reads it with the Artifact tool (`action: read`); when the read says *writer*, it edits
the saved file and republishes to the same `url`, keeping the shared `tokens.css` and components;
otherwise it hands the owner the difference list (spec Edge Cases) and the spec status says so.
Each board is compared with its visual-pass crop (so the mockup step runs after the pass); every
difference is fixed or, for a follow-up, labelled per FR-012. The difference list is written in
`visual-pass/mockup-sync.md` either way, as evidence for SC-004.

### 6. Redesign record (FR-013)

`docs/development/review-redesign.md`: title and status become *Review UI redesign: record* /
built (epic #1277); the Tasks table gains a PR column (R0–R6 merged PRs, R7 this PR); each open
question 0–5 gets the build's answer; a *Differences from the proposal* list (renames, keys bound
differently, parts cut) from each slice's merged spec and PR; a *Follow-ups not built* list (split
view, section 0) with issue links where they exist; the "visual pass at 900 / 1280 / 1920" line in
*Scope and files* is corrected to 1280×800 (clarification 2).

## Test strategy by layer

| Layer | Requirements | What |
|---|---|---|
| Core unit (`micold-core/tests`) | FR-004 | `review_keys_doc.rs`; `documentation_is_not_read.rs` and `scripts/tests/documentation-set.test.sh` for the attribute line |
| Client geometry (`micold-client/tests`) | FR-005, FR-006, FR-007 | `layout_snapshot` (fixture, coverage narrowing), `layout_text_overflow`, `gates/containment.rs`, new `gates/review_tab_fits.rs`, updated `gates/session_area_fits.rs` |
| Docs CI | FR-001–FR-003, FR-008, FR-013, FR-014 | `site/checks/links.sh --sources`, mdBook build; read-throughs in quickstart §A |
| Visual pass (quickstart §B) | FR-009, FR-010, and the look of FR-001 | `visual-pass` skill, `results.md` |
| Mockup comparison (quickstart §C) | FR-011, FR-012 | `mockup-sync.md` |

## Sequencing

Every deliverable reads the build, so the work starts when the blockers have merged. Proposed
milestones for the tasks unit (it may merge them):

- **M1 — guide and key test** (needs R0–R6 merged): §1, §3. Can start before #1128 merges.
- **M2 — fixture and gates** (needs every blocker incl. #1128): §2.
- **M3 — visual pass, mockup, record** (needs M2 merged, so the pass sees the final tree): §4, §5, §6.

## Project Structure

### Documentation (this feature)

```text
specs/1276-review-ui-docs-sync/
├── spec.md
├── plan.md                 # this file
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── guide-shortcut-table.md
│   ├── guide-outline.md
│   ├── review-states.md
│   └── visual-pass-checks.md
├── visual-pass/            # M3: crops, results.md, mockup-sync.md
├── autopilot.md
└── tasks.md                # tasks unit
```

### Source code (repository root)

```text
.gitattributes                                   # + reviewing-changes.md -micold-docs
crates/micold-core/tests/
├── review_keys_doc.rs                           # new (FR-004)
├── terminal_only_doc.rs                         # uses the shared helper
└── support/marked_table.rs                      # new: marker section + table rows (moved, not copied)
crates/micold-client/tests/
├── support/covered_states.rs                    # review states added / redefined
├── gates/review_tab_fits.rs                     # new (FR-007)
├── gates/session_area_fits.rs                   # follows the redefined state; checks pub(crate)
├── gates/status_bar_fits.rs                     # bar invariants as a pub(crate) fn
├── session_area_stability.rs                    # press tests follow the redefined builder
└── fixtures/layout_snapshot.txt                 # one re-record (FR-005)
docs/user-guide/{reviewing-changes,terminal-panes,worktrees-and-sessions,settings,agent-tools}.md
docs/development/{layout-snapshot,review-redesign}.md
```

**Structure Decision**: Existing workspace layout; no new crate or module in production code.

## Complexity Tracking

None.
