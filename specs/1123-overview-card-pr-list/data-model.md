# Data model: Overview cards, a pull request list with state, linked

All in memory, never serialised, never logged with an address or title (feature 040 FR-032, spec
FR-014). Decisions: [research.md](research.md).

## §1 `ListedPr` (core, `micold_core::pr_list`)

One own (non-fork) pull request of a branch, from one reading (R1).

| Field | Type | From |
|---|---|---|
| `number` | `u64` | `number` |
| `url` | `String` | `url`; redacted from `Debug` |
| `state` | `BranchPrState` (feature 1003: open, draft, merged, closed) | `state` + `isDraft`; unknown `state` → whole reading `Passing` |
| `checks` | `CheckStatus` (feature 040: none, pending, passing, failing) | `reduce_checks` of the last commit's counts |
| `review` | `ReviewState` (none, approved, changes requested, review required) | `reviewDecision` |
| `conflict` | `bool` | `mergeable == CONFLICTING` |
| `created_at` | `String` (ISO-8601 UTC, compares as text) | `createdAt` |

The title is read by the shared fragment but not kept: no row shows it (spec Out of scope). No
`Serialize`; `Debug` prints number and state only (test K6).

## §2 `BranchPrList` (core)

Per branch, per successful reading (R2, R4).

| Field | Meaning |
|---|---|
| `prs: Vec<ListedPr>` | own nodes of `o ∪ r ∪ m`, one per number, in FR-003 order (≤ 30) |
| `open_full: bool` | `o` came back with 10 nodes (forks included): more open may exist |
| `merged_full: bool` | `m` came back with 10 nodes: more merged may exist |

- `new(prs, open_full, merged_full)` dedupes by number and sorts: open and draft first, then merged
  and closed; each group `created_at` descending, then higher number first.
- `counts() -> PrCounts`: `open` = open + draft in `prs`, `merged` = merged in `prs`,
  `open_capped = open_full`, `merged_capped = merged_full`. Since `o ⊆ list` and `m ⊆ list`, these
  equal today's per-page own counts with a page size of 10.
- `rows(expanded: bool) -> (Vec<PrRow>, usize)`: the first 2 (collapsed) or all (expanded) as
  rows, and the number not shown (0 expanded).
- Empty list: the branch was read and has no pull request: no row, no counts line.

Invariants: the first entry, when any, is the sidebar's current pull request (`select_pull_request`
picks the newest own open one, else the newest own of any state; open ones are all in `o`) (K3).

## §3 `StatusPhrase` and `PrRow` (core)

`StatusPhrase`: `ChecksFailing`, `ChangesRequested`, `MergeConflict`, `ChecksPending`, `Approved`,
`ChecksPassing`, in this precedence. `StatusPhrase::of(&ListedPr) -> Option<Self>`: `None` for merged
and closed, else the first that applies; `None` when none does (no checks, review none or review
required, no conflict). `label()`: *checks failing*, *changes requested*, *merge conflict*, *checks
pending*, *approved*, *checks passing*.

`PrRow`: `number: u64`, `label: String` ("#N"), `pill: BranchPrState` (word: *Open*, *Draft*,
*Merged*, *Closed*), `phrase: Option<StatusPhrase>`, `url: String` (redacted `Debug`).

## §4 `StatusReading` and client state (changed)

- `StatusReading { statuses, lists: BTreeMap<String, BranchPrList> }`: `counts` replaced; every
  asked branch has an entry.
- Client `features::pr_status::State.lists` replaces `counts`, kept and cleared at the same sites
  (`Ok` replaces; `Unavailable` and `Released` clear; `Passing` and `RateLimited` keep), so rows and
  counts survive a passing failure and a rate-limit pause and go with FR-025 (spec FR-007).
- `CardModel.pull_requests: Option<BranchPrList>`: `Some` only when the card shows counts.

## §5 `CardLink` (core, `overview_expansion`)

`CardLink { Issue, PullRequest(u64) }` names one link of a card (R6). `FocusPart::Link(CardLink)`;
`Event::LinkFocusGained(SessionLocation, CardLink)`, `Event::LinkFocusLost(SessionLocation,
CardLink)`; `focused_link() -> Option<(&SessionLocation, CardLink)>`. A loss on one link of a card
followed by a gain on another link of the same card is no move (no collapse, no re-expansion of an
Escape-collapsed card) (K5).
