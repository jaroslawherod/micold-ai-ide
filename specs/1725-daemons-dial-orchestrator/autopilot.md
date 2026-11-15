# Autopilot ledger — #1725 daemons-dial-orchestrator

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1725 "feat: daemons dial the orchestrator (host directly; container, SSH and pod through the reverse forward)" (part of #1722; covers FR-003, FR-010; blocked by #1724, done). labels: flow:feature, effort:high
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1725
- **Worktree branch**: claude/project-thread-2igsm0
- **Started**: 2026-11-15
- **Phase**: milestones
- **Next step**: milestone M3 (T021–T027), in a fresh thread via `/speckit-autopilot resume`

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1969 | design (spec, plan, tasks) | merged (squash) | 1d36b93ffb1aef3d5c470eaea25454a384126984 |
| #1971 | M1 host link | merged (squash) | 8921b61679a59818fde86a852ec1a87cf7a86d37 |
| #1975 | M2 outage + bounded sends | merged (squash) | 1e9b379e3c749ce740aefcf0bd6a118991b3506a |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | Host daemon links to the local orchestrator (US1; 15 tasks: setup and foundational cannot be split off, nothing observable) | #1971 | merged (squash 8921b6167) |
| M2 | T016–T020 | full | Sessions survive an orchestrator outage; handshake sends bounded (US4) | #1975 | merged (squash 1e9b379e3) |
| M3 | T021–T027 | full | Refusals back off; link status observable (US5) | | pending |
| M4 | T028–T037 | full | Container, SSH-host and pod daemons link through a reverse forward (US2) | | pending |
| M5 | T038–T040 | full | Drop shows offline, reopen restores online (US3) | | pending |

## Decisions

- Tasks unit: speckit-analyze HIGH C1/C2 (who sets `enabled`) resolved as an Assumption in spec.md (user/provisioning, #1726/N11); C3 jitter added to FR-006/FR-007; MEDIUMs C4-C11 fixed in tasks.md/plan.md.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec review | 1 | spec | c5c4ee9eea8c5213009bd9e31ae289e13de2449d | 6580b0d5f69f5d16e0d94c289720723d0f9858a2:c5c4ee9eea8c5213009bd9e31ae289e13de2449d | CHANGES: 3 MAJOR fixed (pod path after N11 is mTLS, PROTOCOL_VERSION 82, FR-005 vs Q2 socket), 3 MINOR fixed; fixes are claim changes, not re-reviewed beyond self-check |
| Spec review | 1 | spec | c5c4ee9eea8c5213009bd9e31ae289e13de2449d | ce951a8ddb4e9f3252212e2563fb06d8ede5cca9:9f5fe99c4d2d09674f417cac5a47b1da6cfaa105 | CHANGES: 3 MAJOR, 3 MINOR, all fixed in 9f5fe99c4 (snapshot taken after the fixes) |
| Spec review | 2 | spec fix diff | 9f5fe99c4d2d09674f417cac5a47b1da6cfaa105 | ce951a8ddb4e9f3252212e2563fb06d8ede5cca9:9f5fe99c4d2d09674f417cac5a47b1da6cfaa105 | CLEAN: round-1 fixes hold (scoped, sonnet, diff against c5c4ee9ee) |
| Plan review | 1 | plan | 51d50295e396f5546030a0ebf185e54f3e96e8b6 | d346f6bd5384d30c245219fa412dd36f021b470f:edb515ca275e9528697839235b08037de5e4f1e7 | CHANGES: 4 MAJOR (ClearAllForwardings scope, stale live socket, encode_announcement NEW, prepare-link app side), 2 MINOR; all fixed |
| Plan review | 2 | plan fix diff | 51d50295e396f5546030a0ebf185e54f3e96e8b6 | f357f1489081de54ce559264732119f2099ffded:edb515ca275e9528697839235b08037de5e4f1e7 | CLEAN (scoped, sonnet); one MINOR comment fixed, prose only |
| Tasks review | 1 | design | 4e9b7e7f8147a6ca39abdb4a0aed819f5efedbb4 | 4efebc3b7d308f99107399a0d9913f9b32676e64:c30e1ae59e0d06bb201b6b3de8d70a51e5540c91 | CHANGES: 5 MAJOR (test wiring, ssh_real feature flag, Verify filters, FR-002 test, needs-update case), 3 MINOR; all fixed |
| Tasks review | 2 | design fix diff | 4e9b7e7f8147a6ca39abdb4a0aed819f5efedbb4 | 4efebc3b7d308f99107399a0d9913f9b32676e64:c30e1ae59e0d06bb201b6b3de8d70a51e5540c91 | CLEAN (scoped, sonnet) |
| Review A | 1 | M1 | 33282fd3e765197a84e2a71662bfbe17495b266d | b3bf65678531ba19356823ba4f311526cffc200c:33282fd3e765197a84e2a71662bfbe17495b266d | CHANGES: 1 MAJOR (no test of server::run wiring), 3 MINOR; all fixed |
| Review A | 2 | M1 | e409f3f94ab32375a0471df628d084758dbe070c | 7dd654c35687f155ee415a77652480b8e758db7a:9b353509ca3082b429e161fcd3ea3bbfae7cbc68 | CLEAN (fix diff, sonnet); later commit only skips the pure dial rules in the no-outbound scan (no_outbound.rs, docs) |
| Review B | 1 | M1 | 8011bd2ae1e7423af4419b4addb7eaa2f4f8a8c1 | 1f0940a172cbebd59b6fb65d90ab571ce84caf61:8011bd2ae1e7423af4419b4addb7eaa2f4f8a8c1 | CHANGES: 2 MAJOR drift records, 1 MINOR test; all fixed |
| Review B | 2 | M1 | d7a3d1066f8747ae9388e8609854f959a72baea6 | f90edcff30b590fb73aed5b450becf8325d1114b:d7a3d1066f8747ae9388e8609854f959a72baea6 | CLEAN (fix diff, sonnet) |
| Review A | 1 | M2 | 2e31bdcd1b62b43879df3a1a9afa37fb0f88858f | bb26161ef2fa2e86022fc2cbb791aab111384099:a199b3c482dad6e2e2c35e8a607dcf758f2ff756 | MINOR-ONLY: CLEAN with 3 MINOR (redundant claim_failed, Detached comment and duplicated set-up, thread-local capture in host test); all fixed, no new round (MINOR only) |
| Review B | 1 | M2 | d120a5c13b499fc7f04b3d9a923813229864c532 | 43ea938da69a7ebd31566488c5881cbcc07b885c:d120a5c13b499fc7f04b3d9a923813229864c532 | MINOR-ONLY: 2 MINOR declined (orchestrator, sonnet); Verify 7 passed, no plan drift |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | Spec review 1 | none | all six findings fixed |
| M2 | Review B 1 | F1 MINOR: outage tests' session/client half has no red mutant | log states it; the behaviour predates M2 and the test is a regression guard |
| M2 | Review B 1 | F2 MINOR: `pub struct Detached` is test-only API | recorded in drift-M2.md; daemon crate is internal, no public API surface |

## Handover

2026-11-15, M2 thread (branch claude/project-thread-itbg1f): PR #1975 squash-merged (1e9b379e3) on green `ci complete`. Review A CLEAN (3 MINOR fixed), review B (orchestrator, sonnet) MINOR-ONLY, 2 MINOR declined. Mutants (advisory) red again from the job's environment, not the diff: baseline failed on netns test (`unshare -rn` unavailable); on M1 it was documentation_is_not_read (no .git in the copy). Reported to the coordinator for a chore. origin/main merged back; diff vs main is ledger only. Next: M3 (T021–T027, tier full) in a fresh cloud thread running `/speckit-autopilot resume` (update the Worktree branch line to that thread's branch). Unit took ~250k tokens over ~73 min in one part.

2026-11-15, M1 thread: PR #1971 squash-merged (8921b6167) on green `ci complete` (mutants advisory, red). Review B (orchestrator, sonnet) round 1 CHANGES (2 MAJOR missing drift records, 1 MINOR I/O-test scope; all fixed in d7a3d1066), round 2 CLEAN. origin/main merged back; diff vs main is ledger only. Next: M2 (T016–T020, tier full) in a fresh cloud thread running `/speckit-autopilot resume` on this branch (update the Worktree branch line if the thread's branch differs).

Cloud notes: same as the design handover. Units have no Agent tool: they run review A headless (`claude -p "<prompt>" --agent autopilot-reviewer --model <m> --permission-mode auto --allowedTools "Read,Grep,Glob,Bash(git *),Bash(grep *),Bash(ls *)"`); the orchestrator runs review B (sonnet, rubrics/conformance.md) and a sonnet worker for its fixes. Brief units to record every UNPLANNED path (tests included) in drift-M<K>.md and run plan-drift.sh. Full gate needs `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`; cargo-nextest and openssh-client may need installing. Open the PR via GitHub MCP from pr-title.md/pr-body.md (attribution block first), draft=false, wait on `ci complete` via `gh api .../commits/<head>/check-runs`, squash with expectedHeadSha, then `git merge origin/main` and check the diff is ledger only.

## Follow-ups not done

- CI "mutants on the diff" is red for reasons outside this flow: on PR #1971 the unmutated baseline failed (exit 4, no mutant tested) because cargo-mutants copies the tree without `.git`, so `micold-core --test main documentation_is_not_read` (`git check-attr`) fails. Likely fix: `--copy-vcs true` in `scripts/mutants-diff.sh`. Reported to the coordinator 2026-11-15 (M2 thread).

## Open escalation

None. Plan mismatch with the revised epic (spec.md, "Plan mismatch") changes no goal; recorded, not escalated.

## Plan decisions (unit Plan 1725)

- sshd (packaging/basic): Unix-socket remote forward already allowed (AllowStreamLocalForwarding defaults to yes); plan adds `AllowStreamLocalForwarding remote` + `StreamLocalBindUnlink yes`. Without it: ssh fails the open with its message, no TCP fallback.
- Forward rides the bridge `ssh`; `remote prepare-link` (new) makes the remote dir, unlinks the old socket, prints the path. User-config hosts with configured forwards skip the link (ClearAllForwardings cannot be scoped).
