# Implementation Plan: Structured View — Select and Copy Transcript Text

**Branch**: `claude/project-thread-zj503u` | **Date**: 2026-10-28 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1550-transcript-text-selection/spec.md` (decisions
D1–D6 in [autopilot.md](autopilot.md)).

## Summary

Make every piece of text the structured transcript draws selectable as one cross-entry range (D1),
copied only on request (D6) by the copy keys, a context menu or per-value copy controls, without
changing the transcript model, its entries, folding, streaming or follow-tail.

A render-free core module `micold_core::structured::selection` holds the model: a position is
`(EntryId, TextPath, byte offset)`, ordered in reading order by construction (`EntryId` is already
the entry's position in the build; `TextPath` is the unit's drawing order inside the entry), with
grapheme snapping, word and source-line expansion, and the per-entry part of a selection that
joins each entry's `lazy` key (research R1, R4, R6). What a unit copies is its **shown text**,
computed render-free from the same inputs the renderers draw (`markdown::shown_text`, list markers,
diff lines without gutter, tab-expanded plain text), and a parity test proves the view draws exactly
those units in that order (R2, R3). A render-free client module `features/transcript_selection.rs`
keeps one selection per session per window, clears it per FR-019 by checking only changed entries
in range (R7), assembles the copied text and emits #940's checked clipboard write (R9).

Two new shared widgets in `micold-material` do the drawing and the pointer: `SelectableText` (styled
spans shaped one paragraph per hard line, so iced 0.14's line-relative `hit_test` is unambiguous;
highlight in `primary_container`/`on_primary_container`) adopted by `Text`, `LinkText`, `CodeBlock`
and `InlineDiff` through a `.selectable(...)` builder; and `SelectionArea`, the transcript's focus
target, gesture owner, edge-scroller and right-click origin (R5). Copy keys are arbitrated in the
existing `features/composer.rs::transcript_key`, which gains the selection, the transcript's focus and the
platform as inputs so a selection is copied and never interrupts (#1403), and no selection keeps #941 FR-012a
(R8). The context menu reuses `micold_material::ContextMenu` (R11). Copy controls generalise #940's
code-block copy state from `CodeId` to a `CopyTarget` (tool value, input, result, diff path), so
tickets, read-back and confirmation are reused unchanged (R10). Details: [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/).

## Technical Context

**Language/Version**: Rust (workspace toolchain, edition as in `Cargo.toml`)

**Primary Dependencies**: iced 0.14 (`advanced` widgets, `Paragraph::{hit_test, span_bounds}`,
`clipboard`), `unicode-segmentation` 1.13 (already in `micold-core` and `micold-material`). No new
dependency.

**Storage**: N/A — selection, menu and copy state are in-memory, per window; nothing persisted.

**Testing**: `cargo test` via `scripts/gate.sh` (cloud) / `mise run gate`: `micold-core` unit and
integration tests (model), `micold-client` reducer and view tests (state, keys, parity),
`micold-material` widget tests by simulated events, in-crate geometry and contrast gates, the
release budget test, quickstart Part B (GUI exception).

**Target Platform**: Linux, macOS, Windows desktop client.

**Project Type**: desktop app (Cargo workspace: `micold-core`, `micold-client`, `micold-material`,
`micold-daemon`; the daemon is untouched).

**Performance Goals**: highlight follows the pointer within 100 ms; select-all + copy at the event
bound within 1 s (SC-003); copy-control confirmation within 1 s (SC-004).

**Constraints**: only entries whose selected part changed are rebuilt (per-entry `lazy`, #939 R12);
no copy on release (D6); LF line breaks on every platform; no grapheme split.

**Scale/Scope**: one transcript at its event bound (#939 FR-020, `MAX_EVENTS`); all entry kinds of
#939/#940/#942/#944.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.* Re-checked after Phase 1:
all PASS, no deviation.

| Principle | Status | How |
|---|---|---|
| **I. Test-First** | PASS | Every decision — order, shown text, grapheme snapping, word/line expansion, clearing, copy text, key arbitration, copy-target values — is in `micold-core` or the render-free `features/transcript_selection.rs`, `features/composer.rs`, `features/transcript_rich.rs`, test-first. The widgets are tested by simulated events in `micold-material`; the view/model agreement by the parity test; only final drawing uses the GUI exception (quickstart Part B). |
| **II. Multi-Session** | PASS | Selection and menu state are `HashMap<SessionId, _>` per window; a pane shows a session at most once (`pane_layout.rs:478`), so per session is per pane; S-7 isolation test drives two sessions and two windows; copy-control state stays per window (#940). Nothing persisted. |
| **III. Worktree Integration** | PASS | No file or VCS operation. |
| **IV. Local-First** | PASS | Only the local system clipboard through iced; no network, nothing leaves the device. |
| **V. Rust + iced** | PASS | iced only. `TextPos` ordering is structural; `CopyTarget`, `Granularity`, `RowPos`, `RowSelection`, `KeyIntent` are closed enums; an empty selection is a value (`is_empty`), not a flag pair; offsets are snapped at construction. |
| **VI. Cross-Platform** | PASS | One implementation; the only platform difference (Cmd vs Ctrl) is a `mac` parameter tested both ways on every host (R8); LF everywhere; clipboard via iced with read-back (#940 R5); Part B on all three CI-built clients. |
| **VII. Documentation** | PASS | `docs/user-guide/worktrees-and-sessions.md` *Structured sessions* gains selecting, copy keys and their interrupt arbitration per platform, the menu and copy controls, in the milestones that make them visible (selection, copy keys and menu in M4b; copy controls in M5); #939's out-of-scope note is updated (FR-021). |
| **VIII. Reusable UI** | PASS | `SelectableText` and `SelectionArea` are new shared components in `micold-material` with builders, showcase entries and gates; `Text`, `LinkText`, `CodeBlock`, `InlineDiff` adopt them by a builder method rather than forking; `ContextMenu`, `IconButton`, `TakesTheKeyboard` reused. |

## Project Structure

### Documentation (this feature)

```text
specs/1550-transcript-text-selection/
├── spec.md
├── plan.md              # this file
├── research.md          # R1–R13
├── data-model.md
├── quickstart.md        # Part A automated, Part B visual pass
├── contracts/
│   ├── selection-model.md   # S-1–S-12
│   ├── selectable-text.md   # T-1–T-6, A-1–A-5
│   └── keys-menu-copy.md    # K-1–K-11, M-1–M-4, C-1–C-6
├── autopilot.md         # ledger
└── tasks.md             # /speckit-tasks (not created here)
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── structured/selection.rs              # new: RowPos, TextPath, TextPos, Selection, RowSelection, snap, word_at, source_line_at, range_in
├── structured/mod.rs                    # + pub mod selection
├── structured/transcript/summary.rs     # + work_value (uncut work field)
└── markdown/                            # + shown_text(&Inlines), list_marker
crates/micold-core/tests/structured_selection.rs   # new

crates/micold-material/src/
├── selectable_text.rs                   # new shared component
├── selection_area.rs                    # new shared component
├── text.rs, link_text.rs                # + .selectable(slot, range, on_hit)
└── lib.rs                               # exports

crates/micold-client/src/
├── features/transcript_selection.rs     # new: per-window SessionSelection, units(), copy_text, update/applied
├── features/structured_transcript.rs    # TranscriptMsg::Select(..), CopyCode → Copy{target}; fold toggle informs selection
├── features/transcript_rich.rs          # CopyTarget generalises CodeId copy state
├── features/composer.rs                 # transcript_key(press, phase, selecting, focused, mac): CopySelection, SelectAll
├── features/window.rs                   # FieldId::Transcript(SessionId)
├── ui/composer.rs                       # transcript_interrupt → transcript_keys subscription
├── ui/transcript/mod.rs                 # SelectionArea around the scrollable; lazy key + RowSelection; context menu
├── ui/transcript/blocks.rs              # RenderCtx.selection; PLAIN renderers selectable
├── ui/transcript/rich.rs                # RICH renderers pass slots/ranges to MarkdownView, InlineDiff
├── ui/transcript/entries.rs             # header/footer/notice/user text selectable; header + body copy controls
├── ui/transcript/permission.rs          # descriptive text selectable; controls not
├── ui/material/markdown_view.rs         # .selection(..)/.on_hit(..): TextPath per inline run, marker, cell
├── ui/material/code_block.rs            # selectable lines (full line text)
├── ui/material/inline_diff.rs           # selectable path/labels/lines; DiffPath copy control
├── shell/panes.rs                      # sync: retain_shown(&sessions, &now) drops selections no pane shows
├── shell/transcript.rs                  # Edge → scroll_by on transcript_scroll_id; copy outcome
└── showcase/                            # entries for the two components
crates/micold-client/tests/
├── features_transcript_selection.rs     # new
├── transcript_selection_parity.rs       # new
├── features_transcript_rich.rs          # + copy targets
└── transcript_open_budget.rs            # + selection variant

docs/user-guide/worktrees-and-sessions.md   # Structured sessions: selecting and copying
specs/939-structured-transcript-view/spec.md # out-of-scope note on copying points to #1550
```

**Structure Decision**: The existing three-crate client layout. Decisions go to `micold-core`
(render-free model and shown text) and render-free `features/` modules; reusable drawing goes to
`micold-material`; the transcript view and shell glue in `micold-client` only wire them.

## Requirement Coverage

Test layers: **core** = `micold-core` tests; **client** = `micold-client` reducer/state tests;
**parity** = `transcript_selection_parity`; **widget** = `micold-material` simulated-event tests;
**geometry** = in-crate gates (`composition_contrast`, layout snapshots); **budget** =
`transcript_open_budget` (release); **§B** = [quickstart.md](quickstart.md) Part B; **docs** = diff
review of the user guide.

| FR / SC | Design | Test layer |
|---|---|---|
| FR-001 | R5 `SelectableText` T-1/T-2, `SelectionArea` A-2; R6 highlight per entry | widget (hit, drag); client (Hit/Press/Release → selection across entries); §B 1 |
| FR-002 | R1 order; R2 units exclude control labels, icons, folded bodies | core S-1/S-2; parity (folded card yields header units only); §B 1 |
| FR-003 | R5 A-3 clamped cursor + `on_edge`; shell `scroll_by` | widget A-3; client (Edge emits scroll task for that session only); §B 2 |
| FR-004 | R5 A-1 controls capture; S-8/S-9 | widget A-1, T-5; client S-9; §B 3 |
| FR-005 | R4 `word_at`, `source_line_at`, T-3 visual line | core S-11; widget T-3; §B 4 |
| FR-006 | R8 K-8 `SelectAll`; S-10 | client (`transcript_key` K-8, S-10, empty transcript no-op); §B 4 |
| FR-007 | R5 A-2 focus → `FieldId::Transcript` | widget A-2; client (Press sets focused field); §B 5 |
| FR-008 | R8 K-1–K-4; R9 checked write; S-12 | client (K table with `mac` both ways; selection kept; no interrupt busy or idle); §B 5 |
| FR-009 | R8 K-5–K-7 | client (K-5 interrupts, K-6 nothing, K-7 never interrupts); #941 composer tests stay green |
| FR-010 | R2 join rules; S-2/S-3 | core (line breaks, no soft-wrap breaks, indentation, diff without gutter); parity; §B 1 |
| FR-011 | R2/R3 `shown_text`, markers, cells | core S-3 (markers, links, inline code, headings, tables); parity; §B 1 |
| FR-012 | R5 A-5; R9; S-8 | widget A-5; client S-8 (no clipboard outcome on release/double/triple click) |
| FR-013 | R11 `ContextMenu`; A-4; M-1–M-4 | widget A-4; client (menu state, Copy disabled when empty, selection unchanged); §B 4 |
| FR-014 | R9 read-back failure → composer notice | client (failed/`None` read-back sets notice, selection kept); §B (manual clipboard refusal not forced) |
| FR-015 | R10 `ToolValue`, `work_value` | core (`work_value` uncut per tool, `None` otherwise); client C-1; §B 6 |
| FR-016 | R10 `ToolInput`, `ToolResult` | client C-2 (held value, streaming value so far, empty); §B 6 |
| FR-017 | R10 `DiffPath` on `InlineDiff` | client C-3; geometry (control fits beside the path); §B 6 |
| FR-018 | R10 reuse of #940 tickets, `TakesTheKeyboard` | client C-4 (per control, per window); widget (Enter/Space); C-6 #940 tests unchanged |
| FR-019 | R1, R7 clearing rules; S-5/S-6 | core S-5; client S-6 (rebuild, eviction, toggle, replaced text, pane switch, forget) |
| FR-019a | R8 K-10/K-11; R11 M-2 refocus | client (focus to composer keeps selection; menu Copy copies and refocuses) |
| FR-020 | R5 colours; T-4 | geometry (`composition_contrast` pair on every transcript surface, both schemes); widget T-4 (unselected pixel-equal); §B 8 |
| FR-021 | user guide + #939 note | docs |
| SC-001 | R2, R3 | core S-2/S-3; parity over every entry kind; §B 1 on three platforms |
| SC-002 | R8 | client K table (100% deterministic); §B 5 |
| SC-003 | R6, R12 | budget (select-all + copy < 1 s; drag step < 100 ms) |
| SC-004 | R10 | client (byte-for-byte per target; confirmation through `COPY_CONFIRM`) |
| SC-005 | R1, R7 | core S-5; client (a full turn of fixture events with a selection: unchanged, highlighted part stable) |
| SC-006 | R5 colours | geometry (contrast ≥ 4.5:1 both schemes); §B 8 screenshots |

## Delivery

Six milestones, each one PR, cut in tasks.md (each fits one session, constitution task-size gate):

- **M1 — selection model (core)**: `structured::selection`, `markdown::shown_text`,
  `markdown::list_marker` (and `markdown_view.rs` calling it), `summary::work_value`; S-1–S-5,
  S-11. *Satisfies*: FR-002, FR-005 (text part), FR-010, FR-011, FR-019 (stability) at the model;
  SC-001, SC-005 at the model.
- **M2 — window state and keys (client, render-free)**: `features/transcript_selection.rs`
  (`units` for every `RowPos`, `copy_text`, clearing, menu state), `TranscriptMsg::Select`,
  `FieldId::Transcript`, `transcript_key` K-1–K-11 (with `focused`), `transcript_keys`
  subscription, copy outcome and failure notice, and `retain_shown` called from
  `shell/panes.rs::sync` with its two-pane test (S-6).
  *Satisfies*: FR-006, FR-008, FR-009, FR-012, FR-014, FR-019, FR-019a at the state; SC-002.
- **M3 — shared components**: `SelectableText`, `SelectionArea`, `.selectable` on `Text`,
  `LinkText`, `CodeBlock`, `InlineDiff`; showcase; contrast and pixel-equality gates. *Satisfies*:
  Principle VIII; FR-001, FR-003, FR-004, FR-005 (visual line), FR-007, FR-020 (component); SC-006.
- **M4a — selectable leaves (not yet user-visible)**: `RenderCtx.selection` and the entry's lazy
  key; every renderer and entry view (blocks, rich, entries, permission, omitted notice) passing
  `TextPath` slots and `range_in` ranges to the M3 components; the parity test
  (`transcript_selection_parity`: what each row draws equals `units`) and the
  `transcript_open_budget` select-all variant. With no `SelectionArea` yet nothing starts a
  selection, so the transcript looks and behaves as before (T-4 pixel-equality holds). *Satisfies*:
  FR-002, FR-010, FR-011 in the view; SC-001 (parity), SC-003 (budget).
- **M4b — selection in the transcript**: `SelectionArea` around the scrollable, hit/press/release
  routing to `TranscriptMsg::Select`, edge scroll in the shell (`scroll_by` on
  `transcript_scroll_id`), the context menu, focus to `FieldId::Transcript`; the user guide's
  selection, copy-key and menu text, where it becomes visible (Principle VII). *Satisfies*:
  FR-001, FR-003–FR-009, FR-012, FR-013, FR-019a end to end.
- **M5 — copy controls and docs**: `CopyTarget`, `TranscriptMsg::Copy`, header/input/result/diff
  path controls, the user guide's copy-controls text, #939's out-of-scope note, quickstart Part B.
  *Satisfies*: FR-015–FR-018, FR-021; SC-004.

## Complexity Tracking

No constitution violation. One design choice recorded for reviewers: `SelectableText` shapes one
paragraph per hard line instead of one per block, because iced 0.14's `Paragraph::hit_test` returns
a line-relative index without the line (research R5); the alternative, reading cosmic-text's buffer
through the concrete renderer, would add direct `cosmic-text`/`iced_graphics` dependencies pinned to
iced's.
