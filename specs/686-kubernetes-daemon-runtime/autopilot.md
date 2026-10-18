# Autopilot ledger — #686 kubernetes-daemon-runtime

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #686 "Support a Kubernetes session daemon runtime" (with owner comment: align image source with #806); labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #686
- **Worktree branch**: claude/project-thread-xelisc
- **Started**: 2026-10-17
- **Phase**: implement
- **Next step**: M9 pushed, gate green (scripts/gate.sh); PR files in the scratchpad (pr-title/pr-body-686-M9); orchestrator opens the PR, reads the kube-cluster job log and sends fixes. Then close.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #965 | design | merged | 0d36906050f9ad7fedde4b06fa0ee0ca31d231c9 |
| #978 | M1a | merged | 0b482e1d265b286a31482b1184499377bec88ab5 |
| #983 | M1b | merged | c256c2fe760e00e8aa3252babca54ebe4ee33d5f |
| #989 | M1c | merged | 499a59245c2e4edd8499f266b9efddc814081275 |
| #992 | M2 | merged | 767530b33afc20e128d27d78852508d7416bffa2 |
| #996 | M3 | merged | 8408b815f208a74217f877563e819cf6a543fcc2 |
| #1000 | M4a | merged | 3ef301a489e845ccbfe2f0af813187b8244e7575 |
| #1005 | M4b | merged | 21ad41546a8228d56012eac2ae359387b0981700 |
| #1011 | M5 | merged | c7d0dbf8393d4686551c81f404bad4028978e1f7 |
| #1015 | M6 | merged | 9639f49ae112f0c73155642adb9dc4a1dfa34bf6 |
| #1025 | M7 | merged | d6d74c272bd4bcc76d081ec171e2e63a00c448f1 |
| #1028 | M8 | merged | cc09ae12641f04b1e7986941ca4ee152539e4d7b |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1a | T106–T116, T143, T020, T026 | full | Revision-2 core types (settings with image/AI CLIs/DaemonResources, names, failures, Cluster.replace), kubeconfig; revision-1 listener auth, Transport::Tunnel, daemon_image reverted | #978 | merged |
| M1b | T019, T117, T118, T021, T025, T119, T120, T027, T121 | full | Pod, PVC, access secret, pod stamp/drift, start through Pull with bounds + rollback over FakeCluster | #983 | merged |
| M1c | T122, T023, T123, T124, T125, T028, T029, T126, T127 | full | Tunnel phase (SSH identification line), LiveCluster, relay, key files | #989 | merged |
| M2 | T079, T072, T056, T073, T081, T075, T059, T076, T128 | full | Core Stop/Remove/forget-locally; failure + pod-gone classifiers; preflight | — | merged (#992) |
| M3 | T063, T064, T065, T067, T068, T069, T102 | full | Allowed credentials as secrets, daemon_env, security contexts (core) | — | merged (#996) |
| M4a | T024, T130, T066, T031, T131, T070, T032 | full | Kubernetes form via Edit (Add entry hidden), shared resources fields, credentials list | — | merged (#1000) |
| M4b | T083, T084, T080, T132, T085, T086, T082, T133, T129 | full | Placement fields, Stop/Remove/Forget locally/Logs from the row, Start → Unavailable | — | merged (#1005) |
| M5 | T134, T022, T074, T135, T030, T077, T136 | full | (#806) Copy + Connect over SSH through the relay; failures at client level | — | merged (#1011) |
| M6 | T138, T139, T141, T142, T033, T140 | full | (#806) AI CLI install, drift prompt, Add entry, user guide 🎯 MVP | — | merged (#1015) |
| M7 | T047–T055, T100 | full | (#806) Clone + worktrees in the pod (RepoClone) | — | merged (#1025) |
| M8 | T057, T058, T060, T061, T062, T101 | full | (#806) Reconnect; pod gone → stopped with reason | #1028 | merged |
| M9 | T088–T096, T105 | full | (#806) kube_real_* suite, `mise run test-kube`, CI job kube-cluster | — | implemented, gate green, PR pending |
Cut notes (revision 2): tasks.md "Revision 2 task map" lists kept/changed/removed T001–T105; new tasks T106–T143 (T137 unused; T143 is M1a's persistence test rework). Order: everything independent of #806 first (M1a–M4b, all `docs-not-needed`: Kubernetes cannot be added before M6), then the #806-blocked milestones (M5–M9). US1 split along the #806 boundary: connect (M5) before CLI install + Add entry (M6), so the Add entry appears only when US1.1 works (no half-wired UI). User-guide tasks T046/T071/T078/T087 folded into T033 (M6), since the guide page appears with the Add entry. Polish T097–T098: close unit.
stash@{0} survivors (all used by M1a–M2, stash dropped 2026-10-18; backed up in /mnt/project-files/backups/686-m1-wip.bundle): keep `kubeconfig.rs` (T020/T026), `lifecycle.rs` (T079/T081), `manifest.rs` (T019, after the SSH image/env/port 2222/access-secret mount changes), `start_tests.rs` (T021, drop the `Image` phase and the volume-size refusal; `Tunnel` = identification line), `Quantity::{to_k8s, parse_k8s}`, `ResourceNames::all_secrets`, `ObjectKind::noun`, `KubeFailure::StopTimeout` + `From<ApiFailure>`, `FakeCluster::hold_on_delete`. Discard `StartPlan::for_image`. Design revision part 4: plan fixed by tasks unit — data-model StartPlan and contract §3 item 1 say `Tunnel` reads the identification line from one `Cluster::port_forward` stream; the client relay is first used in M5.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-004 default image source | #806 build, pushed to a user-named registry; pull stays an option | orchestrator default, pending owner | decision card 2026-10-17 |
| 2 | clarify | FR-004 provider CLIs in the pod | installed at build time | orchestrator default, pending owner | decision card 2026-10-17 |
| 3 | clarify | FR-007 clone source | project git remote with allowed credential; no-remote refused | owner (decision card 2026-10-17T21:33Z): "Git remote" | decision card 2026-10-17 |
| 4 | clarify | private registry pull; push auth | pull secret on FR-008 allow-list; push via local runtime login | agent | spec.md FR-008, FR-002 |
| 5 | clarify | image tag/rebuild; CI image loading | deferred to plan | agent | spec.md FR-017, FR-021 |
| 6 | implement M1 | U16: does `RegistryError::DuplicateName` name the first daemon? | keep the variant field-less; the form text "Another daemon is already named <name>." names it (names match case-insensitively) | unit, from evidence | `features/daemons.rs` form_error; `core/tests/daemons_kube.rs` |
| 7 | implement M1 | M1 too large for one milestone: split? | split M1a (T001–T021, T025–T027, T079, T081; core over FakeCluster, Kubernetes not in the Add form) / M1b (T022–T024, T028–T033, T080, T082, T099; depends on M1a); later milestones depend on M1b | orchestrator | Open escalation of unit 3; tasks.md Milestones |
| 8 | implement M1a | FR-004 provider CLIs in the pod | owner: "Install on start" — installed into the volume when the pod starts; works with pulled images; slower first start; pod needs outbound network. Replaces decision 2. Owner decision, design change pending: spec FR-004 / plan / tasks to be revised by a dedicated unit after M1a merges, before M2 | owner | decision card 2026-10-17T21:33Z |
| 9 | implement M1a | FR-004 image source | owner (verbatim): "use basic image with ssh where you can copy daemon and install ai cli" — replaces decision 1: no build/push/registry; the pod runs a basic image with sshd, the client copies the daemon binary in over SSH and installs the AI CLIs (with decision 8). Changes the start sequence (no `Image` phase; a copy/install step) and likely the manifests. Design revision pending before more implementation | owner | owner instruction 2026-10-17T21:34Z |
| 10 | implement M1a | Same mechanism for the local sandbox container? | owner (verbatim): "should use same mechanism for local \"sandbox\" container". Orchestrator default: #686 builds the sshd-image + copy-daemon-over-SSH + install-CLIs mechanism as a shared part both runtimes can use; moving the local sandbox container runtime onto it is #806's scope (commented there) | owner + orchestrator default | owner instruction 2026-10-17T21:35Z |
| 11 | design rev 2 | Who builds the shared sshd image + copy-daemon + install-CLIs mechanism? | #806's separate run builds it; #686 consumes it (contracts/shared-ssh-image.md, to write) and orders milestones: work independent of it first (core types done, kubeconfig, manifests + pod/PVC/secret lifecycle, port-forward tunnel, stop/remove, failure reasons, Settings form), copy-daemon + install-CLIs last, marked "blocked on #806" | owner via coordinator | coordinator message 2026-10-17 |

## Decisions (M8)

- #806 status (orchestrator, 2026-10-18): #806 is open; M5 to M7 needed nothing from its pending work (M5b notices, M6 per-daemon CLI selection, M7 step bounds and rollback), and neither does M8. Drift checked and fixed in tasks.md before the tests: (1) the 489 label is `lost to daemon <name>` (`links::session_lost_label` already maps `Stopped` to it), not "lost to its daemon"; (2) the `Cluster` port has no node look-up, so a lost node is the pod's phase `Unknown` (`PodGone::classify(status, true)`); (3) `DaemonState::Stopped` stays a unit variant, the reason lives in `Link.stopped_reason` (`DaemonStates::stopped_reason`, `DaemonView.stopped_reason`); (4) the actor's look-up is `daemon::kube_reconnect`, used by `connect_and_pump` when `life.established` (a lost link, or Reconnect on an unreachable daemon that had connected: `resumed`); a 5xx or timeout is `ConnectFailed`, so the state stays `Reconnecting` with the 489 back-off; (5) the scripted pod host answers one connect, so each re-dial in the tests uses a fresh client on the shared cluster.

## Decisions (M7)

- #806 status (orchestrator, 2026-10-18): #806 is open (its M5 merged; M5b, M6, M7 pending). M7 needs nothing from them. Drift fixed: `PROTOCOL_VERSION` on main is 50 (813 bumped it), so RepoClone takes 51 (plan, test-list U82, data-model, research, T048 updated); `SCHEMA_HASH` is generated by `micold-core/build.rs`, so there is no `schema_hash.rs` to re-pin; M5 already passes `MICOLD_REPOS_ROOT=/data/repos` (`kube::credentials::REPOS_ROOT`) to the pod daemon. Pod paths are joined with "/".

## Decisions (M6)

- #806 status (orchestrator, 2026-10-18): #806 is open, but M6 builds on what is on main: `provision::run` installs the AI CLIs (806 M4), and `sandbox::basic_image` pins the image. Task-text drift fixed in tasks.md T138/T139/T141/T142: (1) install runs inside M5's `connect_pod`, so T141 is `pod_target(reach, providers)` + failure mapping, not a new phase; (2) #806 only *warns* on a failed CLI install and goes on to start the daemon, so `connect_pod` turns the warnings into `KubeFailure::Provision` (FR-012, US4.5) and rolls back; (3) the install timeout maps to `PhaseTimeout { Install }`; (4) test paths are `crates/micold-client/tests/`; (5) the install fake is `kube::fake_host::Host` (+ `then_hang`); (6) no pinned digest exists (806 T049), so the default image is `KubeSettings::pod_image()` = `micold-basic:<content hash>` tag; (7) the Add gate is `SHARED_SSH_IMAGE_AVAILABLE`. Nothing M6 needs sits only in #806's M5–M7.

## Decisions (M5)

- Orchestrator (2026-10-18): #806 is not closed, but what M5 needs is on main (M2 `micold_core::provision` #982, M3 static musl daemons #988, M4 AI CLI install + registry-only relay 08687287/ccc232e7). Start M5 against `specs/806-container-daemon-ssh-provisioning/contracts/provisioning.md`; #806's pending M5–M7 (container migration, per-daemon CLI selection, step bounds/rollback) are not needed. Stop with ESCALATE if something is.
- Unit (M5 re-check, tasks.md drift fixed): T134/T135 name `provision::run` and a scripted `SshHost` instead of "#806's provisioning fake"; the one call covers Copy+Install+Connect; HostKeyAlias dropped (PinnedReach has none; known_hosts names `[127.0.0.1]:<relay port>`); `ProvisionTarget.env` added for `daemon_env` (not in the 806 contract); T022/T074 paths `crates/micold-client/tests/`; T030 `Dial::Kubernetes`, `kube_attempt` takes `KubeSettings`; `(#806)` tags dropped from T022/T074/T030/T077/T134/T135.

- Unit 3 (M5 client half, drift fixed in tasks.md T030): `Dial::Kubernetes { name, settings: KubeDial }`; `kube_attempt(&KubeEnv, name, &KubeSettings, sink) -> (Connected, Relay)`, the `Relay` held by `connect_and_pump` for the link's life; `KubeEnv {connect, keygen, host, keys_root, home}` installed once by `shell/startup.rs` (`install_kube`); live `SshKeygen` in `shell/daemons.rs` (ssh-keygen, empty pair on failure, refused by `kube_attempt`); `KubeConnector` moved to `micold_client::kube`; `known_hosts` is `[127.0.0.1]:<relay port> <key>` (`host_alias` removed). A `None` image is `Unavailable` (the pinned SSH image is M6's). The pod login user is `POD_SSH_USER = "micold"`, the basic image's passwd entry for uid 10001 (was "dev" until M9 CI fix 4: sshd refused it). `provision::start::rollback` is now pub. T077: `StartFailed` and install lines go to `DaemonDiagnostics::push_log`. Shared test doubles: `micold_core::kube::fake_host` (Host, refusing/welcoming/closed bridges), `tests/kube_daemon_common`. Copy of a missing build is not exercised in the client tests (core covers it); the host's `have=1` probe is scripted.

## Decisions (M3)

- Unit: with nothing allowed the `Secrets` phase makes no create/replace/delete of a credential secret and reads no source file, but `get`s each possible credential secret name to find one a withdrawal left behind (no list call exists). `StartRequest` gains `home`, `hosts` (remote hosts for the SSH key's known_hosts lines; the caller supplies them, M5+) and `read` (the file reader).

## Review rounds

| M8 A (high) + B (conformance, sonnet) | 1 | bd0a53ac82b90645dfb29ada1a4a95627d8d01f6:ff3ee14e375588e36d83ca9a706c3d133da6dcca | A: CLEAN, 3 MINOR (F2 comment added, F3 catalog removal added, F1 pump-level test declined). B: CHANGES, F1 MAJOR declined (below), F2/F3 MINOR noted |
| M7 B (conformance, sonnet, scoped to the F1/F2 fix) | 2 | 07ebdb8d7f5012cdb1b0b07b80e4cb2c0a1af0e2:62a795e9d0795da82cbf322e0b064d9ac38321d1 CLEAN — 1 MINOR (refusal order: not-connected now precedes no-remote; both send nothing) |
| M4b review A (code-review high) | 1 | e6a9bc19:3ef301a4 | CLEAN — 2 MINOR declined/follow-up |
| M4b review B (conformance, sonnet) | 1 | 13e04614:826e419b | CLEAN — Verify green |
| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| M3 A (code-review high) | 1 | 613bbb5d7241ebee3806b0ad999889186b7cc3ab:9c51cf8e543637af11209d1da7e8fda46cb985d2 | CHANGES: F1 MAJOR fixed (GIT_SSH_COMMAND uses a 0600 copy of the key; ssh refuses group-readable keys); F2–F4 MINOR declined/deferred (Follow-ups) |
| M3 B (conformance) | 1 | same | CHANGES: F1 MAJOR red-run evidence: waiver recorded in cycle-log; F2 MINOR: validate() two-secret rule is a deliberate M3 addition (one secret per kind, ResourceNames) |
| Spec | 1 | 7330b10a0e165a38d678df142737f8a7c1ea47d0:d93c9dbc687412cced0a9b7854cf44323ee3930e | CHANGES: 4 MAJOR, 3 MINOR — all fixed (F1 pod-gone → stopped + user Start; F2 FR-023 + multi-client edges, FR-016 identity; F3 kubeconfig/exec-plugin edges, FR-022; F4 per-phase bounds in FR-012; F5 Reconnect scenario; F6 provider CLIs in FR-004 marker; F7 version wording) |
| Spec | 2 (scoped, sonnet) | aed1af6291d0f65b2e21df970370031d92b506f6:171d36233aff56cab8399d2c2c08b7ed1c109dc2 | CLEAN — 1 MINOR fixed in clarify (SC-003, story 4.5, story 6.2 say "applicable phase bound (FR-012)") |
| Plan | 1 | c2d5b880c183edc0c6dcd702539b137bbeaf2b7e:744377ac1576676ae85cc3ae5c9897190bba1bf2 | CHANGES: 3 MAJOR, 3 MINOR — all fixed (F1 shared `DaemonResources` with separate requests/limits, FR-005 manifest tests, contract wording; F2 `CliRuntime` inherent `build_image`/`push_image`/`repo_digest`, tests over `RecordingRunner`; F3 `sha2` dropped for `sha256_hex`; F4 fields extracted in `container_fields.rs`, recorded in K1; F5 K4 guard restated; F6 FR-007 client reducer tests) |
| Plan | 2 (scoped, sonnet) | b26d31e28d1362689a3bc021fc0af7cdff72869c:48a879226ceb5fe45e669e6aade3331e2c742d6f | CLEAN — 1 MINOR fixed, prose only (plan.md Primary Dependencies, contracts/daemon-image.md: `program_of` is the free function `sandbox::cli::program_of`, not a `CliRuntime` method) |
| Tasks | 1 | 1aab2e66eaf532352e358ce882e50c398d808e64:b3c1e46cab50933fb87b51b5d85bae4ad7b90da8 | CHANGES: 4 MAJOR, 3 MINOR — all fixed (F1 speckit-tdd-plan run: tdd/test-list.md A1–A27, U1–U116, markers on every behavioral task, acceptance tasks T099–T105 per milestone, tdd/cycle-log.md baseline; D14 un-declined; F2 M2/M7 Verify one filter per command; F3 M1 Verify adds `--lib kube::live` and `micold-daemon server::`; F4 credentialed-clone clause moved from M5 to M8 (T092), M3 text points there; F5 checklist ticked, Notes say defaults pending owner; F6 T002 not [P]; F7 T075 names only the M6 classifiers) |
| Tasks | 2 (scoped, sonnet) | bf9cd12b8a7d863016aa8c342f4bf594a34f477e:a8551c98307dc5eb20c1a8b684f92be48c98641b | CLEAN — 1 MINOR fixed (Milestones preamble names T097/T098 Polish as in no milestone, left to the close unit) |
| Spec rev 2 | 1 | 9b02dde817655adc92898292c8f93e391c823d2e:4233f507b15954e5f2b3b193009816ee9ea8fd52 | CHANGES: 3 MAJOR, 3 MINOR — all fixed (F1 FR-023 shared-step concurrency + edge case; F2 keys in the access secret, per client per daemon, FR-015 remaining key; F3 CLI selection US5.1/FR-018, version pinned by client build in FR-004; F4 checklist note; F5 FR-005/US5.2 pod-settings drift offer; F6 FR-024 observable) |
| Spec rev 2 | 2 (scoped, sonnet) | b07e424c9e2a84170a1b988315187afce6dfc70c:24f30ec4ba00e3835c53c0f7316fdc7421cf4a76 | CLEAN — 3 MINOR fixed, prose only (FR-024 per-client key pairs; FR-005 form notice on volume size; FR-004 running sessions of a deselected CLI keep running) |
| Plan rev 2 | 1 | 41559a5d6d08b3aae87bb6486b442919a9827c2a:52d80e58149fb6943f8321d342526864d32ed8d1 | CLEAN — 3 MINOR fixed, prose only (Unavailable before #806; daemon_resources lands in #686 first; protocol 49 coordination) |
| Tasks rev 2 | 1 | c1aab5ca46629522b51f6d4161cc2270a3155835:1ac015d23d158b44cb52d9cd09a5df7c4782b062 | CHANGES: 1 BLOCKER, 2 MAJOR, 2 MINOR — all fixed (F1 T116 keeps feature 027's `protocol/auth.rs`, restores it to origin/main, Verify checks it; F2 new T143 reworks `core/tests/daemons_kube.rs` in M1a; F3 M4a depends on M1c for context listing; F4 T020/T026 register `kubeconfig`; F5 T115 notes `StartPhase::Image` stays until T027) |
| Tasks rev 2 | 2 (scoped, sonnet) | fa5553fcabfc99c580d0b9f2dc1af7c74f0c6834:618e287a0ec8c9de70dc33c69b725c7aa98fdd5f | CLEAN — 1 MINOR fixed, prose only (Parallel Examples names T143 with the M1a tests) |
| M1a review A (code-review high) | 1 | d16c6458b8b7e5664f8db0f01d4324dee0807460:139d6af21c0c8c63b2f7d7e1c12c826d51b0f45b | CLEAN — 3 MINOR: F3 fixed (`DaemonResources` `#[serde(default)]` on the struct, test); F1/F2 declined (pre-existing) |
| M1a review B (conformance, sonnet) | 1 | fbe3dfd0f7a04a22ec882b7cd8b052c14be5b2b1:44c055bedeac1dfe9ace9aea5d22e17d56d17e35 | CLEAN — 1 MINOR fixed (client Cargo.toml `[features]` moved off the showcase comment) |
| M1b review A (code-review high) | 1 | 64880871548a615f665134c536fac38ddf6845f1:0b482e1d265b286a31482b1184499377bec88ab5 | 10 findings, none BLOCKER: F1/F2 fixed (a terminating claim or secret is waited out, a vanished object re-created; tests), F5 fixed (rollback removes a key only when this attempt gave the client its first line; test), F8 fixed (one `update_access` loop for add and remove); F3, F4, F6, F7, F9, F10 declined (below) |
| M1b review B (conformance, sonnet) | 1 | 64880871548a615f665134c536fac38ddf6845f1:0b482e1d265b286a31482b1184499377bec88ab5 | CLEAN — Verify 73 passed; 3 MINOR: F2 counts fixed, F3 (`MICOLD_SSH_DIR` assumed) in Follow-ups, F1 (start tests written with the sequence) accepted and recorded in the cycle log |

| M1c review A (code-review high) | 1 | 92eb8da19e833dfac69f4c82ff46e16132a896ac:9a5aa584e8579a2c9767db3d680952aefd1e27ab | CHANGES: 1 MAJOR (Tunnel did not retry a closed stream — fixed with a red test), 3 MINOR (relay reaped + survives accept errors, key dir sanitised; delete 409 declined) |
| M1c review B (conformance, sonnet) | 1 | 92eb8da19e833dfac69f4c82ff46e16132a896ac:9a5aa584e8579a2c9767db3d680952aefd1e27ab | CHANGES: 1 MAJOR (no cycle-log entries — added, honest that tests came with the code), 1 MINOR (declined) |
| M4a review A (code-review high) | 1 | f29f682624d1c0bd2370fef030eb8ec1d6d102af:153fa9f2b3d8ac3525be149d44e1053093c2e8c4 | CLEAN — 3 MINOR declined (Declined review findings): default provider is one value, MIN_MEMORY floor reused, `offered()` slice order |
| M4a review B (conformance, sonnet) | 1 | same | CLEAN — Verify 22 + 10 passed; 1 MINOR: file fields are plain text paths (no picker; same as the SSH form's key file), noted in the cycle log |
| M5 | A (code-review high) + B (conformance) | ade6ca29b89171b2bf51f114e355f5de2b13fe52:21e2ef5743bdc4b335c71cbd7303f9bfd6df9de6 | A: CLEAN (F1 MINOR fixed: drift text; F2 MINOR declined: `RefusalReason` has no Display, Debug kept until M6 words it); B: CHANGES, F1 MAJOR fixed (A15/A16/A17 tests added), F2 MINOR fixed (cycle-log) |
| M6 | A (high) + B (conformance) | cdafddd27dd68fd9d5a51df4d191af4f6a04f64a:991d8c1b71c05e8cca0ed4344c827bd201b94978 | A: CHANGES, F1 MAJOR fixed (drift prompts queued), F3 MINOR fixed, F2 MINOR declined; B: F1 BLOCKER mid-edit artefact, F2 MINOR fixed; round 2 (sonnet, fix diff): CLEAN, 1 MINOR (unguarded StartFailed on no-token Cancel) left |

## Declined review findings

- M8 review B F1 MAJOR (no test shows sessions with output since the drop): declined; keeping sessions and their output is the pod daemon's 489 behaviour (FR-010), and the scripted fake host has no sessions. M8 proves the client side: same pod re-dialled, nothing recreated, sessions labelled `disconnected`/`lost to daemon`. The sessions-with-output proof is `kube_real_reconnect` (M9, T088–T096). Review A F1/B F2 (no `connect_and_pump` test): declined; the pump is not callable without the iced channel, so its routing is covered through `kube_reconnect` and the state tests.
- M6 review A F2 MINOR (Cancel shown for the whole Starting state, token only read from the install phase): declined; `start()` phases are bounded and Cancel is noticed at the next phase; threading a token into every wait is out of scope.
- M6 review B F1 BLOCKER (client tests did not compile): the tree was mid-edit by a worker while B ran; both client suites green afterwards. B F2 MINOR fixed (an edit clears the drift answer).

| M4b | A F1 MINOR | Stop moves the link to Stopped before the pod delete ends; a failed delete only logs | contract §5 needs Stopped on success; surfacing a failed Stop is a follow-up |
| M4b | A F2 MINOR | Edit can change name/context/namespace of a Kubernetes daemon, orphaning cluster objects | recorded in Follow-ups |

| M2 | A (code-review high) | not-ours objects skipped silently by Stop/Remove; namespace 404 matched by message; PodGone for Failed/Succeeded pods; 403 mapped only in preflight | contract §2/§4/§5 say ours only and list exactly these rows; ApiFailure carries no details.kind; see Follow-ups |
- speckit-analyze D13 (start failure maps to `Unreachable`): already the 491/489 rule, stated in data-model.md state machine delta; no change.
- speckit-analyze D10 (owner decisions pending): no task change; phases 3b/3c are the separable parts.
- speckit-analyze D1 alternative (kind smoke test in M1): declined; pure `to_api_failure`/`delete_params` table tests in M1 instead, real cluster in M8.
- speckit-analyze rev 2 C7 (T033 user guide sits in M6, move to Polish): declined; milestones rule 4 puts the user guide in the feat PR that shows the Add entry (M6); T140 stays the last M6 task. Analyze rev 2 C1–C6 fixed (U18/U19/U21 markers on T108/T113; contract §6 `Unavailable` row; research K3 Tunnel wording; spec Tunnel glossary note; stash note in tasks.md; T137 unused).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1a | A (code-review high) | F1/F2 MINOR: `is_startable()` true for Kubernetes; `set_single_daemon` rewrites a Kubernetes entry's runtime | both lines unchanged from origin/main; no Kubernetes entry can be added before M6 (Add entry hidden); the dial/start path lands in M1c/M5 |
| M1b | A (code-review high) | F3: a create in flight when a phase bound fires is not in `Attempt.created` | client-side cancellation of one request; the object carries both labels, so the next Start adopts it and Remove (M2) deletes it; noted in Follow-ups |
| M1b | A | F4: `await_pod` has no ErrImagePull/Unschedulable/Failed detection | tasks.md defers the classifiers to M2 (T075/T076) |
| M1b | A | F6: foreign `authorized_keys` lines are dropped on write | the access secret is Micold's (labels, created by it); only `micold-client:` lines are meaningful to it |
| M1b | A | F7: access secret mode 0440 root-owned, no `fsGroup` | the pod security context (`fsGroup`, UID) is T064 (M3); T019 fixes `defaultMode` 0440 |
| M1b | A | F9: `PodStamp` as a string re-parsed; a missing stamp reads as all differing | the contract fixes the annotation as canonical JSON text; a pod without a stamp cannot be told apart from a changed one |
| M1b | A | F10: `KeyGen::generate` is synchronous in an async start | only the fake exists; #806 owns the live one and can wrap it in `spawn_blocking` |
| M1c | A F4 MINOR | `deleted_from` maps only 404 to done; a uid-precondition 409 fails | a recreated object must not be reported deleted; failing is the safe reading |
| M1c | B F2 MINOR | `kube_connector` has `expect(dead_code)` until M4b | T029 requires choosing `LiveCluster` once; the wiring is the deliverable, M4b is its first caller |
| M4a | A (code-review high) | F1 default providers from the Settings draft only; F2 `MIN_MEMORY` floor for pod memory; F3 `offered()` slices by position | no per-provider switch exists yet (cycle log); the floor is the shared #806 resources field's; Kubernetes is last in both lists and a test pins the Add form's choices |

## Handover

None.

## Open escalation

None. (The anchor-timing flake seen in earlier gate runs did not recur; gate8 green. #806: nothing from its pending work is needed by M7; schema_hash pin moved 50 → 51.)

## Follow-ups not done

- M4b: make name, context and namespace read-only (or refuse the save) when editing a Kubernetes daemon, since object names and key dir derive from them; Stop failure should show a reason on the row; Logs has no hide action; the list editors have no focus tracking.

- M3 review A MINORs: F2 stale secrets deleted in Secrets phase before Pod drift decision (a kept drifted pod could lose its secret); F3 a rotated credential's payload is not in PodStamp (subPath mounts never update; needs a pod restart); F4 hashed `known_hosts` entries yield an empty pin file (needs ssh-keygen -F or keyscan) and a missing known_hosts fails the start.

- M2 review A (declined/deferred): a 403 or namespace 404 from a create/delete after a green preflight surfaces as `Api` (preflight covers the normal case; `KubeFailure::from_status` is available to the client adapter); `PodGone::classify` and `lifecycle::*` are wired by M4b/M8; Failed/Succeeded non-evicted pods are not in contract §4.

- M1b review A F3: a create in flight when a phase bound fires is not in `Attempt.created`, so rollback misses it (an object with both labels, adopted by the next Start). M2's Remove covers it; consider a name-based sweep of the daemon's three fixed names on a `PhaseTimeout` rollback.
- The access-secret directory variable is assumed to be `MICOLD_SSH_DIR` (`manifest::ACCESS_DIR_ENV`); #806's image must read it, or the constant changes.

## M1c notes

- Tunnel phase: opens one `port_forward` to `SSH_PORT`, reads the identification line; no line within the Tunnel bound, a non-`SSH-2.0-` line, or an exited container (pod not running) is `ImageContract` (the phase has its own timeout, so it is not `PhaseTimeout`). `Started` stays without a `tunnel` field: the probe stream is dropped; the client relay (`Relay`) opens its own per connection (first used in M5). `FakeCluster` serves an SSH banner by default (`set_banner(None)` = silent port).
- `client/kube/keys.rs` (`KubeKeys`) uses `micold_core::owner_only` and the core `KeyGen` port; it does not reuse `provision::keys::DaemonKeys` (#806): that one shells out to `ssh-keygen`, names its files `client_ed25519`, and pins `[127.0.0.1]:port`; the Kubernetes contract fixes `id_ed25519`, `client_id` and the `micold-kube-<key>` alias. The real `KeyGen` is wired with Copy/Connect (M5).
- `LiveCluster::connect(context)` is chosen once as `Capabilities::kube_connector` (expect(dead_code) until M4b). Exec-plugin failures at request time arrive as `Unreachable` text (kube reports them as a request error); resolved plugin paths cover the not-found case (`ExecPlugin`). Untestable against a real cluster here; every decision is in a pure helper with tests.

## Decisions (M9)

- #806 is still open (orchestrator decision): M5–M8 needed nothing of its pending work. M9 uses the basic image as on main (`packaging/basic`) as the SSH image, built in the job (`docker build`, then `packaging/kube/test.Containerfile` on top) and loaded with `kind load docker-image`; nothing is pulled from a registry.
- Drift fixed in the task text: `client-tests/` is `crates/micold-client/tests/` (T088–T094). T089: the stand-in's package source needs the image (an npm builtin config pointing at the docker gateway 172.30.0.1:4873), so the Containerfile exists. T088: the harness creates the cluster if missing but never deletes it (a test binary has no exit hook; each of the six binaries is its own process); `scripts/test-kube.sh` (the `test-kube` task and the CI job) deletes it on exit unless `MICOLD_KUBE_TEST_KEEP=1`. T095: the task calls `scripts/test-kube.sh` under the build lock. T096: no `daemon_runtime` gate, per "same docs-only skip" (runs on every non-docs change); `kube-cluster` added to `ci complete`.
- Pins: kind v0.30.0 (sha256 517ab7fc…, verified by download), kubectl v1.34.0 (sha256 cfda68cb…, from dl.k8s.io), node image kindest/node:v1.34.0.
- Not covered by the real suite, flagged for the orchestrator: T092's "remote needing the allowed git credential clones" and "withdrawal deletes the secret" (need an authenticated git server and allowed-credential fixtures; the unit tests A10–A12 over `FakeCluster` cover them) and the SC-002 600 s run (set `MICOLD_KUBE_AWAY_SECS=600`).
- Not runnable here (no docker/kind/kubectl): the suite compiles (`cargo test -p micold-client --features kube-real-runtime --no-run`) and each test prints `SKIP <name>: <reason>` and passes. CI's `kube-cluster` log is the first real run; expect fixes (image/npm config, RBAC verbs, step names in the timing).
- M9 review A F1 (dash `read -t`), F2 (namespaces rule needs a ClusterRole), F3 (pkill pattern): fixed. Review B F3: fixed (T088 text, harness comment). Review B F1 MAJOR (T092 credential withdrawal / authenticated clone): declined as a cut, not a miss: T092's text now marks both as deferred; the orchestrator should decide whether a follow-up task is wanted. F2 MINOR (one daemon build asserted): noted, not asserted.

## M9 CI fix 1

- Root cause of the pod's `Error` exit: no passwd entry for uid 10001 under a read-only root (entrypoint could not append one); now baked into `packaging/basic/Containerfile`. Insufficient cpu: request defaulted to the 2000m limit; the suite now requests 100m. Failed starts print pod description, logs and events. Details in `tdd/cycle-log.md`. Awaiting CI.
