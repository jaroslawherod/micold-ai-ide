# Data model: Kubernetes session daemon runtime (#686)

New types live in `micold-core` (render-free, testable) unless marked. "New" = does not exist on
`main`; "changed" = an existing type gains a variant or field. Decisions: [research.md](./research.md).
**Revision 2** (owner decisions 8-11): no image build, push or registry; the pod runs an SSH image and
the shared mechanism of [contracts/shared-ssh-image.md](./contracts/shared-ssh-image.md) (#806) copies
the daemon in and installs the AI CLIs. Types marked **(#806)** are that mechanism's and are named
here only as #686 uses them; #806's plan owns their final names.

## Persisted (settings.json, `daemons[]`)

### `DaemonRuntime::Kubernetes(KubeSettings)` — changed (`micold-core/src/daemons.rs`)

Stored as `{"kind":"kubernetes", ...}` by the existing hand-written `DaemonRuntime` serde, additive,
no `settings_version` bump (489 convention). `label()` = "Kubernetes"; `is_startable()` = true;
endpoint shown = `<context> / <namespace> / <pod name>` (FR-001). An older build reads it as
`Unsupported` and writes it back verbatim (491).

### `KubeSettings` — new (`micold-core/src/kube/settings.rs`)

| Field | Type | Rule (refused naming the field, FR-018) |
|---|---|---|
| `context` | `KubeContextName` | non-empty; existence in kubeconfig checked at start, not on save |
| `namespace` | `Namespace` | DNS-1123 label, 1-63 chars |
| `image` | `Option<ImageRef>` (existing `sandbox::image::ImageRef`) | `None` = #806's pinned SSH image (FR-004); a moving tag is allowed but flagged as for the sandbox |
| `providers` | `BTreeSet<AiCli>` | AI CLIs to install (FR-004, FR-018); default = the providers enabled in Settings; may be empty (terminals only) |
| `resources` | `DaemonResources` (below; shared with #806) | each of the four set or unset on its own; a request above its limit refused naming the request field |
| `node_selector` | `BTreeMap<LabelKey, LabelValue>` | Kubernetes label syntax |
| `tolerations` | `Vec<Toleration>` | `key[=value]:Effect` or `key:Exists:Effect`; `Effect` ∈ NoSchedule, PreferNoSchedule, NoExecute |
| `volume_size` | `Quantity` (bytes) | ≥ 1 GiB, default 10 GiB; applies only when the volume is created (FR-005): the form shows a notice when it is edited on a daemon that was started before |
| `storage_class` | `Option<String>` | none = cluster default |
| `allowed` | `BTreeSet<KubeCredential>` | empty by default (rule SP-1 of the sandbox, kept) |

Removed from revision 1 (unmerged M1a code, reverted): `image: DaemonImageSource`, `registry:
Option<RegistryRepo>`, `arch: NodeArch` (the daemon's architecture is found by 489's probe).

Duplicate rule: daemon names stay unique in the registry (491); `RegistryError::DuplicateName` is
field-less and the form text names the name (ledger decision 6).

### `KubeCredential` — new (`micold-core/src/kube/credentials.rs`)

`GitConfig` | `GitCredentials` | `GitSshKey { key: PathBuf }` | `SignIn { cli: AiCli }` |
`RegistryPull { auths: PathBuf }`. Each maps to exactly one secret (`ResourceNames::secret`) and one
mount, or for `RegistryPull` one `imagePullSecrets` entry. Source files are read only during the
`Secrets` phase of a start, never stored. A missing source file fails start naming the file and the
credential (`CredentialSource`). `daemon_env(&allowed) -> Vec<(String, String)>`: the daemon's
environment for the shared start step (`MICOLD_IDLE_STOP=off`, `MICOLD_REPOS_ROOT=/data/repos`, and
each allowed credential's `GIT_CONFIG_*` / `GIT_SSH_COMMAND` variables; research K7).

### `DaemonResources` — new, shared with #806 (`micold-core/src/daemon_resources.rs`)

| Field | Type | Default | Kubernetes mapping (FR-005) |
|---|---|---|---|
| `cpu_request` | `Option<MilliCpus>` | `None` | `resources.requests.cpu` (`<n>m`), omitted when `None` |
| `cpu_limit` | `Option<MilliCpus>` | `sandbox::DEFAULT_MILLI_CPUS` | `resources.limits.cpu`, omitted when `None` |
| `memory_request` | `Option<Bytes>` | `None` | `resources.requests.memory` (`Quantity::to_k8s`), omitted when `None` |
| `memory_limit` | `Option<Bytes>` | sandbox's default memory | `resources.limits.memory`, omitted when `None` |

Moved from revision 1's `daemon_image/mod.rs` (that module is deleted with `DaemonImageSource`).
`MilliCpus` and `Bytes` are the existing `sandbox` newtypes. A container daemon maps the limits to
its `ResourceBudget` ceilings when #806 adopts this type.

### Client-side key files — new (`micold-client/src/kube/keys.rs`, owner-only files)

Per daemon key (K5), under the app's data directory, written with `owner_only.rs`: `id_ed25519`
(this client's private key), `id_ed25519.pub`, `known_hosts` (one line: the pod's host public key
under `HostKeyAlias = micold-kube-<daemon key>`), `client_id` (a random id, the comment of this
client's line in `authorized_keys`). Never in settings, the cluster or logs (FR-008). Deleted by
Remove (FR-015).

## Derived (pure, not persisted)

### `DaemonKey` and `ResourceNames` — new (`kube/names.rs`)

`DaemonKey::of(context, namespace, name)` = first 8 of `protocol::hashing::sha256_hex` over the
three, NUL-separated. `ResourceNames { pod, pvc, access_secret, secret(KubeCredential), pull_secret }`
from `slug(name)` + key (K5); `all_secrets(&allowed)` lists every secret name a daemon may own
(access secret first). `labels()`; `is_ours(labels, key) -> bool`.

### `AccessSecret` — new (`kube/access.rs`)

The SSH access secret's content (K3): `host_key: KeyPair`, `authorized: Vec<AuthorizedKey { client_id,
public_key }>`. Pure: `parse(data)`, `to_data()`, `with_client(client_id, key) -> (Self, changed)`
(idempotent; replaces a line with the same `client_id`), `without_client(client_id)`. A start that
adds a key sends a `replace` guarded by the `resourceVersion` it read; on `409` it re-reads and
re-applies `with_client`, so a concurrent addition by another client is kept (FR-023). The key pairs
come from the **(#806)** key generation through the `KeyGen` port (below).

### `PodStamp` and `PodDrift` — new (`kube/manifest.rs`)

`PodStamp` = the canonical JSON of the settings that shape the pod (image reference, resources, node
selector, tolerations, allowed credentials), written to the pod annotation `micold.dev/pod-settings`.
`drift(stamp_on_pod, stamp_now) -> Vec<&'static str>` names the settings that differ, in form order
(FR-005, US5.2); empty = reuse.

### `Manifests` — new (`kube/manifest.rs`)

Pure builders returning `serde_json::Value`: `pod(&KubeSettings, &ResourceNames, image, &PodStamp)`,
`pvc(..)`, `secret(name, data, labels)`, `access_reviews() -> Vec<(verb, resource)>`. Constants:
`SSH_PORT = 2222`, `IMAGE_USER_UID = 10001`, mount paths (K7). Invariants asserted by tests: no
`Service`/`Ingress` kind ever produced; no `hostPort`, `hostNetwork`, `hostPID`, `privileged`,
`containerPort`; `automountServiceAccountToken: false`; every secret volume is the access secret or
one `allowed` entry (FR-009, SC-004); both ownership labels on every object (FR-016).

### `StartPhase`, `StartPlan` and `Attempt` — new (`kube/start.rs`)

`Config | Preflight | Volume | Secrets | Pod | Schedule | Pull | Tunnel | Copy | Install | Connect`,
with `bound()`: `Pull` 15 min, `Install` 15 min, every other 120 s (FR-012), and `activity()` (the
row text). `StartPlan::kube()` is the fixed list; `Copy`, `Install` and `Connect` call the **(#806)**
provisioning and are **blocked on #806**: until then the sequence ends after `Tunnel`, which is a
port-forward to `SSH_PORT` through the relay that reads an SSH identification line (`SSH-2.0-…`)
within its bound (else `ImageContract`). `Attempt { created: Vec<CreatedResource>, added_key:
Option<ClientId> }` records what this attempt created or added, for rollback (FR-013). `StartOutcome`
= `Ready { tunnel }` | `Drift { fields }` (the pod runs with other settings: ask the user, FR-005) |
`Failed(KubeFailure)`. In a build without the mechanism (no pinned image reference, no live
`KeyGen`), the actor's Start of a Kubernetes daemon found in settings fails at once with
`KubeFailure::Unavailable` ("Kubernetes daemons need the shared SSH image (#806), which this build
does not include"), creates nothing and shows `unreachable` with that reason; the sequence runs only
in tests until then.

### `KubeFailure` — new (`kube/classify.rs`)

`ContextMissing { context, looked_in }`, `NoKubeconfig { looked_in }`, `Unreachable { server, error }`,
`ExecPlugin { command, error, search_path }`, `NamespaceMissing { namespace }`,
`Forbidden { verb, resource }`, `ImagePull { image, message }`, `Unschedulable { message }`,
`ClaimNotBound { claim, message }`, `Conflict { kind, name }`, `CredentialSource { credential, path }`,
`ImageContract { image, missing }`, `HostKeyMismatch { pod }`, `PhaseTimeout { phase }`, `Cancelled`,
`StopTimeout { pod }`, `Api { status, message }`, `Unavailable` (before #806, above), and `Provision(ProvisionFailure)` wrapping the
**(#806)** copy, start and install failures (their texts are #806's; install names the CLI).
Removed from revision 1: `NoRuntime`, `Build`, `Push`, `VolumeTooSmall`. `Display` gives the
user-facing reason (contract strings). Classifiers: `from_status(code, body)`,
`from_pod(&PodStatusView, &[EventView])`, `from_pvc(..)`.

### `PodGone` — new (`kube/classify.rs`)

`Deleted | Evicted { message } | NodeLost { node }` from `PodLookup` (`NotFound`, a pod view with
`deletionTimestamp`, `status.reason`, `phase`). `None` when the pod is present and running.

### `KubeconfigCandidates` and `ExecSearchPath` — new (`kube/kubeconfig.rs`)

`candidates(kubeconfig_env: Option<&OsStr>, home: &Path, platform: ClientPlatform) -> Vec<PathBuf>`;
`search_path(process_path, login_shell_path, home, platform) -> Vec<PathBuf>` (order and de-dup
fixed); `parse_login_shell_output(stdout) -> Option<OsString>` (marker-delimited).

### `CloneTarget` and `CloneFailure` — new (`kube/clone.rs`; K8; blocked on #806)

`remote_for(remotes: &[(name, url)]) -> Result<RemoteUrl, NoRemote | Ambiguous(Vec<name>)>`;
`CloneTarget::for_url(url) -> /data/repos/<slug>-<hash8>`; `CloneFailure` from `git` stderr:
`AuthRequired { remote }`, `DiskFull`, `Unreachable { remote }`, `Other(String)`.

## Ports

### `Cluster` — new (`kube/mod.rs`); live `LiveCluster` (`micold-client/src/kube/live.rs`)

`create`, `get`, `replace(namespace, kind, name, resource_version, manifest)` (**new in revision 2**:
`409` → `ApiFailure` with status 409, for the access secret), `delete` (by name and UID),
`namespace_exists`, `access_review`, `pod_status`, `events`, `port_forward(namespace, pod, port)`,
`logs`. Removed: `node_arch` (revision 1's build choice). `FakeCluster` (`kube/fake.rs`) scripts each
and records calls; `hold_on_delete` keeps a deleted pod terminating for the Stop wait.

### `KeyGen` — new (`kube/access.rs`), implemented by **(#806)**

`generate(comment) -> KeyPair { private_openssh, public_line }`. The fake in tests returns fixed
pairs. The live implementation is #806's key generation, wired in the blocked milestone.

### Relay — new (`micold-client/src/kube/relay.rs`)

`Relay::open(cluster, namespace, pod) -> Relay { port }`: a listener on `127.0.0.1:0`; each accepted
connection opens one `port_forward(.., SSH_PORT)` stream and copies both ways. Closed on disconnect,
stop or remove.

## State machine delta (491/489 `DaemonState`)

| From | Event | To | Note |
|---|---|---|---|
| any established | `Lost(_)` | `Reconnecting` | unchanged (489) |
| `Reconnecting`, `Unreachable` | `Gone(reason)` — **new event** | `Stopped` | sessions → `lost to its daemon`; actor ends; never re-dials (FR-011, FR-023) |
| `Starting` | `StartFailed(reason)` | `Unreachable` with the failure reason | unchanged; Kubernetes actors make no further attempts (as 489's remote rule, R7) |
| `Starting` | `Drift(fields)` — **new event** | `Starting` (awaiting choice) | the row asks "Recreate pod" / "Keep running pod" (FR-005) |
| `Stopped` | `Start` | `Starting` | creates a new pod on the kept volume (story 2.4) |

`DaemonView` gains `stopped_reason: Option<String>` (set by `Gone`, cleared by `Start`) and reuses
`activity` for "Pulling image", "Opening tunnel", "Copying daemon", "Installing <cli>", "Cloning
<repo>".

## Protocol delta (`PROTOCOL_VERSION` 48 → 49; blocked on #806)

`ClientMsg::RepoClone { url: String, dest: String }` → `DaemonMsg::RepoCloned { dest }` |
`DaemonMsg::RepoCloneFailed { dest, failure: CloneFailureWire }`. Handled in
`micold-daemon/src/repo_clone.rs` (new, dispatched from `ops.rs`); `dest` must be under the daemon's
`MICOLD_REPOS_ROOT` (`/data/repos` in the pod; refused otherwise). Two requests for one `dest` are
serialised in the daemon (one in-process lock per `dest`): the second waits and then fetches
(FR-023).

## Removed from revision 1

`DaemonImageSource`, `ImageInputs`, `ImageTag`, `PushedImage` (`daemon_image/`); `RegistryRepo`,
`NodeArch`; `ListenerAuth` (`protocol/auth.rs`, K4); `Transport::Tunnel` (`connect.rs`, K2).
