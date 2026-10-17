# Quickstart: validate the Kubernetes daemon runtime (#686)

Contracts: [contracts/kube-runtime.md](./contracts/kube-runtime.md),
[contracts/daemon-image.md](./contracts/daemon-image.md).

## Part A — automated

Prerequisites (Linux): docker, `kind`, `kubectl` on `PATH`.

1. `mise run test-core` and `mise run test` — pure rules and `FakeCluster` sequences; no cluster.
2. `mise run test-kube` — creates the kind cluster `micold-kube-test` with a registry on
   `localhost:5001`, runs `kube_real_*` one at a time, deletes the cluster. Expected: all pass;
   covers start (build → push → pull by digest), tunnel, session input/output with the stand-in CLI,
   reconnect after a dropped tunnel, stop (PVC kept, worktree present after restart), remove (0
   labelled objects left), RBAC denial (`create pods`, `pods/portforward`), image pull failure,
   unschedulable pod, security context and mounted secrets, two concurrent starts → one pod.
3. CI: job `kube-cluster` runs step 2 on every non-docs change.

## Part B — manual, at a display (visual pass)

Prerequisites: a reachable cluster (kind from step A2 is fine, left running with
`MICOLD_KUBE_TEST_KEEP=1`), a namespace, docker logged in to a registry the cluster can pull from.

1. Settings → Daemons → Add → runtime **Kubernetes**. Expected: context picker lists the kubeconfig's
   contexts; with `KUBECONFIG` pointing nowhere, the form says "no kubeconfig found; looked in …".
2. Leave image source **Build**, clear Registry, Save. Expected: refused naming "Registry".
3. Fill registry, select `claude`, Save, Start. Expected: row activity "Building image" with Cancel,
   then "Pushing image", "Pulling image", then `connected`; endpoint `<context> / <ns> / <pod>`.
4. Create a worktree of a project with an `origin` remote bound to the daemon. Expected: "Cloning …",
   then the worktree row shows the daemon name and `/data/repos/…` path. A project without remote:
   refused with the contract §6 text.
5. Start a session; type; quit the app; reopen. Expected: the session is running with output
   produced meanwhile.
6. `kubectl delete pod <pod>` while connected. Expected: `reconnecting`, then `stopped` "the pod was
   deleted", sessions `lost to its daemon`, no new pod until Start; Start → worktree still present.
7. Stop with a session running. Expected: confirmation with the count; pod gone, PVC kept.
8. Remove. Expected: confirmation naming pod, volume, secrets; afterwards
   `kubectl get all,pvc,secret -l app.kubernetes.io/managed-by=micold-ai-ide` returns nothing.
9. Look at light and dark themes of the form and row states (`starting`, `connected`, `reconnecting`,
   `stopped` with reason, failure text). Record screenshots under `specs/686-kubernetes-daemon-runtime/visual-pass/`.
