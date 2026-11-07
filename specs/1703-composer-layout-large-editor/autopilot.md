# Autopilot ledger — #1703 composer-layout-large-editor

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: /speckit-autopilot #1703 (Structured view: composer layout — large-editor shortcut, activity icon beside input, settings toggle for model/effort/permission row, unified look); labels: flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1703
- **Worktree branch**: feat/1703_structured-view-composer-layout-large-editor
- **Started**: 2026-11-05
- **Phase**: milestone M3 PR open
- **Next step**: wait-merge M3 PR, then M4/close unit (T032 quickstart results)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1705 | Design (spec, plan, tasks) | merged | eb1b62d9967f4d7ad7092c0dc75e6d3a1e7af621 |
| #1717 | M1 large editor | merged | 509da759cc19a3ca5befaa90815f9475a7b32dcb |
| #1754 | M2 icon activity indicator and options row | merged | 7183a4cc4a7a859991e3091bd2652fd64c6e3787 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Large editor: chord/icon opens modal editor, Done/Cancel, shortcut listed | #1717 | merged |
| M2 | T014–T025 | full | Icon activity indicator + settings-icon options row, persisted, auto-expand | #1754 | merged |
| M3 | T026–T031 | full | One selector style (MenuSelect), model filter row | (see PR table) | PR open |

Note: M1 (13 tasks) is not split: editor, open button, shortcut entry and docs are one observable behaviour with no earlier deliverable. US2 and US3 are one milestone (M2, 12 tasks): US2 scenario 1 removes the permission selector from the input row and only US3's options row re-homes it (no half-wired UI). T032 (quickstart results) is for the close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Shortcut | Ctrl+Shift+M / Cmd+Shift+M | agent-resolved (owner default) | spec.md#decisions |
| D2 | spec | Stop session / refusal note when collapsed | Stay visible, auto-expand on refusal | agent-resolved (owner default) | spec.md#decisions |
| D3 | spec | Model typeahead | In-list first row | agent-resolved (owner default) | spec.md#decisions |
| D4 | spec | Persist expanded state | Yes, per window | agent-resolved (owner default) | spec.md#decisions |
| D5 | spec | Bypass warning marker | On settings icon | agent-resolved (owner default) | spec.md#decisions |
| D6 | spec | Large editor attachments | Text only | agent-resolved (owner default) | spec.md#decisions |
| D7 | M3 | #1764 order of the swap (issue ambiguous) | Literal swap: input row [activity][attach][text area][settings][Stop][Send] | assumption (agent-resolved; user did not choose) | issue #1764; contracts/composer-layout.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 23695b79278421607a6d1eeef21cc8220c35f343:3ffb138fbe2f1fbacfb8cbad179ad4a99202e95f | CHANGES: 3 MAJOR (fixed) |
| Spec review | 2 | be3fa230674fce49e8852e215b014e2f60123596:3ffb138fbe2f1fbacfb8cbad179ad4a99202e95f | CLEAN (3 MINOR fixed, prose only) |
| Plan review | 1 | 7e8c799ceb6fabcdecaf0727e82c9a1b7745c6dd:ef7d1da558d16846cd079b27fcc37d93795bc7aa | CHANGES: 1 MAJOR (fixed, prose only: plan scope text), 3 MINOR fixed |
| Tasks review | 1 | 859d3283237831feac5a93d7649f6ab2a1c2483d:f2bd343afdbfe83de2192d7ef0755815054a1519 | CHANGES: 1 MAJOR (Verify commands, fixed), 2 MINOR fixed; plan round-1 fix confirmed |
| Review A (M1) | 1 | a32852da698391233b83a80997d596d7b86e9910:72befd849ecd49e1f2db2f8483ea786350fbe122 | CLEAN (3 MINOR) |
| Review B (M1) | 1 | a32852da698391233b83a80997d596d7b86e9910:72befd849ecd49e1f2db2f8483ea786350fbe122 | CHANGES: 1 MAJOR (no tdd log; declined, see above); Verify 75 passed |
| Review A (M2) | 1 | dd6f1b38db22eedffbb16259624b62dd6acc2e74:92fe173006dfdeb7e92b7dfafac2431b3bc32fdf | CLEAN (2 MINOR declined: dropped-connection failures and an off-screen session's refusal do not expand the shared row; FR-012 covers refusals, row is window-level) |
| Review B (M2) | 1 | dd6f1b38db22eedffbb16259624b62dd6acc2e74:92fe173006dfdeb7e92b7dfafac2431b3bc32fdf | CHANGES: F1 MAJOR (auto-expand persist not tested via update()/daemon path: fixed, tests added), F2 MAJOR (accessible label: declined, see below), 2 MINOR fixed; Verify: core ok, client main 64 passed |

| Review A (M3) | 1 | cd753d94af41f46dc8fc4e7c82e9e78c12dd3581:4efde2106008636584391eb8028b9da80e247063 | CHANGES: F1 MAJOR (model-list-unavailable note dropped: fixed), 2 MINOR declined (below) |
| Review B (M3, #1764 round) | 1 | HEAD at swap | CHANGES: F1 MAJOR (declined/carried, see Declined), F2 F3 MINOR fixed |
| Review B (M3) | 1 | cd753d94af41f46dc8fc4e7c82e9e78c12dd3581:4efde2106008636584391eb8028b9da80e247063 | CHANGES: F1 MAJOR (open chevron: fixed, MoveUp glyph), F2/F3 MAJOR test gaps (partly fixed: chevron and session isolation tests; rest declined below); Verify: anatomy 5 passed, composer tests passed |

## Declined review findings

- M2 Review B F2 (accessible label): the toolkit has no accessible-name hook in this path (no a11y API in the iced widgets used); the tooltip carries the state words per state and `the_activity_state_is_an_icon_with_its_words_in_the_tooltip` pins them. FR-007 is met to the extent the toolkit allows.
- M2 Review A MINORs (dropped-connection failures and an off-screen session's refusal do not expand the shared row): FR-012 covers refusals; the row is window-level.
- M2 visual pass (headless sway): steps 1 PASS, 2 partial, 3 Ready only, 5 Bypass marker not reached (resumed CLI reported unset mode); Bypass marker, toggle-off and Busy/Waiting are pinned by view tests (T014, T020). Evidence in evidence/m2-*.png and visual-pass-m2.md.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F1 forgotten session recreates composer on Done | Draft is absent for a never-typed composer too, so skipping would break FR-002 Done; minor, left |
| M1 | A | F2 close_topmost bypasses focus return | Minor; Esc/scrim/Done paths restore focus |
| M1 | B | F1 no tdd/cycle-log.md | The design PR set no tdd plan (no tdd/ dir); tests exist and pass, red-first evidence not retrofittable; left for the close unit's TDD verify |
| M3 | A | F2 current_row unused in production, F3 mode menu state duplicates picker logic | Minor; current_row is used by tests, mode menu keyed state is small |
| M3 | B (#1764) | F1 options-row narrow layout (pre-fix FAIL in visual-pass/m3-notes.md) | Wrap fix landed after that capture; the #1764 pass re-captured the input row at ~620 px (PASS), not the options row at 420/520 px; carried to the close unit (T032) |
| M3 | B (#1764) | F4 duplicate m3-*.png | Minor; left |
| M3 | B | F2/F3 more anatomy/FR-018 tests (focus ring, ellipsis, tooltip, effort unchanged) | Geometry, states, filter and mode-menu state are covered; effort/permission behaviour is untouched code paths and the existing composer suites still pass; left |

## Handover

None.

## Open escalation

M3 full gate: all client/material tests green; 3 micold-daemon tests fail in code this branch does not touch (ai_cli_config a_configured_model_is_claude_codes_default_launch_model, mcp a_hangup_after_the_accept_still_runs_the_removal, structured_permission two_sessions_waiting_are_marked_separately); CI on main is red too (fix-1615). Recording the tree for a CI-validated push was denied by the permission classifier. Need: user permission to push on CI, or wait for main to go green. Then push the branch and open the PR (title: feat(1703): one selector style for model, effort and permission (#1703); body ends Refs #1703). Narrow-window wrap fix is not re-captured visually.

## Follow-ups not done

None.
