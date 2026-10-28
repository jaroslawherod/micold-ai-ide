# Autopilot ledger — #1550 transcript-text-selection

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1550 (issue #1550: Structured view: text and values cannot be selected or copied); labels: bug, flow:bug (now bug, flow:feature, in-progress; switched from the bug flow, which returned NEXT: feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1550
- **Worktree branch**: claude/project-thread-zj503u
- **Started**: 2026-10-27
- **Phase**: M4a
- **Next step**: M4a implement (T033–T038), continuing the Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1582 | design (spec, plan, tasks) | merged | 5ce410f219e38800d89f0936dbf9f48a96041cb1 |
| #1586 | M1: selection model, shown text and work_value in core | merged | f7c34e9d579bc65daea4a1bd7ca865d0ac9253ea |
| #1590 | M2: window selection state, copy text and copy keys | merged | 94519d3b85af68358495aa1eae645edf53174dc1 |
| #1593 | M3: SelectableText and SelectionArea components | merged | ffad78647cb67b85f478dc80617db267f9cdeddc |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | Selection model, shown text and work_value in core | #1586 | merged |
| M2 | T013–T023 | full | Window selection state, clearing, copy text and copy-key arbitration (render-free) | #1590 | merged |
| M3 | T024–T032 | full | SelectableText and SelectionArea components, .selectable on four leaves | #1593 | merged |
| M4a | T033–T038 | full | Selectable leaves in the transcript, parity and budget (not user-visible) | — | in progress |
| M4b | T039–T043 | full | Selection, edge scroll, menu and copy keys in the transcript; user guide | — | todo |
| M5 | T044–T049 | full | Copy controls, docs, visual pass | — | todo |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Per-block or cross-block selection? | Cross-block: one range across entries in reading order | agent-resolved | issue #1550 "Expected" (across lines and blocks); spec.md#assumptions |
| D2 | spec | Ctrl+C: copy or interrupt (#1403)? | Transcript focused + non-empty selection: copy, never interrupt; no selection: #941 FR-012a unchanged; Cmd+C / Ctrl+Shift+C never interrupt | agent-resolved | spec.md FR-008, FR-009; specs/941-structured-prompt-input/spec.md FR-012a |
| D3 | spec | Which values get copy controls? | Tool card identifying value (header), full input, full result, diff file path; code blocks keep #940's control; no whole-message control | agent-resolved | spec.md FR-015–FR-018 |
| D4 | spec | Platform behaviour? | Identical on Linux/macOS/Windows; only Cmd vs Ctrl differs; LF line breaks everywhere | agent-resolved | spec.md Terms (Copy keys), Edge Cases (Cross-platform) |
| D5 | clarify | FR-011: copy rendered markdown as visible text or source? | Visible rendered text as drawn; links copy shown text; no emphasis/heading/fence markup | agent-resolved | spec.md FR-010 ("as shown"), SC-001; user guide code-block copy "no fences, no tag" (#940 FR-011) |
| D6 | clarify | FR-012: copy to clipboard on drag release, like the terminal view? | No: copy only on request (copy key, context menu Copy, copy control); release never writes the clipboard | decided by user | user answer 2026-10-28 ("No, copy only on request"); spec.md FR-012 |

## Milestone notes

- M2 (agent-resolved): units are joined by what separates them on screen (`Joint`): a line break between lines and rows, a tab between table cells, a space between texts drawn on one line (list marker and item, a card's title and summary, a footer's status and figures). FR-010 makes only real line breaks into line breaks; research R2's "one `\n` whatever their kind" would put a marker and its item on two lines. `Unit` is a struct (`path`, `text`, `joint`, `source`) rather than the contract's `(TextPath, Cow<str>)` pair, and `TranscriptView` takes the window's `RichCache` (`rich`) and the insights rather than one entry's `Derived`.

- M2 red CI (#1590, `fmt + clippy`): main gained `EntryKind::EffortChanged` (#1554) after our merge, so `body_units`' exhaustive match broke. Merged origin/main (b18b611d1); the effort note copies nothing, matching the transcript, which draws an empty `Space` for it until #1554 M4 draws its marker (then give it that text). Test: `an_effort_note_copies_nothing`. The only other `EntryKind` matches in M1/M2 code have a `_` arm.

- M3 (agent-resolved): `Selectable` (micold-material) wraps the element each leaf already draws and only adds hit-testing over its own one-paragraph-per-hard-line `Measure`; with no range the child is the leaf's unchanged element, so T-4's pixel gate holds by construction, at the cost of shaping the text twice (once for drawing, once for hits; both cached). The highlight is the child's: spans split at the range ends drawn with a `primary_container` span background and `on_primary_container` ink (`selectable_text::highlight`). Code and diff lines are one `Selectable` per drawn line with a `base` (the line's start in the unit), measuring tabs-as-spaces and padding but answering in bytes of the unit's raw text; T-6 reports `Shown { slot, base, text }`. `InlineDiff::selectable(part, on_hit)` takes a `DiffPart -> (slot, range)` closure (path, captions, lines are separate units in M2), `CodeBlock::selectable(slot, range, on_hit)` one slot. The material crate has its own `Click` and `Edge` (the client's `Click`/`EdgeDir` sit above it; M4b maps them). The contrast gate's "visibly distinct" is CIE76 ΔE ≥ 10, not a luminance ratio: light `primary_container` is within 1.04–1.10:1 of the code surface and diff tints but ΔE-distinct by hue.

## Tasks-unit notes

- speckit-analyze (2026-10-28): 0 CRITICAL, 0 HIGH. Fixed C1 (stability test moved before the Selection impl), C2 (T018/T019 test and wiring split), C3 (state, SelectableText and entry-view tasks split), D1, U1 (spec FR-011 states the tab/line table rule of research R3), U2 (contrast gate also checks the highlight differs from each surface), U4 (spec Terms: image caption selectable), I2/I3 (plan Delivery *Satisfies* and file tree aligned with tasks). Left: U3, U5, I1, I4 (LOW).
- M2 has 11 tasks, kept whole as the plan's Delivery cuts it: its keys and copy tasks need the state tasks and have no deliverable of their own a user can observe.
- Checklist `requirements.md`: the one unchecked item ("No [NEEDS CLARIFICATION] markers remain") confirmed satisfied by the tasks reviewer and ticked; all items closed.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 14b551961ac69d5a737bde175d79033eaadcc4f0:828bc8e24442d9c9685869c35fe69134934bd233 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | 80048701cd16dc386ae196e8da6d4dd7a429becb:828bc8e24442d9c9685869c35fe69134934bd233 | CLEAN (2 MINOR fixed: rewrap, US2 scenario 6 for FR-019a) |
| Plan | 1 | 94244e6c1589e20bfeeeb532e3c9cd56beb33cee:783f9af1dc50a9469289c5e39f0e25d6a0ad62d2 | CHANGES: 2 MAJOR, 3 MINOR (all fixed: F1 RowPos row key, F2 M4 split into M4a/M4b, F3 retain_shown in shell/panes.rs::sync, F4 `focused` input to transcript_key, F5 EditDiff::path) |
| Plan | 2 | 3974e3a581dc83999537a082f2459b51cb3471c0:6e22abe4c36f2ceabc272035bb95f51647ae92db | CHANGES: 1 MAJOR, 2 MINOR (all fixed: TranscriptView defined, retain_shown session/Primary mapping, prompt rows inside bounds clear) |
| Plan | 3 | 57e895f188c87b43d22698f2f2c2c221b9539a4e:99112b227c7c36a8e9e407663569d56e06d8c413 | CLEAN (1 MINOR fixed, prose only: sessions from Workspace::session_ids_of_project) |
| Tasks | 1 | 91e2a3d9cf7b87ce83bcc7e5e0c7510556892560:12b46edb6afe9910d2dcd96175cf67c05102539a | CLEAN (3 MINOR fixed, prose only: checklist ticked, CopySelection stub noted in T016, M4b Verify records Part B 1–5 visual pass) |
| Review A M1 (code-review high) | 1 | no snapshot (HEAD 350eb1157) | 10 findings: 6 fixed in 75cbb5030 (word_at inside its line, snap via GraphemeCursor, full-width punctuation, unsnapped test offset, list marker arms merged, shared work_str), 4 declined (below) |
| Review B M1 | 1 | 55ede43b68b9972806d53205fc373e5003804172:75cbb5030640e4bb0c886ec779d3a90794010ddf | CLEAN (1 MINOR fixed: tdd/cycle-log.md written from recovered red evidence) |
| Review A M2 (code-review high) | 1 | 7ab3116b4a9b005b9490425ae1a0d2fbb1ce2c15:0c4b3eeccae6405f5dc9285e9a9edf579dfdc70d | 1 BLOCKER, 1 MAJOR, 8 MINOR: fixed BLOCKER (unfocused transcript listener heard a busy turn's Escape/Ctrl+C over dialogs and views: `transcript_keys(state, interrupt)` + `State::transcript_focused`), MAJOR (empty copy text no longer written), MINOR F6 (Show in full clears like a toggle), F7 (menu Copy/Select all refocus), F10 (T021/T023 wording, prose only); 5 MINOR declined (below). No re-review: no BLOCKER/MAJOR left open, fixes tested red→green |
| Review B M2 | 1 | 7ab3116b4a9b005b9490425ae1a0d2fbb1ce2c15:0c4b3eeccae6405f5dc9285e9a9edf579dfdc70d | CLEAN; Verify pass (28/40/18/2); 1 MINOR fixed, prose only (T021 focus.rs join named as T041's) |
| Review A M3 (code-review high, headless) | 1 | c1645efc0134e397e126b86897deccd6f8b41a91:14636211796c477bf6dc6dcc2d0f4feda20206a9 | CHANGES: 1 MAJOR, 3 MINOR: fixed F1 (empty span panicked `cut_at_selection`: `split_at` drops empty ranges; test red→green), F2 (`link_regions` takes the text's wrapping), F3 (selected `Text` measures with Advanced shaping, as its rich-text child draws); F4 declined (below) |
| Review A M3 (scoped, sonnet) | 2 | d371bebf0d185118d9af6c681062e861aae0da8a:4de2424fca9a9a44108a5c0ea28f5e30e29cf581 | CLEAN: F1–F3 fixes hold, F4's reason stands, the `wake_at` edge-scroll change is sound |
| Review B M3 (sonnet) | 1 | d371bebf0d185118d9af6c681062e861aae0da8a:4de2424fca9a9a44108a5c0ea28f5e30e29cf581 | CLEAN; Verify pass (material lib 441, client `ui::material` 648; showcase_completeness 46, material_builder_api 13 unfiltered) |
| Visual pass M3 (headless render) | 1 | 4de2424fc | pass: `visual/m3-visual-pass.md`, `visual/m3-selectable-leaves.png` |

## Declined review findings

- M1 gate: `scripts/gate.sh` green at 341ed42d1 (run with `CARGO_PROFILE_{DEV,TEST}_DEBUG=0 CARGO_INCREMENTAL=0`: the container's disk filled at 0 MB with the default profile). No visual pass: nothing visible changed (list markers draw the same glyphs).

- Review A M1 #1 (Selection does not snap; TextPos fields public): the contract fixes `pub` fields and `start(at, granularity, expand)` without the text; snapping happens where the text is known, `TextPos::in_text` (doc now says so). The hit mapper (M3) builds positions with it.
- Review A M1 #3 (`u16` path segments overflow past 65,535 lines): data-model.md fixes `Vec<u16>`; a code block or tool output is one unit (R2), so lines are not path segments; diffs are capped by `review::limits`.
- Review A M1 #5 (`in_row` re-computes `bounds()`): MINOR, a handful of small clones per row; revisit in M4a's budget test if it shows.
- Review A M1 #8 (S-5 test holds by construction): it checks that entry ids by key stay put while every recorded fixture streams (a transcript property, not the selection's) and that the selected text of a growing entry is unchanged; `copy_text` stability needs `units()`, which is M2's.
| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M2 | A F3 | `Press { clicks }` vs `Hit { click }` granularity | MINOR, contract shape for M4b's wiring: `armed` only records that the area saw the press; `Hit.click` is the gesture's count as the widget reports it. Revisit when M3/M4b wire the widget. |
| M2 | A F4, F5 | `noted`/`prompt_rows` per hit and per message; `placed()` per row | MINOR, performance: M4a's budget test (T033–T038) measures it and caches if it shows. |
| M2 | A F8 | empty plain-text unit for an empty underived entry | MINOR: units mirror what is drawn; whether the plain renderer draws an empty line is checked by M4a's parity test. |
| M2 | A F9 | `clear()` keeps `menu` | MINOR: the menu's Select all stays valid over a cleared selection and its Copy writes nothing; M4b's menu closes on outside clicks (`MenuDismissed`). |
| M3 | A F4 | `SelectionArea` and each `Selectable` count clicks separately: a second click in an adjacent unit within 300 ms gives `on_press(2)` but `Hit.click: Single` | MINOR, same question as M2 A F3: the contract gives both counts (`on_press(clicks)`, `Hit.click`); which one the model trusts is M4b's wiring (M2's `hit` could start from `armed`'s count). Noted in *Follow-ups not done*. |

## Handover

M4a unit 1 (2026-10-28) hit the 150k context cap during exploration; no code written yet. Done: M3 #1593 recorded. Next: implement T033–T038 from these findings (all verified by reading the code):

- **Wiring**: `RenderCtx` (blocks.rs) has 5 literal constructors (ui/terminal.rs:589, showcase prompts.rs:143, surfaces.rs:491/515, tests/support/transcript.rs:69); `BlockRef` 3 (entries.rs, permission.rs, tests/transcript_rich_renderers.rs). Plan: add `RenderCtx.selected: Option<&Selection>` (session-wide, terminal.rs passes `app.transcript_selection.selection(session)`), plus per-row `row: Option<RowPos>` and `selection: Option<&RowSelection>` set by mod.rs `build`; the entry `lazy` closure gets `&dep`, so put `selected.in_row(&RowPos::Entry(id, Under::Body))` in the lazy key tuple and borrow it in the closure. `row: None` = not selectable (subagent thread entries in entries.rs `thread_body`: they are not units, X3/X5 → keep them unselectable). The block's first path segment differs per call (assistant text 0, command output/compaction/thinking 2, tool input 2, tool result 3 in a card / 1 in an orphan) → pass it per block (e.g. `&RenderCtx { at: 2, ..*ctx }` or a BlockRef field).
- **Hit message**: `SelectMsg::Hit { session, pos: TextPos { row, path, offset: hit.offset }, click, line: Some(hit.line) }`; `State::hit` re-snaps with `TextPos::in_text` and ignores hits until `armed` (Press, M4b), so nothing selects before M4b. Map `micold_material::Click` → client `Click` (no From yet).
- **Leaves**: `.selectable(slot, range, on_hit)` exists on `Text`, `LinkText` (micold-material), `CodeBlock::selectable`, `InlineDiff::selectable(part_fn, on_hit)` (client ui/material). Not on `IconLabel` (footer status [0], TerminalPeriod title [0], TurnEnded/Interrupted [0] are IconLabels) nor `DisclosureCard`/`AccordionSection` (title via `Text`, summary via `Ellipsized`, summary not drawn when open) — those need a selectable hook added in micold-material.
- **Parity mismatches to resolve**: `Units::header` adds summary `[1]` even when the card is open, but an open `AccordionSection` draws no summary → drop `[1]` when open (features/transcript_selection.rs `header`, all callers pass `open`). Folded summary is `Ellipsized` (drawn cut, unit is whole): accept the whole text as the measured text.
- **MarkdownView** (ui/material/markdown_view.rs) has no selection API: add `.selection(prefix, Option<RowSelection>)`/`.on_hit(Fn(TextPath, Hit))`, path per R3 as `Units::blocks` builds it (block b → `[p,b]`; list item k marker `[..,k,0]`, item blocks `[..,k,1,..]`; quote children inline at `[..,b,i]`; table cell `[..,b,r,j]`), and add each settled block's clipped selection to its inner `lazy` key, or highlights go stale.
- **T-6 operation**: `micold_material::test_support::ShownTexts<K>` collects `(slot, base, text)`; code/diff lines report `base` per line, so the parity test must join a slot's lines.
- Budget test file: tests/transcript_open_budget.rs; layout_snapshot must stay unchanged.

## Open escalation

None.

## Follow-ups not done

- T021's join of `FieldId::Transcript` in `ui/focus.rs` is T041's (M4b; tasks.md now says so): `SelectionArea` placed in the transcript is the first thing that can take and report the transcript's focus; M2 adds the variant, the key arbitration, the subscription and the menu Copy's focus.
- `SelectMsg::Edge` changes no state in M2: the edge scroll is the shell's (M4b).
- M3 A F4: the area's `on_press(clicks)` and a unit's `Hit.click` are counted separately; M4b decides which the model trusts (M2's `hit` starts from `Hit.click`).
- M3 A F3 residual: a selected proportional `Text` is drawn as rich text (Advanced shaping, kerning) while an unselected one is the text widget (Auto shaping), so selecting kerned text can shift glyphs by subpixels, and in a rare case re-wrap a line. The measure follows the drawn child in both states. The visual pass of M4b/M5 should look for it.
- #944's subagent threads drawn inside entries (X3, X5) are not units: M4a's parity test decides whether they become selectable.
