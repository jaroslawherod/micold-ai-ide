# Autopilot ledger — #1270 review-bar-drawer

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1270 — issue #1270 "Review UI: review bar and comments drawer"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1270
- **Worktree branch**: claude/project-thread-ttad83
- **Started**: 2026-10-24
- **Phase**: implement
- **Next step**: M4 built on `m4-1270` (worktree /home/user/micold-m4) from M3's head 0c4e2d2d while #1402 (M3) runs CI; the orchestrator rebases it onto main after #1402 merges, re-records the layout snapshot, gates, pushes and opens M4's PR; then the close unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1356 | design: spec, clarify, plan, tasks | merged (rebase) | f15ecc59696cceeef6c4fca16b1b7eb7d8776b45 |
| #1369 | M1: core rules review::{bar,drawer}, reply::card_preview, anatomy tokens | merged (rebase) | dde943af17ba1dd8a8caadaa183357d47da43469 |
| #1382 | M2: ReviewBar, SideSheet, LineClamped::when_cut, row reply mode; cards show 4 reply lines | merged (rebase) | 150de7f6724e9021da4913a8b5eb68e1adbafae3 |
| #1402 | M3: reducer, shell and Esc wired to the per-entry review view | open, CI running | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T007, T017–T018, T023–T026, T044–T045 | full | core rules review::{bar,drawer}, reply::card_preview, anatomy tokens, unit-tested (Layer A) | #1369 | merged; full gate green at e0c0e828; reviews A, B clean |
| M2 | T008–T010, T027–T031, T046–T052 | full | ReviewBar, SideSheet, LineClamped::when_cut, ReviewPanelRow reply mode; cards show 4 reply lines (Layer A) | #1382 | merged; full gate green at 0b8e6365; reviews A, B clean; visual pass recorded |
| M3 | T011–T014, T019–T021, T032–T040 | full | reducer, shell, Esc wired to per-entry view; old panel still drawn (Layer B; #1269 closed, blocker cleared) | #1402 (open, CI running) | full gate green at 5358a2b7; reviews A (r2), B clean; visual pass recorded (`visual-pass-m3.md`) |
| M4 | T015–T016, T022, T041–T043, T053–T057 | full | Review tab bar + drawer layout, geometry gate, snapshot, guide, visual pass (Layer B; #1269 closed, blocker cleared) | — (branch `m4-1270`, not pushed; PR title/body in the scratchpad `pr-title-m4.md`, `pr-body-m4.md`) | full gate green at 8b16f5e3 (split gate: fmt, clippy core + workspace, every package's tests, root-only tests, scripts/tests); reviews A (r3 CLEAN), B (r2 CLEAN); visual pass recorded (`visual-pass-m4.md`); not rebased |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | How does the spec depend on #1269 (not merged)? | Only through its contract: each entry has a Review tab whose view state survives switching; drawer state belongs to that view (FR-019, Assumptions) | agent-resolved | prompt; docs/development/review-redesign.md#tasks (R0 row) |
| D2 | spec | Where do send outcomes show? | Success keeps today's snackbar; a failure also stays in the bar until next send or dismissed (FR-006) | agent-resolved | review-redesign.md item 4 ("also shows Sending… and the send error"); features/changes.rs sent_text/send_error_text (snackbars today) |
| D3 | spec | Bar counts: all statuses or non-zero only? | Non-zero only; "No review comments yet" when none | agent-resolved | spec Edge Cases |
| D4 | spec | Drawer tabs' content | Drafts = Draft; Replies = Sent, Applied, Answered (Mark applied / Answer… on Sent rows) | agent-resolved | review-redesign.md item 5 |
| D5 | spec | Pre-execution step skipped | branch-start.sh and issue.sh skipped: gh unauthenticated, branch already at origin/main 3a333f7f | orchestrator | prompt |
| D6 | clarify | How much of a reply does a card show? | At most the first 4 lines as drawn at the card's width, then Show all; drawer Replies rows keep the whole reply (4,000-char cap) (FR-021, US3 #2) | agent-resolved | review-redesign.md#layout items 5, 7 |
| D7 | clarify | Does the drawer open by itself? | Yes, on Replies at the first report of a send started in this window, once per send, only while the entry's Review tab is shown and not closed since the send; open drawer keeps its tab (FR-020) | agent-resolved | review-redesign.md#layout item 5; FR-019; Edge Cases "Two windows" |
| D8 | clarify | Is the auto-open rule tested without drawing? | Yes, added to FR-023 | agent-resolved | constitution I; FR-023 |
| D9 | plan | How is the work split around #1269 (not merged)? | Layer A (M1 core rules in review::{bar,drawer,reply}, M2 shared components) independent of #1269; Layer B (M3 reducer/shell/Esc wiring, M4 layout, snapshot re-record, docs, visual pass) after #1269 merges; building B on today's State.open is a fallback needing a ledger decision | agent-resolved | prompt; plan.md Delivery order; research R11 |

| D10 | tasks | Milestone sizes: M1, M2 have 15 tasks, M3 16, over rule 3's ~10 | Kept per plan D9 (orchestrator: keep the layer split); M1/M2 are pure core / self-contained components, M3 is one reducer change in features/changes.rs that cannot ship half (Panel → ReviewDrawer) | agent-resolved | prompt; plan.md Delivery order |
| D12 | tasks | Checklist item "No [NEEDS CLARIFICATION] markers remain" | Ticked: clarify resolved both markers; tasks reviewer r1 found no checklist finding | agent-resolved | grep finds 0 markers in spec.md |
| D11 | tasks | speckit-analyze HIGH F1 (T015 gate needs drawer), C1 (T034/T042 too large), MEDIUM I1, A1, U1–U3, T1 | Fixed: drawer case moved to T041; reducer and layout tasks split; spec edge "Drawer opens over the line" and SC-004 reworded (plan test-strategy row follows); files-browser, 2 s and line-start checks added; T013/T014 split | agent-resolved | scratchpad analyze.md |
| D14 | M2 | Where do the M2 component tests and helpers live, given tasks.md's paths predate feature 1262's crate split? | `LineClamped` is in `crates/micold-material/src/line_clamp.rs`, so T046/T047 are there; `ReviewBar`, `SideSheet`, card and row stay in `crates/micold-client/src/ui/material/`. One-line text width is the new shared `micold_material::shaped_text::width_of`, which `StatusBar` now uses too (no copy) | agent-resolved | `grep -rn "struct LineClamped"` → micold-material |
| D15 | M2 | `SEND_WIDTH` value | 169: the widest of "Send review (999+)", "Send review (999)", "Sending…" as a filled `Button` lays it out (169.0 measured), rounded up; asserted by `the_widest_label_fits_the_send_width` | agent-resolved | T009 red run: `left: 176.0 right: 169.0` |
| D16 | M2 | Error vs counts when the bar is narrow | The error is kept whole while it fits and the counts shorten first (Full → DraftsOnly → hidden) against the room left of it; the error is ellipsized only once the counts are gone. ⋮ and the error's × are compact icon buttons (the bar is 44 high, under the 48 dp target, as `ui/changes.rs` already does) | agent-resolved | spec Edge Cases "Narrow window"; research R3 |
| D17 | M2 | Does M2 need #1269's per-entry review view? | No: every M2 task is a self-contained component, showcase pose or doc; nothing reads view state | agent-resolved | tasks T008–T010, T027–T031, T046–T052 |
| D13 | M1 | The three new anatomy figures reach no client component until M2/M4, and `anatomy_call_sites` fails on an unbound, unrecorded figure | Recorded them as `NOT_BUILT` gaps in `crates/micold-client/tests/anatomy_call_sites.rs`; M2 (`ReviewBar`) and M4 (drawer layout) remove the entries, which `a_recorded_gap_that_became_bound_is_stale` enforces | agent-resolved | plan missed the client gate; anatomy_call_sites.rs RECORDED |
| D18 | M3 | Where is "#1269's per-entry view" that holds the drawer, send error and overflow state? | `OpenView`, the `V` of `ReviewTabs<OpenView>` (`State.tabs`, keyed by `(project, location)`): `panel: Panel` became `drawer: ReviewDrawer`, plus `send_error`, `overflow_open`. A message for a hidden view (`SendFailed`, `ReviewChanged`) finds its tab by project and wire worktree dir (`view_of_entry`) | agent-resolved | specs/1269-review-tab data-model; `features/changes.rs` `State.tabs` |
| D19 | M3 | The old panel draws the drawer's state until M4, but `Drawer::default()` is closed (FR-011) | The panel is closed when a Review tab is made and opens and closes with **Review (n)** (now `Drawer(Toggle)`), and Esc closes it; it shows the drawer's selected tab, with two chips "Drafts n" / "Replies n" (`Drawer(SelectTab)`) so per-tab rows, scroll and auto-open on Replies are reachable before M4. Covered states that show a Review tab lose the panel column (layout snapshot re-recorded in M3) | agent-resolved | FR-011, T034 ("the old panel reads the drawer state"); plan.md M3 |
| D20 | M3 | `Msg::Drawer(Scrolled)` replaces `PanelScrolled`, but the row list also reports its viewport | `Msg::DrawerScrolled { tab, offset, viewport }` applies `DrawerEvent::Scrolled` and stores `ReviewDrawer.viewport`; `Msg::Drawer` carries the other events | agent-resolved | data-model `ReviewDrawer.viewport`; `VirtualRows::on_scroll(offset, viewport)` |
| D21 | M3 | How does a composer get `ComposerPlace::Drawer`? | `Msg::DrawerEdit(id)` / `Msg::DrawerAnswer(id)` (same rules as `EditComment` / `AnswerComment`, which keep `Card`); not drawn in a row until M4 | agent-resolved | contracts D6, D7; T034 |
| D22 | M3 | Is the overflow menu displaced by, or does it displace, other popovers? | Neither (`popover_displacement.rs` row `review_overflow: []`); `OverflowToggled`/`OverflowDismissed` are also pure state in `app::State::update`, so the registry's dismissals (outside click, scroll beneath, a dialog opening) reach it | agent-resolved | research R8; `app.rs` DiscardCancelled arm |
| D23 | M3 | Esc order between the Overview and the drawer | Registry, Settings draft, expanded card, Overview, then the drawer (last, R8); a closed drawer leaves Esc unused (`None`) | agent-resolved | research R8; `app::on_escape` |
| D25 | M4 | How does the Review tab bind `anatomy::comments_drawer::WIDTH` (D13), when `SideSheet` read it only through core's `drawer::width`? | `SideSheet::width(w)` (the width its call site wants, capped at half the area by core's new `drawer::capped`; `drawer::width` is `capped(WIDTH, area)`); the drawer layout passes `anatomy::comments_drawer::WIDTH`, so the figure reaches `ui/changes.rs` and the `NOT_BUILT` entry is gone | agent-resolved | `anatomy_call_sites.rs` `bindings` scans for the figure's name |
| D26 | M4 | `VirtualRows` has one row height, but a Replies row draws the whole reply and a row can hold a composer | The drawer's rows are a plain `Scrollable` column reporting `DrawerScrolled` per tab; an entry's comments are few enough to build every row | agent-resolved | contract D7 (whole reply), D6 (composer in the row) |
| D27 | M4 | Where does the overflow menu hang, with no press reporting the ⋮'s bounds? | From the ⋮'s top-right, computed from the window size: the bar's right edge is the window's and its bottom the status bar's top (FR-001), and the trailing group has fixed sizes (`review_bar::overflow_trigger_right`, unit-tested against the laid-out ⋮); the panel rises above the bar | agent-resolved | `overflow` covered state: panel 847–1087 × 616–728 over a bar at y 728 |
| D28 | M4 | Diff and file list widths once the review column is gone (SC-004) | The list keeps its old share (2 of 2 + 3 + 2) and the diff area takes the rest (5 of 7), reaching the tab's right edge and the bar; the drawer is stacked over it, the stack kept while closed so the diff's widget state survives a toggle | agent-resolved | `review_bar_fixed` SC-004 case: area ≥ 5/7 of the old padded width |
| D24 | M3 | A connection lost while a send is pending (review B M3 F2) | Left as today: the outcome is unknown (the generic "may or may not have taken effect" notice), so no bar error; the watch stays, and a report of that send still opens the drawer once, as it would had the send gone through | agent-resolved | `shell/daemon_sync.rs` disconnect path; FR-006 names a failed send only |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | fd089e07fa581ec092d2c314ae1f7f1a1ddc28a9:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 1 MAJOR (declined), 3 MINOR (fixed) |
| Plan | 1 | 5f5fd06c9c015ce43b1c2b6043648cfdbcdfd036:eac77a56e214e2eccdb0fb472fca37e75f1274fa | CLEAN: 1 MINOR (fixed, prose only) |
| Tasks | 1 | 42a7804ed6fa03cda55289e1422ccf12057a0998:41be4d60561ee8c3d899e38224a6e17ac52eafd3 | CLEAN: 3 MINOR (F2 fixed: focus/dialog cleanup named in T037; F1, F3 task size left, D10) |
| Review A M1 (code-review high) | 1 | f9a89ea1e47eb2cbbd913fbc60a375822d8a5c1c:0150d727dfd87b94372093b4ab921f5a7101e7c8 | CLEAN: 7 MINOR (2 fixed: card_preview reuses `shortened`, `Drawer::apply` no longer clones per event; 5 declined) |
| Review B M1 (conformance, sonnet) | 1 | b6012172e220e4d9d93f952787685df926024fff:e0c0e82856af85e3d012a459d4ebc13200983d46 | CLEAN |
| Review A M2 (code review high, headless reviewer) | 1 | 8c92d360526b5d06c0d85ae3cfa2941ce96b91ea:68c5b3fe6ec8cee326941591b531e4f411b34580 | CLEAN: 3 MINOR, all fixed (drafts-only test asserts `DraftsOnly` is `None`; card test helpers deduplicated; catalogue pose names match the render labels). A first run returned a bare CLEAN; re-run with JSON output (12 turns) to confirm it read the diff |
| Review B M2 (conformance, headless reviewer) | 1 | 8c92d360526b5d06c0d85ae3cfa2941ce96b91ea:68c5b3fe6ec8cee326941591b531e4f411b34580 | CLEAN |
| Visual pass M2 (in-process tiny-skia, 1280 wide, both schemes) | 1 | — | 1 real finding fixed: Answer… overflowed a 340 px whole-reply row; Mark applied / Answer… moved to their own line. Record: `visual-pass-m2.md` |
| Review A M3 (code review high, headless reviewer) | 1 | 28fbd07040ff3c5c74adc97695d603af7a90c0e3:75b576df73af295f0de414be1c74aa9a1c5dd45c | CHANGES: F1 MAJOR fixed (a send that never left — no daemon, route down — now also fails in the bar and drops the watch: `shell/changes.rs` uses `send_op_bound_req`; test `a_send_with_no_daemon_fails_in_the_bar_and_drops_the_watch`); F2 MINOR fixed (the error clears only when a send starts); F3 MINOR fixed (the overflow menu closes when the tab hides and when a send starts) |
| Review A M3 (scoped, sonnet, headless) | 2 | 9505ca1c084428ed4c4d85434218a2d50d5f49a1:5358a2b738b38a662427bc2c18fe3fb4cc6b93d9 | CLEAN (fix diff of r1; re-run with JSON output: 4 turns) |
| Review B M3 (conformance, sonnet, headless) | 1 | 9505ca1c084428ed4c4d85434218a2d50d5f49a1:5358a2b738b38a662427bc2c18fe3fb4cc6b93d9 | CLEAN: 2 MINOR — F1 fixed, prose only (cycle-log names the third `review_send` test, not run red); F2 recorded as D24 |
| Visual pass M3 (in-process tiny-skia, 1280×800, both schemes) | 1 | — | 1 finding fixed: "No review comments yet" said twice in an empty panel. Record: `visual-pass-m3.md` |
| Visual pass M4 (in-process tiny-skia, 1280×800 and 640×480, both schemes) | 1 | — | 1 finding fixed: drawer rows ran under the list's scrollbar. Record: `visual-pass-m4.md` |
| Review A M4 (code review high, headless, diff inline) | 1 | 2297e2fabf3bfb8d73d667925ef6948621f0d213:612524e931fad5e60f0b5d204d0b1334268f648a | CHANGES: F1 MAJOR fixed (a drawer composer was drawn nowhere while the drawer was closed or on the other tab; now on its card then, `composer_in_drawer`, test); F2 MINOR fixed (overflow items built once) |
| Review B M4 (conformance, sonnet, headless, diff inline) | 1 | 2297e2fabf3bfb8d73d667925ef6948621f0d213:612524e931fad5e60f0b5d204d0b1334268f648a | CHANGES: F1 MAJOR fixed (the drawer's scroll was kept but never restored: `Effect::ScrollDrawer` when its list comes into view, test); F2 MINOR fixed, prose only (visual-pass-m4.md names where steps 4, 5, 7–9 are checked and how to regenerate); F3 MINOR = A F2 |
| Review A M4 (scoped, sonnet, headless) | 2 | 2d4a173b876a71cc1210bdfdd42d2af8c0949533:75b826c75bba3fb940b4e5c34c768c76b7212b60 | CHANGES: F1 MAJOR fixed (a drawer composer of a comment in a file other than the selected one had no card to show on: a row's Edit / Answer… now jumps to the comment first, test `edit_or_answer_in_a_drawer_row_selects_the_comments_file`); F2 MINOR declined (an empty list's offset is 0) |
| Review A M4 (scoped, sonnet, headless) | 3 | 9a586d0a5a5b1e5e3bbc605251976eee92c0ded2:8b16f5e3786a411ff8dafdd53674b1349570b48d | CLEAN: 1 MINOR left (a refused jump — file hidden by a toggle or not in the diff — still opens the drawer composer; its text is kept and is back in the row when the drawer reopens). Recorded in *Follow-ups not done* |
| Review B M4 (scoped, sonnet, headless) | 2 | 2d4a173b876a71cc1210bdfdd42d2af8c0949533:75b826c75bba3fb940b4e5c34c768c76b7212b60 | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — | Spec r1 | F1 MAJOR: Clear sent / Discard drafts act on the whole entry today, not the shown diff | Wrong: today they act on what is shown (reviewing-changes.md "Commenting on a pull request": "act only on the comments of what is shown"; features/changes.rs discard_prompt and status_counts use in_scope(view_scope)). The older "Clearing and discarding" wording is stale; added Story 2 scenario 5 to make the rule explicit |
| M1 | Review A r1 | MINOR: a report while the Review tab is not shown consumes the watch | Specified: T024 and data-model `Reported` ("`shown == false` consumes the watch"), FR-020 "only while this entry's Review tab is shown" |
| M1 | Review A r1 | MINOR: `SEND_WIDTH = 176` is unmeasured | Provisional by design; T009 (M2) measures the widest label and sets it |
| M1 | Review A r1 | MINOR: `SendButton::primary` duplicates `enabled` | Data model and contract B4 name the field; kept for the component's API |
| M1 | Review A r1 | MINOR: Close/Escape on a closed drawer keep a pending watch | Intended: closing an already closed drawer is not a close; Esc must stay unused (returns false); pinned by `escape_on_a_closed_drawer_keeps_a_pending_watch` |
| M4 | Review A r2 | MINOR: `ScrollDrawer` is not re-sent when the list first appears in an already open drawer | The list appears only when the tab gains its first row, and an empty list's offset is 0, which a new `Scrollable` already shows |
| M1 | Review A r1 | MINOR: a reply ending in `\n` measures an extra line | A stored reply is trimmed (`Report::reply`), so it never ends in a newline |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- A drawer row's Edit / Answer… on a comment whose jump is refused (file hidden by Committed/Uncommitted, or not in the diff) opens the composer with no card to show it on while the drawer is closed; the text is kept and reappears in the row on reopen (review A M4 r3 MINOR).
- docs/user-guide/reviewing-changes.md "Clearing and discarding" says Clear sent / Discard drafts act on "the entry"; they act on what is shown (PR or working tree). The R1 docs rewrite (FR-024) should fix it.
