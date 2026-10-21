# Autopilot ledger — #1123 overview-card-pr-list

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #1123: Overview cards: pull request list with state, linked. Blocked by: #1121 (expanded view for the rest of the list). The app reads PR counts only. Read, for each PR of a branch: number, URL, state (open, draft, merged, closed), checks (passing, failing, running, none), review (approved, changes requested) and merge conflict. Reuse the existing PR status reading and its rate-limit handling; no new polling. The card shows up to two PR rows (number as a link, state pill, checks or review in words), the rest on expand. A press on the number opens the PR and does not activate the card. A worktree with no PR shows no row. Acceptance: core type and parser tests; unreadable states keep today's dash reasons; counts derive from the list; docs and layout snapshot updated. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #1123
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-21
- **Phase**: tasks
- **Next step**: tasks review A r1 (orchestrator dispatches; prompt 1123-prompt-tasks-r1.md in the scratchpad); then close checklists (all ticked), specs-only gate, design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T019 | full | Cards show pull request rows (#N, state pill, status words), two collapsed with "+N more", all expanded; counts derive from the same list and read *10+* (43 requests per branch, was 44); snapshot `overview-pr-rows` | — | pending |
| M2 | T020–T028 | full | A click on a row's number opens the pull request, no card activation | — | pending |
| M3 | T029–T037 | full | Numbers are Tab stops with every 1122 link rule, arrows step from a number; Escape/menu hand-offs per link | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Base | origin/main f1d7d1b5 (#1121, #1122 merged) | agent-resolved | orchestrator prompt |
| D2 | spec | Check and state words (review A r1 F2) | Reuse feature 1003's words: *pending*, not *running*; states open, draft, merged, closed. Row order keeps open/draft first on purpose. Word choice also named in the FR-004 clarify question | orchestrator default | 1003 FR-001, FR-003; 040 FR-008; no clash found |
| D3 | clarify | FR-004: one status phrase or each? | One phrase per row by precedence: checks failing, changes requested, merge conflict, checks pending, approved, checks passing (first three in 813's attention order); 1003's words | orchestrator default, owner card pending | 813 card_state order (crates/micold-core/src/overview.rs ~l.328); 1003 FR-003 |
| D4 | clarify | FR-008: how many PRs per branch? | Newest 10 of any state + newest 10 open/draft (today's `o`/`r` pages), no more GitHub points than today; a count of 10 or more reads *10+* (replaces 813's *100+* on the card) | orchestrator default, owner card pending | pull_request.rs status_query (NEWEST_TEN, COUNT_PAGE) |
| D5 | clarify | Second clarify round | No critical ambiguities after D3/D4 (they only fix wording and a cap); clarify CLEAN | agent-resolved | spec.md scan |
| D6 | plan | How counts derive from a 10-cap list at no more cost | A newest-10 merged page `m<i>` of scalar fields only (no `commits`) replaces the 100-node `oc`/`mc` count pages: 44 → 43 connection requests per branch by GitHub's rate-limit score (plan review A r1 F1 corrected the earlier node-count figure); spec FR-008, FR-006 (fork *N+*) and Key Entities (title not kept) amended with the plan | agent-resolved | research.md R1, R2 |
| D7 | tasks | Milestone cut | US1 ships whole in M1 (list, counts and rows, T001–T019): tasks review A r1 F1 showed a split of US1 leaves the first half no deliverable (its scenarios sc. 3, 4, 6, 8 all hold before this feature; *10+* shows less than today), and the folded diff is not too big to review (one core module, a renamed map, one card builder), so no split along sc. 1/2/5; US2 (P2) ships in M1 with the rows: its only rule is `rows(expanded)` (K4) and a card that draws rows must draw them right expanded; US3 (P1) split along its scenarios into M2 (pointer, sc. 1–3, 5) and M3 (keyboard, sc. 4); Polish T038–T039 code-free → close unit | agent-resolved | milestones.md rules 1–4, 6 |
| D8 | tasks | Contract P1 count | contracts/reading.md P1 said "1 open (draft counted)" for four states; open + draft is 2, fixed to "2 open (the draft counted) · 1 merged" (plan text fix) | agent-resolved | data-model §2 `counts()` |
| D9 | tasks | `speckit-analyze` | Not invokable in this cloud session (Skill: unknown skill); mechanical coverage check run instead (every FR-001–FR-017, SC-001–SC-007, K1–K6, K5a–K5g, Q1–Q2, P1–P7, U1–U7 named in tasks.md; all tasks in checklist format); the tasks review r1 is asked to apply the analyze checks too | agent-resolved | this unit |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A spec | 1 | afc9cb5d94279a3c0ae00d37d80855a145ded446:f1d7d1b5b3251cbb108318452ccb6a4ede3d1f3e | CHANGES: 1 MAJOR (F1 Story 1 sc 4 dash vs kept rows), 3 MINOR (F2 feature 1003 words, F3 FR-010/SC-002 after Escape, F4 function name in Edge Cases); all four fixed in spec.md, none declined |
| Review A spec | 2 | 825ca900990f4c8ff4f75b661ae433152e67cae6:91b05eb41e1d852e023be0f044cc857a68bc5800 (fix diff 451f1d2c..91b05eb4, sonnet) | CLEAN, 1 MINOR (F1 FR-007 FR-025 wording); fixed, prose only, no round 3 |
| Review A plan | 1 | 53ffa8f39a3a4caa715af2e64d4fca74c90e6a87:a55fb1b02c4bca128a2f493349a45dbf294074aa | CHANGES: 1 BLOCKER (F1 rate-limit points: request-count rule, `m<i>` with `...pr` costs 63 > 44), 3 MAJOR (F2 focus pairing for several links, F3 client hand-offs/menu return per link, F4 `StatusChip` does not fit a PR pill), 3 MINOR (F5 K3 edge cases, F6 address in Msg, F7 missing alternatives); all seven fixed in research R2/R5–R10, data-model §1–§5, contracts, plan, quickstart; none declined; round 2 on sonnet over the fix diff |
| Review A plan | 2 | 9bc517ce26e6300ed4d9f258bb7fd82b0e0fad46:f3fad1fa408ff0b5345496fd0d6917e547bb9eed (fix diff 53ffa8f3:a55fb1b0 → f3fad1fa, sonnet) | CLEAN, 2 MINOR (F1 card-rows.md U1–U7 heading, F2 R9 contrast wording); both fixed, prose only, no round 3 |
| Review A tasks | 1 | 8c136e70f07d2dc1576652bbf31d1be9c59a6e81:e246bec8f4aaf98c75e15a2f3fe1292736a8455e | CHANGES: 2 MAJOR (F1 M1 has no deliverable, F2 `step_card` wrong file/name and T031 in another milestone), 2 MINOR (F3 `pull_request_source.rs` literals, F4 M3 Verify runs no core tests); all four fixed in tasks.md (M2 folded into M1, renumbered M1–M3), research R6, ledger D7; none declined; round 2 on sonnet over the fix diff |

### Review A tasks round 1 verdict (verbatim)

````
# 1123 tasks review A r1 (snapshot 8c136e70:e246bec8)

VERDICT: CHANGES
F1 [MAJOR] specs/1123-overview-card-pr-list/tasks.md:§Milestones M1 (l.271–278) — M1 does not include the P1 story. It draws no row. The US1 scenarios it claims (sc. 3, 4, 6, 8) are all "no pull request row" or "keeps the same rows" cases, which hold already before this feature. So the only change a user can see on `main` is *100+* turning into *10+*, which shows less than today. The 43-vs-44 request figure is not something a user can observe. That breaks milestones.md rule 1 and rule 3: a split must run along an acceptance scenario so that each half has a deliverable.
   Evidence: "Satisfies: US1 sc. 3, 4, 6, 8" and Deliverable "a full page reads *10+* where it read *100+*"; Story 1 sc. 3: "it shows no pull request row and no counts line (feature 813 as today)".
   Fix: Fold M2 into M1 (M1 = T001–T019, the list plus the rows, collapsed and expanded). Renumber M3 and M4 to M2 and M3. Record in ledger D7 why the story stays whole: no scenario split leaves M1 a deliverable. If the diff is too big, split M1 along sc. 1/2/5 instead (rows drawn) so that M1 shows rows.
F2 [MAJOR] specs/1123-overview-card-pr-list/tasks.md:T025, T031 (l.161, l.185) — The arrow step is put in the wrong file under the wrong symbol. `card_grid.rs` has no `step_from`. Arrow stepping is `step_card` in `crates/micold-client/src/ui/overview.rs` (l.432), and it holds one link per card (`cards: Vec<(Id, Id)>`, `is_link`, `from`). `step_from` is only a test helper in that file's `mod tests` (l.1241). The same wrong name is in research R6. As written, no task changes `step_card`. Also, T025 (M3) carries that change while its test, T031, sits in M4, which breaks rule 7 (tests travel with the code).
   Evidence: `grep -n step crates/micold-client/src/ui/material/card_grid.rs` → nothing; `ui/overview.rs:432: pub fn step_card<T…>(forward: bool, cards: &[SessionLocation])`, with `/// Each card's id and its link's id. cards: Vec<(Id, Id)>`.
   Fix: Take the step mapping out of T025. Add to T034 or T035 (M4): "`step_card` in `ui/overview.rs` takes each card's id and all of its link ids (issue link and drawn numbers)". Move T031's test into `ui/overview.rs` `mod tests` next to the existing `step_from` cases. Correct R6's sentence to match.
F3 [MINOR] specs/1123-overview-card-pr-list/tasks.md:T006 (l.81) — Renaming `StatusReading.counts` breaks `crates/micold-core/tests/pull_request_source.rs`, which builds `StatusReading { …, counts: BTreeMap::new() }` at l.54 and l.74. No task names that file.
   Evidence: `crates/micold-core/tests/pull_request_source.rs:54:        counts: BTreeMap::new(),`
   Fix: In T006, add "`lists: BTreeMap::new()` in `pull_request_source.rs`'s `StatusReading` literals".
F4 [MINOR] specs/1123-overview-card-pr-list/tasks.md:§Milestones M3 Verify (l.294) — T023 adds `CardLink` to `crates/micold-core/src/overview_expansion.rs`, but M3's Verify runs no core tests.
   Evidence: M3 Verify lists only `cargo test -p micold-client …` commands.
   Fix: Add `mise run test-core` to M3's Verify and to Phase 5's Independent Test.
````

Fixes: F1 M2 folded into M1 (T001–T019, list, counts and rows, collapsed and expanded); old M3/M4 renumbered M2/M3; Documentation, Organization, Strategy and Notes follow; ledger D7 says why US1 stays whole (no split along sc. 1/2/5: the folded diff is not too big). F2 T025 no longer touches the arrow step; T035 changes `step_card` in `ui/overview.rs` (`cards: Vec<(Id, Vec<Id>)>`, `is_link`/`from` match any link id, the `CardStepped` caller in main.rs passes them) and makes T031 pass; T031 moved to `ui/overview.rs` `mod tests` beside the `step_from` cases (both in M3); research R6 corrected. F3 T006 names the two `StatusReading` literals of `tests/pull_request_source.rs`. F4 `mise run test-core` added to M2's Verify (new numbering) and Phase 5's Independent Test.

### Review A plan round 2 verdict (verbatim)

````
# 1123 plan review A r2 (snapshot 9bc517ce:f3fad1fa)

VERDICT: CLEAN
F1 [MINOR] specs/1123-overview-card-pr-list/contracts/card-rows.md:32 — Section heading still reads "Client — tests U1–U4", but U5–U7 now sit under it (l.43, 49, 53). The plan and quickstart do say U1–U7.
   Evidence: `## Client — tests U1–U4` directly above `- U5 hand-off (main_tests.rs, ...)`
   Fix: Retitle to "Client — tests U1–U7".
F2 [MINOR] specs/1123-overview-card-pr-list/research.md:R9 (contrast sentence) — "each PrMark tint on the card surface" does not say what the gate measures. `Tag` draws the accent as text on its own faint accent tint (`ui/material/tag.rs` `style::chip(accent)`), not on the bare card surface.
   Evidence: R9: "Contrast is covered by composition_contrast.rs for each PrMark tint on the card surface in both themes"
   Fix: Say the gate measures the `PrMark::role` text colour over the `Tag` chip fill, for each of the four marks in both themes.
````

Fixes: F1 contracts/card-rows.md heading now "Client — tests U1–U7". F2 research.md R9: the gate measures the `PrMark::role` text colour over the `Tag` chip fill (`style::chip(accent)`), for each of the four marks in both themes.

### Review A plan round 1 verdict (verbatim)

````
# 1123 plan review A r1 (snapshot 53ffa8f3:a55fb1b0)

VERDICT: CHANGES
F1 [BLOCKER] research.md:R2 (also plan.md Constraints, contracts/reading.md "Points per branch") — The plan works out the 260 → 90 figure with GitHub's node-limit rule (multiply `first` down the tree). Rate-limit points use a different rule: count the requests needed for each connection, then divide by 100. By that rule the new query costs more than today's, which breaks amended FR-008 ("MUST cost GitHub no more rate-limit points per reading than today's reading"). Per branch today: `o` = 1 + 10 (`commits`) + 10 (`contexts`) = 21, `r` = 21, `oc` = 1, `mc` = 1, so 44. The plan's `o` + `r` + `m` (all `...pr`) is 63. A 50-branch request goes from about 22 to about 32 points.
   Evidence: pull_request.rs:189 `commits(last: 1) { … contexts(first: 1) …` sits in FRAGMENT, and the plan's `m<i>` uses `{ nodes { ...pr } }`.
   Fix: have `m<i>` ask only `number url state isDraft createdAt isCrossRepository`, with no `commits`. Merged rows show no status words, so nothing is lost. That makes the cost 21 + 21 + 1 = 43 ≤ 44. Put the request-count arithmetic in R2 and contracts/reading.md, and add a Q-test that `m<i>` has no `commits(`.
F2 [MAJOR] research.md:R6 / data-model.md §5 / card-rows.md K5 — R6 says `just_lost` already treats a Tab from one link to another on the same card as no move. It doesn't. `gained`/`lost` only pair a part with `part.other()` (Card↔Link), and `FocusPart::Both` remembers no link. With `Link(CardLink)`:
   - Loss first: losing `Link(#1)` and then gaining `Link(#2)` doesn't match (`Link(#2).other()` is `Card`). The card collapses and then re-expands, even after Escape (breaks FR-010).
   - Gain first: `Both` and then the loss sets `focus_part = Link(#1).other() = Card`. So `focused_link()` is `None` while #2 has focus, the menu return is lost, and #2's later loss is taken as a move inside the card, so a Focus-expanded card stays expanded after focus leaves.
   Evidence: overview_expansion.rs:322 `if lost.location == location && lost.part == part.other()`; :356–358 `self.focus_part == part.other() … self.focus_part = part.other()`.
   Fix: redesign the focus model in R6/data-model §5. Pair a loss with any gain on the same location (any part other than the lost one). Track the part that holds focus by the last gain, with no `other()` and no `Both`. Make `link_held` hold `(SessionLocation, CardLink)`. Add K5 cases for link→link in both report orders, with and without Escape.
F3 [MAJOR] plan.md Scale/Scope "main.rs" / research.md R6–R7 — The client's focus hand-offs only know about the issue link and are keyed by location. The plan doesn't say how they change, and no test covers them:
   - `FocusReturn { to_link: bool }`, `shown_focused_link`, `LinkFocus.link`, `link_focus_after` and `link_focus_handoff` all check `issue_link_for`.
   - Escape on a focused row-3 number, or a reading that drops the focused pull request, takes away the focused link. Today's hand-off only fires when `issue_link_for` is `None`, so the keyboard is left on a widget that is gone.
   - The menu's focus return goes back to `card_link_id(&target)`, the issue link, not the number that opened it.
   Evidence: main.rs:704 `let link = to_link && target == location && issue_link_for(&app.core, &target).is_some();` and :783 `if issue_link_for(&app.core, &location).is_some() { return None; }`.
   Fix: in R6, change `FocusReturn.to_link` to `Option<CardLink>`, make `shown_focused_link` return `(SessionLocation, CardLink)`, and add a "link shown" query per `CardLink` (a number is shown only when its row is drawn at the card's current form). Hand off to the card when the focused number stops being drawn, and add U-tests for Escape on a hidden-row number and for the menu return to a number.
F4 [MAJOR] research.md:R9 — `StatusChip` does not fit a pull request state pill. Its only constructor takes a Jira `&TicketStatus`, and its accessible text and accent come from Jira categories. The plan doesn't say how it would map pull request states onto it. It also misses the pull request widgets that already exist (`PullRequestRow`, `PrMark`, `PullRequestIndicator`), listing only "a new pill widget" as the alternative.
   Evidence: status_chip.rs:32 `pub fn new(status: &TicketStatus, roles: Roles) -> Self`, and pull_request_row.rs exists ("`#number` … state mark").
   Fix: name the exact construction, for example a new `StatusChip` constructor that takes a word and a role, with its accessible text. Record `PullRequestRow` and `PrMark` as rejected (or reused) alternatives under Principle VIII.
F5 [MINOR] data-model.md §2 Invariants / card-rows.md K3 / reading.md P6 — "the first row is the current pull request for every combination" is not true in two edge cases:
   - Ties on `created_at`: `select_pull_request` uses `max_by`, which keeps the last of equal maxima, while the list breaks ties by higher number.
   - An `r` page full of forks while `m` holds an own merged pull request: `statuses` has no entry but the list has a first row.
   Evidence: pull_request.rs:274 `.max_by(|a, b| a.created_at.cmp(&b.created_at))`.
   Fix: in K3, generate distinct `created_at` values and exclude a fork-filled `r`, or state both edge cases as known and accepted.
F6 [MINOR] research.md:R8 — `PullRequestLinkActivated(String)` puts the address in the message. `IssueLinkActivated(location)` instead works the address out again from current state, so a link that is gone opens nothing. The plan calls this "exactly as `IssueLinkActivated`", which it is not.
   Evidence: main.rs:888–889 `Msg::IssueLinkActivated(location) => { … issue_link_for(&app.core, &location)`.
   Fix: carry `(SessionLocation, u64)` and look the address up in `State.lists` in main.rs, which also keeps the address out of `Msg` `Debug` (FR-014). Otherwise, record why this differs.
F7 [MINOR] research.md:R5, R7, R8, R10 — no "Alternatives considered", which the file's own header requires for each decision.
   Evidence: R5 ends at "**Rationale**: … as `CardModel` already clones strings." and has no Alternatives line.
   Fix: add the rejected alternatives to each, for example R5 storing rows in the `PrDisplay::Counts` variant, and R7 an explicit focusable flag.
````

Fixes: F1 R2 rewritten with GitHub's documented score (sum of connection requests / 100, rounded, min 1): today 44 per branch (o 21, r 21, oc 1, mc 1), now 43 (`m<i>` asks a scalar-only `merged` fragment, no `commits`); Q2 pins it, P7 dedupe keeps the `o`/`r` node. F2 R6 + data-model §5: `FocusPart { Card, Link(CardLink) }` = part of the last gain, no `Both`/`other()`; a loss pairs with the next gain of any other part of the same card; a loss of a part that is not the last-gained one is ignored; `link_held: (SessionLocation, CardLink)`; K5a–K5g (link→link both orders, with and without Escape, 1122 cases kept). F3 R6 client: `card_link_shown`, `shown_focused_link`/`LinkFocus.link` hold `(SessionLocation, CardLink)`, hand-off when the focused link is no longer drawn, `FocusReturn.to_link: Option<CardLink>`, `MenuOrigin::KeyboardFromLink(CardLink)`, `LinkMenuRequested(loc, CardLink)`; U5 (Escape on row-3 number, dropped PR), U6 (menu return to a number). F4 R9: pill = `Tag::new(PrMark::word(), PrMark::role(roles))`, no tooltip; `StatusChip`, `PullRequestRow`, `PullRequestIndicator` glyph, new widget recorded as alternatives. F5 data-model §2 states both edge cases as accepted; K3 generator excludes them, each pinned by an example. F6 R8: `PullRequestLinkActivated(SessionLocation, u64)`, address looked up by `pull_request_link_for`; `PrRow` drops `url`; U7. F7 alternatives added to R5, R7, R8, R10.

### Review A spec round 2 verdict (verbatim)

````
# 1123 spec review A r2 (snapshot 825ca900:91b05eb4)

VERDICT: CLEAN
F1 [MINOR] specs/1123-overview-card-pr-list/spec.md:FR-007 — "or found pull requests cannot be read at all" is garbled. It reads as if pull requests were found and then could not be read, while 040 FR-025 means the reading finds that pull requests cannot be read at all.
   Evidence: "(the reading never succeeded for it, or found pull requests cannot be read at all, feature 040 FR-025)"
   Fix: Reword to "or the reading finds that pull requests cannot be read at all (feature 040 FR-025)", matching the Edge Cases entry "Reading finds pull requests cannot be read at all".
````

### Review A spec round 1 verdict (verbatim)

````
# 1123 spec review A r1 (snapshot afc9cb5d:f1d7d1b5, spec at 451f1d2c)

VERDICT: CHANGES
F1 [MAJOR] specs/1123-overview-card-pr-list/spec.md:72-74 (Story 1, scenario 4) — The scenario says a rate-limited reading always shows the dash and no row. The Rate-limit edge case (l.147-148) says the card keeps its rows if it had them, and so does today's code: `pr_display` returns the counts before it checks `pause_until`. As written, the scenario cannot pass.
   Evidence: crates/micold-client/src/ui/overview.rs:657 `if let Some(counts) = pr.counts.get(&branch) { return PrDisplay::Counts(*counts); }` comes before the `pause_until` match (l.663).
   Fix: Limit scenario 4's rate-limited and not-read-yet cases to "a branch with no earlier successful reading". Add a scenario where a card that had rows keeps them through a rate-limit pause and through a passing failure.
F2 [MINOR] specs/1123-overview-card-pr-list/spec.md:42-44, FR-004 — The spec never mentions feature 1003 (specs/1003-review-worktree-prs). Its Review changes list already shows each pull request's number, state and checks, ordered by created time and then by number, and it calls running checks *pending*. This spec uses *running* for the same thing, so the app would use two different words for one state.
   Evidence: 1003 FR-003 "Check status is one of passing, failing, pending or none, computed and marked the same way as the sidebar's PR checks."
   Fix: Add 1003 to the Input line's builds-on list and to Terms. Either reuse its check words and state words, or state the difference on purpose. Make the word choice part of the FR-004 clarify question.
F3 [MINOR] specs/1123-overview-card-pr-list/spec.md:213-216 (FR-010) — The claim that a focused card is always expanded, so every row's number is reachable by keyboard, is false after Escape. Feature 1121 FR-005 lets Escape collapse a focused card, and 1122 FR-008 says moving focus to the card's own link does not expand it again. Numbers 3 and up are then not drawn, so they are not Tab stops.
   Evidence: 1122 FR-008 "moving focus between a card and its own link MUST NOT re-expand a card collapsed by Escape".
   Fix: Drop the "so every row's number is reachable" clause, or say how the hidden numbers are reached after Escape. Also fix SC-002's "N+1 key presses": when the card has an issue link, the Nth row needs N+2.
F4 [MINOR] specs/1123-overview-card-pr-list/spec.md:151-152 (Edge Cases) and checklists/requirements.md:9,30 — The edge case names a code function (`reduce_checks`). The checklist still ticks "No implementation details", and its note excuses only FR-016.
   Evidence: "read as running, never as passing (feature 040 `reduce_checks`)."
   Fix: Cite feature 040's requirement (FR-008, which defines the combined check result) instead of the function name.
````

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
