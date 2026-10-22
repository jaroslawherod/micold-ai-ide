# Autopilot ledger — #1210 worktree-session-guard

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1210 (issue #1210: Guard: stop sessions from working inside a worktree they created); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1210
- **Worktree branch**: feat/1210_guard-stop-sessions-from-working-inside-a-worktree
- **Started**: 2026-10-21
- **Phase**: milestone M2 (PR open)
- **Next step**: open the M2 PR; orchestrator waits on CI, merges, then M3

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1214 | Design | merged | 46582900eb258d140dd971d45c5d5e39e35131d3 |
| #1225 | M1 | merged | 575961879f7902b37fe9cbcfdd7f17be621bb12c |
| #1248 | M2 | open | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010, T028 | full | Edit/write guard + allowlist, deny-body channel proven | #1225 | merged |
| M2 | T011–T014 | full | Bash commands judged | #1248 | in review; red CI (windows path separators in shell.rs tests) fixed |
| M3 | T015–T017 | full | EnterWorktree and git worktree add denied | - | pending |
| M4 | T018–T020 | full | create_worktree hand-off text | - | pending |
| M5 | T021–T027 | full | Opt-out setting and toggle | - | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | tasks | Prefetch worktree name (plan flag) | No repo convention (grep of scripts, .claude, docs finds none); keep `autopilot-prefetch` prefix constant; spec assumption corrected | agent | research.md R5, T001 |
| 2 | tasks | JSON deny from HTTP hook unverified (plan flag) | T002 is the first task of M1; negative result escalates | agent | tasks.md T002 |
| 3 | tasks | US4 allowlist placement | Folded into M1 so the live guard cannot block target-shared writes or prefetch worktrees on main | agent | milestones.md rule 6 |

## Review rounds

- plan r1 (d3ad9a32...:d703bce5...): CHANGES, 2 MAJOR fixed. plan r2 (7fbf2284...:fe467967...): CLEAN (MINOR: version.rs, daemon_sync.rs not listed; covered by "grep it").

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 04953a8e0fcf81bad3b44bee76d980bf3a569b97:65cefbbe38d05d3054bce51ba4a2c4b3ab37a9e9 | CHANGES: 2 MAJOR (fixed) |
| Review spec | 2 | 41605b740bcbaa3d9d2d22ecdcae8ba555dd538f:65cefbbe38d05d3054bce51ba4a2c4b3ab37a9e9 | CLEAN |
| M1 review A (code-review high) | 1 | 86ee425da1e2cbbfebc38cb19d1b072940fe6fb6:c667802d37e322354801010490755f2daaeae0b1 | 4 fixed (cwd fallback, prefetch root, drive-like names, spawn_blocking), rest declined |
| M1 review B | 1 | cd5bfcf1f255e7ef932ace763cf9b56411e8122e:45a6426ca6d00e7f96b854a111ffe6810469f2fe | CLEAN (1 MINOR: no log-capture test for fail-open warn; behaviour covered by verdict tests) |

- tasks r1 (c3782371...:847ac3b5...): analyze fixed D1 (T028), A1, B1, E1, T2; reviewer CLEAN (3 MINOR fixed, prose/labels only).

## Declined review findings

- M1 review A: unlisted dir under `.claude/worktrees/` allowed as removed worktree (F1) — the spec's
  removed-worktree edge case requires it; create_worktree refreshes the cache.
- M1 review A: lexical `..` before symlinks (F4) — mandated by research R4.
- M1 review A: activity noted before a denial (F6) — the turn continues after a denial, so Working is right.
- M1 review A: stale binding after rename (F2), guard-map leak (F7), re-resolving the list per call (F10) — edge or negligible; rename-while-running noted as follow-up.
- M2 review A (snapshot aecaf022...:df514cb3...): CLEAN, 5 MINOR declined (best-effort per FR-004): prefix of globbed words, fd-prefixed redirects and `--opt=path`, comments/heredocs, cd after keywords/subshells, per-target re-resolution.
- M2 review B (sonnet, snapshot 12986681...:df514cb3...): CLEAN; Verify green (36 core, 5 daemon).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

- Red CI attempt 1 (run 39649966814): Windows "Report rust-cache hit" step exited 1 on a cache miss (`du target` under pipefail, no target dir); `|| true` added in ci.yml. Not caused by this flow. Full gate green locally (GIT_CONFIG_GLOBAL=/dev/null: host insteadOf breaks git_credential_grant test).
