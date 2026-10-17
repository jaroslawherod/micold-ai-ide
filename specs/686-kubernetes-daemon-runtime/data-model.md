# Data model: Kubernetes session daemon runtime (#686)

New types live in `micold-core` (render-free, testable) unless marked. "New" = does not exist today;
"changed" = an existing type gains a variant or field. Decisions: [research.md](./research.md).

## Persisted (settings.json, `daemons[]`)

### `DaemonRuntime::Kubernetes(KubeSettings)` — changed (`micold-core/src/daemons.rs`)

Stored as `{"kind":"kubernetes", ...}` by the existing hand-written `DaemonRuntime` serde
(`daemons.rs` ~L758), additive, no `settings_version` bump (489 convention). `label()` =
"Kubernetes"; `is_startable()` = true; endpoint shown = `<context> / <namespace> / <pod name>`
(FR-001). An older build reads it as `Unsupported` and writes it back verbatim (491).

### `KubeSettings` — new (`micold-core/src/kube/settings.rs`)

| Field | Type | Rule (refused naming the field, FR-018) |
|---|---|---|
| `context` | `KubeContextName` | non-empty; existence in kubeconfig checked at start, not on save (edge: context changes) |
| `namespace` | `Namespace` | DNS-1123 label, 1-63 chars |
| `image` | `DaemonImageSource` (below) | — |
| `registry` | `Option<RegistryRepo>` | required when `image` is `Build` (FR-004); `host[:port]/path`, no tag, no digest |
| `arch` | `NodeArch` (`Amd64` default, `Arm64`) | build choice only |
| `budget` | `sandbox::ResourceBudget` (existing; #806's resources) | requests = limits from the budget; `pids` and `storage_bytes` ignored for Kubernetes and hidden in its form |
| `node_selector` | `BTreeMap<LabelKey, LabelValue>` | Kubernetes label syntax |
| `tolerations` | `Vec<Toleration>` | `key[=value]:Effect` or `key:Exists:Effect`; `Effect` ∈ NoSchedule, PreferNoSchedule, NoExecute |
| `volume_size` | `Quantity` (bytes) | ≥ 1 GiB, default 10 GiB (Assumptions); a smaller value than an existing claim is refused at start naming the claim |
| `storage_class` | `Option<String>` | none = cluster default |
| `allowed` | `BTreeSet<KubeCredential>` | empty by default (rule SP-1 of the sandbox, kept) |

Duplicate rule: daemon names stay unique in the registry (491), so "same context, namespace and name"
cannot be configured twice on one client (edge case); `RegistryError::DuplicateName` names the first.

### `KubeCredential` — new (`micold-core/src/kube/credentials.rs`)

`GitConfig` | `GitCredentials` | `GitSshKey { key: PathBuf }` | `SignIn(AiCli)` |
`RegistryPull { auths: PathBuf }`. Each maps to exactly one secret (`secret_name`) and one mount
(`mount`), or for `RegistryPull` one `imagePullSecrets` entry. Source files are read only during
the `Secrets` phase of a start, never stored. A missing source file fails start naming the file and
the credential.

### `DaemonImageSource` — new, shared with #806 (`micold-core/src/daemon_image/mod.rs`)

| Variant | Fields | Notes |
|---|---|---|
| `Build` (default) | `dockerfile: Option<PathBuf>`, `providers: BTreeSet<AiCli>` | `None` = the app's template; providers default to the CLIs the Settings provider list enables |
| `Pull` | `reference: ImageRef` (existing `sandbox::image::ImageRef`) | used as given; a moving tag is allowed but flagged as for the sandbox (FR-024b) |

## Derived (pure, not persisted)

### `DaemonKey` and `ResourceNames` — new (`kube/names.rs`)

`DaemonKey::of(context, namespace, name)` = 8 hex of SHA-256 over the three, NUL-separated.
`ResourceNames { pod, pvc, secret(KubeCredential), pull_secret }` from `slug(name)` + key (K5).
`Ownership { managed_by, daemon_key }` labels; `is_ours(labels) -> bool`.

### `ImageInputs` and `ImageTag` — new (`daemon_image/tag.rs`)

`ImageInputs { daemon_sha256, containerfile_sha256, providers, arch }` →
`ImageTag(<registry>/micold-daemon:<version>-<12 hex>)`. `PushedImage { tag, digest }` →
`pod_image() = <registry>/micold-daemon@<digest>`.

### `Manifests` — new (`kube/manifest.rs`)

Pure builders returning `serde_json::Value`: `pod(&KubeSettings, &ResourceNames, image, &allowed)`,
`pvc(..)`, `secret(name, data)`, `access_reviews(&allowed) -> Vec<Value>`. Invariants asserted by
tests: no `Service`/`Ingress` kind ever produced; no `hostPort`, `hostNetwork`, `hostPID`,
`privileged`; `automountServiceAccountToken: false`; every volume mount of a secret corresponds to one
`allowed` entry (FR-009, SC-004); both ownership labels on every object (FR-016).

### `StartPhase` and `PhaseBound` — new (`kube/start.rs`)

`Config | Preflight | Image | Volume | Secrets | Pod | Schedule | Pull | Tunnel`, with
`bound()`: `Image` 30 min, `Pull` 15 min, every other 120 s (FR-012). `StartPlan` is the sequence
for a settings value (Pull choice omits `Image`). `Attempt { created: Vec<CreatedResource> }` records
what this attempt created for rollback (FR-013).

### `KubeFailure` — new (`kube/classify.rs`)

`ContextMissing { context, looked_in }`, `NoKubeconfig { looked_in }`, `Unreachable { server, error }`,
`ExecPlugin { command, error, search_path }`, `NamespaceMissing { namespace }`,
`Forbidden { verb, resource }`, `NoRuntime`, `Build { output_tail }`, `Push { registry, error }`,
`ImagePull { image, message }`, `Unschedulable { message }`, `ClaimNotBound { claim, message }`,
`Conflict { kind, name }`, `CredentialSource { credential, path }`, `PhaseTimeout { phase }`,
`Cancelled`. `Display` gives the user-facing reason (contract strings). Classifiers:
`from_status(code, body)`, `from_pod(&PodStatusView, &[EventView])`, `from_pvc(..)`,
`from_runtime(kind, CommandOutput)` (reuses `sandbox::runtime::classify` shape).

### `PodGone` — new (`kube/classify.rs`)

`Deleted | Evicted { message } | NodeLost { node }` from `PodLookup` (`NotFound`, a pod view with
`deletionTimestamp`, `status.reason`, `phase`). `None` when the pod is present and running.

### `KubeconfigCandidates` and `ExecSearchPath` — new (`kube/kubeconfig.rs`)

`candidates(kubeconfig_env: Option<&OsStr>, home: &Path, platform: ClientPlatform) -> Vec<PathBuf>`;
`search_path(process_path, login_shell_path, home, platform) -> Vec<PathBuf>` (order and de-dup
fixed); `parse_login_shell_output(stdout) -> Option<OsString>` (marker-delimited).

### `CloneTarget` and `CloneFailure` — new (`kube/clone.rs`; clone-source part, K8)

`remote_for(remotes: &[(name, url)]) -> Result<RemoteUrl, NoRemote | Ambiguous(Vec<name>)>`;
`CloneTarget::for_url(url) -> /data/repos/<slug>-<hash8>`; `CloneFailure` from `git` stderr:
`AuthRequired { remote }`, `DiskFull`, `Unreachable { remote }`, `Other(String)`.

### `ListenerAuth` — new (`protocol/auth.rs`, K4)

`listener_auth(listen_addr, token_path_env) -> Result<ListenerAuth, AuthConfigError>`:
`Token(path)` | `LoopbackNoToken` (empty path + loopback address only) | error for an empty path
with a non-loopback address.

## State machine delta (491/489 `DaemonState`)

| From | Event | To | Note |
|---|---|---|---|
| any established | `Lost(_)` | `Reconnecting` | unchanged (489) |
| `Reconnecting`, `Unreachable` | `Gone(reason)` — **new event** | `Stopped` | sessions → `lost to its daemon`; actor ends; never re-dials (FR-011, FR-023) |
| `Starting` | `StartFailed(reason)` | `Unreachable` with the failure reason | unchanged; Kubernetes actors make no further attempts (as 489's remote rule, R7) |
| `Stopped` | `Start` | `Starting` | creates a new pod on the kept volume (story 2.4) |

`DaemonView` gains `stopped_reason: Option<String>` (set by `Gone`, cleared by `Start`) and reuses
`activity` for "Building image", "Pushing image", "Pulling image", "Cloning <repo>".

## Protocol delta (`PROTOCOL_VERSION` 48 → 49)

`ClientMsg::RepoClone { url: String, dest: String }` → `DaemonMsg::RepoCloned { dest }` |
`DaemonMsg::RepoCloneFailed { dest, failure: CloneFailureWire }`. Handled in
`micold-daemon/src/repo_clone.rs` (new, dispatched from `ops.rs`); `dest` must be under the daemon's `MICOLD_REPOS_ROOT`
(`/data/repos` in the pod; the request is refused otherwise).
