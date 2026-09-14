# Autopilot ledger — 030-windows-installer

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: "use /speckit-autopilot" (adopted mid-feature; the feature's original input is in spec.md)
- **Kind**: feature
- **Worktree branch**: feat/windows-endpoint
- **Started**: 2026-09-14
- **Phase**: 4-milestones
- **Next step**: push main merged into `feat/windows-package` (#314) once the local check passes; the user merges #314 in the web UI. Then Review A and B on M2 (#332).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #314 | M1 Windows installer package (`feat/windows-package` → main) | open, needs `workflow` token scope to merge | — |
| #332 | M2 Windows daemon endpoint (`feat/windows-endpoint` → `feat/windows-package`, retarget to main after #314) | open | — |

Spec and design PRs predate the flow: spec, plan and tasks ride in #314.

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | see tasks.md § Milestones | x64 and ARM64 installers built in CI, attached to releases, documented | #314 | ci |
| M2 | see tasks.md § Milestones | installed build connects to its daemon; smoke blocks CI on both Windows legs | #332 | in-review |
| M3 | T062–T064 | full gate, quickstart Part M, research/plan updated | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | One PR or several? | "all in one PR"; later split into #314 and the stacked #332 | user | tasks.md § Implementation Strategy (2026-09-13) |
| D2 | 4-milestones | How to cut milestones for a feature already in two PRs? | M1 = #314, M2 = #332, M3 = close tasks | agent-resolved | this ledger, 2026-09-14 |
| D3 | 4-milestones | Force-push after amending pushed commits? | Never; the auto-mode classifier denied it. Bring #332 onto main by merging main in, then squash-merge | agent-resolved | session 2026-09-13 |
| D4 | 4-milestones | How does #314 merge without the `workflow` token scope? | "I'll merge it in the web UI"; then "rebase 314 / or merge main into it" | user | AskUserQuestion 2026-09-14 |
| D5 | 4-milestones | Drop U9 (pid record removed on clean exit)? | "Drop U9 (Recommended)" | user | AskUserQuestion 2026-09-14; cycle-log cycle 78 |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Open escalation

None.

## Follow-ups not done

- clippy `large_enum_variant` in `crates/micold-daemon/src/singleton.rs:29` (Windows only).
- Unused imports in `tests/available_here.rs` on Windows; dead `users_settings` in the `settings_refuses_save_over_failed_read` tests.
- Restart Manager (`CloseApplications=force`) also stops the daemon during an in-use upgrade.
- Duplicate task ids T074–T076 (US3 series and U71–U73 series).
- A local Windows cross-check builds executables without the icon resource.
- Another flow reuses spec number 030 ("settings rail slide", PRs #334/#339/#340).
- The speckit-autopilot skill is missing from this worktree's `.claude/skills`.
