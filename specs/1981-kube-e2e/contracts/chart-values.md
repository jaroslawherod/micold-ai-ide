# Chart values contract (`deploy/helm/micold`)

| Key | Default | Notes |
|---|---|---|
| `image.repository` | `ghcr.io/jaroslawherod/micold-orchestrator` | name fixed by the 1722 plan; conformance-tested |
| `image.tag` | chart `appVersion` | CI sets `dev` |
| `image.pullPolicy` | `IfNotPresent` | kind uses `Never` via `ci/e2e.yaml` |
| `state.mode` | `file` | `file` or `postgres` (the #1948 store; needs `postgres.enabled` or `database.existingSecret`); anything else fails: `state.mode=<v> is not supported by this chart: use file or postgres` (#1732 M5) |
| `state.persistence.enabled` / `size` / `storageClass` | `true` / `1Gi` / unset | false renders `emptyDir` |
| `database.existingSecret` | `""` | `state.mode=postgres`: a Secret with the URL under `url`; empty uses the chart's Postgres |
| `postgres.enabled` / `image` / `size` / `storageClass` | `false` / `postgres:18.7-alpine3.24@sha256:…` / `1Gi` / unset | #1732 M5: StatefulSet, Service and a Secret (generated password, kept across upgrades, and the URL) |
| `orchestrator.api.enabled` / `port` | `false` / `8730` | `--api-addr=127.0.0.1:<port>`: pod loopback, reached by port-forward (no TLS until #1733) |
| `orchestrator.allowOrigins` | `[]` | one `--allow-origin=<origin>` each |
| `daemons.count` / `seedImage` | `0` / `alpine/git:v2.54.1@sha256:…` | #1732 M5: seeded daemon sidecars `daemon-<i>` in the orchestrator pod (the link is a local socket), each with a `seed-<i>` init container running `files/seed-catalog.sh` |
| `ops.enabled` / `ops.port` | `true` (M2) / `9090` | adds `--ops-addr=0.0.0.0:<port>` and probes |
| `web.enabled` / `web.image.*` | `false` | renders nothing when false; with true and an image, Deployment + Service |
| `testDaemon.enabled` | `false` | M4, set only by `ci/e2e.yaml`: second container sharing `/run/micold` |
| `testing.corruptRegistry.enabled` | `false` | M4, only `ci/corrupt-registry.yaml`: pinned busybox init container writes an invalid `orchestrator/registry.json` |
| `testing.breakage` | `none` | M4, test-only enum `none\|command\|user\|volume\|dataDir\|opsPort`, set only by the harness matrix (T029) via `--set`; each value renders one deliberately wrong manifest, bypassing the `securityContext` rule. `volume` is the same case as `ci/no-persistence.yaml` (emptyDir) |
| `securityContext` | non-root 65532, read-only root FS, no escalation, drop ALL, `RuntimeDefault` | not user-weakenable in the schema except by `testing.breakage=user` |

Fixed paths: data `/var/lib/micold`, socket `/run/micold/orchestrator.sock`, tmp `/tmp` (emptyDir). The mount path equals the `--data-dir` argument (tested).
