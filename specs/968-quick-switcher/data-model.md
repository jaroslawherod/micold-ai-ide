# Data Model: Quick-Switcher

**Feature**: 968 | [plan](./plan.md) | [research](./research.md)

Nothing is persisted (Principle IV). All state below lives on one window's `State` (FR-015) and is
gone when the switcher closes, except the entries, which are never stored at all.

## Core — `micold-core/src/quick_switch.rs` (new)

### `EntryKey` — what an entry stands for

```text
enum EntryKey { Location(SessionLocation), Session(SessionId), Project(PathBuf) }
```

Identity across list changes (FR-013): the highlight is stored as a key, never as an index.

### `Candidate` — one entry as the rules see it

| Field | Type | Rule |
|---|---|---|
| `key` | `EntryKey` | unique in a list |
| `name` | `String` | the displayed name, the only text matched |
| `parent` | `Option<usize>` | a session's location candidate index; `None` for locations and projects |
| `selectable` | `bool` | `false` only for an unavailable project (FR-008) |

Input order is FR-004's order; `parent` always points earlier in the list.

### `Rank` and `Ranked`

```text
enum Rank { Prefix, Run, Scattered }          // derived Ord is the rank order (FR-005)
struct Ranked { index: usize, rank: Rank, spans: Vec<Range<usize>> }
```

`spans` are byte ranges of `name` to emphasise; empty for a session kept only by its location.

### Functions

| Function | Rule | FR |
|---|---|---|
| `filter(&[Candidate], &Query) -> Vec<Ranked>` | empty query: every candidate in order, rank `Prefix`, no spans. Else keep in-order matches, sort stably by rank, then insert each matched location's unmatched sessions right after it with its rank | 004, 005 |
| `reseat(&[Candidate], &[Ranked], Option<&EntryKey>) -> Option<usize>` | the key's position when listed and selectable; else the first selectable; `None` when none | 005, 013 |
| `step(&[Candidate], &[Ranked], usize, Dir) -> usize` | next selectable above/below; stays at the first/last selectable | 007 |

`Dir` is `navigation::Dir` (Previous/Next), reused.

## Client — `micold-client/src/features/quick_switch.rs` (new, render-free)

### `Entry` — a candidate plus what the row shows (FR-006)

| Field | Type | Source |
|---|---|---|
| `candidate` | `Candidate` | below |
| `kind` | `Kind { Location, Session, Project }` | |
| `context` | `String` | session: its location's name; location: its tag labels (`naming::Tag`), e.g. `feat · PROJ-12 · missing`; project: `2 running · 1 unread`, plus `unavailable` |
| `current` | `bool` | location: `current_session_location()` or `sidebar.nav_stop`; session: `session.active` |

`candidates(&State) -> Vec<Entry>` (research R4):

1. For each `location` in `navigation::location_order(state)`: its `location_node` → a location
   entry (name `"Default"` or the node's `display_name`, tags), then each session of
   `unfolded_sessions()` → a session entry (`session.label.display()`, `parent` = the location's
   index).
2. For each `SwitcherEntry` of `state.switcher_entries()` with `!is_active`: a project entry
   (`label`, `running_count`, `unread_count`, `selectable = available`).

### `State` (field `quick_switch` on the root `State`)

| Field | Type | Meaning |
|---|---|---|
| `open` | `Option<Open>` | `None`: closed |

```text
struct Open {
  query: String,                      // kept across list changes (FR-013)
  highlighted: Option<EntryKey>,      // re-seated on every derivation (R7)
  terminal_had_focus: bool,           // read at open (R5, FR-010)
  keyboard_row_before: Option<RowRef>,// restored on close without choice (R8)
}
```

`Option<Open>` makes "a query or highlight with the switcher closed" unrepresentable (Principle V).

### `Msg`

| Message | From | Effect |
|---|---|---|
| `Toggled { terminal_had_focus }` | both chord paths (contracts/keys.md K3) | open (gate U1) or close without choice |
| `QueryChanged(String)` | the field | set query; highlight → first selectable of the new list |
| `Moved(Dir)` | the type-ahead's `on_move` | `step` |
| `Chosen(EntryKey)` | Enter (highlighted) or a click | resolve to a `Step` (U3); close when it resolves |
| `Dismissed` | Escape, scrim click, Tab (registry cancellation) | close without choice |

### State transitions

```text
Closed --Toggled [gate open]--> Open{query:"", highlighted: first selectable}
Open --QueryChanged/list change--> Open (reseat)
Open --Chosen [resolves]--> Closed + Step applied
Open --Chosen [gone / unavailable]--> Open (unchanged)
Open --Dismissed | Toggled--> Closed (keyboard_row restored)
```
