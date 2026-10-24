# Contract: Jump-to review entries (#1274)

Extends the quick-switcher of `specs/968-quick-switcher` (opened by the #1127 title bar field or
Ctrl/Cmd+Shift+P). Core rules J1–J3 are M1; J4–J5 are M3 (after #1127).

## J1 — Keys and texts (core `review::jump::review_candidates`)

| Entry | `EntryKey` | `name` (matched first) | `detail` (matched only if name misses) | Row shows |
|---|---|---|---|---|
| File | `ReviewFile(path)` | file name (`tests.rs`) | full path (`daemon/tests.rs`) | name, directory, reviewed state |
| Comment | `ReviewComment(id)` | its text, lines joined by spaces | its file path | status, `path:line`, first line of text |

Files in file order, then comments in comment order (`navigate::comment_order`), of the shown source
only.

## J2 — Ranking (core `quick_switch::filter`)

Sort key `(via, rank, index)` with `Via::Name < Via::Detail` and 968's `Rank` (prefix, run,
scattered). A candidate with `detail: None` behaves exactly as before; the existing 968 tests stay
green unchanged (FR-019).

## J3 — Spans

Spans index `name` for `Via::Name` and `detail` for `Via::Detail`, so the row emphasises the text
that matched.

## J4 — Position (client `features/quick_switch.rs::candidates`)

Active project's locations and their sessions, then the current entry's review entries (J1), then
the other projects. No Review tab for the current entry → no review entries (FR-019).

## J5 — Choosing

| Entry | Effect |
|---|---|
| `ReviewFile(path)` | show the entry's Review tab; select `path` through `select_file` (row in view) |
| `ReviewComment(id)` | show the Review tab; apply `step_comment`'s effect for that id (K7) |

Neither changes the current location, session or project, nor starts or stops a session (FR-018);
a key no longer listed when chosen does nothing (968 U3 rule).
