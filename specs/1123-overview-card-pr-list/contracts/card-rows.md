# Contract: rows on the card (core rules, client widgets)

## Core (`micold_core::pr_list`, `overview_expansion`) — unit tests K1–K6

- K1 order: open/draft before merged/closed; newest first; equal `created_at` → higher number.
- K2 `StatusPhrase::of`: each precedence pair (failing beats changes requested beats conflict beats
  pending beats approved beats passing); merged/closed → `None`; draft like open; review required
  alone → `None`.
- K3 the first row is the current pull request for every combination of states (property over
  generated lists).
- K4 `rows(false)` → ≤ 2 rows and `len - 2` more; `rows(true)` → all, 0 more; ≤ 2 listed → same
  rows both ways, 0 more.
- K5 `CardLink`: Tab issue link → PR link → next PR link of one card emits no collapse; after
  Escape collapse, a move between the card and any of its links does not re-expand;
  `focused_link()` names the link that has focus; the menu return goes to that link.
- K6 `ListedPr`/`PrRow` `Debug` holds no address; no `Serialize` (extend
  `tests/pull_request_is_never_stored.rs`).

## Client — tests U1–U4

- U1 `OverviewCard::pull_requests(rows, more)`: draws "#N", the pill word and the phrase per row;
  "+N more" only when `more > 0`; at 280/380/480 px the number and pill are whole and the phrase
  ellipsizes (SC-006).
- U2 a click on "#N" publishes `on_pr_link(N)` and not `on_press`; a click on the pill or phrase
  publishes `on_press` (FR-011); no ripple from a press on the number.
- U3 Tab from the card: issue link, then each drawn number in order, then the next card;
  Shift+Tab reverse; a collapsed card's hidden numbers are not stops.
- U4 `CardGrid`: a rest on any number counts as a rest on the card's link (1122 FR-007); a press on
  a number of card 1 leaves card 0's keyboard; arrows from a focused number step cards.

## Wiring

`ui/overview.rs` builds rows from `CardModel.pull_requests` with the card's `expanded` flag,
ids `card_link_id(location, CardLink::PullRequest(n))`, focus events through `Event::LinkFocus*`.
`Msg::PullRequestLinkActivated(Address)` → `main.rs` opener path of `IssueLinkActivated` (feature
031 FR-015 on failure).
