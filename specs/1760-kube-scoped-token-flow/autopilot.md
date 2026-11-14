# Autopilot ledger — #1760 kube-scoped-token-flow

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1760: Scoped-token step 11: assert the credential flow in a kind (Kubernetes) pod; labels: none originally, now flow:feature (switched from chore)
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1760
- **Worktree branch**: claude/project-thread-c875n9
- **Started**: 2026-11-12
- **Phase**: milestones
- **Next step**: M2 PR (orchestrator: Review B, visual pass quickstart §B, CI, merge), then M3

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1870 | design (spec, plan, tasks) | merged | 55e614ed76ba667e09c0ac9649abc04114f93393 |
| #1914 | M1 | merged | 17284c645186eeb705ba403c114ee757f3ea3d53 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017, T020 | full | tokens delivered to pod Secret and daemon.toml behind the opt-in (setting only via daemons.json) | #1914 | merged |
| M2 | T018–T019 | light | form tick and user guide | pending PR | built, awaiting B, visual pass and CI |
| M3 | T021–T027 | full | kube_real_scoped_token and forge harness | - | pending |

## Decisions

- FR-008: per-daemon opt-in for delivering GitHub tokens into a cluster Secret, off by default (decided by user, clarify round 1).
- FR-009, FR-010: agent-resolved from the repo (see spec Clarifications).

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec review | 1 | spec.md | c1af633993e0d08fce04690906bf0074fe12df1d | dc49940e4d330da0d81222db5812aa2e88ce4a13:c1af633993e0d08fce04690906bf0074fe12df1d | CHANGES: 1 MAJOR declined (open markers go to clarify), 3 MINOR fixed |
| Plan review | 1 | plan.md, research.md, data-model.md, contracts | 2ea1951e53ce0daa188acf55cc01f06b058a2534 | e5bec1b3666ceed4478d5c774ad949d9017a7fac:2ea1951e53ce0daa188acf55cc01f06b058a2534 | CHANGES: 2 MAJOR fixed (runtime-project source narrowed to saved projects; KubeEnv.tokens seam), 3 MINOR fixed |
| Plan review | 2 | fix diff | 2ea1951e53ce0daa188acf55cc01f06b058a2534 | fbbd354fff65ccc938db0d8d610033b6124a24d6:2ea1951e53ce0daa188acf55cc01f06b058a2534 | CLEAN (3 MINOR: 1 fixed, 2 left for tasks) |
| Plan review | 2 | plan fix diff | 44e5569db6c05e6b2c3da07cd58d488222b8f023 | fbbd354fff65ccc938db0d8d610033b6124a24d6:2ea1951e53ce0daa188acf55cc01f06b058a2534 | CLEAN |

| Tasks analyze | 1 | tasks.md | - | - | 1 HIGH (arm C vs AS3) and 4 MEDIUM (task order, M1 docs vs M2 tick, mount vs home, test-first notes) fixed; prose/task-list only |
| Tasks review | 1 | tasks.md, milestones | f8a94ddf6 | 893ed26f20eb272e8a1574de8cdaf6fe034c8fdd:f8a94ddf620220ccd11a592b6644bb6b2a5a3d57 | CHANGES: 1 MAJOR fixed (user-guide task T020 moved to M1), 2 MINOR fixed; prose/task-list only, no re-review |
| Tasks review | 1 | tasks.md, milestones | 240eae3bb528e9fe0d018c9d4b501bcdbf124c8c | 893ed26f20eb272e8a1574de8cdaf6fe034c8fdd:f8a94ddf620220ccd11a592b6644bb6b2a5a3d57 | CHANGES: 1 MAJOR fixed, 2 MINOR fixed; prose/task-list only |
| Review A (code-review high) | 1 | M1 | c5fb01da75a62b53009826c1d649440184d47587 | 127f0833ef54e6b40f7306db53653abf0ea414a0:3c7c46c82761f64a23599beeaab880b4ce202b67 | CHANGES: 3 MAJOR fixed (F1 key per token name, F2 SecretToken kept out of PodStartup, F3 no projects dial while off), 3 MINOR: F4 comment fixed and retry behaviour declined, F5 Secret total checked, F6 shared checks; round 2 pending |
| Review B (conformance) | 1 | M1 | c5fb01da75a62b53009826c1d649440184d47587 | 127f0833ef54e6b40f7306db53653abf0ea414a0:3c7c46c82761f64a23599beeaab880b4ce202b67 | CHANGES: F1 MAJOR fixed (drift-M1.md), F2 MINOR fixed (PR body moved out of the tree); round 2 (Verify) pending |
| Review A | 2 | M1 | afc83aba87b0ffac0ffc2b88d92285c92a893f6f | bd50b71e8142636b177dd984f110c553e105c671:afc83aba87b0ffac0ffc2b88d92285c92a893f6f | CLEAN (all 6 round-1 findings fixed; F4 retry behaviour declined) |
| Review B | 2 | M1 | afc83aba87b0ffac0ffc2b88d92285c92a893f6f | bd50b71e8142636b177dd984f110c553e105c671:afc83aba87b0ffac0ffc2b88d92285c92a893f6f | CLEAN (1 MINOR: stale handover path, fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | spec review r1 | F1 MAJOR: three open markers (FR-008..010) leave scope unsettled | They are the template's allowed three; the rubric sends each to clarify. FR-008 is escalated there. Fixed F2, F3, F4 (prose/edge case). |
| M1 | Review A r1 | F4 MINOR (behaviour part): a token-read failure on an established link retries with back-off instead of a terminal StartFailed | The token file was readable when the link was made; a retry picks up a file the user fixes (chmod 600, a new token) with no action, and each failed attempt shows the reason naming the token. A terminal StartFailed would need a manual restart for a transient or fixed problem. The comment is corrected. |

## Handover

2026-11-14 M2 unit (cloud, branch claude/project-thread-c875n9): previous PR #1914 merged at 17284c645186eeb705ba403c114ee757f3ea3d53. T018–T019 built: tick in the Credentials section after Allowed credentials, `Msg::GithubTokensToggled`, `FieldId::DaemonKubeGithubTokens`, geometry and reducer tests, user guide wording. Targeted tests, fmt, clippy (client, all targets) green. Review A (self, high, small diff): no findings. Still needed from the orchestrator: Review B (conformance, sonnet), the visual pass (quickstart §B), push/PR (gh unavailable; PR body at the unit's scratchpad pr-body-m2.md), CI, merge.
2026-11-14 M1 orchestrator (cloud): Review A and B round 2 CLEAN at afc83aba8; PR #1914 squash-merged (17284c645186eeb705ba403c114ee757f3ea3d53) on green CI (all checks, incl. the real kube cluster job). main merged back into claude/project-thread-myub4h. Next: a fresh thread runs `/speckit-autopilot resume #1760` and dispatches the M2 milestone unit (T018–T019, Tier light, model sonnet) on this branch; previous PR #1914, merge SHA above.

2026-11-14 M1 fix unit (cloud): Review A r1 (6 findings) and Review B r1 (2) acted on in c5fb01da7: all fixed but F4's retry behaviour (declined, see table). Targeted tests, fmt and clippy (core, client) green. PR body updated at the unit's scratchpad (outside the tree, Review B F2). Next: round 2 of A and B on the fix diff from snapshot 127f0833e:3c7c46c82, then open the M1 PR.
2026-11-14 M1 unit (cloud): previous PR #1870 merged at 55e614ed76ba667e09c0ac9649abc04114f93393. T001–T017 and T020 built test-first (tdd/test-list.md, tdd/cycle-log.md), committed and pushed on claude/project-thread-myub4h; PR body kept out of the tree (orchestrator scratchpad). Next: orchestrator runs Review A and B, the unit fixes findings, then the PR opens.
2026-11-14: design PR #1870 squash-merged (55e614ed7) on green CI. Next: a fresh thread runs `/speckit-autopilot resume #1760` and dispatches the M1 milestone unit (T001–T017, T020) on this branch.

2026-11-14 takeover (cloud thread): main merged in; plan re-checked against #1838/PR #1886. That fix changed only the daemon's client-token default (`CONTAINER_TOKEN_PATH` = `/run/micold/token`); this plan's GitHub token files live under `STARTUP_ENV_MOUNT` (`/run/micold/secrets/startup`, unchanged on main) and all anchors it names (`kube_attempt`, `POD_FILE_LIMIT`, `PodStartup`, `StartRequest::secret_files`, `GithubTokenSetting::from_store`, `KubeEnv`, `parse_token_file`) still exist. No spec/plan change needed.

Earlier: tasks, milestones, analyze fixes and the tasks review are committed (HEAD 9562dc383+). Not pushed: gate-hook refuses push until `mise run gate` is green (the classifier denied hand-recording the tree). A detached `mise run gate` was started, log `$SCRATCHPAD/gate.log` of the previous unit (/tmp/claude-1000/.../c47cee65-.../scratchpad/gate.log), queued behind another worktree's build lock. Next: when GATE_EXIT=0 (or rerun `mise run gate` detached), `git push --force-with-lease -u origin HEAD`, open the design PR (title `docs(1760): specify and plan kube scoped token flow (#1760)`, body ends `Refs #1760`), record its number in the ledger.

## Open escalation

None.

## Follow-ups not done

- Spec prose (tasks.md T020, data-model, plan, research R2) calls the store `daemons.json`; there is no such file: the daemon registry is the `daemons` list of `settings.json`. The user guide (T020) names `settings.json`; the spec artifacts were left as they are.
- docs/user-guide/sandboxed-daemon.md promises a sandbox pushes with a pasted token, but no client code delivers `[[github.tokens]]` to a container (deliver_container) either; separate follow-up.
