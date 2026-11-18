#!/usr/bin/env bash
# The chart's kind-mode values deploy/helm/micold/ci/local.yaml (#1732 M5, FR-020, FR-021, FR-022):
# Postgres StatefulSet, Service and Secret; the orchestrator on state.mode=postgres reading the URL
# from that Secret through a [database] config; the seeded daemon sidecars dialling its socket; the
# allow-origin list; the web image on 8731; and the exact state.mode refusal. Needs `helm`
# (scripts/install-kube-tools.sh); skips without it. kubeconform runs in scripts/chart-check.sh.

set -uo pipefail
ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
CHART="$ROOT/deploy/helm/micold"
command -v helm >/dev/null 2>&1 || { echo "chart-local.test: SKIP: helm is not on PATH (scripts/install-kube-tools.sh installs it)"; exit 0; }

failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

render() { helm template micold "$CHART" --namespace micold "$@" 2>&1; }
# doc <file> <kind> <name>: the one rendered document of that kind and name.
doc() {
  python3 - "$@" <<'PY'
import sys, yaml
f, kind, name = sys.argv[1:4]
for d in yaml.safe_load_all(open(f)):
    if d and d.get("kind") == kind and d["metadata"]["name"] == name:
        print(yaml.safe_dump(d)); break
PY
}

if ! render -f "$CHART/ci/local.yaml" >"$tmp/local.yaml"; then fail "ci/local.yaml does not render: $(cat "$tmp/local.yaml")"; fi
doc "$tmp/local.yaml" StatefulSet micold-postgres >"$tmp/pg"
grep -q "image: docker.io/library/postgres:18.7-alpine3.24@sha256:" "$tmp/pg" && grep -q "volumeClaimTemplates" "$tmp/pg" && pass "Postgres StatefulSet with a pinned image and a PVC" || fail "postgres statefulset: $(cat "$tmp/pg")"
doc "$tmp/local.yaml" Service micold-postgres | grep -q "port: 5432" && pass "Postgres Service on 5432" || fail "postgres service"
doc "$tmp/local.yaml" Secret micold-postgres >"$tmp/sec"
grep -q "url:" "$tmp/sec" && grep -q "password:" "$tmp/sec" && pass "Postgres Secret holds the password and the URL" || fail "postgres secret: $(cat "$tmp/sec")"
doc "$tmp/local.yaml" ConfigMap micold-orchestrator-config >"$tmp/cm"
grep -q 'url_file = \\"/etc/micold/database/url\\"\|url_file = "/etc/micold/database/url"' "$tmp/cm" && pass "orchestrator config names the URL file" || fail "orchestrator config: $(cat "$tmp/cm")"
doc "$tmp/local.yaml" Deployment micold-orchestrator >"$tmp/orch"
grep -q -- "- --config=/etc/micold/daemon.toml" "$tmp/orch" && grep -q "secretName: micold-postgres" "$tmp/orch" && pass "orchestrator runs with the database config and Secret" || fail "orchestrator db wiring"
grep -q -- "- --api-addr=127.0.0.1:8730" "$tmp/orch" && grep -q -- "- --allow-origin=http://127.0.0.1:8731" "$tmp/orch" && grep -q -- "- --allow-origin=http://localhost:8731" "$tmp/orch" && pass "API on pod loopback, both origins allowed" || fail "api/origin args: $(grep -- '--' "$tmp/orch")"
grep -q "name: daemon-0" "$tmp/orch" && grep -q "name: seed-0" "$tmp/orch" && pass "a seeded daemon sidecar dials the orchestrator socket" || fail "daemon sidecar missing"
doc "$tmp/local.yaml" ConfigMap micold-seed >"$tmp/seed"
grep -q "seed-catalog.sh" "$tmp/seed" && pass "the seed script ships in a ConfigMap" || fail "seed configmap"
doc "$tmp/local.yaml" Deployment micold-web >"$tmp/web"
grep -q 'image: micold-web:local' "$tmp/web" && grep -q "containerPort: 8731" "$tmp/web" && pass "web image micold-web:local on 8731" || fail "web: $(cat "$tmp/web")"

# Defaults render no Postgres, no daemon, no api args.
render >"$tmp/default.yaml"
grep -q "micold-postgres\|daemon-0\|--api-addr" "$tmp/default.yaml" && fail "defaults render kind-mode pieces" || pass "defaults render none of it"

# state.mode: file and postgres render, anything else fails with the exact message.
for mode in sqlite bogus; do
  out="$(render --set state.mode=$mode)"
  grep -qF "state.mode=$mode is not supported by this chart: use file or postgres" <<<"$out" && pass "state.mode=$mode refused" || fail "state.mode=$mode: $out"
done
render --set state.mode=postgres --set postgres.enabled=true >/dev/null && pass "state.mode=postgres renders" || fail "state.mode=postgres"
out="$(render --set state.mode=postgres)"
grep -q "state.mode=postgres needs postgres.enabled=true or database.existingSecret" <<<"$out" && pass "postgres without a database is refused" || fail "postgres without db: $out"

exit "$failures"
