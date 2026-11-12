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

M3 (T017-T020) code and docs are written but UNVERIFIED and tasks.md T017-T020 are NOT yet ticked. Branch was reset to origin/main (M2 merged as #1827, cb41c216e); work is uncommitted-then-WIP-committed on top.
Done: `PlusMenuModel`/`PlusItem`/consts in `features/session_strip.rs` (one `Member::Plus` pushed after all tabs, press = `HeaderBar(Toggled(Plus))`); `PlusMenu` surface + registry entry with mutual `displaces` against `WorktreeMenu` (`header_bar.rs`, `overlay/registry.rs`, `tests/popover_displacement.rs`); `app.rs` closes the Plus menu when `AdditionalRequested`/`ShellInstanceOpenRequested` runs; `ui/session_area.rs`: `plus_menu()` overlay (hung_from computed anchor), `new_session` icon, `switch_slot` and the cluster's `switch` removed; `ui/mod.rs` pushes `header_plus_menu` LAST (so layout gate paths that address dialogs by index do not shift); tests added in `tests/features_session_strip.rs`; `feature_registration_cost.rs` ALLOWED row session_strip->header_bar; `session_area_stability.rs` switch_slot entry dropped; `gates/session_area_fits.rs` switch-slot tests removed/rewritten (cluster is now words + restart); user guide edited (terminal-panes.md, worktrees-and-sessions.md, structured-transcript.md).
Next: (1) `scripts/build-lock.sh cargo test -p micold-client --no-fail-fast --test main --test popover_displacement --test feature_registration_cost --test layout_snapshot` (builds wait on other worktrees' lock; run detached, poll <170 s loops) and fix reds: expect strip gates (`strip_controls_anchor_the_trailing_edge::new_session_and_the_splits...`, `strip_controls_hold_their_size`, `strip_controls_share_a_midline`, tab_children_fit, `session_area_fits` incl. `a_long_title...`) to need the removed New-session control dropped/repointed, and `gates/*` paths for `Member::Plus`; then regenerate the fixture `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot` and review its diff. (2) tick T017-T020 in tasks.md, drift note if any. (3) Review A (high) + B (sonnet), full `mise run gate`, push, PR `feat(1793): add the single + menu (#1793)`, body per tasks/pr.md, `Refs #1793`. Visual pass not run (no Xvfb). The `+` anchor is approximate (computed from sidebar width and strip offsets), confirm in review.

## Open escalation

RESOLVED (human, option 1: slim in-body row; spec US1 AC1, FR-001, SC-001 amended with a clarification entry). Was: Tasks review round 1 (snapshot a2463a39:901c3164): CHANGES, 4 MAJOR. F2-F4 fixed in tasks.md (collapse gate moved to M1, M5 dropped, gate registration in layout_snapshot.rs, test-first and gate retirement for R6 relocations). F1 open: research R6 default (conditional slim row in the transcript body for the tasks accordion and mode label) contradicts spec US1 AC1 / FR-001 / SC-001 "exactly one header row precedes the event list" whenever background tasks or a mode exist. Needs a decision; tasks T026/T034 are written to the default and change with the answer.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
