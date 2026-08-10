# Phase 1 Data Model: Natural Terminal Focus Flow

Scope: `crates/micold-client/src/app.rs`. Nothing in `micold-core` or the wire protocol changes —
focus is a property of this client's window, not of the session model the daemon shares.

## Stored state

| Field | Type | Default | Persisted | Meaning |
|---|---|---|---|---|
| `terminal_released` | `bool` | `false` | no | The user has explicitly handed the keyboard from the terminal to the application. **New** — replaces `terminal_focused`. |
| `focused_field` | `Option<FieldId>` | `None` | no | Which text field holds the keyboard. **Exists** (BUG-003); this feature is its second consumer. |
| `overlay` | `Overlay` | `None` | no | The open modal, if any. **Exists**, unchanged. |
| `help_menu_open`, `project_switcher_open`, `sidebar_filter_open` | `bool` | `false` | no | Open popovers. **Exist**, unchanged. |
| `project_menu_open`, `worktree_menu_open`, `session_menu_open` | `Option<…>` | `None` | no | Open context menus. **Exist**, unchanged. |
| `active_session` | `Option<SessionId>` | `None` | no (derived on restore) | The displayed session. **Exists**, unchanged. |
| `terminal_context_menu` | `Option<(u16, u16)>` | `None` | no | The pane's own right-click menu. **Exists** — deliberately *not* a term of the predicate (research R4). |

**Removed**: `pub terminal_focused: bool`. Its seven assignment sites collapse into two intent-named
helpers (below) and one derived answer.

Neither focus fact is written to disk. `terminal_released` is one moment's decision (spec
Clarification Q1) and launch re-derives focus from what is displayed (FR-012a).

## Derived state

```rust
impl State {
    /// Whether the displayed session's terminal holds the keyboard (FR-009).
    /// The single answer; nothing stores it.
    pub fn terminal_focused(&self) -> bool {
        self.active_session.is_some()
            && !self.terminal_released
            && self.focused_field.is_none()
            && self.overlay == Overlay::None
            && !self.any_menu_open()
    }

    /// Any popover or context menu that takes the keyboard while it is open (FR-004).
    /// `terminal_context_menu` is excluded — it is pane furniture (FR-007, research R4).
    fn any_menu_open(&self) -> bool {
        self.help_menu_open
            || self.project_switcher_open
            || self.sidebar_filter_open
            || self.project_menu_open.is_some()
            || self.worktree_menu_open.is_some()
            || self.session_menu_open.is_some()
    }
}
```

`any_menu_open()` is the fifth term and belongs to User Story 4. It lands as a stub returning `false`
with the rest of the predicate, and is filled in by that story so its tests are observed failing
first (Principle I).

### Invariants the predicate makes structural

| Invariant | Requirement | Why it cannot be violated |
|---|---|---|
| No terminal holds the keyboard when none is displayed | FR-012, FR-016, FR-020 | `active_session.is_some()` is a conjunct |
| A text field and the terminal never both hold it | FR-018 | `focused_field.is_none()` is a conjunct |
| A dialog and the terminal never both hold it | FR-017 | `overlay == None` is a conjunct |
| Only the displayed session's terminal is eligible | FR-020 | `active_session` is the only session the predicate names |
| Output never changes the holder | FR-019 | No term of the predicate is written by output or lifecycle |

## Mutations

Two intent-named helpers are the only writers of `terminal_released`. A source-level assertion in
`tests/terminal_bar_stability.rs` (`no_scattered_release_writes`) keeps it that way — it scans all of
`crates/micold-client/src/**/*.rs`, not just `app.rs`, because the helpers are `pub(crate)` and
`features/session.rs` calls them. Seven scattered assignments are what this feature is undoing, and
the gate is what stops the eighth.

```rust
/// The user is being put in front of a terminal (FR-011, FR-021a, FR-008b).
///
/// Clears the explicit release *and* any text-field focus: a press on the pane, or a navigation
/// that displays a terminal, is a request for that terminal — it must not be defeated by a field
/// that still believes it holds the keyboard. Without the second line the predicate stays false
/// after a press into the pane made while the sidebar filter had focus, and FR-008b is unmet.
pub(crate) fn focus_terminal(&mut self) {
    self.terminal_released = false;
    self.focused_field = None;
}

/// The user handed the keyboard back to the application (FR-021).
pub(crate) fn release_terminal(&mut self) { self.terminal_released = true; }
```

`pub(crate)` rather than private: `restore_after_activation` lives in `features/session.rs`, a
sibling module, so a private method on `State` is not reachable from it. The constraint that matters
is the gate above, not the visibility.

Clearing `focused_field` here composes safely with the existing blur path: `FieldFocusChanged(id,
false)` is guarded by `if self.focused_field == Some(field)`, so a blur arriving after the press is a
no-op rather than a second write.

### Transition table

| Trigger | Effect | Requirement |
|---|---|---|
| `SessionStarted` | `focus_terminal()` | FR-011 |
| `SessionSelected` | `focus_terminal()` | FR-011 |
| `TerminalModeToggled` | `focus_terminal()` | FR-011 |
| `ShellInstanceOpenRequested` | `focus_terminal()` | FR-011 |
| `ShellInstanceSelected` | `focus_terminal()` | FR-011 |
| `ShellInstanceCloseRequested` | `focus_terminal()` | FR-011 |
| `restore_after_activation` (project switch) | `focus_terminal()` — was `terminal_focused = false` | FR-011 |
| Application launch | nothing: `Default` is `terminal_released: false` | FR-012a |
| `TerminalFocused` (press on the pane) | `focus_terminal()` — clears the release **and** `focused_field`, so a press into the pane wins over a field that had the keyboard | FR-008b, FR-018, FR-021 |
| `TerminalFocusReleased` (chord or affordance) | `release_terminal()` | FR-021 |
| `FieldFocusChanged(id, true)` | `focused_field = Some(id)` (existing) ⇒ predicate false | FR-004 |
| `FieldFocusChanged(id, false)` | `focused_field = None` (existing, guarded) ⇒ predicate true again unless released | FR-010 |
| `open_overlay(_)` | `overlay = …`, `focused_field = None` (existing) ⇒ predicate false | FR-017 |
| Overlay closed | `overlay = None` ⇒ predicate true again unless released | FR-010 |
| Session closed / removed / worktree deleted | `active_session = None` (existing) ⇒ predicate false. The two `terminal_focused = false` lines that accompanied it are deleted as redundant | FR-012 |
| Window blur / focus (`WindowFocusChanged`) | **nothing** — no term is touched, so the holder survives the round trip | FR-013–FR-015 |
| Terminal output, lifecycle change, background session activity | **nothing** | FR-019 |

### Explicit release vs navigation

`terminal_released` outranks the default (FR-021) but is cleared by every navigation in the table
(FR-021a): it is one application-wide fact about the present moment, not a property a session
carries. So a release survives a dialog round-trip and a window switch, and does not survive the
user deliberately going to a terminal.

## Entities from the spec, mapped

| Spec entity | Runtime representation |
|---|---|
| Keyboard holder | Not stored. Answered by `terminal_focused()` and, for everything else, by whichever of `focused_field` / `overlay` / menu flag is set |
| Displayed terminal | `active_session` + that session's `mode` / `active_shell` (existing, unchanged) |
| Transient holder | `focused_field`, `overlay`, and the menu flags — each already ends itself, which is why FR-010 needs no restore stack |
| Explicit release | `terminal_released` |
| Suspended holder | **No runtime existence.** Nothing mutates on window blur, so there is nothing to suspend or restore. Recorded here because the spec names the entity and a reader will look for it |

## Pure functions (unchanged signatures unless noted)

| Function | Change |
|---|---|
| `route_key(terminal_focused: bool, KeyOutput) -> KeyRouting` | None. Callers pass `state.terminal_focused()` instead of the field (FR-022/FR-023) |
| `press_routing(focused: bool, MouseMode, shift) -> PressRouting` | None. The caller passes `focused_now` — true also for the press that grants focus (FR-008b, research R5) |
| `press_grants_focus(focused, is_left_press, over_bounds) -> bool` | **New**, `pub(crate)` in `ui/material/terminal_pane.rs`, unit-tested inline. `!focused && is_left_press && over_bounds`. It exists so the granting press's answer is a tested rule rather than a branch inside `Widget::update`, which Principle I's GUI-wiring exception does not cover |
| `on_escape(&State) -> Option<Message>` | None |
| `ui::subscription(&State)` | Reads `state.terminal_focused()`; logic unchanged |
