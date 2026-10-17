# Feature Specification: Kubernetes session daemon runtime

**Feature Branch**: `claude/project-thread-xelisc`

**Created**: 2026-10-17

**Status**: Draft

**Input**: User description: GitHub issue #686, "Support a Kubernetes session daemon runtime" (split out
of #491; depends on its multi-daemon model). Owner comment (2026-10-13): align the Kubernetes image
source with #806's decision (build/push an image vs. pull a given one); share the resources and
image-source form with #806.

## Overview

Spec 491 gave the app a list of daemons, each with a runtime (host, container, and since spec 489 SSH),
and binds every worktree to one daemon. This feature adds a **Kubernetes** runtime: the session daemon
runs in a pod in a cluster the user can reach with their own kubeconfig.

The user picks a kube context, a namespace and the daemon's image, plus resources and scheduling
constraints. The client creates the pod (or reuses the one it created before) and reaches the daemon
only through the Kubernetes API's port-forward tunnel; nothing is exposed as a Service or Ingress. The
project's repository is cloned into a persistent volume claim (PVC) owned by the daemon, so worktrees
live in the cluster and survive pod restarts. Push/pull credentials and AI CLI sign-ins reach the pod
only as Kubernetes secrets, and only those the user explicitly allows. **Stop** deletes the pod and
keeps the volume; **Remove** deletes both after confirmation.

The image source and the resources fields are the ones #806 introduces for container daemons (one
shared form): see FR-004 and FR-005.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Work on a project in a Kubernetes daemon end to end (Priority: P1)

A developer adds a Kubernetes daemon, starts it, binds a project's new worktree to it and starts a
`claude`, `copilot` or `pi` session there, typing in its terminal as with any other daemon.

**Why this priority**: This is the feature's reason to exist.

**Independent Test**: With a reachable cluster, add a Kubernetes daemon with defaults, start it, create
a worktree bound to it, start a session and type in it; input and output reach the session.

**Acceptance Scenarios**:

1. **Given** a cluster reachable through the user's kubeconfig, **When** the user adds a Kubernetes
   daemon (context, namespace, image source) and starts it, **Then** a pod running the session daemon
   is created in that namespace and the daemon reaches `connected`.
2. **Given** a connected Kubernetes daemon, **When** the user creates a worktree bound to it, **Then**
   the project's repository is present in the daemon's volume and the worktree is created there,
   shown with the daemon's name and the in-pod path.
3. **Given** that worktree, **When** the user starts a `claude`, `copilot` or `pi` session or opens a
   terminal, **Then** it runs in the pod and accepts input and shows output.
4. **Given** the Kubernetes daemon was started before by this client and its pod still exists,
   **When** the user starts it again, **Then** the existing pod is reused, not duplicated.
5. **Given** the daemon is connected, **When** anyone inspects the namespace, **Then** no Service or
   Ingress was created for it; the client's only path to the daemon is the API tunnel.

---

### User Story 2 - Sessions outlive the client (Priority: P1)

A developer closes the app (or loses the network) while sessions run in the pod, comes back later and
finds them still running with the output produced meanwhile.

**Why this priority**: Long-running remote work is the main reason to run in a cluster.

**Independent Test**: Start a session producing output, quit the client, wait, reopen, reconnect;
the session is running and shows the output produced while the client was away.

**Acceptance Scenarios**:

1. **Given** sessions running in the pod, **When** the client quits or the tunnel drops, **Then** the
   sessions keep running in the pod.
2. **Given** the tunnel dropped, **When** the client reconnects (automatically or by the user's
   reconnect), **Then** the sessions are shown running with their output since the drop, and no second
   pod or duplicate session is created.
3. **Given** the pod was deleted, evicted or lost with its node meanwhile, **When** the client
   reconnects, **Then** the daemon shows `stopped` with a reason naming why the pod is gone, the
   sessions that were running are shown as `lost to its daemon` (spec 491 term), and the client does
   not recreate the pod by itself.
4. **Given** that stopped daemon, **When** the user starts it, **Then** a new pod is created on the
   kept volume and every worktree is still present.

---

### User Story 3 - Only what the user allows reaches the pod (Priority: P1)

A developer allows a push/pull credential and one AI CLI sign-in for a Kubernetes daemon; only those
two reach the pod, as secrets, and the pod runs as non-root with a read-only root filesystem.

**Why this priority**: The cluster may be shared; a leaked credential or a privileged pod is a
security failure, not a cosmetic one.

**Independent Test**: Allow one credential and one sign-in, start the daemon, inspect the pod: only
those two secrets are mounted, the image contains no credential, the pod runs as a non-root user with
a read-only root filesystem.

**Acceptance Scenarios**:

1. **Given** the user allowed a set of push/pull credentials and AI CLI sign-ins, **When** the pod
   starts, **Then** exactly those are mounted from Kubernetes secrets and nothing else is.
2. **Given** the user allowed none, **When** the pod starts, **Then** no credential or sign-in secret
   is created or mounted, and git operations needing credentials and sign-in-requiring CLIs report
   that they lack them.
3. **Given** the user withdraws an allowed credential and restarts the daemon, **When** the pod
   starts, **Then** the secret is deleted from the namespace and no longer mounted.
4. **Given** any Kubernetes daemon pod, **When** inspected, **Then** it runs as a non-root user, with
   a read-only root filesystem (writable paths limited to the volume and scratch space), and without
   privilege escalation.

---

### User Story 4 - Failures are reported with their reason, not as a hang (Priority: P1)

A developer whose namespace forbids pod creation, whose image cannot be pulled, or whose pod cannot be
scheduled sees the daemon's state change to a failure that names the cause.

**Why this priority**: Kubernetes failures are otherwise silent (a pod stays `Pending` forever); a hang
is the worst outcome for the user.

**Independent Test**: For each of RBAC denial, image pull failure and an unschedulable pod, start the
daemon and read the state and reason in Settings → Daemons.

**Acceptance Scenarios**:

1. **Given** the user's identity may not create pods (or secrets, volumes, or port-forwards) in the
   namespace, **When** the daemon starts, **Then** it shows a failure naming the denied action and
   resource, and nothing partially created is left behind.
2. **Given** the image cannot be pulled, **When** the daemon starts, **Then** it shows a failure naming
   the image and the cluster's pull error.
3. **Given** no node can host the pod (resources, node selector or tolerations), **When** the daemon
   starts, **Then** it shows a failure with the scheduler's reason.
4. **Given** the kube context is missing from the kubeconfig or the cluster is unreachable, **When**
   the daemon starts, **Then** it shows a failure naming the context or the connection error.
5. **Given** any of these failures, **When** it occurs, **Then** the reason is shown within the start
   timeout (FR-012) and other daemons are unaffected.

---

### User Story 5 - Manage a Kubernetes daemon in Settings (Priority: P2)

A developer adds, edits, starts, stops, reconnects and removes a Kubernetes daemon from Settings →
Daemons, with the same row and actions as any other runtime.

**Why this priority**: Without it the runtime cannot be configured; it builds on the 491 surface.

**Independent Test**: Add a Kubernetes daemon, edit its resources, start, stop (volume kept), start
again (worktrees still there), then remove (pod and volume gone after confirmation).

**Acceptance Scenarios**:

1. **Given** the add form, **When** the user chooses the Kubernetes runtime, **Then** they can pick a
   kube context (from their kubeconfig) and a namespace, choose the image source and resources (the
   #806 form), and set node selector, tolerations, volume size and allowed credentials and sign-ins.
2. **Given** a Kubernetes daemon, **When** the user edits it and saves, **Then** the new settings
   apply on its next start; a running pod is not changed silently.
3. **Given** a running daemon, **When** the user chooses **Stop**, **Then** the pod is deleted, the
   volume (and its worktrees) is kept, and the state becomes `stopped`.
4. **Given** a Kubernetes daemon shown `unreachable` whose pod still runs, **When** the user chooses
   **Reconnect**, **Then** the tunnel is re-established and its running sessions are shown again with
   their output.
5. **Given** a daemon, **When** the user chooses **Remove**, **Then** a confirmation says the pod, the
   volume with its worktrees and the daemon's secrets will be deleted, plus spec 491's bound-worktree
   counts, and nothing is deleted until confirmed.
6. **Given** the removal was confirmed, **When** it completes, **Then** the pod, volume and secrets the
   client created are gone from the namespace and the worktrees follow spec 491 FR-014.

---

### User Story 6 - Proven end to end in CI (Priority: P2)

A maintainer relies on CI to drive a real cluster with a scripted stand-in AI CLI.

**Why this priority**: The isolation and failure promises are only trustworthy when exercised for real.

**Independent Test**: Run the documented task with a local throwaway cluster available: all scenarios
pass with no sign-in, provider network access or cost.

**Acceptance Scenarios**:

1. **Given** a throwaway cluster created in the job, **When** the client deploys the daemon pod and
   drives it with the stand-in AI CLI, **Then** pod creation, tunnelling, a session's input and output,
   reconnect, stop (volume kept) and remove pass.
2. **Given** the same cluster, **When** RBAC denial, image pull failure and an unschedulable pod are
   provoked, **Then** each is reported with its reason within the timeout.
3. **Given** no cluster or no opt-in, **When** a developer runs the normal test task, **Then** these
   tests do not run; a documented task runs them on demand and CI runs them on changes that can
   affect them.

---

### Edge Cases

- **Namespace missing**: start fails naming the namespace; the client does not create namespaces.
- **Context changes in kubeconfig** (renamed, removed, credentials expired): start or reconnect fails
  naming the context; the stored daemon is unchanged.
- **Pod created by someone else with the same name**: the client reuses only pods it created (marked
  as such); otherwise start fails naming the conflicting pod.
- **Two Kubernetes daemons on the same context and namespace**: allowed when names differ; each owns
  its own pod, volume and secrets, which never collide. A second daemon with the same context,
  namespace and daemon name is refused naming the first.
- **Volume full**: worktree creation or git operations fail with the disk-full reason; existing
  sessions are not killed by the client.
- **Pod evicted, deleted outside the app or node lost while connected**: the daemon goes
  `reconnecting` and sessions show `disconnected`; once the client sees the pod is gone it stops the
  attempts, shows `stopped` with the reason (evicted, deleted, node lost), marks the sessions
  `lost to its daemon`, and waits for the user's **Start** (story 2.3, 2.4; FR-011).
- **The daemon in the pod reports a version different from the client's**: the daemon is marked `version mismatch` per spec
  491 FR-008 and sessions are not started; no running session is ended to resolve it.
- **Stop while sessions run**: confirmation states how many running sessions will end; worktrees stay.
- **Remove when the cluster is unreachable**: the confirmation offers to forget the daemon locally and
  states that cluster resources could not be deleted, naming them.
- **Volume access mode / storage class absent**: start fails naming the storage problem from the
  cluster (claim stays `Pending` with the reason).
- **Port-forward denied while pod creation is allowed**: reported as RBAC denial naming port-forward.
- **Repository branch already checked out in another daemon's worktree**: refused per spec 491 FR-016,
  naming the worktree and daemon.
- **Two clients start the same daemon at once** (same context, namespace and daemon name, from two
  windows or two machines): at most one pod results; the client that loses the race reuses the pod
  the other created (FR-023).
- **One client stops or removes a daemon while another is connected**: the other client shows the
  daemon `stopped` with a reason saying the pod was deleted, and its sessions `lost to its daemon`; it
  never recreates the pod on its own (FR-011, FR-023). After a remove, its next **Start** creates a new,
  empty volume.
- **Same daemon name configured on another machine for the same context and namespace**: it is the
  same daemon; both clients share its pod, volume and secrets (spec 489 FR-021). A same-named
  resource in the namespace that the app did not create is a conflict (FR-016).
- **Kubeconfig location per OS**: the `KUBECONFIG` variable is honoured as a list of files, separated
  by `;` on Windows and `:` on macOS and Linux, merged in order; without it the file `.kube/config` in
  the user's home directory (`%USERPROFILE%` on Windows) is used. None found: the add form says no
  kubeconfig was found and where it looked.
- **Exec credential plugin** (cloud CLIs that the kubeconfig runs to log in): missing from the
  app's search path (for example when the app is launched from the macOS Dock or Finder rather than a
  shell), or failing, makes start fail naming the plugin, its error, and the search path used.
- **Windows, macOS, Linux clients**: same behaviour on all three, including the two cases above.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST offer a daemon runtime "Kubernetes" in the daemon list with the same
  name, runtime, endpoint, version, state and bound-worktree surfaces as other runtimes (spec 491
  FR-015); the endpoint shown is context / namespace / pod.
- **FR-002**: The user MUST be able to choose a kube context from their kubeconfig and a namespace;
  every other cluster connection setting MUST come from the kubeconfig. The system MUST NOT store,
  request or log cluster credentials.
- **FR-003**: The system MUST create the daemon's pod in the chosen namespace when none exists and
  MUST reuse the pod it created before when one exists, never creating a second pod for one daemon.
- **FR-004**: The daemon's image source MUST use the same choice and form as container daemons in
  #806, and the default MUST follow #806's default rather than a published release image.
  [NEEDS CLARIFICATION: #806 defaults to building an image locally from a Dockerfile with the current
  daemon binary; a cluster cannot use a locally built image without a registry. Is the Kubernetes
  default (a) build locally and push to a registry the user names, (b) pull only, with the user naming
  an image (e.g. one built and pushed per #806), or (c) build inside the cluster? Under each, how do
  the AI provider CLIs the user selects for the daemon (#806 installs them in the built image) get
  into the pod's image?]
- **FR-005**: The user MUST be able to set CPU and memory requests and limits (the #806 resources
  fields), a node selector, tolerations and the volume size; these persist with the daemon and apply
  on the next start.
- **FR-006**: The system MUST reach the daemon only through the Kubernetes API's port-forward tunnel
  and MUST NOT create a Service, Ingress or any other network exposure, in the cluster or on the
  client beyond loopback.
- **FR-007**: The system MUST store worktrees on a persistent volume owned by the daemon, populated by
  cloning the project's repository into it, so worktrees survive pod deletion and restart.
  [NEEDS CLARIFICATION: where does the clone come from — (a) the project's git remote, using an
  allowed push/pull credential, with projects that have no remote refused; or (b) the client's local
  repository, transferred to the pod over the tunnel (works without a remote)?]
- **FR-008**: Push/pull credentials and AI CLI sign-ins MUST reach the pod only when the user
  explicitly allows each for that daemon, only as Kubernetes secrets the system creates in the
  namespace, and never inside the image. Withdrawn ones MUST be deleted on the next start.
- **FR-009**: The pod MUST run as a non-root user, with a read-only root filesystem (writable only on
  the volume and scratch space), with privilege escalation disabled, and with no secret mounted that
  FR-008 did not allow.
- **FR-010**: Sessions and terminals MUST keep running in the pod when the client disconnects or
  quits, and MUST be shown with their output produced meanwhile on reconnection (as spec 489 FR-006).
- **FR-011**: The system MUST reconnect a dropped tunnel automatically with the spec 489 FR-007 back-off
  and user stop/retry, use the spec 489 FR-008 state set (including `reconnecting`), and MUST NOT
  create a second pod or duplicate sessions by reconnecting. Sessions follow spec 489 FR-019. When the
  client finds the daemon's pod gone (deleted, evicted, node lost), it MUST stop the attempts, show the
  daemon `stopped` with that reason, mark its sessions `lost to its daemon`, and MUST NOT recreate the
  pod until the user starts the daemon.
- **FR-012**: The system MUST report, with a reason naming the cause, at least: RBAC denial (naming
  the action and resource), image pull failure (naming the image and cluster error), unschedulable pod
  (with the scheduler's reason), volume claim not bound (with its reason), missing context or
  namespace, unreachable cluster, and a missing or failing exec credential plugin. It MUST NOT hang or
  retry silently. Each start phase has its own bound, after which start fails naming the phase:
  everything except the image pull and any image build or push at most 120 seconds; the image pull at
  most 15 minutes; an image build or push (if FR-004 resolves to one) shows its output live, can be
  cancelled by the user, and fails at most 30 minutes after it starts.
- **FR-013**: A failed start MUST NOT leave behind a pod or secrets it created in that attempt; the
  volume is kept only if it existed before the attempt.
- **FR-014**: **Stop** MUST delete the pod and keep the volume; after confirmation when sessions are
  running, stating how many will end.
- **FR-015**: **Remove** MUST, after a confirmation naming the pod, volume and secrets to be deleted
  (with spec 491 FR-012/FR-014 counts), delete every cluster resource the system created for the
  daemon, then forget the daemon; when the cluster is unreachable it MUST say which resources remain.
- **FR-016**: The system MUST mark cluster resources it creates as belonging to the daemon, identified
  by context, namespace and daemon name (so another client's daemon with the same three is the same
  daemon), act only on resources so marked, and refuse to reuse or delete anything else, naming the
  conflict.
- **FR-017**: The system MUST detect a client and daemon version mismatch per spec 491 FR-008 and
  spec 489 FR-011 and MUST NOT start sessions on a mismatched daemon.
- **FR-018**: The user MUST be able to add, edit, start, stop, reconnect and remove a Kubernetes daemon
  from Settings → Daemons and see its recent errors and the pod's logs; blank or invalid fields are
  refused naming the field; a second daemon with the same name is refused.
- **FR-019**: A Kubernetes daemon being started, failing or unreachable MUST NOT change the state or
  responsiveness of any other daemon, including while the app starts.
- **FR-020**: The system MUST persist Kubernetes daemons, their settings and bindings locally, with no
  cloud dependency beyond the user's cluster, and MUST NOT leak state between daemons or sessions.
- **FR-021**: The automated end-to-end suite MUST create a throwaway local cluster in the job, deploy
  the daemon pod and drive it with the scripted stand-in AI CLI, covering pod creation, tunnelling,
  a session's input and output, reconnect, stop (volume kept), remove, RBAC denial, image pull failure
  and an unschedulable pod. The tests MUST be off by default locally, runnable by a documented task,
  and run in CI.
- **FR-022**: Every user-facing behaviour above MUST work equivalently on Linux, macOS and Windows
  clients, including kubeconfig discovery (the `KUBECONFIG` list with the OS's separator, else the
  home directory's `.kube/config`) and exec credential plugins resolved the same way whether the app
  was started from a shell or from the desktop.
- **FR-023**: Concurrent starts of one daemon, from one or several clients, MUST result in at most one
  pod, the later start reusing it. A Stop or Remove by one client MUST be shown to every other connected
  client as `stopped` with a reason saying the pod was deleted, with its sessions `lost to its daemon`.

### Key Entities

- **Kubernetes daemon**: a daemon (spec 491) whose runtime is Kubernetes; settings: kube context,
  namespace, image source (#806), resources, node selector, tolerations, volume size, allowed
  credentials and sign-ins.
- **Daemon pod**: the pod running the session daemon; created or reused on start, deleted on stop.
- **Daemon volume**: the persistent volume claim holding the cloned repositories and worktrees; kept
  on stop, deleted on remove.
- **Daemon secrets**: one per allowed credential or sign-in; created on start, deleted when withdrawn
  or on remove.
- **Tunnel**: the client's port-forward connection through the Kubernetes API to the daemon.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user with a reachable cluster can add a Kubernetes daemon, start it, create a worktree
  and type into a running session in under 5 minutes, excluding image build and pull time.
- **SC-002**: After the client is away for 10 minutes, 100% of sessions still running in the pod are
  shown running with their output since disconnection.
- **SC-003**: RBAC denial, image pull failure and an unschedulable pod are each reported with their
  reason within the start timeout in 100% of CI runs; none ends as an indefinite `starting`.
- **SC-004**: The pod has 0 secrets mounted beyond those the user allowed, and runs as non-root with a
  read-only root filesystem.
- **SC-005**: After **Stop** and a new start, 100% of the daemon's worktrees are present; after
  **Remove**, 0 cluster resources created for the daemon remain.
- **SC-006**: 0 Services or Ingresses exist for any Kubernetes daemon.

## Assumptions

- The user has `kubectl`-style access through a kubeconfig, a namespace they may create pods, secrets,
  PVCs and port-forwards in, and a default (or chosen) storage class. The app does not install or
  manage clusters, namespaces or RBAC.
- One volume per daemon holds every project bound to it; volume size defaults to 10 GiB.
- The pod is a bare pod managed by the client, not a Deployment or StatefulSet; the client recreates it
  on start.
- Default resources match #806's container defaults; node selector and tolerations default to none.
- The in-cluster daemon is the same build as the client's (version-matched), consistent with #806 and
  spec 489 FR-005.
- Multiple clients connecting to one Kubernetes daemon behave as spec 489 FR-021.

## Out of Scope

- Creating or managing clusters, namespaces, RBAC roles or storage classes.
- Exposing the daemon via Service, Ingress, LoadBalancer or NodePort.
- Moving worktrees or sessions between a Kubernetes daemon and another daemon.
- Autoscaling, multiple pods per daemon, or high availability.
- Sharing one volume between several daemons.
