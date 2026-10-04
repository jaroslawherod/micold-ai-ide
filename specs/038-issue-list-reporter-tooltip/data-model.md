# Data Model: Reporter, Labels and a Description Tooltip in the Issue List

**Feature**: [spec.md](./spec.md) · **Plan**: [plan.md](./plan.md)

Nothing here is persisted. "NEW" marks what does not exist yet; everything else is feature 034's.

## 1. `Issue` (`micold_core::github`), extended

| Field | Type | Rule |
|---|---|---|
| `number`, `title`, `labels`, `updated_at` | as today | unchanged |
| `reporter` NEW | `String` | The author's login as GitHub reports it. `GHOST_LOGIN` (`"ghost"`) when the node's `author` is `null` or absent (FR-002). Never empty. |
| `description` NEW | `String` | `description_from(bodyText)`: whitespace folded, trimmed, capped (§2). Empty when the body has no text (FR-020), and on a listed issue until the description pass delivers its page (FR-024; contracts/issue-fields.md §6). |
| `row_text` | `String` | The **match text**, what `typeahead::rank` reads (§4). No longer the text a row displays. |

Construction: `Issue::new(number, title, labels, updated_at)` keeps its signature and yields
`reporter = "ghost"`, `description = ""`. Two chainable setters fill the new fields and rebuild
`row_text`: `.reported_by(login)` and `.described(body_text)` (the latter applies
`description_from`). `issue_from_node` uses both; existing call sites compile unchanged.

Derives: `Clone, PartialEq, Eq`. **No `Serialize`** (034, held by test). `Debug` is hand-written and
prints `reporter` and `description` as `<redacted>` (FR-025).

Accessors NEW: `reporter() -> &str`, `description() -> &str`, `title_line() -> String`,
`details_line() -> String`, `emphasis(&[Range<usize>]) -> RowEmphasis`.

## 2. Description (`description_from`) NEW

Input: the node's `bodyText` (GitHub's plain-text rendering). Output: the string held on `Issue`.

1. Every run of Unicode whitespace becomes one space; leading and trailing space is removed.
2. If the result has more than `DESCRIPTION_MAX_CHARS` (600) characters, it is cut at a character
   boundary to 600, trailing space is removed, and `…` is appended.
3. An input with no non-whitespace character gives `""`.

## 3. Display lines NEW

| Line | Text | Requirement |
|---|---|---|
| Title line | `#<number> <title>` | FR-001 |
| Details line | `<reporter>`, then `"  ·  "` + `labels.join(", ")` only when `labels` is not empty | FR-002, FR-003 |

`RowEmphasis { title: Vec<Range<usize>>, details: Vec<Range<usize>> }` — byte ranges of the title
line and the details line. Ranges are sorted, non-overlapping, and lie on character boundaries.

## 4. Match text and the emphasis mapping

| Milestone | `row_text` | Parts mapped by `emphasis` |
|---|---|---|
| M1 (US1) | `#N title` + (`"  ·  "` + labels) — as today | title part → title line; labels part → details line, after the reporter and its separator |
| M3 (US2) on | `#N title` + `"  ·  "` + reporter + (`"  ·  "` + labels) | title part → title line; reporter part → start of the details line; labels part → details line |

Rules of `emphasis(spans)`, where `spans` are byte ranges of `row_text` from `Match.spans`:

- A span inside one part is rebased to that part's offset in its display line.
- A span crossing a separator is split; the separator's bytes carry no emphasis.
- A span outside every part (cannot happen with `rank`, but the function is total) is dropped.
- The description is in no part and never in `row_text` (FR-014).

Loaded and searched issues are ranked by the same `rank` call over `row_text`, so FR-012's "one
rule" holds without a second filter.

## 5. `RestTimer` (`micold_core::tooltip`) NEW

```text
enum RestTimer { Away, Waiting { anchor: (f32, f32), since: Instant }, Open, Spent }
```

| Input | Meaning |
|---|---|
| `observe(cursor: Option<(f32, f32)>, now: Instant, delay: Duration) -> Rest` | `cursor` is the position when it is over the trigger, else `None` |
| `press()` | the trigger was pressed |
| `reset()` | the trigger now describes something else (a changed subject) |

`Rest { open: bool, wake_at: Option<Instant> }`: `wake_at` is `Some(since + delay)` only in
`Waiting`.

| State | Input | Next |
|---|---|---|
| `Away` | cursor over the trigger at `p` | `Waiting { anchor: p, since: now }` |
| `Waiting` | cursor farther than `REST_TOLERANCE` from `anchor` | `Waiting { anchor: p, since: now }` |
| `Waiting` | within tolerance, `now - since < delay` | unchanged |
| `Waiting` | within tolerance, `now - since >= delay` | `Open` |
| `Open` | cursor over the trigger, anywhere | `Open` |
| `Spent` | cursor over the trigger | `Spent` |
| any | cursor not over the trigger | `Away` |
| any | `press()` | `Spent` |
| any | `reset()` | `Away` |

`REST_TOLERANCE = 4.0` logical pixels, Euclidean distance from `anchor`; a distance of exactly 4.0
is within tolerance. The anchor does not drift: movement inside the tolerance keeps the first
anchor.

## 6. `clamp_to_lines` (`micold_core::tooltip`) NEW

`clamp_to_lines(text, max_lines, lines_of: impl Fn(&str) -> usize) -> Cow<str>`

- `lines_of(text) <= max_lines`: returns `text` borrowed, unchanged.
- Otherwise: the longest prefix `p` such that `lines_of(p + "…") <= max_lines`, cut at a character
  boundary, backed up to the last word boundary when one lies within the last 24 characters of the
  cut, trailing space removed, `…` appended.
- A text already ending in `…` never ends in `……`.
- `lines_of` is assumed monotonic in prefix length; the search is a binary search over character
  boundaries.

## 7. Picker row (`material::picker::Row`), extended

| Field | Type | Rule |
|---|---|---|
| `label`, `spans`, `enabled` | as today | unchanged |
| `details` NEW | `Option<(String, Vec<Range<usize>>)>` | Present → two-line wrapping mode. Absent → today's single-line row. |
| `tooltip` NEW | `Option<String>` | Present and not empty → the row is wrapped in a rest-delay tooltip. |
| `key` NEW | `Option<u64>` | The identity of what the row describes; passed to the tooltip as its subject. |

The issue picker builds `Row::new(title_line, emphasis.title).details(details_line,
emphasis.details)` and, from M5, `.key(number)` and `.tooltip(description)` when the description is not
empty.

## 8. State ownership (Principle II)

| State | Owner | Lifetime |
|---|---|---|
| Issues with reporter and description | `WorktreeForm`'s `IssueList` / `SearchState` | the open form |
| Highlight | the form reducer | the open form |
| `RestTimer`, the clamped text cache | the tooltip widget's state, in one window's widget tree | while the row is in the tree |

No state is global, shared between windows, or written to disk or logs.
