# micold Helm chart

Deploys the micold orchestrator (`micold-daemon orchestrator`) and, optionally, the web UI. This is
the first slice of the Kubernetes deployment in `specs/1722-rn-client-k8s/plan.md`; #1740 extends
it. Design and values contract: `specs/1981-kube-e2e/` (`contracts/chart-values.md`).

```sh
docker build -f Dockerfile.orchestrator -t ghcr.io/jaroslawherod/micold-orchestrator:dev .
helm install micold deploy/helm/micold -n micold --create-namespace --set image.tag=dev
```

No image is published yet: the release workflow that builds, signs and pushes the images, and keeps
`appVersion` in step with the release, belongs to #1740. Until then build `:dev` locally (and load
it into the cluster, e.g. `kind load docker-image`, or with podman `podman save -o img.tar <image>` and
`kind load image-archive img.tar`) as above. `scripts/local-stack-kind.sh` and `scripts/e2e-kind.sh` use docker,
else podman; `MICOLD_CONTAINER_RUNTIME` picks one (`scripts/container-runtime.sh`).

## What it renders

| Values | Resources |
|---|---|
| defaults | orchestrator `Deployment` (1 replica, `Recreate`), its `PersistentVolumeClaim` and its ops `Service` |
| `state.persistence.enabled=false` | no `PersistentVolumeClaim`; the data directory is an `emptyDir` |
| `ops.enabled=false` | no ops `Service`, no `--ops-addr` and no probes |
| `web.enabled=true` | also a web `Deployment` and `Service` (needs `web.image.repository`) |

With `ops.enabled` the orchestrator runs `--ops-addr=0.0.0.0:<ops.port>`: `GET /healthz` answers
200 whenever the process does, `GET /readyz` 200 once the registry is loaded and the link listener
is bound (503 before). The startup and liveness probes use `/healthz`, the readiness probe `/readyz`,
and the `<release>-orchestrator` Service exposes the port. No `/metrics` yet (#1742).

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
| `state.mode` | `file` | `file` or `postgres` (needs `postgres.enabled` or `database.existingSecret`); anything else fails to render |
| `state.persistence.enabled` | `true` | `false` loses the registry with the pod |
| `state.persistence.size` | `1Gi` | |
| `state.persistence.storageClass` | `""` | empty uses the cluster default |
| `database.existingSecret` | `""` | `state.mode=postgres`: a Secret holding the database URL under `url` |
| `postgres.enabled` / `image` / `size` / `storageClass` | `false` / pinned `postgres:18.7-alpine3.24` / `1Gi` / `""` | a single-instance Postgres for `state.mode=postgres` |
| `orchestrator.api.enabled` / `port` | `false` / `8730` | the client API on the pod's loopback, reached with `kubectl port-forward` |
| `orchestrator.allowOrigins` | `[]` | web origins allowed to open `/api/ws` |
| `daemons.count` / `seedImage` | `0` / pinned `alpine/git` | seeded daemons as sidecars of the orchestrator pod (`ci/local.yaml`, `mise run local-stack`) |
| `ops.enabled` / `ops.port` | `true` / `9090` | the ops listener, its Service and the probes |
| `web.enabled` | `false` | renders nothing when false |
| `web.image.repository` / `tag` / `pullPolicy` | `ghcr.io/jaroslawherod/micold-web` / `appVersion` / `IfNotPresent` | |
| `web.containerPort` / `web.service.port` | `8080` / `80` | |
| `resources` | `{}` | the orchestrator container's requests and limits |

| `testDaemon.enabled` | `false` | test only (`ci/e2e.yaml`): a second container, same image, running `micold-daemon e2e-link idle` on the shared `/run/micold` volume |
| `testing.corruptRegistry.enabled` | `false` | test only (`ci/corrupt-registry.yaml`): a digest-pinned busybox init container writes an invalid `orchestrator/registry.json` |
| `testing.breakage` | `none` | test only: `command`, `user`, `volume`, `dataDir` or `opsPort` renders one deliberately wrong manifest for the e2e seeded-breakage matrix |

`values.schema.json` refuses any other key.

## Checking it

`mise run chart-check` (or `scripts/chart-check.sh`) runs `helm lint`, `kubeconform` over the
defaults and every `ci/*.yaml`, then `scripts/tests/chart-conformance.test.sh`. It needs `helm` and
`kubeconform`; `scripts/install-kube-tools.sh helm kubeconform` installs the pinned versions.
`scripts/chart-check.sh --image` builds `Dockerfile.orchestrator` and runs it as UID 65532 on a
read-only root. CI runs both in the `kube-chart` job.
