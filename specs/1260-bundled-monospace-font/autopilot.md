# Autopilot ledger — #1260 bundled-monospace-font

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1260 — "Bundle a monospace font so code blocks and diffs look the same on every host" (follow-up from #940; #1276 depends on it). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1260
- **Worktree branch**: claude/project-thread-c0bgc0
- **Started**: 2026-10-24
- **Phase**: plan done
- **Next step**: tasks unit (plan CLEAN after 2 rounds; FR-007 terminal adoption awaits the user's confirmation — plan D4 lists the swap if they decline)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-007: does the terminal pane adopt the bundled monospace face? | Yes (option A): terminal cells use the bundled face (Bold for bold cells), keeping its own grid metrics | agent | 018 FR-012 exempts the terminal from Roboto/type scale only, not from a bundled mono; `cell_font` uses `Font::MONOSPACE`; `CellMetrics::new` hard-codes 0.6 em = Fira Mono's advance; Principle VI |
| 2 | plan | Release bundles carry no font licence today (spec assumption wrong): which licences ship? | All font licence files (Fira Mono OFL, Roboto OFL, Material Symbols Apache-2.0) plus PROVENANCE.md, in a `fonts` licence dir of deb/rpm/.app/installer | agent | research R5: deb/rpm assets, macos-bundle.sh, .iss [Files] ship no `assets/fonts/LICENSE*`; OFL requires licence to travel with the font |
| 3 | plan | How `Font::MONOSPACE` reaches the bundled face, robust to a same-name host font | Shared `install_monospace`: load Regular+Bold, remove other `Fira Mono` faces, set fontdb monospace family; app, showcase and both test renderers call it | agent | research R2: fontdb `query` ties go to the first-loaded (host) face; cosmic-text default monospace family `Noto Sans Mono` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Clarify | 1 | FR-007 | resolved by agent (requirement changed: FR-007, FR-010, edge case) |
| Clarify | 2 | after round 1 | no critical ambiguities; unit ends CLEAN |
| Spec review | 1 | 2a0f5efa62e0546cb87daef04623b4c3b3b572ad:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (3 MINOR, all fixed: Regular+Bold decided as FR-001/FR-011; SC-002 reworded; checklist tick corrected, FR-001/FR-006 reworded as outcomes). Reviewer ran as headless `claude -p` (no Agent tool in this unit) |
| Plan review | 1 | 4b3fe7335ab982bd2656ab335f9a7da631006aee:5f36be6f05b0fd5cb520da88bf1c3c6f00ef576b | CHANGES (2 MAJOR: Windows .iss allow-list in packaging_excludes_showcase.rs; FR-007 decline path understated) — both fixed. Headless `claude -p` |
| Plan review | 2 | 3b69b243e65412cceb69de7ab247213b1a4f1a6f:5f36be6f05b0fd5cb520da88bf1c3c6f00ef576b | CLEAN (1 MINOR: DestDir parser helper unnamed — fixed, prose only). Headless `claude -p --model sonnet` |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
