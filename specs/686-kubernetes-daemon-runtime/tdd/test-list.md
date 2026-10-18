---
feature: 686-kubernetes-daemon-runtime
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 28
planned_at: 270a530f
updated_at: 270a530f
suite_baseline: red # container runs as root: 6 permission-denial tests fail, none touches this feature; see cycle-log.md
---

# Test List: Kubernetes session daemon runtime

Trace ids: `USn.m` is user story n, acceptance scenario m in `spec.md`; `FR-nnn` and `SC-nnn` are
its functional requirements and success criteria; `Edge:` names a `spec.md` edge case.

Behavior markers in `tasks.md` (`[A3]`, `[U12]`) are the link `/speckit.tdd.run` ticks by. A test
task carries the ids of the behaviors it writes; the implementation task after it carries the same
ids.

## Outer loop: acceptance behaviors

The profile's acceptance runner is the real-runtime suite. For this feature that is the
`kube_real_*` suite (feature `kube-real-runtime`, M8), which exists only after every story. Each
milestone therefore opens its acceptance behaviors at the highest level that runs without a
cluster: client integration tests over `FakeCluster` (`client-tests/`), or the composed start
sequence over `FakeCluster` in `core/kube/start.rs`. That is weaker than a real cluster; the `kube_real_*`
test named in the last column of the M8 rows is the end-to-end confirmation. A criterion whose
observable result needs a real pod (A3) is hosted only in the real suite.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1  | Starting an added Kubernetes daemon with the default image creates one pod in its namespace, copies the daemon in, installs the selected AI CLIs, and the row reaches `connected` with endpoint `<context> / <namespace> / <pod>` (M6) | US1.1, FR-001, FR-003, FR-004 | example | PENDING | `kube_daemon_start` (FakeCluster); real: `kube_real_endtoend` |
| A2  | Creating a worktree bound to a connected Kubernetes daemon clones the project's remote into its volume and shows the worktree with the daemon's name and in-pod path | US1.2, FR-007 | example | PENDING | `features_daemons` (fake daemon); real: `kube_real_endtoend` |
| A3  | A session started in that worktree runs in the pod, takes typed input and shows its output | US1.3 | example | PENDING | `kube_real_endtoend` (real cluster only) |
| A4  | Starting the daemon again while its pod exists adopts that pod and creates none | US1.4, FR-003, FR-023 | example | PENDING | `kube_daemon_start`; core half (M1b): `core/kube/start.rs` |
| A5  | A connected Kubernetes daemon has no Service or Ingress in its namespace | US1.5, FR-006, SC-006 | example | PENDING | `kube_daemon_start`; real: `kube_real_endtoend`; core half (M1b): `core/kube/start.rs` |
| A6  | With the tunnel dropped and the pod running, the pod's sessions are still running when the client looks again | US2.1, FR-010 | example | PENDING | `kube_daemon_reconnect`; real: `kube_real_reconnect` |
| A7  | Reconnecting after a drop shows the sessions running with their output since the drop, with no pod created | US2.2, FR-011, SC-002 | example | PENDING | `kube_daemon_reconnect`; real: `kube_real_reconnect` |
| A8  | Reconnecting to a deleted, evicted or node-lost pod shows `stopped` with that reason, the sessions `lost to its daemon`, and creates no pod | US2.3, FR-011 | example | PENDING | `kube_daemon_reconnect`; real: `kube_real_reconnect` |
| A9  | Start on that stopped daemon creates a new pod on the kept volume | US2.4, SC-005 | example | PENDING | `kube_daemon_reconnect`; real: `kube_real_lifecycle` |
| A10 | A start with a set of allowed credentials and sign-ins mounts exactly those secrets in the pod and no other | US3.1, FR-008, SC-004 | example | PENDING | `core/kube/start.rs` (FakeCluster); real: `kube_real_security` |
| A11 | A start with nothing allowed creates and mounts no credential, sign-in or pull secret | US3.2, FR-008 | example | PENDING | `core/kube/start.rs`; real: `kube_real_security` |
| A12 | A start after a credential was withdrawn deletes that secret and no longer mounts it | US3.3, FR-008 | example | PENDING | `core/kube/start.rs`; real: `kube_real_security` |
| A13 | Every pod the start creates runs non-root, with a read-only root filesystem and no privilege escalation | US3.4, FR-009, SC-004 | example | PENDING | `core/kube/start.rs`; real: `kube_real_security` |
| A14 | A start the namespace's RBAC forbids fails naming the denied verb and resource, and leaves nothing it created | US4.1, FR-012, FR-013 | example | PENDING | `kube_daemon_failures`; real: `kube_real_failures`; core (M2): `core/kube/start.rs` |
| A15 | A start whose image cannot be pulled fails naming the image and the cluster's pull error | US4.2, FR-012 | example | PENDING | `kube_daemon_failures`; real: `kube_real_failures`; core (M2): `core/kube/start.rs` |
| A16 | A start whose pod cannot be scheduled fails with the scheduler's reason | US4.3, FR-012 | example | PENDING | `kube_daemon_failures`; real: `kube_real_failures`; core (M2): `core/kube/start.rs` |
| A17 | A start with a missing context or an unreachable cluster fails naming the context or the connection error | US4.4, FR-012 | example | PENDING | `kube_daemon_failures`; core (M2): `core/kube/start.rs` |
| A18 | Each start failure is shown within its phase bound while another daemon's state stays unchanged | US4.6, FR-012, FR-019, SC-003 | example | PENDING | `kube_daemon_failures`; real: `kube_real_failures` |
| A19 | The add form for runtime Kubernetes offers the kubeconfig's contexts, namespace, image, AI CLIs, resources, node selector, tolerations, volume size and allowed credentials | US5.1, FR-002, FR-004, FR-005, FR-018 | example | PENDING | `features_daemons` |
| A20 | Saving an edit of a running Kubernetes daemon changes nothing in the cluster and shows "applies on next start"; the next start of a pod whose settings differ offers Recreate or Keep | US5.2, FR-005 | example | PENDING | `features_daemons` |
| A21 | Stop on a running daemon deletes its pod, keeps its volume and shows `stopped` | US5.3, FR-014, SC-005 | example | PENDING | `core/kube/lifecycle.rs` (FakeCluster); real: `kube_real_lifecycle` |
| A22 | Reconnect on an `unreachable` daemon whose pod runs re-dials the tunnel and shows its running sessions (M8) | US5.4, FR-018 | example | PENDING | `features_daemons`, `kube_daemon_reconnect` |
| A23 | Remove asks for confirmation naming the pod, volume, secrets and this computer's SSH key files with 491's counts, and deletes nothing before it | US5.5, FR-015 | example | PENDING | `features_daemons` |
| A24 | A confirmed Remove leaves no pod, volume or secret the client created, and no key file of that daemon | US5.6, FR-015, SC-005 | example | PENDING | `core/kube/lifecycle.rs`; real: `kube_real_lifecycle` |
| A25 | On a throwaway kind cluster the suite passes pod creation, tunnel, session I/O, reconnect, stop and remove | US6.1, FR-021 | example | PENDING | `kube_real_endtoend`, `kube_real_reconnect`, `kube_real_lifecycle` |
| A26 | On that cluster RBAC denial, a pull failure an unschedulable pod and an AI CLI install failure are each reported within their bound | US6.2, FR-021, SC-003 | example | PENDING | `kube_real_failures` |
| A27 | `cargo test --workspace` without the feature runs no `kube_real_*` test; `mise run test-kube` and CI's `kube-cluster` job run them | US6.3, FR-021 | example | PENDING | `mise.toml` `test-kube`; `crates/micold-core/tests/ci_gate_covers_every_job.rs` |
| A28 | A start whose pod cannot install an AI CLI fails naming the CLI and its install error; an image without the SSH server contract fails naming the image and what it lacks | US4.5, FR-004, FR-012 | example | PENDING | `kube_daemon_start` (M6); core: `core/kube/start.rs` (`ImageContract`, M1c); real: `kube_real_failures` |

## Inner loop: unit behaviors

### `crates/micold-core/src/kube/settings.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1  | An empty context is refused naming "context"; a non-empty one is accepted without reading a kubeconfig | FR-018, FR-002 | example | DONE | `core/kube/settings.rs` tests::an_empty_context_is_refused_and_any_non_empty_one_is_accepted_unread |
| U2  | A namespace of 1 and of 63 DNS-1123 characters is accepted; 0, 64, or one with an uppercase letter is refused naming "namespace" | FR-018 | example | DONE | `core/kube/settings.rs` tests::a_namespace_must_be_a_dns_1123_label |
| U3  | ~~Image source Build with no registry is refused naming "Registry"; Pull with none is accepted~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U4  | ~~A registry with a tag or a digest is refused naming "registry"; `host[:port]/path` is accepted~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U5  | Node-selector keys and values outside Kubernetes label syntax are refused naming the field | FR-005, FR-018 | example | DONE | `core/kube/settings.rs` tests::node_selector_entries_follow_label_syntax |
| U6  | Tolerations parse `key[=value]:Effect` and `key:Exists:Effect` for the three effects; any other effect is refused naming "tolerations" | FR-005, FR-018 | example | DONE | `core/kube/settings.rs` tests::{tolerations_parse_both_forms_for_the_three_effects, a_toleration_with_another_effect_is_refused_naming_tolerations} |
| U7  | A volume size of exactly 1 GiB is accepted and one below it refused; the default is 10 GiB | FR-005 | example | DONE | `core/kube/settings.rs` tests::the_volume_is_at_least_one_gib_and_ten_by_default |
| U8  | Defaults: no image, no storage class, nothing allowed, volume 10 GiB | FR-004, FR-005, FR-008 | example | DONE | `core/kube/settings.rs` tests::defaults_are_no_image_no_storage_class_and_nothing_allowed |
| U117 | `Quantity::to_k8s` / `parse_k8s` round-trip `Mi`/`Gi` memory and millicore CPU; an unparseable quantity is refused naming the field | FR-005 | example | DONE | `core/kube/settings.rs` quantity_tests::{a_quantity_is_written_in_the_largest_exact_binary_unit, a_quantity_is_read_with_binary_decimal_or_no_suffix, writing_then_reading_is_the_same_quantity} |
| U118 | `image` is `None` by default and a given reference is kept as is; a moving tag is flagged as `sandbox::image` flags it | FR-004 | example | DONE | `core/kube/settings.rs` tests::{the_image_is_none_by_default_and_a_given_reference_is_kept_as_is, a_moving_image_tag_is_flagged_as_the_sandbox_flags_it, an_unparseable_stored_image_does_not_read} |
| U119 | `providers` defaults to every enabled AI CLI and may be empty | FR-004, US5.1 | example | DONE | `core/kube/settings.rs` tests::providers_default_to_the_enabled_clis_and_may_be_empty |

### `crates/micold-core/src/daemon_resources.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U9  | ~~The image source defaults to Build with no Dockerfile and the providers the Settings list enables~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U10 | ~~A Pull reference with a moving tag is flagged as `sandbox::image` flags it~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U11 | Resources default to no requests and the sandbox's CPU and memory limits; each of the four can be set alone | FR-005 | example | DONE | `core/daemon_resources.rs` tests::{resources_default_to_no_requests_and_the_sandbox_limits, each_of_the_four_can_be_set_alone} |
| U12 | A request equal to its limit is accepted; one above it is refused naming the request field | FR-005 | example | DONE | `core/daemon_resources.rs` tests::a_request_above_its_limit_is_refused_naming_the_request |

### `crates/micold-core/src/daemons.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U13 | `DaemonRuntime::Kubernetes` round-trips through settings as `{"kind":"kubernetes", ...}` with no settings-version bump | FR-020 | example | DONE | `core/tests/daemons_kube.rs` {the_runtime_round_trips_flat_with_its_kind, a_saved_document_keeps_its_version} |
| U14 | A reader that does not know `kubernetes` keeps it as `Unsupported` verbatim | FR-020 | example | DONE | `core/tests/daemons_kube.rs` an_unreadable_kubernetes_entry_is_kept_verbatim_as_unsupported |
| U15 | The Kubernetes runtime's label is "Kubernetes", it is startable, and its endpoint is `<context> / <namespace> / <pod>` | FR-001 | example | DONE | `core/tests/daemons_kube.rs` the_runtime_is_labelled_startable_and_shows_context_namespace_pod |
| U16 | A second daemon with an existing name is refused naming the first | FR-018, Edge: two daemons same namespace | example | DONE | `core/tests/daemons_kube.rs` a_second_daemon_with_the_same_name_is_refused |
| U17 | `Reconnecting` or `Unreachable` plus `Gone(reason)` becomes `Stopped` with that reason; Start from it becomes `Starting` and clears the reason | FR-011 | example | PENDING | |

### `crates/micold-core/src/kube/names.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U18 | The daemon key is the first 8 hex of the SHA-256 of context, namespace and name joined by NUL; equal triples give equal keys | FR-016, FR-023 | example | DONE | `core/kube/names.rs` the_key_is_eight_hex_of_the_nul_joined_triple |
| U19 | The slug is DNS-1123 and 40 characters at most, for names at and over that length | FR-016 | example | DONE | `core/kube/names.rs` the_slug_is_dns_1123_and_at_most_forty_characters |
| U20 | Resource names are `micold-<slug>-<key>` for the pod and its `-data`, `-ssh-access`, `-gitconfig`, `-git-credentials`, `-git-ssh`, `-signin-<cli>`, `-pull` siblings | FR-016, FR-024 | example | DONE | `core/kube/names.rs` resource_names_share_the_base_and_differ_by_suffix, the_access_secret_is_the_base_with_ssh_access |
| U21 | An object is ours only when both ownership labels match; one label alone is not ours | FR-016, Edge: pod created by someone else | example | DONE | `core/kube/names.rs` an_object_is_ours_only_when_both_labels_match |
| U120 | `all_secrets(&allowed)` lists the access secret first, then one name per allowed credential | FR-016, FR-024 | example | DONE | `core/kube/names.rs` all_secrets_lists_the_access_secret_first_then_one_per_allowed_credential |

### `crates/micold-core/src/protocol/auth.rs` and `crates/micold-daemon/src/server.rs` (revision 2: removed)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U22 | ~~An empty `MICOLD_TOKEN_PATH` with a non-loopback listen address stops the daemon at start (feature 027's rule, captured before the change)~~ (revision 2: removed) | FR-006 (K4 guard) | characterization | REMOVED |  |
| U23 | ~~A set token path gives `Token(path)`~~ (revision 2: removed) | FR-006 | example | REMOVED |  |
| U24 | ~~An empty path with `127.0.0.1` or `::1` gives `LoopbackNoToken`~~ (revision 2: removed) | FR-006 | example | REMOVED |  |
| U25 | ~~An empty path with a non-loopback address gives `AuthConfigError`~~ (revision 2: removed) | FR-006 | example | REMOVED |  |

### `crates/micold-core/src/kube/classify.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U26 | Every revision-2 `KubeFailure` variant except `Provision` (M5) displays exactly its contract §6 text: `ImageContract`, `HostKeyMismatch`, `StopTimeout`, `Api`, `Unavailable` added; `NoRuntime`, `Build`, `Push`, `VolumeTooSmall` gone | FR-012 | example | DONE | `core/kube/classify.rs` tests::{every_failure_reads_as_the_contract_says, an_api_failure_without_a_closer_reason_is_api} |
| U27 | ~~A failed build becomes `Build` carrying the output's last lines~~ (revision 2: removed) | FR-012 | example | REMOVED |  |
| U28 | ~~A push refused for login, permission or reachability becomes `Push { registry, error }`~~ (revision 2: removed) | FR-012, Edge: registry push refused | example | REMOVED |  |
| U29 | A pod `404` or `deletionTimestamp` is `Deleted`; `Evicted` is `Evicted { message }`; phase `Unknown` or a `404` node is `NodeLost { node }`; a running ready pod is not gone | FR-011 | example | PENDING | |
| U30 | A `403` becomes `Forbidden { verb, resource }`; a namespace `404` becomes `NamespaceMissing`; a connect error becomes `Unreachable { server, error }` | FR-012, Edge: namespace missing | example | PENDING | |
| U31 | `ErrImagePull`, `ImagePullBackOff` and `InvalidImageName` become `ImagePull` on first sight | FR-012 | example | PENDING | |
| U32 | `PodScheduled=False, reason=Unschedulable` becomes `Unschedulable { message }` | FR-012 | example | PENDING | |
| U33 | A `ProvisioningFailed` event or an unbound immediate claim becomes `ClaimNotBound` | FR-012, Edge: storage class absent | example | PENDING | |

### `crates/micold-core/src/connect.rs` (revision 2: removed)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U34 | ~~A boxed duplex stream as `Transport::Tunnel` completes the 491 handshake against a daemon stub~~ (revision 2: removed) | FR-006 | example | REMOVED |  |

### `crates/micold-core/src/kube/manifest.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U35 | No manifest has a `Service` or `Ingress` kind, `hostPort`, `hostNetwork`, `hostPID`, `privileged` or `containerPort` | FR-006, SC-006 | example | PENDING | |
| U36 | The pod has `restartPolicy: Always`, no service-account token and no service links | FR-006 | example | PENDING | |
| U37 | The container's env sets only `HOME`, `TMPDIR` and the access-secret directory; the access secret is mounted read-only at `/run/micold/ssh` with mode 0440 | FR-006, FR-024 | example | PENDING |  |
| U38 | The PVC is `ReadWriteOnce` with the volume size, mounted at `/data`, with an `emptyDir` at `/tmp`; the storage class is set when given and omitted when not | FR-005, FR-007 | example | PENDING | |
| U39 | Every object carries both ownership labels and the name annotation | FR-016 | example | PENDING | |
| U40 | Node selector and tolerations appear as given; none given → neither key | FR-005 | example | PENDING | |
| U41 | Each resource field set alone, all set, or none set yields exactly those keys under requests and limits | FR-005 | example | PENDING | |
| U42 | ~~The Build choice adds `kubernetes.io/arch=<arch>` and the pod image is the pushed digest reference~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U43 | The pod and container security contexts are non-root 10001, read-only root, no escalation, all capabilities dropped, `RuntimeDefault` seccomp | FR-009 | example | PENDING | |
| U44 | The access secret is always mounted; one more secret mount per allowed entry; nothing allowed → no further secret volume and no `imagePullSecrets` | FR-008, FR-009, FR-024, SC-004 | example | PENDING |  |
| U122 | `PodStamp` is the canonical JSON of image, resources, node selector, tolerations and allowed credentials, equal for equal settings, written as annotation `micold.dev/pod-settings` | FR-005 | example | PENDING |  |
| U123 | `drift` names the differing settings in form order; equal stamps give none | FR-005, US5.2 | example | PENDING |  |

### `crates/micold-core/src/kube/kubeconfig.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U45 | `KUBECONFIG` splits on `;` on Windows and `:` on macOS and Linux, keeping order | FR-022 | example | DONE | `core/kube/kubeconfig.rs` kubeconfig_splits_on_the_platform_separator_keeping_order |
| U46 | Without `KUBECONFIG` the candidate is `<home>/.kube/config`, home `%USERPROFILE%` on Windows and `$HOME` elsewhere | FR-022 | example | DONE | `core/kube/kubeconfig.rs` without_kubeconfig_the_candidate_is_the_home_config |
| U47 | The search path is process `PATH`, then the login shell's (not on Windows), then the fixed tool directories, first occurrence kept | FR-022, Edge: exec credential plugin | example | DONE | `core/kube/kubeconfig.rs` the_search_path_is_process_then_login_shell_then_tool_dirs_first_kept, windows_ignores_the_login_shell_path, no_path_at_all_leaves_the_tool_dirs |
| U48 | Login-shell output yields the text between the markers, and nothing without them | FR-022 | example | DONE | `core/kube/kubeconfig.rs` login_shell_output_yields_the_text_between_the_markers |

### `crates/micold-core/src/kube/start.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U49 | `StartPlan::kube()` is the 11 phases `Config, Preflight, Volume, Secrets, Pod, Schedule, Pull, Tunnel, Copy, Install, Connect` | FR-004, FR-012 | example | PENDING |  |
| U50 | Phase bounds: `Pull` and `Install` 15 min, every other 120 s; at its bound under a paused clock a phase fails `PhaseTimeout { phase }`, and not one tick before | FR-012 | example | PENDING |  |
| U51 | A missing namespace fails `NamespaceMissing` and creates nothing | FR-012, Edge: namespace missing | example | PENDING | |
| U52 | The PVC is created on the first start and reused on the next | FR-003, FR-007 | example | PENDING | |
| U53 | A `409` on an object that is ours is adopted and not recorded as created | FR-003, FR-023 | example | PENDING | |
| U54 | A `409` on an object that is not ours fails `Conflict { kind, name }` | FR-016 | example | PENDING | |
| U55 | A pod being deleted is waited out, then created | FR-003 | example | PENDING | |
| U56 | ~~A volume size below the existing claim's is refused naming the claim~~ (revision 2: removed) | FR-005 | example | REMOVED |  |
| U57 | Any failure deletes this attempt's created objects in reverse by name and UID, the PVC only if this attempt created it, adopted objects never | FR-013 | example | PENDING | |
| U58 | Allowed secrets are created or adopted; a withdrawn secret that is ours is deleted by name and UID; one not ours is untouched | FR-008 | example | PENDING | |
| U59 | Nothing allowed → no secret call at all; credential source files are read only in the `Secrets` phase | FR-008 | example | PENDING | |
| U60 | Preflight reviews `create pods`, `create persistentvolumeclaims`, `create pods/portforward`, `delete pods`, `create secrets` and `update secrets` always (the access secret); a denial fails `Forbidden` before any create | FR-012, US4.1 | example | PENDING |  |
| U61 | No kubeconfig, a missing context and a failing exec plugin fail the `Config` phase with `NoKubeconfig`, `ContextMissing` and `ExecPlugin` | FR-012, FR-022 | example | PENDING | |
| U62 | A pull failure, an unschedulable pod or an unbound claim fails the start on first sight, before the phase bound | FR-012 | example | PENDING | |
| U126 | The `Secrets` phase creates the access secret when absent, adds this client's key by `replace` when missing, re-reads on a `409` and keeps the other client's key, and writes nothing when the key is present | FR-023, FR-024 | example | PENDING |  |
| U127 | Rollback removes the key this attempt added (re-read and `replace`), and deletes the access secret only if this attempt created it | FR-013, FR-024 | example | PENDING |  |
| U128 | A running pod whose stamp differs gives `StartOutcome::Drift { fields }`; `Recreate` deletes and recreates the pod keeping the volume; `Keep` connects to it as is | FR-005, US5.2 | example | PENDING |  |
| U129 | The `Tunnel` phase opens one `port_forward(namespace, pod, SSH_PORT)` stream and reads an identification line starting `SSH-2.0-` within its bound; none, a non-SSH line or an exited container fails `ImageContract` | FR-004, FR-006, US4.5 | example | PENDING |  |
| U138 | **(#806)** `Copy` passes the SSH target (`127.0.0.1`, relay port, user, key file, known_hosts file, `HostKeyAlias micold-kube-<key>`) and `daemon_env(&allowed)`; a host key mismatch fails `HostKeyMismatch`, a copy error `Provision` | FR-004, FR-006, FR-024 | example | PENDING |  |
| U139 | **(#806)** `Install` passes `providers` and the volume's install root; output lines reach the activity log; Cancel → `Cancelled` and rollback; 15 min → `PhaseTimeout { Install }`; a failed install → `Provision` naming the CLI | FR-004, FR-012, US4.5 | example | PENDING |  |
| U140 | **(#806)** `Connect` bridges the 491 handshake through the relay to the daemon started over SSH | FR-006 | example | PENDING |  |

### `crates/micold-core/src/kube/fake.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U121 | `FakeCluster::replace` with the current `resourceVersion` stores the object under a new one; a stale one answers `409` and changes nothing; `hold_on_delete` keeps a deleted pod terminating until released | FR-023 | example | DONE | `core/kube/fake.rs` tests::{replace_with_the_current_version_stores_it_under_a_new_one, replace_with_a_stale_version_is_a_conflict_and_changes_nothing, a_held_pod_stays_terminating_after_its_delete, port_forward_records_the_pod_and_port} |

### `crates/micold-core/src/kube/access.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U124 | `AccessSecret::parse` / `to_data` round-trip `ssh_host_ed25519_key`, `ssh_host_ed25519_key.pub` and `authorized_keys` (one line per client, comment `micold-client:<client id>`) | FR-024 | example | PENDING |  |
| U125 | `with_client` is idempotent and replaces a line with the same client id; `without_client` removes only that client's line | FR-015, FR-024 | example | PENDING |  |

### `crates/micold-core/src/kube/lifecycle.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U63 | Stop deletes the pod by name and UID with grace 10 s, waits until it is gone within 120 s, and keeps PVC and secrets | FR-014 | example | PENDING | |
| U64 | Remove deletes pod, secrets and PVC that are ours, counting a `404` as done | FR-015 | example | PENDING | |
| U65 | Forget-locally lists the deterministic resource names, the access secret among them, and makes no cluster call | FR-015, Edge: remove when unreachable | example | PENDING |  |

### `crates/micold-core/src/kube/credentials.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U66 | `GitCredentials` maps to one secret mounted at `/run/micold/creds/git-credentials`, with `GIT_CONFIG_*` for the store helper in `daemon_env` | FR-008 | example | PENDING |  |
| U67 | `GitSshKey` maps to one secret at `/run/micold/creds/ssh/{id,known_hosts}`, with `GIT_SSH_COMMAND` (`IdentitiesOnly=yes`, `StrictHostKeyChecking=yes`) in `daemon_env`; known-hosts lines for the remote's host only | FR-008 | example | PENDING |  |
| U68 | `GitConfig` and `SignIn(cli)` mount by subPath at `/data/home/.gitconfig` and the provider's auth file under `HOME` | FR-008 | example | PENDING | |
| U69 | `RegistryPull` maps to one `kubernetes.io/dockerconfigjson` secret `<base>-pull` in `imagePullSecrets` | FR-008 | example | PENDING | |
| U70 | A missing source file fails `CredentialSource { credential, path }` | FR-008 | example | PENDING | |
| U136 | `daemon_env(&allowed)` returns exactly the env the allowed credentials need, and nothing when nothing is allowed | FR-008 | example | PENDING |  |

### `crates/micold-core/src/daemon_image/containerfile.rs` and `tag.rs` (revision 2: removed)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U71 | ~~The rendered Containerfile installs exactly the selected providers at the versions pinned in `packaging/sandbox/Containerfile`~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U72 | ~~The rendered Containerfile runs as UID/GID 10001 with `HOME`, `XDG_DATA_HOME`, `TMPDIR` set, the daemon as entrypoint, and no credential `COPY` or `ENV`~~ (revision 2: removed) | FR-004, FR-008, FR-009 | example | REMOVED |  |
| U73 | ~~An `AiCli` with no pinned `ARG` fails the render test~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U74 | ~~The tag is `<registry>/micold-daemon:<version>-<12 hex>`, equal for equal inputs and different when daemon bytes, Containerfile, provider set or arch change; never a moving tag~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U75 | ~~The pod image of a pushed image is `<registry>/micold-daemon@<digest>`~~ (revision 2: removed) | FR-004 | example | REMOVED |  |

### `crates/micold-core/src/sandbox/cli.rs` (revision 2: removed)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U76 | ~~`build_image` runs `<rt> build --platform linux/<arch> -f <file> -t <tag> <context>`, with `--build-context micold=<tmp>` for a user Dockerfile~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U77 | ~~`push_image` runs `<rt> push <tag>`~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U78 | ~~`repo_digest` picks the `<registry>/micold-daemon` entry; none fails `Push` "the registry returned no digest"~~ (revision 2: removed) | FR-004, FR-012 | example | REMOVED |  |

### `crates/micold-core/src/kube/clone.rs`, `protocol/messages.rs`, `crates/micold-daemon/src/repo_clone.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U79 | The clone remote is `origin`, else the only remote; none is `NoRemote`; several without `origin` is `Ambiguous(names)` | FR-007, Edge: project without a remote | example | PENDING | |
| U80 | The clone target is `/data/repos/<slug>-<hash8(url)>`, the same for the same URL | FR-007 | example | PENDING | |
| U81 | Git stderr classifies as `AuthRequired { remote }`, `DiskFull`, `Unreachable { remote }` or `Other`, each with its contract §6 text | FR-007, Edge: remote needs a credential, Edge: volume full | example | PENDING | |
| U82 | `RepoClone`, `RepoCloned`, `RepoCloneFailed` round-trip and `PROTOCOL_VERSION` is 51 | FR-007, FR-017 | example | PENDING | |
| U83 | The daemon clones into a `.partial-` directory and renames it on success; an existing destination is fetched | FR-007 | example | PENDING | |
| U84 | An auth failure leaves no directory behind | FR-007, Edge: remote needs a credential | example | PENDING | |
| U85 | A destination outside `MICOLD_REPOS_ROOT` is refused | FR-007, FR-020 | example | PENDING | |

### `crates/micold-client/src/kube/live.rs` (`image.rs` rows: revision 2, removed)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U86 | A relative exec command is rewritten to its first match on the search path; an absolute one is left; none found fails `ExecPlugin { command, error, search_path }` | FR-022 | example | PENDING | |
| U87 | Contexts are listed from the merged files in order and carry no `users[]` data | FR-002 | example | PENDING | |
| U88 | API errors keep code, reason and message; `409` on create is `AlreadyExists`; `404` on delete is done; a connection error is `Unreachable` | FR-012 | example | PENDING | |
| U89 | Delete parameters carry the UID precondition and the grace seconds | FR-016 | example | PENDING | |
| U90 | ~~The daemon binary is the client's own on a Linux client of that arch, else the shipped `linux-<arch>` one; absent fails naming the arch~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U91 | ~~A tag already present skips the build; the push always runs; the pod uses the digest~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U92 | ~~Build output lines reach the daemon's diagnostics log~~ (revision 2: removed) | FR-004, FR-012 | example | REMOVED |  |
| U93 | ~~Cancel kills the build and fails `Cancelled`; the 30-minute bound kills it and fails `PhaseTimeout { Image }`~~ (revision 2: removed) | FR-012 | example | REMOVED |  |
| U94 | ~~No container runtime installed fails `NoRuntime`~~ (revision 2: removed) | FR-012, Edge: no local container runtime | example | REMOVED |  |

### `crates/micold-client/src/kube/relay.rs` and `keys.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U130 | `Relay::open` listens on `127.0.0.1:0`; each accepted connection opens one `port_forward(.., SSH_PORT)` and copies bytes both ways; dropping the relay closes the listener and its connections | FR-006 | example | PENDING |  |
| U131 | Key files `id_ed25519`, `id_ed25519.pub`, `known_hosts`, `client_id` per daemon key are owner-only; `known_hosts` is one line `micold-kube-<daemon key> <host public key>`; `client_id` is stable across loads; `delete` removes them | FR-024 | example | PENDING |  |

### `crates/micold-client/src/features/daemons.rs` (reducer)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U95 | Opening a Kubernetes daemon in Edit shows the merged kubeconfig's contexts, or "no kubeconfig found; looked in <paths>" | FR-002, FR-022 | example | PENDING |  |
| U96 | Save refuses an invalid Kubernetes field naming it; a saved row shows "Kubernetes" and the endpoint | FR-018, FR-001 | example | PENDING | |
| U97 | The container form's limits fields behave as before the extraction into `container_fields.rs` | FR-005 (K1) | characterization | PENDING |  |
| U98 | ~~Image source defaults to Build with the enabled providers; Build without a registry is refused naming "Registry"~~ (revision 2: removed) | FR-004 | example | REMOVED |  |
| U99 | The four resource fields are edited on their own; a request above its limit is refused naming the request field | FR-005 | example | PENDING | |
| U100 | A worktree of a project with no remote bound to a Kubernetes daemon is refused with the reason and sends nothing | FR-007 | example | PENDING | |
| U101 | A project with a remote sends `RepoClone` before `WorktreeCreate`, showing "Cloning <repo>" | FR-007 | example | PENDING | |
| U102 | The bound worktree's identity is `micold-remote://<daemon-id>/data/repos/…` | FR-007 | example | PENDING | |
| U103 | A `stopped` row shows its reason | FR-011 | example | PENDING | |
| U104 | The allowed-credentials list starts with nothing checked; SSH key and registry auth need a file | FR-008 | example | PENDING | |
| U105 | Stop with running sessions confirms naming how many end | FR-014 | example | PENDING | |
| U106 | Remove with the cluster unreachable offers forget-locally naming what stays; the access secret keeps this client's key as authorized text | FR-015, FR-024, Edge: remove when unreachable | example | PENDING |  |
| U107 | Node selector, tolerations, volume size and storage class each refuse an invalid value naming the field | FR-005, FR-018 | example | PENDING | |
| U108 | Logs fetches the last 200 lines on demand | FR-018 | example | PENDING | |
| U134 | The Add form's runtime choices omit Kubernetes until the mechanism is wired (M6), then list it | FR-018, US5.1 | example | PENDING |  |
| U135 | The AI CLI checkboxes default to the enabled providers; the form shows the volume notice | FR-004, FR-005, US5.1 | example | PENDING |  |
| U141 | **(#806)** A `Drift { fields }` start shows "The running pod differs from the settings (<fields>). Recreate it? …" with Recreate and Keep | FR-005, US5.2 | example | PENDING |  |
| U143 | Remove and Forget locally delete this daemon's key files | FR-015, FR-024 | example | PENDING |  |

### `crates/micold-client/src/ui/material/key_value_list.rs` and `toleration_list.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U109 | The key/value editor parses and formats `key=value` rows, round trip equal | FR-005 | example | PENDING | |
| U110 | The toleration editor parses and formats each accepted toleration form, round trip equal | FR-005 | example | PENDING | |

### `crates/micold-client/src/daemon.rs` (actor, over `FakeCluster`)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U111 | Two actors starting the same daemon on one cluster create exactly one pod and both connect | FR-023, Edge: two clients start at once | example | PENDING | `kube_daemon_start` |
| U112 | A version-mismatched handshake shows `VersionMismatch` and starts no session | FR-017 | example | PENDING | `kube_daemon_start` |
| U113 | App launch with the cluster unreachable delays neither the window nor other daemons | FR-019 | example | PENDING | `kube_daemon_start` |
| U114 | A lookup answering 5xx or timing out keeps `Reconnecting` with 489's back-off | FR-011 | example | PENDING | `kube_daemon_reconnect` |
| U115 | After another client's Remove, Start creates a new, empty PVC and a pod | FR-023 | example | PENDING | `kube_daemon_reconnect` |
| U116 | A failed start shows `Unreachable` with its reason and does not retry; its full output reaches diagnostics | FR-012 | example | PENDING | `kube_daemon_failures` |
| U133 | Start of a Kubernetes daemon in a build without the mechanism fails at once with `Unavailable`, makes no cluster call, and the row shows `unreachable` with that reason | FR-012 | example | PENDING |  |

## Invariants and edge cases still to place

None. Every edge case in `spec.md` maps to a behavior above or to Out of scope.

## Out of scope

- Session input and output over the tunnel at unit level: the 489/491 session ops are unchanged and
  already tested over `Transport`; A3 is proved on a real pod only.
- Sign-in-requiring CLIs reporting a missing sign-in (US3.2's second half): the CLIs' own behaviour,
  observed in `kube_real_security`, not a unit of this feature.
- Volume full during worktree creation (Edge): covered by U81's `DiskFull` text; the client kills no
  session (unchanged 489 behaviour).
- Branch already checked out in another daemon's worktree (Edge): spec 491 FR-016, unchanged.
- Creating clusters, namespaces, RBAC or storage classes; Service, Ingress, LoadBalancer or NodePort
  exposure; moving worktrees between daemons; autoscaling or several pods per daemon; one volume
  shared by daemons: `spec.md` Out of Scope.
- Property-based tests: the profile has no property library; invariants are sampled at their
  boundaries (U2, U7, U12, U19, U50).

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- Unit test in `src/`: `scripts/build-lock.sh cargo test -p {crate} --lib {module}::tests::{name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace`
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets`
- Acceptance: `scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1`
  (this feature's real suite: `cargo test -p micold-client --features kube-real-runtime kube_real_ --no-fail-fast -- --test-threads=1`, T095)
- Coverage: none (no cargo-llvm-cov)
- Mutation: none (no cargo-mutants)
- Assert on the `N passed` count, never the exit code alone: a filter matching nothing exits 0.
