# Data Model: Keyboard Navigation

**Feature**: 803 | [plan](./plan.md) | [research](./research.md)

All of it is per-window memory; nothing is persisted or sent (Principle IV).

## Core (`micold-core/src/navigation.rs`, new)

| Item | Shape | Rules |
|---|---|---|
| `Dir` | `enum { Next, Previous }` | — |
| `step` | `fn step<T: PartialEq + Clone>(order: &[T], current: Option<&T>, dir: Dir) -> Option<T>` | empty → `None`; `current` in `order` → neighbour with wrap; a one-entry list → that entry; `current` absent or `None` → first (Next) / last (Previous) |

## Client keys (`micold-client/src/keymap.rs`)

| Item | Shape |
|---|---|
| `NavLevel` | `enum { Session, Location, Project }` |
| `NavAction` | `struct { level: NavLevel, dir: micold_core::navigation::Dir }`, `Copy` |
| `PhysicalKey` | `enum { BracketLeft, BracketRight, Comma, Period, PageUp, PageDown, Other }` |
| `KeyInput.physical` | new field, `PhysicalKey` (`Other` when unknown) |
| `NAV_CHORDS` | the one table, [contracts/keys.md](./contracts/keys.md) K1 |
| `KeyOutput::Navigate(NavAction)` | tier-1 output, never forwarded |
| `nav_chord`, `nav_chord_labels`, `is_primary_shift` | functions, K1–K4 |

## Client state (`micold-client/src/features/`)

| Item | Where | Shape | Invariant |
|---|---|---|---|
| `RowRef` | `sidebar.rs` | `enum { Location(SessionLocation), Session(SessionId) }` | names a row by identity, not index, so it survives the list arriving late |
| `sidebar.nav_stop` | `sidebar::State` | `Option<SessionLocation>` | set only by a location step that landed on a location with no session; cleared by any current-session change, `project_entered`, and every clearing of `keyboard_row` |
| `sidebar.keyboard_row` | `sidebar::State` | `Option<RowRef>` | at most one row holds the sidebar's keyboard; set only by `Message::Navigate`; cleared when that row reports losing focus |
| `Step` | `navigation.rs` (new) | `enum { None, Session(SessionId), Stop(SessionLocation), Project(PathBuf) }` | what one press resolves to ([contracts/ui.md](./contracts/ui.md) U2) |
| `Message::Navigate(NavAction)` | `app.rs` | new top-level message | handled in `main.rs` (U3) |
| `SidebarMsg::KeyboardRowLeft` | `sidebar.rs` | new message | clears `keyboard_row` and `nav_stop` |

## Orders (derived at each press, never stored)

| Order | Source |
|---|---|
| Location | `navigation::location_order(&State) -> Vec<SessionLocation>` over `sidebar_entries()`: `Default`, `Worktree(n)`, and in an expanded `Group` each `RunRow::Worktree(n)` |
| Session | the current location's node `listed_sessions()` ids |
| Project | `switcher_entries()` paths where `available` |

## Transitions of the "current" triple

```text
press(Location) ── target has main ──▶ Selected(main) ─▶ active=main, nav_stop=None
               └─ target empty ─────▶ nav_stop=target (active unchanged)
press(Session)  ── nav_stop=None ───▶ Selected(next in listed_sessions)
press(Project)  ───────────────────▶ Reopened(path) ─▶ project_entered: nav_stop=None
```
