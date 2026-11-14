# Autopilot ledger — #1813 supervisor-skill

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot supervisor M2: supervisor skill that queues issues and starts sessions (issue #1813); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1813
- **Worktree branch**: claude/project-thread-9ycwrh
- **Runs as**: the orchestrator runs units as `autopilot-unit` in a cloud thread (no `mise`, no `gh`; PRs opened by the orchestrator)
- **Started**: 2026-11-12
- **Phase**: M2 (implement)
- **Next step**: dispatch the M2 milestone unit (T014–T018)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1865 | Design | merged | bec5d62c7 |
| #1885 | M1 | merged | 2084ae6eb |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | capped, gated queue start with ledger | #1885 | merged |
| M2 | T014–T018 | full | resume from ledger, report, watcher | #… | pending |
| M3 | T019–T021 | full | overlap sequencing | #… | pending |
| M4 | T022–T023 | light | small-context skill-text proofs | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |
| D6 | design-pr | PR #1865 red on kube real-cluster job, also red on main; how to proceed? | Merge over the red job (user does the override; hook blocks `--admin`) | decided by user | AskUserQuestion 2026-11-13 |
| D5 | plan | Plan review round 3 still found 2 MAJOR; accept hand fixes or one more round? | Accept fixes (commit 09892a167); no fourth round | decided by user | AskUserQuestion 2026-11-12 |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Review A M1 | 1 | M1 | b6e04930c7c84a94da5c97b12cadcf4d31c923b1 | a3b4af56a7683a14d7f22b75d739c98df0c973fb:b6e04930c7c84a94da5c97b12cadcf4d31c923b1 | CHANGES: 5 MAJOR, 3 MINOR, all fixed (F8 by SKILL text) |
| Review B M1 | 1 | M1 | b6e04930c7c84a94da5c97b12cadcf4d31c923b1 | a3b4af56a7683a14d7f22b75d739c98df0c973fb:b6e04930c7c84a94da5c97b12cadcf4d31c923b1 | CHANGES: 1 MAJOR (drift record, fixed), 2 MINOR (fixed in drift-M1.md and T007/T008) |
| Review spec | 1 | spec | 62f51d14dbd48b0ba3434ad53aed66a19cd26b65 | c1af633993e0d08fce04690906bf0074fe12df1d:62f51d14dbd48b0ba3434ad53aed66a19cd26b65 | CHANGES: 3 MAJOR, fixed |
| Review spec | 2 | spec | 0fb591fff8b90b2d0562841744f93891dd323802 | 6362a8c68621e9b59d41d9acad67d623b69dbd07:0fb591fff8b90b2d0562841744f93891dd323802 | CLEAN (3 MINOR fixed, prose only) |
| Review plan | 1 | design | 09892a1672a95d7ec7f8772b8f1fe16d30dd566a | 215dda838db4069559cf0af8fc45ed9bf133cc53:3cb9cd476b47253d273ced1dd535825e4c22ba4a | CHANGES: 3 MAJOR, fixed |
| Review plan | 2 | design | 09892a1672a95d7ec7f8772b8f1fe16d30dd566a | 282ce57f9c076ebb5fa2df68e3a6b2cb74dd0d7c:9c767eb4ab5e77c5bbbb7976e8aad1b2e4b276fb | CHANGES: 1 MAJOR, fixed |
| Review plan | 3 | design | 09892a1672a95d7ec7f8772b8f1fe16d30dd566a | 8fefdbf6fba62a8c2ff5177f66f97b4f3f284331:65b3da22ecaa145ce24e100a77aa940e8e0c51a0 | CHANGES: 2 MAJOR (spec/quickstart consistency), fixed, not re-reviewed |
| Review tasks | 1 | tasks | 1a1ee197be192270688f4301e1277bd959e95660 | 487bf465b9eb49788881a347aac95e089bd15703:0e835be57dbacca34a2f129c496cd69f7e5f26b0 | CLEAN (2 MINOR fixed, prose only) |
| Review A M1 | 2 | M1 | 1b47d969deda1a299d42c6d4098ce871113166ea | 883e9e28e4bcb50d050a1933e298048422f50072:1b47d969deda1a299d42c6d4098ce871113166ea | CLEAN (scoped, sonnet; all 8 round-1 findings verified fixed) |
| Review B M1 | 2 | M1 | 1b47d969deda1a299d42c6d4098ce871113166ea | 883e9e28e4bcb50d050a1933e298048422f50072:1b47d969deda1a299d42c6d4098ce871113166ea | CHANGES: 1 MAJOR (5 MISSING paths not in drift-M1.md), fixed |
| Review B M1 | 3 | M1 | d85c11e2d86f1483d7fea77719665d2beec3d849 | c6eec4d17d8f2c33413a5ae234d64920cd3608fe:d85c11e2d86f1483d7fea77719665d2beec3d849 | CLEAN (scoped, sonnet) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 merged in #1885 (squash 2084ae6eb) on 2026-11-14. Next: a fresh cloud thread runs `/speckit-autopilot resume #1813` and dispatches the M2 milestone unit (T014–T018). That thread works on its own `claude/...` branch: set the **Worktree branch** line above to it first (resume.sh finds the ledger by that line; it falls back to origin/main). No open findings. Reviews ran as `autopilot-reviewer` subagents dispatched by the orchestrator (units have no Agent tool); `verdict-guard.sh`/`wait-merge.sh` need gh, so the orchestrator merged via GitHub MCP after the ledger's Review rounds were CLEAN for the head.

## Open escalation

None.

## Follow-ups not done

- Full gate: all green except `micold-daemon session_extras::tests::a_signed_commit_succeeds_with_the_delivered_key` (signing, cloud env; no Rust touched). `scripts/tests/*.test.sh` ran individually, all green.
- T008: the supervised argument text lives in `rules/supervised.md` (SKILL.md is at its 60-line ceiling), linked from SKILL.md's route table.

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
