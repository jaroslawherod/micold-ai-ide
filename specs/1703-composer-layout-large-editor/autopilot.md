# Autopilot ledger — #1703 composer-layout-large-editor

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: /speckit-autopilot #1703 (Structured view: composer layout — large-editor shortcut, activity icon beside input, settings toggle for model/effort/permission row, unified look); labels: flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1703
- **Worktree branch**: feat/1703_structured-view-composer-layout-large-editor
- **Started**: 2026-11-05
- **Phase**: milestone M2 implementing (handover)
- **Next step**: finish M2: confirm tests green, docs T016/T025, reviews A+B, visual pass, full gate, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1705 | Design (spec, plan, tasks) | merged | eb1b62d9967f4d7ad7092c0dc75e6d3a1e7af621 |
| #1717 | M1 large editor | merged | 509da759cc19a3ca5befaa90815f9475a7b32dcb |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Large editor: chord/icon opens modal editor, Done/Cancel, shortcut listed | #1717 | merged |
| M2 | T014–T025 | full | Icon activity indicator + settings-icon options row, persisted, auto-expand | | pending |
| M3 | T026–T031 | full | One selector style (MenuSelect), model filter row | | pending |

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

## Declined review findings

- M2 Review B F2 (accessible label): the toolkit has no accessible-name hook in this path (no a11y API in the iced widgets used); the tooltip carries the state words per state and `the_activity_state_is_an_icon_with_its_words_in_the_tooltip` pins them. FR-007 is met to the extent the toolkit allows.
- M2 Review A MINORs (dropped-connection failures and an off-screen session's refusal do not expand the shared row): FR-012 covers refusals; the row is window-level.
- M2 visual pass (headless sway): steps 1 PASS, 2 partial, 3 Ready only, 5 Bypass marker not reached (resumed CLI reported unset mode); Bypass marker, toggle-off and Busy/Waiting are pinned by view tests (T014, T020). Evidence in evidence/m2-*.png and visual-pass-m2.md.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F1 forgotten session recreates composer on Done | Draft is absent for a never-typed composer too, so skipping would break FR-002 Done; minor, left |
| M1 | A | F2 close_topmost bypasses focus return | Minor; Esc/scrim/Done paths restore focus |
| M1 | B | F1 no tdd/cycle-log.md | The design PR set no tdd plan (no tdd/ dir); tests exist and pass, red-first evidence not retrofittable; left for the close unit's TDD verify |

## Handover

Unit M2 (branch reset to origin/main 509da759c, M1 merged) ran out of context after implementing in the working tree (committed as WIP, not pushed, no gate run yet). Done: T014-T015, T017-T024 code and tests (core settings key `composer_options_open`; `Composers.options_open`, `Msg::OptionsToggled(SessionId)`, `Composers::refusal/expand_options/options_toggled`; shell `update` wrapper + `persist_options_if_changed`, `persist_composer_options_open`, boot restore in startup.rs `restore_folds`, daemon_sync model/effort/permission refusals expand + persist; ui/composer.rs: `status_row` now Option (Stop session + refusal note only), `options_row`, `options_button` (settings IconButton + Bypass marker + tooltip), `activity_indicator` (icon-only via new `BusyIndicator::icon_only(Icon)` in micold-material), `Seams.bypass`; pins updated: gates/composer_fits.rs, composer_tab.rs (Tab now passes Open large editor first), overlay_registration INLINE_FLAGS, state_scan MUTATORS, settings_sections CHOSEN_ELSEWHERE, settings_issue_mapping list, layout_snapshot fixture regenerated with UPDATE_LAYOUT_SNAPSHOT=1).

Next: (1) run `scripts/build-lock.sh cargo test -p micold-client --test main` and `--test layout_snapshot` detached and fix remaining failures (last run before the final composer_fits edits had 5 failures in main: composer_tab x2 (fixed since), feature_write_isolation (MUTATORS entries added), features_composer stop_session test (fixed), overlay_registration (added); layout_snapshot had 3 composer_fits failures (edited since)); (2) docs T016 and T025 in docs/user-guide/structured-input.md (icon indicator with tooltip; settings icon "Session options", options row Model/Effort/Permission, persisted per window, auto-expand on refusal, Bypass marker); also grep docs for places naming the status words/permission selector in the input row; (3) tick T014-T025 in specs/1703-composer-layout-large-editor/tasks.md; (4) showcase/catalogue unaffected unless a test says otherwise; (5) review A (code-review high) with scoped gate, review B (conformance), visual pass (quickstart section B steps 1,2,3,5; fixture layout), full `mise run gate`, push (`git remote prune origin` then `git push -u origin HEAD`), PR titled `feat(1703): icon activity indicator and session options row (#1703)`, ledger PR row.

## Open escalation

None.

## Follow-ups not done

None.
