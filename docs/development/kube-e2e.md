# End-to-end tests on kind

The orchestrator's end-to-end suites run against the real image and the real Helm chart on a
throwaway [kind](https://kind.sigs.k8s.io/) cluster. `scripts/e2e-kind.sh` does the whole run, and
CI's `kube-e2e` job runs the same script on every change to a `micold-kube` path
(`.gitattributes`).

## Run it locally

You need docker, kind, kubectl, helm and cargo on `PATH`. `scripts/install-kube-tools.sh kind
kubectl helm` installs the pinned versions into `~/.local/bin`.

```sh
mise run e2e-kind
```

The script:

1. deletes any cluster with the same name, then creates `micold-e2e`;
2. builds `Dockerfile.orchestrator` as `ghcr.io/jaroslawherod/micold-orchestrator:e2e` and
   loads it into the cluster with `kind load docker-image`;
3. installs the chart `deploy/helm/micold` with `ci/e2e.yaml` and `--wait`, into namespace
   `micold-e2e`;
4. runs `cargo test -p micold-e2e --test <suite>` once per suite, in the order `ops`, `restart`,
   `link` (`restart` expects an empty registry, so it goes before the suites that register
   daemons), with `MICOLD_E2E=1` and `MICOLD_E2E_REQUIRED=1`;
5. runs `refusal` last: it reinstalls the chart with `ci/corrupt-registry.yaml` (an init container
   plants an invalid `orchestrator/registry.json`) and expects the pod never Ready and its log to
   name the file;
6. runs the seeded-breakage matrix (SC-003): for each of `command`, `user`, `volume`, `dataDir`
   and `opsPort` it uninstalls the release (which deletes its PVC, so the corrupt registry
   cannot leak), installs it with `--set testing.breakage=<v>` and runs `ops`, `restart` and
   `link` until one fails. A breakage no suite catches fails the run;
7. on failure, writes pods, `describe`, events and container logs to
   `target/e2e-logs/<cluster>/` (CI uploads that directory as the `kube-e2e-logs` artifact);
8. deletes the cluster.

The `link` and `restart` suites reach the orchestrator's link socket through the chart's test
daemon (`testDaemon.enabled`, set by `ci/e2e.yaml`): a second container, same image, running
`micold-daemon e2e-link idle`. The suites `kubectl exec` `e2e-link register --name N [--hold]` and
`e2e-link list` into it; no other path to the socket exists.

A failing step does not stop the later ones, so one CI round reports every failure; the exit status
is the first failure's.

The exit status is the suites' exit status. If the install never becomes Ready, the script prints
the pods, the `describe` output and the last log lines. On `ImagePullBackOff` it also names the
image the node could not pull.

## Skips

- The harness runs on Linux amd64 only. On any other host it prints a `skip` line and exits 0.
- A missing tool exits 1 and names the tool, before any cluster work.
- Run outside the harness (for example by `cargo test --workspace`), every suite test prints
  `skipped <test>: MICOLD_E2E is not set ...` and passes. With `MICOLD_E2E_REQUIRED=1` that skip
  fails the test instead.

## Environment

| Variable | Default | Meaning |
|---|---|---|
| `MICOLD_E2E_CLUSTER` | `micold-e2e` | kind cluster name; a distinct name runs beside another cluster |
| `MICOLD_E2E_NAMESPACE` | `micold-e2e` | namespace of the release, passed to the tests |
| `MICOLD_E2E_SUITES` | every directory under `crates/micold-e2e/tests` | comma list, e.g. `ops,link` |
| `MICOLD_E2E_MATRIX` | `1` with the default selection, else `0` | `1` runs the seeded-breakage matrix for a chosen selection, `0` skips it |
| `MICOLD_E2E_KEEP` | unset | `1` keeps the cluster for debugging |
| `MICOLD_E2E_LOG_DIR` | `target/e2e-logs/<cluster>` | where failure logs go |

The tests also read `MICOLD_E2E_CONTEXT` (default `kind-micold-e2e`), which the harness sets. Every
`kubectl` call names that context and the namespace, so a suite never acts on your current
context.

## Add a suite

A suite is a directory `crates/micold-e2e/tests/<name>/` with a `main.rs` (`mod` lines for its
files) and a `[[test]]` entry with the same name in `crates/micold-e2e/Cargo.toml`. Add the name to
`KNOWN_SUITES` in `scripts/e2e-kind.sh`. Every test starts with the gate:

```rust
let Some(cluster) = micold_e2e::env::cluster_or_skip("<suite>::<test>") else { return };
```

Use `cluster.exec(...)`, `cluster.port_forward(...)` and `cluster.kubectl(...)` from
`micold_e2e::cluster`. Do not call `kubectl` directly.

### Placeholder suites: `network` and `ui`

`network` (blocked on #1732) and `ui` (blocked on #1954) exist as placeholders. The harness lists
them as `suite <name>: skipped, blocked on #<issue>` and runs no cargo test for them; run directly,
their one test prints the same skip reason. To make one real, in its own directory plus the
harness list only:

1. Remove its entry from `PLACEHOLDERS` in `scripts/e2e-kind.sh`.
2. `network`: replace `crates/micold-e2e/tests/network/main.rs` with tests that start with the
   gate above. `ui`: add the Playwright project in `e2e/ui/` (see its README) and have the suite
   run it with the web URL from the harness.
3. Set any chart values the suite needs (for `ui`, `web.enabled=true`) in `ci/e2e.yaml`.

## Debugging

Keep the cluster with `MICOLD_E2E_KEEP=1 mise run e2e-kind`, then use the context `kind-micold-e2e`
and the namespace `micold-e2e`. The orchestrator image is distroless and has no shell. To look
inside the pod, attach an ephemeral debug container that shares the orchestrator's process
namespace:

```sh
kubectl --context kind-micold-e2e -n micold-e2e debug -it deploy/micold-orchestrator \
  --image=busybox:1.37 --target=orchestrator -- sh
```

Reach the ops port with `kubectl port-forward svc/micold-orchestrator 9090` and
`curl localhost:9090/readyz`. When you are done, delete the cluster with
`kind delete cluster --name micold-e2e`.
