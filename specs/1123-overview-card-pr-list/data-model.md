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
| `checks` | `CheckStatus` (feature 040: none, pending, passing, failing) | `reduce_checks` of the last commit's counts; none for a node only `m` returned (R2: `m` asks no `commits`) |
| `review` | `ReviewState` (none, approved, changes requested, review required) | `reviewDecision`; none for an `m`-only node |
| `conflict` | `bool` | `mergeable == CONFLICTING`; false for an `m`-only node |
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
- An own node whose state and draft flag this version does not know (`branch_pr_state` fails)
  fails the whole reading as unreadable, as the selected node does today, not only that node: a
  reading is whole or none, and an answer that cannot be understood leaves every row as it was
  (feature 040 FR-019), and GitHub's `PullRequestState` enum is closed
  (`OPEN`, `CLOSED`, `MERGED`), so skipping such a node would only hide a schema change behind a
  shorter list (review A M1 r1 F2).

Invariants: the first entry, when any, is the sidebar's current pull request (`select_pull_request`
picks the newest own open one, else the newest own of any state; open ones are all in `o`) (K3),
whenever the branch's own pull requests have distinct `createdAt` and `r` holds at least one own
node. Two edge cases are known and accepted: (a) two own pull requests created in the same second
(GitHub's `createdAt` has one-second resolution): the list puts the higher number first while
`select_pull_request`'s `max_by` keeps the later node of the page; (b) `r` full of fork pull
requests while `m` holds an own merged one: the sidebar shows none and the card lists the merged
one. Both need a branch name shared with ten forks or two pull requests opened in one second on one
branch; the row is still a real pull request of the branch, and changing feature 040's selection
for them is out of scope.

## §3 `StatusPhrase` and `PrRow` (core)

`StatusPhrase`: `ChecksFailing`, `ChangesRequested`, `MergeConflict`, `ChecksPending`, `Approved`,
`ChecksPassing`, in this precedence. `StatusPhrase::of(&ListedPr) -> Option<Self>`: `None` for merged
and closed, else the first that applies; `None` when none does (no checks, review none or review
required, no conflict). `label()`: *checks failing*, *changes requested*, *merge conflict*, *checks
pending*, *approved*, *checks passing*.

`PrRow`: `number: u64`, `label: String` ("#N"), `pill: BranchPrState` (word: *Open*, *Draft*,
*Merged*, *Closed*), `phrase: Option<StatusPhrase>`. No address: activation carries the location
and number and looks the address up when pressed (R8).

## §4 `StatusReading` and client state (changed)

- `StatusReading { statuses, lists: BTreeMap<String, BranchPrList> }`: `counts` replaced; every
  asked branch has an entry.
- Client `features::pr_status::State.lists` replaces `counts`, kept and cleared at the same sites
  (`Ok` replaces; `Unavailable` and `Released` clear; `Passing` and `RateLimited` keep), so rows and
  counts survive a passing failure and a rate-limit pause and go with FR-025 (spec FR-007).
- `CardModel.pull_requests: Option<BranchPrList>`: `Some` only when the card shows counts.

## §5 `CardLink` and the focus model (core, `overview_expansion`)

`CardLink { Issue, PullRequest(u64) }` names one link of a card (R6).

| Item | Was (1122) | Now |
|---|---|---|
| `FocusPart` | `Card`, `Link`, `Both`; `other()` | `Card`, `Link(CardLink)`; the part of the last gain |
| `Event::LinkFocusGained` / `LinkFocusLost` | `(SessionLocation)` | `(SessionLocation, CardLink)` |
| `link_held` | `Option<SessionLocation>` | `Option<(SessionLocation, CardLink)>` |
| `JustLost.part` | `FocusPart` | `FocusPart` (with its link) |
| `focused_link()` | `Option<&SessionLocation>` | `Option<(&SessionLocation, CardLink)>` |

Transitions on the focused card `L` (everything else as 1122):

- loss of part `P`, then (next event) gain on `L` of part `Q ≠ P`: one move, restore the state
  before the loss, `focus_part = Q`.
- gain on `L` of `Q ≠ focus_part`: one move, `focus_part = Q`, no form change.
- gain on `L` of `Q = focus_part`: a repeated report, nothing.
- loss on `L` of `P ≠ focus_part`: the old half of a move, nothing.
- loss on `L` of `P = focus_part`: held when `P` is a link and a menu is open; else 1121's loss
  (collapse a `Focus` expansion) with `just_lost` recorded.

With only `Card` and `Link(Issue)` these are 1122's transitions (K5 keeps its cases).

Client (`features/overview.rs`, `ui/overview.rs`, `main.rs`, R6): `FocusReturn.to_link:
Option<CardLink>`; `MenuOrigin::KeyboardFromLink(CardLink)`; `Msg::LinkMenuRequested(SessionLocation,
CardLink)`; `Msg::PullRequestLinkActivated(SessionLocation, u64)`; `card_link_id(&SessionLocation,
CardLink)`; `card_link_shown(&State, &SessionLocation, CardLink) -> bool`; `pull_request_link_for(&State,
&SessionLocation, u64) -> Option<String>`; `shown_focused_link` and `LinkFocus.link` hold
`(SessionLocation, CardLink)`.
