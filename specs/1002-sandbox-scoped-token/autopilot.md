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
- **Next step**: M1 verify.md: fix review A F1/F2 (see *Handover*), then scoped gate, review B, full gate, PR text.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1090 | design (spec, plan, tasks) | merged | 04b29291209b144376228864f40d0f46ac87d103 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | No user git identity in a sandbox (FR-001–003, FR-014) | — | in progress |
| M2 | T014–T022 | full | Sanitized shared git config, helper reset (FR-004) | — | planned |
| M3 | T023–T029 | full | In-container helper + in-memory per-repo file (FR-006/007/015) | — | planned |
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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 98d24270a31995887551b4940385e98d06de202d:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | pending |
| Plan | 1 | c9fa31d9664b54c8a2795dccf16d38b1abf14079:6d822ec17c37bcde07f294ff2509e6682eda33e8 | CLEAN (4 MINOR, fixed, prose only) |
| Tasks | 1 | b6f5094100888a88679ecc6fe8c89bc1dc627695:3bbb1d8d627753e4926daa27fa7e7c7c12d3df48 | CHANGES (1 MAJOR: SourceNotSetUp App wording in M4, fixed via SOURCE_SETUP_HINT; 2 MINOR fixed) |
| Tasks | 2 | b248659e6d8d9fd04545078d5e6bf2c4668c4bb7:402875090f6540516eb724bb15368a2067a33956 | CLEAN (sonnet) |
| M1 A (code-review high) | 1 | e42b32340533c275d5ebd756929ac2e0dd7212c6:71e9883368ab4624e997e3c39e96939f9bc60f76 | CHANGES (2 MAJOR open: F1, F2; 6 MINOR, see Handover) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

**From**: M1 unit 4 (2026-10-19), at ~135k context. All of T001–T013 done and ticked (commit "wip(1002): M1 client, notice, T006, T007, T011–T013"). Scoped gate (`scripts/autopilot/scoped-gate.sh`, run with `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`) at handover: fmt and clippy green, tests green up to `micold-core --test settings_refuses_save_over_failed_read`, which fails only because the container runs as root (it is in `scripts/gate.sh`'s `root_only` list); the scoped gate stops there, so the rest of its test run is unconfirmed — the full gate `scripts/gate.sh` handles root-only tests. Fixed this unit: test expectations after the removal (sandbox_argv mount counts, start_tests kept items via serde, client AllowedKind count, settings_sections DEFERRED entry for the `#[serde(skip)]` flag), `DaemonView.notice_dismissable` (core) drives the Dismiss button. Visual pass (T011, quickstart C9) passed: `specs/1002-sandbox-scoped-token/visual-pass/README.md`. Quickstart C9 wording fixed to "dismiss it, reopen → gone" (prose only; the notice stays until dismissed, per D14/T006).
**Review A round 1** (code-review high, run as `claude -p` on the snapshot in *Review rounds*) found, to act on per review-rounds.md:
- **F1 MAJOR** `sandbox/lifecycle.rs` `adopt`: a pre-#1002 container's agent-socket mount is recognised only if today's `SSH_AUTH_SOCK` equals the path at its creation (agent sockets are per-login random paths, and `survive_logout` containers outlive logins). Proposed fix: stamp new containers with a label (e.g. `micold.git-identity=none`, read into `ContainerFacts`) and treat a container without it whose mount destinations include anything outside home/state/history/secret/current credentials/the `~/.git-credentials` path *and* is not a project dir as retired — or simpler: replace every unlabelled container whose daemon entry still carries `notice == RemovedGitCredentials`. Pick one, record a decision, test it in `lifecycle.rs` `adoption_tests`.
- **F2 MAJOR** `kube/start.rs` ~540: a pre-#1002 pod (key mounted, `GIT_SSH_COMMAND`) can be kept through the drift prompt (`DriftChoice::Ask` → `Keep`). Fix: when the existing pod's stamp `allowed` lists a removed kind (or mounts a retired secret volume), force `Recreate`, never offer Keep; test in `start_tests.rs` with the fake cluster.
- MINOR (fix if cheap): F3 retired-mount replace of a running container ends sessions silently (say so in the row / stopped reason); F4 `client shell/sandbox.rs:533` re-reads `SSH_AUTH_SOCK` — pass `CredentialLayout::conventional(&facts.home, None)` since `mounts.retired` is already chained; F5 `tests/sandbox_argv.rs:427` sets env process-wide and never restores, and `argv::create` never reads env (test proves little: assert on the `HostileHost` full spec instead); F6 `micold-daemon/tests/sandbox_real_no_user_credentials.rs` copies the `hostile_host` markers (enable `kube-fake` on micold-daemon's dev-dep and import `HostileHost`); F7 `daemons.rs:75` notice says "are replaced by" a token M1 does not ship — reword "will be replaced by …; until that is set up, the sandbox cannot push"; F8 `kube/credentials.rs:139` `daemon_env(_allowed)` ignores its param — drop it.
**Next step**: fix F1, F2 (and cheap MINORs), commit, review A round 2 (sonnet, fix diff via `review-snapshot.sh diff`), scoped gate, review B (conformance, sonnet), full gate `scripts/gate.sh`, push, PR title/body to scratchpad (body notes T002–T005, T008, T009 tests written after the code, D16; T012 not run here: no container runtime).

## Open escalation

None in this unit. D7 awaits jaro's answer on the orchestrator's decision card.

## Follow-ups not done

- Jira API tokens live under `data_dir()/jira`, which the legacy default container daemon mounts whole as its state dir: readable inside that sandbox. Out of #1002's scope (not a git credential); worth its own issue.
