# Autopilot ledger — #1002 sandbox-scoped-token

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1002 — "Sandbox container with daemon should use a fine-grained token, not the user's GitHub token and SSH key" (body in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1002
- **Worktree branch**: claude/project-thread-2cdrmk
- **Started**: 2026-10-19
- **Phase**: milestones
- **Next step**: M3: orchestrator opens the M3 PR (pr-m3-title.txt / pr-m3-body.md in the unit's scratchpad), waits on CI and merges; then M4.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1090 | design (spec, plan, tasks) | merged | 04b29291209b144376228864f40d0f46ac87d103 |
| #1096 | M1 | merged | 07ad3c1f936358fd25ecf021e7009459c9ca2458 |
| #1099 | M2 | merged | abb25cf8907de6acef23805b2334f679f0060af7 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | No user git identity in a sandbox (FR-001–003, FR-014) | — (to open) | gate green at 17370db2, pushed |
| M2 | T014–T022 | full | Sanitized shared git config, helper reset (FR-004) | #1099 | merged |
| M3 | T023–T029 | full | In-container helper + in-memory per-repo file (FR-006/007/015) | — (to open) | gate green, pushed |
| M4 | T030–T038 | full | Protocol delivery, Schedule, revocation with FakeSource (FR-008–010, 012) | — | planned |
| M5 | T039–T049 | full | [source] GitHub App source + client wiring (FR-005) | — | planned |
| M6 | T050–T055 | full | Status rows + PR/issues toggles (FR-011, FR-013) | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which runtimes are in scope? | Container and Kubernetes daemons; remote SSH (user's own machine) out | agent-resolved | spec.md#Context |
| D2 | spec | Non-GitHub remotes? | No git credentials in a sandbox; old opt-ins not kept for them (least privilege) | agent-resolved | spec.md#Assumptions |
| D3 | spec | Default permissions? | contents rw + metadata r; PRs and issues per-daemon opt-ins, off by default | agent-resolved | spec.md FR-011 |
| D4 | spec | One credential per daemon or per repo? | Per repository | agent-resolved | spec.md#Assumptions |
| D5 | clarify | Keep old opt-ins for non-GitHub remotes? | No; no credentials, status says host unsupported (confirms D2) | agent-resolved | issue #1002 body ("rather than falling back") |
| D7 | clarify | Where does the scoped credential come from (FR-005)? | (A) user-registered GitHub App; client holds ID + key on device, mints 1 h single-repo installation tokens | provisional, pending jaro's answer on the decision card | spec.md#Clarifications; constitution.md#IV |
| D8 | clarify | Round 2 | No critical ambiguities; App key storage and setup UX deferred to plan | agent-resolved | spec.md |
| D9 | plan | Where does the App private key live on the device? | Sibling of data_dir (`micold-ai-ide-secrets/`), owner-only; data_dir is mounted whole into the legacy default sandbox | agent-resolved | research.md R5 |
| D10 | plan | Who renews while no client is connected? | Nobody: tokens lapse within 1 h, pushes fail closed, next connect re-grants (US3-4) | agent-resolved | research.md R8 |
| D6 | clarify | May a project-hosted service mint credentials? | No; client mints/holds on the device | agent-resolved | constitution.md#IV, #Technology constraints |
| D11 | tasks | Milestone cut vs plan's 3 | 6 milestones: plan's M1 split by FR (US1 removal / FR-004 sanitizer), plan's M2 split into git side (M3) and delivery (M4); the [source] GitHub App is confined to **M5** (was "M3" in the plan); M6 = status + FR-011 toggles. M1 (13 tasks) not split: the enum removal forces the client items to change in the same compile, and the `#[serde(skip)]` removal flag means the notice cannot lag the removal. M5 (11 tasks) not split: App settings without the wiring would be half-wired UI | agent-resolved | tasks.md#Milestones; milestones.md rules 3, 6 |
| D12 | tasks | speckit-analyze (14 findings, 2 HIGH) | All fixed: Verify/Satisfies aligned with quickstart steps; fixture placed in `sandbox/hostile_host.rs`; kept items enabled in FR-014 tests; old container git becomes `NoCredentialReason::GitTooOld` via `GitCredentialTargets.git_too_old` (plan Risks, contracts/protocol.md, data-model §6, spec Key entity updated — plan fix); `MAX_LIFETIME_SECS` const; two-renewal test; quickstart step 12 for FR-011; notice visual pass moved to M1 | agent-resolved | tasks.md, plan.md#Risks |
| D13 | M1 | No `tdd/test-list.md` (design skipped `speckit-tdd-plan`); the mandatory `tdd.run` hook needs one | Drive M1 test-first by hand in tasks.md's order (T001–T007 tests before T008–T011): write each test, run it and record its red line in this ledger's *Review rounds*-adjacent note or the commit message, then implement. No test list is created for one milestone | agent-resolved | precedent: specs/9xx ledgers "test tasks are ordered first by hand" |
| D14 | M1 | Where the `RemovedGitCredentials` notice is stored, given `ContainerSettings.migration` is one `Option` already used by `MovedOffReleasedImage` (cleared on a basic-image start, which would also wipe ours) and `KubeSettings` has no notice field | On `DaemonEntry`: new `#[serde(default, skip_serializing_if = "Option::is_none")] notice: Option<MigrationNotice>`, DaemonEntry read through a `RawDaemonEntry` (`#[serde(from)]`) whose `From` takes `SandboxProfile::take_removed_git_credentials()` / `KubeSettings::take_removed_git_credentials()` and sets it; `migrated_entry` does the same for the legacy `daemon` block. `DaemonRegistry::dismiss_notice(id)`; client row shows it beside the image notice with a dismiss message. 11 `DaemonEntry {` literals gain `notice: None` | agent-resolved | daemons.rs:46-85, features/daemons.rs:588-593 |
| D16 | M1 | D13's test-first order was not kept: units 2–3 wrote the core removal (T008–T010) before its tests T002–T006, and T009's Kubernetes tests after the code | Tests kept as written (each asserts the removed item's absence, which the code now guarantees); the PR body says T002–T005, T008, T009 tests came after the code | agent-resolved | commits d22a1a75, 71e98833 |
| D15 | M1 | Kubernetes secrets `<base>-git-credentials` / `<base>-git-ssh` already in a cluster from before the change: removing the variants drops them from `every_kind()`, so start (start.rs:409) and remove (lifecycle.rs:101) would never delete them and the user's key would stay in the cluster | Keep their names as `ResourceNames::retired_secrets()`; start deletes them (ours-labelled only) beside the withdrawn kinds, and `remove` deletes them too (`delete_ours` tolerates absence). Test it in `start_tests.rs` with the fake cluster | agent-resolved | kube/start.rs:409-424, kube/lifecycle.rs:94-114 |
| D17 | M1 | Review A F1: a pre-#1002 container's SSH-agent mount is recognised only when today's `SSH_AUTH_SOCK` equals its creation path | Least privilege: new containers carry the label `io.micold.git-identity=none`; `adopt` replaces any existing container without it (running, legacy released image or not), ahead of every other branch. The retired-path check stays as a second line. Cost: every container made before #1002 is replaced once on its next start, ending sessions in it; the `RemovedGitCredentials` notice says so (F3) | agent-resolved | reviewA.out F1/F3; sandbox/lifecycle.rs `adopt` |
| D18 | M1 | Red CI: `sandbox_real_provision_a_running_legacy_container_is_attached_then_replaced` got `Replace{MadeBeforeCredentialRemoval}` where FR-017 expects `Legacy` | Not a D17/FR-017 conflict: the real-runtime harness (`start_sandbox_on`) builds its `create` argv by hand, not via `argv::create`, and omitted the `io.micold.git-identity=none` label, so its container really was unlabelled. Harness now passes the label (mirroring argv); D17 stands (an unlabelled container is replaced, least privilege), FR-017 still holds for labelled released-image containers. Other real-runtime tests do not call `adopt`/read facts, so none shares the assumption | agent-resolved | sandbox_real_support/mod.rs; CI log |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 98d24270a31995887551b4940385e98d06de202d:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | pending |
| Plan | 1 | c9fa31d9664b54c8a2795dccf16d38b1abf14079:6d822ec17c37bcde07f294ff2509e6682eda33e8 | CLEAN (4 MINOR, fixed, prose only) |
| Tasks | 1 | b6f5094100888a88679ecc6fe8c89bc1dc627695:3bbb1d8d627753e4926daa27fa7e7c7c12d3df48 | CHANGES (1 MAJOR: SourceNotSetUp App wording in M4, fixed via SOURCE_SETUP_HINT; 2 MINOR fixed) |
| Tasks | 2 | b248659e6d8d9fd04545078d5e6bf2c4668c4bb7:402875090f6540516eb724bb15368a2067a33956 | CLEAN (sonnet) |
| M1 A (code-review high) | 1 | e42b32340533c275d5ebd756929ac2e0dd7212c6:71e9883368ab4624e997e3c39e96939f9bc60f76 | CHANGES (2 MAJOR F1, F2 fixed (D17); 6 MINOR F3–F8 fixed) |
| M1 A (sonnet) | 2 | 833375deedd427f1a1c0dfa38313f7936923038e:17370db2c46b156d528932658e24a5ca2fa58b7e | CLEAN |
| M1 B (conformance, sonnet) | 1 | 833375deedd427f1a1c0dfa38313f7936923038e:17370db2c46b156d528932658e24a5ca2fa58b7e | CHANGES (F1 MAJOR declined, see below; F2 MINOR: Verify covered by the full gate's `cargo test --workspace`; F3 MINOR: D17 stated in the PR body) |
| M1 red-CI fix D18 (sonnet) | 1 | 045a4f1811c6371aa92bff326aa70e42bbdbbf38:8e97a4027dbbf4c58e3fd6b7c40a4ecc5f8d5971 | CLEAN |
| M2 A (code-review high) | 1 | 0c7576a111f7bca073769e7193c66f5d11d9aac6:07ad3c1f936358fd25ecf021e7009459c9ca2458 | CHANGES (F1 MAJOR fixed: value-with-userinfo and smtp/imap keys dropped, contract §3 updated; F2 MINOR kube tests need git, as CI has it; F3 MINOR git>=2.31 noted) |
| M2 B (conformance, sonnet) | 1 | 70552b50588c4b06e4e005f3105dcb4274fe43dc:07ad3c1f936358fd25ecf021e7009459c9ca2458 | CLEAN (2 MINOR left: proxyAuthMethod drop is harmless; client test skips without git) |
| M3 B (conformance + correctness, sonnet; one reviewer for A and B) | 1 | f62bba4f347d8a6361d83cac442b621f119d2a6f:79e2d3e0e3335139d58a5d999979f26b0472e0b0 | CLEAN (2 MINOR: no tdd/cycle-log.md; sandbox_argv run collided with a build, green in the gate). Review A as the `code-review` skill not run separately |

## Declined review findings

- M1 B F1 [MAJOR] no `tdd/cycle-log.md` red evidence for T002–T005, T008, T009 (Constitution I): declined — already decided in D16 (tests kept as written, each asserts the removed item's absence), and the PR body discloses that those tests came after the code. Re-deriving red runs would mean reverting the removal against tests that no longer compile on the old code.
| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None in this unit. D7 awaits jaro's answer on the orchestrator's decision card.

## Follow-ups not done

- Jira API tokens live under `data_dir()/jira`, which the legacy default container daemon mounts whole as its state dir: readable inside that sandbox. Out of #1002's scope (not a git credential); worth its own issue.
