# Tasks: Split the Terminal Area into Panes

**Input**: `specs/484-terminal-split-panes/` (plan.md, spec.md, data-model.md, contracts/, research.md, quickstart.md)

**Tests**: Mandatory (Constitution I). In every phase the test task comes first, is run and seen to fail for the right reason, and only then is the implementation task done.

**Documentation**: each story's user-guide update is in that story's phase (Constitution VII; CI's user-guide gate).

**Cross-platform**: logic stays in render-free `micold-core`; chords use `Cmd` on macOS, `Ctrl` elsewhere (Constitution VI).

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [x] T001 Add the empty `crates/micold-core/src/pane_layout.rs` (types only: `PaneId(u32)`, `Axis`, `TerminalRef { session: SessionId, process: SessionProcess }`, `Pane { id, terminal: Option<TerminalRef> }`, `PaneNode`, `PaneLayout { root, focused }`, `Refusal`) and export it from `crates/micold-core/src/lib.rs`. Fields private; constructors validate (data-model.md).

## Phase 2: Foundational (blocks all stories)

- [x] T002 [P] Write failing unit tests in `crates/micold-core/src/pane_layout.rs` (`#[cfg(test)]`): single-pane default; `split(pane, axis)` moves focus to the new pane and refuses at 6 leaves and below the minimum size (research R10: 20 columns × 5 rows of cell size plus the header strip, constants in one place) with a typed `Refusal`; `show` / `show_or_focus` keep "a `TerminalRef` appears in at most one leaf: no duplicates, ever"; a property-style loop over random op sequences keeps 1 ≤ leaves ≤ 6, `focused` is a leaf, `PaneId`s unique; `rects(total, min)` tiles the whole area with no overlap and scales dividers down below the total minimum; `prune(live)` turns gone terminals into empty panes and never collapses the tree.
- [x] T003 Implement `split`, `show`, `show_or_focus`, `rects`, `prune`, ratio clamp "ratio in [0.05, 0.95]" in `crates/micold-core/src/pane_layout.rs` until T002 passes (`mise run test-core`).
- [x] T004 [P] Write failing tests in `crates/micold-core/tests/` (next to `schema_hash.rs`): `GridFrame` gains `process` (`#[serde(default)]` → `Primary`) and round-trips under the postcard codec; `SessionInput` / `SessionResize` gain `process: Option<SessionProcess>`; new `ClientMsg::SetViewedTerminals { project, terminals: Vec<TerminalRef> }` (`TerminalRef` = `(SessionId, SessionProcess)`), `ClientMsg::SetPaneLayout { project, layout: Option<String> }` and `ProjectSnapshot.pane_layout: Option<String>` (`#[serde(default, skip_serializing_if)]`) so the whole 36 → 37 wire change lands once (handling arrives in M4 (T032)); an absent `process` (`None`) decodes to today's behaviour (contracts/wire.md).
- [x] T005 Implement the wire change in `crates/micold-core/src/protocol/{messages,grid,version}.rs`: `PROTOCOL_VERSION` 36 → 37, move the `tests/schema_hash.rs` pin and every hard-coded version pin (grep `PROTOCOL_VERSION` and `36`). `SetViewedSession` / `SessionAttachProcess` stay. Daemon treats `SetPaneLayout` as unknown-and-ignored until T032.
- [x] T006 [P] Write failing daemon tests in `crates/micold-daemon/tests/` (or beside `server.rs`): `SetViewedTerminals` streams exactly the named terminals (two processes of one session; two sessions) and replaces the previous set; a frame for `(session, process)` reaches only clients whose set contains it; `SessionInput` / `SessionResize` with `Some(process)` act on that PTY only and leave the session's other processes untouched; `None` behaves as before; unknown terminals ignored; ≤ 6 entries; first entry recorded via `remember_foreground`; a failed resize of one terminal does not affect another.
- [x] T006a [P] Write a failing integration test over a real daemon in `crates/micold-daemon/tests/` (Constitution II isolation gate): two sessions in different worktrees each shown in a pane, output of one never appears in the other's frames, and removing one session empties only its pane.
- [x] T007 Implement multi-attach in `crates/micold-daemon/src/{state,server,framer}.rs`: `LiveSession.attached` becomes a set of attached processes (the framer already runs per process); route input / resize by `(session, process)`; map `SetViewedSession` / `SessionAttachProcess` to a one-element set. Make T006 pass.
- [x] T008 [P] Write failing client tests in `crates/micold-client/src/main_tests.rs` (and `crates/micold-client/tests/`): `App.grids` keyed by `TerminalRef`, a frame for terminal X updates only X's grid / selection / scrollback / link state (FR-003); `SessionInput` goes to the focused pane's terminal; a project with one pane behaves exactly as today incl. tab strip and empty-terminal messages (FR-017); switching projects switches layouts and never shows one project's terminal in another's area (FR-016); `core.session.active` follows the focused pane's session.
- [x] T009 Pane decisions (split refusal, `show_or_focus`, focus-on-press, swap, routing) live in render-free reducers in `crates/micold-client/src/features/session.rs`; `ui/terminal.rs` is glue only (Constitution I). Implement client state in `crates/micold-client/src/main.rs`, `crates/micold-client/src/features/session.rs` (`PaneMsg`) and `crates/micold-client/src/shell/daemon_sync.rs`: `App.pane_layouts: HashMap<project path, PaneLayout>`, `App.grids: HashMap<TerminalRef, GridCache>`, `App.pane_sizes: HashMap<PaneId,(u16,u16)>` replacing `last_grid`, send `SetViewedTerminals` whenever the visible set changes; new sessions still start with the focused pane's size (`send_pane_size`). Make T008 pass.

**Checkpoint**: one pane renders and works as before on the new addressing.

## Phase 3: User Story 1 - Watch two terminals side by side (P1) 🎯 MVP

**Goal**: split the terminal area, pick a terminal per pane, see both live (FR-001, FR-002, FR-004, FR-011 empty pane).

**Independent test**: split once, choose a terminal: two live panes in 2 interactions (SC-001).

- [x] T010 [P] [US1] Write failing geometry / unit tests for the new shared widget under `crates/micold-client/src/ui/material/` (anatomy test file beside `resize_handle.rs`'s): `SplitView` lays children out from `PaneLayout::rects`, draws a divider hit area of usable size, respects the minimum pane size, and shrinking the window below the layout's total minimum scales dividers down, in both themes.
- [x] T011 [US1] Add `crates/micold-client/src/ui/material/split_view.rs` (shared builder widget, Constitution VIII; layout and divider drawing only, modelled on `resize_handle.rs`) and register it in the material module. Make T010 pass.
- [x] T012 [P] [US1] Write failing client tests in `crates/micold-client/src/main_tests.rs`: split by the pane header button then one choice in the empty-pane picker = 2 interactions (SC-001); a new pane opens on a terminal not shown elsewhere when one exists, otherwise the empty pane with a picker (FR-004); a 7th split shows the visible refusal reason (FR-001); choosing a tab-strip, sidebar or pane-picker choice calls `show_or_focus` (the terminal already shown is focused, not duplicated); a terminal whose process exited shows its exit state in its pane while other panes are unaffected.
- [x] T013 [US1] Implement pane rendering in `crates/micold-client/src/ui/terminal.rs`: one `terminal_pane` widget (own cache) per leaf inside `SplitView`, pane header (terminal name, split vertical / horizontal buttons via `icon_button`), empty-pane state with `picker`, transient refusal message; wire `PaneMsg::{Split, Show}` and tab strip / sidebar to `show_or_focus`. No timers. Make T012 pass.
- [x] T014 [US1] Docs: create `docs/user-guide/terminal-panes.md` (split, pick a terminal, empty pane, 6-pane cap), link it in `docs/SUMMARY.md`; `CHANGELOG.md` is generated by release-please, so no entry.

**Checkpoint**: M1 deliverable works on main.

## Phase 4: User Story 2 - Keyboard goes to exactly one pane (P1)

**Goal**: exactly one focused pane, clearly marked; input only there (FR-010, FR-011).

**Independent test**: with 2–6 panes type; only the focused pane's terminal receives it (SC-002).

- [x] T015 [P] [US2] Write failing tests: geometry test of the focus mark in `crates/micold-client/src/ui/` (2 px accent border plus header strip in the focused colour; visible without hover; not colour alone; light and dark); client tests in `main_tests.rs`: for 2–6 panes a key reaches only the focused pane's terminal; a press in an unfocused pane focuses it and is consumed, not forwarded (feature 023's one-press rule); exactly one pane focused whenever the area is displayed; focus is restored to the same pane after the window regains focus or a dialog / menu closes (feature 023).
- [x] T016 [US2] Implement in `crates/micold-client/src/ui/terminal.rs` and `crates/micold-client/src/main.rs`: per-pane focus state, `PaneMsg::FocusPane(id)`, the focus mark, key routing keyed by the focused pane. Make T015 pass.
- [x] T017 [US2] Docs: add the focus section to `docs/user-guide/terminal-panes.md`.

## Phase 5: User Story 5 - Each pane is a real terminal of its own size (P1)

**Goal**: each pane resizes its own PTY (FR-012).

**Independent test**: `stty size` in each pane matches its pane (SC-003).

- [x] T018 [P] [US5] Write failing tests: `on_terminal_resized` in `crates/micold-client/src/main_tests.rs` sends `SessionResize{session, process: Some(..), cols, rows}` per pane, never a size for the session's other processes; during a divider drag sizes coalesce to at most one send per frame boundary (sent from the layout pass when the size changed, no timer) and the final size is always sent, including on drag end with no further layout pass (the drag-end event itself flushes it; coalescing bound is one send per rendered frame, no timer); a closed or replaced terminal keeps its last size until shown again; a daemon test in `crates/micold-daemon/tests/` that two panes of one session hold distinct PTY sizes.
- [x] T019 [US5] Implement per-pane resize in `crates/micold-client/src/shell/daemon_sync.rs` (`on_terminal_resized`, `send_pane_size` gain the pane identity) and `crates/micold-client/src/ui/terminal.rs`. Make T018 pass.
- [x] T020 [US5] Docs: add "each pane has its own size" to `docs/user-guide/terminal-panes.md`.

## Phase 6: User Story 3 - Keyboard shortcuts for panes (P2)

**Goal**: split, close, and focus movement from the keyboard (FR-009).

**Independent test**: each chord does its action and is never forwarded (contracts/keybindings.md).

- [ ] T021 [P] [US3] Write failing tests: core `focus_dir(Direction)` in `crates/micold-core/src/pane_layout.rs` uses the unit-square rects (nearest neighbour; stays put at the edge); `crates/micold-client/src/keymap.rs` tests: a test enumerates feature 006's forwarded chords and asserts none equals a pane chord; existing `Ctrl/Cmd+Shift+E/T/C/V` unchanged; pane chords are rebindable where the existing shortcuts are; are checked before terminal encoding and never reach `encode`; a refused action (cap, minimum) shows a visible reason.
- [ ] T022 [US3] Implement `focus_dir` in `crates/micold-core/src/pane_layout.rs` and `PaneAction` chords `Ctrl/Cmd+Shift+D` (split vertical), `H` (split horizontal), `W` (close focused), `Arrow` (focus) in `crates/micold-client/src/keymap.rs`, rebindable where the existing shortcuts are (research R6). Ship D/H/Arrow here; the `W` close chord is added in T027 once `close` exists. Make T021 pass.
- [ ] T023 [US3] Docs: add the shortcut table to `docs/user-guide/terminal-panes.md` and to the shortcut reference page.

## Phase 7: User Story 4 - Resize, close and rearrange panes (P2)

**Goal**: drag dividers, close panes, swap terminals (FR-005, FR-006, FR-007, FR-008).

**Independent test**: drag, double press, close, swap; sessions keep running (SC-005).

- [ ] T024 [P] [US4] Write failing unit tests in `crates/micold-core/src/pane_layout.rs`: `close` refuses the last pane, sibling replaces the parent, focus → nearest surviving leaf; `swap(a, b)`; `set_ratio` clamped by minimum sizes; `reset_equal`.
- [ ] T025 [US4] Implement `close`, `swap`, `set_ratio`, `reset_equal` in `crates/micold-core/src/pane_layout.rs`. Make T024 pass.
- [ ] T026 [P] [US4] Write failing tests: `SplitView` divider drag (pattern of `resize_handle.rs`, widget sees all mouse events once dragging), double press → equal, no pane below the minimum (geometry gate); client tests in `main_tests.rs`: dragging a header onto another pane swaps; closing a pane or replacing its terminal sends no stop / restart / detach message and the session stays in tab strip and sidebar (FR-007, SC-005); closing the focused pane moves focus; closing the last pane is refused with a visible reason.
- [ ] T027 [US4] Implement divider drag and double press in `crates/micold-client/src/ui/material/split_view.rs`, pane header close button and drag-to-swap in `crates/micold-client/src/ui/terminal.rs`, and the `Ctrl/Cmd+Shift+W` chord in `crates/micold-client/src/keymap.rs`. Make T026 pass.
- [ ] T028 [US4] Docs: add resize, close and swap to `docs/user-guide/terminal-panes.md`.

## Phase 8: User Story 6 - The layout survives a restart (P2)

**Goal**: layout stored per project, restored, degrading safely (FR-013, FR-014, FR-016).

**Independent test**: arrange panes, restart, same arrangement; corrupt field → one pane (SC-006, SC-007).

- [ ] T029 [P] [US6] Write failing tests in `crates/micold-core/src/store.rs` / `crates/micold-core/tests/`: `StoredPaneLayout` round-trip (contracts/pane-layout-file.md); survives unrelated saves (new session, rename; `from_workspace` rebuilds the state); degrades to `None` for `layout_version` > 1 or missing, bad JSON shape (rest of the state file still loads), > 6 leaves, duplicated terminal, `focused` not a leaf, ratio outside [0.05, 0.95]; unresolved terminals are kept and shown as empty panes; `forget` deletes it; no `schema_version` bump.
- [ ] T030 [US6] Implement `Workspace.pane_layouts: BTreeMap<PathBuf, PaneLayout>` in `crates/micold-core/src/workspace.rs`, `StoredProjectState.pane_layout: Option<StoredPaneLayout>` (`#[serde(default, skip_serializing_if)]`) with load / save mapping and validation in `crates/micold-core/src/store.rs`, serde in `pane_layout.rs`. Make T029 pass.
- [ ] T031 [P] [US6] Write failing wire / daemon tests: `ClientMsg::SetPaneLayout { project, layout: Option<String> }` validates and persists in the catalog (invalid → rejected, stored layout kept); `ProjectSnapshot.pane_layout: Option<String>` (`#[serde(default, skip_serializing_if)]`) carries it on connect; the wire shapes already exist from T005 (no pin change).
- [ ] T032 [US6] Implement `SetPaneLayout` and the snapshot field in `crates/micold-core/src/protocol/messages.rs` and `crates/micold-daemon/src/catalog.rs` (`snapshot()`, persist). Make T031 pass.
- [ ] T033 [P] [US6] Write failing client tests in `main_tests.rs`: the layout is restored from the snapshot on connect and `prune` empties gone terminals; every layout change sends `SetPaneLayout`; a project with no stored layout shows one pane on the last session; the stored `focused` pane is restored.
- [ ] T034 [US6] Implement restore and save in `crates/micold-client/src/main.rs` and `crates/micold-client/src/shell/daemon_sync.rs`. Make T033 pass.
- [ ] T035 [US6] Docs: add "your layout is remembered per project" to `docs/user-guide/terminal-panes.md`.

## Final Phase: Polish (close unit, no code)

- [ ] T036 Run quickstart Part A and Part B (`visual-pass`) and record the results in `specs/484-terminal-split-panes/quickstart.md` (FR-015's idle CPU is manual-only; no pane timer or subscription is added, asserted in T013's review), including the 6-pane idle CPU probe (SC-004, ≤ +10%).
- [ ] T037 Tick the tasks, close the spec in `autopilot.md`.

## Dependencies

Setup → Foundational → US1 → US2 → US5 (all M1, so panes never ship unfocused or at the wrong PTY size) → US3 (M2) → US4 (M3) → US6 (M4). Within a phase: test task, then its implementation. US3's `W` chord needs `close` (T025): it ships in T027.

## Parallel opportunities

T002 ∥ T004 ∥ T006 ∥ T008 (different files; each implementation task follows its test). T010 ∥ T012.

## Implementation Strategy

MVP first: M1 delivers split + choose + live panes on the new `(session, process)` addressing. Each later story is a self-contained milestone that merges on its own.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Live panes with focus and their own size 🎯 MVP

- **Tasks**: T001–T020 and T006a
- **Deliverable**: A project's terminal area can be split (button) up to 6 panes; each pane shows a chosen terminal live (an AI CLI beside a regular terminal of the same session) at its own PTY size; exactly one pane is focused and visibly marked, typing reaches only it. No close control yet: a pane is emptied by choosing another terminal (closing arrives in M3). M1 folds US1, US2 and US5 because panes without focus or per-pane size would be half-wired; the diff exceeds the usual split threshold and no split along an acceptance scenario leaves a working deliverable.
- **Satisfies**: US1 acceptance scenarios 1–4, US2, US5 acceptance scenarios; FR-001, FR-002, FR-003, FR-004, FR-010, FR-011, FR-012, FR-016, FR-017; SC-001, SC-002, SC-003
- **Verify**: `mise run test-core`, `mise run test`; quickstart Part B steps 1–5 and 7
- **Depends on**: —
- **Tier**: full

### M2 — Pane shortcuts

- **Tasks**: T021–T023
- **Deliverable**: Chords split vertically / horizontally and move focus between panes, never forwarded to a terminal. (Close chord arrives in M3.)
- **Satisfies**: US3 acceptance scenarios for split and focus; FR-009 (split, focus)
- **Verify**: `mise run test` (keymap tests); quickstart Part B steps 1, 4
- **Depends on**: M1
- **Tier**: full

### M3 — Resize, close, rearrange

- **Tasks**: T024–T028
- **Deliverable**: Dividers drag and reset on double press, panes close (sessions keep running) and swap terminals; `Ctrl/Cmd+Shift+W` closes the focused pane.
- **Satisfies**: US4 acceptance scenarios; US3 close chord; FR-005, FR-006, FR-007, FR-008, FR-009; SC-005
- **Verify**: `mise run test-core`, `mise run test`; quickstart Part B step 6
- **Depends on**: M2
- **Tier**: full

### M4 — Layout survives restart

- **Tasks**: T029–T035
- **Deliverable**: The layout is stored per project and restored on restart; a corrupt or newer layout degrades to one pane.
- **Satisfies**: US6 acceptance scenarios; FR-013, FR-014, FR-016; SC-006, SC-007
- **Verify**: `mise run test-core`, `mise run test`; quickstart Part B step 8
- **Depends on**: M3
- **Tier**: full
