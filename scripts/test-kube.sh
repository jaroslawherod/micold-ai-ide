#!/usr/bin/env bash
# Run the `kube_real_*` tests (feature 686, FR-021) against a throwaway kind cluster. Used by
# `mise run test-kube` and by CI's `kube-cluster` job. Needs docker, kind and kubectl on PATH; the
# tests create the cluster `micold-kube-test` and the stand-in images themselves.
#
# MICOLD_KUBE_TEST_KEEP=1 keeps the cluster and the test namespaces afterwards.
# MICOLD_KUBE_AWAY_SECS is how long the reconnect test keeps the client away (default 60; 600 for
# the SC-002 run).
# CARGO_WRAP (e.g. scripts/build-lock.sh) prefixes the cargo calls.
set -euo pipefail
cd "$(dirname "$0")/.."

for tool in docker kind kubectl; do
  command -v "$tool" >/dev/null 2>&1 || { echo "test-kube: $tool is not installed" >&2; exit 1; }
done

cleanup() {
  # The git remote the tests start outlives each test binary; stop it with the run.
  pkill -f 'git[ -]daemon .*--port=9418' 2>/dev/null || true
  if [ "${MICOLD_KUBE_TEST_KEEP:-}" != 1 ]; then
    kind delete cluster --name micold-kube-test >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

# shellcheck disable=SC2086
${CARGO_WRAP:-} cargo build -p micold-daemon
# shellcheck disable=SC2086
${CARGO_WRAP:-} cargo test -p micold-client --features kube-real-runtime \
  kube_real_ --no-fail-fast -- --test-threads=1
