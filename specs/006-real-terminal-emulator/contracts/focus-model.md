# Contract: Focus Model & Key Routing

> **Superseded by [`specs/023-terminal-focus-flow/contracts/focus-model.md`](../../023-terminal-focus-flow/contracts/focus-model.md) (v2)**, including this file's BUG-001 amendment.
>
> What survives verbatim: the **routing rule** (a focused terminal takes the keys and the app's
> shortcuts stand down; an unfocused one lets no key reach any PTY) and the **write gate** (bytes
> reach the PTY only while the session is `Running`, dropped otherwise, never buffered). Feature 023
> changes *when* the terminal is focused, not what being focused means.
>
> What v2 replaces: everything in the State and Transitions sections below. `State.terminal_focused`
> is no longer a stored field — it is derived from `active_session`, an explicit
> `terminal_released`, `focused_field`, and the overlay registry — so the click-outside release, the
> project-switch rule, and the auto-focus list here are all restated there. Read this file for the
> history; read v2 for the behaviour.

Governs FR-009, FR-010, FR-011, FR-012, FR-012a. The goal: keys reach the `claude` process
**only** while the terminal is focused; otherwise they drive the application.

## State

- `State.terminal_focused: bool` (pure core). Default **false**. Only the `active_session`'s
  terminal can be focused; at most one focused terminal.

## Transitions

| From → To | Trigger |
|-----------|---------|
| unfocused → focused | A session becomes the displayed session — `SessionStarted` or `SessionSelected` — auto-focuses that session's terminal (bugfix BUG-001, FR-010/FR-010a). |
| unfocused → focused | Explicit user action: click on the terminal pane (`Message::TerminalFocused`). (FR-010) |
| focused → unfocused | Click outside the pane; reserved chord `Ctrl+Shift+E` / `Cmd+Shift+E`; or the pane header "release focus" affordance (all → `Message::TerminalFocusReleased`). (FR-011) |
| focused → unfocused | Session close (the displayed session goes away). |
| focused → unfocused | Project switch/open: focus does not carry across; the restored session (if any) is displayed unfocused until the user selects/starts it or clicks (BUG-001). |

Focus release MUST NOT disrupt or terminate the running session (FR-011).

**Bugfix BUG-001 — auto-focus on select/start.** The prior rule that `SessionSelected` *cleared*
focus is superseded: selecting (or starting) a session now *focuses* that session's terminal so
the user can type into the AI CLI immediately. Because clicking a sidebar item is a click *outside*
the pane (which by FR-011 publishes `Message::TerminalFocusReleased`), the implementation MUST
ensure the auto-focus of the newly-selected session wins over that click-outside release — e.g. the
`SessionSelected`/`SessionStarted` reducer sets `terminal_focused = true` and this is applied after
any release produced by the same click. The routing rule and the `Running`-only write-gate below
are unchanged, so auto-focus never lets keys reach a non-`Running` or background process, and the
release mechanisms still guarantee the user is never trapped.

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
