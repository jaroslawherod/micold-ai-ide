# Autopilot ledger — #1123 overview-card-pr-list

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #1123: Overview cards: pull request list with state, linked. Blocked by: #1121 (expanded view for the rest of the list). The app reads PR counts only. Read, for each PR of a branch: number, URL, state (open, draft, merged, closed), checks (passing, failing, running, none), review (approved, changes requested) and merge conflict. Reuse the existing PR status reading and its rate-limit handling; no new polling. The card shows up to two PR rows (number as a link, state pill, checks or review in words), the rest on expand. A press on the number opens the PR and does not activate the card. A worktree with no PR shows no row. Acceptance: core type and parser tests; unreadable states keep today's dash reasons; counts derive from the list; docs and layout snapshot updated. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #1123
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-21
- **Phase**: plan
- **Next step**: plan review A round 1 (orchestrator dispatches; prompt 1123-prompt-plan-r1.md in the scratchpad), then tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Base | origin/main f1d7d1b5 (#1121, #1122 merged) | agent-resolved | orchestrator prompt |
| D2 | spec | Check and state words (review A r1 F2) | Reuse feature 1003's words: *pending*, not *running*; states open, draft, merged, closed. Row order keeps open/draft first on purpose. Word choice also named in the FR-004 clarify question | orchestrator default | 1003 FR-001, FR-003; 040 FR-008; no clash found |
| D3 | clarify | FR-004: one status phrase or each? | One phrase per row by precedence: checks failing, changes requested, merge conflict, checks pending, approved, checks passing (first three in 813's attention order); 1003's words | orchestrator default, owner card pending | 813 card_state order (crates/micold-core/src/overview.rs ~l.328); 1003 FR-003 |
| D4 | clarify | FR-008: how many PRs per branch? | Newest 10 of any state + newest 10 open/draft (today's `o`/`r` pages), no more GitHub points than today; a count of 10 or more reads *10+* (replaces 813's *100+* on the card) | orchestrator default, owner card pending | pull_request.rs status_query (NEWEST_TEN, COUNT_PAGE) |
| D5 | clarify | Second clarify round | No critical ambiguities after D3/D4 (they only fix wording and a cap); clarify CLEAN | agent-resolved | spec.md scan |
| D6 | plan | How counts derive from a 10-cap list at no more cost | A newest-10 merged page `m<i>` replaces the 100-node `oc`/`mc` count pages (260 → 90 points per branch); spec FR-008, FR-006 (fork *N+*) and Key Entities (title not kept) amended with the plan | agent-resolved | research.md R1, R2 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A spec | 1 | afc9cb5d94279a3c0ae00d37d80855a145ded446:f1d7d1b5b3251cbb108318452ccb6a4ede3d1f3e | CHANGES: 1 MAJOR (F1 Story 1 sc 4 dash vs kept rows), 3 MINOR (F2 feature 1003 words, F3 FR-010/SC-002 after Escape, F4 function name in Edge Cases); all four fixed in spec.md, none declined |
| Review A spec | 2 | 825ca900990f4c8ff4f75b661ae433152e67cae6:91b05eb41e1d852e023be0f044cc857a68bc5800 (fix diff 451f1d2c..91b05eb4, sonnet) | CLEAN, 1 MINOR (F1 FR-007 FR-025 wording); fixed, prose only, no round 3 |
| Review A plan | 1 | 53ffa8f39a3a4caa715af2e64d4fca74c90e6a87:a55fb1b02c4bca128a2f493349a45dbf294074aa | pending |

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
