# Autopilot ledger — 030-windows-installer

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: "use /speckit-autopilot" (adopted mid-feature; the feature's original input is in spec.md)
- **Kind**: feature
- **Worktree branch**: feat/windows-endpoint
- **Started**: 2026-09-14
- **Phase**: 4-milestones
- **Next step**: record A16's first run and mutant kill (cycle 78), revert the mutant commit 0bda3770, tick T085; then Review A and B on M2.

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
| M2 | see tasks.md § Milestones | installed build connects to its daemon; smoke blocks CI on both Windows legs | #332 | in-progress (T085) |
| M3 | T010, T018, T062–T064 | pid record lifecycle, full gate, research/plan updated | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | One PR or several? | "all in one PR"; later split into #314 and the stacked #332 | user | tasks.md § Implementation Strategy (2026-09-13) |
| D2 | 4-milestones | How to cut milestones for a feature already in two PRs? | M1 = #314, M2 = #332, M3 = close tasks | agent-resolved | this ledger, 2026-09-14 |
| D3 | 4-milestones | Force-push after amending pushed commits? | Never; the auto-mode classifier denied it. Bring #332 onto main by merging main in, then squash-merge | agent-resolved | session 2026-09-13 |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Open escalation

Sent 2026-09-14 (AskUserQuestion + push notification), phase 4-milestones, M2/M3:

1. Category 4, missing access: #314 cannot be merged by this token (no `workflow` scope). Recommended: run `gh auth refresh -h github.com -s workflow`, or merge #314 in the web UI.
2. Category 1, scope: U9 ("lock_path removed on clean exit", E4.1, R4) has no clean exit to observe, and unlinking the Unix flock file is unsafe (cycle-log cycle 6). Recommended: drop U9 and the "removes on clean exit" wording; U12 already makes a stale record harmless.

## Follow-ups not done

- clippy `large_enum_variant` in `crates/micold-daemon/src/singleton.rs:29` (Windows only).
- Unused imports in `tests/available_here.rs` on Windows; dead `users_settings` in the `settings_refuses_save_over_failed_read` tests.
- Restart Manager (`CloseApplications=force`) also stops the daemon during an in-use upgrade.
- Duplicate task ids T074–T076 (US3 series and U71–U73 series).
- A local Windows cross-check builds executables without the icon resource.
- Another flow reuses spec number 030 ("settings rail slide", PRs #334/#339/#340).
- The speckit-autopilot skill is missing from this worktree's `.claude/skills`.
