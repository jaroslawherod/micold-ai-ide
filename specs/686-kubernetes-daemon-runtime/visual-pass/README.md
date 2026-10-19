# Visual pass — #686 Kubernetes daemon runtime (T097)

Run 2026-10-19 on main at 5c479c10. The client (`micold-ai-ide`) and daemon were built from that tree
and run on a private Xvfb display with `ICED_BACKEND=tiny-skia`, driven with xdotool, with a private
data directory and a throwaway kubeconfig. The kubeconfig had contexts `kind-dev` (namespace `dev`)
and `prod-eu`, and its server was unreachable. The container has no Kubernetes cluster, no `ssh` and
no `ssh-keygen`.

| Quickstart Part B | Screenshot | Seen | Result |
|---|---|---|---|
| 1. Add → runtime Kubernetes, context picker | `01-add-kubernetes-contexts-light.png` | The runtime list offers Kubernetes. The Connection section lists the kubeconfig's contexts as chips, followed by Context* and Namespace*. Picking `kind-dev` fills its namespace `dev`. | pass |
| 1. AI CLI checkboxes | `07-image-ai-clis-light.png` | The Image field reads "Empty uses the default SSH image". The AI CLI list has Claude Code ticked, the provider enabled in Settings. | pass |
| Form: credentials, resources | `06-credentials-resources-light.png` | The allowed credentials list opens with "Nothing reaches the pod unless it is ticked here". It shows CPU/memory request and limit with their units. | pass |
| Form: placement and storage | `05-placement-storage-volume-note-light.png` | Shows the node selector, tolerations and volume size (10Gi). The volume size shows "Applies only to a new volume, after Remove". Storage class reads "Empty uses the cluster default". | pass |
| Blank namespace refused | `04-blank-namespace-refused-light.png` | Saving with the namespace cleared turns the Namespace* field red with "This field is required", and the form stays open. | pass |
| Row endpoint and failure text | `02-row-endpoint-and-failure-light.png` | The row reads `kind-dev / dev / micold-cluster-bded006d` with Kubernetes and Unreachable chips. The reason is "cannot make an SSH key: is `ssh-keygen` installed?", which is correct for this container. | pass |
| Edit opens the Kubernetes form | `03-edit-connection-light.png` | Edit shows the saved context and namespace, with the context chip selected. | pass |
| 9. Dark theme | `08-row-and-form-dark.png`, `09-runtime-list-dark.png` | The row, failure text, form sections and runtime list are legible in dark, with no clipped or overlapping glyphs. | pass |
| 2–8. Start, drift, clone, survive quit, pod deleted, stop, remove | — | Not runnable here, because there is no cluster. The real-cluster CI job `kubernetes daemon on a real cluster (linux)` covers the same flows. It runs the `kube_real_*` suite: end to end start to typed input, reconnect, deleted pod, stop and remove with nothing left, failures, and concurrent starts. That job was green on #1047 (703099f) and on main. | not runnable, covered by CI |
| 9. Row states starting / connected / reconnecting | — | These states need a reachable cluster. | not runnable here |

A grey pill over the "Runs in" field while the list opened was a press ripple caught mid-frame under
software rendering. It is gone once the frame settles (`09-runtime-list-dark.png`), so it is not a
defect. No visual defects were found.
