# Autopilot ledger — #488 codex-opencode-providers

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement issue #488: Add Codex CLI and OpenCode as session providers; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #488
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-09
- **Phase**: milestone M2
- **Next step**: M2 reviews, visual pass, gate, push; orchestrator opens the PR from scratchpad/pr-488-m2.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #679 | Design (spec, plan, tasks) | MERGED (rebase) | 96f85d69d1e0d335f23e44fc93097483b913a5d7 |
| #680 | M1 | MERGED (rebase) | cf3881fd2bad0e79bc61c9be3669b77f3c6ef2df |
| (orchestrator opens) | M2 | pending | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Start Codex/OpenCode, remembered, wire 39 | #680 | merged |
| M2 | T012–T015 | light | Unavailable providers explained | pending (orchestrator opens) | in progress |
| M3 | T016–T022 | full | Codex resume + naming (seam) | | todo |
| M4 | T023–T026 | full | OpenCode resume + naming | | todo |
| M5 | T027–T030 | light | Honest activity, tool-server, first prompt | | todo |
| M6 | T031–T034 | full | Sandbox image + sign-in | | todo |

## Decisions

- M1 T001 probe (npm install worked): codex 0.162.0, opencode-ai 1.18.35; V8/V10 fields/V13/V14/V15 need a signed-in session so were not probed, fallbacks stay; OPENCODE_DISABLE_AUTOUPDATE=1 set in launch_env; Codex has no env switch. M1 launches Fresh and Resume identically (no args) until M3/M4.
- Tasks unit: US1/US2 split (rule 3) and US3 split Codex/OpenCode; US6 docs ride in each story's milestone; Polish (T035) left to the close unit. Analyze findings I1,I2,C1,C2,T1,A1–A3,D1–D3 fixed in spec/tasks; SC-001 reworded (no number to measure).
- M2 T012/T013: characterization tests passed on existing behaviour (daemon: unavailable listing, MCP and app-path refusal naming the command with no record/terminal, availability after install without restart, removed provider keeps its provider); no code change (T013). T014 client test added in missing_cli_is_reported_where_it_is_chosen.rs.
- M2 visual pass: chooser and Settings follow the 037 design (missing CLIs named with the reason in a note, not as dimmed entries; chooser note only when 2+ CLIs are available). Kept; not changed in M2.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

- M2 review A (fresh, high): CLEAN, 3 MINOR (MCP/app same-reason assertion added; comment fixed; 'as the default' dropped from docs). Review B (sonnet): could not run cargo (denied in its sandbox); the unit ran both Verify commands green itself; only MINOR otherwise. No counted rounds beyond 1.

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Plan | 1 | 528308c54088c27ab49cdd224d332fc9a3bf590f:e5185f251b532f03ff50c09d75a2730c42efbaab | CHANGES: 3 MAJOR fixed (minted-id design via binding files; required trait methods; CredentialLayout Vec),  2 MINOR fixed |
| Plan | 2 | 48a1570d5b81aa926304890b0d296d225ada78ce:e5185f251b532f03ff50c09d75a2730c42efbaab | CLEAN (2 MINOR, fixed prose only) |
| Spec | 1 | 7c296cdcbfffe844d73667e0a7202ac201a4cd1c:cb0c493ce4098522100c5f5837df60a8b6d37777 | CLEAN (3 MINOR, fixed prose only) |
| Tasks | 1 | e22c6bd38c87bb5d49e9aab99ac717d28b8552ef:96244711417951fe05bf6105add19e0cbd91cbfb | CLEAN (3 MINOR, fixed prose only; T035 covers FR-014/SC-006 in close) |
| M1 A (reviewer, sonnet) | 1 | 43ed7a7abe06108b9f82c3a5b75739a1068c7c3f:96f85d69d1e0d335f23e44fc93097483b913a5d7 | CHANGES: 1 MAJOR fixed (ai_cli error text + schema enum now from AiCli::ALL), 2 MINOR fixed (comment-tolerant trust header, docs) |
| M1 A (reviewer, sonnet) | 2 | a959e0e9f42b1e0fa2053a46082bfc93cb27c4b6:67df06aff34a0c102150116786686f8522188556 | CLEAN (2 MINOR: TWO const renamed WITHOUT_PI; `]` inside a trust-header comment declined) |
| M1 B (conformance, sonnet) | 1 | same | CHANGES: only BLOCKER was that the reviewer could not run cargo; the orchestrating unit ran Verify itself (green); 1 MINOR fixed |

## Declined review findings

- M1 A round 2, MINOR trust.rs `project_header` uses the last `]`, so a comment containing `]` after the header is not read as trusted: declined, it fails closed (the project is simply not treated as trusted) and the file is Codex's own.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M2 done locally (gate stamped at tree d942c5fa, commits on claude/project-thread-wysm57, PR text in scratchpad/pr-488-m2.md). Push refused (stale info; remote branch deleted after M1 merge): orchestrator pushes, opens the PR.

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

None yet.
- M2 gap vs FR-003 wording / plan.md:68: Settings dropdown lists only available providers, the others named in the note; with a single installed CLI the chooser has no chevron and no note, so a missing provider is not explained there. Existing 037 behaviour; decide whether a later milestone or the close unit should list unavailable entries.
