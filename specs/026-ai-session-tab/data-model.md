# Phase 1 data model: The AI Session as a Tab

The feature adds **no persisted state and no protocol field**. It adds one view-level type, widens
one existing piece of view state, and derives everything else from the session record the client
already holds. What follows is that shape.

## Entities

### `StripTab` *(new, view-level)*

Which member of the strip something refers to.

| Variant | Payload | Means |
|---|---|---|
| `Instance` | `ShellInstanceId` | one open Regular Terminal instance |
| `Ai` | — | the session's single AI CLI process |

**Why a type and not an `Option<ShellInstanceId>`.** Principle V asks that invalid states be
unrepresentable, and this is where FR-005's "never zero, never two" is either structural or a rule
somebody has to keep. `None` already means something else in this file — "the session has no active
instance" — so overloading it to also mean "the AI tab" gives one value two meanings and makes
"which tab is marked" unanswerable in the case that matters. A closed two-variant enum makes the
marked tab a total function of `(TerminalMode, Option<ShellInstanceId>)`, so exactly one tab is
marked because there is nowhere else for the answer to go.

**Ordering.** The strip renders instances in the session's own list order, then the AI tab last
(FR-002). The AI tab is not in the scrolling region (FR-002b), so "last" is a placement, not an
index.

### `Session` *(existing, unchanged)*

Read, not modified. The three fields this feature reads:

- `mode: TerminalMode` — `AiCli` or `Regular`. With `active_shell`, it determines the marked tab.
- `active_shell: Option<ShellInstanceId>` — which instance the pane shows in `Regular` mode.
- `lifecycle: SessionLifecycle` and each `shells[i].lifecycle: ShellLifecycle` — the two lifecycles
  the stopped mark reads, through R2's single predicate rather than directly.

### Stopped mark *(new, presentation)*

A reserved slot on every tab, drawn as a dot when that tab's process is stopped and as an empty
space of the same size otherwise (FR-012c, R4). Built from `material::ActivityBadge` extended with
an emphasis-taking constructor (R3). Independent of the active indicator, so a tab can carry both
(FR-012a).

## Derived values

None of these is stored. All are pure functions of the session record, which is what keeps them
testable without a renderer (Principle I).

| Value | From | Requirement |
|---|---|---|
| `marked_tab(session) -> StripTab` | `mode`, `active_shell` | FR-005 |
| `process_stopped(session, tab) -> bool` | `lifecycle` / `shells[].lifecycle`, via R2's predicate | FR-012, FR-012d |
| `tab_menu_items(session, tab) -> Vec<MenuItem>` | the same predicate, plus `tab` for Close | FR-006a, FR-004 |
| `menu_opens(session, tab) -> bool` | `!tab_menu_items(..).is_empty()` | FR-006b |
| `overflowing(viewport, content) -> Option<Edge>` | scroll offset and content width | FR-002e |

The second and third share one predicate deliberately (R2): FR-012d requires the mark and the menu
to agree, and deriving both from one function is what makes that structural.

## State transitions

The feature introduces no transition of its own. It **displays** two existing lifecycles and
**selects** between existing panes.

```text
AI process    Idle ──▶ Starting ──▶ Running ──▶ Restarting{n} ──▶ Failed
                                          └──▶ InterruptedResumable
              mark:   ●     ·           ·          ·                ●          ●
                   (stopped)(none)   (none)     (none)          (stopped)  (stopped)

Instance      NotStarted ──▶ Starting ──▶ Running ──▶ Exited
              mark:  ●            ·          ·          ●
```

`●` = stopped mark shown, `·` = slot reserved and empty. The two rows are the same rule applied to
two vocabularies (R1): the mark is shown for exactly the states a restart can act on.

**Selection**, unchanged in substance and now reachable two ways:

```text
                   press the AI tab  ─┐
                                      ├──▶ mode = AiCli      (FR-006, FR-008)
                   press the toggle  ─┘
   press a terminal tab ─────────────────▶ mode = Regular, active_shell = that instance
```

FR-008's "the toggle and the AI tab must not be able to disagree" is structural for the same reason
as above: both write `mode`, and the marked tab is derived from it. There is no second selection to
keep in step.

## Widened view state

`State::shell_instance_menu: Option<(ShellInstanceId, u16, u16)>` becomes
`Option<(StripTab, u16, u16)>` (R8) — the tab the menu was opened on, and the press point. One
surface, one registration, one `POPOVERS` entry; the AI tab's menu is the same menu with Close
filtered out (FR-004, FR-006a).

## Invariants

1. **Exactly one marked tab** (FR-005). Structural: `marked_tab` is total and returns one
   `StripTab`.
2. **The mark and the menu agree** (FR-012d). Structural: one predicate (R2).
3. **Every tab has the same children** (R4, feature 023 FR-008a). Both slots are always present;
   only what is drawn in them changes.
4. **Every tab is the same fixed width** (FR-010a, feature 012 FR-004c), and no child is laid out
   under the minimum interactive target (012 SC-010). Held by
   `tests/gates/tab_children_fit.rs`, which will run against the AI tab once it is in a covered
   state (R9).
5. **No cross-session effect** (FR-011). Structural: every value above takes one session.
