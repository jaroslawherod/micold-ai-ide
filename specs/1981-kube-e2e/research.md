# Research: kind e2e and Helm chart (#1981)

Each decision lists what was rejected.

## R1. Reach the link protocol through a test-only daemon sidecar (D1)
The orchestrator listens only on a local socket. Decision: second container in the same pod sharing an `emptyDir` at `/run/micold`. Rejected: `kubectl exec` of a socat bridge into a TCP port (adds a TCP link the product does not have and tests a path no daemon takes); waiting for #1729's network link (blocks the epic); a separate daemon pod (cannot share the socket).

## R2. State mode `file` on a PVC (D2)
Decision: `file` only; others fail at render. Rejected: SQLite on a PVC (#1948's schema, not ready for the chart); PostgreSQL via CloudNativePG (#1740 owns it, and a CI database multiplies live rounds); `emptyDir` default (loses restart-persistence coverage; kept as `state.persistence.enabled=false`).

## R3. Ops listener without an HTTP framework
Two fixed GET routes, no body parsing. Decision: `tokio::net::TcpListener`, read the request line, write a fixed `HTTP/1.1` response, close. Rejected: `axum`/`hyper` now (a framework choice that belongs to #1732 and #1742, and a new dependency tree for two routes); a Unix-socket probe (kubelet probes are HTTP/TCP/exec, and distroless has no exec binary). #1742 may replace the module wholesale when it adds `/metrics`.

## R4. Hand-rolled flag parsing
`parse_mode` already inspects raw `OsString` arguments and the daemon has no CLI crate dependency. Decision: small parser with env fallback injected as a map, for tests. Rejected: adding `clap` for three flags.

## R5. Milestone order: ops before harness
See plan Summary. Rejected: keeping the spec's order and giving M2's smoke an unprobed pod (Ready means "container started", so the smoke proves nothing about readiness and the probe wiring would first be exercised in M4).

## R6. One shared pinned-tools script
`kube-cluster` inlines kind and kubectl pins in `ci.yml`. Decision: move them to `scripts/install-kube-tools.sh` that both jobs call and that also pins helm and kubeconform. Rejected: a composite action (pins then live in YAML env, not testable by a local shell test); duplicating pins (drift).

## R7. Chart checks: helm lint + kubeconform + template conformance
`helm lint` catches template errors, `kubeconform` schema errors, the conformance test catches name drift against the 1722 plan (FR-016). Rejected: `chart-testing` (`ct`), which wants a cluster for install tests and duplicates the kind job.

## R8. Image base
`gcr.io/distroless/cc-debian12:nonroot` (UID 65532) matches the 1722 plan. The builder is the `rust` Debian image so glibc matches. Rejected: `scratch` plus musl (the daemon links `cc` libs; changing target is out of scope), Alpine (no `cc` match).

## R9. e2e as a Rust crate, not shell
Reuses link-protocol types and `tokio`, and skips cleanly in `cargo test --workspace`. Rejected: bats/shell assertions (cannot speak the link protocol), extending `micold-client`'s feature-gated tests (the e2e needs its own env contract and suite layout).

## R10. Kind and helm usage
Reuse `kind` v0.30.0 and `kubectl` v1.34.0 pins and the `curl | sha256sum --check | install` pattern from the `kube-cluster` job. `helm` and `kubeconform` get the same pattern; exact versions and sha256 are looked up from the upstream release `checksums` files in the M1 task, not guessed here.
