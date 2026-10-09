# Data Model: Terminal Split Panes

## PaneLayout (core, `pane_layout.rs`)
- `root: PaneNode`, `focused: PaneId` (must name a leaf).
- `PaneNode::Leaf(Pane)` | `PaneNode::Split { axis: Axis(Horizontal|Vertical), ratio: f32, first: Box, second: Box }`.
- `Pane { id: PaneId, terminal: Option<TerminalRef> }`; `PaneId` is a u32 allocated by the layout, stable across save/load.
- `TerminalRef { session: SessionId, process: ProcessRef }`, `ProcessRef` is the existing `SessionProcess` (`Primary | Shell(ShellInstanceId)`), not a new type.

Invariants (enforced by constructors/ops, so violations are unrepresentable): 1 ≤ leaves ≤ 6; ratio in [0.05, 0.95] and respects minimums at the current size; `focused` is a leaf; a `TerminalRef` appears in at most one leaf; `PaneId`s unique.

State transitions: `split` (leaf → Split with new empty-or-unshown pane, focus moves to new pane), `close` (leaf removed, sibling replaces parent, focus → nearest surviving leaf), `swap`, `show`, `prune`, `set_ratio`, `reset_equal`. Last pane never closes.

## Stored form (`StoredPaneLayout`, in `StoredProjectState.pane_layout`)
See [contracts/pane-layout-file.md](contracts/pane-layout-file.md). Loading validates all invariants; any failure → `None` (single pane showing the last session, today's behaviour).

## Client runtime
- `App.pane_layouts: HashMap<project path, PaneLayout>`.
- `App.grids: HashMap<TerminalRef, GridCache>` (was `SessionId`).
- `App.pane_sizes: HashMap<PaneId, (u16,u16)>` (was single `last_grid`).
- Drag state lives in the `SplitView` widget tree state, not in `App`.
