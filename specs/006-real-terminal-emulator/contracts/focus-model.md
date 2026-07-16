# Contract: Focus Model & Key Routing

Governs FR-009, FR-010, FR-011, FR-012, FR-012a. The goal: keys reach the `claude` process
**only** while the terminal is focused; otherwise they drive the application.

## State

- `State.terminal_focused: bool` (pure core). Default **false**. Only the `active_session`'s
  terminal can be focused; at most one focused terminal.

## Transitions

| From → To | Trigger |
|-----------|---------|
| unfocused → focused | Explicit user action: click on the terminal pane (`Message::TerminalFocused`). (FR-010) |
| focused → unfocused | Click outside the pane; reserved chord `Ctrl+Shift+E` / `Cmd+Shift+E`; or the pane header "release focus" affordance (all → `Message::TerminalFocusReleased`). (FR-011) |
| focused → unfocused | `SessionSelected`, session close, or project switch (re-focus is a fresh explicit action). |

Focus release MUST NOT disrupt or terminate the running session (FR-011).

## Routing rule (the gate)

Let `focused = State.terminal_focused`.

- **`focused == true`**:
  - `ui::subscription(state)` MUST NOT bind the app's global keyboard shortcuts (return
    `Subscription::none()` for key handling) so Esc and app chords are not consumed by the app
    while the terminal owns the keyboard.
  - `TerminalPane::on_event` handles the key: `keymap::encode` → if `ReleaseFocus` emit
    `TerminalFocusReleased`; if `Copy`/`Paste` act on the clipboard; else emit
    `TerminalAction(Write(bytes))`. Returns `Captured`.
- **`focused == false`**:
  - `TerminalPane::on_event` returns `Ignored` for all key events (no `TermAction` emitted); no
    bytes reach any PTY (FR-009).
  - The app subscription/shortcuts handle keys as before (e.g. Esc closes overlays via
    `on_escape`, unchanged).

A pure predicate `route_key(focused, overlay, KeyOutput) -> {App | TerminalWrite(bytes) |
ReleaseFocus | Copy | Paste | Drop}` encodes this and is unit-tested in `tests/terminal_focus.rs`.

## Write gating by lifecycle (FR-012a)

Even when focused, the binary writes `TermAction::Write`/`Paste` bytes to the PTY **only** when
the displayed session is `SessionLifecycle::Running`. In `Starting`/`Restarting`/`Failed`, the
bytes are dropped (no buffering); focus, scroll, selection, and copy still function, and the
pane header shows the session status. Isolation: bytes are applied only to the displayed,
focused session's `RuntimeTerminal` — never a background session (FR-012).

## Visual & tests

- The focused pane MUST show a visible focus indicator (e.g. a border/ring in an accent role),
  and an always-visible affordance to release focus.
- Tests: `route_key` truth table (focused vs not, each `KeyOutput`), incl. `ReleaseFocus` never
  producing PTY bytes; write-gating asserted at the reducer/binary seam using the 005 session
  lifecycle fake.
