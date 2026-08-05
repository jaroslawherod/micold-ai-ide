# Contract: Matching, Ranking and Truncation

**Module**: `micold_core::typeahead` | **Feature**: [../spec.md](../spec.md) | **Consumers**: the
branch picker's reducer, the component showcase, any future picker.

This is the render-free half of the feature. Everything below is decidable from strings alone and is
testable without a renderer (FR-021). No item in this contract mentions branches, worktrees or git
(FR-019).

---

## §1 Normalisation

```
Query::new(text) -> Query
```

1. Leading and trailing whitespace is removed from `text`.
2. The remainder is case-folded once, at construction.
3. Interior whitespace is **kept** and matched literally — a branch name may contain none, so a query
   with an interior space simply finds nothing rather than being silently split into terms.
4. `/`, `-`, `_`, `.` and every other character are ordinary characters. Nothing is a metacharacter and
   nothing needs escaping (spec, Edge Cases).

**Q1.1** `Query::new("  Feat/Login  ")` matches a candidate named `feat/login`.
**Q1.2** `Query::new("   ")` is empty and matches everything.

---

## §2 Tiers

`match_one(name, query) -> Option<Match>` tries the tiers in order and returns the **first** that
hits. `name` is compared case-folded; the returned offsets index the original `name`.

### §2.1 Literal — always active

Hits when the folded `name` contains the folded `query`. `at` is the byte offset of the leftmost
occurrence. `spans` is the single range covering it.

**Q2.1.1** query `log`, name `feat/login` → `Literal`, `at = 5`, spans `[5..8]`.
**Q2.1.2** query `feat/l`, name `Feat/Login` → `Literal`, `at = 0`, spans `[0..6]`.

> **Tier order, corrected during implementation.** These were originally ordered
> literal → subsequence → single-edit. A test caught the consequence: a dropped-letter typo is
> *also* a subsequence — `reportng` sits inside `reporting` in order — so the subsequence tier
> claimed every deletion typo and emphasised it as scattered characters, leaving FR-010's
> whole-word rule reachable only by substitutions. `feat/r-e-p-o-r-t-i-n-g` with a gap at the `i`
> is exactly the broken word the clarification session rejected. The single-edit tier is therefore
> tried **first**, and ranks above subsequence: it is the closer reading of what was typed.

### §2.2 Single edit — active from 3 query characters, anchored below 5

Hits when some window of `name` of length `q-1`, `q` or `q+1` (where `q` is the query's character
length) is within Levenshtein distance 1 of the query. `at` is the window's start; `spans` is the
**whole window** as one range (FR-010). Windows of the same length as the query are preferred, so a
substitution beats an insertion that also happens to work.

**Q2.2.1** query `reportng`, name `feat/reporting` → `SingleEdit`, spans one range covering
`reporting`.
**Q2.2.2** query `repot`, name `feat/reporting` → `SingleEdit` over the window `repor`.
**Q2.2.3** query `xyz`, name `feat/reporting` → `None` (FR-008).
Below **5** characters the tier is **anchored**: only same-length windows are considered, and the
window's first and last characters must equal the query's. Above it, unrestricted.

**Q2.2.4** query `frep`, name `feat/reporting` → **not** `SingleEdit`: the same-length window
`/rep` disagrees with the query at its first character, so it is not read as a typo. It falls to
§2.3.
**Q2.2.5** query `lagi`, name `feat/login` → `SingleEdit` over `logi`: four characters, but the ends
agree, so one wrong character between them is a slip (SC-004).

> **Anchored below five, arrived at in two steps.** This tier was originally gated at the same three
> characters as §2.3, and the first abbreviation test caught what that costs. One edit reaches a
> window of length `q-1`, so at `q = 3` a **two-character** window answered a three-character search
> — `llo` matched `lo` — and at `q = 4` the punctuation-for-letter substitution `/rep` claimed
> `frep` before the subsequence tier could read it as `f` … `rep`. Since §2.4 makes the first hit
> final, that put every short abbreviation into the wrong tier and the wrong emphasis.
>
> The first fix was a flat floor of five characters, and `/speckit-converge` caught what *that*
> cost: SC-004 promises that one wrong character still finds the branch and says nothing about
> length, but `lagi` — one substitution from `logi` — then found nothing at all.
>
> Both original failures disagree with the query at an **end**, which is the narrower cause. The
> first character of a search is the one the developer is surest of, and the last is what they have
> just typed; a wrong character *between* them is a slip, while a different first letter is a
> different word. So the tier now runs from three characters, anchored below five — which keeps
> `frep` and `llo` out and lets `lagi` in.

### §2.3 Subsequence — active from 3 query characters

Hits when every character of the query appears in `name` in order, matched **greedily leftmost**.
`at` is the offset of the first matched character. `spans` is one range per matched character.

Greedy-leftmost is normative, not an implementation note: it is what makes the highlight the same on
every run for the same inputs, which SC-005 depends on.

Ranges that end up adjacent are merged, so an abbreviation whose characters happen to align
consecutively is emphasised as one run rather than several abutting ones.

**Q2.3.1** query `frep`, name `feat/reporting` → `Subsequence`, spans covering `f` at 0..1 and `rep`
at 5..8 — two ranges, the second merged from three adjacent characters.
**Q2.3.2** query `fl`, name `feat/login` → **no approximate attempt at all**: two characters is
below the floor, and no literal `fl` exists, so the result is `None` (FR-006a).

### §2.4 Tier exclusivity

A name that matches literally is **never** reported as `Subsequence` or `SingleEdit`, even though it
also satisfies both. First hit wins, and the tier decides the highlight.

**Q2.4.1** query `log`, name `feat/log` → `Literal`, not `Subsequence`.

---

## §3 Ranking

```
rank(items, key_fn, query) -> Vec<(usize, Match)>
```

Returns the indices of matching items paired with their matches, ordered by:

1. `kind` ascending — `Literal`, then `SingleEdit`, then `Subsequence` (FR-007; see the tier-order
   note in §2);
2. then `at` ascending — an earlier match position ranks higher;
3. then the caller's original order, preserved by a **stable** sort (FR-007).

Non-matching items are absent. An empty query returns every item, in input order, each with an empty
`spans` (FR-002).

**Q3.1** items `["feat/log", "feat/dialog-cleanup"]`, query `log` → `feat/log` first (both `Literal`;
`at` 5 versus 10).
**Q3.2** items `["feat/reporting", "chore/log"]`, query `log` → `chore/log` first: a `Literal` always
precedes any approximate match.
**Q3.3** Two candidates with the same `kind` and the same `at` come back in the order they were given,
on every run.
**Q3.4** `rank(items, key, Query::new(""))` returns `0..items.len()` in order.

---

## §4 Truncation

```
fit_around(name, spans, available, measure) -> (String, Vec<Range<usize>>)
```

Returns the longest window of `name` that fits within `available`, chosen so that the emphasised
ranges remain inside it, together with those ranges rebased onto the returned string.

1. If the whole `name` fits, it is returned unchanged with `spans` unchanged.
2. Otherwise the window is grown around the emphasised region until adding one more character on
   either side would exceed `available`.
3. A cut at the start is marked with a leading `…`; a cut at the end with a trailing `…`; both cuts
   get both (FR-011d).
4. Cuts land only on character boundaries.
5. The ellipsis is included in the measurement, so the result never exceeds `available`.
6. If the emphasised region alone does not fit, the window is the region's own leading portion — a
   row is never returned with no emphasis visible.
7. `measure` is supplied by the caller and is the only thing that knows about fonts.

**Q4.1** With a monospace stand-in, a name whose match sits at the end comes back with a **leading**
ellipsis and the match intact.
**Q4.2** A name whose match sits at the start comes back with a **trailing** ellipsis and the match
intact.
**Q4.3** A name that fits is returned byte-identical, with `spans` untouched.
**Q4.4** Every returned range is within the returned string's bounds and lands on char boundaries.
**Q4.5** Multi-byte names are never cut mid-character.

---

## §4a Ranking quality benchmark

**SC-003**: a pinned corpus of realistic branch names and a fixed set of `query → intended branch`
pairs live beside the tests. At least **95%** of the pairs must place the intended branch in the top
five results of `rank`.

1. The corpus and the pairs are committed data, not generated — the figure must be reproducible on
   every run and comparable across changes.
2. Pairs cover all three tiers: literal fragments, abbreviations, and single typos.
3. The assertion is on the **rate**, so an individual pair may legitimately fail; a change that drops
   the rate below 95% fails the build and names the pairs that regressed.

This is the only check on ranking *quality* as opposed to ranking *rules* — §3 pins the ordering
rules, and this pins whether those rules actually surface the branch a developer meant.

---

## §4b The keyboard rule

```
intent_for(key, highlight, rows_len, highlighted_enabled) -> Option<Intent>
```

Render-free, for the same reason everything else here is: FR-017 and FR-017a are decision logic, and
Principle I's GUI exception covers only glue that has none. The widget translates its input events
into `Key` and applies the returned `Intent`; it decides nothing itself. `micold-client`'s
`keymap.rs` is the precedent — same split, same reason.

| Key | Intent | Notes |
|---|---|---|
| `Down` | `Move(Next)` | saturates at the last row; never wraps |
| `Up` | `Move(Prev)` | saturates at the first row; never wraps |
| `Enter` | `Pick` | **only** when `highlighted_enabled`; otherwise `None` (FR-012a) |
| `Escape` | `Dismiss` | |
| anything else | `None` | falls through to the field, so typing never leaves it |

**Q4b.1** `Down` at the last row returns `Move(Next)` that resolves to the last row again, not to the
first.
**Q4b.2** `Enter` with `highlighted_enabled = false` returns `None` — no pick, no dismissal.
**Q4b.3** `Enter` with `highlight = None` returns `None`.
**Q4b.4** `Down` with `rows_len = 0` returns `None` rather than a highlight of zero.
**Q4b.5** An ordinary character key returns `None` for every state.

---

## §5 Performance

**Budget**: `rank` over 500 names of realistic length (up to ~60 characters) for a query of up to 32
characters completes in **≤16 ms** — one frame at 60 fps (SC-002).

Held by a test in `micold-core` over synthetic names, not by inspection. There is no cache and no
debounce: FR-005 requires the visible results to correspond to the complete current text, and a
debounce is exactly a window in which they do not (research R11).

---

## §6 What this module must not do

- It must not name a colour, a font, a size or any rendering type — it is compiled into the iced-free
  crate and could not.
- It must not know what it is ranking. `rank` takes a key function; nothing branch-shaped crosses the
  boundary. Nor may it name a rendering type: `Key` is this module's own enum, not the rendering
  stack's.
- It must not hold state between calls. Two calls with the same inputs return equal outputs, always
  (which is what makes Q3.3 checkable).
