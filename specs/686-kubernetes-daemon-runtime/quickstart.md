# Quickstart: validate the Kubernetes daemon runtime (#686)

Contracts: [contracts/kube-runtime.md](./contracts/kube-runtime.md),
[contracts/shared-ssh-image.md](./contracts/shared-ssh-image.md). Steps marked **(after #806)** need
the shared SSH image mechanism and run from the milestone that wires it.

## Part A — automated

1. `mise run test-core` and `mise run test` — pure rules, the start/stop/remove sequences over
   `FakeCluster` and a fake `KeyGen`, the relay over an in-memory stream, the Settings reducers; no
   cluster. Expected: all pass. Before #806 this is the whole proof: start runs from `Config` to
   `Tunnel` (the SSH identification line read through the relay), every failure class of contract
   §6 up to `Tunnel` is reported with its text, rollback leaves only a pre-existing PVC, Stop keeps
   the PVC, Remove deletes every object we own and nothing else, two racing starts end with one pod
   and both clients' keys in the access secret, and the Add form does not list Kubernetes.
2. **(after #806)** `mise run test-kube` (Linux; docker, `kind`, `kubectl` on `PATH`) — creates the
   kind cluster `micold-kube-test`, makes the SSH image available to it, runs `kube_real_*` one at a
   time, deletes the cluster. Expected: all pass; covers start (pull, tunnel, daemon copy, CLI install
   of the stand-in CLI), session input/output, reconnect after a dropped tunnel, stop (PVC kept,
   worktree present after restart), remove (0 labelled objects left), RBAC denial (`create pods`,
   `pods/portforward`), image pull failure, unschedulable pod, CLI install failure, security context
   and mounted secrets, two concurrent starts → one pod, one daemon, both keys.
3. **(after #806)** CI: job `kube-cluster` runs step 2 on every non-docs change.

## Part B — manual, at a display (visual pass)

Before #806 (form only): run the component showcase or the client with a Kubernetes daemon already in
`settings.json`. Expected: the row shows `<context> / <ns> / <pod>`; Edit opens the Kubernetes form
(context picker or "no kubeconfig found; looked in …", namespace, image, AI CLIs, resources, node
selector, tolerations, volume size, storage class, allowed credentials); a blank namespace is refused
naming "Namespace"; editing the volume size shows "Applies only to a new volume, after Remove"; the
Add form's runtime list has no Kubernetes entry. Light and dark themes.

**(after #806)** Prerequisites: a reachable cluster (kind from step A2, left running with
`MICOLD_KUBE_TEST_KEEP=1`) and a namespace.

1. Settings → Daemons → Add → runtime **Kubernetes**. Expected: context picker lists the kubeconfig's
   contexts; AI CLI checkboxes default to the providers enabled in Settings.
2. Leave the image empty, select `claude`, Save, Start. Expected: row activity "Pulling image",
   "Opening tunnel", "Copying daemon", "Installing claude" (with Cancel), then `connected`; endpoint
   `<context> / <ns> / <pod>`.
3. Edit the node selector, Save; quit the app, reopen it and choose Start while the pod still runs.
   Expected:
   "The running pod differs from the settings (Node selector). Recreate it?" with the session count.
4. Create a worktree of a project with an `origin` remote bound to the daemon. Expected: "Cloning …",
   then the worktree row shows the daemon name and `/data/repos/…` path. A project without remote:
   refused with the contract §6 text.
5. Start a session; type; quit the app; reopen. Expected: the session is running with output
   produced meanwhile.
6. `kubectl delete pod <pod>` while connected. Expected: `reconnecting`, then `stopped` "the pod was
   deleted", sessions `lost to its daemon`, no new pod until Start; Start → worktree still present,
   daemon and CLI not copied or installed again.
7. Stop with a session running. Expected: confirmation with the count; pod gone, PVC kept.
8. Remove. Expected: confirmation naming pod, volume, secrets; afterwards
   `kubectl get all,pvc,secret -l app.kubernetes.io/managed-by=micold-ai-ide` returns nothing.
9. Look at light and dark themes of the form and row states (`starting`, `connected`, `reconnecting`,
   `stopped` with reason, failure text). Record screenshots under
   `specs/686-kubernetes-daemon-runtime/visual-pass/`.
