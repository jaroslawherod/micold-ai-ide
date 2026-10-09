# Research: Terminal Split Panes

Each entry: Decision / Rationale / Rejected.

## R1. Addressing terminals on the wire
**Decision**: add `process: SessionProcess` to `GridFrame`, `SessionInput`, `SessionResize` (default `Primary`/attached) and a `SetViewedTerminals` set message; bump `PROTOCOL_VERSION` to 37. Evidence: `ClientMsg::SetViewedSession` carries one session; `SessionAttachProcess` "exactly one is attached"; `LiveSession.attached: SessionProcess` (daemon `state.rs`); client `grids: HashMap<SessionId, GridCache>`.
**Rationale**: AI CLI + regular terminal of one session side by side is the issue's first example.
**Rejected**: (a) client-only, one pane per session — cannot show a session's CLI and its terminal together; (b) one session id per process — invasive catalog change; (c) a second connection per pane — doubles handshake/auth and breaks ordering.

## R2. Layout data structure
**Decision**: binary split tree, each leaf a pane with stable `PaneId`; ratio in (0,1) per split.
**Rejected**: iced `pane_grid` — its state is not serialisable in 0.14 without wrapper glue, its drag/minimum/focus semantics differ from the spec (double-press reset, scale-down), and Principle VIII wants a shared themed widget; flat grid — cannot express nested H/V splits.

## R3. Persistence location
**Decision**: new optional field in the per-project state file (as `last_session`, feature 025), no schema bump.
**Rejected**: separate `layout.json` — a second file to forget/migrate; the settings file — layout is per project, not global.

## R4. Terminal reference durability
**Decision**: `TerminalRef { session: SessionId, process: ProcessRef }` with `ProcessRef = Primary | Shell(ShellInstanceId)`; resolves against the workspace on load, else the pane becomes empty.
**Rejected**: storing PTY ids — not stable across daemon restarts.

## R5. Focus and input routing
**Decision**: `PaneLayout.focused` is the single owner; keys are routed to `focused`'s `TerminalRef`, never to `core.session.active` directly; `active` is derived from it. Unfocused press is consumed by the pane (feature 023 rule).
**Rejected**: per-pane focus flags — allow two focused panes (violates FR-010).

## R6. Shortcuts
**Decision**: `Ctrl/Cmd+Shift+D/H/W/Arrow` per the clarification; handled in `keymap.rs` before `encode`; rebindable through the same mechanism as existing chords. `Ctrl+Shift+Arrow` conflicts with word-selection in some terminals' text fields only outside the terminal; feature 006's map is checked by a test (contracts/keybindings.md).
**Rejected**: `Ctrl+Alt+Arrow` (GNOME/KDE workspace switch), tmux-style prefix (modal state, unlike other app shortcuts).

## R7. Resize coalescing
**Decision**: the pane widget already reports size only on change; during a drag the app records the latest size per pane and sends it from the layout pass at most once per frame, plus a final send on drag end (FR-012). No timer.
**Rejected**: debounce timer — a per-pane timer breaks FR-015 and delays the final size.

## R8. Redraw sharing
**Decision**: per-terminal grid cache and per-pane widget cache; a frame invalidates only its pane.
**Rejected**: one shared canvas for the whole area — every frame repaints all panes.

## R9. Test-first order
Core tree and store first (pure), then wire/daemon, then client routing, then widget geometry, then docs; each behind a failing test (tdd test list is derived in the tasks unit).

## R10. Minimum pane size
**Decision**: 20 columns × 5 rows of the current cell size plus the header strip (constants in `pane_layout.rs`, one place); layout pass takes the minimum in pixels. Below the total minimum, ratios scale down proportionally.
