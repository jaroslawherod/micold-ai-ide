# Implementation Plan: Split the Terminal Area into Panes

**Branch**: `claude/project-thread-wysm57` (spec dir `484-terminal-split-panes`) | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/484-terminal-split-panes/spec.md` (clarified, CLEAN)

## Summary

Replace the single displayed terminal with a per-project **pane layout**: a pure split tree in
`micold-core` (`pane_layout.rs`, render-free, fully unit-tested), persisted as one new
`#[serde(default)]` field of the project's existing state file, and rendered by a new shared
`SplitView` widget in the client. The hard part is not drawing: today the client and daemon assume
**one viewed session and one attached process** (`SetViewedSession`, `SessionAttachProcess`,
`GridFrame`/`SessionInput`/`SessionResize` addressed by `SessionId` alone, `App.grids:
HashMap<SessionId, GridCache>`, a single `App.last_grid`). Showing an AI CLI next to a regular
terminal of the *same* session needs terminals addressed by `(SessionId, SessionProcess)`, so the
plan makes one additive wire change (research R1, [contracts/wire.md](contracts/wire.md)). Input,
focus, and PTY size are all keyed by the pane's terminal, never by "the active session".

## Technical Context

**Language/Version**: Rust (workspace edition as in `Cargo.toml`), iced 0.14.0

**Primary Dependencies**: existing only: `iced` (client), `serde`/`serde_json` (core store). No new crates.

**Storage**: per-project state file (`StoredProjectState` in `crates/micold-core/src/store.rs`), new optional `pane_layout` field, no `schema_version` bump (same rule as `last_session`, feature 025). Local only.

**Testing**: `cargo test` via `mise run test-core` (layout tree, store round-trip/degradation), client tests in `crates/micold-client/tests/` and `main_tests.rs` (routing, resize, key handling), daemon tests for the wire change, geometry gates (`ui/cdk`/material anatomy tests) for the focus mark and divider, quickstart §B visual pass.

**Target Platform**: Linux, macOS, Windows desktop (Principle VI).

**Project Type**: desktop-app (iced client + session daemon + render-free core).

**Performance Goals**: idle CPU with 6 panes within 10% of one pane (SC-004); no per-pane timers.

**Constraints**: offline; coalesced resize during drag; one PTY has one size (no mirroring); daemon survives client restart (layout rebuilt from the store, terminals re-attached).

**Scale/Scope**: ≤ 6 panes per project; ~1 core module, 1 shared widget, wire addition, ~6 client touch points.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 (no change).*

- [x] **I. Test-First**: every task in tasks.md pairs a failing test first; the layout tree is pure core code driven by unit tests; wire and routing changes start from failing client/daemon tests ([research.md](research.md) R9). PASS
- [x] **II. Multi-Session**: layout is per project; a pane references `(SessionId, SessionProcess)`; frames, input and resize are keyed by that pair so output never crosses panes; removing a session empties its pane only. PASS
- [x] **III. Worktree Integration**: panes show sessions of any worktree of the project; no file/VCS operation is added; the layout spans worktrees (spec Assumptions). PASS
- [x] **IV. Local-First**: layout lives in the local per-project state file; forgetting a project already deletes that file (feature 014), so the layout goes with it; nothing remote. PASS
- [x] **V. Rust + iced**: `PaneTree`/`PaneId`/`TerminalRef` types make "no panes", "two panes one terminal" and "ratio out of range" unrepresentable (constructors validate; see [data-model.md](data-model.md)). PASS
- [x] **VI. Cross-Platform**: `Cmd`/`Ctrl+Shift` chords chosen against reserved desktop chords; widget code is platform-neutral; CI covers all three. PASS
- [x] **VII. Documentation**: new `docs/user-guide/terminal-panes.md` + `docs/SUMMARY.md` + shortcut list in the same change; CHANGELOG entry. PASS
- [x] **VIII. Reusable UI**: divider/split container is a new shared builder widget `material/split_view.rs` (modelled on `resize_handle.rs`, reusing its hover/drag design); focus mark and empty-pane picker reuse `keyboard_focus`/`picker` primitives. PASS

## Design

### D1. Core: `micold-core/src/pane_layout.rs` (new, render-free)

`PaneLayout { root: PaneNode, focused: PaneId }`; `PaneNode = Leaf(Pane) | Split { axis, ratio, first, second }`. Pure operations: `split(pane, axis)` (refuse at 6 panes or below-minimum, returning a typed `Refusal` the UI shows: FR-001, minimum-size edge), `close(pane)` (refuses the last: FR-006), `swap(a, b)` (FR-008), `show(pane, terminal)` and `show_or_focus(terminal)` (at-most-one-pane invariant, FR-004/Edge), `focus_dir(Direction)` using the rectangles from a unit-square layout pass (FR-009), `set_ratio` (clamped by minimum sizes), `reset_equal` (double press), `rects(total: Size, min: Size)` (geometry; scales dividers down below the total minimum), `prune(live: &dyn Fn(&TerminalRef)->bool)` (gone terminals become empty panes, never collapse: FR-014). Serde form is versioned and lenient ([data-model.md](data-model.md)).

### D2. Store: persistence (FR-013/014/016)

**Ownership: the daemon's catalog**, like `foreground_by_project` (its docs: written by the daemon). `StoredProjectState` is rebuilt from `Workspace` on every save (`from_workspace`), so the layout must live in `Workspace`: new `pane_layouts: BTreeMap<PathBuf, PaneLayout>` beside `foreground_by_project`, mapped to/from `StoredProjectState.pane_layout: Option<StoredPaneLayout>` (`#[serde(default, skip_serializing_if)]`) in `from_workspace` and the load path, and written by the existing atomic temp+rename save. Client↔daemon: new `ClientMsg::SetPaneLayout { project, layout: Option<String> }` (the layout's JSON, validated by `pane_layout.rs` on the daemon before storing) and a `pane_layout: Option<String>` field on `ProjectSnapshot` (`protocol/messages.rs`, built in `catalog.rs::snapshot()` from `Workspace.pane_layouts`), so the client restores it on connect (D3, contracts/wire.md). A test saves unrelated state (new session, rename) and asserts the layout survives (FR-013/SC-006). A layout with unknown `layout_version`, bad JSON shape, ≥7 leaves, or duplicate terminals degrades to `None` → single pane (never an error to startup; the rest of the state file still loads — a layout fault must not mark the project unreadable). `forget` already removes the file.

### D3. Wire: terminals addressed by `(session, process)` — [contracts/wire.md](contracts/wire.md)

Additive change, `PROTOCOL_VERSION` 36 → 37 with the `tests/schema_hash.rs` pin moved: `GridFrame` gains `process`; `SessionInput` and `SessionResize` gain `process: Option<SessionProcess>` (`None` = the attached process only, today's behaviour; a resize never touches a session's other processes, FR-012); new `ClientMsg::SetPaneLayout` (D2) and `ClientMsg::SetViewedTerminals { project, terminals: Vec<TerminalRef> }` tells the daemon every terminal to stream (≤ 6). `SessionAttachProcess`/`SetViewedSession` remain for compatibility and map to a one-element set. Daemon `LiveSession.attached` becomes a set of attached processes; the framer already runs per process.

### D4. Client state

`App.pane_layouts: HashMap<ProjectPath, PaneLayout>` replaces "the displayed terminal" reads; `core.session.active` stays the *focused pane's session* so features that follow the active session (sidebar highlight, toolbar, attention) keep working (FR-017). `App.grids` is keyed by `TerminalRef`; `last_grid` becomes per pane (`HashMap<PaneId,(u16,u16)>`) with the focused pane's value used for starting new sessions (`send_pane_size`). `on_terminal_resized` and `send_pane_size` (`shell/daemon_sync.rs`) gain the pane identity and sends `SessionResize{session, process, cols, rows}`; during a divider drag sizes are coalesced to the drag end plus at most one send per ~100 ms frame boundary (no timer: sent from the existing layout pass when the size changed).

### D5. Client UI

- `ui/material/split_view.rs` (new shared widget, builder, Principle VIII): lays out children from `PaneLayout::rects`, draws dividers, drag via the `resize_handle.rs` pattern (widget sees all mouse events once dragging, so no capture layer), double press → equal.
- `ui/terminal.rs`: renders one `terminal_pane` per leaf. Each pane carries its own focus state; unfocused pane press → `FocusPane(id)` consumed (not forwarded), same as feature 023's one-press rule. Focus mark: 2 px accent border plus a header strip with the terminal name in the focused colour (not colour alone).
- Pane header (title, split/close buttons, drag handle for swap) and empty-pane picker reuse `picker`/`icon_button`.
- Tab strip / sidebar selection → `PaneLayout::show_or_focus` on the focused pane.
- Redraw sharing (spec Concurrency note): each `terminal_pane` is its own iced widget with its own cache; a frame for terminal X invalidates only X's pane. No timers; the existing focus-gated polls are untouched (FR-015).
- Keymap: `keymap.rs` gets `PaneAction` chords `Ctrl/Cmd+Shift+D/H/W/Arrow`, checked *before* terminal encoding so they are never forwarded; rebindable where the existing shortcuts are ([research.md](research.md) R6).

### D6. Docs

`docs/user-guide/terminal-panes.md`, `docs/SUMMARY.md`, shortcut reference, `CHANGELOG.md`.

## Requirement map

| FR | Where |
|---|---|
| FR-001, 004, 006, 008 | D1 ops; D5 header/shortcuts |
| FR-002, 003 | D3 per-terminal frames; D5 one `terminal_pane` per leaf |
| FR-005 | D1 `set_ratio`/`reset_equal`; D5 `SplitView` |
| FR-007 | D1 `close`/`show` only touch the layout; no daemon stop call (test asserts none sent) |
| FR-009 | D5 keymap + D1 `focus_dir` |
| FR-010, 011 | D4 focus owner; D5 focus mark + empty pane |
| FR-012 | D3 `SessionResize.process`; D4 coalescing |
| FR-013, 014, 016 | D2, [data-model.md](data-model.md) |
| FR-015 | D5 redraw sharing; no timers; perf probe in quickstart |
| FR-017 | D4 `active` kept; one-pane layout renders as today |

## Test strategy

| Layer | Covers |
|---|---|
| Core unit (`mise run test-core`) | tree ops, caps/minimums, focus direction, prune, serde round-trip, newer-version and corrupt degradation, at-most-one-pane invariant (property-style loop over op sequences) |
| Core store | `pane_layout` round-trip, survives unrelated saves, other state intact when layout corrupt, forget deletes (FR-013/014, SC-006/007) |
| Daemon | multi-attach streams two processes of one session and two sessions; input/resize routed by `(session, process)`; old-shape messages still work |
| Client | SC-001 (split + one choice = 2 interactions, by mouse and by key); FR-003 per-pane selection, link and scrollback state (grid/selection keyed by terminal); key routed to focused pane only for 2–6 panes (SC-002), unfocused press focuses and is not delivered, resize per pane (SC-003), close keeps sessions (SC-005, no stop message), project switch swaps layouts, chords never reach `encode` |
| Geometry gates | divider hit area, minimum pane size, focus mark non-colour cue, both themes |
| Quickstart §B visual pass | focus mark, empty pane, drag feel, two live panes, `stty size` per pane, 6-pane idle CPU |

## Project Structure

### Documentation (this feature)

```text
specs/484-terminal-split-panes/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── pane-layout-file.md   # on-disk layout JSON
│   ├── wire.md               # protocol 36 -> 37
│   └── keybindings.md        # pane chords
└── tasks.md                  # /speckit-tasks
```

### Source Code

```text
crates/micold-core/src/pane_layout.rs          # new: tree, ops, geometry, serde
crates/micold-core/src/store.rs                # StoredProjectState.pane_layout + load/save
crates/micold-core/src/protocol/{messages,grid,version}.rs   # D3
crates/micold-daemon/src/{state,server,framer}.rs            # multi-attach, routing
crates/micold-client/src/ui/material/split_view.rs           # new shared widget
crates/micold-client/src/{main,main_tests,keymap}.rs   # App.grids, last_grid live in main.rs
crates/micold-core/src/workspace.rs            # pane_layouts beside foreground_by_project
crates/micold-daemon/src/catalog.rs            # persist/restore layout
crates/micold-client/src/features/session.rs   # PaneMsg
crates/micold-client/src/shell/daemon_sync.rs  # per-pane resize, view set
crates/micold-client/src/ui/terminal.rs        # pane area rendering
docs/user-guide/terminal-panes.md, docs/SUMMARY.md, CHANGELOG.md
```

**Structure Decision**: existing three-crate split; pure logic in core, widget in the client's material layer, wire addition in core protocol + daemon.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Wire bump 36→37 | Frames/input/resize are `SessionId`-only and the daemon attaches one process per session; the headline case is AI CLI beside a terminal of the same session | Client-only layout rejected: the daemon would stream one process, so the second pane could never be live (research R1) |
