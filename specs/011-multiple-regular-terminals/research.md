# Phase 0 Research: Multiple Regular Terminal Instances per Session

All Technical-Context unknowns are resolved below. Evidence was gathered by reading the current
implementation (`src/session.rs`, `src/app.rs`, `src/main.rs`, `src/keymap.rs`, `src/icons.rs`,
`src/store.rs`, `src/ui/terminal.rs`, `src/ui/material/terminal_pane.rs`,
`src/ui/material/tree_view.rs`, `src/ui/sidebar.rs`) and the prior feature 010 spec/plan/research
this feature extends.

## R1 — Identity for a shell instance: a per-session monotonic counter, not a UUID or bare index

**Decision**: Add `ShellInstanceId(pub u32)`, allocated from a per-`Session` monotonic counter
(`next_shell_id`, starting at 1) that is never decremented or reused, even after the instance it
named is closed:

```rust
pub struct ShellInstanceId(pub u32);

pub struct ShellInstance {
    pub id: ShellInstanceId,
    pub lifecycle: ShellLifecycle,
}
```

**Rationale**: Unlike `SessionId` (a UUID — must be globally unique across the whole app, and is
the `claude --resume` handle), a shell instance's id only ever needs to be unique *within its
own session*, and only for as long as the app runs (spec: no persistence of instances across
restart, FR-017). A small monotonic counter satisfies that with no dependency, and — because the
spec's own Assumptions section says the number *is* the display label ("identified by their
creation order, e.g. sequentially numbered") — reusing the id as the switcher row's label
collapses two concerns (stable identity, display order) into one field instead of keeping them
in sync separately. A bare `Vec` index was rejected outright: closing instance 2 of 3 would
silently renumber instance 3 to index 1, so a background `Message::ShellInstanceRunning(id)` or
`ShellInstanceExited(id)` racing a close could land on the wrong instance — exactly the kind of
identity/position conflation Principle V's "make invalid states unrepresentable" argues against.

**Alternatives considered**: `Uuid` (like `SessionId`) — rejected as unnecessary weight for an
identity that never leaves the process and never needs global uniqueness; it would also make the
switcher row's label a separate, easy-to-drift field instead of the id itself.

## R2 — Storage shape: `Vec<ShellInstance>` (pure) + `HashMap<ShellInstanceId, RuntimeTerminal>` (gui)

**Decision**: The pure core (`Session.shells`) keeps instances in an ordered `Vec<ShellInstance>`
— order matters for FR-012's "next in list, else previous" rule and for the switcher row's
display order (append-on-open, per the spec's Assumptions). The gui-side live-process map
(`SessionTerminals`, `src/ui/terminal.rs`) keys its `RuntimeTerminal`s by `ShellInstanceId` in a
`HashMap` instead — process handles don't need to be ordered (the `Vec` on `Session` already
carries order; a session's process map is only ever looked up by id), and a `HashMap` avoids an
`O(n)` scan on every write/pump/exit-check as the instance count grows.

**Rationale**: Splitting "ordering + lifecycle" (pure, small, `Clone`/`Eq`) from "the actual
live process handles" (gui-only, not `Clone`/`Eq`, one per id) is exactly the same pure/gui split
feature 010 already drew for the single-shell case (`ShellLifecycle` on `Session` vs.
`Option<RuntimeTerminal>` on `SessionTerminals`) — this just widens both sides from a single slot
to a keyed collection instead of introducing a new split.

**Alternatives considered**: A single `Vec<(ShellInstanceId, RuntimeTerminal)>` on the gui side,
mirrored 1:1 with the pure `Vec` — rejected: keeping two parallel `Vec`s in sync (same order,
same length, same ids at the same positions) after a close/reorder is exactly the kind of
invariant that's easy to violate by hand; a `HashMap` keyed by the same id the pure side already
uses needs no positional syncing at all.

## R3 — The switcher row: compose from existing primitives, don't force `TreeView` into a status bar

**Decision**: Build the "open new instance" button and the instance-switcher row directly in
`src/ui/terminal.rs::pane()` from the same shared, builder-API primitives (`IconButton`,
`Tooltip`) already used for that function's mode toggle, restart control, and release-focus
control — plain `iced::widget::row!`/`button`/`text` composition, no new `src/ui/material/`
component.

**Rationale**: `TreeView`/`TreeItem` (`src/ui/material/tree_view.rs`) is the project's existing
reusable "list of items + one active index" primitive (used by the sidebar for worktrees →
sessions), and the spec explicitly calls out reusing that *pattern*. But `TreeView` is built for
an indented, vertically-stacked, expand/collapse hierarchy — forcing it into a slim horizontal
bottom-bar row (no nesting, no expand/collapse, numbered chips rather than tree rows) would
misuse a component outside the shape it was designed for, which is its own kind of violation of
Principle VIII's reuse intent. The bar already contains three other controls
(mode toggle, restart button, release-focus button) built the same inline way from
`IconButton`/`Tooltip`/`button`/`row!` — reusing that established local pattern is more faithful
to "don't fork a bespoke one-off" than introducing a mismatched use of an unrelated shared
widget. The *behavioral* pattern (ordered list + one active index) is still reused — at the data
layer (`Session.shells` + `active_shell`, R2) exactly as `Vec<Session>` + `active_session`
already works — just not the sidebar's specific tree-shaped widget.

**Alternatives considered**: Promoting a new generic "chip list / selector strip" component into
`src/ui/material/` for this and any future compact-switcher need — rejected for now as premature
abstraction for a single call site; Principle VIII's promotion clause applies when a shared need
is discovered, and today there is exactly one place that needs this shape. If a second compact
switcher need appears later, promoting the two call sites' common shape into a shared component
then is the right sequencing.

## R4 — The new-instance keyboard shortcut: a new `KeyOutput` variant, same precedence tier as the existing reserved chords

**Decision**: Extend `src/keymap.rs`'s `KeyOutput` with `NewTerminalInstance`, detected by a new
`is_new_terminal_chord(key, mods)` predicate checked in `encode()` at the same precedence tier
as `is_release_chord` (before printable-character handling, so `Ctrl+Shift+T` never falls through
to "type a literal T"), following the exact same `cfg(target_os = "macos")` platform split
(`Cmd+Shift+T` on macOS, `Ctrl+Shift+T` elsewhere). `TerminalPane`'s key-event handler
(`src/ui/material/terminal_pane.rs`) gets one new match arm publishing
`Message::ShellInstanceOpenRequested`, alongside its existing `ReleaseFocus`/`Copy`/`Paste` arms
— meaning, like those chords, it only fires while the terminal pane holds keyboard focus
(`self.focused`), not as a global application-wide shortcut.

**Rationale**: `is_release_chord`/`copy_paste_action` are the project's one existing pattern for
"a reserved chord that must never reach the PTY as bytes"; a third reserved chord is additive,
not a new mechanism. Keeping the chord *detection* pure and total in `src/keymap.rs` (it doesn't
know or care about `TerminalMode`) while letting the **binary**-side reducer for
`ShellInstanceOpenRequested` decide whether the session is actually in Regular mode before
opening anything is what makes the spec's edge case ("pressing the shortcut in AI CLI mode does
nothing, and does not also switch modes") a one-line guard at the call site rather than new
keymap-level state.

**Alternatives considered**: A window-level/global shortcut independent of pane focus — rejected
for consistency: every other reserved chord in this app (release-focus, copy, paste) is
pane-focus-gated, and introducing the one exception here would be a surprising asymmetry with no
stated requirement driving it.

## R5 — Persistence: unchanged from feature 010, by design

**Decision**: No `StoredSession` field is added, changed, or removed. `mode:
StoredTerminalMode` remains the only persisted terminal-related field.

**Rationale**: The spec's non-goal is explicit ("no new persistence of terminal instances across
app restart beyond whatever already exists for the single-terminal case today") and FR-017
requires reopening a session to resume with **at most one** freshly-started instance regardless
of how many were open before — i.e., exactly feature 010's existing restart behavior, unchanged.
Adding any instance-count/id persistence would both contradict the spec and be dead weight, since
nothing reads it back.

**Alternatives considered**: Persisting the instance count (but not process state) so a restart
could recreate "however many empty instances there were" — explicitly rejected by the spec's
Edge Cases section ("the prior instance count is not restored").
