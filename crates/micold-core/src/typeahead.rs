//! Type-ahead matching, ranking, truncation and keyboard rules (feature 021).
//!
//! Everything a type-ahead decides, decided here — over plain strings and this module's own key
//! enum, with no knowledge of what is being searched and no way to name a rendering type. The
//! component in `micold-client` translates its inputs into these terms, applies the answers, and
//! decides nothing itself.
//!
//! That split is Principle I's doing rather than tidiness: "a literal match outranks an
//! approximate one", "Down stops at the last row rather than wrapping" and "Enter on an
//! unavailable row does nothing" are business rules, and business rules do not live in code that
//! `tests/` cannot reach. `micold-client`'s `keymap.rs` already draws the same line for the
//! terminal's key encoding.
//!
//! Contract: `specs/021-branch-typeahead-search/contracts/match-ranking.md`.

use std::ops::Range;

/// One character's case-folded form.
///
/// `char::to_lowercase` yields an *iterator*, because a few characters lower-case into several
/// (`İ` → `i` + a combining dot). Taking the first keeps one input character mapped to exactly one
/// comparison character, which is what lets every offset this module returns index the **original**
/// name rather than some folded copy of it. Branch names do not contain those characters; a rule
/// that silently shifted every offset by one would matter everywhere.
fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// A normalised search text: trimmed, case-folded, and held as characters because the floor the
/// approximate tiers sit behind is counted in characters (contract §1).
///
/// Folding happens once, here, rather than per candidate — with 500 candidates on every keystroke
/// that is the difference between one pass over the query and five hundred.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Query {
    chars: Vec<char>,
}

impl Query {
    /// The query `text` means, with surrounding whitespace and letter case removed.
    ///
    /// Interior whitespace survives and is matched literally: splitting on it would be a second
    /// matching rule that nothing specifies.
    pub fn new(text: &str) -> Self {
        Self {
            chars: text.trim().chars().map(fold).collect(),
        }
    }

    /// Whether this query filters anything. An empty query matches everything (FR-002).
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// The query's length in characters — the number FR-006a's three-character floor reads.
    pub fn char_len(&self) -> usize {
        self.chars.len()
    }

    /// The folded characters, for the tiers to walk.
    pub(crate) fn chars(&self) -> &[char] {
        &self.chars
    }
}

/// Which tier matched — carried on the result rather than recomputed, because it decides both the
/// rank (contract §3) and the shape of the emphasis (FR-010).
///
/// The derived `Ord` **is** the ranking order, and the declaration order is also the order the
/// tiers are tried in. Reordering these variants reorders every result list in the application.
///
/// Why a typo outranks an abbreviation, and is tried first: a dropped-letter typo is *also* a
/// subsequence — `reportng` sits inside `reporting` in order — so trying subsequence first would
/// claim every deletion typo and emphasise it as scattered characters, leaving FR-010's
/// whole-word rule reachable only by substitutions. `feat/r-e-p-o-r-t-i-n-g` with a gap at the `i`
/// is the broken word the design explicitly rejected. Trying the closer reading first also ranks
/// better: someone who types `reportng` has almost certainly mistyped `reporting` rather than
/// abbreviated something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchKind {
    /// The name contains the query verbatim, ignoring case.
    Literal,
    /// Some window of the name is within one insert, delete or substitution of the query.
    SingleEdit,
    /// The query's characters occur in order, not necessarily adjacent.
    Subsequence,
}

/// What one candidate scored against one query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// The tier that hit.
    pub kind: MatchKind,
    /// Byte offset of the match's start in the **original** name — the secondary rank key.
    pub at: usize,
    /// Byte ranges of the original name to emphasise: ascending, non-overlapping, and always on
    /// character boundaries. Empty exactly when the query is empty.
    pub spans: Vec<Range<usize>>,
}

impl Match {
    /// The match an empty query produces: everything matches, nothing is emphasised (FR-002).
    fn everything() -> Self {
        Self {
            kind: MatchKind::Literal,
            at: 0,
            spans: Vec::new(),
        }
    }
}

/// A name prepared for comparison: its characters folded, with the byte offset each one started at.
///
/// The offsets are why this exists. Comparing against a lower-cased *copy* of the name would give
/// offsets into that copy, and folding can change a string's length — so every span the caller
/// received would be subtly wrong for exactly the names that are hardest to notice. Folding
/// per-character and keeping the original offsets alongside means every range this module returns
/// indexes the name the caller passed in.
struct Folded {
    chars: Vec<char>,
    /// `offsets[i]` is where character `i` starts; the final entry is the name's length, so
    /// `offsets[i..j]` describes a byte range without a special case at the end.
    offsets: Vec<usize>,
}

impl Folded {
    fn new(name: &str) -> Self {
        let mut chars = Vec::with_capacity(name.len());
        let mut offsets = Vec::with_capacity(name.len() + 1);
        for (at, c) in name.char_indices() {
            chars.push(fold(c));
            offsets.push(at);
        }
        offsets.push(name.len());
        Self { chars, offsets }
    }

    fn len(&self) -> usize {
        self.chars.len()
    }

    /// The byte range covering characters `from..to`.
    fn span(&self, from: usize, to: usize) -> Range<usize> {
        self.offsets[from]..self.offsets[to]
    }
}

/// What `name` scores against `query`, or `None` if it does not match at all.
///
/// The tiers are tried in order and the **first hit wins**. That is not an optimisation: a name
/// found literally also satisfies both approximate tiers, and reporting the weaker one would sort
/// it below genuine approximations and scatter its emphasis across three characters instead of
/// marking a word (contract §2.4).
pub fn match_one(name: &str, query: &Query) -> Option<Match> {
    if query.is_empty() {
        return Some(Match::everything());
    }

    let folded = Folded::new(name);
    let q = query.chars();

    if let Some(m) = literal(&folded, q) {
        return Some(m);
    }

    // FR-006a: below three characters, single-edit tolerance would match nearly every branch and
    // an abbreviation is indistinguishable from noise. Literal only.
    if q.len() < MIN_APPROXIMATE_CHARS {
        return None;
    }

    // Single edit is tried first — a typo read as the word it meant beats the same typo read as
    // scattered characters. Below five it is tried in a narrowed form; see `single_edit`.
    if let Some(m) = single_edit(&folded, q) {
        return Some(m);
    }

    subsequence(&folded, q)
}

/// The query length at which the approximate tiers switch on (FR-006a).
const MIN_APPROXIMATE_CHARS: usize = 3;

/// The query length at or above which the single-edit tier runs unrestricted (contract §2.2).
///
/// Below it the tier still runs, but only over windows that are *anchored*: see
/// [`anchored_single_edit`] for what that means and why the restriction is shaped that way rather
/// than as a flat floor.
const UNRESTRICTED_SINGLE_EDIT_CHARS: usize = 5;

/// Tier 1 — the name contains the query verbatim, at its leftmost occurrence.
fn literal(name: &Folded, query: &[char]) -> Option<Match> {
    if query.len() > name.len() {
        return None;
    }
    let last_start = name.len() - query.len();
    (0..=last_start)
        .find(|&i| name.chars[i..i + query.len()] == *query)
        .map(|i| {
            let span = name.span(i, i + query.len());
            Match {
                kind: MatchKind::Literal,
                at: span.start,
                spans: vec![span],
            }
        })
}

/// Tier 2 — the query's characters occur in order, matched greedily leftmost.
///
/// Greedy-leftmost is normative rather than incidental (contract §2.2). Several alignments are
/// usually possible, and if the choice among them were left to chance the same search would
/// emphasise different characters on different runs — which makes "you can see why each result is
/// here" (SC-005) unverifiable.
fn subsequence(name: &Folded, query: &[char]) -> Option<Match> {
    let mut spans = Vec::with_capacity(query.len());
    let mut next = 0usize;
    for &wanted in query {
        let found = (next..name.len()).find(|&i| name.chars[i] == wanted)?;
        spans.push(name.span(found, found + 1));
        next = found + 1;
    }
    Some(Match {
        kind: MatchKind::Subsequence,
        at: spans[0].start,
        spans: merge_adjacent(spans),
    })
}

/// Tier 3 — some window of the name is within one insert, delete or substitution of the query.
///
/// The whole window is emphasised as a single run, including the character that differs, so the
/// emphasised text still reads as a word (FR-010). Marking only the characters that literally
/// aligned would leave a gap at the typo — a broken word, which is harder to read than no emphasis
/// at all.
fn single_edit(name: &Folded, query: &[char]) -> Option<Match> {
    let q = query.len();
    let anchored = q < UNRESTRICTED_SINGLE_EDIT_CHARS;
    // A window is at most one character longer than the query, so a name shorter than that cannot
    // hold one. Checked first because it is the cheapest way to dismiss a long query outright.
    if name.len() + 1 < q {
        return None;
    }
    if !within_one_edit_is_possible(name, query) {
        return None;
    }

    // Same length first, so a substitution is preferred over an insertion that happens to work —
    // the closest reading of the query wins, and the choice is fixed rather than incidental.
    //
    // A short query gets same-length windows only. The other two widths are what let a
    // *two*-character window answer a three-character search, which is a coincidence rather than a
    // typo; at four characters and below there is not enough query left for the difference to mean
    // anything.
    let widths: &[usize] = if anchored { &[q] } else { &[q, q + 1, q - 1] };
    for start in 0..=name.len() {
        for &width in widths {
            if width == 0 || start + width > name.len() {
                continue;
            }
            let window = &name.chars[start..start + width];
            if anchored && !anchored_single_edit(window, query) {
                continue;
            }
            if edit_distance_at_most_one(window, query) {
                let span = name.span(start, start + width);
                return Some(Match {
                    kind: MatchKind::SingleEdit,
                    at: span.start,
                    spans: vec![span],
                });
            }
        }
    }
    None
}

/// A cheap rejection before the window scan: every query character absent from the name needs an
/// edit of its own, so two absentees put the whole name out of reach.
///
/// This is what keeps the worst case — a query nothing matches, where every candidate falls
/// through every tier — inside the frame budget (SC-002).
fn within_one_edit_is_possible(name: &Folded, query: &[char]) -> bool {
    let mut missing = 0;
    for &c in query {
        if !name.chars.contains(&c) {
            missing += 1;
            if missing > 1 {
                return false;
            }
        }
    }
    true
}

/// Whether a short query may be read as a typo of `window` at all.
///
/// The rule: the first and last characters must agree. Below five characters the tier had to be
/// narrowed — two searches were being read as typos when they were plainly not — and both had the
/// same shape. `frep` against `feat/reporting` is one substitution from the window `/rep`, so the
/// tier claimed it and emphasised a slash; but a developer who types `frep` has abbreviated
/// `f`…`rep`, not mistyped `/rep`. `llo` against `release/local-login` matched the two-character
/// window `lo`, which is a coincidence at that length rather than a typo.
///
/// What both have in common is disagreement at an **end**. The first character of a search is the
/// one the developer is surest of — it is the letter the branch starts with, or the initial they
/// are abbreviating from — and the last is what they have just typed. A wrong character *between*
/// them is a slip; a different first letter is a different word. So `lagi` still finds `logi`, and
/// `frep` is left to the tier that reads it correctly.
///
/// Above five characters none of this is needed: one wrong character in five or more is a small
/// enough share that the window cannot be a coincidence, and anchoring would then refuse the
/// perfectly ordinary typo of getting the first letter wrong.
fn anchored_single_edit(window: &[char], query: &[char]) -> bool {
    match (window.first(), query.first(), window.last(), query.last()) {
        (Some(wf), Some(qf), Some(wl), Some(ql)) => wf == qf && wl == ql,
        _ => false,
    }
}

/// Whether two character slices are within Levenshtein distance 1.
///
/// Bounded, so it is a linear scan rather than a matrix: at equal lengths the distance is at most
/// one exactly when at most one position differs, and at lengths differing by one exactly when
/// deleting a single character from the longer one yields the shorter.
fn edit_distance_at_most_one(a: &[char], b: &[char]) -> bool {
    match a.len().abs_diff(b.len()) {
        0 => a.iter().zip(b).filter(|(x, y)| x != y).count() <= 1,
        1 => {
            let (long, short) = if a.len() > b.len() { (a, b) } else { (b, a) };
            let mut skipped = false;
            let mut i = 0;
            for (j, &c) in short.iter().enumerate() {
                let _ = j;
                if long[i] != c {
                    if skipped {
                        return false;
                    }
                    skipped = true;
                    i += 1;
                    if i >= long.len() || long[i] != c {
                        return false;
                    }
                }
                i += 1;
            }
            true
        }
        _ => false,
    }
}

/// Joins ranges that touch, so a subsequence that happens to align consecutively is emphasised as
/// one run rather than as several abutting ones. Purely so the rendered result has no seams in it.
fn merge_adjacent(spans: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::with_capacity(spans.len());
    for span in spans {
        match out.last_mut() {
            Some(last) if last.end == span.start => last.end = span.end,
            _ => out.push(span),
        }
    }
    out
}

/// Which of `items` match `query`, in the order they should be shown.
///
/// Returns **indices** into the caller's slice rather than clones, which is what lets this order
/// things it knows nothing about: `key` says where the text is, and nothing else crosses the
/// boundary (FR-019).
///
/// The sort is stable, and that is load-bearing: two candidates matching in the same tier at the
/// same offset have nothing left to separate them, so they must keep the order the caller gave —
/// otherwise an identical search would produce a differently-ordered list each time it ran
/// (contract §3).
pub fn rank<T>(items: &[T], key: impl Fn(&T) -> &str, query: &Query) -> Vec<(usize, Match)> {
    let mut out: Vec<(usize, Match)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match_one(key(item), query).map(|m| (i, m)))
        .collect();
    out.sort_by(|(_, a), (_, b)| a.kind.cmp(&b.kind).then(a.at.cmp(&b.at)));
    out
}

/// The character standing in for what did not fit. One glyph rather than three dots: it is what the
/// typographic convention actually is, and it costs a third of the width. Matches `Ellipsized`.
const ELLIPSIS: char = '…';

/// The longest window of `name` that fits in `available` **and still contains its emphasis**,
/// together with those spans rebased onto the returned string (FR-011d).
///
/// The ordinary answer — truncate the tail — hides exactly the text the developer searched for when
/// the match sits near the end, leaving a row with no visible reason for being in the list. So the
/// window follows the emphasis, and the ellipsis goes wherever the cut fell.
///
/// `measure` is supplied by the caller because only the rendering layer knows about fonts; passing
/// it in is what makes this rule provable without a renderer (FR-021) and provable *with* one where
/// the widths are real.
pub fn fit_around(
    name: &str,
    spans: &[Range<usize>],
    available: f32,
    measure: impl Fn(&str) -> f32,
) -> (String, Vec<Range<usize>>) {
    if name.is_empty() || measure(name) <= available {
        return (name.to_string(), spans.to_vec());
    }

    let cuts: Vec<usize> = name
        .char_indices()
        .map(|(at, _)| at)
        .chain(std::iter::once(name.len()))
        .collect();
    let index_of = |byte: usize| cuts.partition_point(|&c| c < byte);

    // What must stay visible. With no emphasis there is nothing to keep, so the window starts at
    // the beginning and this behaves exactly like an ordinary trailing truncation.
    let (keep_from, keep_to) = match (spans.first(), spans.last()) {
        (Some(first), Some(last)) => (index_of(first.start), index_of(last.end)),
        _ => (0, 0),
    };

    // Grow one character at a time, alternating sides, so the match ends up centred in whatever
    // room there is rather than pinned against one edge.
    let (mut from, mut to) = (keep_from, keep_to);
    let render = |from: usize, to: usize| {
        let mut out = String::new();
        if from > 0 {
            out.push(ELLIPSIS);
        }
        out.push_str(&name[cuts[from]..cuts[to]]);
        if to < cuts.len() - 1 {
            out.push(ELLIPSIS);
        }
        out
    };

    // The emphasis alone may not fit; shrink it from the right until it does, so a row is never
    // returned with no emphasis visible at all.
    while to > from && measure(&render(from, to)) > available {
        to -= 1;
    }

    let mut grow_end = true;
    loop {
        let grown = if grow_end && to < cuts.len() - 1 {
            Some((from, to + 1))
        } else if !grow_end && from > 0 {
            Some((from - 1, to))
        } else if to < cuts.len() - 1 {
            Some((from, to + 1))
        } else if from > 0 {
            Some((from - 1, to))
        } else {
            None
        };
        let Some((next_from, next_to)) = grown else {
            break;
        };
        if measure(&render(next_from, next_to)) > available {
            break;
        }
        (from, to) = (next_from, next_to);
        grow_end = !grow_end;
    }

    let out = render(from, to);
    let shift = if from > 0 { ELLIPSIS.len_utf8() } else { 0 };
    let window = cuts[from]..cuts[to];
    // Clipped to the window rather than dropped by it. A span the window only half covers still has
    // half its characters on screen, and dropping it would leave that text drawn as ordinary body
    // copy — a row with a visible match and no visible reason for being in the list, which is the
    // exact outcome this function exists to prevent. Both ends are character boundaries, so the
    // clamp cannot land mid-character.
    let rebased = spans
        .iter()
        .filter_map(|s| {
            let start = s.start.max(window.start);
            let end = s.end.min(window.end);
            (start < end).then(|| (start - window.start + shift)..(end - window.start + shift))
        })
        .collect();

    (out, rebased)
}

/// A key press, in this module's own terms.
///
/// Deliberately not the rendering stack's key type: the rule below has to be testable in a crate
/// that cannot name one. The widget maps its events onto this, exactly as the terminal's widget
/// maps its own onto `keymap::KeyInput`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Move towards the end of the list.
    Down,
    /// Move towards the start of the list.
    Up,
    /// Take the highlighted row.
    Enter,
    /// Close the list without taking anything.
    Escape,
    /// Move focus out of the field.
    Tab,
    /// Anything else — belongs to the field.
    Other,
}

/// Which way the highlight moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Towards the start of the list.
    Prev,
    /// Towards the end of the list.
    Next,
}

/// What a key press means for an open result list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Move the highlight.
    Move(Direction),
    /// Take the highlighted row.
    Pick,
    /// Close the list, taking nothing.
    Dismiss,
}

/// Where the highlight lands after moving `direction` through `rows` results, or `None` when there
/// is nowhere to land (FR-017a).
///
/// The companion to [`intent_for`]: that decides *whether* a key moves the highlight, this decides
/// *where to*. Both halves of the rule live here for the same reason — "Down stops at the last row"
/// is a decision a user can see, and a reducer that reimplemented it would be a second copy free to
/// drift from the first. It has drifted before: the picker and the gallery each grew their own, and
/// disagreed about where `Up` from nowhere lands.
///
/// Saturating rather than wrapping, and a move from nowhere *enters* the list — at the first row
/// going down, the last going up — which is how a keyboard reaches a list it has not touched.
pub fn move_highlight(
    highlight: Option<usize>,
    direction: Direction,
    rows: usize,
) -> Option<usize> {
    if rows == 0 {
        return None;
    }
    // A highlight that already sits past the end is pulled back onto the list before it moves, so
    // one press is always enough to reach a real row — `Prev` from a stale index would otherwise
    // step from nowhere to somewhere still out of range.
    let from = highlight.map(|i| i.min(rows - 1));
    Some(match (from, direction) {
        (None, Direction::Next) => 0,
        (None, Direction::Prev) => rows - 1,
        (Some(i), Direction::Next) => (i + 1).min(rows - 1),
        (Some(i), Direction::Prev) => i.saturating_sub(1),
    })
}

/// What `key` means, given where the highlight is, how many rows there are, and whether the
/// highlighted row can be chosen. `None` means the key belongs to the field (FR-017, FR-017a).
///
/// Every rule a user can observe is here rather than in the widget: movement saturates instead of
/// wrapping, because a list whose length changes on every keystroke gives no sense of place to wrap
/// around in; and `Enter` refuses an unavailable row, because FR-012a has to hold on both routes to
/// a pick or the mouse enforces a rule the keyboard ignores.
pub fn intent_for(
    key: Key,
    highlight: Option<usize>,
    rows: usize,
    highlighted_enabled: bool,
) -> Option<Intent> {
    match key {
        Key::Escape => Some(Intent::Dismiss),
        // Tab takes focus out of the field, and a list that outlived the focus that opened it would
        // go on claiming Enter and the arrows from whatever was tabbed to — so the next Enter would
        // pick a branch instead of pressing the button the developer had just reached. The key is
        // still the field's: the widget publishes this without capturing it, so focus moves on.
        Key::Tab => Some(Intent::Dismiss),
        Key::Enter => (highlight.is_some() && highlighted_enabled).then_some(Intent::Pick),
        Key::Down | Key::Up if rows == 0 => None,
        Key::Down => match highlight {
            Some(i) if i + 1 >= rows => None,
            _ => Some(Intent::Move(Direction::Next)),
        },
        Key::Up => match highlight {
            Some(0) => None,
            _ => Some(Intent::Move(Direction::Prev)),
        },
        Key::Other => None,
    }
}

/// Whether an open list **claims** `key`, or only acts on one that is on its way past.
///
/// The companion to [`intent_for`], and only meaningful for a key that has one: this answers what
/// happens to the *event* once the list has decided what the key means.
///
/// Claiming is what stops a key doing two things at once — Enter taking a row must not also submit
/// the dialog behind the list, and Escape closing the list must not also close the dialog. Tab is
/// the single exception, and it is an exception on purpose: what Tab is *doing* is moving focus
/// out, so the list dismisses and lets it through. Swallowing it would leave a developer unable to
/// tab past the picker at all.
///
/// **Here rather than in either widget, because both controls need it and they answer keys by two
/// different routes** (feature 022, T030 — FR-024, SC-008). The search picker's openness is its
/// caller's, so `cdk::picker` publishes a message and reports the claim to the runtime; the
/// select's is its own, so it acts in its own `update` and reports the claim itself. Each had
/// written this rule out, and they had already come apart: the select applied the Tab exception
/// and then never captured anything at all, so Escape closed its list *and* the dialog behind it.
/// One `matches!` in two places is exactly enough room for that.
pub fn claims(key: Key) -> bool {
    !matches!(key, Key::Tab)
}
