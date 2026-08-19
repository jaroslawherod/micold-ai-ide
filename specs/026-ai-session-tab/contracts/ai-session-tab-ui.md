# Contract: the AI tab, the always-visible strip, and its overflow

GUI-only (`src/ui/terminal.rs::pane` and `instance_switcher_row`). Governs FR-001–FR-012e.

**Extends** `012-multiple-regular-terminals/contracts/terminal-instance-switcher-ui.md`. Where the
two disagree, this one wins for the strip's *visibility* (FR-003 supersedes 012 FR-005) and for the
strip's *membership* (FR-001). Everything that contract says about tab form, fixed width, the
indicator's top edge and the close control is unchanged and is inherited.

## Membership and order

The strip's members, left to right:

```text
[ instance 1 ][ instance 2 ] … [ instance N ]   |   [ AI ]
└──────────── scrolling region ────────────┘       └ pinned ┘
```

- One tab per open instance, in the session's own list order (unchanged from 012).
- The AI tab last, always present, **outside** the scrolling region (FR-002, FR-002b).
- The strip is rendered whenever a session is displayed, including at zero and one instances
  (FR-003). At zero instances it is the AI tab alone, marked.

## The AI tab

| Property | Value | Requirement |
|---|---|---|
| Label | `Icon::AiCli`, the glyph the mode toggle already shows for that mode. No text. | FR-009 |
| Width | The same fixed `TAB_WIDTH` as a terminal tab. | FR-010a |
| Leading slot | The stopped-mark slot, always reserved. | FR-012c, R4 |
| Trailing slot | Reserved and **empty** — no close control, by any press. | FR-004, FR-010a |
| Indicator | The same top-edge accent rule, on the same terms. | FR-005, FR-010 |
| Primary press | `mode = AiCli`. Starts/stops/restarts nothing. A press while already displayed is a no-op. | FR-006, FR-007 |
| Secondary press | The terminal tab's menu **minus Close**; nothing at all when that would be empty. | FR-006a, FR-006b |

The trailing slot is reserved rather than reclaimed for two reasons, and both are load-bearing: the
tab would otherwise be narrower than its neighbours (FR-010a), and the icon would otherwise sit off
the tab's midline, which the gate `tab_children_fit::a_tabs_content_sits_on_its_tabs_midline`
fails on — at the 4.6dp that feature 012's BUG-005 measured.

## The stopped mark

- Drawn in the **leading slot** of any tab whose process is stopped; the slot is reserved and empty
  otherwise. Never a pushed-or-not child (feature 023 FR-008a — a conditional child inside a
  pressable tab drops the press).
- Same mark, same place, on both kinds of tab (FR-012).
- Independent of the active indicator; a tab may carry both and they must not read as one
  (FR-012a).
- Shown for exactly the states that tab's menu can act on (FR-012d):

| Tab | Mark shown | No mark |
|---|---|---|
| AI | `Idle`, `Failed`, `InterruptedResumable` | `Starting`, `Restarting { .. }`, `Running` |
| Instance | `NotStarted`, `Exited` | `Starting`, `Running` |

- A `Starting` or `Restarting` process stays distinguishable from a running one by the existing
  in-progress presentation — the bar's status text (`starting…`, `restarting…`) and the pane's
  empty-state message — not by this mark (FR-012e).

**The two columns above are not to be written twice.** They are one predicate, generalised from
`attached_process_restartable`, which the menu also reads (research R2). A second match statement
anywhere in this feature is the defect 012 BUG-004 was.

## Overflow

- When the terminal tabs need more width than the bar can give, they **scroll horizontally at their
  own fixed width** (FR-002a). No tab is shrunk, ellipsised or dropped.
- The AI tab, the "+", the mode toggle, the session title and the status keep their full size and
  position regardless of instance count (FR-002c).
- The strip scrolls to the **mouse wheel** while the pointer is over it. No scroll-arrow controls
  (FR-002f).
- An edge with content beyond it carries a **fade**; when the content beyond it is the marked tab,
  that edge says so specifically (FR-002e).
- Changing the marked tab **scrolls it into view** (FR-002d). A user may then scroll away by hand.

## Interaction with the primary mode toggle

Unchanged and now doubled: the toggle keeps its position and its behaviour (FR-008, 012 FR-006).
Both it and the AI tab write the session's `mode`, and the marked tab is derived from `mode` — so
they cannot disagree, structurally, rather than by being kept in step. Pressing either from a
terminal moves the pane and the indicator together.

## What this contract does not cover

- **Keyboard access to the strip.** Out of scope, explicitly (spec "Out of scope"). The mode toggle
  remains the route that needs no pointer.
- **Renaming a tab**, and any menu item that would.
- **Closing the AI CLI process** by any route.
- **How lifecycle is tracked.** This contract presents state the application already holds.
