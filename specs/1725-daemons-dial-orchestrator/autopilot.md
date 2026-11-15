# Autopilot ledger — #1725 daemons-dial-orchestrator

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1725 "feat: daemons dial the orchestrator (host directly; container, SSH and pod through the reverse forward)" (part of #1722; covers FR-003, FR-010; blocked by #1724, done). labels: flow:feature, effort:high
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1725
- **Worktree branch**: claude/project-thread-t27ol0
- **Started**: 2026-11-15
- **Phase**: milestones
- **Next step**: M4 review B (orchestrator, sonnet) and review A round 2 on the F1 fix diff (snapshot in Review rounds), then open the PR from pr-title.md/pr-body.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1969 | design (spec, plan, tasks) | merged (squash) | 1d36b93ffb1aef3d5c470eaea25454a384126984 |
| #1971 | M1 host link | merged (squash) | 8921b61679a59818fde86a852ec1a87cf7a86d37 |
| #1975 | M2 outage + bounded sends | merged (squash) | 1e9b379e3c749ce740aefcf0bd6a118991b3506a |
| #1976 | M3 backoff + link status | merged (squash) | 2880eeb17 |
| #1979 | M4 reverse forward | merged (squash) | bc0a44f22 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | Host daemon links to the local orchestrator (US1; 15 tasks: setup and foundational cannot be split off, nothing observable) | #1971 | merged (squash 8921b6167) |
| M2 | T016–T020 | full | Sessions survive an orchestrator outage; handshake sends bounded (US4) | #1975 | merged (squash 1e9b379e3) |
| M3 | T021–T027 | full | Refusals back off; link status observable (US5) | #1976 | merged (squash 2880eeb17) |
| M4 | T028–T037 | full | Container, SSH-host and pod daemons link through a reverse forward (US2) | pending (branch claude/project-thread-t27ol0) | implemented; review A round 1 fixed; gate green but for env-flaky script tests; awaiting review B + A round 2, PR |
| M5 | T038–T040 | full | Drop shows offline, reopen restores online (US3) | | pending |

## Decisions

- Tasks unit: speckit-analyze HIGH C1/C2 (who sets `enabled`) resolved as an Assumption in spec.md (user/provisioning, #1726/N11); C3 jitter added to FR-006/FR-007; MEDIUMs C4-C11 fixed in tasks.md/plan.md.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
- M4: a user-config host whose `ssh -G` shows a configured forward, or fails, fails the open with a message (tasks.md T028), not "connect without the link" (research R2, plan ledger line). Review A F2 flagged that this stops SSH hosts with a `LocalForward`/`RemoteForward`/`DynamicForward` from connecting through the app. Kept: T028 is the later, more specific artifact and FR-005 says the connection never runs silently without the forward. Reversible in one `match` in `provision::admit_link`; the orchestrator may ask the user.

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
| Review A | 1 | M3 | 11cf6ef6aacaa53f201f8e063e94ea6e94dcdaeb | b6c2d388a5ead6484138edc3efa314604677c7e0:02fd9250fb4dd000800238ca044234da717f8a4b | MINOR-ONLY: CLEAN with 3 MINOR (refused-to-absent change not logged, spawn_as public seam, SC-004 log test could not catch a per-retry line); all fixed in 11cf6ef6a, no new round (MINOR only) |
| Review B | 1 | M3 | 7f09644ad | HEAD | CLEAN (orchestrator, sonnet): 1 MINOR accepted (T022 never red; cycle 10 records mutation checks); Verify 7+53 passed, no plan drift |
| Review B | 1 | M4 | 3cb569d6ab1f015c24d50888a2849667dd040d3a | cfb92da73b2f574658db5d0f3f4b07ba8ab15897:3cb569d6ab1f015c24d50888a2849667dd040d3a | CLEAN (orchestrator, sonnet; also review A round 2 of the F1 fix): 1 MINOR declined (no ssh_real case with a user ClearAllForwardings yes; argv unit test covers it); Verify 4+52 passed, no plan drift |
- M4 review A round 1 (opus, headless, snapshot 5d226ed04ff2b56679eb538fb78df9e077fde4a1:e7dbd2a39a4bba360d2f05781b9e794843ecd9a0): CHANGES, 2 MAJOR, 2 MINOR. F1 fixed (`ClearAllForwardings=no` on the link, argv test); F2 fixed, prose only (research R2 marked superseded by T028; behaviour kept, see Decisions); F3 fixed (test renamed, asserts `Prepare`); F4 declined. Fix diff is code (F1): round 2 left to the orchestrator with review B (sonnet).

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | Spec review 1 | none | all six findings fixed |
| M2 | Review B 1 | F1 MINOR: outage tests' session/client half has no red mutant | log states it; the behaviour predates M2 and the test is a regression guard |
| M2 | Review B 1 | F2 MINOR: `pub struct Detached` is test-only API | recorded in drift-M2.md; daemon crate is internal, no public API surface |
- M4 A F4 (second `ssh -G` per user-config connection): declined; it only reads files under a 5 s bound, and carrying the first answer through `ProvisionOptions` couples the timeout probe to the link; not worth the coupling.

## Handover

2026-11-15, M4 thread (branch claude/project-thread-t27ol0): PR #1979 squash-merged (bc0a44f22) on green `ci complete` after one fix: diff coverage (80%) failed because the `prepare-link` subprocess run is invisible to lcov; the command was split into `prepare_link_command` (resolver injected) and tested in process. Review A round 1 CHANGES (fixed), review B + round 2 CLEAN, 1 MINOR declined. Mutants (advisory) cancelled/red as before. Behaviour decision for jaro: SSH hosts with their own forwards, or a failing `ssh -G`, now fail to connect with a message (T028; revert is one match in `provision::admit_link`). Next: M5 (T038–T040) in a fresh thread.
2026-11-15, M4 unit (branch claude/project-thread-t27ol0, previous PR #1976 merged at 2880eeb17): T028–T037 done and ticked; cycles 11–14 and review A fixes in tdd/cycle-log.md; drift-M4.md written, plan-drift clean. Review A round 1 (opus, headless) CHANGES: F1 fixed in code (`ClearAllForwardings=no`), F2 prose + Decisions (T028 fail-the-open kept; may need the user), F3 fixed, F4 declined. Round 2 of A on the F1 fix not run by the unit. ssh_real_link 4 passed against a real sshd installed in the sandbox (`apt-get install openssh-server`, `mkdir /run/sshd`). Full gate: Rust green; scripts/tests/autopilot.test.sh flaky from sandbox git (`push negotiation failed`), left to CI. PR not opened: pr-title.md/pr-body.md ready, branch pushed. Next: review A round 2 + review B, then open the PR.

2026-11-15, M3 unit (branch claude/project-thread-2igsm0, previous PR #1975 merged at 1e9b379e3c749ce740aefcf0bd6a118991b3506a): T021–T027 done and ticked; cycles 8–10 and review A fixes in tdd/cycle-log.md; drift-M3.md written, plan-drift clean. Review A (opus, headless) CLEAN with 3 MINOR, all fixed in 11cf6ef6a. Not run by the unit (orchestrator runs them): review B, visual pass (nothing visible changed). PR not opened: pr-title.md/pr-body.md ready, branch pushed. Next: review B on origin/main...HEAD, then open the PR.
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
