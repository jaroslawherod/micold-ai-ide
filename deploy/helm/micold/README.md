# micold Helm chart

Deploys the micold orchestrator (`micold-daemon orchestrator`) and, optionally, the web UI. This is
the first slice of the Kubernetes deployment in `specs/1722-rn-client-k8s/plan.md`; #1740 extends
it. Design and values contract: `specs/1981-kube-e2e/` (`contracts/chart-values.md`).

```sh
helm install micold deploy/helm/micold -n micold --create-namespace
```

## What it renders

| Values | Resources |
|---|---|
| defaults | orchestrator `Deployment` (1 replica, `Recreate`) and its `PersistentVolumeClaim` |
| `state.persistence.enabled=false` | the `Deployment` only; the data directory is an `emptyDir` |
| `web.enabled=true` | also a web `Deployment` and `Service` (needs `web.image.repository`) |

The orchestrator's Service and probes arrive with its ops listener (#1983).

Every pod runs as UID 65532 with `runAsNonRoot`, a read-only root filesystem, no privilege
escalation, all capabilities dropped, the `RuntimeDefault` seccomp profile and no service account
token. These are fixed, not values.

The orchestrator runs with fixed paths: `--data-dir=/var/lib/micold` (the PVC or `emptyDir`; the
registry is `orchestrator/registry.json` inside it), `--socket=/run/micold/orchestrator.sock` (an
`emptyDir`) and `/tmp` (an `emptyDir`).

## Values

| Key | Default | Notes |
|---|---|---|
| `image.repository` | `ghcr.io/jaroslawherod/micold-orchestrator` | |
| `image.tag` | the chart's `appVersion` | CI and the kind harness use `dev` |
| `image.pullPolicy` | `IfNotPresent` | |
| `state.mode` | `file` | the only mode; `sqlite` (#1948) and `postgres` (#1740) fail to render |
| `state.persistence.enabled` | `true` | `false` loses the registry with the pod |
| `state.persistence.size` | `1Gi` | |
| `state.persistence.storageClass` | `""` | empty uses the cluster default |
| `database.existingSecret` | `""` | reserved for #1948 / #1740 |
| `web.enabled` | `false` | renders nothing when false |
| `web.image.repository` / `tag` / `pullPolicy` | `ghcr.io/jaroslawherod/micold-web` / `appVersion` / `IfNotPresent` | |
| `web.containerPort` / `web.service.port` | `8080` / `80` | |
| `resources` | `{}` | the orchestrator container's requests and limits |

`values.schema.json` refuses any other key.

## Checking it

`mise run chart-check` (or `scripts/chart-check.sh`) runs `helm lint`, `kubeconform` over the
defaults and every `ci/*.yaml`, then `scripts/tests/chart-conformance.test.sh`. It needs `helm` and
`kubeconform`; `scripts/install-kube-tools.sh helm kubeconform` installs the pinned versions.
`scripts/chart-check.sh --image` builds `Dockerfile.orchestrator` and runs it as UID 65532 on a
read-only root. CI runs both in the `kube-chart` job.
