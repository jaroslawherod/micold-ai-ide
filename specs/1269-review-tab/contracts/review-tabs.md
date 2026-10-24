# Contract: the Review tab rules (`micold_core::review_tabs`, new)

Render-free, iced-free, generic over the kept view `V`. Each rule is a core unit test
(`mise run test-core`), written red first, with `V = u32` (or `String`) and keys built from
`PathBuf` and `micold_core::session::SessionLocation`. Types: [../data-model.md](../data-model.md)
§Core. Decisions: [../research.md](../research.md) R1–R4, R8, R9.

Notation: `P`, `Q` projects; `D` the Default entry, `W`, `X` worktrees; `(P, W)` a key; `s1`, `s2`
session ids.

## T1. Open or show (FR-002, FR-003, US2.1, US2.2, SC-005)

`open_or_show(key, current: Option<SessionId>, make: impl FnOnce() -> V) -> Opened`

| Before | Result | After |
|---|---|---|
| no tab for `key` | `Opened::Created` (`make` called once) | tab `{ view, Fresh, return_to: current }`, `shown == Some(key)` |
| tab hidden, `Fresh` | `Opened::Shown { reread: false }` (`make` not called) | `shown == Some(key)`, view unchanged; `return_to = current` |
| tab hidden, `Changed` or `Unwatched` | `Opened::Shown { reread: true }` | as above, freshness `Fresh` |
| tab is shown | `Opened::AlreadyShown` | nothing changes (not `return_to`, not freshness) |

Any number of `open_or_show(key, ..)` calls leave exactly one tab for `key` (`len()` counted). Opening
`(P, X)` while `(P, W)` is shown hides `(P, W)` (it keeps its tab and view) and shows `(P, X)`.

## T2. Hide (FR-004, FR-005, US1.2, US1.4)

`hide() -> bool`: `shown` becomes `None`; answers whether a tab was shown. Every tab, view, freshness
and `return_to` is unchanged. `hide` on nothing shown answers `false` and changes nothing.

## T3. Close with × (FR-007, US3.1, US3.2)

`close(key, live: &[SessionId], main: Option<SessionId>) -> Option<Closed<V>>`

- No tab for `key`: `None`, nothing changes.
- Otherwise the tab is removed and `Closed { view, was_shown, show }` answered: `was_shown` is
  `shown == Some(key)` before (then `shown` becomes `None`); `show` is the tab's `return_to` when it
  is in `live`, else `main`, else `None`.
- A later `open_or_show(key, ..)` answers `Created` (no kept view survives ×).
- Closing one tab never changes another tab.

## T4. Drop rules (FR-008, US3.3, US3.4, SC-002, edge case *Project removed*)

- `drop_gone_worktrees(project, names: &BTreeSet<String>) -> Vec<EntryKey>`: removes every
  `(project, Worktree(dir))` with `dir` not in `names`; never `(project, Default)`; never a key of
  another project. Clears `shown` and `last_session` entries of what it removes. Answers the removed
  keys.
- `drop_project(project) -> Vec<EntryKey>`: removes every key of `project`, the same way.
- **Nothing else removes a tab.** Property test: from a map with tabs for `(P, D)`, `(P, W)`,
  `(Q, X)`, any sequence of `open_or_show`, `hide`, `files_changed`, `location_left`,
  `comments_changed`, `remember`, `show_if_present` (≤ 200 steps, every key and session in the
  fixture) never lowers `len()`.

## T5. Freshness and the changed mark (FR-010, FR-011, US4.3–US4.6, SC-003)

| Call | Tab state before | Result | After |
|---|---|---|---|
| `files_changed(key)` | shown | `Change::Reread` | unchanged (the client reads it, as today) |
| `files_changed(key)` | hidden, any | `Change::Marked` | `Changed` |
| `files_changed(key)` | no tab | `Change::Nothing` | — |
| `location_left(key)` | hidden or shown, `Fresh` | — | `Unwatched` |
| `location_left(key)` | `Changed` | — | `Changed` (the mark stays) |
| `comments_changed(key)` | hidden, `Fresh` | — | `Unwatched` |
| `comments_changed(key)` | shown | — | unchanged |
| `open_or_show(key, ..)` / `show_if_present(key)` | hidden, `Changed` or `Unwatched` | `reread: true` | `Fresh` |

`location_left` on the shown key is how the client records "covered" (Overview, Settings, Compare,
R13): the review stays the shown tab, and the next `uncovered(key) -> bool` answers `true` exactly
once when it was `Unwatched` and sets `Fresh`. Showing a tab answers `reread: true` at most once per
change: a second show with no change between answers `false`.

## T6. Last-shown session (FR-003, FR-007, US2.5)

- `remember(key, session)`: the location's last-shown session in this window is `session`.
- `session_for(key, live: &[SessionId], main: Option<SessionId>) -> Option<SessionId>`: the
  remembered session if in `live`, else `main`, else `None`.

| remembered | live | main | answer |
|---|---|---|---|
| `s2` | `[s1, s2]` | `s1` | `s2` |
| `s2` (closed) | `[s1]` | `s1` | `s1` |
| none | `[s1]` | `s1` | `s1` |
| none | `[]` | none | `None` (sessionless: the Review tab alone is shown) |

## T7. Last session of a location closed (edge case, R8)

`show_if_present(key) -> Option<bool>`: when a tab for `key` exists it becomes the shown tab and the
answer is `Some(reread)` as in T1; with no tab, `None` and nothing changes.

## T8. Faces: the tab and the Changes button (FR-009, FR-010, FR-012, US2.3, US2.4, US4.1)

`ReviewTabFace::of(drafts: Option<usize>, freshness: Freshness, shown: bool)`:

| drafts | freshness | shown | label | badge | changed | tooltip |
|---|---|---|---|---|---|---|
| `None` (comments unknown) | `Fresh` | any | `Review` | none | no | `Review` |
| `Some(0)` | `Fresh` | any | `Review` | none (never `0`) | no | `Review` |
| `Some(1)` | `Fresh` | no | `Review` | `1` | no | `Review · 1 draft` |
| `Some(3)` | `Changed` | no | `Review` | `3` | yes | `Review · 3 drafts\nChanged since you looked` |
| `Some(3)` | `Changed` | yes | `Review` | `3` | no (a shown tab never carries the mark) | `Review · 3 drafts` |
| `Some(0)` | `Unwatched` | no | `Review` | none | no | `Review` |
| `Some(120)` | `Fresh` | no | `Review` | `99+` text, count kept | no | `Review · 120 drafts` |

`badge` is `Option<NonZeroUsize>`, so a zero badge is unrepresentable; `badge_text()` caps at `99+`.

`ChangesButton::of(has_tab: bool, shown: bool)`:

| has_tab | shown | press | selected | tooltip first line |
|---|---|---|---|---|
| no | — | `Press::Create` | no | `Open the review` |
| yes | no | `Press::Show` | no | `Show the review` |
| yes | yes | `Press::None` | yes | `The review is shown` |

The client sends the same message for `Create` and `Show` (R9); `Press::None` still sends it and the
reducer answers `AlreadyShown` (T1), so a press while shown changes nothing (clarification 1).
