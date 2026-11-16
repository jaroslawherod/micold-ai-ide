# Autopilot ledger — #1726 app-via-orchestrator

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1726 "feat: the desktop app talks only to the orchestrator (aggregated lists, routed ops and streams, starts it when absent)" (part of #1722; covers FR-005, FR-006, FR-007, FR-010; blocked by #1725, closed). Cloud overrides: no mise, gh unauthenticated, gate scripts/gate.sh, GitHub MCP for PRs; desktop change minimal (iced app sunsetting), prefer orchestrator/protocol-level tests; aim 3-4 milestones. labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1726
- **Worktree branch**: claude/project-thread-sn2sdc
- **Started**: 2026-11-16
- **Phase**: done
- **Next step**: handoff
- **PR**: close PR pending (opened by the orchestrator from pr-close.md)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1996 | design | merged | 9f6e6784d |
| #1999 | M1 | merged | 2f62cbf31 |
| #2004 | M2 | merged | 5ee726f07 |
| #2007 | M3 | merged | f6aa5b806 |
| #2012 | M4 | merged | d4e51acab |
| close | close | pending | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | contract, core, requests and streams routed through the orchestrator | #1999 | merged |
| M2 | T014–T016 | full | aggregated lists, 2 s deadline, SC-003 bench | #2004 | merged |
| M3 | T017–T022 | full | desktop app uses one orchestrator connection (host only) | #2007 | merged |
| M4 | T023–T034 | full | SSH, container and Kubernetes daemons routed through the orchestrator (FR-007 in full) | #2012 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-002 per-daemon list deadline | 2 s, daemons asked in parallel | agent | SC-004 budget |
| 2 | clarify | Who writes `[orchestrator_link]` into provisioned daemons | this feature, the app on provision (FR-013) | agent | specs/1725-daemons-dial-orchestrator/spec.md#Assumptions |
| 3 | tasks | M1 has no observable user story (milestones rule 1) | superseded by #4: contract+core merged into the US2 milestone | agent | tasks review r1 F1 |
| 4 | tasks | Milestones bundling two stories (rule 2) | keep combined, reasons in each milestone's Note (US4 halves and US5 have no code of their own) | agent | tasks review r1 F4 |
| 5 | plan (M4) | Provisioned daemons on non-Linux clients | Linux only, #1897: no exception, FR-007 holds for every placement | user (plan r1 F1) | #1897 |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| spec | 1 | design | 7c25a28d9e4bec1417923f0dd050b8627f79bebd | f3b201b3e2dc11ea9d3cb4d365efcd62e7a71cae:b3f47d077d50c0a5339d29f608b5f351591a7057 | CLEAN (3 MINOR applied in clarify) |
| plan | 1 | design | e458a16fca33bf98126ebd10ded77ccc18266133 | 8487a8d58e89bb17100164e9cf76b5dfba6a0c1d:e458a16fca33bf98126ebd10ded77ccc18266133 | pending |
| plan | 2 | design | 6e241d6b1c3744d78f712e7998d7f26265227f3a | a6f9de08200acab60e831c1660b93e5f63aeb46a:6e241d6b1c3744d78f712e7998d7f26265227f3a | pending |
| plan | 2 | design | 6e241d6b1c3744d78f712e7998d7f26265227f3a | a6f9de08200acab60e831c1660b93e5f63aeb46a:6e241d6b1c3744d78f712e7998d7f26265227f3a | CLEAN (1 MINOR applied: R6 slugify trim/fallback) |
| tasks | 1 | design | aac33e2f643fabba7645ab9a83e0acec7db3e0c8 | a3a350cbee7cbea5eb623c27828a8217ba16f17a:aac33e2f643fabba7645ab9a83e0acec7db3e0c8 | CHANGES (2 MAJOR, 2 MINOR; all fixed) |
| tasks | 2 | design | 64197e73ad0729840a83965f8c5364b7950443fd | c73c0d8512eb2c789d089baa13c46fcb6cb0613a:64197e73ad0729840a83965f8c5364b7950443fd | CLEAN |
| code A+B (M1) | 1 | M1 | 52b08b293b4bf90cb8227ce28d2724bdbcd4cebe | 181479f93ce2eaed78d7913c940dba7f37a4ea0b:52b08b293b4bf90cb8227ce28d2724bdbcd4cebe | pending (orchestrator dispatches) |
| code A+B (M1) | 1 | M1 | c2d10f85181940a6640d88ff0061ac7680fefb1f | 181479f93ce2eaed78d7913c940dba7f37a4ea0b:52b08b293b4bf90cb8227ce28d2724bdbcd4cebe | CHANGES fixed: A F1-F5 (hub keeps queued answers, answered only once queued, per-route queue toward link, seq gap; F4 documented in contract), B F1 (drift-M1.md) |
| code A+B (M1) | 2 | M1 | 2aa9f0a4e5ba1b186cb09e0b8a2c7bf7ce2c1ee6 | 5aa7439a20be847cc0dd0815fc78ff252713794f:2aa9f0a4e5ba1b186cb09e0b8a2c7bf7ce2c1ee6 | CLEAN (MINOR 1 fixed in contract prose; 2 and 3 declined as bounded/unchanged) |
| code A+B (M3) | 1 | M3 | 0deffa8b600c9fccf833c546c3a4af79d9ddd400 | 1a054e45102f489bfff8d934d5a83c5d148f659d:0deffa8b600c9fccf833c546c3a4af79d9ddd400 | CHANGES fixed: A F1 (dead-link route: close before marking table dead, insert refused under lock, writer drops route on send failure), A F2 (try_send, full queue ends that route), B F2 (T021 run + drift entry), B F3, B F4 (tdd/cycle-log.md); B F1 to user, in Follow-ups |
| code A+B (M3) | 2 | M3 | c5c93f8ef6579f879265921f41fabd306d60e9bb | 0deffa8b600c9fccf833c546c3a4af79d9ddd400:c5c93f8ef6579f879265921f41fabd306d60e9bb | pending |
| code A+B (M3) | 2 | M3 | d4561fb3d4b9619c6c11b56e850be0fcd729da40 | 0deffa8b6:c5c93f8ef | CLEAN (1 MINOR declined: log on full-queue drop optional) |
| plan (M4) | 1 | M4 amendment | 7c108cf44e1683840bf89b351db2dee83f951f2c | a1a5c51dd66f08fa5d7abcc922b1d82d456b4643:7c108cf44e1683840bf89b351db2dee83f951f2c | CHANGES (1 MAJOR Windows exception removed, Linux only #1897; 3 MINOR fixed: platform wording, orchestrator_link unconditional, drift-M3 correction) |
| plan (M4) | 2 | M4 amendment | 979a3cb6dc220061f18bb1047ba34e49b72fb48f | 82585ae2076bce904f57eb42cf8e961869a5d6af:979a3cb6dc220061f18bb1047ba34e49b72fb48f | CLEAN |
| tasks (M4) | 1 | M4 | c3142fa3458e199c0d7a507db721c9922b713cdf | a47e6feffdf13bbc125154a00aa5209affb59f36:c3142fa3458e199c0d7a507db721c9922b713cdf | CHANGES (2 MAJOR: T027 file named, T030 split + T031 provisioned-path test; 3 MINOR fixed) |
| tasks (M4) | 2 | M4 | 99a436ea5332cc5ac847dfdd5fe130bddee194ce | 2a8a84273ecdd7a70f43c03f0020550dc0b13f9d:99a436ea5332cc5ac847dfdd5fe130bddee194ce | CHANGES fixed (mechanical, no r3) |
| code A+B (M4) | 1 | M4 | 9c0fd3c817d60d1b3dc28eb374a17db990bdf21f | f6aa5b806:9c0fd3c817d60d1b3dc28eb374a17db990bdf21f | superseded by next row |
| code A+B (M4) | 1 | M4 | b9ef49cb7a86756da6c801e6c2722d0b3e0a20fb | f6aa5b806:9c0fd3c817d60d1b3dc28eb374a17db990bdf21f | CHANGES (A: 3 MINOR fixed: typed RouteEnded, holder-end race, no fallback router; B: 4 MAJOR fixed: placement + late-refusal tests, cycle-log rows/waivers, T031 Cargo [[test]] text, drift unplanned paths; 2 MINOR fixed: PR body coverage, T033 now run locally with sshd; CI: sandbox_real arity, ssh_real holder RealTime) |
| code A+B (M4) | 2 | M4 | c6796ee405ac7529d6634b5281e5c67cd63f1860 | 9c0fd3c81:b9ef49cb7 | CLEAN (1 MINOR ledger wording fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover
None.

## Open escalation

None.

## Follow-ups not done

- M4 gate (cloud): `scripts/gate.sh` fmt and clippy pass; `cargo nextest` is not installed here, and a `cargo test --workspace` fallback filled the disk (link failures), so the workspace test run is CI's. Targeted runs passed: `orchestrator_provisioned`, `daemon_config_delivery_host`, `orchestrator_single_connection`. The `ssh_real_link` routed case passed locally with sshd. Warning: `scripts/tests/autopilot.test.sh` (or a sibling) checked out `main` in this worktree and left stray files; restored by hand.

- M3 review B F1 (FR-007): SSH, container and Kubernetes actors still connect directly (drift-M3.md "Scope narrowed"); jaro chose 2026-11-16 to do it as M4.

## Follow-ups (M2 review)
- list.rs:111 mid-ask link drop reported as timeout, not offline
- client.rs:188 uncapped concurrent OrchList per client

## Follow-ups (close)

- Close gate (cloud): `scripts/gate.sh` fmt and clippy only; the `cargo test --workspace` run does not fit the disk here and is left to CI.
- Close checks: `speckit-converge` found no unbuilt behaviour (all T001-T034 done; M2 has no recorded review B, so converge ran); plan-drift unrecorded drift for M2 and M3 recorded in drift-M2.md and drift-M3.md.
- `speckit-tdd-verify` verdict FAIL (tdd/verification.md, audited 5811fa2): 14 of 18 test-bearing tasks have no recorded red run (squashed M1-M3), the SC-003 benchmark is `#[ignore]` and no gate runs it. Remediation is tasks.md Phase 9 (T035-T045), test-only, not done in this close (no workers or disk for revert-and-run). Done in the close: the `bench-routing` mise task (half of T038; a CI/gate call is still to add). Needs an issue or a decision to waive.
- Untested by decision (drift-M4.md): status-cell text of a refused link (the cell shows the actor's `ConnectFailed` text unchanged), pod relay teardown order in `LinkLife` (relay dropped after the holder; checked by reading), and "no client connection on the daemon's socket" (the linked test daemon has no socket, so the check would be vacuous).
