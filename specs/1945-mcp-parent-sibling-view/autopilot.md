# Autopilot ledger — #1945 mcp-parent-sibling-view

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1945 (issue #1945: MCP: let a session see its parent session, parent worktree and siblings, and message its parent; scope items 1-5 per the issue body); labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1945
- **Worktree branch**: claude/project-thread-boi1v0
- **Started**: 2026-11-15
- **Cloud notes**: units have no Agent tool, so the orchestrator dispatches reviews A (code-review high, opus) and B (conformance, sonnet) itself and sends findings back; merge each PR by MCP squash with expectedHeadSha after `ci complete` is green (jaro approved merge-on-green for this run); `mutants on the diff` fails at baseline on unrelated netns tests (advisory, commented on #1965); `diff coverage` (80%) is required, so cover error paths. Disk is short: wipe target-shared/.
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1947 | design | MERGED | 00f66d534eedc83b1c08c7614e50f5a32194eaa8 |
| #1965 | M1 | MERGED | 2d713bcf7f9db032cfd15e828d4e4ead7a81fc2a |
| #1978 | M2 | MERGED | 1fb7d00e222b74a70a98bc45cc138defe4aa24b8 |
| #1988 | M3 | MERGED | 5160c889a3369f1eab532cf92f9031b57ab334a9 |
| (pending) | close | OPEN once the orchestrator opens it | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T005, T016, T017 | full | Unparented sessions see default and siblings | #1965 | merged |
| M2 | T006–T014 | full | neighbourhood and send_parent_input tools | #1978 | merged |
| M3 | T015, T018–T020 | full | Parent recorded before start; agents told | #1988 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Unparented worktree visibility | Treated as child of `default` for visibility only, no link recorded | user (jaro, 2026-11-15) | issue #1945 scope item 1 |
| D2 | implement M3 | A worktree directory literally named `default` collides with the root's ref `default` | Reserve the name: refuse creating a worktree named `default` (any create path, clear error naming the reservation), document it in the user guide; an existing dir named `default` keeps M2 behaviour (excluded from the neighbourhood) and the docs say so. Safest: no output format or ref changes, smallest diff; the follow-up MINORs (b),(c) are then moot for new worktrees | user (jaro, decision card 2026-11-15) | ledger Follow-ups, M2 review A r2 F2 / r3 |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec.md | 214f266cddafb8d31574e169e83779f56cf41864 | 45981247472c178d3a281f97d0df0d2ea12c268b:214f266cddafb8d31574e169e83779f56cf41864 | CHANGES: 3 MAJOR, 2 MINOR (all fixed in 78ada4fd) |
| Plan | 1 | plan.md, research.md, data-model.md, contracts | b0980ab565139b251fc87a9497dd36449c28edc0 | 7c4851301a2b528fc4f864afac018e9f8c408791:b0980ab565139b251fc87a9497dd36449c28edc0 | CHANGES: 4 MAJOR, 1 MINOR (fixed; round 2 by orchestrator) |
| Plan | 2 | plan.md fix diff (sonnet, scoped) | 9422046fe | - | CLEAN, 2 MINOR (stub snapshots the link into its marker; quickstart step 2 adds mcp_cross_session; applied by tasks unit) |
| Tasks | 1 | tasks.md, milestones | ee81b0c00822cfb84eb4da4e069425cb28d221cb | 1661122c6c63da6343b641227e009ff7efcf2f0b:ee81b0c00822cfb84eb4da4e069425cb28d221cb | CHANGES: 1 MAJOR, 3 MINOR (fixed; scoped re-review by orchestrator) |
| Spec | 2 | spec.md (fix diff, sonnet) | 78ada4fd75dd1ca8e7c8ff1aebf16e466acde566 | - | CLEAN, 1 MINOR (FR-006 sentence; fixed in clarify unit) |
| M1 review A | 1 | code-review high | e8d0eb7b6fe8746ec74aa98cd05f9e977e000c6d | 428ab0399774fef30f66d8ad6cc66d69dcfa8650:e8d0eb7b6fe8746ec74aa98cd05f9e977e000c6d | CHANGES (A-F1..F4; fixed in the next M1 commit, T016/T017 pulled in) |
| M1 review B | 1 | conformance | e8d0eb7b6fe8746ec74aa98cd05f9e977e000c6d | 428ab0399774fef30f66d8ad6cc66d69dcfa8650:e8d0eb7b6fe8746ec74aa98cd05f9e977e000c6d | CHANGES (B-F1..F3; cycle-log, drift-M1, implicit loop fixed) |
| tasks | 2 | design | a3392c6f602d1ccf19b73576fce51cddfc115fb5 | 5304203d510eef87bf8351eb34b6d456f161cfe9:a3392c6f602d1ccf19b73576fce51cddfc115fb5 | CLEAN (sonnet, scoped) |
| A | 2 | M1 | e265b36a5efb795a3e5b5f4ddea2878008c05b85 | b04e7ce9281b10af5b6f11587a6984cf9c8ff8d7:e265b36a5efb795a3e5b5f4ddea2878008c05b85 | CLEAN (sonnet, scoped; MINOR: no-parent create no longer retires stale ids, carried to M3) |
| B | 2 | M1 | e265b36a5efb795a3e5b5f4ddea2878008c05b85 | b04e7ce9281b10af5b6f11587a6984cf9c8ff8d7:e265b36a5efb795a3e5b5f4ddea2878008c05b85 | CLEAN (sonnet, scoped; MINOR M3 range fixed) |
| A | 3 | M1 | 456b7be22340360facd6754ccd05e84bc3352ff3 | 8220a2e6e6b532e0ee4f0b4c8cf6b69b797ae483:456b7be22340360facd6754ccd05e84bc3352ff3 | CLEAN (sonnet, scoped CI fix: link-failure tests) |
| A | 1 | M2 | 45c59a68b | 9d35f00a:45c59a68b | CHANGES: 2 MAJOR, 3 MINOR (F1 shell target, F2 dir named `default` as parent, F3 own dir `default`, F4 audit target, F5 double resolve; all fixed in the next M2 commit) |
| B | 1 | M2 | 45c59a68b | 9d35f00a:45c59a68b | CLEAN (1 MINOR: cycle 10 red by mutation, no fix needed) |
| A | 2 | M2 (scoped, 45c59a68b..87425df21) | 87425df21 | - | CHANGES: 1 MAJOR, 3 MINOR (F1 scope keyed by the ref `default`: fixed, root kept apart from dir names; F3 shell-only parent wording: fixed; F4 scope check on the parent target: fixed; F2 dir named `default` excluded from the neighbourhood: not changed, carried to M3) |
| A | 3 | M2 (scoped, 87425df21..e8805a8c4) | e8805a8c4ff759a20cbe0f614db8520ff3cde832 | - | CLEAN (sonnet, scoped; 3 MINOR carried to M3: shell-only wording, list_resumable_sessions filter and output refs collapse root vs dir named default) |
| A | 1 | M3 | 122c0c61d68d84c7bab4dd73daeb5588b897f1c0 | fd1153ec754a24b7edd5b870520a5ed52ccf7fae:122c0c61d68d84c7bab4dd73daeb5588b897f1c0 | CHANGES: 1 MAJOR, 2 MINOR (F1 T015 test could not tell before from just after: rewritten with a slow store and a stand-in that reads the saved link first, red by moving the record; F2 no-parent record is log-only, `create_unparented_worktree` for the app and runs; F3 worker prompt line names `send_parent_input`) |
| B | 1 | M3 | 122c0c61d68d84c7bab4dd73daeb5588b897f1c0 | fd1153ec754a24b7edd5b870520a5ed52ccf7fae:122c0c61d68d84c7bab4dd73daeb5588b897f1c0 | CLEAN, 2 MINOR (cycle 16 red: recorded; T015 path: fixed) |
| A | 2 | M3 | 29dd0fa023c9527f60f55b7238922a8d3e6ed453 | - | CLEAN (sonnet, scoped 122c0c61d..29dd0fa02; 1 MINOR cycle-log count fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Cloud orchestrator thread handed over after M2 merged (2026-11-15, PR #1978, squash 1fb7d00e2; cloud rule: fresh thread per milestone). Next: M3 (T015, T018–T020), then close. M3 should also decide the `default`-named-directory follow-ups below. Cloud notes: units have no Agent tool, so the orchestrator dispatches review A (code-review high, opus; re-rounds sonnet) and review B (conformance, sonnet) and sends findings to a fresh part unit; merge by MCP squash with expectedHeadSha after `ci complete` is green (jaro approved merge-on-green for this run); `mutants on the diff` fails at baseline on netns tests (advisory, commented on #1965 and #1978); `diff coverage` (80%) is required. nextest must be installed per thread from the GitHub release.

## Open escalation

None.

## Follow-ups not done

None. Settled in M3: the name `default` is reserved (D2), so review A round 3 MINORs (b) and (c) (`list_resumable_sessions` filter and output refs that collapse the root and a directory named `default`) are moot for new worktrees; a hand-made pre-existing `default` directory stays excluded from the neighbourhood and is documented as such. The no-parent create retires a stale id again (ops.rs), the "only shell sessions" wording is fixed, the `send_parent_input` scope check is a `debug_assert!`, and drift-M2.md has its table header.
