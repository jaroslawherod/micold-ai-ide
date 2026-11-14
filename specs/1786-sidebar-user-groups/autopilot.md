# Autopilot ledger — #1786 sidebar-user-groups

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1786 "feat: sidebar: let the user create groups"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1786
- **Worktree branch**: claude/project-thread-krubw0
- **Started**: 2026-11-11
- **Phase**: implement
- **Next step**: milestone M1 continues from *Handover* (T008, T011–T017, T044, then verify and PR)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1848 | Design | merged 2026-11-12 | 553474089e559f95557125168150c3a3abfdcc95 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017, T044 | full | Layout core, store, create a group | #… | in progress |
| M2 | T018–T023 | full | Rename and delete a group | #… | pending |
| M3 | T024–T028 | full | Move to group from the menu | #… | pending |
| M4 | T029–T034 | full | Drag between lists and cancel | #… | pending |
| M5 | T035–T042 | full | Reorder, collapse, no state grouping | #… | pending |
| M6 | T043, T045 | full | Showcase poses and visual pass | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |
| D2 | implement M1 | Where is the layout applied? | In `State::sidebar_list` (the drawn list), not `sidebar_entries`: the Overview's cards walk `sidebar_entries` and must not lose a collapsed group's cards | agent-resolved | crates/micold-client/src/ui/overview.rs `cards` |
| D3 | implement M1 | How are ids never reused across restarts? | A stored `next_id` counter in the layout file | agent-resolved | data-model.md § UserGroup `id` |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 2 | spec.md | 58e1734ce000a84d1fef7957a5f76047a693cc51 | 15197570fe64dd3fe6ce468637233ff476ad4e90:58e1734ce000a84d1fef7957a5f76047a693cc51 | CLEAN |
| Plan | 2 | design | 076a350f4e2bebc369741dda330ff1265ccea2cb | fe475105b6c85398bc85de75395b5c8b0f586243:8c6d9c8dfc934231e7defc72aab8e90441e8fa1b | CLEAN |
| Tasks | 1 | tasks.md, milestones | fd659fa9 | fd659fa948c3b865c9e5891240b2f31309838abb:d9fddb2b92550718f4a637e8d771d0cbf244709d | CHANGES (1 MAJOR fixed: gate wiring and snapshot moved to M1) |
| Tasks | 2 | fix diff | 1fc59a5f | 70700fb0bd62ac2308bdaa57a5598ba424ce7882:1fc59a5f9b962824a590f71b1c41069072f626c6 | CLEAN (1 MINOR fixed, prose only) |
| Plan | 1 | design | 33647c442736a049f12d6cc8752134e390fc02f0 | 9c4e947e275c19ed0709a25e96d98d7e43160c6c:c0341c2bd14d57f5c0d8119083fd10ec6f2fb7ac | CHANGES (3 MAJOR fixed) |
| Tasks | 2 | fix diff | 7ad2a0361951595eea2d898d6746b37a1ddb0036 | 70700fb0bd62ac2308bdaa57a5598ba424ce7882:1fc59a5f9b962824a590f71b1c41069072f626c6 | CLEAN |
| Plan | 3 | design | 5d3885de780da42ac43772e0841f49a11dce4d3c | e984a4491e90ea566d4e00fe11bf0e87b4597b0f:5d3885de780da42ac43772e0841f49a11dce4d3c | CHANGES (1 MAJOR fixed: rename_dialog.rs line restored in plan.md) |
| Plan | 4 | design | 5d3885de780da42ac43772e0841f49a11dce4d3c | e984a4491e90ea566d4e00fe11bf0e87b4597b0f:5d3885de780da42ac43772e0841f49a11dce4d3c | CLEAN (1 MINOR left: text.rs not listed in plan, pre-existing) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 1 (cloud thread, autopilot-unit dispatched from a cloud thread; the orchestrator fell back
to nothing; no `mise`, no `gh`: cargo directly, GitHub through MCP) handed over at the context cap.
Base: origin/main `fa0e41a7`; branch `claude/project-thread-krubw0`, two local commits, not pushed,
no PR yet.

- **Done (ticked)**: T001–T007, T009, T010. Core `micold-core/src/sidebar_layout.rs` (types,
  `to_json`/`try_from_json`/`from_json`, `reconcile`, `create`, private `valid_name(name, except)`
  ready for rename); store `layouts_path`/`load_layout`/`save_layout` on `JsonFileStore`,
  `ProjectStore` defaults and `FakeProjectStore`; client `SidebarEntry::UserGroup(GroupHeaderNode
  {id,name,collapsed,count})`, `sidebar::State.layouts: BTreeMap<PathBuf, SidebarLayout>`,
  `State::sidebar_layout_loaded`, `State::sidebar_layout`, `layout_item`, `apply_layout`, applied in
  `State::sidebar_list`. Evidence: `tdd/cycle-log.md` cycles 1–3.
- **Next**: T008 (GroupHeader/DropMarker builders, `TypeRole::UserGroupHeader`), T011→T012
  (`features/sidebar_group_actions.rs`; tests in new `tests/sidebar_group_actions.rs`, wire it as
  `mod sidebar_group_actions;` in `tests/main.rs`, alphabetical), T013, T014, T015, T016, T017,
  T044; then verify.md (review A, review B, visual pass), gate, PR.
- **Open notes for the next unit**:
  - `tests/root_state_is_shared.rs` has an interim `COMPONENT_LOCAL` pin for `sidebar.layouts`;
    once a second module writes it (T012, or the shell's load on project entry) the test fails on
    the stale pin: remove the entry then.
  - `ui/sidebar.rs` `build_items` skips `SidebarEntry::UserGroup` (`=> continue`): T016 draws the
    header there. `row_heights` counts a header as one one-line row; draw it at that height.
  - The client tests are one executable (#1631): Verify commands are
    `cargo test -p micold-client --test main <filter>`, not `--test sidebar_group_actions`.
  - Nothing loads the layout from the store yet: the shell must call
    `sidebar_layout_loaded(project, store.load_layout(project))` when a project is entered
    (`Outcome::ProjectEntered`), and save after each committed operation (R9). Before saving, the
    stored layout should be reconciled against the full (unfiltered) live list.
  - T007's `reveal_ready` expand (U29) and header-as-stop (U28) are M5 on the test list.

## Open escalation

None. <or, one question:>

```
- **Category**: <number and name from rules/escalation.md>
- **Question**: <one sentence>
- **Options**:
  1. <label> (Recommended) — <description>
  2. <label> — <description>
- **Recommended**: <label> — <reason>
- **Evidence**: <paths, commands, results>
- **Banner**: sent <date>
```

<A batch: `### Q1`, `### Q2`... each holding Question, Options, Recommended and Evidence; Category and
Banner stay above them. `scripts/autopilot/run-status.sh` reads these fields.>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
