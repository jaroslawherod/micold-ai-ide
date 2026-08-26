# Contract: bottom-bar mode toggle + restart affordance

> **RETIRED for the toggle half, 2026-08-21 (feature 027 FR-001).** The control this contract
> governs no longer exists: feature 026 put the AI CLI process in the bar's tab strip, and 027
> deleted the button that had been the other way there. Everything below about the toggle — its
> placement, its icon-as-mode-indicator, its press behaviour — describes the application as it was
> up to 027 and is kept for that record only. **The restart affordance half is still live** and
> still governs `pane()`; nothing in 027 touches it. What replaces the toggle is specified by
> `specs/027-tabs-only-switching/spec.md` and by 026's `ai-session-tab-ui.md`.

Governs FR-001, FR-002, FR-009, and the spec Clarifications (2026-07-18: toggle placement).
Applies to `pane()` in `src/ui/terminal.rs`, the function that already builds the terminal's
bottom status bar (`bottom_bar`: session title left, status right, ~~conditional "release focus"
button, feature 006~~).

*(Bugfix `012-multiple-regular-terminals` BUG-001: the release-focus button is retired from this bar
— `023-terminal-focus-flow` FR-021b. Nothing this contract governs changes; the references below are
struck because they describe the bar's inventory, not the toggle's own behaviour.)*

## Placement

The toggle is a single `IconButton` (`src/ui/material/icon_button.rs`) added to the existing
`bar` row in `pane()`, alongside the session title/status ~~/release-focus~~ elements — **not** a new
toolbar, **not** a dropdown/overflow menu item. It is present whenever a session's terminal is
displayed (FR-002), in both modes.

## Icon + tooltip (mode indicator, FR-009)

The button's glyph reflects the **current** `Session.mode` (what you're in, not what you'd
switch to — consistent with "the terminal pane MUST display... an indicator of which mode is
currently active"):

| `Session.mode` | Glyph                          | Tooltip (via `Tooltip::new`, `src/ui/material/mod.rs`) |
|-----------------|--------------------------------|----------------------------------------------------------|
| `AiCli`         | new `Icon::AiCli`-family glyph | "AI CLI — switch to Regular Terminal"                    |
| `Regular`       | new `Icon::RegularTerminal`-family glyph | "Regular Terminal — switch to AI CLI"          |

Pressing the button always emits `Message::TerminalModeToggled` (unconditional — no disabled
state; switching is always legal, contracts/terminal-mode-lifecycle.md).

```rust
let toggle = Tooltip::new(
    IconButton::new(mode_glyph(session.mode), roles)
        .on_press(Message::TerminalModeToggled)
        .into(),
    mode_tooltip(session.mode),
    roles,
);
```

## Restart affordance (FR-013)

When the currently-attached process is not running (predicate in
contracts/terminal-mode-lifecycle.md), a second small control appears in the same bar —
~~reusing the existing "release focus" button's `text_button` styling pattern~~ **a `Button` in the
`Text` variant** — labeled to restart
the attached process (e.g. "restart", mirroring the existing status text's terse style: `"idle"`,
`"failed"`, `"restarting…"`). Pressing it emits `Message::TerminalRestartRequested`. It is absent
whenever the attached process is running/starting (nothing to restart).

## Acceptance mapping

- Spec User Story 1, Scenario 1 & 3 / User Story 3, all scenarios ⟵ this contract's toggle.
- Spec Edge Case "plain shell exits... restart affordance" ⟵ this contract's restart control.
- SC-003 (users identify the active mode from the indicator alone) ⟵ the icon+tooltip table
  above being the single, unambiguous source of mode display (no separate indicator element to
  drift out of sync with the button, per the 2026-07-18 clarification).

**Bugfix**: 2026-08-14 — `012-multiple-regular-terminals` BUG-001 The bottom bar loses its
release-focus button (`023-terminal-focus-flow` FR-021b). Three references struck: the bar-inventory
descriptions in the header and Placement, and — the one that would actually have gone stale — the
restart affordance's styling precedent, which pointed at that button rather than naming a style. A
precedent that names another control is only as durable as that control; the restart affordance now
names its own variant directly, which is what it meant all along. Neither the toggle nor the restart
control changes behaviour, and no requirement this contract governs (FR-001, FR-002, FR-009, FR-013)
is affected.
