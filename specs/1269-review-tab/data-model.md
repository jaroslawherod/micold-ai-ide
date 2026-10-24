# Data model: Review as a per-worktree tab that keeps its state (#1269)

Everything here lives in window memory only (spec *Assumptions*, D2): nothing is persisted, nothing
is sent to the session service or off the device. Rules: [contracts/review-tabs.md](contracts/review-tabs.md)
(T1–T8), [contracts/review-tab-ui.md](contracts/review-tab-ui.md) (U1–U16).

## Core (`crates/micold-core/src/review_tabs.rs`, new)

### `EntryKey`

`type EntryKey = (PathBuf, SessionLocation)` — a project and one of its locations (Default or a
worktree's `dir_name`). Same pair as `features::changes::State::remembered`.

### `Freshness`

| Variant | Meaning | Mark drawn |
|---|---|---|
| `Fresh` | last read current as far as the window knows | no |
| `Changed` | files changed while the location was shown and the tab hidden | yes, while hidden |
| `Unwatched` | the tab was not watched for a while (location left, covered, read in flight, comments changed) | no |

Transitions: T5. Showing a `Changed` or `Unwatched` tab answers `reread: true` once and makes it
`Fresh`.

### `ReviewTab<V>`

| Field | Type | Notes |
|---|---|---|
| `view` | `V` | the kept view; the client's `OpenView` |
| `freshness` | `Freshness` | T5 |
| `return_to` | `Option<SessionId>` | the current session when the tab was last shown (T1); × shows it if still live (T3) |

### `ReviewTabs<V>`

| Field | Type | Invariant |
|---|---|---|
| `tabs` | `BTreeMap<EntryKey, ReviewTab<V>>` | at most one tab per key (FR-002) |
| `shown` | `Option<EntryKey>` | names a key of `tabs`, or `None` |
| `last_session` | `BTreeMap<EntryKey, SessionId>` | entries of removed keys are removed with them |

Fields are private; the API is T1–T7: `open_or_show`, `hide`, `close`, `drop_gone_worktrees`,
`drop_project`, `files_changed`, `location_left`, `comments_changed`, `uncovered`, `remember`,
`session_for`, `show_if_present`, plus readers `shown_key()`, `shown()`, `shown_mut()`, `get(key)`,
`has(key)`, `len()`, `iter()`. `Default` is empty.

### Answers

- `Opened`: `Created` | `Shown { reread: bool }` | `AlreadyShown`.
- `Closed<V>`: `{ view: V, was_shown: bool, show: Option<SessionId> }`.
- `Change`: `Reread` | `Marked` | `Nothing`.

### `ReviewTabFace`

`{ label: &'static str ("Review"), badge: Option<NonZeroUsize>, changed: bool, tooltip: String }`,
from `ReviewTabFace::of(drafts: Option<usize>, freshness, shown)`; `badge_text()` gives `1`…`99`,
`99+`. T8.

### `ChangesButton`

`{ press: Press (Create | Show | None), selected: bool, tooltip: &'static str }`, from
`ChangesButton::of(has_tab, shown)`. T8.

## Client (`crates/micold-client`)

### `features::changes::State` (changed)

| Field | Before | After |
|---|---|---|
| `open` | `Option<OpenView>` | removed |
| `tabs` | — | `ReviewTabs<OpenView>` |
| everything else (`next_seq`, `pending`, `layout`, `reviews`, `pr_enabled`, `remembered`, `header`) | | unchanged |

`OpenView` is unchanged: it already holds every kept-view part the spec lists (selected file,
`list_offset`, `diff_offset`, `expansions`, whole-file through `expansions`, `toggles`, `files.shown`,
`source`, `browse`/`stack`, `composer`, `pick`, `panel`).

### `features::changes::Msg` (changed)

| Message | Before | After |
|---|---|---|
| `Opened { project, entry }` | open a new view, resetting it | `open_or_show` (T1); a fresh view only when `Created` |
| `Closed` | drop the view | removed; replaced by `TabClosed { project, entry }` (×, T3) |
| `SessionSelected` | drop the view | `hide` (T2) |
| `WorktreesListed(names)` | drop a gone worktree's view | `drop_gone_worktrees` for the active project, whose worktree list it is (T4); the header's rule unchanged |
| `Changed` | re-read open view and header | also marks a hidden tab of the header's location (T5, U6) |
| `ReviewChanged { .. }` | update `reviews` | also `comments_changed` for a hidden tab of that entry |

### `app::State` (new readers, render-free)

- `review_shown(&self) -> Option<&EntryKey>`: `tabs.shown_key()` when its project is the active one.
- `shown_location(&self) -> Option<SessionLocation>`: the shown review's location, else
  `current_session_location()` (R5).
- `review_watch_entry(&self) -> Option<&SessionLocation>`: the shown review's entry while not covered
  (R13).
- `review_covered(&self) -> bool`: Overview open, a Settings draft, or Compare open.

### `features::session_strip` (changed, M3)

- `Member::Review { project: PathBuf, entry: SessionLocation, face: ReviewTabFace, shown: bool }`.
- `MemberKey::Review` — a unit variant: the strip shows one location, so it holds at most one
  Review member, and `MemberKey` stays `Copy + Hash`.
- `StripModel::of` reads `shown_location()`; `StripModel.location` stays.

### `ui/material` (new component, M3)

- `CountBadge` (`ui/material/count_badge.rs`): `new(count: NonZeroUsize, roles)`, `.tooltip(..)`,
  `From<CountBadge> for Element`.
- `tab::REVIEW_WIDTH`: fixed width of the Review member (label, `99+` badge, dot, ×).
