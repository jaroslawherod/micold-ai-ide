# Data Model: Back/Forward Navigation

**Feature**: 1465 | [plan](./plan.md) | [research](./research.md)

Nothing is persisted (FR-001, Principle IV). All state lives on the one window's root `State` and
is gone at restart.

## Core — `micold-core/src/history.rs` (new)

### `Dir`

```text
enum Dir { Back, Forward }
```

### `View` and `Place` — where the main area is (spec *Key Entities*)

```text
enum View {
    Session { location: SessionLocation, id: SessionId },
    Review(SessionLocation),
    Overview,
    Compare(GroupId),
}
struct Place { project: PathBuf, view: View }     // derive Clone, PartialEq, Eq, Debug
```

Equality is structural: two places are equal when the project and the view (with its location,
session or group) are. The Overview and Compare carry no location: they are project-wide views
(research R3). A Settings panel, a navigation stop with nothing shown and the welcome screen are
not places, so they cannot be represented.

### `History<P>` — the list and its position

| Field | Type | Invariant |
|---|---|---|
| `entries` | `Vec<P>` | `len() <= CAP` (50, FR-005, SC-004); no two neighbouring entries equal |
| `pos` | `usize` | `pos < entries.len()` when non-empty; `0` when empty |

`entries` and `pos` are private; `History::default()` is empty. Readers: `current() -> Option<&P>`,
`entries() -> &[P]`, `position() -> usize` (tests and the client's gate).

### Operations

| Operation | Rule | FR |
|---|---|---|
| `record(p)` | `p == current()` → nothing. Else drop every entry after `pos`, push `p`, `pos` = last; over `CAP` drop the oldest (front) and keep `pos` on `p`. | 002, 003, 004, 005 |
| `step(dir, shown: Option<&P>, exists: impl FnMut(&P) -> bool) -> Option<P>` | Walk from `pos` towards `dir` one entry at a time. An entry `exists` rejects is **removed** (and `pos` adjusted when it was before `pos`); an entry equal to `shown` is passed over and kept. The first other entry becomes `pos` and is returned. None found → `pos` unchanged (gone entries passed over are still removed) and `None`. After any removal, neighbouring equal entries are merged into one, keeping `pos` on the merged entry if it was on either. | 006, 007, US4 |
| (none) | Moving never records: `step` does not call `record`. | 007 |

`step` never returns an entry equal to `shown`, so a press always shows somewhere different or
does nothing (US4 AS4).

## Client — `micold-client/src/features/history.rs` (new)

### `State` — on the root as `history`

| Field | Type | Rule |
|---|---|---|
| `list` | `micold_core::history::History<Place>` | the window's one history (FR-001) |
| `moving` | `bool` | set by a press that resolved to a move, cleared by `finish_history_move`; observation is off while set (research R2) |

### `Msg` — `Message::History(Msg)` at the root

```text
enum Source { Button, Key }
enum Msg { Pressed { dir: Dir, source: Source } }
```

### `Step` — what one press resolves to

```text
enum Step {
    None,            // blocked, a field holds the key, nothing to go to
    ReviewBack,      // the shown Review tab's own Back (FR-011)
    Show(Place),     // the target; `list.pos` already moved to it, `moving` set
}
```

### Functions

| Function | Rule | FR |
|---|---|---|
| `resolve(app: &mut crate::app::State, msg, lookup: &dyn Fn(&Path) -> Option<Vec<PathBuf>>) -> Step` | `navigation::blocked(app)` → `None`. `Source::Key` and (`app.terminal_focused()` or `app.live_focused_field().is_some()`) → `None`. `Back`, shown place a Review tab, `changes::can_go_back` on its view → `ReviewBack`. Else `list.step(dir, app.shown_place().as_ref(), \|p\| exists(app, p, lookup))` → `Show(p)` with `moving = true`, or `None`. | 006, 008–012 |
| `exists(app, place, lookup) -> bool` | research R5. `lookup(project)` gives the worktree directories of a non-active project (`None` = unreadable → its worktree locations count as gone). | 006, edge *Gone targets* |
| `observe(state, shown: Option<Place>)` | `moving` → nothing. `Some(p)` → `list.record(p)`. `None` → nothing. | 002, 003 |
| `finish(state, shown)` | `moving = false`, then `observe(state, shown)`. | 007, research R2 |

## Root — `micold-client/src/app.rs`

| Item | Rule |
|---|---|
| `State::shown_place() -> Option<Place>` | research R3's order. |
| `State::press_history(msg, lookup) -> Step` | `history::resolve(self, msg, lookup)`: the root drives the feature, as `tests/feature_registration_cost.rs::only_the_root_drives_a_feature` requires. |
| `State::observe_place()` | `history::observe(&mut self.history, self.shown_place())`; called by the binary after every message. |
| `State::finish_history_move()` | `history::finish(&mut self.history, self.shown_place())`. |
| `State::apply_history(step) -> Vec<Message>` | research R6: `ReviewBack` → `[Changes(BrowseBack)]`; `Show(place)` → `[Project(Reopened(project))]` when `project` is not active, then the covers' closes and the view's message; `None` → `[]`. |

## Root — the live focused field

`State::live_focused_field(&self) -> Option<FieldId>` (in `app.rs`, which may read every feature): `focused_field`, except a
`ReviewComposer` whose comment box is closed and a `Composer(_)` / `QueuedMessage(_)` while the
Review tab is shown (research R8). `changes::keys::review_holder` is refactored onto it.

## Client — `micold-client/src/keymap.rs`

| Item | Rule |
|---|---|
| `history_key(&KeyInput) -> Option<Dir>` | [contracts/keys.md](./contracts/keys.md) H1 |
| `history_key_labels(Dir) -> (String, String)` | `(Linux/Windows, macOS)`: `("Alt+←", "Cmd+[")`, `("Alt+→", "Cmd+]")` |
| `history_key_label(Dir) -> String` | the running platform's form, for the sheet |

## State transitions (one press)

```text
Pressed ──blocked / field / terminal (Key)──▶ None
   │
   ├─ Back ∧ Review shown ∧ can_go_back ──▶ ReviewBack ──▶ BrowseBack (place unchanged)
   │
   └─ step finds p ──▶ Show(p), moving=true ──▶ follow-ups via root update (observe off)
                                              └─▶ finish_history_move: moving=false, observe
```
