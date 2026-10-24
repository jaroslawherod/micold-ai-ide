# Contract: the session area and its tab strip (`features::session_strip`, `ui/session_area.rs`)

## Strip rules (render-free; each row is a client unit test of `StripModel::of`)

### S1. Members and order (FR-008, Clarifications 1 and 2)

Location with main `M` (2 shell instances, AI tab marked) and helpers `H1`, `H2`; `M` shown:

`[Session(M, shown, marked), Shell(M,1), Shell(M,2), Plus, Session(H1), Session(H2)]`

`H1` shown instead: `[Session(M), Session(H1, shown), <H1's shells>, Plus, Session(H2)]`. Another
session's shell tabs are never members (US2 scenario 7). A session an agent started follows the
session that started them, folded or not (`unfolded_sessions`). One session, no shell instance:
`[Session(M, shown, marked), Plus]`.

### S2. Marked (US2 scenarios 2 and 7)

Exactly one member is marked: the shown session's own tab when its mode is AI (or Regular with no
active instance), else its active shell tab. No member of another session is marked.

### S3. Stopped (US2 scenario 5, edge case *several sessions*)

A session tab is stopped when that session's AI process is `Idle`, `Failed` or
`InterruptedResumable`; a shell tab when that instance is `NotStarted` or `Exited` or missing. A
stopped `H1` never marks `M`'s tab, and the reverse. `restart` is `Some` exactly when the shown
session's marked tab is stopped, and it targets that process (`restart_message`, feature 012
BUG-004).

### S4. Status (US2 scenario 6)

`status` is the shown session's words: *running*, *starting…*, *restarting…*, *failed*,
*failed after N attempts*, *idle*, *interrupted* (AI mode); *running*, *starting…*, *exited*,
*idle* (Regular mode) — today's `session_status`, unchanged.

### S5. Presses (US2 scenarios 2 and 3)

| Member | Press |
|---|---|
| shown session's tab | `Session(TerminalAiCliSelected(id))` |
| another session's tab | `Session(Selected(id))`, as its sidebar row |
| shell tab | `Session(ShellInstanceSelected(session, instance))` (another pane showing it is focused, feature 484) |
| shell tab's close | `Session(ShellInstanceCloseRequested(session, instance))` |
| `Plus` | `Session(ShellInstanceOpenRequested)` |
| **New session** | `Session(AdditionalRequested(location))` (confirmation included; feature 776) |
| split side by side / stacked | `Session(Pane(Split(focused pane, axis)))` |
| restart | `restart` |

Every strip control that changes the marked tab arms the reveal (`arm_tab_reveal`), as today
(`tests/tab_reveal.rs`).

### S6. Overflow and reveal arithmetic (FR-010)

`member_offsets(&[Member]) -> Vec<(f32, f32)>` (leading, trailing) from the fixed widths
(`SESSION_WIDTH`, `tab::WIDTH`, the "+" size) and `spacing::SM` gaps; `strip_overflow(offset,
viewport, members)` and `scroll_into_view(index, offset, viewport, members)` read it. Viewport 0
means not measured: no edge is beyond. The marked-beyond cue accents the faded edge on the side the
marked member lies (feature 027 FR-002e).

| Members (widths) | Viewport | Offset | Leading beyond | Trailing beyond |
|---|---|---|---|---|
| 1 session + plus | 340 | 0 | no | no |
| 12 members | 340 | 0 | no | yes |
| 12 members | 340 | end | yes | no |

## Strip layout (glue)

### S7. Structure (FR-008, FR-009, FR-010)

`row![ EdgeFade(Scrollable(Keyed::row(members))) Fill, status_cluster, new_session, split_buttons ]`
with a child list that never varies. `status_cluster` is `row![Ellipsized(status) capped at
STATUS_MAX_WIDTH, restart?]` with the restart last. Session tabs `Tab::new(..)` at `SESSION_WIDTH`
with the stopped mark in the leading slot and an empty reserved trailing slot; shell tabs as today
(`Tab` at `tab::WIDTH`, close in the trailing slot). Indicator edge `Bottom` (the strip sits above
the panes it selects; feature 012 FR-004b's rule, inverted with the strip's position).

### S8. Stability (FR-011, FR-012)

Source-level gates (`tests/session_area_stability.rs`): the strip's and the header's builders read no
focus (`terminal_focused`) and push no child under an `if` on session shape; no release-focus
control. A press on a member survives a member added before it (keyed row unit test in
`cdk/keyed.rs`).

### S9. Fit (FR-013, SC-004)

At 640×480 (sidebar 300 px) and 1280×800, light and dark, with 1, 6 and 12 members: every member and
fixed control is laid out at its requested width (`gates/strip_controls_hold_their_size.rs`,
`gates/tab_children_fit.rs`); New session and the split buttons are the strip's last children, flush
with its trailing padding (`gates/strip_controls_anchor_the_trailing_edge.rs`); no element of the
header or strip overlaps another or leaves the window (`gates/session_area_fits.rs`).

## Session area (glue)

### A1. Order (FR-001, SC-005)

With a session shown: header, strip, pane area, in that order, and no element below the pane
area's bottom edge inside the session area (gate). Changes view open: header, then the Changes view.
Overview open: the Overview only. No session: today's empty message, no header, no strip.

### A2. Panes (edge case *several panes*)

Header and strip are drawn once above `panes::view`; each pane keeps its header (name, state, split
buttons, close; `ui/panes.rs::header` unchanged).
