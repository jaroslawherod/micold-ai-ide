# Autopilot ledger — #1703 composer-layout-large-editor

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: /speckit-autopilot #1703 (Structured view: composer layout — large-editor shortcut, activity icon beside input, settings toggle for model/effort/permission row, unified look); labels: flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1703
- **Worktree branch**: feat/1703_structured-view-composer-layout-large-editor
- **Started**: 2026-11-05
- **Phase**: tasks
- **Next step**: design PR merge, then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1705 | Design (spec, plan, tasks) | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Large editor: chord/icon opens modal editor, Done/Cancel, shortcut listed | | pending |
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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Unit M1 (T001-T013) hit the 150k cap while waiting on the build lock. Previous PR #1705 merged (eb1b62d99); branch-start.sh was run (reset to origin/main).
Done (uncommitted work committed as WIP): all code, tests and docs for T001-T013 are WRITTEN but NOT yet compiled or run, and tasks.md checkboxes T001-T013 are NOT yet ticked:
- icons: `Icon::OpenInFull` (U+F1CE, verified in the font cmap) + tests/icons.rs (count 87, new pin test) + PROVENANCE.md row
- `features/large_editor.rs` (State{open,hovered}, Msg, `update(&mut app::State, Msg)`, `editor_key`, `LargeEditorDialog` surface), `Message::LargeEditor`, `State.large_editor`, registry entry, `ui/large_editor.rs` (dialog, `body`, `field()`), `shell/large_editor.rs` (focus), main.rs arm, `shell/composer.rs` (`KeyIntent::LargeEditor`, `Msg::LargeEditorRequested`)
- `features/composer.rs`: chord, `open_editor_visible` + its unit test, `Msg::LargeEditorRequested`; `ui/composer.rs`: `open_editor_button`, `text_area_with_button` (stack + mouse_area hover)
- keys.rs sheet entry + test; showcase entry (surfaces.rs); docs structured-input.md + terminal-panes.md
- tests: tests/features_large_editor.rs, features_composer.rs chord test, overlay_registration.rs dialog test
Next: run `mise run check` (a first run only reached a showcase type error, fixed with `.map(|_| Message::NoOp)`; one detached `mise run check` may still be queued behind another worktree's build lock, log in the old scratchpad), fix compile errors, then run `scripts/build-lock.sh cargo test -p micold-client --test features_large_editor --test features_composer --test overlay_registration --test icons` and the lib tests for `open_editor_visible` / keys.rs; check `tests/surface_registration_cost.rs` and `features_are_render_free.rs` still pass (large_editor.rs names `crate::floating_surface` only). Then tick T001-T013 in tasks.md, then verify.md flow (scoped gate + review A high, review B, visual pass for the new button/dialog, full gate), PR.
Open risks: hover button overlays the text area top-right; `Done` on a forgotten composer recreates it (minor).

## Open escalation

None.

## Follow-ups not done

None.
