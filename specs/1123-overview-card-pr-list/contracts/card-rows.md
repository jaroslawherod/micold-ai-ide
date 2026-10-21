# Contract: rows on the card (core rules, client widgets)

## Core (`micold_core::pr_list`, `overview_expansion`) — unit tests K1–K6

- K1 order: open/draft before merged/closed; newest first; equal `created_at` → higher number.
- K2 `StatusPhrase::of`: each precedence pair (failing beats changes requested beats conflict beats
  pending beats approved beats passing); merged/closed → `None`; draft like open; review required
  alone → `None`.
- K3 the first row is the current pull request for every combination of states (property over
  generated lists with distinct `created_at` per branch and at least one own node in `r`; the two
  accepted edge cases of data-model §2 are each pinned by one example test of what they give).
- K4 `rows(false)` → ≤ 2 rows and `len - 2` more; `rows(true)` → all, 0 more; ≤ 2 listed → same
  rows both ways, 0 more.
- K5 `CardLink` and the last-gain focus model (data-model §5). Each case in both report orders,
  loss first (`LinkFocusLost(a, P)`, `LinkFocusGained(a, Q)`) and gain first:
  - K5a link → link on a Focus-expanded card: stays expanded, `focused_link() == (a, Q)`.
  - K5b link → link after Escape collapsed the card: stays collapsed, no re-expansion (FR-010).
  - K5c card → `PullRequest(n)` and `PullRequest(n)` → card, expanded and Escape-collapsed: no form
    change; `focused_link()` is `(a, PullRequest(n))`, then `None`.
  - K5d gain first `#1` → `#2`, then `#2` loses to another card's gain: the card collapses (the
    reviewer's "stays expanded after focus leaves" case).
  - K5e a stale loss of `#1` after `#2` gained and the card has since taken it back: nothing.
  - K5f menu: `LinkFocusGained(a, PullRequest(n))`, `MenuOpened`, `LinkFocusLost(a,
    PullRequest(n))` holds `(a, PullRequest(n))`; `focused_link()` is `None` under the menu; the
    return gain on `a` restores `focus_part`; `WindowBlurred` under it applies the loss;
    `Listed` without `a` drops it.
  - K5g every 1122 K5 case, rewritten with `CardLink::Issue`, passes unchanged (including
    `Listed` keeping the pairing and the repeated report from the hand-off).
- K6 `ListedPr` `Debug` holds no address, `PrRow` has none; no `Serialize` (extend
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
- U5 hand-off (`main_tests.rs`, as 1122's U6): Escape on a focused row-3 number collapses the card
  and the keyboard goes to the card (`link_focus_after` returns `card_id`), the card stays
  collapsed and `focused_link()` is `None`; Escape on a focused row-1 number re-focuses that
  number's id (the card changed form, the number is still drawn); a reading that drops the focused
  pull request hands off to the card; `card_link_shown` for a row-3 number is false collapsed and
  true expanded.
- U6 menu return: the Menu key on a focused number opens the menu with
  `KeyboardFromLink(PullRequest(n))`; dismissing it focuses `card_link_id(a, PullRequest(n))`;
  choosing an entry focuses the card; a number no longer drawn when the menu closes returns to the
  card. The issue link's 1122 menu-return tests pass with `CardLink::Issue`.
- U7 `PullRequestLinkActivated(a, n)` opens the address `pull_request_link_for` finds; with `n` no
  longer listed it opens nothing; its `Debug` has no address.

## Wiring

`ui/overview.rs` builds rows from `CardModel.pull_requests` with the card's `expanded` flag,
ids `card_link_id(location, CardLink::PullRequest(n))`, focus events through
`Event::LinkFocus*(location, CardLink)`, the pill as `Tag::new(PrMark::word(), PrMark::role(roles))`.
`Msg::PullRequestLinkActivated(location, n)` → `main.rs` looks the address up with
`pull_request_link_for` and takes the opener path of `IssueLinkActivated` (feature 031 FR-015 on
failure). Hand-offs and the menu return: research R6, U5–U6.
