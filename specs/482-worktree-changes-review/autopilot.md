# Autopilot ledger — #482 worktree-changes-review

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #482 "Review a worktree's changes with inline comments and send them to the session" (scope verbatim from the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #482
- **Worktree branch**: claude/project-thread-v1va8z
- **Started**: 2026-10-06
- **Phase**: implement
- **Next step**: milestone M1 (T001–T026)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #624 | whole branch (design + every milestone), opened by the orchestrator | draft | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T026 | full | Changes view with the changed-file list (MVP) | #624 | in progress |
| M2 | T027–T040 | full | Unified diff of the selected file | — | todo |
| M3 | T041–T049 | full | Side-by-side layout, syntax colouring, kept layout | — | todo |
| M4 | T050–T063 | full | Comments on lines and ranges | — | todo |
| M5 | T064–T075 | full | Send comments to the running session | — | todo |
| M6 | T076–T080 | full | Send when no session is running | — | todo |
| M7 | T081–T089 | full | Live refresh and outdated comments | — | todo |
| M8 | T090–T097 | full | Clear, discard, and removal with the worktree | — | todo |

## Decisions

- Clarify round 1: US3 scenario 4 and FR-003 answered by the orchestrator as defaults (user away); recorded in spec.md Clarifications. No further ambiguities, no second round.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

- Tasks: US1 split in three (list / unified diff / layouts+colour), US2 in two (comments / send), US4 in two (refresh+outdated / tidy), along acceptance scenarios. M1 stays at 26 tasks because Setup + Foundational (core value types, protocol v30 in one edit — a second bump would fail schema_hash.rs) must ride with it. Polish T098–T100 changes no code: close unit.
- Gate (tasks unit): specs-only, `scripts/check-criteria-observables.sh` passed (spec has no criteria table). Design PR: none of its own — this run ships one PR for the whole branch (orchestrator opens it); PR body written to the session scratchpad pr-body-482.md.
- speckit-analyze (0 CRITICAL, 2 HIGH, 6 MEDIUM, 5 LOW): fixed in tasks.md (D1 watch::Debouncer + cap_spans as tested pure functions, D2 two-session test moved to T066, D3–D7, D12); spec.md edits (D8 no-bracketed-paste edge, D3 worktree removed outside the app, D10 FR-016 wording, D13 toggles reset on open) and plan.md Constitution row I (D1). D9, D11 left: wording only.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c5e15968c86ce0c17544cde68a31ff74e2fb1e8b:107ab7ed3aa9b067a4dca5c02aca6e6709d4d158 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | c960571fb7bed1fa440f89fe3b9720fd30c8d897:c74e2d4a9e09412fc66dd4915e3c7e105b785552 | CLEAN: 2 MINOR (both fixed) |
| Plan | 1 | 76b23838a33fc28d5c9f6db389cc15f46acd39a1:3a5ac98c80a82e8c74fb02606f5bee99309eaeeb | CLEAN: 3 MINOR (all fixed, prose only) |
| Tasks | 1 | bdc7eb7285b7355f190266e00dd32362649c017c:a284e9b2ea58bc2c731608a1600a913ad9b91dec | CLEAN: 3 MINOR (all fixed: checklist ticked, T067 outdated list deferred to M7, M1 Verify + SC-001/SC-005 mapped) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 3 handed over at 150k. Pushed nothing (no green gate yet).
- **Done, ticked:** T001–T013, T016–T018 (commit `2606df4e`: postcard fix — `ReviewComment`
  serialises via a private `StoredComment` repr, no `flatten`; T009 `diff_layout_setting` 3 passed,
  test-after; cycle-log rows added).
- **T014/T015 red written (uncommitted → in the handover commit):** `crates/micold-client/src/features/changes.rs`
  (types, `Msg`, `Effect::ReadList { seq, entry, toggles }`, `Load::Loading { seq, again, last }`,
  `pending: Option<Effect>`, view-model `list_body`/`base_line`/`committed_available`/`shown_list`,
  consts `DEFAULT_ENTRY_NOTE`, `BOTH_HIDDEN`) with **stub bodies**; `tests/features_changes.rs`
  (14 tests); `ui/material/virtual_rows.rs` (`visible_range`/`spacers` stubbed, 6 tests;
  registered in `material/mod.rs`; `features/mod.rs` declares `changes`). Red recorded in cycle-log (13/13 and 4/6 failing on the stubs).
- **Next:** implement T019 (`visible_range`: start = offset/row_h − overscan, end = ceil((offset+viewport)/row_h) + overscan, clamp; offset past end → tail of `viewport/row_h + overscan` rows — see tests) and T020 (reducer per the tests; base line `Compared with <branch> at <7-char sha>`, NoCommonHistory text must contain "no history in common" and "only uncommitted changes"; empty "No changes against <branch>"; both off → `BOTH_HIDDEN`).
  Wiring plan (keeps `tests/feature_registration_cost.rs` rules: a feature names no other feature; only `app.rs`/features call reducers; shape B = `pub fn update(&mut App, Msg) -> Task<Message>` in `src/shell/changes.rs`):
  `Message::Changes(changes::Msg)` declined by `State::update` like `PrStatus`; `State.changes` field; `State::update_changes(msg) -> Effect` and `State::take_changes_effect()`; main.rs routes `Message::Changes` to `shell::changes::update`.
  Sidebar `Msg::ReviewChangesRequested(SessionLocation)` → `Outcome::ChangesRequested(loc)` (new in `features/mod.rs`); `app::interpret` opens the view and stores the read in `state.changes.pending`; main.rs intercepts `Message::Sidebar(msg @ SidebarMsg::ReviewChangesRequested(_))` (as it does `ShowAgentWorktreesToggled`) then runs the pending read.
  V2: `Message::Session(SessionMsg::Selected(_))` arm in app.rs also sends `Msg::SessionSelected`; V3: `Outcome::WorktreesReplaced(names)` in `interpret` also sends `Msg::WorktreesListed(names)`.
  Default row has no menu: reuse `WorktreeMenu` with `dir_name == ""` for the Default row (the wire's convention), `on_right_press` on the Default row in `ui/sidebar.rs`, `worktree_menu_items` gives only "Review changes" for `""`.
  Toggles: use `ToggleChip` (`.active().disabled()`), not `LabelledToggle` (it has no on/off state) — record as a Decision.
  `tests/settings_sections.rs` `DEFERRED` likely needs `("diff_layout", "482 T041")`.
  Then T021 shell read (`GitCli.review_base` + `change_list` in `spawn_blocking`), T022–T024, docs T025, visual T026, then verify.md (gate: raw mise.toml `gate` commands; 6 root-only permission tests fail locally).

## Open escalation

None.

## Follow-ups not done

None yet.
