# Autopilot ledger — #1793 structured-view-header-one-row

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: /speckit-autopilot #1793 (labels: enhancement, flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1793
- **Worktree branch**: feat/1793_feat-structured-view-header-in-one-row-status-and
- **Started**: 2026-11-10
- **Phase**: milestone M4 (in progress)
- **Next step**: continue M4 from Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1797 | Design | merged | ad2d932dc0b678b50e19e0b3646e8036d62c6e8a |
| #1798 | M1 | merged | 1c675009f14aaf26035b3c593512a76365ad5009 |
| #1827 | M2 | merged | cb41c216e778ce87dcc7379f7852f3ffa85fa3bf |
| #1847 | M3 | merged | 747d325b504576e2bc43d96c2804e9ee430c09d7 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012, T028 | full | Top bar ring, status, Changes (with collapse gate) | #1798 | merged |
| M2 | T013–T016 | full | Worktree menu with raw events and view control | #1827 | merged |
| M3 | T017–T020 | full | Single `+` menu | #1847 | merged |
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
| M3 Review A (high) | 1 | M3 |  | edb22866:18a8d7b5 | CHANGES (1 MAJOR anchor y fixed, 2 MINOR fixed) |
| M3 Review B (sonnet) | 1 | M3 |  |  | CHANGES (2 MAJOR: drift-M3.md written, orphaned switch offer removed; 2 MINOR fixed); fixes are gate-covered, no re-run |
| M3 Review A+B | 1 | M3 | 1341b48328fe557270f4d776fb13371590a30dc0 | 94fab10166996ae7d1b1a78b37d0209300817186:1341b48328fe557270f4d776fb13371590a30dc0 | CLEAN after fixes (A 1 MAJOR, B 2 MAJOR fixed; gate green) |
| M3 Review A (high) | 2 | M3 | c9a0aca2f3183ea98d7930ebe34c1019f5689022 | e7d809e858b3a14d0c9d98208cce436e08d65c72:c9a0aca2f3183ea98d7930ebe34c1019f5689022 | CLEAN (round 2: fix diff since reviewed head 1341b483 is ledger-only, no code change; round 1 findings fixed and covered by the round-1 A+B CLEAN row) |
| M3 Review B (sonnet) | 2 | M3 | c9a0aca2f3183ea98d7930ebe34c1019f5689022 | e7d809e858b3a14d0c9d98208cce436e08d65c72:c9a0aca2f3183ea98d7930ebe34c1019f5689022 | CLEAN (round 2: fix diff since reviewed head 1341b483 is ledger-only, no code change; round 1 findings fixed and covered by the round-1 A+B CLEAN row) |
| M3 Review A (high) | 3 | M3 | 379b3719e2c7a47e8e1176795a9fd244397bccff | rebase:379b3719e2c7a47e8e1176795a9fd244397bccff | CLEAN (rebase onto main; code diff identical, fixture regenerated for #1792) |
| M3 Review B (sonnet) | 3 | M3 | 379b3719e2c7a47e8e1176795a9fd244397bccff | rebase:379b3719e2c7a47e8e1176795a9fd244397bccff | CLEAN (rebase onto main; code diff identical, fixture regenerated for #1792) |
| M3 Review A+B | 3 | M3 | 379b3719e2c7a47e8e1176795a9fd244397bccff | rebase:379b3719e2c7a47e8e1176795a9fd244397bccff | CLEAN (rebase onto main; code diff identical, fixture regenerated for #1792) |

## Declined review findings

None. Plan review: round 1 CHANGES (3 MAJOR, fixed), round 2 (sonnet) CLEAN; snapshot 082794ab:53c2cfac. Open for tasks unit: session-bar leftovers placement (research R6) is a recommended default the spec does not state.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M4 unit 2 hit the 150k cap. Work is in a second local WIP commit on top of b07534df9, NOT pushed, no PR yet. Unit 1's items 5 (user guide) and 6 (drift-M4.md, verify, reviews, full gate, PR) are untouched.
**Done in unit 2** (`cargo check --tests` clean):
- Strip is a 40 px row: `session_area::strip` has `.padding([0, SM]).height(ROW_HEIGHT)`; `+` and the split buttons use `.target(ROW_HEIGHT)` (`panes::split_buttons_in_row`; `split_buttons_disabled` removed).
- `tests/gates/session_area_fits.rs` re-pointed (header tests/consts removed, paths shifted one level up, `strip_height` replaces `header_height` for `composer_fits.rs`, `strip_parts` = tab region + each control child, `CONTROLS=[0,0,1]`); `session_bar_fits.rs` retired (T035) with its `#[path]`; new `tests/gates/header_row_fits.rs` registered (3 tests, pass: strip == ROW_HEIGHT, 2 area children, chip at 0/3/1200 hidden, many tabs). `session_area_stability.rs` header/status_cluster pieces removed. `tests/support/covered_states.rs`: removed all `session.header*` and `session.strip.status` anchors and consts, shifted `[0,0,1,1,1..]`->`[0,0,1,1,0..]` and `[0,0,1,1,2..]`->`[0,0,1,1,1..]`.
**Left (layout_snapshot has 11 failures; `main` binary never ran: the lock was held by another worktree; run `scripts/build-lock.sh cargo test -p micold-client --test main --no-fail-fast` detached):**
1. `Member::Plus` width: `PLUS_WIDTH` in `features/session_strip.rs` must become `ROW_HEIGHT` (40) (`strip_controls_hold_their_size` says Plus laid out 40 vs fixed 48); check `member_offsets` tests in `tests/features_session_strip.rs`.
2. `SESSION_STRIP_SPLIT` const is `[0,0,1,1,0,0,1,1]` (assumes restart is controls child 0); the controls group is now `[restart?, splits, chip?]`, so anchor per state (`...,1,0]` without restart, `...,1,1]` with). `an_anchor_whose_path_does_not_resolve` names `session-area-review-shown`; also `review_tab_fits.rs:414` still expects the `session.header` anchor (remove from its list ~line 42).
3. Gates still using the removed status cluster: `strip_controls_share_a_midline.rs` (line 33 status anchor), `strip_controls_anchor_the_trailing_edge.rs` (status + splits last two children -> splits (or the chip) are the last child, flush with the strip's padding), `strip_controls_hold_their_size.rs`.
4. `containment.rs:128` `TAB_STRIP_CONTENT = ["0/0/0/1/1/1/0/0/0/0/0/0"]` -> `"0/0/0/1/1/0/0/0/0/0/0/0"` (recorded overflow is now that node; `review_tab_fits` escapes by 364 px at that path).
5. `session_area_fits::the_progress_bar_has_its_width_beside_its_words`: `VIEW_HEADER` is relative to SESSION_AREA but the test also uses `header=[SESSION_AREA, VIEW_HEADER]` with `part(&[2])` etc. (path `[1,0,0,1]` not laid out: `PROGRESS_BAR=[1,0,0,1]` is absolute from SESSION_AREA yet the test joins it to `header`; it was `[2,0,0,1]` = absolute; use `[SESSION_AREA, PROGRESS_BAR]` directly, and slot `part(&[2])` relative to VIEW_HEADER is correct). Also remove unused imports (`CheckStatus`, `PrState`, `PullRequestStatus`, `ReviewState`) and unused `assert_fits` in `session_area_fits.rs`.
6. Regenerate `tests/fixtures/layout_snapshot.txt` (`UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`) and review its diff (expect strip 40 px, no header row).
7. Run the `main` binary and fix (`terminal_tabs.rs` source-text tests, `transcript_*`); FR-011 "tabs shrink": tabs are fixed-width, only scroll exists; record as follow-up (or implement).
8. `plus_menu` anchor: in `ui/session_area.rs::plus_menu` replace the estimate (`BOTTOM_EDGE + 2*(48+2*SM)`) with `BOTTOM_EDGE + ROW_HEIGHT` and clamp x to `[0, tab_strip_viewport_width]` when `+` is scrolled out; add a test (context_menu_anchor gate or popover test); confirm worktree-menu keyboard highlight (M2 minor).
9. User guide T027/T031: `docs/user-guide/structured-transcript.md`, `docs/user-guide/worktrees-and-sessions.md` (~1499-1600 still describes the session bar meter/totals/toggle); remove dead `shown::hidden_text`/`SHOW_HIDDEN`/`SHOW_RAW_EVENTS*` if unused.
10. `drift-M4.md`; tick T021-T027, T029, T031, T034, T035 in tasks.md; scoped gate, Review A (high) + B, full gate (`mise run gate`), push, PR (title `feat(1793): one-row header (#1793)`, `Refs #1793`). A part 3 is the last allowed.

## Open escalation

RESOLVED (human, option 1: slim in-body row; spec US1 AC1, FR-001, SC-001 amended with a clarification entry). Was: Tasks review round 1 (snapshot a2463a39:901c3164): CHANGES, 4 MAJOR. F2-F4 fixed in tasks.md (collapse gate moved to M1, M5 dropped, gate registration in layout_snapshot.rs, test-first and gate retirement for R6 relocations). F1 open: research R6 default (conditional slim row in the transcript body for the tasks accordion and mode label) contradicts spec US1 AC1 / FR-001 / SC-001 "exactly one header row precedes the event list" whenever background tasks or a mode exist. Needs a decision; tasks T026/T034 are written to the default and change with the answer.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
