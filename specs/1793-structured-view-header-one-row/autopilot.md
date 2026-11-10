# Autopilot ledger — #1793 structured-view-header-one-row

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: /speckit-autopilot #1793 (labels: enhancement, flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1793
- **Worktree branch**: feat/1793_feat-structured-view-header-in-one-row-status-and
- **Started**: 2026-11-10
- **Phase**: milestone M1 (in progress)
- **Next step**: continue M1 from Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1797 | Design | merged | ad2d932dc0b678b50e19e0b3646e8036d62c6e8a |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012, T028 | full | Top bar ring, status, Changes (with collapse gate) | | pending |
| M2 | T013–T016 | full | Worktree menu with raw events and view control | | pending |
| M3 | T017–T020 | full | Single `+` menu | | pending |
| M4 | T021–T027, T029, T031, T034, T035 | full | One-row header | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks | 1 | a2463a39:901c3164 | CHANGES (4 MAJOR, fixed) |
| Tasks | 2 (sonnet) | fix diff | CLEAN (2 MINOR, fixed) |
| Spec | 1 | bb128d1c688f3a42eaea356845c2b2497ccfa4d1:c63bfc7c625e9cac5e814565ff80e2d20296bb2c | CLEAN (3 MINOR, fixed prose only) |
| M1 Review A (high) | 1 | 5ff40e78:ca940bff | CLEAN (2 MINOR: changes_button near-copy and floor constant, left) |
| M1 Review B (sonnet) | 1 | 14aea717:34cfb594 | CLEAN |

## Declined review findings

None. Plan review: round 1 CHANGES (3 MAJOR, fixed), round 2 (sonnet) CLEAN; snapshot 082794ab:53c2cfac. Open for tasks unit: session-bar leftovers placement (research R6) is a recommended default the spec does not state.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 code, tests, gates, showcase, user guide done and committed locally (branch NOT pushed, no PR yet). Review A (high) CLEAN, review B (sonnet) CLEAN (see Review rounds). Layout snapshot fixture accepted; sibling_parity exempts the ring/status readouts; arch guards classified (state_scan MUTATORS `follow`, NO_OWNER `HeaderBar`, ALLOWED_CROSS_FEATURE_NAMES header_bar->session_strip).
Last full `mise run gate` was running (previous run's 4 failures were fixed by the last two commits; fmt/clippy of the last fix `segments` made `#[cfg(test)] pub(crate)` unverified). Next: (1) run `mise run gate` detached (hold.sh), fix anything red (watch for dead-code warnings on `Segment::label/is_selected` in micold-material); (2) visual pass could NOT run (no Xvfb/xdotool, no seeded fixture; worker said sway+grim route would need a seeded private XDG_DATA_HOME) -- say so in the PR body or retry with the sway route; (3) commit, `git push --force-with-lease -u origin HEAD`, open the PR per tasks/pr.md (title `feat(1793): show status, Changes and context meter in the top bar (#1793)`, body Milestone M1 of 4, Refs #1793), record PR number in the ledger. Declined/left MINORs of A: changes_button near-copy of session_area::changes_button, CHANGES_ICON_FLOOR magic 70.0.

## Open escalation

RESOLVED (human, option 1: slim in-body row; spec US1 AC1, FR-001, SC-001 amended with a clarification entry). Was: Tasks review round 1 (snapshot a2463a39:901c3164): CHANGES, 4 MAJOR. F2-F4 fixed in tasks.md (collapse gate moved to M1, M5 dropped, gate registration in layout_snapshot.rs, test-first and gate retirement for R6 relocations). F1 open: research R6 default (conditional slim row in the transcript body for the tasks accordion and mode label) contradicts spec US1 AC1 / FR-001 / SC-001 "exactly one header row precedes the event list" whenever background tasks or a mode exist. Needs a decision; tasks T026/T034 are written to the default and change with the answer.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
