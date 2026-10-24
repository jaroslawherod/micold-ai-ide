# Implementation Plan: Layout redesign — user guide, layout snapshot and visual pass

**Branch**: `claude/project-thread-bznqpy` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1128-layout-redesign-docs/spec.md`

## Summary

The redesign's closing pass, with no behaviour change (FR-020). Stage 1, now: the user guide's
mentions of the status bar (#1124) and of the session area's context header and tab strip (#1126)
are corrected and linked to one owning section each; the bar under the terminal and every
ambiguous "the bar" go. Stage 2, after #1125 and #1127 merge: the same for the sidebar and the title
bar, an opening section in `worktrees-and-sessions.md` naming the four regions, two whole-window
covered states (`window-session-view`, `window-overview`) with a region gate and a regenerated
fixture, `layout-snapshot.md` updated, and a visual pass of four frames against the mockup boards.
Stale mentions are found and verified by one fixed set of grep patterns
([contracts/doc-edits.md](contracts/doc-edits.md)). Design decisions: [research.md](research.md).

## Technical Context

**Language/Version**: Markdown (user guide, `site/media.toml`); Rust stable via `mise` for the test
fixtures in M3 only

**Primary Dependencies**: none new. Existing: the layout-snapshot apparatus
(`crates/micold-client/tests/support/{layout.rs,covered_states.rs}`), `micold_core::tokens`
(`anatomy`, `contrast`), lychee through `site/checks/links.sh`

**Storage**: N/A. Nothing persisted or sent (Principle IV)

**Testing**: docs: `site/checks/links.sh --sources` and the pattern sweep (quickstart A1, A2); M3:
`tests/layout_snapshot.rs` (fixture, containment, new `gates/window_regions_fit.rs`) and
`tests/layout_text_overflow.rs`, with a mutation proof (A3); M4: quickstart Part B

**Target Platform**: Linux, macOS, Windows desktop; the visual pass is Linux only (recorded)

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: two covered states add little to the suite; their warm cost is measured and
recorded in `layout-snapshot.md` (R7)

**Constraints**: no change under `crates/*/src` (FR-020); stage 2 waits for #1125 and #1127 on main;
edits to pages outside the four limited to the sentences that name a removed element (FR-011)

**Scale/Scope**: 8 user-guide pages (the four, plus `appearance-theming.md`, `icons.md`,
`settings.md`, `sandboxed-daemon.md`), `site/media.toml`, `docs/development/layout-snapshot.md`;
tests: 2 covered states, 1 gate file, `strip_with` extended, the fixture; 4 screenshots and one record

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | No production code. The only code is test code (two covered states, gate W); its failing run is shown by the mutation proof (quickstart A3, contract *Mutation proof*), since it holds a layout that already ships (R6). |
| II. Multi-Session | PASS | Nothing changes per session. The pages say a row moves between groups as its state changes, the header and status bar follow the session shown, the Overview badge counts every worktree (Edge cases); `window-session-view` holds several worktrees in several states and a multi-tab strip. |
| III. Worktree Integration | PASS | No file or VCS operation added; fixtures use invented worktrees under `/fixture/project`. |
| IV. Local-First | PASS | No network: the link check runs offline (`--offline`, R10); fixtures read nothing from the host; screenshots are drawn locally. |
| V. Rust + iced | PASS | Test code in Rust against the iced view; no new dependency. |
| VI. Cross-Platform | PASS | Every chord in the four pages carries its macOS form (FR-018, R11); platform-specific title-bar behaviour is described if #1127 ships any; the visual pass records that macOS and Windows were not checked by eye. CI runs M3's tests on all three. |
| VII. Documentation | PASS | The feature is documentation; links checked in CI (`site/checks/links.sh --sources`). |
| VIII. Components | PASS | No UI code. Fixture builders are reused and extended (`with_every_card_state`, `strip_with`, `running_sandbox`), not copied. |

Post-design re-check: unchanged, all PASS. No Complexity Tracking entries.

## Design

### Milestones

| ID | Stage | Deliverable | Starts | Paths |
|---|---|---|---|---|
| M1 | 1 | Status bar and session-area mentions corrected and linked; ambiguous bars named | now | `docs/user-guide/{worktrees-and-sessions,overview,settings,sandboxed-daemon}.md` (doc-edits E1–E12) |
| M2 | 2 | Sidebar and title-bar mentions; FR-009 opening section; empty and no-worktree states; media alt text | #1125 and #1127 merged | the eight pages, `site/media.toml` (S1–S15) |
| M3 | 2 | `window-session-view`, `window-overview`, gate W, regenerated fixture, `layout-snapshot.md` passage | after #1127; may run beside M2 | `crates/micold-client/tests/support/covered_states.rs`, `tests/gates/window_regions_fit.rs` (new), `tests/layout_snapshot.rs` (one `#[path]` mod), `tests/fixtures/layout_snapshot.txt`, `docs/development/layout-snapshot.md` |
| M4 | 2 | Visual pass: 4 frames, 20 verdicts, contrast | after M3 | `specs/1128-layout-redesign-docs/visual-pass.md`, `visual-pass/*.png` |

If #1125 or #1127 is closed unmerged, the stage-2 parts for its region are dropped and the ledger
says so (Edge cases); M3 and M4 then compose what shipped.

### User guide (M1, M2)

- Owning sections per [research R2](research.md#r2--owning-sections); the status bar stays at
  `appearance-theming.md#status-bar`, the header and strip at their `terminal-panes.md` sections.
- `worktrees-and-sessions.md#the-tab-strip` keeps its instance rules and drops its second list of
  the strip's members (E6).
- Hits are found and the result verified by the patterns in
  [doc-edits §Patterns](contracts/doc-edits.md#patterns) (R3); `site/media.toml` alt text and
  captions of the four pages' media are in the sweep (R4).
- M2's opening section (FR-009) goes first in `worktrees-and-sessions.md`, before *Opening a
  project*; a renamed heading (`project-selection.md` S4) has its inbound links re-pointed.
- `docs/development/review-redesign.md` is not changed (spec Assumptions).

### Layout snapshot (M3)

- States and anchors: [contracts/covered-states.md](contracts/covered-states.md), composition in
  [research R5](research.md#r5--whole-window-covered-states-names-and-composition).
- `strip_with(shells, helpers)` gains a variant taking the base `State` and the location, so the
  strip is built on `with_every_card_state()` at `feat-working`; existing callers keep their
  result. At `feat-working` it replaces the session `with_every_card_state()` added there with its
  main session, keeping `ActivitySignal::Working`, so the row stays in its group (R5).
- Gate W reads `micold_core::tokens::anatomy::status_bar::HEIGHT` and the title bar's
  `anatomy::app_bar::HEIGHT` (today 64.0; #1127 makes it 44) and `BOTTOM_EDGE` (`HEIGHT +
  DIVIDER`), never literals (R6). `title_bar` anchors the bar, not its container; the 1 px
  hairline is the bar's bottom edge, so the body starts at `BOTTOM_EDGE` (W1, W3). W5 asserts on
  the `State`, not the records.
- Regenerate and confirm byte equality on main after #1127 (R7); `layout-snapshot.md` *What is
  covered* gets one passage listing each region's states with the issue that added them (#1124
  `status-bar-*`, #1125 `main-shell-sidebar-grouped` and its later states, #1126 `session-area-*`,
  #1127's states) and the two whole-window states (FR-014), plus their measured cost.

### Visual pass (M4)

- Drawn from the M3 states through `ui::view` and tiny-skia, harness in the scratchpad (R8).
- Record per [contracts/visual-pass-record.md](contracts/visual-pass-record.md); contrast per R9;
  defects to the ledger's follow-ups (D1).

## Requirement map and test layer

| Requirement | Where | Checked by |
|---|---|---|
| FR-001 | M1 E2, E8–E12 | quickstart A2 (stage 1 patterns) |
| FR-002 | M1 E3–E7 | A2 |
| FR-003 | M1 E1, E3–E5 | A2 (`the bar` pattern) |
| FR-004 | R2; M1 E2, E6–E10 | A2, SC-002 review of each hit |
| FR-005 | M2 S1–S11 | A2 (stage 2 patterns) |
| FR-006 | M2 S2–S3 | A2; review against #1125's shipped row and `keymap.rs` |
| FR-007 | M2 S4–S5 | A2; A1 for the renamed heading's links |
| FR-008 | M2 S13 | A2 (`quick switcher` pattern) |
| FR-009 | M2 S12 | review; A1 |
| FR-010 | R2; M2 | A2, SC-002 |
| FR-011 | M1 E10; M2 S7–S10 | A2 over all user-guide pages |
| FR-012 | M3, covered-states contract | geometry gates: `layout_snapshot` fixture, gate W |
| FR-013 | M3, R7 | A3 regeneration diff; `layout_text_overflow`; containment |
| FR-014 | M3 `layout-snapshot.md` | review; A1 |
| FR-015 | M4, record contract | quickstart §B |
| FR-016 | M4, R9 | quickstart §B3; `composition_contrast.rs` tests cited |
| FR-017 | M1, M2 | review against the running app's code (the shipped feature's spec and view) |
| FR-018 | R11; M1, M2 | A2 (chord check) |
| FR-019 | all docs milestones | A1 (`site/checks/links.sh --sources`, CI) |
| FR-020 | all | `git diff --stat origin/main -- crates/*/src` empty at each PR |
| SC-001–SC-003, SC-007 | M1, M2 | A1, A2 |
| SC-004, SC-005 | M3 | gate W; mutation proof (A3) |
| SC-006 | M4 | §B record count |

## Project Structure

### Documentation (this feature)

```text
specs/1128-layout-redesign-docs/
├── spec.md
├── plan.md              # this file
├── research.md          # R1–R11
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── doc-edits.md
│   ├── covered-states.md
│   └── visual-pass-record.md
├── autopilot.md         # ledger
├── visual-pass.md       # M4
├── visual-pass/         # M4 screenshots
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
docs/user-guide/
├── worktrees-and-sessions.md   # M1, M2
├── terminal-panes.md           # M1 (owning sections; links), M2
├── overview.md                 # M1, M2
├── project-selection.md        # M2
├── appearance-theming.md       # M2 (title bar), owning #status-bar
├── icons.md, settings.md, sandboxed-daemon.md   # FR-011 sentences only
docs/development/layout-snapshot.md               # M3
site/media.toml                                   # M2 alt text and captions
crates/micold-client/tests/
├── support/covered_states.rs   # M3: two states, strip_with extended
├── gates/window_regions_fit.rs # M3: new
├── layout_snapshot.rs          # M3: one #[path] mod
└── fixtures/layout_snapshot.txt  # M3: regenerated
```

**Structure Decision**: no source directory changes; test fixtures under
`crates/micold-client/tests/` and Markdown under `docs/` and `specs/`.

## Complexity Tracking

None.
