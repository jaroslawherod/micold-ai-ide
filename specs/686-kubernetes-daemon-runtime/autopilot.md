# Autopilot ledger — #686 kubernetes-daemon-runtime

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #686 "Support a Kubernetes session daemon runtime" (with owner comment: align image source with #806); labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #686
- **Worktree branch**: claude/project-thread-xelisc
- **Started**: 2026-10-17
- **Phase**: design revision 2
- **Next step**: design revision 2 (continue from Handover): fix spec review round 1 findings, then plan, tasks, test-list; then re-cut M1a

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #965 | design | merged | 0d36906050f9ad7fedde4b06fa0ee0ca31d231c9 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1a | T001–T021, T025–T027, T079, T081 | full | Core runtime over FakeCluster: settings, manifests, kubeconfig, start with rollback, stop/remove | — | in progress |
| M1b | T022–T024, T028–T033, T080, T082, T099 | full | Add (given image), start/adopt, tunnel to connected, stop, remove from the client | — | planned |
| M2 | T034–T046 | full | Build → push → pod by digest; requests and limits | — | planned |
| M3 | T047–T055 | full | Clone remote + worktrees in the pod (RepoClone, protocol 49) | — | planned |
| M4 | T056–T062 | full | Reconnect; pod gone → stopped with reason | — | planned |
| M5 | T063–T071 | full | Allowed credentials only; pod security context | — | planned |
| M6 | T072–T078 | full | Failure reasons (preflight RBAC, pull, schedule, PVC) | — | planned |
| M7 | T083–T087 | full | Rest of the form: placement, volume, Reconnect, Logs | — | planned |
| M8 | T088–T096 | full | kube_real_* suite, `mise run test-kube`, CI job kube-cluster | — | planned |

Cut notes: US1 (>10 tasks) split along its acceptance scenarios and the plan's separable parts (3a given image, 3b image source, 3c clone source). US5 Stop/Remove (T079–T082) ship in M1 so the Kubernetes row reachable from the form has working Stop and Remove (no half-wired UI). FR-013 rollback and per-phase timeouts moved into M1 (analyze D5/D8). Polish T097–T098 change no code: close unit. M8 adds no user-visible behaviour: `docs-not-needed` if its PR is `feat`.

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 7330b10a0e165a38d678df142737f8a7c1ea47d0:d93c9dbc687412cced0a9b7854cf44323ee3930e | CHANGES: 4 MAJOR, 3 MINOR — all fixed (F1 pod-gone → stopped + user Start; F2 FR-023 + multi-client edges, FR-016 identity; F3 kubeconfig/exec-plugin edges, FR-022; F4 per-phase bounds in FR-012; F5 Reconnect scenario; F6 provider CLIs in FR-004 marker; F7 version wording) |
| Spec | 2 (scoped, sonnet) | aed1af6291d0f65b2e21df970370031d92b506f6:171d36233aff56cab8399d2c2c08b7ed1c109dc2 | CLEAN — 1 MINOR fixed in clarify (SC-003, story 4.5, story 6.2 say "applicable phase bound (FR-012)") |
| Plan | 1 | c2d5b880c183edc0c6dcd702539b137bbeaf2b7e:744377ac1576676ae85cc3ae5c9897190bba1bf2 | CHANGES: 3 MAJOR, 3 MINOR — all fixed (F1 shared `DaemonResources` with separate requests/limits, FR-005 manifest tests, contract wording; F2 `CliRuntime` inherent `build_image`/`push_image`/`repo_digest`, tests over `RecordingRunner`; F3 `sha2` dropped for `sha256_hex`; F4 fields extracted in `container_fields.rs`, recorded in K1; F5 K4 guard restated; F6 FR-007 client reducer tests) |
| Plan | 2 (scoped, sonnet) | b26d31e28d1362689a3bc021fc0af7cdff72869c:48a879226ceb5fe45e669e6aade3331e2c742d6f | CLEAN — 1 MINOR fixed, prose only (plan.md Primary Dependencies, contracts/daemon-image.md: `program_of` is the free function `sandbox::cli::program_of`, not a `CliRuntime` method) |
| Tasks | 1 | 1aab2e66eaf532352e358ce882e50c398d808e64:b3c1e46cab50933fb87b51b5d85bae4ad7b90da8 | CHANGES: 4 MAJOR, 3 MINOR — all fixed (F1 speckit-tdd-plan run: tdd/test-list.md A1–A27, U1–U116, markers on every behavioral task, acceptance tasks T099–T105 per milestone, tdd/cycle-log.md baseline; D14 un-declined; F2 M2/M7 Verify one filter per command; F3 M1 Verify adds `--lib kube::live` and `micold-daemon server::`; F4 credentialed-clone clause moved from M5 to M8 (T092), M3 text points there; F5 checklist ticked, Notes say defaults pending owner; F6 T002 not [P]; F7 T075 names only the M6 classifiers) |
| Tasks | 2 (scoped, sonnet) | bf9cd12b8a7d863016aa8c342f4bf594a34f477e:a8551c98307dc5eb20c1a8b684f92be48c98641b | CLEAN — 1 MINOR fixed (Milestones preamble names T097/T098 Polish as in no milestone, left to the close unit) |
| Spec rev 2 | 1 | 9b02dde817655adc92898292c8f93e391c823d2e:4233f507b15954e5f2b3b193009816ee9ea8fd52 | CHANGES: 3 MAJOR, 3 MINOR — all fixed (F1 FR-023 shared-step concurrency + edge case; F2 keys in the access secret, per client per daemon, FR-015 remaining key; F3 CLI selection US5.1/FR-018, version pinned by client build in FR-004; F4 checklist note; F5 FR-005/US5.2 pod-settings drift offer; F6 FR-024 observable) |
| Spec rev 2 | 2 (scoped, sonnet) | b07e424c9e2a84170a1b988315187afce6dfc70c:24f30ec4ba00e3835c53c0f7316fdc7421cf4a76 | CLEAN — 3 MINOR fixed, prose only (FR-024 per-client key pairs; FR-005 form notice on volume size; FR-004 running sessions of a deselected CLI keep running) |

## Declined review findings

- speckit-analyze D13 (start failure maps to `Unreachable`): already the 491/489 rule, stated in data-model.md state machine delta; no change.
- speckit-analyze D10 (owner decisions pending): no task change; phases 3b/3c are the separable parts.
- speckit-analyze D1 alternative (kind smoke test in M1): declined; pure `to_api_failure`/`delete_params` table tests in M1 instead, real cluster in M8.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

**Design revision 2 in progress (unit "Design revision 686", handed over at 150k).** Uncommitted
work is committed with this handover as `docs(686): design revision 2 — spec and research (WIP)`.

Done:
- spec.md revised (Status line "revision 2"): SSH image (#806's), copy daemon over SSH via 489
  FR-005, AI CLIs installed on start, no build/push/registry; new FR-024 (consume #806's mechanism,
  no second copy); FR-004/006/008/009/012/015/017/021, edge cases, entities, SC-001/004,
  assumptions, out of scope rewritten. Pod SSH auth: per-daemon access secret `<base>-ssh-access`
  (host key pair + `authorized_keys`, one line per client), client private keys owner-only on each
  client, host key pinned from the secret, sshd on pod loopback 127.0.0.1:2222.
- research.md: revision-2 preamble; K1 (consume #806, interface in contracts/shared-ssh-image.md
  — NOT YET WRITTEN, working module name `micold-core/src/ssh_image/`), K2 (loopback relay
  `client/kube/relay.rs` → port-forward per accepted connection; 489 system `ssh` with HostKeyAlias
  + per-daemon known_hosts; `Transport::Tunnel` not needed), K3 (access secret, multi-client keys
  via resourceVersion-guarded update, sshd reads authorized_keys from the mount, host key copied to
  /tmp 0600), K4 (listener_auth removed: revert T008/T016), K6 phases (Config, Preflight [+update
  secrets], Volume, Secrets, Pod, Schedule, Pull, Tunnel; then blocked on #806: Copy 120 s,
  Install 15 min, Connect 120 s; KubeFailure drops NoRuntime/Build/Push, adds ImageContract,
  HostKeyMismatch, Provision(..)), K7 (pod shape; daemon env passed by the provisioning start step,
  not pod env, since sshd clears env), K10 small, K12 (kind, no registry; blocked on #806).
- Spec review rev 2 round 1: CHANGES (full text /mnt/project-files/backups/686-rev2-spec-review-r1.md).
  Open, to fix then scoped re-review (sonnet, `review-snapshot.sh diff 9b02dde8…:4233f507…`):
  F1 MAJOR concurrency of shared steps (copy, install, access-secret key add) on one volume/secret
  by two clients — add edge case + FR-023 clause; F2 MAJOR US3.4 "accepts only the daemon's key"
  vs FR-008 per-client keys — say "only keys in the access secret"; F3 MAJOR AI CLIs to install and
  "wanted version" undefined — add to FR-004/US5.1/FR-018/entity (selected in the form, default the
  enabled providers; version pinned by the client build per #806); F4 MINOR checklists/requirements.md
  note stale; F5 MINOR edits vs adopted pod (say: an edit recreates the pod on next start when the pod
  spec differs; volume kept); F6 MINOR FR-024 delivery constraint → make observable ("Kubernetes is not
  listed in Add until …") or move to Assumptions.

Next (in order): fix F1–F6 in spec.md; scoped spec re-review; data-model.md, plan.md,
contracts/kube-runtime.md, new contracts/shared-ssh-image.md (replaces contracts/daemon-image.md:
delete it), quickstart.md; plan review (fresh reviewer, plan rubric); tasks.md re-cut + tdd/test-list.md
(mark T001–T018 kept/changed/removed: T005/T012 DaemonImageSource → remove, keep DaemonResources moved
to `core/daemon_resources.rs`; T004/T011 drop `registry`, `arch`, `RegistryRepo`, add `image:
Option<ImageRef>`, `providers`; T008/T016 revert; T010/T017 revert `Transport::Tunnel`; T009/T018
variant list change; T015 port 7727 → 2222; T034–T046 removed; new relay, access-secret, blocked-on-#806
tasks), milestones in coordinator's order (core types done → kubeconfig → manifests + pod/PVC/secret
lifecycle → port-forward tunnel/relay → stop/remove → failure reasons → Settings form (Add entry hidden)
→ blocked on #806: copy+install+connect and Add entry, clone/worktrees, reconnect, real-cluster CI);
tasks review; note what in stash@{0} survives (kubeconfig T020, lifecycle T079, manifest T019 needs
sshd image/env/port changes, start T021 needs phase list without Image and with Tunnel=sshd login,
`Quantity::to_k8s`, `ResourceNames::all_secrets` + access secret, `KubeFailure::StopTimeout`,
`FakeCluster::hold_on_delete` survive); commit. Branch still unpushed; never reset.

## Open escalation

None. Clarify round 1 questions remain with the owner as decision cards (spec uses recommended defaults).

## Follow-ups not done

None yet.
