---

description: "Task list for feature 1270: review bar and comments drawer"
---

# Tasks: Review bar and comments drawer

**Input**: Design documents from `specs/1270-review-bar-drawer/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), every behaviour task has a
test task before it, written and seen failing first. The core rules of the new modules are tested
in `crates/micold-core/tests/review_bar.rs` and `crates/micold-core/tests/review_drawer.rs`
(integration tests against the public API); `card_preview` in the existing `#[cfg(test)]` module of
`crates/micold-core/src/review/reply.rs`; components in their own in-crate test modules. The GUI
exception covers only the composition glue in `crates/micold-client/src/ui/changes.rs`, backed by
the `review_bar_fixed` geometry gate, the layout snapshot and quickstart §B.

**Layers (plan D9, research R11)**: **Layer A** (Phases 1–2 and every task marked *Layer A*) is
pure `micold-core` and shared `ui/material/` components; it touches no file #1269 changes and can
merge any time. **Layer B** (tasks marked *Layer B*) edits `features/changes.rs`, `ui/changes.rs`,
`shell/`, `app.rs` and `overlay/`, and attaches to the per-entry review view that #1269 introduces:
**it is blocked until #1269 merges**. Building Layer B on today's single `State.open` is a fallback
that needs a ledger decision first.

**Documentation**: Per Constitution Principle VII, each milestone that changes what a user sees
updates `docs/user-guide/reviewing-changes.md` in the same PR (FR-024).

**Cross-platform**: no OS branch; the bar, menu and drawer are the app's own widgets, Esc goes
through the shared `on_escape` (Principle VI). Quickstart §C repeats the interactive steps on macOS
and Windows.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1–US4 from spec.md

---

## Phase 1: Setup

**Purpose**: the two new core modules exist, so every later test file compiles against them.

- [ ] T001 *Layer A* Create `crates/micold-core/src/review/bar.rs` and `crates/micold-core/src/review/drawer.rs` with module headers (feature 1270, links to `specs/1270-review-bar-drawer/data-model.md`), declare `pub mod bar; pub mod drawer;` in `crates/micold-core/src/review/mod.rs`, and create the empty test files `crates/micold-core/tests/review_bar.rs` and `crates/micold-core/tests/review_drawer.rs`

---

## Phase 2: Foundational

**Purpose**: the sizes every later component and rule reads.

- [ ] T002 *Layer A* Red: in `crates/micold-core/tests/tokens_anatomy.rs` assert `anatomy::review_bar::HEIGHT == 44.0`, `anatomy::comments_drawer::WIDTH == 360.0`, `anatomy::review_bar::SEND_WIDTH > 0.0`, and that all three appear in the anatomy table the module's property tests enumerate
- [ ] T003 *Layer A* Add `pub mod review_bar { HEIGHT = 44.0, SEND_WIDTH }` and `pub mod comments_drawer { WIDTH = 360.0 }` to `crates/micold-core/src/tokens/anatomy.rs` and to its anatomy table; `SEND_WIDTH` gets a provisional value, fixed by T009's measurement (data-model "Core: tokens::anatomy")

**Checkpoint**: `mise run test-core` green.

---

## Phase 3: User Story 1 — Send a review from one fixed place (Priority: P1) 🎯 MVP

**Goal**: a fixed 44 px bar above the status bar shows the shown diff's counts and **Send review
(n)** at a position that never moves; a failed send stays in the bar.

**Independent test**: open a Review tab, add drafts, send; the bar's counts and button follow, the
button's bounds equal across drawer, counts and error states (SC-001); quickstart §B steps 1, 2, 5,
10.

### Layer A — core rules and the bar component

- [ ] T004 [P] [US1] *Layer A* Red: in `crates/micold-core/tests/review_bar.rs` test `ScopeCounts::of` counts only the given scope's comments (working tree vs a pull request, Story 2 #5), `is_empty`, `reported_or_sent() = sent + applied + answered`; `counts_text`: `Full` all zero → `"No review comments yet"`; `Full` non-zero → only non-zero statuses in order Draft, Sent, Applied, Answered joined by `" · "` (`"3 drafts · 2 sent · 1 applied · 1 answered"`), `"1 draft"` singular, sent/applied/answered not inflected; `DraftsOnly` → `"3 drafts"`/`"1 draft"`, `None` when `draft == 0`; `Hidden` → `None` (FR-002, R3, contract B2)
- [ ] T005 [US1] *Layer A* Red: in `crates/micold-core/tests/review_bar.rs` test `send_button(counts, sending, sendable)`: 0 drafts → `"Send review (0)"` disabled; n drafts sendable → `"Send review (n)"` enabled; 1,000 drafts → `"Send review (999+)"`; `sending` → `"Sending…"` disabled whatever the counts; `sendable == false` (pull request not in the list) → disabled; `primary == enabled` in every case (FR-003, FR-005, R4, contract B4); and `error_text("why") == "Couldn't send the review: why"` (FR-006)
- [ ] T006 [US1] *Layer A* Implement `ScopeCounts` (`of`, `is_empty`, `reported_or_sent`), `CountsForm { Full, DraftsOnly, Hidden }` and `counts_text` in `crates/micold-core/src/review/bar.rs` until T004 passes
- [ ] T007 [US1] *Layer A* Implement `SendButton { label, enabled, primary }`, `send_button` and `error_text` in `crates/micold-core/src/review/bar.rs` until T005 passes
- [ ] T008 [US1] *Layer A* Red: in the `#[cfg(test)]` module of new `crates/micold-client/src/ui/material/review_bar.rs` test `ReviewBar` layout: at three widths it draws the longest `CountsForm` that fits (Full, then DraftsOnly, then Hidden); the trailing group (Send review at `SEND_WIDTH`, ⋮ trigger, **Comments** toggle, right to left) has identical bounds across labels `"Send review (0)"`, `"Send review (999+)"`, `"Sending…"`, any counts, error on/off and drawer flag on/off; the widest label fits `SEND_WIDTH`; the bar is `review_bar::HEIGHT` high; the error's × emits its message (contract B2, B3)
- [ ] T009 [US1] *Layer A* Implement `ReviewBar` in `crates/micold-client/src/ui/material/review_bar.rs` (builder ending in `.into()`, existing roles only: surface container, outline divider on top, contract B10), export it in `crates/micold-client/src/ui/material/mod.rs`, and set `anatomy::review_bar::SEND_WIDTH` in `crates/micold-core/src/tokens/anatomy.rs` to the measured width of the widest label, until T008 passes
- [ ] T010 [US1] *Layer A* Add a `review_bar` showcase entry (poses: no comments, drafts, sending, error shown, narrow width with counts hidden) in `crates/micold-client/src/showcase/sections/review.rs` and register it in `crates/micold-client/src/showcase/catalogue.rs`

### Layer B — reducer and shell (blocked until #1269 merges)

- [ ] T011 [US1] *Layer B* Red: in `crates/micold-client/tests/features_changes.rs` test `review_bar(state)`: `None` while a session tab is shown (FR-001, B1); for the shown Review tab, counts of the shown scope only and `SendButton` from core incl. "Sending…" while the entry is sending and disabled for a pull request not in the list (B4); `SendPressed` emits today's `Effect::ReviewSend` unchanged in content (FR-004), and clears `send_error` (B5; the drawer's send watch is started by T039); `SendFailed { project, worktree_dir, message }` sets `send_error` to `"Couldn't send the review: <message>"` on that entry's view only, and the next `SendPressed` or `SendErrorDismissed` clears it (FR-006, B6); while the files browser is shown the bar keeps the scope of the source it was opened from (Edge Cases "Files browser shown"); existing send tests stay green
- [ ] T012 [US1] *Layer B* In `crates/micold-client/src/features/changes.rs` add `ReviewBarModel { counts, send, overflow, error, drawer_open }` and `review_bar(state)` (`drawer_open` read from the view's panel open flag until T034 moves it into `ReviewDrawer`), the per-entry view's `send_error: Option<String>`, `Msg::SendFailed` and `Msg::SendErrorDismissed`, the `SendPressed` changes, make `send_error_text` delegate to `micold_core::review::bar::error_text`, and replace `send_action`, `status_counts` and `review_toolbar_label` by `review_bar` (moving their callers in `crates/micold-client/src/ui/changes.rs` to it so the window still draws today's panel), until T011 passes
- [ ] T013 [US1] *Layer B* Red: in the `#[cfg(test)]` module of `crates/micold-client/src/shell/daemon_sync.rs` test that a failed `ReviewSend` answer yields `Msg::SendFailed` for its entry (`project`, `worktree_dir`) as well as today's error snackbar, and that a delivered send keeps today's success snackbar wording
- [ ] T014 [US1] *Layer B* Give `PendingOp::ReviewSend` the entry key (`project`, `worktree_dir`) in `crates/micold-client/src/shell/changes.rs` and route the failure to `Msg::SendFailed` in `crates/micold-client/src/shell/daemon_sync.rs` (R10), until T013 passes

### Layer B — Review tab layout (blocked until #1269 merges)

- [ ] T015 [US1] *Layer B* Red: create `crates/micold-client/tests/gates/review_bar_fixed.rs` and register it in `crates/micold-client/tests/layout_snapshot.rs` with `#[path = "gates/review_bar_fixed.rs"] mod review_bar_fixed;`; assert SC-001's drawer-independent cases: **Send review (n)**'s bounds are equal across 0 and 99 drafts, "Sending…" and error on/off, and the bar is `review_bar::HEIGHT` high, directly above the status bar at full tab width (the drawer-open case is added by T041)
- [ ] T016 [US1] *Layer B* In `crates/micold-client/src/ui/changes.rs` remove **Review (n)** and **Send review** from the Changes header (FR-009, B9) and draw `ReviewBar` from `review_bar(state)` as the last row of the Review tab's body, wiring Send, the error's × (`SendErrorDismissed`) and **Comments** (`Drawer(DrawerEvent::Toggle)`, B8), until T015 passes

**Checkpoint**: US1 core (M1), bar component (M2), wiring (M3) and layout (M4) each green on their own.

---

## Phase 4: User Story 2 — Tidy comments from the bar's overflow (Priority: P1)

**Goal**: **Clear sent** and **Discard drafts…** live in the bar's ⋮ menu, enabled by the shown
diff's counts; Esc or an outside click closes the menu and changes nothing.

**Independent test**: with drafts and sent comments, open ⋮, check each item's availability, clear,
discard, and close the menu with Esc; quickstart §B step 3.

### Layer A — core rule

- [ ] T017 [US2] *Layer A* Red: in `crates/micold-core/tests/review_bar.rs` test `overflow(counts, sending)`: `clear_sent` true when any of sent, applied or answered is non-zero (incl. only applied, only answered) and false otherwise; `discard_drafts` true only when `draft > 0 && !sending` (FR-007, R2, contract B7)
- [ ] T018 [US2] *Layer A* Implement `Overflow { clear_sent, discard_drafts }` and `overflow` in `crates/micold-core/src/review/bar.rs` until T017 passes

### Layer B — reducer and overlay (blocked until #1269 merges)

- [ ] T019 [US2] *Layer B* Red: in `crates/micold-client/tests/features_changes.rs` test `OverflowToggled` opens and closes the view's `overflow_open`; the Clear sent item emits today's clear for the shown scope and closes the menu; Discard drafts… opens today's confirmation; `OverflowDismissed` closes the menu and changes nothing else (Story 2 #1–#4); `review_bar(state).overflow` follows `overflow` from core
- [ ] T020 [P] [US2] *Layer B* Red: in `crates/micold-client/tests/overlay_registry.rs` test that `ReviewOverflowMenu` is registered as a popover surface and that an outside click or Esc dismisses it before anything else (R8)
- [ ] T021 [US2] *Layer B* Implement `overflow_open`, `Msg::OverflowToggled`, `Msg::OverflowDismissed` and the `ReviewOverflowMenu` surface in `crates/micold-client/src/features/changes.rs`, register it in `crates/micold-client/src/overlay/registry.rs`, and replace `tidy_actions` by `review_bar(state).overflow` (moving its callers in `crates/micold-client/src/ui/changes.rs`), until T019 and T020 pass

### Layer B — layout (blocked until #1269 merges)

- [ ] T022 [US2] *Layer B* In `crates/micold-client/src/ui/changes.rs` draw the overflow menu with the app's `MenuOverlay`, anchored to the bar's ⋮ `MenuTrigger`, items enabled per `Overflow`

**Checkpoint**: US2 core (M1), reducer (M3), menu drawn (M4).

---

## Phase 5: User Story 3 — Read drafts and replies in a drawer over the diff (Priority: P2)

**Goal**: the right-hand review column becomes a drawer stacked over the diff, with **Drafts** and
**Replies** tabs, closed by default, closed by Esc, × or **Comments**, and opening itself on Replies
at the first report of a send this window started.

**Independent test**: toggle the drawer; the diff's bounds and first line do not move (SC-002);
every comment is in exactly one tab (SC-006); switch tabs and entries and come back; quickstart §B
steps 4, 6–9, 11.

### Layer A — core rules

- [ ] T023 [P] [US3] *Layer A* Red: in `crates/micold-core/tests/review_drawer.rs` test `DrawerTab::of` (Draft → Drafts; Sent, Applied, Answered → Replies), `label` (`"Drafts 3"`, `"Replies 4"`, US3 #7), `empty_text` (`"No drafts"`, `"No replies yet"`, FR-018), `START_HINT` equals today's hint; `tab_counts` over every scope of the entry; a property over all four statuses × both scopes that every comment is in exactly one tab (FR-013, SC-006); `rows_of` orders by (scope, path bytewise, `line_of`, side, created) with the working tree first (FR-014); `width(area) == min(360, area / 2)` at 1,000 and 500 px (FR-010)
- [ ] T024 [US3] *Layer A* Red: in `crates/micold-core/tests/review_drawer.rs` test `Drawer::default()` closed on Drafts with no watch (FR-011); `apply` returns whether anything changed: `Toggle` open↔closed, closing drops `watch` (FR-008); `Close` closes and drops `watch`; `Escape` closes and returns `false` when already closed (FR-012); `SelectTab`; `Scrolled { tab, offset }` keeps one offset per tab (FR-019); `SendStarted(ids)` replaces any earlier watch; `SendFailed` drops it; `Reported { ids, shown }`: first report of the watched send opens on Replies, a second report does not reopen, `shown == false` consumes the watch and stays closed, a drawer closed since the send stays closed, an open drawer keeps its tab, a report of another send changes nothing (FR-020); `newly_reported`: Draft or Sent → Applied/Answered is new, absent → Applied/Answered is new, Applied → Answered is not
- [ ] T025 [US3] *Layer A* Implement `DrawerTab` (`of`, `label`, `empty_text`), `START_HINT`, `tab_counts`, `rows_of` and `width` in `crates/micold-core/src/review/drawer.rs` until T023 passes
- [ ] T026 [US3] *Layer A* Implement `Drawer { open, tab, offsets: [u32; 2], watch: Option<SendWatch> }`, `SendWatch { ids: BTreeSet<CommentId> }`, `DrawerEvent`, `Drawer::apply` and `newly_reported` in `crates/micold-core/src/review/drawer.rs` until T024 passes

### Layer A — components

- [ ] T027 [P] [US3] *Layer A* Red: in the `#[cfg(test)]` module of new `crates/micold-client/src/ui/material/side_sheet.rs` test `SideSheet` width `min(360, half the area)` at two area widths, the title and × laid out, × emits its message (contract D2, D3)
- [ ] T028 [US3] *Layer A* Implement `SideSheet` (title, ×, content, width from `micold_core::review::drawer::width`) in `crates/micold-client/src/ui/material/side_sheet.rs` and export it in `crates/micold-client/src/ui/material/mod.rs`, until T027 passes
- [ ] T029 [P] [US3] *Layer A* Red: in the `#[cfg(test)]` module of `crates/micold-client/src/ui/material/review_panel_row.rs` test whole-reply mode: the reply is laid out whole and wrapped, **Show all** only when the reply was shortened past 4,000 characters, **Mark applied** and **Answer…** laid out only when their handlers are set (contract D7)
- [ ] T030 [US3] *Layer A* Implement whole-reply mode and `on_mark_applied`/`on_answer`/`on_show_reply` builders on `ReviewPanelRow` in `crates/micold-client/src/ui/material/review_panel_row.rs` until T029 passes
- [ ] T031 [US3] *Layer A* Add a `side_sheet` showcase entry and `review_panel_row` reply-mode poses (long reply, Sent row with actions) in `crates/micold-client/src/showcase/sections/review.rs`, registered in `crates/micold-client/src/showcase/catalogue.rs`

### Layer B — reducer, Esc and docs (blocked until #1269 merges)

- [ ] T032 [US3] *Layer B* Red: in `crates/micold-client/tests/features_changes.rs` test `Msg::Drawer(event)` applies `Toggle`, `Close`, `Escape`, `SelectTab` and `Scrolled` to the shown view's drawer; the drawer's state and composer text (`ComposerPlace::Drawer`) survive switching away and back with #1269's view map and closing the drawer, and are dropped with the tab (FR-019, D10); `review_bar(state).drawer_open` follows it (B8)
- [ ] T033 [P] [US3] *Layer B* Red: in `crates/micold-client/tests/overlay_dispatch_ordering.rs` test Esc order: an open overflow menu, discard dialog or Show all dialog closes first; then the shown drawer closes; with the drawer closed Esc does today's (FR-012, SC-003)
- [ ] T034 [US3] *Layer B* In `crates/micold-client/src/features/changes.rs` replace `Panel` by `ReviewDrawer { state: Drawer, focus, hidden, to_show, viewport }` in #1269's per-entry view, add `ComposerPlace { Card, Drawer }` to `Composer`, and replace `PanelToggled`/`PanelScrolled` by `Msg::Drawer`; the old panel view in `crates/micold-client/src/ui/changes.rs` reads the drawer state so the window still draws (the panel shows until M4); until T032 passes
- [ ] T035 [US3] *Layer B* Add the last arm of `on_escape` in `crates/micold-client/src/app.rs` that sends `Drawer(DrawerEvent::Escape)` when the shown Review tab's drawer is open, until T033 passes
- [ ] T036 [US3] *Layer B* Red: in `crates/micold-client/tests/features_changes.rs` test `drawer_rows(state, tab)` per D5–D7 (groups via `drawer_groups`, "(not in the list)" pull requests, not-in-diff mark, Edit/Delete withheld while a send holds the draft, `markable` for Sent, whole reply up to `CARD_CHARS` with `has_full_reply` past it); `DrawerRowSelected` does today's jump and a refused jump sets today's reason (FR-017); after a `ReviewChanged` the rows and tab counts are recomputed in the same update (FR-022; the 2 s bound is the existing push path's, plan Performance Goals), a focused row that disappears loses focus, and an open Show all dialog of a removed comment closes
- [ ] T037 [US3] *Layer B* In `crates/micold-client/src/features/changes.rs` rename `PanelRow`/`panel_groups` to `DrawerRow`/`drawer_groups`, build `drawer_rows` on `rows_of`, and replace `PanelRowSelected` by `Msg::DrawerRowSelected` (moving callers in `crates/micold-client/src/ui/changes.rs`), until T036 passes
- [ ] T038 [US3] *Layer B* Red: in `crates/micold-client/tests/features_changes.rs` test auto-open over `ReviewChanged` sequences: `SendPressed` starts the watch with the sent drafts' ids (B5); a report of this window's send opens the drawer on Replies once; a later report does not reopen it; a report of another window's send does not open it; a report while the Review tab is not shown consumes the watch; a failed send drops it (FR-020, D11)
- [ ] T039 [US3] *Layer B* In `crates/micold-client/src/features/changes.rs` apply `DrawerEvent::SendStarted(ids)` on `SendPressed`, `DrawerEvent::SendFailed` on `SendFailed`, and on `ReviewChanged` compute `newly_reported` against the previous `ReviewView` and apply `DrawerEvent::Reported { ids, shown }` to that entry's view, until T038 passes
- [ ] T040 [US3] *Layer B* Update "The review panel" in `docs/user-guide/reviewing-changes.md` for what M3 makes visible: Esc closes the panel (the drawer's state drawn the old way until M4), and the panel opens itself on the first report of a send started in this window

### Layer B — layout (blocked until #1269 merges)

- [ ] T041 [US3] *Layer B* Red: in `crates/micold-client/tests/gates/review_bar_fixed.rs` add SC-001's drawer case (**Send review (n)**'s bounds equal with the drawer closed and open), SC-002 (the diff pane's bounds and first visible row are equal with the drawer closed and open), SC-004 (at 1280×800 with the drawer closed the diff spans the tab body beside the file list), and FR-017's line start (the drawer's left edge is at or right of the diff area's middle, so a jumped-to line starts left of it)
- [ ] T042 [US3] *Layer B* In `crates/micold-client/src/ui/changes.rs` replace the right-hand review column by the diff and a `SideSheet` stacked over its right side, from under the review header to the top of the bar, opened and closed by the drawer state, × sending `Drawer(DrawerEvent::Close)`, until T041 passes
- [ ] T043 [US3] *Layer B* In `crates/micold-client/src/ui/changes.rs` draw the drawer's content: the Drafts/Replies `TabStrip` with `DrawerTab::label` counts, each tab's rows from `drawer_rows` through `ReviewPanelRow` (whole-reply mode on Replies, Mark applied and Answer… on Sent rows, the composer in the row for `ComposerPlace::Drawer`), `empty_text` and `START_HINT`; checked by the `drawer-drafts` and `drawer-replies` covered states (T053) and quickstart §B steps 4, 6, 8

**Checkpoint**: US3 core (M1), components (M2), reducer and Esc (M3), drawer drawn (M4).

---

## Phase 6: User Story 4 — See the agent's outcome on the comment card (Priority: P2)

**Goal**: a replied card shows at most the first 4 lines of the reply as drawn at its width, then
**Show all** only when cut.

**Independent test**: a card with a short reply shows it whole without Show all; a long reply is cut
at 4 lines with Show all; quickstart §B step 6.

### Layer A — core rule

- [ ] T044 [P] [US4] *Layer A* Red: in the `#[cfg(test)]` module of `crates/micold-core/src/review/reply.rs` test `card_preview(reply, lines_of)`: a reply of at most 4 lines is kept whole with `cut == false`; a single long line that wraps counts its drawn lines; over 4 lines is cut with `cut == true`; the 4,000-character `CARD_CHARS` cap still applies and sets `cut`; control characters are shown as `display()` shows them (FR-021, R7)
- [ ] T045 [US4] *Layer A* Implement `CARD_LINES: usize = 4`, `Preview { text: Cow<str>, cut: bool }` and `card_preview` (`display()`, then the `CARD_CHARS` cap, then `tooltip::clamp_to_lines(.., CARD_LINES, lines_of)`) in `crates/micold-core/src/review/reply.rs` until T044 passes

### Layer A — components and docs

- [ ] T046 [P] [US4] *Layer A* Red: in the `#[cfg(test)]` module of `crates/micold-client/src/ui/material/line_clamp.rs` test `LineClamped::when_cut(element)`: the element is laid out under the text only when the measure cut it, and takes no height otherwise
- [ ] T047 [US4] *Layer A* Implement `LineClamped::when_cut` in `crates/micold-client/src/ui/material/line_clamp.rs` until T046 passes
- [ ] T048 [US4] *Layer A* Red: in the `#[cfg(test)]` module of `crates/micold-client/src/ui/material/review_comment.rs` test that a card's reply is drawn through `LineClamped` with `CARD_LINES` lines, **Show all** laid out only when the reply is cut or was `shortened`, and a short reply without it (US4 #1, #2, contract D12)
- [ ] T049 [US4] *Layer A* Draw the reply in `ReviewCommentCard` (`crates/micold-client/src/ui/material/review_comment.rs`) through `LineClamped` with `CARD_LINES` and pass **Show all** to `when_cut`; keep the `reply(text, shortened)` signature so `crates/micold-client/src/ui/changes.rs` is untouched; until T048 passes
- [ ] T050 [US4] *Layer A* Add `review_comment` poses with a short reply and a reply cut at 4 lines in `crates/micold-client/src/showcase/sections/review.rs`
- [ ] T051 [US4] *Layer A* Update "Statuses and replies" and "Reading a whole reply" in `docs/user-guide/reviewing-changes.md`: a card shows at most the first 4 lines of a reply, then **Show all**
- [ ] T052 [US4] *Layer A* Run the layout snapshot (`crates/micold-client/tests/layout_snapshot.rs`); if a covered state shows a card with a long reply, re-record `crates/micold-client/tests/fixtures/layout_snapshot.txt` with that card's diff alone (plan Delivery order, M2)

**Checkpoint**: US4 is complete after M2 on today's panel.

---

## Phase 7: Polish & Cross-Cutting Concerns (Layer B, blocked until #1269 merges)

- [ ] T053 Red, first task of M4: add the `review-bar`, `drawer-drafts`, `drawer-replies` and `overflow` covered states to `crates/micold-client/tests/support/covered_states.rs` (FR-024); the layout snapshot fails until T054 re-records it
- [ ] T054 Re-record `crates/micold-client/tests/fixtures/layout_snapshot.txt` with `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot` and check its diff holds only the Review tab's states (quickstart §A)
- [ ] T055 Rewrite `docs/user-guide/reviewing-changes.md` for the bar and drawer: *Sending a review* (the bar, its counts, Send review, the error), *The review panel* → *The comments drawer* (tabs, rows, Esc, auto-open), *Clearing and discarding* (the overflow menu; acts on what is shown, fixing the stale "the entry" wording), *Statuses and replies*; it supersedes the interim wording of T040 and T051 (FR-024)
- [ ] T056 Run quickstart §B with the `visual-pass` skill at 1280×800 in light and dark and save its screenshots and notes in `specs/1270-review-bar-drawer/visual-pass/` (SC-007)
- [ ] T057 Run `mise run gate` green on the final tree

---

## Dependencies & Execution Order

- **Setup (T001) → Foundational (T002–T003)** → every story's Layer A tasks.
- **Layer A core** (T004–T007, T017–T018, T023–T026, T044–T045) depends only on Phase 2.
- **Layer A components** (T008–T010, T027–T031, T046–T052) depend on the core they read
  (`ReviewBar` on `bar`, `SideSheet` on `drawer::width`, the card on `card_preview`).
- **Layer B** (T011–T016, T019–T022, T032–T043, T053–T057) depends on **#1269 merged** and on all
  of Layer A. Within Layer B the reducer tasks (T011–T014, T019–T021, T032–T040) come before the
  layout tasks; M4 starts with the covered states (T053) and the geometry gate (T015, T041), Red,
  then the layout tasks (T016, T022, T042, T043) and the rest of Polish.
- Stories: US1 and US2 share `review/bar.rs` and the bar component; US3's reducer (T034) moves the
  panel flag that US1's `review_bar` reads (T012) into `ReviewDrawer`, so T012 comes first; US4 is
  independent of the others.

## Parallel Opportunities

- T004 (`tests/review_bar.rs`), T023 (`tests/review_drawer.rs`) and T044 (`reply.rs`) in parallel;
  then T006/T007/T018 sequentially in `bar.rs` beside T025/T026 in `drawer.rs` and T045.
- T027 (`side_sheet.rs`), T029 (`review_panel_row.rs`) and T046 (`line_clamp.rs`) in parallel with
  T008 (`review_bar.rs`). Showcase tasks T010, T031, T050 share `sections/review.rs`: sequential.
- T020 (`overlay_registry.rs`) and T033 (`overlay_dispatch_ordering.rs`) run in parallel with the
  `features_changes.rs` test tasks.

## Implementation Strategy

Layer A first, merged as M1 (core rules) and M2 (components, the card's 4-line rule live on
today's panel). The run then waits for #1269; M3 wires the reducer, shell and Esc to the per-entry
view with the old panel still drawn, and M4 draws the bar and drawer, re-records the snapshot,
adds the geometry gate, rewrites the guide and runs the visual pass.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Core rules for the bar, drawer and card 🎯 MVP

- **Tasks**: T001–T007, T017–T018, T023–T026, T044–T045
- **Deliverable**: `micold_core::review::{bar, drawer}` and `reply::card_preview` decide every
  count text, the send button, the overflow items, the error wording, tab membership and order, the
  drawer's state machine with Esc and auto-open, the drawer width and the card's 4-line preview,
  all unit-tested without drawing; the app is unchanged
- **Satisfies**: FR-002, FR-003, FR-005, FR-006 (wording), FR-007, FR-010 (width), FR-011,
  FR-012 (state), FR-013, FR-014, FR-018, FR-019 (state), FR-020 (rule), FR-021 (rule), FR-023;
  SC-006
- **Verify**: `mise run test-core` (tests `review_bar`, `review_drawer`, `tokens_anatomy`,
  `review::reply::tests`)
- **Depends on**: —
- **Tier**: full
- **Docs**: no user-visible change: `docs-not-needed` label

### M2 — Shared components; cards show 4 reply lines

- **Tasks**: T008–T010, T027–T031, T046–T052
- **Deliverable**: `ReviewBar`, `SideSheet`, `LineClamped::when_cut` and `ReviewPanelRow`'s
  whole-reply mode exist with their tests and showcase entries; on today's panel a comment card
  shows at most 4 reply lines, then **Show all** only when cut
- **Satisfies**: US4 acceptance scenarios 1–2; FR-021; FR-003 and FR-010 (component geometry);
  FR-024 (showcase, guide for cards)
- **Verify**: `mise run gate` (component tests in `ui/material/{review_bar,side_sheet,line_clamp,review_comment,review_panel_row}.rs`);
  the showcase's `review_bar`, `side_sheet` and `review_comment` entries
- **Depends on**: M1
- **Tier**: full
- **Docs**: T051 updates *Statuses and replies* and *Reading a whole reply*

### M3 — Reducer, shell and Esc wired to the per-entry review view

- **Tasks**: T011–T014, T019–T021, T032–T040
- **Blocked by**: **#1269 merged** (the per-entry Review tab and its view map). Do not start
  before; the fallback onto today's `State.open` needs a ledger decision.
- **Deliverable**: the per-entry view holds the drawer, send error and overflow state; Esc closes
  the review panel; the panel opens itself on Replies at the first report of a send from this
  window; a failed send is routed to its entry; the old panel still draws, and the bar's messages
  stay unreachable from the UI until M4
- **Satisfies**: US1 acceptance scenarios 3–5 (state); US2 1–4 (state); US3 6, 8 and auto-open
  (state); FR-001, FR-004, FR-006, FR-007, FR-008, FR-012, FR-015–FR-017, FR-019, FR-020, FR-022;
  SC-003, SC-005
- **Verify**: `mise run gate` (`tests/features_changes.rs`, `tests/overlay_registry.rs`,
  `tests/overlay_dispatch_ordering.rs`, `shell::daemon_sync` tests)
- **Depends on**: M1, M2; #1269
- **Tier**: full
- **Docs**: T040 updates *The review panel* for Esc and auto-open

### M4 — Review tab layout, snapshot, guide and visual pass

- **Tasks**: T015–T016, T022, T041–T043, T053–T057
- **Blocked by**: **#1269 merged**
- **Deliverable**: the Review tab shows the fixed review bar above the status bar and the comments
  drawer over the diff; the header no longer holds Review (n) or Send review; the geometry gate
  holds SC-001, SC-002 and SC-004; the layout snapshot and user guide describe the new layout
- **Satisfies**: US1 1–6, US2 1–4, US3 1–8 (drawn); FR-001, FR-009, FR-010, FR-017 (line start),
  FR-024; SC-001, SC-002, SC-004, SC-007
- **Verify**: `mise run gate` (`layout_snapshot` incl. `review_bar_fixed`); quickstart §B with
  screenshots in `specs/1270-review-bar-drawer/visual-pass/`
- **Depends on**: M1, M2, M3; #1269
- **Tier**: full
- **Docs**: T055 rewrites the review chapter
