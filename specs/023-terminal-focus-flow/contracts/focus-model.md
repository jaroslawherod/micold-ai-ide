# Contract: Focus Model & Key Routing (v2)

**Supersedes** `specs/006-real-terminal-emulator/contracts/focus-model.md`, including its BUG-001
amendment. That contract's routing rule and write gate survive verbatim; what changes is *when* the
terminal holds the keyboard. Feature 006's file should be left in place with a pointer to this one.

Governs FR-001–FR-026 of feature 023, and continues to govern 006's FR-009–FR-012a.

## The rule in one line

The displayed terminal holds the keyboard unless the user gave it away or something that types took
it.

## State

- `State.terminal_released: bool` — the user's explicit release. Default `false`. One
  application-wide fact, not per session.
- `State::terminal_focused() -> bool` — **derived, never stored**:

  ```text
  active_session.is_some()
    && !terminal_released
    && focused_field.is_none()
    && !any_surface_takes_keyboard()
  ```

- `any_surface_takes_keyboard()` asks the overlay registry, not a list of flags:
  `registry::open_dialog(state).is_some()`, or any surface in `registry::open_popovers(state)` whose
  `SurfaceId` is not `"terminal_context_menu"`. The pane's own right-click menu is pane furniture
  (FR-007) — taking the keyboard to open it would stop the user typing — and it is the only
  exclusion; the project, worktree and session context menus all take it. Reading the registry
  rather than naming flags means a surface registered later participates automatically, which is
  feature 024's FR-009 ("one line per surface, and this is the only such list") doing the work
  research R2 asked for.

At most one terminal is ever focused, and only the displayed session's (FR-020) — structural, since
`active_session` is the only session the predicate names.

## Transitions

| From → To | Trigger |
|-----------|---------|
| released → focused | Any navigation that puts a terminal in front of the user: `SessionStarted`, `SessionSelected`, `TerminalModeToggled`, `ShellInstanceOpenRequested`, `ShellInstanceSelected`, `ShellInstanceCloseRequested`, project switch (`restore_after_activation`). Each clears `terminal_released` (FR-011, FR-021a). |
| released → focused | A press on the pane (`Message::TerminalFocused`), which also acts on the terminal — see *The granting press* below (FR-008b). |
| focused → released | The reserved chord `Ctrl+Shift+E` / `Cmd+Shift+E`, or the release affordance in the pane's status bar (`Message::TerminalFocusReleased`) (FR-021). |
| focused → not focused | A text field takes focus (`FieldFocusChanged(_, true)`), an overlay opens, or a menu opens (FR-004, FR-017, FR-018). Not a release: `terminal_released` is untouched. |
| not focused → focused | That field blurs, that overlay closes, that menu closes — unless the user had released the terminal, in which case it stays released (FR-010). No restore stack: the predicate simply reads true again. |
| focused → not focused | The displayed session goes away (`active_session = None`) (FR-012, FR-016). |
| launch | `Default::default()` has `terminal_released: false`, so a restored displayed session is focused (FR-012a). |
| window blur / focus | **No transition.** Nothing is written, so nothing needs restoring (FR-013–FR-015). |
| output, lifecycle change, background session activity | **No transition** (FR-019). |

Focus release MUST NOT disturb the attached process — no restart, no interruption, no lost output,
no resize (FR-025, 006 FR-011).

## What a press does

| Press lands on | Focus | Action |
|---|---|---|
| A control that types (text field, or a menu/dialog that opens on the press) | That control takes it; the terminal yields | The control acts on that press (FR-004) |
| A control that types nothing (icon button, toggle, action menu item) | **Unchanged** | The control acts on that press (FR-005) |
| Non-interactive space | **Unchanged** | Nothing (FR-006) |
| A disabled control | **Unchanged** | Nothing (FR-008) |
| The pane's furniture — scrollbar, status bar, context menu | Terminal holds it (granted if it did not) | The furniture acts (FR-007) |
| The pane, while it does not hold the keyboard | Terminal takes it | The press acts in full (FR-008b) |

Two prohibitions carry the weight:

- **No press outside the pane may reach the attached process** (FR-003).
- **No press may be consumed solely to grant focus** (FR-008b), and no control may need a second
  press because the terminal happened to hold the keyboard (FR-002).

### The granting press

`TerminalPane::update` publishes `Message::TerminalFocused` for a left press over its bounds while
unfocused, and then routes that same press as if focused. Both halves are pure functions, unit-tested
inline; `Widget::update` holds no rule of its own:

```text
grants      = press_grants_focus(self.focused, is_left_press, over_bounds)  // !focused && left && over
focused_now = self.focused || grants
press_routing(focused_now, mouse_mode, shift)
```

Without `focused_now`, the press is routed on the previous view's `false` and a mouse-aware program
never sees it — the reported bug's mirror image.

`Message::TerminalFocused` reaches the reducer as `focus_terminal()`, which clears the explicit
release **and** `focused_field`. A press on the pane while a text field held the keyboard therefore
gives the terminal the keyboard on that press (FR-008b), which FR-018 permits precisely because it is
a user press.

## No intermediate holder (FR-008a)

A press or navigation moves the keyboard **directly** from its old holder to its new one. Forbidden,
explicitly:

- releasing focus and re-asserting it within one interaction (the shape BUG-001's two
  `Task::done(Message::TerminalFocused)` calls in `src/main.rs` took — both deleted, since the race
  they won no longer exists);
- any view in which the focus indication is drawn for a holder the user did not ask for, however
  briefly.

**Structural precondition.** The terminal's bottom bar MUST NOT add or remove children as a function
of focus. A focus-conditional child shifts every sibling after it, and iced's positional tree diff
then drops the pressed sibling's `is_pressed` state, swallowing the click (research R1). The
release-focus affordance is therefore always present, with only its `on_press` gated. Enforced by
`tests/terminal_bar_stability.rs`.

## Routing rule (the gate) — unchanged from 006

Let `focused = State::terminal_focused()`.

- **`focused == true`**:
  - `ui::subscription(state)` MUST NOT bind the app's global keyboard shortcuts (returns
    `Subscription::none()`), so Esc and app chords are not consumed by the app while the terminal
    owns the keyboard.
  - `TerminalPane::update` handles the key: `keymap::encode` → `route_key` → `ReleaseFocus`,
    `Copy`/`Paste`, `NewTerminalInstance`, or `TerminalAction(Write(bytes))`. Captured.
- **`focused == false`**:
  - `TerminalPane::update` returns without capturing any key event; no bytes reach any PTY
    (FR-023, 006 FR-009).
  - The app subscription/shortcuts handle keys as before.

`route_key(focused, KeyOutput) -> KeyRouting` keeps its signature and its tests; only its argument's
provenance changes (a call to the predicate, not a field read).

## Write gating by lifecycle — unchanged from 006

Even when focused, bytes reach the PTY only while the displayed session is `Running`; in
`Starting`/`Restarting`/`Failed` they are dropped, with no buffering (the daemon holds this gate).
Automatic focusing does not change it: a terminal may hold the keyboard in front of a process that
is not running, and what the user types is discarded exactly as today.

## Visual

- The focused pane shows a visible focus indicator, and the release affordance is always present in
  the status bar — enabled only while focused (FR-024).
- The indication MUST match the actual holder at every observed moment, including transitions the
  user did not initiate by pressing, and MUST show no intermediate state (FR-024, FR-008a).

## Tests

- `tests/terminal_focus.rs` — the predicate's truth table over its four terms; every navigation
  clearing a release; a release surviving a dialog round-trip and a window switch; launch focusing a
  restored session; `route_key` unchanged.
- `src/ui/material/terminal_pane.rs`'s inline `mod tests` — `press_routing`'s existing truth table,
  the new `press_grants_focus` (FR-008b), and the assertion that no press outside the pane's bounds
  produces a `TerminalAction(Write(..))` for any button, mouse mode, or modifier (FR-003, SC-008).
  Inline because both functions are `pub(crate)`.
- `tests/terminal_bar_stability.rs` — the bottom bar does not branch on focus (FR-002/FR-008a's
  structural precondition), in the shape of `tests/showcase_glue.rs`.
- `quickstart.md` Part B — the visual pass, run headlessly with the `visual-pass` skill.
