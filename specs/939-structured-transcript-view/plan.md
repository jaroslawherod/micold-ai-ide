# Implementation Plan: Structured Session Transcript View — read-only assistant text, thinking and tool calls

**Branch**: `claude/project-thread-61i2k8` | **Date**: 2026-10-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/939-structured-transcript-view/spec.md`

## Summary

Replace #938's plain one-line-per-event log of a structured session with a read-only transcript.
A new render-free core module, `micold_core::structured::transcript`, takes the same
`StructuredEvents` messages the log takes and maintains, incrementally, an ordered list of entries:
session start, user messages, assistant text, thinking cards, tool-call cards paired with their
results by tool use id, turn footers, notices and raw rows for everything it does not model.
Partial-message deltas grow their entry in place and the finished message supersedes them by
position (research R3). The client keeps one transcript per session per window plus its view state
(card fold set, follow-tail) in a render-free feature module, and draws it in `ui/transcript/`
through one replaceable renderer per block kind — the seam #940 swaps (FR-022, R10). Cards are a
new shared `DisclosureCard` built on `AccordionSection`; follow-tail uses the scrollable's relative
end offset (R9). No new event, wire message or protocol version (*Client only*). Details:
[research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/).

## Technical Context

**Language/Version**: Rust stable (workspace toolchain)

**Primary Dependencies**: existing only — `serde_json` (reading `raw` for block starts and result
shapes, R3, R5), `iced` 0.14 (`lazy`, `stack`, `operation::snap_to`/`scroll_to`), the shared
`ui/material` library. No new crate.

**Storage**: none. Transcript and view state are per window, in memory (spec *Assumptions*).

**Testing**: `cargo test` — core fixture tests (`crates/micold-core/tests/structured_transcript.rs`
on #938's recorded `tests/fixtures/stream-json/*.jsonl`) and unit tests in the module; client
reducer tests (`crates/micold-client/tests/features_structured_transcript.rs`); a release-only
budget test (`transcript_open_budget.rs`) run in CI's release-budget step; quickstart Part B visual
pass for draw code.

**Target Platform**: Linux, macOS, Windows (client only)

**Project Type**: desktop app — iced client over the shared render-free core

**Performance Goals**: SC-004 — 5,000 events, every card open, first show < 1 s in release; no
update + layout stalls input > 100 ms while streaming or scrolling (R12).

**Constraints**: same event bound as the plain log (`MAX_EVENTS` = 5,000, FR-020); no protocol
change; PTY sessions untouched (SC-006); failure never by colour alone (FR-012).

**Scale/Scope**: a handful of structured sessions per window; tool results up to several MiB.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: every decision (event → entry rules, pairing, streaming supersede, bounds,
  sub-agent counting, summaries, figures, fold and follow state) is in `micold-core` or the
  render-free `features/structured_transcript.rs`, driven by failing tests first. Only the draw code
  in `ui/transcript/` and the pane wiring use the GUI exception, validated by
  [quickstart.md](quickstart.md) Part B and the budget test.
- [x] **II. Multi-Session**: transcript and view state are keyed by `SessionId` per window; an
  isolation test drives two sessions and asserts no shared fold, follow or entries (FR-019). Nothing
  new is persisted; sessions' persistence and restore are #938's, unchanged.
- [x] **III. Worktree Integration**: no file or VCS operation; the session's location is #938's.
- [x] **IV. Local-First**: nothing leaves the device; the view reads events the local service already
  sends.
- [x] **V. Rust + iced**: iced only. `ToolStatus`, `EntryKind`, `Follow` are closed enums; an
  entry's key is fixed at construction (no setter), so a card cannot change identity; a result is
  only reachable through `ToolCard.result` or `OrphanResult`, never as a free entry.
- [x] **VI. Cross-Platform**: core never branches on the OS; the end-of-scroll test works in whole
  pixels with a slack that covers momentum and fractional trackpad steps (R9); Part B checks wheel,
  trackpad and scrollbar.
- [x] **VII. Documentation**: `docs/user-guide/worktrees-and-sessions.md` *Structured sessions*
  describes the transcript in place of the log, in the milestone that shows it (FR-023).
- [x] **VIII. Reusable UI**: cards are a new shared `material::DisclosureCard` composed from
  `AccordionSection`, `Accordion`, `Button`, `Ellipsized` and `IconLabel`, with a builder and a
  showcase entry; `Text` gains a `.monospace()` builder step; footer and "jump to latest" are
  composed from shared primitives (R11). No transcript-local widget.

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/939-structured-transcript-view/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── transcript-model.md     # core model input, event→entry rules, guarantees, seams for #940–#944
│   └── block-renderers.md      # the per-block-kind renderer seam (FR-022)
└── tasks.md                    # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/structured/
├── mod.rs                      # + pub mod transcript; − log_line re-exports (R13)
├── log_line.rs                 # REMOVED with the plain log (R13)
└── transcript/                 # NEW, render-free
    ├── mod.rs                  # Transcript: apply (replace-or-continue), bound, accessors
    ├── entry.rs                # Entry, EntryKey, EntryKind, ToolCard, ToolStatus, TurnFooter, ResultPart
    ├── build.rs                # one event → entries: rules of contracts/transcript-model.md
    ├── stream.rs               # OpenStream: message_start/block start/deltas/supersede (R3)
    ├── summary.rs              # input_summary (FR-009, R6)
    ├── result.rs               # result_parts from the User event's raw (R5)
    └── figures.rs              # footer formatting (R14)
crates/micold-core/tests/
└── structured_transcript.rs    # NEW: G1–G5 over the recorded fixtures + synthetic cases

crates/micold-client/src/
├── features/structured_transcript.rs  # NEW render-free: per-session Transcript + fold set + Follow
├── features/structured_log.rs  # REMOVED (R13)
├── features/mod.rs             # module swap
├── app.rs                      # State.structured_transcript replaces structured_log
├── shell/daemon_sync.rs        # StructuredEvents → structured_transcript; forget paths
├── shell/…                     # TranscriptMsg handling; snap_to / scroll_to tasks (R9)
├── icons.rs                    # + Thinking, ToolCall, ToolRunning, ToolDone, ToolFailed, ToolNoResult, JumpToLatest
├── ui/transcript/              # NEW draw code (GUI exception)
│   ├── mod.rs                  # view(state, session, renderers): Scrollable + lazy entries + jump control
│   ├── entries.rs              # one fn per EntryKind, composing shared components
│   └── blocks.rs               # BlockRenderers, RenderCtx, BlockCache, PLAIN (contracts/block-renderers.md)
├── ui/terminal.rs              # structured primary → ui::transcript::view; − structured_log_body
├── ui/panes.rs                 # tiled pane of a structured primary → ui::transcript::view (R15)
├── ui/material/disclosure_card.rs  # NEW shared component (R11)
├── ui/material/accordion_section.rs # + .leading(Icon), .trailing(Element) used by DisclosureCard
├── ui/material/text.rs         # + Text::monospace()
├── ui/material/mod.rs          # register DisclosureCard
└── showcase/catalogue.rs, showcase/sections/  # DisclosureCard entry
crates/micold-client/tests/
├── features_structured_transcript.rs  # NEW reducer tests (fold, follow, isolation, rebuild)
├── features_structured_log.rs  # REMOVED (R13)
├── transcript_open_budget.rs   # NEW release-only SC-004 budget (#[ignore] in debug)
└── icons.rs                    # + new codepoints
.github/workflows/ci.yml        # release-budget step + transcript_open_budget
docs/user-guide/worktrees-and-sessions.md  # Structured sessions: transcript (FR-023)
```

**Structure Decision**: the existing three-crate split; the daemon is untouched. The model is core
(fixtures, reuse by #940–#944), view state a render-free client feature, drawing `ui/transcript/`.

## Seams for the rest of epic #945

Later plans build on these; they are the stable surface of this feature.

| Seam | Where | For |
|---|---|---|
| Transcript model: `Transcript`, `Entry`, `EntryKey` (stable), `revision`, `EntryKind` (closed, extended by adding variants) | `micold_core::structured::transcript`; [contracts/transcript-model.md](contracts/transcript-model.md) | all |
| One renderer per block kind: `BlockRenderers { assistant_text, thinking, tool_input, tool_result }`, `BlockRenderers::PLAIN`, `BlockCache` keyed by `(SessionId, EntryKey, revision)` | `ui/transcript/blocks.rs`; [contracts/block-renderers.md](contracts/block-renderers.md) | #940 swaps renderers, fills the cache |
| The transcript pane is input-free; its column is the place #941 adds an input row under | `ui/transcript/mod.rs` | #941 |
| Tool card lookup by tool use id (`Transcript::tools`) | core | #942 prompts attach to cards |
| `SessionStart.model` | core | #943 |
| Sub-agent events: not entries, counted on `ToolCard.subagent_messages` or the `SubagentNotice`, kept in the history, `Transcript::subagent_events(id)` | core (R7) | #944 nests them under the card |
| `TurnFooter` per-turn cost and duration | core (R14) | #944 builds on it, does not re-implement |

## Requirement Coverage

Test layers: **core** = `micold-core` unit/fixture tests; **client** = `micold-client` reducer
tests; **geometry** = the material library's in-crate gates and `micold-client/tests/gates/`
(see below); **budget** = `transcript_open_budget` (release); **§B** = [quickstart.md](quickstart.md)
Part B visual pass; **docs** = diff review of the user guide.

Geometry gates added with the shared components (M4) and the pane (M6):

- `DisclosureCard` joins the elevated-surface scan of `ui/material/style_outline_discipline.rs`
  (its failed outline is the one border it draws), the card case of `ui/material/ripple_clipping.rs`
  (the header ripple stays inside the card's shape), and `ui/material/composition_contrast.rs`
  (header label, muted summary, and the failed label/icon in the error role are legible on the
  card's fill in both schemes).
- `tests/gates/disclosure_card_header_fits.rs` (NEW, wired into `tests/layout_snapshot.rs` like
  `structured_row_marker_fits.rs`): folded and open, failed and not, the header's leading icon,
  one-line summary and trailing status stay inside the header row, and the summary is one line.
- `tests/gates/transcript_jump_control_fits.rs` (NEW): the floated "jump to latest" control lies
  inside the transcript viewport and above its bottom edge, and covers no part of the scrollbar.
- `tests/layout_snapshot.rs` gains a transcript pane state (a `tool-turn` transcript, one card
  open, detached) so a layout change to it is a reviewed fixture change.

| FR / SC | Design | Test layer |
|---|---|---|
| FR-001 | R13; `ui/terminal.rs`, `ui/panes.rs` route a structured primary to `ui::transcript::view`; plain log removed | §B 1, 6; client (terminal session state untouched) |
| FR-002 | contracts/transcript-model.md rules; `EntryKind` | core G1 on `text-turn`, `tool-turn` |
| FR-003 | R4; `EntryKind::Raw` | core (`tolerance.jsonl`: every unknown line a raw row in place) |
| FR-004 | R8 derived omitted notice; `TurnEndedWithoutResult`, `Oversized` entries | core (omitted from service and from the bound; daemon note; oversized) |
| FR-005 | R14; `TurnFooter`, `figures.rs` | core (`error-result.jsonl` failed footer + text; omitted figures absent; formats) |
| FR-006 | `Transcript::is_empty`; "No output yet" in `ui/transcript` | core (empty); §B |
| FR-007 | R5; `tools` index | core G1 (one card per use, result inside, no result entry) |
| FR-008 | R5; `ToolStatus` transitions | core (running → done/failed in place; turn end → no result) |
| FR-009 | R6; `summary.rs` | core (rule per tool, generic, non-object input, one line, cap) |
| FR-010 | R11 `DisclosureCard` header button; `open` set starts empty | client (default folded, toggle); geometry (`disclosure_card_header_fits`, keyboard focus stop as `AccordionSection`'s in-crate test); §B 2 |
| FR-011 | R5 `ResultPart`; block-renderers R-5 | core (block-list result: text parts + named image) ; §B |
| FR-012 | R11: label + icon + error role | geometry (`composition_contrast` failed role, `style_outline_discipline`); client (a failed card and footer carry the "Failed" label and `Icon::ToolFailed`, asserted on the entry view model); §B 3 |
| FR-013 | R2 stable keys; fold set keyed by `EntryKey` in view state | core G3; client (persists across events, session switch, rebuild) |
| FR-014 | R3 `stream.rs` | core (each delta grows the same entry; thinking and tool input) |
| FR-015 | R3 supersede in place | core G2 |
| FR-016 | R2 per-entry `revision`; R9 absolute offset when detached | core G2 (other entries' revisions unchanged); client; §B 4 |
| FR-017 | R9 `Follow`, jump control | client (at end / away / back / jump; control shown only while detached); geometry (`transcript_jump_control_fits`); §B 4 |
| FR-018 | R9 initial `Following`, `snap_to(END)` on first show | client; §B 1 |
| FR-019 | R15; state keyed by `SessionId` per window | client (two sessions isolated); §B 5 |
| FR-020 | R8 bound and eviction; `OrphanResult` | core G4 (count, entries leave the top, result without its call) |
| FR-021 | R8 replace-or-continue; fold pruning | core (rebuild: no duplicates); client (fold kept for present keys) |
| FR-022 | R10; contracts/block-renderers.md | client `transcript_renderer_seam.rs` (NEW): swaps `assistant_text` for a marker renderer, builds the view over the `tool-turn` transcript, and asserts the marker appears once per assistant-text entry while the other kinds' output, entry order and fold state are unchanged (R-1, R-2) |
| FR-023 | `docs/user-guide/worktrees-and-sessions.md` | docs |
| FR-024 | R7 | core G5 (no entry, counts, orphan notice, deltas not counted, `subagent_events`) |
| SC-001 | R4, R5 | core G1 over every recorded fixture |
| SC-002 | R3 | core G2 on `partial-deltas`, `tool-turn` |
| SC-003 | R9 `Follow::scrolled` | client |
| SC-004 | R12 | budget (CI release step) |
| SC-005 | R11 | geometry (contrast, both schemes); §B 3 (greyscale screenshots) |
| SC-006 | no change to terminal paths | `scripts/gate.sh` (whole suite) |

## Delivery

Six milestones, cut in [tasks.md](tasks.md): **M1** core model for finished events (entries,
cards, pairing, footers, raw rows, notices, summaries, figures, sub-agent counts); **M2** core
streaming, supersede, bound, eviction and re-sync; **M3** client feature module (per-session
transcript, fold set, follow state, messages) with the wire routed to it beside the plain log;
**M4** shared components (`DisclosureCard`, `AccordionSection` extension, `Text::monospace`, icons,
showcase); **M5** `ui/transcript/` with the renderer seam, lazy entries and the SC-004 budget test
(drawn in the showcase and the budget test, not yet in the pane); **M6** the pane switch (single
and tiled), scroll tasks, plain log removal, user guide and the Part B visual pass. The transcript
is unreachable from the session pane until M6, so the user guide changes with it.

## Complexity Tracking

None.
