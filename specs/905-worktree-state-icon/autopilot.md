# Autopilot ledger — #905 worktree-state-icon

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #905 — "Worktree should show a state icon: idle/waiting, blocked or running". Each worktree entry should show an icon for its current state: Idle / waiting (no work in progress, or waiting for user input); Blocked (cannot proceed, e.g. needs approval, failed, waiting on a lock); Running (the session in the worktree is actively working). Expected: the icon is visible on the worktree in the list without opening it; state is derived from the worktree's session and updates live; icons are distinguishable by shape as well as colour. labels: enhancement (orchestrator added in-progress, flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #905
- **Worktree branch**: claude/project-thread-h057j5
- **Started**: 2026-10-17
- **Phase**: implement
- **Next step**: M1 verify.md from *Handover* (all T001–T017 ticked)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #913 | design (spec, plan, tasks) | merged | a09e523796e7d9b571b410bce28848745c1e9734 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Worktree and Default rows show a live state icon; showcase, contrast gate, user guide and icon reference | — | in progress |
| M2 | T018–T026 | full | A mid-turn permission stop makes its worktree Blocked (protocol 48) | — | pending |

M1 holds 17 tasks (over the ~10 of milestones.md rule 3): it is Foundational + US1 + US3, because the shared `StateIcon` must ship with its showcase entry, contrast proof and icon-reference rows (Principles VII, VIII; speckit-analyze C1, C2). US1 has no acceptance-scenario split that leaves a deliverable without the component. T027–T028 (visual pass, §C) go to the close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | Mid-turn question (not a permission): Blocked or Idle? | Blocked, same as a permission | agent | specs/613-notification-kinds/spec.md#Terms; micold-core attention.rs NotificationKind::NeedsPermission |
| 2 | clarify | Several sessions: all or main only? | All un-archived sessions, Blocked > Running > Unknown > Idle | agent | specs/575-workspace-attention-list/spec.md#Terms (location row counts every session) |
| 4 | implement | `report`'s codepoint | `U+F052`, not research's `U+E160` (that is Material *Icons*' `report`; the Symbols `.codepoints` manifest maps `report` to `f052`, no entry for `e160`); spec artifacts updated | agent | upstream `variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].codepoints` |
| 5 | implement | `StateIcon` default size: contract names `TypeRole::SidebarLabel`, which does not exist | `TypeRole::SidebarName` (the sidebar tree's label role); the host passes `label_role.size()` anyway | agent | `crates/micold-client/src/ui/material/text.rs` |
| 6 | implement | Showcase entry `variants` Running/Blocked/Idle (contract S6) | `posed` instead, `variants: &[]`: C4 requires `variants` to be a library (`ui/`) enum's, and `LocationState` is micold-core's; T014 checks `posed` | agent | `tests/showcase_completeness.rs` c4_no_variant_outlives_itself |
| 3 | clarify | Unknown activity: Idle or no icon? | No icon, reserved empty slot, tooltip "State unknown" | agent | specs/010-daemon-session-persistence FR-016c; specs/488-codex-opencode-providers FR-009 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 2148471a6892a9acfe8441d5914d9cacd029f1e4:bfdc456e3ecbab4afa9c30045923409081433f23 | CHANGES: 1 MAJOR, 2 MINOR (all fixed) |
| Spec review | 2 | 047d8dca47a8242fd7e54bccd72425e87b1e87dc:bfdc456e3ecbab4afa9c30045923409081433f23 | CLEAN: 2 MINOR (fixed, prose only) |
| Plan review | 1 | 7ff73d272ce69913ac4b831931834346e5e5a681:7093a3877e09df8c0b61a273d62ad507a90888f6 | CLEAN: no findings |
| Tasks review | 1 | 09001da43f215b58f9b3a1028df23f2279894dc7:cd966ec8f3b1df26c9b4c8ee1af19cfe0bbb0fcd | CLEAN: 1 MINOR (checklist item ticked, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 2 stopped at the 150k cap after implement. Not pushed, no PR. Base origin/main a09e5237.

- **Done**: T001–T017 all ticked and green locally: `cargo test -p micold-client --lib` (637), `--test icons icons_font sidebar_state_icon showcase_completeness showcase_captions material_boundary` all ok; `cargo fmt --all` run. Core `location_state` tests green from unit 1.
- **Not yet run**: `cargo test -p micold-core`, clippy, the full gate, any review.
- **Next**: verify.md from step 1: scoped gate (`scripts/autopilot/scoped-gate.sh`) detached + review A (`code-review` high on `origin/main...HEAD`), then review B (conformance, sonnet) and the visual pass (showcase `StateIcon` entry; sidebar rows), then full gate `scripts/gate.sh` detached, then pr.md (cloud overrides: push, write PR title/body to scratchpad, orchestrator opens PR).
- **Visible change**: yes (sidebar rows, showcase) → visual pass required.
- Build helper: detached `scripts/build-lock.sh cargo …` with `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`. `$SCRATCHPAD` is unset in this env; use the session scratchpad path.

## Open escalation

None.

## Follow-ups not done

None.
