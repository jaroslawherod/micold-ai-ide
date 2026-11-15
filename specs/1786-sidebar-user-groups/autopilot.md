# Autopilot ledger — #1786 sidebar-user-groups

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1786 "feat: sidebar: let the user create groups"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1786
- **Worktree branch**: claude/project-thread-sufjt3
- **Started**: 2026-11-11
- **Phase**: milestones (M3)
- **Next step**: M3 review B and visual pass (orchestrator), then open the PR from specs/1786-sidebar-user-groups/pr-M3.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1848 | Design | merged 2026-11-12 | 553474089e559f95557125168150c3a3abfdcc95 |
| #1905 | M1 | merged 2026-11-14 | b479914eb6744a3691cd90d805b7547e24464392 |
| #1913 | M2 | merged 2026-11-15 | 6006f3e02039a34205aa3afad24a23e7a8d6b511 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017, T044 | full | Layout core, store, create a group | #1905 | merged |
| M2 | T018–T023 | full | Rename and delete a group | #1913 | merged |
| M3 | T024–T028 | full | Move to group from the menu | #… | pushed, awaiting review B, visual pass and PR |
| M4 | T029–T034 | full | Drag between lists and cancel | #… | pending |
| M5 | T035–T042 | full | Reorder, collapse, no state grouping | #… | pending |
| M6 | T043, T045 | full | Showcase poses and visual pass | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |
| D2 | implement M1 | Where is the layout applied? | In `State::sidebar_list` (the drawn list), not `sidebar_entries`: the Overview's cards walk `sidebar_entries` and must not lose a collapsed group's cards | agent-resolved | crates/micold-client/src/ui/overview.rs `cards` |
| D3 | implement M1 | How are ids never reused across restarts? | A stored `next_id` counter in the layout file | agent-resolved | data-model.md § UserGroup `id` |
| D4 | implement M1 | Does "New group" also get a button in a project header? | No: since feature 1125 the sidebar has no project header and its header buttons are the chip bar's more menu; contract C6a's three entry points (more menu, empty-area menu, chord) are the contract | agent-resolved | contracts/sidebar-groups.md C6a; plan.md line 80 |
| D5 | implement M1 | `GroupHeader::new(name, on_toggle)` with no toggle message before US6? | `GroupHeader::new(name).on_toggle(m)`: no twisty until M5 adds `Msg::Toggle`, so M1 draws no dead control | agent-resolved | tasks.md T040 |
| D6 | implement M2 | T022 says the group menu opens "by the 803 chord", but no row menu has a keyboard opener | The menu opens by right click; its items are keyboard-operable like any menu. The keyboard opener lands with the header's tab stop in T040 (M5), which now names it (803 chord / Menu key on the focused header, with its test); U30 is PARTIAL | agent-resolved | tasks.md T040; test-list U30 |
| D7 | implement M3 | Is a header's count the group's members or the rows shown under the active filter? | The rows shown, with the hidden members said aloud: `n` while none is hidden, `n of m` while a tag filter hides some, `empty` only for a group that holds no member. A group is not empty because a filter hides its members, and the count never claims rows the list does not draw (C3: hidden rows are not targets, their stored places stay). A member whose worktree is gone is not counted as hidden | agent-resolved | spec.md edge cases (empty group; tag filter hides worktrees); contracts/sidebar-groups.md C3, C5; tests `a_header_counts_the_rows_shown_*` |

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
| M1 diff (A, code-review high) | 1 | origin/main...14c552a6a | 14c552a6a | fa0e41a7:14c552a6a | CHANGES (3 fixed: hint counted headers, chord reset an open dialog, layout file kept on forget; plus the sibling_dir cleanup; 6 declined) |
| Review B M1 | 1 | conformance | 18bc3ebc25f945817224ca706f2089e258172acd | 571a6f78f42446d00816ad36b6d4fbad107069fa:18bc3ebc25f945817224ca706f2089e258172acd | CHANGES (1 MAJOR fixed: drift-M1.md; 1 MINOR fixed: Verify line) |
| Visual pass M1 | 1 | quickstart B1 | 18bc3ebc25f945817224ca706f2089e258172acd | — | PASS (screenshots /mnt/project-files/issue-1786/m1-*.png; header has no collapse glyph yet, M5) |
| M2 diff (A, code-review high) | 1 | origin/main...HEAD | 1899fc214e2463840e7778e90ac0d3a0f6ff7164 | 70213a3cb713aa6253c12e0215a8276a1fa7d027:1899fc214e2463840e7778e90ac0d3a0f6ff7164 | CHANGES (4 fixed: stale menus survive Help, more menu and switcher; rename kept the menu under an open dialog; no-op rename wrote the store; duplicate menu code; 4 declined) |
| Review B M2 | 1 | M2 | 53565ed9eb4b8c572a8a84459f0c1a0bab55badc | b479914eb6744a3691cd90d805b7547e24464392:53565ed9eb4b8c572a8a84459f0c1a0bab55badc | CHANGES (2 MAJOR fixed: drift-M2.md, T040 names keyboard opener; 2 MINOR fixed) |
| Review B M2 | 2 | M2 | 4c3288732b05a7f614473ad3c58c82a7467a151b | 53565ed9eb4b8c572a8a84459f0c1a0bab55badc:4c3288732b05a7f614473ad3c58c82a7467a151b | CLEAN (scoped re-review of da2697f8e, 4c3288732) |
| Visual pass M2 | 1 | M2 | 53565ed9eb4b8c572a8a84459f0c1a0bab55badc | b479914eb6744a3691cd90d805b7547e24464392:53565ed9eb4b8c572a8a84459f0c1a0bab55badc | CLEAN (PASS; screenshots /mnt/project-files/issue-1786/m2-*.png; later fixes docs only) |
| Review B M3 | — | conformance | — | — | PENDING (orchestrator runs it; no Agent tool in this cloud thread) |
| Visual pass M3 | — | quickstart | — | — | PENDING (orchestrator runs it) |
| M3 diff (A, code-review high) | 1 | origin/main...HEAD | 23a411d07f6a30308c2923efb6e0cee3278f1fc4 | 2687a887a67c6a031fda6a8fb6897ee53fe046fd:2f9d3665853d6d3d41fa9ac8d8b5135f74db9cfd | CHANGES (5 fixed: header counted gone rows as hidden, choosing the row's own list reordered it, New group over an open dialog closed it, listed ignored filters, adopt rule copied twice; 3 declined) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | A stored `collapsed: true` hides rows under a header with no twisty | M1 never writes `collapsed: true` and M5 (same release) adds the toggle; no released build reads a file this build cannot open |
| M1 | A r1 | Header says "empty" while a filter hides every member | No group can hold a row before M3 (Move to group); revisit the count's meaning there (follow-up below) |
| M1 | A r1 | Empty-area menu does not displace the session menus, more menu or help menu | It displaces exactly what `WorktreeContextMenu` does, the rule the row menus follow; pinned by `popover_displacement.rs` |
| M1 | A r1 | `NamingDraft` does not record its project | The project cannot change while the modal dialog is up through the UI; M2 reworks the draft's targets |
| M2 | A r1 | Menu is right-click only; U30/T022 claim the 803 chord | No row menu has a keyboard opener in the codebase; the header tab stop is T040 (M5). U30 marked PARTIAL, decision D6 |
| M2 | A r1 | `GroupMenu` holds only the group id; a project switch with the menu open could act on another project's group | The runs and worktree menus keep ids the same way; an unknown id is refused, and a menu is closed by every dialog and popover |
| M2 | A r1 | `OnRightPress` is `'static` | The sidebar's rows are `'static`; a lifetime on `GroupHeader` buys nothing here |
| M2 | A r1 | Rename `GroupContextMenu` to `UserGroupContextMenu` | The task names it `GroupContextMenu`; every registry use is fully qualified and its id is `user_group_menu` |
| M1 | A r1 | `refusal()` restates `LayoutError`'s Display | The dialog's words are contract C2's (quoted name); the core's Display is for logs |
| M1 | A r1 | `apply_layout` clones per `sidebar_list()`; layout I/O on the UI thread | Small data (a handful of groups); the write happens only after a group operation, as `save_runs` does. Not a measured cost |
| M3 | A r1 | Menu with many groups is taller than the window and never scrolls; long names overflow; a group named "Ungrouped" reads like the Ungrouped entry | Every row menu is flat and clamped the same way; scrolling or truncating menus is not this feature's scope, and the desktop app is being sunset |
| M3 | A r1 | Moving an unheld row to Ungrouped does nothing | An unheld row is already ungrouped, in stable order; saving it would pin it to the front. Consistent with choosing the list a row is already in |
| M3 | A r1 | Header-fits gate only exercises `hidden: 0` | The gate's `group_header_row` pose is the showcase's; its `n of m` pose belongs to M6 (T043). `count_words` is unit-tested |

## Handover

M3 unit finished T024–T028 (all ticked). Core `SidebarLayout::move_item`/`adopt`/`holds`/`list_of`; client `Msg::Move`, `MoveTarget`, `move_menu_items`, `NamingTarget::NewAndMove`, the "Move to ..." entries in the worktree menu (and so the Overview card menu, the keyboard path) and the run group menu; the header count decision D7 (`n`, `n of m`, `empty`). Review A ran in round 1 (5 fixed, 3 declined). Review B and the visual pass are PENDING: this cloud thread has no Agent tool, so the orchestrator runs them after the unit returns and records their rounds above. Next: run them, fix any BLOCKER or MAJOR, open the PR from `specs/1786-sidebar-user-groups/pr-M3.md` (title `#1786: feat: sidebar: let the user create groups (M3)`), wait for CI and merge, then M4 (T029–T034). Four micold-core permission tests fail only as root (settings_refuses_save_over_failed_read, settings_write_is_logged, two in worktree_leftovers): ignore them.

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

- M5 (T040): the visual pass could not see keyboard focus move into a pointer-opened menu (group menu and worktree-row menu alike: Down/Tab showed no highlight, Return hit the button underneath). Check menu keyboard focus when the header tab stop and keyboard opener land.
- Seen in the M2 visual pass, not isolated: after Ctrl+Shift+G (New group) the More menu stayed open behind the naming dialog and after Create.

