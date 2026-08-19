# Contract: bottom-bar "open instance" affordance + instance-switcher row

GUI-only (`src/ui/terminal.rs::pane`). Governs FR-001, FR-004–FR-007, FR-011–FR-013.

## Placement

Both controls live in the same bottom status bar `pane()` already renders (session name, status
text, restart control, ~~release-focus control,~~ primary AI-CLI/Regular mode toggle — feature
010). The primary mode-toggle `IconButton` keeps its existing position, anchoring the bar's
bottom-right corner (unchanged, per the feature 010 spec Clarification); the new controls are
inserted before it in the bar's `row!`, alongside the restart ~~/release-focus~~ control.

*(Bugfix BUG-001: the release-focus control is retired from this bar — see `023-terminal-focus-flow`
FR-021/FR-021b and `006-real-terminal-emulator` `contracts/focus-model.md`. Its removal must be
unconditional: 023 FR-008a requires the bar's child list not to vary with focus, and the gate
`crates/micold-client/tests/terminal_bar_stability.rs::bar_does_not_branch_on_focus` still enforces
that.)*

## "Open a new instance" control

- **Visible** whenever the active session's `mode == TerminalMode::Regular` — regardless of
  `shells.len()` (0, 1, or many). This is what lets a user go from zero or one instance to two
  (spec Assumptions) even though the list/switcher portion below is hidden until there are two.
- **Never visible** while `mode == TerminalMode::AiCli` — there is nothing to open a sibling
  instance of when no Regular Terminal is even displayed.
- On press: dispatches `Message::ShellInstanceOpenRequested` (same message the
  `Ctrl+Shift+T`/`Cmd+Shift+T` chord dispatches — contracts/keyboard-shortcut.md).
- Built from `IconButton::new(Icon::AddTerminalInstance, r)` + `Tooltip`, the same builder
  components the mode toggle and restart control already use in this bar — no new shared
  component (research.md R3).

## Instance-switcher row

- **Visible** only when the active session's `shells.len() > 1` (FR-005). At 0 or 1 instances it
  renders nothing — pixel-identical to today's (feature 010) single-instance experience, matching
  User Story 2 Scenario 1.
- **Contents**: one ~~small entry~~ **tab** per element of `session.shells`, in list order
  (append-on-open — data-model.md), each labeled with its `ShellInstanceId`'s numeric value (the
  spec's own "sequentially numbered" assumption — no separate title source exists for a shell
  instance). *(Bugfix BUG-001: "small entry" left the entries' visual form open, and the row was
  built with a container on the active entry only. Every entry is a tab — FR-004a.)*
- **Tab form** (BUG-001, FR-004a; container rule superseded by BUG-002, FR-004b): ~~every tab —
  active and inactive alike — is a container of the same shape and size.~~ **No tab draws a
  container.** Inside a tab, the label is horizontally centred and the close control is pinned to
  the trailing edge (a fill between them, not a fixed gap). A tab's size does not depend on whether
  it is active, so activation never reflows the row (SC-008) — which now means the *indicator* must
  occupy its space whether or not it is drawn, rather than appearing and pushing the row.
  ~~give the label a minimum width so single- and double-digit ids do not resize their tab either.~~
  **Every tab is one fixed width** (`TAB_WIDTH`), the same whatever it contains (FR-004c). That is
  not cosmetic: the indicator is a rule, and a rule spans the width it is given. Sized inside a
  content-width tab, its `Length::Fill` resolves against the *button's* available space rather than
  the label's — the active tab stretched to several times its neighbour's width, and activation
  resized it under the pointer. The visual pass caught it; no gate could, because every node was
  where its own layout said it was. The figure is set by what must fit: `spacing::SM` either side,
  then two `anatomy::button::MIN_TOUCH_TARGET` widths (the close control, and the leading spacer
  balancing it) plus a readable label. A longer label ellipsises within the tab rather than widening
  it, which is also the behaviour a name needs once instances can be renamed (BUG-002, "Related").
  *(Bugfix BUG-004: that list is the derivation with a child missing from it. A tab that is
  individually restartable carries the **restart affordance** below as a fourth child, wider than
  the other three together, and the figure was set without it — so the affordance laid out at 0.0dp
  and the close control was squeezed to 45.2, below the target the same sentence names. The width
  MUST be **derived** from every child the widest tab can hold, including the restart affordance,
  and MUST NOT be a number chosen against an observed arrangement: a `Length::Fixed` on the tab is a
  budget for its children, and iced settles a shortfall by shrinking the trailing ones rather than
  by overflowing — so the failure is a control disappearing, which no layout invariant reports.
  The leading spacer must likewise balance the whole trailing group, not the close control alone,
  or the label is off centre by a restart button's width on any tab that has one — FR-004a.
  **Resolved by moving the affordance out** (FR-010b): deriving the width with it in comes to 204dp
  against 136, and three instances would then take 628dp of a 1014dp bar. So the tab's children are
  the leading spacer, the label and the close control, the derived width is
  `2·spacing::SM + 2·anatomy::button::MIN_TOUCH_TARGET + 2·spacing::XS + a 16dp label floor = 136`,
  and the spacer balances the whole trailing edge again because the close control is all that is on
  it. The 8dp against the old literal is the label floor the literal did not reserve.)*
- **Active entry** (`session.active_shell == Some(entry.id)`) is marked by an **active indicator**
  — a user must be able to tell which instance is active from this row alone (FR-004, SC-004),
  without opening it. ~~mirroring `TreeItem::selected`~~ ~~The distinction MUST be **container
  versus container** (active: filled/high-emphasis; inactive: low-emphasis container), never
  container versus nothing.~~ *(Both superseded: the `TreeItem` analogy by BUG-001, the
  container-versus-container rule by BUG-002. Each was reaching for "make the active one obvious"
  without naming the idiom, and a tab strip's idiom is an indicator.)*
- **Active indicator** (BUG-002, FR-004b): an accent bar spanning the active tab's width, thick
  enough to read at a glance rather than a hairline, drawn at the tab's **top** edge — this bar is
  anchored to the window's bottom, so the pane a tab selects is above it and a bottom indicator
  would point away from what it marks. The active tab's label additionally takes the accent colour,
  so the cue is carried twice: without a container, one thin line alone is what the original
  rationale rightly warned against (SC-004, SC-009). Inactive tabs are low-emphasis labels — no
  container, no indicator. Every tab reserves the indicator's height whether or not it draws one,
  so activation moves colour and nothing else (SC-008).
- Clicking an entry's body dispatches `Message::ShellInstanceSelected(entry.id)`.
- Each entry carries its own close action (a small trailing icon/button, reusing ~~`Icon::Delete`
  — the same icon feature 008's delete affordance already uses~~ **`Icon::Close`**) dispatching
  `Message::ShellInstanceCloseRequested(entry.id)` (FR-011). *(Bugfix BUG-001: the implementation
  has always used `Icon::Close`, which is the right glyph for dismissing a tab — `Icon::Delete` is
  feature 008's destructive-delete affordance. The contract is corrected to match rather than the
  code changed to match the contract.)*
- **Nested-control colour** (BUG-001, FR-011a): the close control — and the per-entry restart
  affordance below — takes the **tab's own** foreground colour, so it reads at the same emphasis
  as that tab's label in every state. It must not keep a tint chosen for the surrounding bar's
  background: on the active tab's fill that pairing is near tone-on-tone and the control
  disappears (SC-007). Note that `IconButton` defaults its tint to the roles' `on_surface`; inside
  a filled container that default is wrong and must be overridden explicitly.
- Each entry that is individually "restartable" (its own `lifecycle ∈ {NotStarted, Exited}`,
  contracts/shell-instance-lifecycle.md) shows its own restart affordance, dispatching
  `Message::ShellInstanceRestartRequested(entry.id)` — **independent** per entry, and (BUG-004,
  FR-010a) **laid out at its full size**, since "shows" is satisfied by a control the user cannot
  press and FR-010 is not. *(BUG-004, FR-010b: it is **not drawn in the tab**. A secondary (right)
  press on a tab opens a context menu for that instance — "Restart" when its own lifecycle offers
  it, "Close" always — anchored at the press point on the window's overlay. Not the pane's: that
  hosts the terminal's own copy/paste menu because its anchor is pane-local, and a tab's press point
  is already in window space. The wrapper that reports the press is `ui/cdk/context_area.rs`, which
  intercepts only the right button so the tab's own `on_press` still selects the instance.)*
  ~~shows its own restart affordance,~~ the switcher's remaining nested control is the close; the existing
  session-level restart control in this same bar continues to reflect only the *currently
  attached* process's overall restartability (AI CLI in `AiCli` mode; the active shell instance
  in `Regular` mode) and is unaffected by a background sibling instance's state.

## Interaction with the primary mode toggle

Unchanged from feature 010: the primary toggle only flips `Session.mode`. It never touches
`shells`/`active_shell` — switching into Regular mode shows whichever instance `active_shell`
already names (FR-007), lazily calling `open_shell_instance()` only if `shells` is currently
empty (first-ever switch for that session, or the state just after the last instance was closed
and `mode` fell back to `AiCli`, FR-013).

## Session status text (bar's left/center text, feature 010, extended)

In `Regular` mode, the status text reflects the **currently active instance's** lifecycle only
(`session.active_shell_lifecycle()`, data-model.md) — not an aggregate of all open instances'
states. A background sibling instance exiting does not change this text; only its own row entry
in the switcher (above) reflects that.
