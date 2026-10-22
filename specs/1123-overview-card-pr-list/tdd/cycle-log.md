# TDD cycle log: 1123 (M1)

The feature has no `tdd/test-list.md` (as 1121): tasks.md's test tasks are the list, and each row
names the task pair it covers. Append only. Baseline: `main` at 9607e14b (design PR #1230 merged),
green in CI.

## M1 (US1, US2: rows, counts from the list)

**Written after the fact (review B M1 r1 F1).** M1 landed as one commit (efff51eb) and its unit
kept no red output, so the red column below does not quote a failing run for T001–T018. What can
still be shown is structural: each test task names symbols that do not exist at the baseline, so
those tests could not compile there (`git grep` at 9607e14b: no `pub mod pr_list`, no
`PrMark::word`, no `CardPrRow`; `COUNT_PAGE` still present). Whether each behavioural assertion
failed on its own before its implementation is **not evidenced**; the close unit's tdd-verify should
treat these rows as green-only. Rows from review fixes onward carry real red output.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001, T002 | K1 `BranchPrList::new` dedupes by number and orders open/draft then merged/closed, newest first; K2 `StatusPhrase::of` precedence and words; `counts()` from the list, *10+* on a full page; `rows(expanded)` 2 collapsed + more, all expanded; `ListedPr`'s `Debug` omits the url | not captured; `micold_core::pr_list` absent at baseline (compile red) | `pr_list` unit tests, `cargo test -p micold-core --all-targets` green in the M1 gate at efff51eb |
| T003, T004, T006 | Q1/Q2 query has `m<i>` with scalar fields, no `oc`/`mc`, no branch name; parser builds lists from `o ∪ r ∪ m`, drops forks, `o`/`r` wins over `m`; fixtures re-recorded | not captured; the Q1 assertion (no `oc<i>`) contradicts the baseline query, which still had `COUNT_PAGE` | `pull_request_query`, `pull_request_parse`, `pull_request_counts`, `pull_request_source` green in the M1 gate |
| T005, T006 | K3 the list's first entry is `select_pull_request`'s pick, exhaustively over up to three own pull requests | not captured; needs `BranchPrList` (compile red) | `pull_request_select` green in the M1 gate |
| T007, T008 | `State.lists` replaced by an `Ok` reading, cleared by `Unavailable` and release, kept through rate limit and passing failure | not captured; `State.lists` absent at baseline (`counts`) | `features_pr_status` 30 passed (review B M1 r1 Verify) |
| T010, T011, T013, T014 | U1 rows drawn collapsed and expanded ("#N", pill word, phrase; *checks failing* in `error`; "+N more"); `PrMark::word`; solid pill contrast ≥ 4.5:1 in every card state, both schemes (D11) | not captured; `CardPrRow`, `.pull_requests`, `PrMark::word` absent at baseline (compile red) | `--lib overview_card pull_request_indicator composition_contrast` 90 passed (review B M1 r1 Verify) |
| T012, T015 | card rows from `State.lists` by `rows(expanded)`: sc. 1, collapsed two of four with "+2 more", expanded all, no list no row | not captured; needs `State.lists` (compile red) | `--test overview` 74 passed (review B M1 r1 Verify) |
| T016, T017 | showcase poses; covered state `overview-pr-rows` | not captured | `showcase_completeness` 43, `layout_snapshot` 84 passed (review B M1 r1 Verify) |
| T009, T018 | user guide | docs | — |
| visual r1 observation (US1 sc. 4, FR-007) | the dash keeps its reason as the tooltip on the expanded card, as on the collapsed one | `cargo test -p micold-client --lib -- the_dash_carries`: `the_dash_carries_its_reason_collapsed_and_expanded` `assertion left == right failed: expanded true left: []` (collapsed passed) | `counts_line` shared by both bodies wraps the dash in its `Tooltip`; `--lib overview_card` 77 passed |
| review A M1 r1 F1, F3, F4 | refactors with no new behaviour: `features::changes::pr_mark` reused, `StatusPhrase::label` takes the first three words from `CardReason`, `PrMark::on_role` merged into the one impl | guards: `each_phrase_has_its_words`, the U1 pill tests | green in the full gate (see ledger) |

## M2 (US3 part 1: a number opens its pull request by pointer)

Red runs stubbed the behaviour behind the new API (builders stored, numbers still plain text, no
link ids, no `main.rs` arm), so each test below failed for its behavioural reason, not to compile.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T020, T024 | U2: a click on "#12" publishes `pr 12` only, no ripple, keyboard stays; split press/release publishes nothing; pill or words activate the card; the card names its numbers' link ids after the issue link's; numbers no Tab stops yet; plain text without `.on_pr_link` | `--lib -- overview_card`: `a_click_on_a_number_publishes_only_its_pull_request`, `a_press_and_release_split_between_a_number_and_the_card_publish_nothing` FAILED; `the_card_names_its_numbers_link_ids_for_the_grid` `left: [] right: [Id(Custom("pr-12")), Id(Custom("pr-11"))]`. The pointer, pill/words and no-Tab-stop tests passed at red: they hold before the change too (the card's own pointer is the hand; the pill was never a link) and guard against regressions | `card_link` shared by the issue link and each number; `--lib -- overview_card card_grid ui::overview ui::focus text_link` 186 passed |
| T021, T025 | U4: a rest on a number publishes the card's `on_link_rest`; a press on card 1's number leaves card 0 the keyboard; one timed wake, none after (G3) | `a_rest_on_a_number_is_a_rest_on_a_link_of_its_card` `left: ["rest 1"] right: ["link rest 1"]`; `a_click_on_a_number_leaves_the_keyboard_with_the_card_that_holds_it` `left: ["open 1"] right: ["pr 1 12"]`; `a_rest_on_a_number_asks_for_no_frame_once_it_is_over` `left: (["rest 1"], Wait)` | `RestMessages.link_ids`, `ScreenBounds::of_any` in one pass; same run green |
| T022, T026 | the mounted Overview's click on "#12" asks `PullRequestLinkActivated(feat-0, 12)` and not the activation; the root opens the listed address, nothing once gone, FR-015 notification on failure | `ui::overview` `a_click_on_a_rows_number_asks_for_its_pull_request` FAILED; `--bin micold-ai-ide -- pull_request_link`: `pull_request_link_opens_the_listed_address` `left: [] right: ["https://github.com/o/r/pull/12"]`, `pull_request_link_failure_notifies` `left: None` (arm stubbed to `Task::none()`); `pull_request_link_opens_nothing_once_gone` passed at red (nothing opened either way) | `open_card_link` shared with `IssueLinkActivated`; bin filter 5 passed in the scoped gate |
| T022 (tests/overview.rs) | the message changes no state and prints no address; `pull_request_link_for` finds the listed address, none for an unlisted number, a gone card, Default, or the dash | written with the implementation (not run red) | scoped gate |
| T023 | `CardLink`, `card_link_id(location, CardLink)`: refactor, every existing test unchanged; ids of numbers collide with no other part | guard: `no_worktree_name_makes_one_cards_id_another_cards_part` extended | green |
| T027 | user guide | docs | — |
| Review A M2 r1 F1 | U7, FR-014: a listed address that is no GitHub page (`file:`, `javascript:`, another host) is never handed to the browser | `--bin micold-ai-ide -- pull_request_link_`: `pull_request_link_opens_no_address_off_github` FAILED (the opener was asked) | `pull_request_link_for` filters on `GITHUB_ADDRESS_PREFIX`; 4 passed |
| Review A M2 r1 F2 | a number takes the pointer, a click and a rest on its label only: the top pixel of "#11" opens #11, a press just above "#12" activates the card, a rest there is a card rest; `TextLink::hit_on_label` leaves the gutter's pointer and press to the neighbours | `a_number_takes_the_pointer_on_its_label_only` `left: ["pr 12"] right: ["pr 11"]`; `a_rest_in_a_numbers_gutter_is_a_rest_on_the_card` `left: ["link rest 1"] right: ["rest 1"]`; `a_link_hit_on_its_label_leaves_its_gutter_to_its_neighbours` `left: Pointer right: None` | `LinkBody::hit` (label box with `hit_on_label`) for pointer, press, underline and the container report; `ScreenBounds::of_any` reads containers only; `--lib -- card_grid overview text_link focus` 281 passed |
| Review A M2 r1 F3–F5 | refactors: `of_any(&[Id])` (no clone), page/menu chords only on a link that answers keys, `ScreenBounds` computes only what its mode reads | existing tests unchanged | same runs green |

## M3 (US3 part 2: the numbers by keyboard)

Red runs stubbed the behaviour behind the new API, as in M2: the core events took a `CardLink` but
kept the `Both`/`other()` pairing; the card stored `.on_pr_link_keyboard_focus` and
`.on_pr_link_menu` but drew no number as a Tab stop; `card_stops` listed no numbers;
`card_link_shown` held every number shown. Each test below failed for its behavioural reason.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T029, T033 | K5a–f: link → link and card ↔ number moves in both report orders keep the form (expanded, or collapsed after Escape) and name the last gained part; a number that gained last loses to another card; a stale loss of an older number changes nothing; a number's menu holds its focus, the return restores it, a blur applies the loss, a listing without the card drops it; K5g the 1122 cases with `CardLink::Issue` unchanged | `cargo test -p micold-core --lib -- overview_expansion`: 6 FAILED, e.g. `a_move_from_link_to_link_after_escape_stays_collapsed` `gain first: false left: Some("a") right: None`; `a_number_that_gained_last_loses_to_another_card` `left: None right: Some("a")`; `a_numbers_menu_holds_its_focus_until_the_return` `left: None right: Some((Worktree("a"), PullRequest(5)))`; 51 passed (K5g and 1121's K1) | last-gain `FocusPart { Card, Link(CardLink) }`; 57 passed |
| T030, T034 | U3: Tab from the card reaches the issue link, then each drawn number in row order, then the next card; Shift+Tab reverses; collapsed only the two drawn numbers; Enter/Space open the number without ripple; Menu/Shift+F10 its menu, captured; Page keys the card's; a number below the fold scrolls into the body | `--lib -- overview_card`: 4 FAILED: `tab_reaches_each_drawn_number_after_the_issue_link` `left: ["link focus false"] right: ["link focus false", "pr 12 focus true"]`; `numbers_are_tab_stops_only_with_focus_reports…` `left: Some(1) right: Some(3)`; `keys_on_a_focused_number…`, `a_focused_number_below_the_fold…` `left: [] right: ["pr 11 focus true"]` | `pr_number` passes the focus reports and menu message through `card_link`; 87 passed |
| T031, T035 (`step_card`) | U4: from a focused number the arrows go to the neighbouring cards, never onto a number or link | `--lib -- ui::overview::tests::the_arrows`: 2 FAILED: `the_arrows_step_from_a_number_to_the_neighbouring_cards` `left: Some(Id(Custom("overview-card/feat-0"))) right: Some(Id(Custom("overview-card")))`; `the_arrows_step_between_cards_and_never_onto_a_link` landed on `overview-card-pr-7/feat-0` | `card_stops` lists every listed number; `--lib -- overview_card card_grid overview` 182 passed |
| T032, T035 (root) | U5: Escape on a row-3 number hands the keyboard to the collapsed card; on a row-1 number back to the number; a reading without the focused pull request hands it to the card; `card_link_shown` follows the form. U6: a number's menu opens as `KeyboardFromLink(#n)`; dismissed it returns to the number, to the card after a choice or once the number is gone | `--bin micold-ai-ide -- link_menu card_focus_return link_focus card_link_shown`: 4 FAILED: `link_focus_escape_on_hidden_number_goes_to_card` `left: …pr-11… right: …overview-card/feat-1122_x`; `card_focus_return_focuses_the_number` `#13 is gone`; `link_focus_moves_to_card_when_pull_request_goes` `left: None`; `card_link_shown_follows_the_form` | `card_link_shown` from `rows(expanded)`; 23 passed (bin filter), `--test overview` 88 passed |
| T032 (tests/overview.rs, ui mount) | a number's focus reports and menu request reach the reducer and the menu with `CardLink::PullRequest(n)`; dismissed menu `to_link == Some(PullRequest(n))` | written with the implementation (not run red); the `CardLink`-carrying signatures did not exist before T035 | same runs green |
| T036 | user guide | docs | — |
