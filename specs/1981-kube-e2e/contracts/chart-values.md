# Chart values contract (`deploy/helm/micold`)

| Key | Default | Notes |
|---|---|---|
| `image.repository` | `ghcr.io/jaroslawherod/micold-orchestrator` | name fixed by the 1722 plan; conformance-tested |
| `image.tag` | chart `appVersion` | CI sets `dev` |
| `image.pullPolicy` | `IfNotPresent` | kind uses `Never` via `ci/e2e.yaml` |
| `state.mode` | `file` | only `file`; others fail render naming #1948 and #1740 |
| `state.persistence.enabled` / `size` / `storageClass` | `true` / `1Gi` / unset | false renders `emptyDir` |
| `database.existingSecret` | `""` | reserved, unused until #1948/#1740 |
| `ops.enabled` / `ops.port` | `true` (M2) / `9090` | adds `--ops-addr=0.0.0.0:<port>` and probes |
| `web.enabled` / `web.image.*` | `false` | renders nothing when false; with true and an image, Deployment + Service |
| `testDaemon.enabled` | `false` | M4, set only by `ci/e2e.yaml`: second container sharing `/run/micold` |
| `testing.corruptRegistry.enabled` | `false` | M4, only `ci/corrupt-registry.yaml`: pinned busybox init container writes an invalid `orchestrator/registry.json` |
| `testing.breakage` | `none` | M4, test-only enum `none\|command\|user\|volume\|dataDir\|opsPort`, set only by the harness matrix (T029) via `--set`; each value renders one deliberately wrong manifest, bypassing the `securityContext` rule. `volume` is the same case as `ci/no-persistence.yaml` (emptyDir) |
| `securityContext` | non-root 65532, read-only root FS, no escalation, drop ALL, `RuntimeDefault` | not user-weakenable in the schema except by `testing.breakage=user` |

Fixed paths: data `/var/lib/micold`, socket `/run/micold/orchestrator.sock`, tmp `/tmp` (emptyDir). The mount path equals the `--data-dir` argument (tested).
