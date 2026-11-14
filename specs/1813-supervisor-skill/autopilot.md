# Autopilot ledger — #1813 supervisor-skill

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot supervisor M2: supervisor skill that queues issues and starts sessions (issue #1813); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1813
- **Worktree branch**: claude/project-thread-87zjfg
- **Runs as**: the orchestrator runs units as `autopilot-unit` in a cloud thread (no `mise`, no `gh`; PRs opened by the orchestrator)
- **Started**: 2026-11-12
- **Phase**: M4 (implement)
- **Next step**: dispatch the M4 milestone unit (T022–T023, tier light: model sonnet)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1865 | Design | merged | bec5d62c7 |
| #1885 | M1 | merged | 2084ae6eb |
| #1888 | M2 | merged | 7f4a47735 |
| #1889 | M3 | merged | f6805eb68 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | capped, gated queue start with ledger | #1885 | merged |
| M2 | T014–T018 | full | resume from ledger, report, watcher | #1888 | merged |
| M3 | T019–T021 | full | overlap sequencing | #1889 | merged |
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
| Review A M2 | 1 | M2 | a6b47dde3e2d0cd38c646518923d0f1597a95089 | 57d4fd80ad63c2c15d6948227164676e10f22d0d:a6b47dde3e2d0cd38c646518923d0f1597a95089 | CHANGES: 1 MAJOR, 3 MINOR, all fixed |
| Review B M2 | 1 | M2 | a6b47dde3e2d0cd38c646518923d0f1597a95089 | 57d4fd80ad63c2c15d6948227164676e10f22d0d:a6b47dde3e2d0cd38c646518923d0f1597a95089 | CHANGES: 1 MAJOR (drift-M2.md), fixed |
| Review A M2 | 2 | M2 | 65f3c5c54e06942a0c097dc61725d4e619230b18 | 256c7c859e5fb9cfc28f374abe306c9ab5c51110:65f3c5c54e06942a0c097dc61725d4e619230b18 | CLEAN (scoped, sonnet; 4 round-1 findings verified fixed, 1 MINOR wording) |
| Review B M2 | 2 | M2 | 65f3c5c54e06942a0c097dc61725d4e619230b18 | 256c7c859e5fb9cfc28f374abe306c9ab5c51110:65f3c5c54e06942a0c097dc61725d4e619230b18 | CLEAN (scoped, sonnet) |
| Review A M3 | 1 | M3 | b3feeb311945de92fbe0cf6cfd91c22104879d6b | a207cda5e89efb15af68529069367e1f8505f8bf:3ea92d58dd3fb8dfc67f520adffbc4aa13c48003 | CHANGES: 2 MAJOR (CRLF, globs), fixed; MINOR bullet key fixed |
| Review B M3 | 1 | M3 | b3feeb311945de92fbe0cf6cfd91c22104879d6b | a207cda5e89efb15af68529069367e1f8505f8bf:3ea92d58dd3fb8dfc67f520adffbc4aa13c48003 | CLEAN (MINOR flow-label note added to SKILL.md) |
| Review A M3 | 2 | M3 | b3feeb311945de92fbe0cf6cfd91c22104879d6b | 5433ce69c700d2689d8ce9926656b0d93aaa1af0:b3feeb311945de92fbe0cf6cfd91c22104879d6b | CLEAN (scoped, sonnet; 3 round-1 findings verified fixed, 1 MINOR empty Touches line, optional) |

## Red runs

M2 (before implementing, `bash scripts/tests/supervisor.test.sh`): `supervisor: 112 case(s), 45 failure(s)`.
- T014 map, retry, skipped, check-owner cases: `unknown subcommand map|retry|check-owner` (exit 2, want 0/1), failing for the missing subcommands.
- T016 watcher cases: `supervisor-watch.sh: No such file or directory` (exit 127), failing for the missing script.
- T018 skill-text cases: grep exit 1 (no *Restart*, `check-owner`, `supervisor.sh map`, `retry`, `list_sessions`, `run_in_background`, `supervisor-watch.sh` in SKILL.md).

M3 (before implementing, `bash scripts/tests/supervisor.test.sh`): `supervisor: 149 case(s), 29 failure(s)`.
- T019 touches/overlap cases: `unknown subcommand touches|overlap` (exit 2, want 0), failing for the missing subcommands; `next` hold cases fail because `next` ignores Touches.
- T021 skill-text cases: grep exit 1 (no *Touches* section, `gh issue view <n> --json body`, `supervisor.sh touches`, `ledger set <n> Touches`, "treated as disjoint" in SKILL.md).

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M3 merged in #1889 (squash f6805eb68) on 2026-11-14. Next: a fresh cloud thread runs `/speckit-autopilot resume #1813` and dispatches the M4 milestone unit (T022–T023, tier light, `model: "sonnet"`). That thread works on its own `claude/...` branch: merge this handover commit's branch into it, then set the **Worktree branch** line above to it. Carried optional MINORs: Review A M3 round 2, a body with an empty `Touches:` line before a populated one takes the empty one (grep -m1); Review A M2 round 2, SKILL.md Restart step 2 wording. Reviews ran as `autopilot-reviewer` subagents dispatched by the orchestrator; the orchestrator opened the PR and squash-merged via GitHub MCP. `scripts/gate.sh` still stops on the cloud-only red `a_signed_commit_succeeds_with_the_delivered_key`; units ran `scripts/tests/*.test.sh` by hand and CI was green.

## Open escalation

None.

## Follow-ups not done

- Full gate: all green except `micold-daemon session_extras::tests::a_signed_commit_succeeds_with_the_delivered_key` (signing, cloud env; no Rust touched). `scripts/tests/*.test.sh` ran individually, all green.
- T008: the supervised argument text lives in `rules/supervised.md` (SKILL.md is at its 60-line ceiling), linked from SKILL.md's route table.

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
