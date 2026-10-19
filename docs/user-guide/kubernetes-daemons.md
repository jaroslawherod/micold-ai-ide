# Running the session service in a Kubernetes cluster

A daemon can run as a pod in a Kubernetes cluster you already have access to, and your worktrees
bound to it run their sessions there. Micold reads your own kubeconfig, stores no cluster
credential, and needs neither `kubectl`, a registry nor a local container runtime.

## Adding one

1. Open **Settings > Daemons** and choose **Add daemon**.
2. Name it, and set **Runs in** to **Kubernetes**.
3. The form groups the rest in sections: **Connection** (open first), **Image**, **Credentials**,
   **Resources** and **Placement and storage**; a section with an error opens itself. A closed
   section shows a one-line summary of its settings, and a `*` after its title when a required field
   inside it is empty. In **Connection**, choose the **Context** from the **Context from your
   kubeconfig** select above the **Context** field (it appears when the kubeconfig lists contexts;
   you can still type a context in **Context**) and enter the **Namespace**. Everything else about reaching the cluster comes from the kubeconfig. Micold does
   not create namespaces.
4. In **Image**, tick the **AI CLIs installed in the pod when it starts**; in **Credentials**, tick
   any credentials the pod may use (see [Allowed credentials](#allowed-credentials)).
5. Save, then choose **Start** on its row.

If no kubeconfig is found the form says so instead of listing contexts.

## The image

**Image** empty uses the app's basic image: an SSH server, a POSIX shell and the tools for
installing the AI CLIs, running as a non-root user, with no daemon and no credential inside. The
client pulls the build it was released against (`ghcr.io/jaroslawherod/micold-basic`, pinned by
content hash), so the image never has to match the app's version.

You can give your own image. It must provide the same: an SSH server listening on port 2222 of the
pod's loopback, a POSIX shell, what installing the AI CLIs needs, a non-root user, and no daemon or
credential. An image that does not is reported on start (see [When it does not
start](#when-it-does-not-start)). A private image needs **the registry credentials** allowed.

## What happens when it starts

The row shows **Starting** and, under it, the step it is in: checking the kubeconfig and your
permissions, the volume, the secrets, the pod, scheduling, pulling the image, opening the tunnel,
copying the daemon, installing the AI CLIs and connecting.

- The client copies its own daemon build into the volume, so the pod always runs the daemon of the
  app you use.
- It installs the AI CLIs you ticked (`claude`, `copilot`, `pi`), each at the version this build
  pins, skipping one already installed. The output appears live in the daemon's **Logs**, with a
  **Cancel** button; a start that is cancelled fails with `start cancelled`.
- Pulling the image and installing are each bounded to 15 minutes; every other step to 120 seconds.
  Nothing waits indefinitely.
- A CLI you add later is installed on the next start. One you remove is no longer offered for new
  sessions; its running sessions continue and its files stay on the volume until Remove.

A start that fails removes what that attempt created and shows the reason on the row; nothing is
retried until you choose **Start** or **Reconnect**.

## What is created in the namespace

Everything is labelled `app.kubernetes.io/managed-by=micold-ai-ide` and `micold.dev/daemon=<key>`.

| Object | Name | Kept until |
|---|---|---|
| Pod | `micold-<name>-<key>` | Stop or Remove |
| Volume claim (your worktrees, the daemon, the AI CLIs) | `micold-<name>-<key>-data` | Remove |
| Secret, SSH access (the pod's host key and your computer's public key) | `micold-<name>-<key>-ssh-access` | Remove |
| One secret per allowed credential | `...-gitconfig`, `...-git-credentials`, `...-git-ssh`, `...-signin-<cli>`, `...-pull` | the next start after you withdraw it, or Remove |

**No Service, Ingress, network policy, service account, role or namespace is created.** Micold
reaches the pod only through the Kubernetes API's port-forward, so nothing is exposed in the cluster
or on your computer beyond loopback. Your private key stays on your computer, readable by you only.
A resource of the same name that Micold did not create is never touched: start fails and names it.

Your account needs to create pods, volume claims and secrets, update secrets, delete pods and use
`pods/portforward` in the namespace. A missing permission is refused before anything is created.

## The pod's security settings

The pod runs as a non-root user with a read-only root filesystem (writable only on the volume and
`/tmp`), privilege escalation disabled, all capabilities dropped and the runtime's default seccomp
profile. It does not mount a service account token. Its SSH server listens on loopback only, accepts
only the key in the access secret, and refuses passwords, root login and any forwarding but the
daemon's own.

## Allowed credentials

**Nothing reaches the pod unless it is ticked.** Each tick becomes a secret in the namespace,
mounted read-only, and never goes into the image:

- **the git config**, as `~/.gitconfig` on the volume;
- **the git credentials**, at `/run/micold/creds/git-credentials`;
- **an SSH key**, with its **SSH key file**, at `/run/micold/creds/ssh`;
- **the sign-in** of each AI CLI (Claude Code, GitHub Copilot, Pi Coding Agent), at the path that
  CLI reads under its home;
- **the registry credentials**, with its **Registry credentials file**, as an image-pull secret.

A ticked credential whose file is missing fails the start (`<credential> is allowed but <path> does
not exist`). Unticking one deletes its secret on the next start.

## The form

| Field | Meaning |
|---|---|
| **Context** | A context from your kubeconfig. |
| **Namespace** | Where the pod, volume and secrets go. It must exist. |
| **Image** | Empty uses the default SSH image. |
| **AI CLIs installed in the pod when it starts** | Claude Code, GitHub Copilot, Pi Coding Agent; by default the providers enabled in Settings. |
| **CPU request**, **CPU limit** | Cores, e.g. 0.5 or 2. Empty reserves or limits none. |
| **Memory request**, **Memory limit** | Mebibytes. Empty reserves or limits none. |
| **Node selector** | One `key=value` per line. |
| **Tolerations** | One per line. |
| **Volume size** | Kubernetes form, e.g. `20Gi`. Applies only to a new volume. |
| **Storage class** | Empty uses the cluster default. |
| **Allowed credentials** | See above. |

## Edits apply on the next start

Saving the form of a running daemon shows "applies on next start"; the running pod is not changed.
The volume size is read only when the volume is created: on a daemon started before, the field says
so, and a new size takes effect after **Remove**.

When **Start** finds the pod running with settings that differ from the saved ones, it asks:

> The running pod differs from the settings (<fields>). Recreate it? Its <n> running session(s)
> will end; the volume is kept.

**Recreate pod** deletes the pod and creates it again with the saved settings. **Keep running pod**
continues with the pod unchanged. Micold never recreates a pod on its own.

## Worktrees on a Kubernetes daemon

When you create a worktree that runs on a Kubernetes daemon, the pod has no copy of your project, so
Micold asks the daemon to clone the project's git remote (`origin`, or the only remote) onto its
volume, under `/data/repos`, and creates the worktree there. The form says "Cloning <project>" while
it runs. The worktree appears under the daemon's name with its in-pod path, and survives the pod
being deleted, because the clone is on the volume. Creating a second worktree of the same project
fetches the existing clone instead of cloning again.

- A project with no git remote is refused: "<project> has no git remote; a Kubernetes daemon clones
  from it". With several remotes and none called `origin`, the refusal names them.
- Commits you have not pushed stay on your computer; the pod sees only what the remote holds.
- A remote that needs a credential needs it allowed for the daemon (see Allowed credentials);
  otherwise the create stops with "cloning <url> needs a credential: allow Git credentials or an SSH
  key for <daemon>". A full volume stops it with "the volume of <daemon> is full". Nothing
  half-cloned is left behind.

## Stop, Remove and Forget locally

- **Stop** deletes the pod and keeps the volume, so worktrees survive. It asks first when sessions
  are running ("Stop <name>? <n> running session(s) will end. Worktrees on its volume are kept.").
  **Start** creates a new pod on the same volume.
- **Remove** deletes the pod, the volume with its worktrees, the secrets and this computer's key,
  after a confirmation that names each of them and the cluster, namespace and counts of bound
  worktrees.
- **Forget locally** only drops the entry and this computer's key. It is offered when Remove cannot
  reach the cluster; the pod, volume and secrets stay in the cluster, and your key stays authorized
  in the access secret until they are deleted.

## Logs

Choose **Logs** on the row for the last 200 lines of the pod's log, fetched when you ask, and for
the output of the daemon copy and AI CLI installs while a start runs. Recent errors are listed
under the row.

## Sessions outlive the client

Sessions and terminals run in the pod, so closing Micold or losing the network does not stop them.
When you open Micold again, or the network returns, the daemon reconnects to the same pod and its
sessions are shown running, with the output they produced while you were away. Reconnecting never
creates a second pod or a duplicate session.

## When the connection drops

As for [an SSH daemon](remote-daemons.md#when-the-network-drops), the row shows **Reconnecting**
while Micold retries (after 1, 2, 4, 8, then every 15 seconds), and its sessions show
`disconnected`. Choose **Stop** on the attempt to give up; the row then shows **Unreachable**, and
**Reconnect** on it dials the pod again with the first delay. If the cluster cannot say whether the
pod is there (a timeout or a server error), the row stays **Reconnecting** and Micold keeps trying.

## When the pod is gone

If the pod was deleted, evicted or lost with its node while you were away, the row says
**Stopped** with the reason, and the sessions that were running show `lost to daemon <name>`:

| Text | Meaning |
|---|---|
| `stopped: the pod was deleted` | Someone deleted the pod, or it is being deleted. |
| `stopped: the pod was evicted: <message>` | The cluster evicted it; the message is the kubelet's. |
| `stopped: the pod's node <node> was lost` | The node it ran on is gone or does not answer. |

Micold does not recreate the pod by itself. Choose **Start**: a new pod is created on the kept
volume and every worktree is still there. If someone else removed the daemon meanwhile (pod and
volume), Start creates a new, empty volume and pod, and its worktrees are gone.

## When it does not start

| Text | What to do |
|---|---|
| `no kubeconfig found; looked in <paths>` | Create or point `KUBECONFIG` at your kubeconfig. |
| `context <context> is not in the kubeconfig (<paths>)` | Pick a context that exists. |
| `cannot reach the cluster of <context> at <server>: <error>` | Check the network, VPN and the cluster address. |
| `the kubeconfig's credential plugin <command> failed: <error> (searched <path>)` | Fix or install the plugin the kubeconfig names. |
| `namespace <ns> does not exist; Micold does not create namespaces` | Create the namespace, or choose another. |
| `not allowed to <verb> <resource> in <ns>` | Ask for the permission named, e.g. `create pods/portforward`. |
| `the cluster cannot pull <image>: <message>` | Check the image name, and allow the registry credentials for a private one. |
| `no node can run the pod: <scheduler message>` | Relax the node selector or resources, or add tolerations. |
| `volume <pvc> is not bound: <message>` | Check the storage class and the volume size. |
| `a <kind> named <name> already exists in <namespace> and was not created by Micold; rename the daemon or remove it` | Rename the daemon or delete the object. |
| `<credential> is allowed but <path> does not exist` | Fix the path or untick it. |
| `<image> does not provide the SSH server Micold needs: <missing>` | Use an image that follows [the image's contract](#the-image). |
| `the SSH host key of <pod> does not match the one in its access secret; refusing to connect` | Something changed the pod or secret; Remove and add the daemon again. |
| `<phase> did not finish within <bound>` | Look at the cluster's events for that step. |
| `pod <pod> was not gone within 120 s` | The cluster is slow to delete it; try Stop again. |
| `start cancelled` | You cancelled; choose Start. |
| `the cluster answered <status>: <message>` | The cluster's own error. |
| `Kubernetes daemons need the shared SSH image (#806), which this build does not include` | This build cannot start Kubernetes daemons. |

A copy, start or install failure names the step and, for an AI CLI, the CLI and its error.

## Your other daemons are not affected

A Kubernetes daemon that is starting, failing or unreachable never changes another daemon, its
worktrees or its sessions.
