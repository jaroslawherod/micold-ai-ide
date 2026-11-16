# Ops HTTP contract (M2)

Listener bound only when `--ops-addr` / `MICOLD_ORCHESTRATOR_OPS_ADDR` is set.

| Request | Response |
|---|---|
| `GET /healthz` | 200 `ok` whenever the process answers |
| `GET /readyz` | 200 `ready` once the registry is loaded and the link listener is bound; else 503 `not ready` |
| other path | 404; other method on these paths: 405 |

No `/metrics` (#1742). Plain text, `Connection: close`.
