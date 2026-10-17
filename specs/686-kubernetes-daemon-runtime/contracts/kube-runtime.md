# Contract: the Kubernetes runtime (cluster objects, start/stop/remove, user-facing strings)

Types: [../data-model.md](../data-model.md). Decisions: [../research.md](../research.md) K4-K11.

## 1. Cluster objects the client creates (and nothing else)

All in the daemon's namespace, all labelled `app.kubernetes.io/managed-by=micold-ai-ide` and
`micold.dev/daemon=<key>`, annotated `micold.dev/daemon-name=<name>`. `<base>` =
`micold-<slug(name)>-<key>`.

| Object | Name | Created | Deleted |
|---|---|---|---|
| Pod | `<base>` | start, when absent | Stop, Remove, failed start that created it |
| PersistentVolumeClaim (`ReadWriteOnce`, `volume_size`, `storage_class`) | `<base>-data` | first start | Remove only; failed start only if that start created it |
| Secret (Opaque) per allowed credential | `<base>-gitconfig`, `<base>-git-credentials`, `<base>-git-ssh`, `<base>-signin-<cli>` | start, when allowed | start after withdrawal, Remove, failed start that created it |
| Secret (`kubernetes.io/dockerconfigjson`) | `<base>-pull` | start, when `RegistryPull` allowed | as above |

Never created: Service, Ingress, NetworkPolicy, ServiceAccount, Role, Namespace, or any object
without both labels. Pod invariants: research K7 (asserted by `kube/manifest.rs` tests and by
`kube_real_security`).

Pod container env: `MICOLD_LISTEN_ADDR=127.0.0.1:7727`, `MICOLD_TOKEN_PATH=` (empty, K4),
`MICOLD_IDLE_STOP=off`, `MICOLD_REPOS_ROOT=/data/repos`, `HOME=/data/home`,
`XDG_DATA_HOME=/data/state`, `TMPDIR=/tmp`, plus the git credential env of K7 for allowed
credentials only.

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
phase as `activity` while it runs. Cancel during `Image` stops the build or push child and fails the
start with `Cancelled` (rollback runs). On any failure: rollback (`Attempt.created` in reverse),
state `Unreachable` with the reason, no further attempts until the user's Start or Reconnect.

After `Tunnel`: the handshake of 491/489. A refusal for version mismatch → `VersionMismatch` (491
FR-008); no session is started (FR-017).

## 4. Reconnect and gone (FR-011)

On a lost tunnel: `Reconnecting` with 489's back-off; each attempt first `GET`s the pod.

| Lookup | Result |
|---|---|
| present, `Running`, ready | re-dial the tunnel |
| `404`, or `deletionTimestamp` set | `Gone(Deleted)` → "stopped: the pod was deleted" |
| `status.reason == Evicted` | `Gone(Evicted)` → "stopped: the pod was evicted: <message>" |
| `phase == Unknown`, or its node `404` | `Gone(NodeLost)` → "stopped: the pod's node <node> was lost" |
| API unreachable / 5xx / timeout | stay `Reconnecting` (back-off continues; user stop/retry as 489) |

On `Gone`: sessions → `lost to its daemon`; the actor ends; the pod is not recreated.

## 5. Stop and Remove (FR-014, FR-015)

- **Stop** with running sessions: confirm "Stop <name>? <n> running session(s) will end. Worktrees on
  its volume are kept." Then delete the pod (grace 10 s), wait until gone (120 s), state `Stopped`.
- **Remove**: confirm "Remove <name>? This deletes from <context>/<namespace>: pod <pod>, volume
  <pvc> with its worktrees, secrets <list>." plus 491's bound-worktree counts. Then delete pod,
  secrets, PVC; forget the daemon; worktrees follow 491 FR-014.
- **Remove, cluster unreachable**: "The cluster could not be reached (<error>). Forget <name> on this
  computer only? These stay in <context>/<namespace>: <pod>, <pvc>, <secrets>." Choosing it forgets
  the daemon locally and touches nothing remote.

## 6. User-facing failure strings (FR-012, US4)

| `KubeFailure` | Text |
|---|---|
| `NoKubeconfig` | `no kubeconfig found; looked in <paths>` |
| `ContextMissing` | `context <context> is not in the kubeconfig (<paths>)` |
| `Unreachable` | `cannot reach the cluster of <context> at <server>: <error>` |
| `ExecPlugin` | `the kubeconfig's credential plugin <command> failed: <error> (searched <path>)` |
| `NamespaceMissing` | `namespace <ns> does not exist; Micold does not create namespaces` |
| `Forbidden` | `not allowed to <verb> <resource> in <ns>` (e.g. `create pods/portforward`) |
| `NoRuntime` | `a container runtime (docker or podman) is needed to build the image; pulling a given image needs none` |
| `Build` | `image build failed: <last lines>` (full output in the daemon's log) |
| `Push` | `pushing to <registry> failed: <runtime error>` |
| `ImagePull` | `the cluster cannot pull <image>: <message>` |
| `Unschedulable` | `no node can run the pod: <scheduler message>` |
| `ClaimNotBound` | `volume <pvc> is not bound: <message>` |
| `Conflict` | §2 |
| `CredentialSource` | `<credential> is allowed but <path> does not exist` |
| `PhaseTimeout` | `<phase> did not finish within <bound>` |
| `Cancelled` | `start cancelled` |

Clone (K8): `<project> has no git remote; a Kubernetes daemon clones from it`,
`<project> has several remotes and none is origin: <names>`, `cloning <url> needs a credential:
allow Git credentials or an SSH key for <daemon>`, `the volume of <daemon> is full`.

## 7. Settings → Daemons (FR-001, FR-018)

Row: name, "Kubernetes", `<context> / <namespace> / <pod>`, version, state (+ `stopped_reason`),
bound worktrees — 491's row. Actions: Edit, Start, Stop, Reconnect, Remove, Logs (pod log tail,
last 200 lines, fetched on demand), recent errors (491 diagnostics). Form: context picker (contexts
from the merged kubeconfig, or the "no kubeconfig found" message), namespace, the shared image
source + resources section ([daemon-image.md](./daemon-image.md)), registry, architecture, node
selector, tolerations, volume size, storage class, allowed credentials (checkbox list; SSH key and
registry auth with a file picker). Edit of a running daemon saves and shows "applies on next start"
(story 5.2).
