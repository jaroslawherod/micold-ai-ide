# Phase 1 Data Model: Branch Selector Type-Ahead Search

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

Three groups: the render-free matching types in `micold-core` (new), the form state they feed in
`micold-client` (extended), and the showcase's own state (extended). Nothing is persisted — see
"Lifetime and persistence" at the end.

---

## 1. Matching — `micold_core::typeahead` (new module)

The whole module knows about strings and nothing else (FR-019). It never sees a `BranchCandidate`.

### `Query`

The normalised search text.

| Field | Type | Notes |
|---|---|---|
| `text` | `String` | trimmed of leading/trailing whitespace, case-folded (FR-003) |

**Rules**

- An empty `text` matches everything, in input order (FR-002).
- `text.chars().count() < 3` disables the approximate tiers (FR-006a).
- Construction folds case once, so no per-candidate allocation happens inside the scan (R11).

### `MatchKind`

Which tier matched. Carried on the result rather than recomputed, because it decides the highlight
shape (FR-010) and the rank (FR-007).

| Variant | Meaning |
|---|---|
| `Literal` | the name contains the query verbatim, ignoring case |
| `SingleEdit` | some window of the name is within one insert/delete/substitute of the query |
| `Subsequence` | the query's characters occur in order, not necessarily adjacent |

Declaration order is both the order the tiers are **tried** and the ranking order:
`Literal < SingleEdit < Subsequence`. Single-edit precedes subsequence because a dropped-letter typo
is also a subsequence, and whichever tier is tried first claims it — see the tier-order note in
[contracts/match-ranking.md §2](./contracts/match-ranking.md#2-tiers).

### `Match`

What one candidate scored against one query.

| Field | Type | Notes |
|---|---|---|
| `kind` | `MatchKind` | the tier that hit |
| `at` | `usize` | byte offset of the match's start in the original name; the secondary rank key (FR-007) |
| `spans` | `Vec<Range<usize>>` | byte ranges of the name to emphasise, ascending and non-overlapping |

**Rules**

- `spans` is exactly one range for `Literal` and `SingleEdit`; one range per corresponding character
  for `Subsequence` (FR-010).
- Every range is a valid char boundary pair in the original name — the emphasis is applied to the
  untruncated name and rebased later (§2, R8).
- `spans` is empty if and only if the query is empty.

### `Key` and `Intent`

The keyboard rule, kept render-free for the same reason the matching rule is (FR-021). `Key` is this
crate's own platform-neutral enum — the rendering layer translates its input events into it, exactly
as `micold-client`'s `keymap.rs` already does for the terminal.

| `Key` | `Intent` |
|---|---|
| `Down`, `Up` | `Move(Direction)` — saturating at the ends, never wrapping |
| `Enter` | `Pick` — only when the highlighted row is enabled; otherwise `None` |
| `Escape` | `Dismiss` |
| `Other` | `None` — falls through to the field |

`intent_for(key, highlight: Option<usize>, rows_len, highlighted_enabled) -> Option<Intent>` is a pure
function of those four inputs. It is what FR-017 and FR-017a are tested against; the widget only
translates and applies.

### Free functions

| Function | Shape | Requirement |
|---|---|---|
| `match_one(name, &Query) -> Option<Match>` | one candidate, one query | FR-003, FR-004, FR-006, FR-006a, FR-008 |
| `rank<T>(items, key_fn, &Query) -> Vec<(usize, Match)>` | filters and orders, stable | FR-007 |
| `fit_around(name, spans, available, measure) -> (String, Vec<Range<usize>>)` | truncation that keeps the emphasis visible | FR-011d |
| `intent_for(key, highlight, rows_len, enabled) -> Option<Intent>` | the keyboard rule | FR-017, FR-017a |

`rank` returns *indices* into the caller's slice rather than cloning items — the caller owns the
candidates, and this is what keeps the module ignorant of what it is ranking.

`fit_around` takes `measure: impl Fn(&str) -> f32` so it is exercisable with a monospace stand-in and
then re-provable against real shaping, exactly as `ellipsized::fit` is today (R8).

---

## 2. Form state — `micold_client::app::WorktreeForm` (extended)

Existing fields are unchanged. `candidates` and `selected_branch` keep their current meaning
(FR-012, FR-013, FR-014).

| New field | Type | Notes |
|---|---|---|
| `branch_query` | `String` | raw, as typed — normalisation happens at match time |
| `branch_matches` | `Vec<(usize, Match)>` | recomputed on every keystroke; indices into `candidates` |
| `branch_list_open` | `bool` | whether the result list is showing |
| `branch_highlight` | `Option<usize>` | index **into `branch_matches`**, not into `candidates` (R15) |

**Invariants**

1. `branch_matches` always corresponds to the current `branch_query` and `candidates` — it is derived,
   never edited in place (FR-005).
2. `branch_query.trim().is_empty()` ⟹ `branch_matches` names every candidate, in `candidates` order
   (FR-002).
3. `branch_highlight` is `None` or a valid index into `branch_matches`. Any recompute that would leave
   it dangling resets it to the first match — a keystroke may never leave the highlight pointing at a
   row that is no longer there.
4. `selected_branch` is **never** written by a query change, a highlight move, or a dismissal — only by
   an explicit pick of an **available** candidate (FR-014, FR-012a). A blocked candidate can therefore
   never become the selection.
5. `branch_list_open` is false whenever `source != BranchSource::Existing`.
6. `branch_list_open` is set **only** by focus, a query change, a pick, a dismissal, or a source
   change — never as a side effect of rendering, and never inferred from whether `branch_matches` is
   empty. An open list with no matches is a real state: it is what shows the no-match message
   (FR-015).

**State transitions**

| Message | Effect |
|---|---|
| `AddWorktreeBranchFocused` | opens the list (FR-001b). Leaves the query, the matches and the selection untouched — focusing shows what is on offer, it does not filter |
| `AddWorktreeBranchQueryChanged(String)` | sets `branch_query`, recomputes `branch_matches`, re-seats `branch_highlight` per invariant 3, opens the list |
| `AddWorktreeBranchHighlightMoved(Up \| Down)` | moves within `branch_matches`, saturating at both ends |
| `AddWorktreeBranchSelected(BranchCandidate)` | if the candidate is available: sets `selected_branch`, closes the list, leaves `branch_query` alone. If it is blocked: **no effect at all** — the list stays open and the selection is untouched (FR-012a) |
| `AddWorktreeBranchDismissed` | closes the list; changes nothing else. Emitted by Escape, by a click outside, and by the field losing focus — three triggers, one effect (FR-001b) |
| `AddWorktreeBranchesListed(Vec<BranchCandidate>)` | existing message; now also recomputes `branch_matches` against the current query |
| `AddWorktreeSourceChanged(_)` | existing message; now also clears `branch_query`, `branch_matches`, `branch_highlight` and closes the list |

`can_submit()` and `preview()` are untouched: they read `selected_branch`, which only
`AddWorktreeBranchSelected` writes, and only for an available candidate (FR-013).

`branch_query` is likewise never written by anything except `AddWorktreeBranchQueryChanged` — which is
what makes FR-014a ("the field holds only the search text") a structural property rather than a
convention.

---

## 3. Row view model — `micold_client::ui::material::typeahead` (new)

What the component is handed for one row. Deliberately not a `BranchCandidate` (FR-019).

| Field | Type | Notes |
|---|---|---|
| `label` | `String` | the full text of the row, including whatever explains an unavailability |
| `spans` | `Vec<Range<usize>>` | byte ranges of `label` to emphasise |
| `enabled` | `bool` | false for a branch held elsewhere — rendered, still listed, **not pickable** (FR-012, FR-012a) |

The current selection is passed to the component as an index alongside the rows rather than as a
per-row flag (FR-014b), so "which row is selected" cannot disagree with itself across rows.

The branch picker builds one of these per entry in `branch_matches`, taking `label` from
`BranchCandidate`'s existing `Display` (so the origin and in-use suffixes survive verbatim) and
`spans` from the `Match`. The mapping is the only place branch vocabulary and component vocabulary
meet.

---

## 4. Showcase state — `micold_client::showcase::state` (extended)

| New field on `Showcase` | Type | Notes |
|---|---|---|
| `typeahead_query` | `String` | the gallery example's own search text |
| `typeahead_highlight` | `Option<usize>` | same rule as the form's |
| `typeahead_open` | `bool` | whether its list is showing. **False at rest** — the entry starts closed, as the picker does. *(Added by BUG-001; FR-020a.)* |

| New message | Payload |
|---|---|
| `TypeaheadQueryChanged` | `String` |
| `TypeaheadHighlightMoved` | direction |
| `TypeaheadPicked` | `usize` |
| `TypeaheadFocused` | — |
| `TypeaheadDismissed` | — |

The open rule is `WorktreeForm.branch_list_open`'s, applied to the same messages: reaching the field
opens, a query change opens, a pick closes, a dismissal closes. Stated as "the same rule" rather than
restated, because the gallery having its **own** open rule is precisely the defect BUG-001 records.

`Message` loses its `Copy` derive as a direct consequence (R16); `Clone`, `Debug`, `PartialEq` and
`Eq` are kept. Sample rows come from `showcase::samples` as fixed data, so the gallery stays
deterministic (feature 020, FR-022).

---

## Lifetime and persistence

Nothing here is persisted or sent anywhere. `WorktreeForm` exists only while the add-worktree modal is
open and is discarded when it closes, so the search text dies with the form — which is the spec's "no
persistence" assumption, held structurally rather than by a policy someone has to remember. The
matching module holds no state at all between calls, and the showcase's copy is process-local by
construction (feature 020, FR-020). Principle IV is unaffected: no new file, no new directory, no
network.
