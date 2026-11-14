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
- **Phase**: verify (M1 PR open)
- **Next step**: wait on CI for #1905 and merge; then M2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1848 | Design | merged 2026-11-12 | 553474089e559f95557125168150c3a3abfdcc95 |
| #1905 | M1 | draft, CI pending | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017, T044 | full | Layout core, store, create a group | #1905 | PR open (draft) |
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
| D4 | implement M1 | Does "New group" also get a button in a project header? | No: since feature 1125 the sidebar has no project header and its header buttons are the chip bar's more menu; contract C6a's three entry points (more menu, empty-area menu, chord) are the contract | agent-resolved | contracts/sidebar-groups.md C6a; plan.md line 80 |
| D5 | implement M1 | `GroupHeader::new(name, on_toggle)` with no toggle message before US6? | `GroupHeader::new(name).on_toggle(m)`: no twisty until M5 adds `Msg::Toggle`, so M1 draws no dead control | agent-resolved | tasks.md T040 |

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | A stored `collapsed: true` hides rows under a header with no twisty | M1 never writes `collapsed: true` and M5 (same release) adds the toggle; no released build reads a file this build cannot open |
| M1 | A r1 | Header says "empty" while a filter hides every member | No group can hold a row before M3 (Move to group); revisit the count's meaning there (follow-up below) |
| M1 | A r1 | Empty-area menu does not displace the session menus, more menu or help menu | It displaces exactly what `WorktreeContextMenu` does, the rule the row menus follow; pinned by `popover_displacement.rs` |
| M1 | A r1 | `NamingDraft` does not record its project | The project cannot change while the modal dialog is up through the UI; M2 reworks the draft's targets |
| M1 | A r1 | `refusal()` restates `LayoutError`'s Display | The dialog's words are contract C2's (quoted name); the core's Display is for logs |
| M1 | A r1 | `apply_layout` clones per `sidebar_list()`; layout I/O on the UI thread | Small data (a handful of groups); the write happens only after a group operation, as `save_runs` does. Not a measured cost |

## Handover

M1 unit 3 finished M1: all of T001–T017 and T044 are ticked. Review A ran in round 1 (4 fixed, 6 declined, see the tables above). Review B (conformance) and the visual pass ran afterwards: review B found 1 MAJOR (plan drift record) and 1 MINOR, both fixed; the visual pass under Xvfb passed. origin/main was merged in at 45383389c, the branch is pushed, and draft PR #1905 is open. The PR author is the reviewer, so a review request was refused. Four micold-core permission tests fail only as root in the container: settings_refuses_save_over_failed_read, settings_write_is_logged, and two in worktree_leftovers.

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

- M3: decide whether a header's count is the group's members or its rows shown under the active filter (review A r1, declined for M1).
