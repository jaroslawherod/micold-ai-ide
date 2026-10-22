# Autopilot ledger — #943 structured-commands-fallback

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 943 — issue #943 "Structured session: slash commands, model switching and PTY fallback" (epic #945); labels: enhancement, flow:feature (in-progress added at start)
- **Kind**: feature
- **Effort**: default
- **Issue**: #943
- **Worktree branch**: claude/project-thread-k392yq
- **Started**: 2026-10-22
- **Phase**: implement M1
- **Next step**: M1 in progress (T001–T008 done; next T009–T018, then gate, reviews, PR). Cross-feature holds: M2 ← #941 M1, M6 ← #941 M2, M7 ← #941 M4, M8 ← #941 M6, M9 ← #942 M7 (PromptCard) and #942 M8 (SC-007). Decision 2 provisional (M5 only)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1237 | design (spec, plan, tasks) | merged (rebase) | f27e1182 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | Core: commands, terminal-only, palette, model, control vs recorded fixtures | | in progress |
| M2 | T019–T024 | full | Core transcript: chips, command output, compact/clear, model and terminal markers. **Needs #941 M1** | | pending |
| M3 | T025–T031 | full | Daemon: command list at spawn; `/model` and terminal-only refused | | pending |
| M4 | T032–T038 | full | Daemon: model choice persisted, `--model`, live `set_model` | | pending |
| M5 | T039–T042 | full | Daemon: resume the post-`/clear` conversation — **provisional, decision 2** | | pending |
| M6 | T043–T049 | full | Daemon: switch kind. **Needs #941 M2** | | pending |
| M7 | T050–T054 | full | Client render-free palette, picker, switch offer. **Needs #941 M4** | | pending |
| M8 | T055–T060 | full | Window palette, picker, markers, guide. **Needs #941 M6** | | pending |
| M9 | T061–T065 | full | Window switching, offers, terminal-only table. **Needs #942 M7 (PromptCard) / M8 (SC-007)** | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify r1 | Terminal-period turns shown in the transcript? (US5 s2) | Only marked, not rebuilt from the CLI store; FR-023 updated | agent | 938 spec US7; 582 Out of Scope |
| 2 | clarify r1 | After `/clear`, which conversation do later launches resume? (US1 s5) | **Provisional A, pending jaro**: structured sessions follow the CLI (FR-005a); terminal keeps BUG-904 trade-off. Revert to B/C if he picks otherwise | user (provisional, via coordinator) | BUG-904 decision 1; 582 FR-017 |
| 3 | clarify r2 | Terminal launch after structured `/clear`; terminal `/clear` | Resume recorded handle; terminal `/clear` unseen | agent | 582 FR-017 |
| 4 | plan | FR-003: the CLI never writes a skill's expanded instructions to stdout; what goes behind the command? | The command's description; expansion only if a later CLI reports it; FR-003 and US1 s3 amended | agent | research.md R4, probe P-3 (2.1.307) |
| 5 | tasks | analyze I1/I2: does only M2's chip wait on #941 M1, and are `saved.rs`/`provider.rs` in the plan? | M2 is held whole on #941 M1; plan tree lists both files (plan.md updated) | agent | analyze report I1, I2 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 67da24dc3a70beca179b57d029e7f152bc1a7ab6:dc6473291290b51830029c741e2fe6ab5630544e | CHANGES: 2 MAJOR, 3 MINOR — all fixed |
| Spec review | 2 (sonnet, fix diff) | 910550013b1158c635de912a8ee68881ce5de668:dc6473291290b51830029c741e2fe6ab5630544e | CLEAN (3 MINOR, fixed, prose only) |
| Plan review | 1 | bbe19f9619357fdbd70dce02a8e1225612cdd4e6:8915672b72d2ed2e420a78fd89db407984f5ce8f | CHANGES: 2 MAJOR, 2 MINOR — all fixed |
| Plan review | 2 (sonnet, fix diff) | cdb8aa85a308959029eb2b14aa15dfa62867367d:8915672b72d2ed2e420a78fd89db407984f5ce8f | CLEAN (1 MINOR, fixed, prose only) |
| speckit-analyze | 1 (sonnet) | — | 0 CRITICAL, 1 HIGH (C1), 4 MEDIUM, 5 LOW — C1, C2, G1, I1, I2, U1–U3 fixed (I1/I2 in plan.md); I3 left to jaro (decision 2), D1 informational |
| Tasks review | 1 | de7f53dab39c9aba91fd555cefc704e366ccdb3f:ee00a672e9c7ceefaa1edde46c12c37b7a0a103e | CHANGES: 1 MAJOR (Part B 3 / SC-007 moved M8 → M9), 3 MINOR — all fixed |
| Tasks review | 2 (sonnet, fix diff) | 0c12e486473d3a8cc0937758fdd4473a3e424f23:ee00a672e9c7ceefaa1edde46c12c37b7a0a103e | CLEAN (1 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
