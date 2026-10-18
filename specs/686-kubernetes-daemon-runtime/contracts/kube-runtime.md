# Contract: the Kubernetes runtime (cluster objects, start/stop/remove, user-facing strings)

Types: [../data-model.md](../data-model.md). Decisions: [../research.md](../research.md) K2-K11. What
#686 needs from #806's shared mechanism: [shared-ssh-image.md](./shared-ssh-image.md). Steps marked
**(blocked on #806)** call that mechanism.

## 1. Cluster objects the client creates (and nothing else)

All in the daemon's namespace, all labelled `app.kubernetes.io/managed-by=micold-ai-ide` and
`micold.dev/daemon=<key>`, annotated `micold.dev/daemon-name=<name>`. `<base>` =
`micold-<slug(name)>-<key>`.

| Object | Name | Created | Deleted |
|---|---|---|---|
| Pod | `<base>` | start, when absent (or when the user chose to recreate it, §3) | Stop, Remove, failed start that created it |
| PersistentVolumeClaim (`ReadWriteOnce`, `volume_size`, `storage_class`) | `<base>-data` | first start | Remove only; failed start only if that start created it |
| Secret (Opaque), SSH access (K3) | `<base>-ssh-access` | first start | Remove only; failed start only if that start created it |
| Secret (Opaque) per allowed credential | `<base>-gitconfig`, `<base>-git-credentials`, `<base>-git-ssh`, `<base>-signin-<cli>` | start, when allowed | start after withdrawal, Remove, failed start that created it |
| Secret (`kubernetes.io/dockerconfigjson`) | `<base>-pull` | start, when `RegistryPull` allowed | as above |

Never created: Service, Ingress, NetworkPolicy, ServiceAccount, Role, Namespace, or any object
without both labels. Pod invariants: research K7 (asserted by `kube/manifest.rs` tests and by
`kube_real_security`).

**Pod**: one container, image = `KubeSettings.image` or #806's pinned SSH image reference; command
left to the image; no `containerPort`; the SSH server on `127.0.0.1:2222` (the image's contract,
shared-ssh-image §1). Annotation `micold.dev/pod-settings` = the `PodStamp` (data model). Pod env
holds only what the image's SSH server needs (`HOME=/data/home`, `TMPDIR=/tmp`, the access-secret
mount path); the daemon's environment is passed by the shared start step (shared-ssh-image §3).

**Access secret** data keys: `host_ed25519_key`, `host_ed25519_key.pub`, `authorized_keys`
(one line per client, comment `micold-client:<client id>`). Mounted read-only at `/run/micold/ssh`.
Updated only by `replace` with the `resourceVersion` read in the same step; on `409`, re-read and
re-apply (FR-023).

## 2. Ownership and adoption (FR-003, FR-016, FR-023)

- `POST` → `201`: record in `Attempt.created`.
- `POST` → `409 AlreadyExists` → `GET`: both labels match → adopt (not added to `Attempt.created`);
  else fail `Conflict` "a <kind> named <name> already exists in <namespace> and was not created by
  Micold; rename the daemon or remove it".
- A pod being deleted (`deletionTimestamp` set) when start runs: wait for it to go (within the `Pod`
  bound), then create.
- Delete: by name with `preconditions.uid` from a `GET` in the same step; `404` counts as done.

## 3. Start sequence

Phases, bounds and failure classes: research K6, `StartPhase` in the data model. The row shows the
phase as `activity` while it runs.

1. `Config`, `Preflight` (access reviews: `create pods`, `create persistentvolumeclaims`, `create
   secrets`, `update secrets`, `create pods/portforward`, `delete pods`; a denial fails before anything
   is created), `Volume`, `Secrets` (access secret created, or this client's key added to it; allowed
   secrets reconciled, withdrawn ones deleted), `Pod`, `Schedule`, `Pull`, `Tunnel` (one `Cluster::port_forward` stream to the pod's SSH
   port, the stream the client relay opens per connection; the SSH identification line read from it;
   the relay itself is client-side and first used by `Copy` and `Connect`).
2. **Pod drift**: when `Pod` adopts a pod whose `micold.dev/pod-settings` differs from the saved
   settings, start pauses with `Drift { fields }`. The row says "The running pod differs from the
   settings (<fields>). Recreate it? Its <n> running session(s) will end; the volume is kept." with
   **Recreate pod** (delete it as Stop does, then continue from `Pod`) and **Keep running pod**
   (continue with it unchanged).
3. **(blocked on #806)** `Copy` (probe, copy if absent, start with `daemon_env`; login with the pinned
   host key), `Install` (the selected CLIs at their pinned versions; output streamed to the daemon's
   log; Cancel stops it and fails the start with `Cancelled`), `Connect` (489's bridge and the 491
   handshake; a version mismatch → `VersionMismatch`, no session started, FR-017).

On any failure: rollback (`Attempt.created` in reverse, and this attempt's key removed from an
adopted access secret), state `Unreachable` with the reason, no further attempts until the user's
Start or Reconnect.

## 4. Reconnect and gone (FR-011)

On a lost tunnel: `Reconnecting` with 489's back-off; each attempt first `GET`s the pod.

| Lookup | Result |
|---|---|
| present, `Running`, ready | re-open the relay, re-dial 489's bridge |
| `404`, or `deletionTimestamp` set | `Gone(Deleted)` → "stopped: the pod was deleted" |
| `status.reason == Evicted` | `Gone(Evicted)` → "stopped: the pod was evicted: <message>" |
| `phase == Unknown`, or its node `404` | `Gone(NodeLost)` → "stopped: the pod's node <node> was lost" |
| API unreachable / 5xx / timeout | stay `Reconnecting` (back-off continues; user stop/retry as 489) |

On `Gone`: sessions → `lost to its daemon`; the actor ends; the pod is not recreated.

## 5. Stop and Remove (FR-014, FR-015)

- **Stop** with running sessions: confirm "Stop <name>? <n> running session(s) will end. Worktrees on
  its volume are kept." Then close the relay, delete the pod (grace 10 s), wait until gone (120 s,
  else `StopTimeout`), state `Stopped`.
- **Remove**: confirm "Remove <name>? This deletes from <context>/<namespace>: pod <pod>, volume
  <pvc> with its worktrees, secrets <list>." plus 491's bound-worktree counts. Then delete pod,
  secrets (access secret included), PVC; delete this client's key files; forget the daemon; worktrees
  follow 491 FR-014.
- **Remove, cluster unreachable**: "The cluster could not be reached (<error>). Forget <name> on this
  computer only? These stay in <context>/<namespace>: <pod>, <pvc>, <secrets>; this computer's key
  stays authorized in <base>-ssh-access until they are deleted." Choosing it deletes this client's key
  files, forgets the daemon locally and touches nothing remote.

## 6. User-facing failure strings (FR-012, US4)

| `KubeFailure` | Text |
|---|---|
| `NoKubeconfig` | `no kubeconfig found; looked in <paths>` |
| `ContextMissing` | `context <context> is not in the kubeconfig (<paths>)` |
| `Unreachable` | `cannot reach the cluster of <context> at <server>: <error>` |
| `ExecPlugin` | `the kubeconfig's credential plugin <command> failed: <error> (searched <path>)` |
| `NamespaceMissing` | `namespace <ns> does not exist; Micold does not create namespaces` |
| `Forbidden` | `not allowed to <verb> <resource> in <ns>` (e.g. `create pods/portforward`) |
| `ImagePull` | `the cluster cannot pull <image>: <message>` |
| `Unschedulable` | `no node can run the pod: <scheduler message>` |
| `ClaimNotBound` | `volume <pvc> is not bound: <message>` |
| `Conflict` | §2 |
| `CredentialSource` | `<credential> is allowed but <path> does not exist` |
| `ImageContract` | `<image> does not provide the SSH server Micold needs: <missing>` (e.g. `no SSH server answered on port 2222`, `the container exited: <reason>`) |
| `HostKeyMismatch` | `the SSH host key of <pod> does not match the one in its access secret; refusing to connect` |
| `PhaseTimeout` | `<phase> did not finish within <bound>` |
| `StopTimeout` | `pod <pod> was not gone within 120 s` |
| `Cancelled` | `start cancelled` |
| `Provision(..)` | #806's text (copy, start, install; install names the CLI and its error) |
| `Api` | `the cluster answered <status>: <message>` |
| `Unavailable` | `Kubernetes daemons need the shared SSH image (#806), which this build does not include` |

Clone (K8, blocked on #806): `<project> has no git remote; a Kubernetes daemon clones from it`,
`<project> has several remotes and none is origin: <names>`, `cloning <url> needs a credential:
allow Git credentials or an SSH key for <daemon>`, `the volume of <daemon> is full`.

## 7. Settings → Daemons (FR-001, FR-018)

Row: name, "Kubernetes", `<context> / <namespace> / <pod>`, version, state (+ `stopped_reason`),
bound worktrees — 491's row. Actions: Edit, Start, Stop, Reconnect, Remove, Logs (pod log tail,
last 200 lines, fetched on demand), recent errors (491 diagnostics). Form: context picker (contexts
from the merged kubeconfig, or the "no kubeconfig found" message), namespace, image (empty = the
default SSH image), AI CLIs to install (checkboxes `claude`, `copilot`, `pi`; default the providers
enabled in Settings), the shared resources section (#806), node selector, tolerations, volume size
(on a daemon started before: "Applies only to a new volume, after Remove"), storage class, allowed
credentials (checkbox list; SSH key and registry auth with a file picker). Edit of a running daemon
saves and shows "applies on next start" (story 5.2).

**Add entry (FR-024)**: the runtime choice lists "Kubernetes" only in a build where the shared
mechanism is wired (the blocked milestone that connects). Before it, the form and its reducer exist
and are tested, and Kubernetes daemons already in settings show in the list, but none can be added.
