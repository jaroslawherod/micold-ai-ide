# Autopilot ledger — #1793 structured-view-header-one-row

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: /speckit-autopilot #1793 (labels: enhancement, flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1793
- **Worktree branch**: feat/1793_feat-structured-view-header-in-one-row-status-and
- **Started**: 2026-11-10
- **Phase**: milestone M3 (in progress)
- **Next step**: M3 verify gates, review, PR (see Handover)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1797 | Design | merged | ad2d932dc0b678b50e19e0b3646e8036d62c6e8a |
| #1798 | M1 | merged | 1c675009f14aaf26035b3c593512a76365ad5009 |
| #1827 | M2 | merged | cb41c216e778ce87dcc7379f7852f3ffa85fa3bf |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012, T028 | full | Top bar ring, status, Changes (with collapse gate) | #1798 | merged |
| M2 | T013–T016 | full | Worktree menu with raw events and view control | #1827 | merged |
| M3 | T017–T020 | full | Single `+` menu | | pending |
| M4 | T021–T027, T029, T031, T034, T035 | full | One-row header | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Tasks | 1 |  |  | a2463a39:901c3164 | CHANGES (4 MAJOR, fixed) |
| Tasks | 2 (sonnet) |  |  | fix diff | CLEAN (2 MINOR, fixed) |
| Spec | 1 |  |  | bb128d1c688f3a42eaea356845c2b2497ccfa4d1:c63bfc7c625e9cac5e814565ff80e2d20296bb2c | CLEAN (3 MINOR, fixed prose only) |
| M1 Review A (high) | 1 |  |  | 5ff40e78:ca940bff | CLEAN (2 MINOR: changes_button near-copy and floor constant, left) |
| M1 Review B (sonnet) | 1 |  |  | 14aea717:34cfb594 | CLEAN |
| M2 Review A (high) | 1 |  |  | 1b71d9df:4b30f70c | CLEAN (1 MINOR: Chosen doc, left) |
| M2 Review A+B (sonnet) | 2 | M2 | bc9262d05a01f8146f898a15d2a0896dc2e19b29 | d28d938bd831dae098ca3fd7dcb6453d3054c43f:bc9262d05a01f8146f898a15d2a0896dc2e19b29 | CLEAN (2 MINOR: worktree menu keyboard highlight to confirm in M4; no tdd dir, left) |

## Declined review findings

None. Plan review: round 1 CHANGES (3 MAJOR, fixed), round 2 (sonnet) CLEAN; snapshot 082794ab:53c2cfac. Open for tasks unit: session-bar leftovers placement (research R6) is a recommended default the spec does not state.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M3 (T017-T020) code is written; targeted tests mostly fixed; tasks.md T017-T020 NOT yet ticked; no review, no full gate, no push, no PR yet. The branch is not rebased on current origin/main (72ea93fee, 8 commits ahead of M2 merge): run `git fetch && git rebase origin/main` before the full gate.
Done this unit (uncommitted, WIP-commit them): `PlusMenu::ID`/`WorktreeMenu::ID` consts (registry macro needs them); dialog layer paths re-pointed 10 -> 12 (covered_states.rs, layout_snapshot.rs, gates/containment.rs strings) because the closed Plus menu adds two overlay nodes; strip anchors: `SESSION_STRIP_NEW_SESSION` replaced by `SESSION_STRIP_STATUS` ([..,1,0]), `SESSION_STRIP_SPLIT` = [..,1,1], gates strip_controls_* and `session_area_fits.rs` (strip is three children, one touch-checked control) rewritten; session_area_stability restart path [1,0,1]; tests terminal_tabs (Plus press = HeaderBar Toggled), terminal_focus (Plus filtered, "+ menu: Terminal" press added), features_session_strip (Dismissed first, so `follow` records the selection); text-overflow: KNOWN_OVERFLOWS entry for the scroll-cut tab in `session-area-six-tabs` (layout_text_overflow.rs); layout_snapshot.txt regenerated (UPDATE_LAYOUT_SNAPSHOT=1), not yet reviewed beyond the added 0/9,0/10 closed-menu nodes.
Still red (last run /scratchpad t5.log):
1. `session::update` now pushes `Outcome::OverlayDismissed(header_bar::PlusMenu::ID)` to close the menu after `AdditionalRequested`/`ShellInstanceOpenRequested`: `feature_registration_cost` (session names header_bar) and `surface_registration_cost` (PlusMenu named outside its module) reject it. (A direct `state.header_bar` write in session.rs was rejected by `feature_write_isolation`; a root-arm write by `root_is_routing_only`.) Pick another route: e.g. header_bar feature closes the menu itself (extend `State::follow` key with the active session's shell count plus whether the confirm-additional dialog is open), or give header_bar a generic `Outcome` consumer; then drop the `OverlayDismissed` push and `update_inner` wrapper in `features/session.rs`. Test `features_session_strip::choosing_an_entry_closes_the_plus_menu` states the contract.
2. `gates/review_tab_fits.rs` `laid_out` filter (drops overflows under the strip tab row) did not take: node_path is `0/0/0/1/1/1/0/0/0/0/0/0/2/0/0/0/0/1/0`, my prefix from `path_token(&[&[0], SESSION_STRIP_TABS].concat())` has probably a different format (check `path_token`, maybe no leading 0 means the root already included): print it and fix; or the filter is not on the code path of `findings` (line ~281 and 431 use `laid_out`, so check the prefix).
3. `terminal_tabs::each_strip_member_sends_its_s5_message`: the Plus member is now LAST (after the other session): move the `(MemberKey::Plus, HeaderBar Toggled(Plus))` tuple after `MemberKey::Session(other)`.
4. Remaining unchecked after these: `feature_write_isolation`, `layout_text_overflow`, whole `layout_snapshot` and `main` re-run (3375/3378 passed in t4 before the session.rs change), `popover_displacement` ok, `mise run check` clippy on the `+` anchor code.
Then: tick T017-T020 in tasks.md, review A (high) + B (sonnet), rebase, full `mise run gate`, push, PR `feat(1793): add the single + menu (#1793)` per tasks/pr.md, `Refs #1793`. Visual pass not run (no Xvfb). The `+` anchor is approximate (sidebar width and strip offsets), confirm in review. Build lock waits are long (other worktrees): start tests detached with `scripts/build-lock.sh cargo test -p micold-client --no-fail-fast --test main --test layout_snapshot ...` (`features_session_strip` is a module of `--test main`, not its own target).

## Open escalation

RESOLVED (human, option 1: slim in-body row; spec US1 AC1, FR-001, SC-001 amended with a clarification entry). Was: Tasks review round 1 (snapshot a2463a39:901c3164): CHANGES, 4 MAJOR. F2-F4 fixed in tasks.md (collapse gate moved to M1, M5 dropped, gate registration in layout_snapshot.rs, test-first and gate retirement for R6 relocations). F1 open: research R6 default (conditional slim row in the transcript body for the tasks accordion and mode label) contradicts spec US1 AC1 / FR-001 / SC-001 "exactly one header row precedes the event list" whenever background tasks or a mode exist. Needs a decision; tasks T026/T034 are written to the default and change with the answer.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
