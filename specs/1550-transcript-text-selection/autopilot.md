# Autopilot ledger — #1550 transcript-text-selection

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1550 (issue #1550: Structured view: text and values cannot be selected or copied); labels: bug, flow:bug (now bug, flow:feature, in-progress; switched from the bug flow, which returned NEXT: feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1550
- **Worktree branch**: claude/project-thread-zj503u
- **Started**: 2026-10-27
- **Phase**: M6
- **Next step**: M6 (T052–T055: split-pane copy key, diff anchors), then a new close unit (speckit-tdd-verify and speckit-converge were not run by the first close unit: M6 was already known, so they run once, after it)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1582 | design (spec, plan, tasks) | merged | 5ce410f219e38800d89f0936dbf9f48a96041cb1 |
| #1586 | M1: selection model, shown text and work_value in core | merged | f7c34e9d579bc65daea4a1bd7ca865d0ac9253ea |
| #1590 | M2: window selection state, copy text and copy keys | merged | 94519d3b85af68358495aa1eae645edf53174dc1 |
| #1593 | M3: SelectableText and SelectionArea components | merged | ffad78647cb67b85f478dc80617db267f9cdeddc |
| #1595 | M4a: selectable leaves in the transcript, parity and budget | merged | 9df69fa710e866a99c1f1d3c2d5b9027181f595c |
| #1597 | M4b: selection, edge scroll, menu and copy keys in the transcript | merged | 84bca0374abaa1b9891b471c0c63546cea6474f7 |
| #1601 | M5: copy controls, docs, visual pass | merged | c99e47aa18ac402d66ba30a06de62d367f62339c |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | Selection model, shown text and work_value in core | #1586 | merged |
| M2 | T013–T023 | full | Window selection state, clearing, copy text and copy-key arbitration (render-free) | #1590 | merged |
| M3 | T024–T032 | full | SelectableText and SelectionArea components, .selectable on four leaves | #1593 | merged |
| M4a | T033–T038 | full | Selectable leaves in the transcript, parity and budget (not user-visible) | #1595 | merged |
| M4b | T039–T043 | full | Selection, edge scroll, menu and copy keys in the transcript; user guide | #1597 | merged |
| M5 | T044–T049 | full | Copy controls, docs, visual pass | #1601 | merged |
| M6 | T052–T055 | full | Split-pane copy key and diff anchors (close unit's cut, from M5's visual pass) | — | in progress (handover) |

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

- M4b (agent-resolved): the selection area's click count (`SelectMsg::Press.clicks`, counted over the whole transcript) decides a new selection's granularity, not the unit's `Hit.click` (M3 A F4). `SelectMsg::Edge` while dragging becomes `Effect.scroll = Scroll::By(±EDGE_SCROLL_STEP)` (12 px a frame), performed by the shell as `scroll_by` on `transcript_scroll_id`; without a drag it scrolls nothing. A `Press` gives `FieldId::Transcript(session)` the keyboard; the area's own focus changes join the same field through `TrackFocusAs` (`ui/focus.rs`) when the view has prompts (the window's view always has). The context menu floats on the window overlay (`ui/mod.rs`, built by `ui::transcript::context_menu`), because the area reports the right press in window pixels; anchor clamped into the window like the other menus.

- M4b visual pass (real client, Xvfb, the daemon tests' `claude` stand-in replaying recorded fixtures): Part B 1–5 pass, and the folded summary highlight; record in `visual-pass/m4b-visual-pass.md`, copies in `/mnt/project-files/issue-1550/`. Only one interrupt was sent in the whole pass (the no-selection Ctrl+C), none from the copies.

- M4b gate: `scripts/gate.sh` green at 2787890f0 (run with `CARGO_PROFILE_{DEV,TEST}_DEBUG=0 CARGO_INCREMENTAL=0`). Its first two runs were red on `feature_registration_cost` (the menu surface names `TranscriptMsg`: allowed with its reason) and `popover_displacement` (the new popover's row: closes nothing, closed by nothing).

- M5 (agent-resolved): every tool card now has a `Derived` (`DerivedBody::Tool` for non-edit tools) so its copy controls' state lives in `Derived::copy` and the entry's lazy key picks up the stamp; `RichCache::copy(session, &Entry, CopyTarget)` computes the value with `copy_value` (pub). `TranscriptMsg::CopyCode` → `Copy { session, key, target }`; the transcript feature copies main-transcript entries, the root (`App::update_transcript`) copies a card in an open subagent thread (`Effect::copy`). All controls are `code_block::copy_control` (now `pub(crate)`): `ToolValue` in the header's trailing row (iff `work_value` is `Some`), `ToolInput`/`ToolResult` right-aligned above the input/result, `DiffPath` via `InlineDiff::copy_path`. The showcase's diffs show the idle control and the MultiEdit pose has a long path (layout fixture re-accepted). `transcript_rich` → `transcript_selection::shown::unshown_caption` allowed in `feature_registration_cost`.

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
| Review A M4a (code-review high, headless) | 1 | c075d239edb9fb5794a20938776e24b2e64ec5ca:5868b588e642776496bfb67358be5e2e420e8e09 | CLEAN: 3 MINOR, all declined or deferred (below); the five handover deviations raised nothing |
| Review B M4a (sonnet, headless) | 1 | c075d239edb9fb5794a20938776e24b2e64ec5ca:5868b588e642776496bfb67358be5e2e420e8e09 | CLEAN; Verify pass (parity 4, layout_snapshot 169, transcript_open_budget 6 release) |
| Review A M4b (code-review high) | 1 | 8ed12633c4299afbbc7528d8b38457dc748e0a4c:6b00d8dec2e6da6dc62eb39a23eecd4a0d0c7f5a | 2 real (fixed: F1 menu not a registered surface, Escape interrupted under it; F7 summary highlight could slice off a char boundary), F2 fixed in part (one menu per window), 5 declined MINOR |
| Review B M4b (conformance, sonnet) | 1 | 8ed12633c4299afbbc7528d8b38457dc748e0a4c:6b00d8dec2e6da6dc62eb39a23eecd4a0d0c7f5a | CHANGES: F1 MAJOR no M4b TDD log (fixed: cycle-log M4b with a mutant red run, 8 of 40 FAILED); F2 MINOR summary highlight not under a task (fixed, prose: noted under T041); F3 MINOR view wiring untested (visual pass covers it) |
| Review A M5 (code-review high, headless) | 1 | 5d2f55723c6022905be2569aded0638079b10e46:134dfd722ca191711f85359eca732cc52b48bb1f | CHANGES: F1 MAJOR header control `align_right(Fill)` halved the summary (fixed: bare control in the header, snapshot summary 804.3 of 834.3); F4 MINOR duplicated copy lookup (fixed: `entries::copy_of`); F2, F3 MINOR declined |
| Review A M5 (sonnet, fix diff) | 2 | 5d2f55723c6022905be2569aded0638079b10e46:134dfd722ca191711f85359eca732cc52b48bb1f → working tree | CLEAN |
| Review B M5 (conformance, sonnet, headless) | 1 | 5d2f55723c6022905be2569aded0638079b10e46:134dfd722ca191711f85359eca732cc52b48bb1f | CHANGES: F1 MAJOR the three tool-card controls untested through the view (fixed: two renderer tests, press/Enter/Space); F2 MAJOR root thread-copy arm untested (fixed: App-level test); F3 MINOR (fixed: mutant red runs logged). Verify: features_transcript_rich 24, layout_snapshot 171 passed. Fixes were tests only: no re-run needed beyond the scoped gate |
| Review A M6 (code-review high, headless) | 1 | efdf193fd0886ebb5faaab45c69715611a497247:92be93d4edf52c06dea6f501f1e629c5ffffdb7d | CHANGES: 1 MAJOR, 4 MINOR. F1 MAJOR fixed: `SelectionArea` swapped in the window position, off by the offset inside a scrollable (the showcase gallery); it now learns the scrollable's shift from each frame's redraw (its own batch in iced_winit), test `a_press_in_a_scrolled_area_lands_where_it_happened` (red without). F2–F5 MINOR, declined below |

## Declined review findings

- Review A M5 F2 (MINOR): `work_value` re-parses a card's input JSON on each rebuild of that card to decide the header control. A rebuild happens only when the entry's revision changes (cached elsewhere), and the summary needs the same parse; a `has_work_value` in core would change core's `ToolCard`, outside M5. Follow-up if a profile shows it.
- Review A M5 F3 (MINOR): `transcript_rich` → `transcript_selection::shown::unshown_caption` is allowlisted in `feature_registration_cost` with its reason: one caption shared by what is shown and what is copied, so they cannot drift.

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
| M4a | A F1 | `MarkdownView` holds a large code block back only with `on_show`, but drops its selectability on `large` alone | MINOR: the units side (`Units::markdown`, `derived.is_large`) also drops a large block on `large` alone, so view and copy agree (parity test); the transcript always sets `on_show`. |
| M4a | A F2 | `thread_body` draws subagent thread cards with `row: None` (not selectable, not copied) | MINOR, deliberate per the M4a parity decision on #944's threads (X3, X5); kept in *Follow-ups not done* for M4b/M5. |
| M4a | A F3 | `selectable_diff` rebuilds `diff_units` per draw for byte lengths | MINOR, performance: T034's slowest drag step is 27 ms against the 100 ms budget. |
| M4b | A F2 (rest) | the menu draws for whichever session holds it, not only the active one | MINOR: now one menu per window (opening one closes the other); a session switch while it is open is rare and the registered surface's dismissal rules apply. |
| M4b | A F3 | edge scroll is a fixed 12 px per frame, so its speed follows the frame rate | MINOR: frames are the area's own `motion::FRAME` wakes, not the display's refresh. |
| M4b | A F4 | `SelectMsg::Hit.click` is no longer read | MINOR: kept as the widget's report (contract shape); the doc says the press's count decides. |
| M4b | A F5 | edge gating split between root and `State` | fixed: `State::edge_scroll` holds the rule; the root maps it to `Scroll::By`. |
| M4b | A F6 | a release the area never sees leaves the drag on | MINOR: X11/winit deliver the release of a press begun in the window (implicit grab); a modal opening mid-drag is not a path the UI offers. |
| M4b | A F8 | `dragging` lags the press by one message | MINOR: the press's own hit (T-1) arrives in the same event as `on_press`, so the drag starts before any frame past the edge. |
| M6 | A F2 | only `SelectMsg::Press` focuses the pane; the menu's Copy/Select all in an unfocused pane leave `session.active` on the other | MINOR: the menu acts on the session it was opened in (M4b/M5), so its Copy copies the right pane; the copy keys follow the press that made the selection. |
| M6 | A F3 | a drag that ends right of the hunk's `sideways` viewport gets no cursor, so it does not reach the line's end | MINOR: the fix answers the end of a line for a pointer right of the text inside the viewport (the case the visual pass saw); past the viewport the area's clamp and the next move cover it. |
| M6 | A F4 | `state.pointer` can go stale when an ancestor or overlay takes moves | MINOR: the shift learned at each redraw absorbs a stale pointer (shift = given − pointer), so the next press without a move lands at the frame's cursor, as before M6. |
| M6 | A F5 | the batch-cursor fix lives in `SelectionArea` only, not in a root wrapper for every widget | MINOR, altitude: no other widget has shown the symptom; a root-level fix is out of #1550's scope. |

## Handover

M6 unit 1 (context over 150k after implementing). Done, committed locally (not pushed, no PR):
- 607bd17b8 T052–T053: `shell::transcript::update` focuses the pane showing the session on a
  `SelectMsg::Press` (new `shell::panes::focus_pane_showing`): the selection area captures the
  press, so the pane's `mouse_area` FocusPane never fired and `session.active` stayed the other
  pane's. `ui::composer::transcript_keys_session` split out of `transcript_keys` for the test.
  Test: `tests::panes::a_drag_in_either_pane_of_a_split_gives_that_pane_the_copy_key`
  (main_tests.rs; red on main for the reason: listener `None`).
- 8bfc401d8 T054 + T055 code: root cause of the diff anchor is not the scrollable: iced_winit
  hands every event of a batch `window.state.cursor()` (where the batch ended), so a fast drag's
  press landed where the drag got to. `SelectionArea` now tracks the pointer from `CursorMoved`
  and forwards each mouse event at its own place; `Selectable` answers a dragged pointer right of
  one of its lines with that line's end. Tests in `ui/material/inline_diff.rs`:
  `a_fast_drag_on_diff_lines_anchors_at_the_press`, `a_drag_right_of_a_short_diff_line_selects_to_its_end`
  (both red without the fix). contracts/selectable-text.md T-2/A-2 updated. T052–T054 ticked;
  T055 stays open until the visual pass is recorded.
- Checked green locally: `cargo test -p micold-material`, `-p micold-client --lib`, `--bin micold-ai-ide`.

Next: scoped gate (`scripts/autopilot/scoped-gate.sh`) with review A (high) in its shadow; review B
(conformance, sonnet); visual pass via the `visual-pass` skill in an autopilot-worker writing
`visual-pass/m6-visual-pass.md` (quickstart Part B steps 1 and 9: split-pane Ctrl+C in both panes,
a fast drag banana→fig on the edit-turn diff anchors at banana, a same-line drag right of a short
diff line selects to its end; screenshots also to /mnt/project-files/issue-1550/); tick T055;
full gate (`scripts/gate.sh`) after `git fetch origin && git merge origin/main`; push with
`git push -u origin claude/project-thread-zj503u`; PR body to the scratchpad path the orchestrator
gave (`pr-1550-m6-body.md`), title `#1550: Structured view: text and values cannot be selected or
copied (M6)`, `Refs #1550`.

## Open escalation

None.

## Follow-ups not done

- **Pre-existing, not #1550 (separate GitHub issue)**: *Tool card darkens cumulatively on hover and stays dark*. Repro (Linux, Xvfb + lavapipe): open a structured session, get a tool card (e.g. a Bash call), unfold it with its chevron, hover the chevron (or, from M5 on, a copy control): the whole card darkens with a translucent layer that gets darker on later frames and stays after the pointer leaves; texts drawn later show as light patches; fold+unfold resets it. The pre-M5 client (84bca0374) does the same (mean grey 0.853 → 0.772 → 0.727) and its sidebar went fully black the same way, so it predates M5 (suspect a state layer drawn without a clear, or lavapipe). Evidence: `visual-pass/m5-tool-card-darkening.png`, `visual-pass/m5-darkening-pre-m5.png`.
- **M5 visual pass, step 9 FAIL (M4b code, needs a fix in #1550; cut into M6, T052–T053)**: in a two-pane split of two structured sessions, Ctrl+C after a drag-selection copies nothing in either pane (menu Copy works; single pane works). Likely cause: `ui::composer::transcript_keys` subscribes only for `state.session.active` and requires `App::transcript_focused()` (`focused_field == Transcript(active)`), while `App::update_selection`'s press sets `focused_field = Transcript(pressed session)` without focusing that pane (`shell::panes::focus_pane` is what moves `session.active`). Fix: key the subscription off the focused transcript's session (or focus the pane on a transcript press), with an App-level test: two panes, press in the non-active pane, Ctrl+C copies it.
- **M5 visual pass, minor (M4 code; cut into M6, T054–T055)**: on a diff's lines the selection anchors where the drag first moves, not at the press (a fast drag from `banana` started at `fig`), and a same-line drag ending right of a short diff line's end selects nothing. Prose, code results and raw events anchor at the press. Suspect the diff's `sideways` scrollable taking the press.
- T021's join of `FieldId::Transcript` in `ui/focus.rs` is T041's (M4b; tasks.md now says so): `SelectionArea` placed in the transcript is the first thing that can take and report the transcript's focus; M2 adds the variant, the key arbitration, the subscription and the menu Copy's focus.
- `SelectMsg::Edge` changes no state in M2: the edge scroll is the shell's (M4b).
- M3 A F4: the area's `on_press(clicks)` and a unit's `Hit.click` are counted separately; M4b decides which the model trusts (M2's `hit` starts from `Hit.click`). **M4b: done**, the area's count (`Press.clicks`) decides the granularity (see M4b note).
- M3 A F3 residual: a selected proportional `Text` is drawn as rich text (Advanced shaping, kerning) while an unselected one is the text widget (Auto shaping), so selecting kerned text can shift glyphs by subpixels, and in a rare case re-wrap a line. The measure follows the drawn child in both states. The visual pass of M4b/M5 should look for it.
- #944's subagent threads drawn inside entries (X3, X5) are not units: M4a draws them with `row: None` (neither selectable nor copied; M4a Review A F2). M4b/M5 decide whether they become units. **M4b: left as they are** (a thread's rows would need their own `RowPos` and units; nothing in the spec's FRs names threads). Open for M5 or a follow-up issue. **M5: left for a separate issue** (spec FRs name no threads; a thread's rows need their own `RowPos` and units). Copy controls on a thread's cards do work.
- M4a: a folded card's summary (Ellipsized) answers hits but draws no highlight (M4b: done, `Ellipsized::highlighted` draws the shown part of the range, the ellipsis standing for a selected cut tail).
