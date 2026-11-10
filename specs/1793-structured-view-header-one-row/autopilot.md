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

## Declined review findings

None. Plan review: round 1 CHANGES (3 MAJOR, fixed), round 2 (sonnet) CLEAN; snapshot 082794ab:53c2cfac. Open for tasks unit: session-bar leftovers placement (research R6) is a recommended default the spec does not state.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 in progress by a unit that hit the context cap; branch has one local WIP commit, NOT pushed, no PR yet (#1797 merged at ad2d932dc).
Done (uncommitted-or-WIP code, compiles, `cargo test -p micold-material --lib -- context_ring segmented give_way toolbar` green): 
- `crates/micold-material/src/context_ring.rs` (ContextRing, T001+T004 minus showcase entry), `segmented.rs` (SegmentedControl, T002+T005 minus showcase entry), exported in lib.rs.
- `GiveWayRow::hide(rank, child)` added in give_way.rs (Drop slot; hugging lanes now drop only zero-width AND zero-height nodes). `Toolbar::action_giving_way(rank, floor, el)` added in toolbar.rs (trailing action that shortens at `rank` down to `floor` dp; centre rank now 2, leading 3, so ring text (rank 0) then Changes label (rank 1) go first). Reason: trailing actions were `kept`, so R2's "GiveWayRow in Toolbar::action" could not collapse.
- `crates/micold-client/src/features/header_bar.rs` (HeaderBarModel, MeterText, Status, rank consts RING_TEXT_RANK=0 / CHANGES_LABEL_RANK=1, HeaderMenu, Msg, State with `update` and `follow`), registered in features/mod.rs; `session_working` added in session_strip.rs (beside session_status; the task text says structured_insights.rs but session_status lives in session_strip.rs); `Message::HeaderBar(Msg)` + `State.header_bar` wired in app.rs (update arm only).
Left (next unit):
1. Tests first/after: T003 + T007 in `crates/micold-client/tests/features_header_bar.rs` (None on Overview, order, MeterText text/tooltip/accessible, empty state, Status mapping incl. failed/exited -> Idle, two-session no-leak, rank consts, HeaderMenu reducer incl. `follow`). Wire `State::follow(selection)` to run after every `State::update` (selection = shown_location + active session) so a selection change closes the menu; not done yet.
2. Showcase entries + catalogue.rs entries for ContextRing and SegmentedControl (copy CountBadge's pattern in showcase/catalogue.rs and showcase/sections/atoms.rs).
3. T009/T028 gates `tests/gates/top_bar_items.rs`, `top_bar_collapse.rs` registered by `#[path]` in tests/layout_snapshot.rs (state: `title_bar_crowded()` or `transcript_pane_insights()` from tests/support/covered_states.rs; pattern status_bar_fits.rs / title_bar_fits.rs).
4. T010/T011: in `ui/toolbar.rs::view`, when `HeaderBarModel::of(state)` is Some, add before settings: ring cluster via `.action_giving_way(RING_TEXT_RANK.., SIDE, ContextRing...)` (ring text rank 0 inside is the GiveWayRow hide; outer rank 0), status Tag (Idle: secondary_container/on_secondary_container; Working: primary/on_primary), Changes button (`ui::session_area::changes_press(state,&location)` + `Button::with_content(GiveWayRow().hide(0,label)).leading(Icon::Git)`, outer `.action_giving_way(CHANGES_LABEL_RANK, ICON_TARGET, ..)`). Existing `tests/gates/title_bar_fits.rs` asserts 5 bar children and Settings = children[3]: with a session shown the bar now has 3 more (ring, status, Changes), update that gate (8 children; trailing group starts at the ring). Check other gates (window_regions_fit, etc.) with the scoped gate.
5. T012 user guide `docs/user-guide/structured-transcript.md`. Then scoped gate, review A (high), review B (conformance, sonnet), visual pass, full gate, PR per tasks/pr.md. Old header controls stay untouched in M1.
Build tip: a `setsid nohup ... &` + `scripts/autopilot/hold.sh <log> '^EXIT='` per run; the build lock is often held by another worktree.

## Open escalation

RESOLVED (human, option 1: slim in-body row; spec US1 AC1, FR-001, SC-001 amended with a clarification entry). Was: Tasks review round 1 (snapshot a2463a39:901c3164): CHANGES, 4 MAJOR. F2-F4 fixed in tasks.md (collapse gate moved to M1, M5 dropped, gate registration in layout_snapshot.rs, test-first and gate retirement for R6 relocations). F1 open: research R6 default (conditional slim row in the transcript body for the tasks accordion and mode label) contradicts spec US1 AC1 / FR-001 / SC-001 "exactly one header row precedes the event list" whenever background tasks or a mode exist. Needs a decision; tasks T026/T034 are written to the default and change with the answer.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
