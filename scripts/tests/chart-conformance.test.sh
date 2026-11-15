#!/usr/bin/env bash
# Rendered-template checks of the chart deploy/helm/micold (#1981: FR-002..004, FR-016, FR-019).
#
# `helm template` renders the chart with each case's values and the checks read the output. The
# literals below are copied from specs/1722-rn-client-k8s/plan.md (image name, `state.mode`,
# `database.existingSecret`), so a rename in the chart fails here before #1740 builds on it.
#
# Without `helm` on PATH it skips with the reason and exits 0, so `scripts/gate.sh` passes on a
# machine without it; MICOLD_CHART_REQUIRED=1 (the `kube-chart` CI job) turns the skip into a
# failure. CHART overrides the chart directory (the shell test of chart-check.sh uses it).

set -uo pipefail

ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
CHART="${CHART:-$ROOT/deploy/helm/micold}"

if ! command -v helm >/dev/null 2>&1; then
  if [ "${MICOLD_CHART_REQUIRED:-}" = 1 ]; then
    echo "chart-conformance: helm is not on PATH and MICOLD_CHART_REQUIRED=1" >&2
    exit 1
  fi
  echo "chart-conformance: SKIP: helm is not on PATH (scripts/install-kube-tools.sh installs it)"
  exit 0
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

failures=0
cases=0

ok() { cases=$((cases + 1)); printf 'ok    %s\n' "$1"; }
fail() { cases=$((cases + 1)); failures=$((failures + 1)); printf 'FAIL  %s\n' "$1"; }

# render <name> [helm template args...]: renders into $work/<name>.yaml, its stderr into .err.
render() {
  local name="$1"
  shift
  helm template micold "$CHART" --namespace micold "$@" >"$work/$name.yaml" 2>"$work/$name.err"
}

# check <description> <file> <grep -E pattern>: the rendered file has a line matching.
check() {
  if grep -qE -- "$3" "$2"; then ok "$1"; else fail "$1 (no line matching /$3/ in $(basename "$2"))"; fi
}

# refute <description> <file> <grep -E pattern>: no line matches.
refute() {
  if grep -qE -- "$3" "$2"; then fail "$1 (a line matches /$3/ in $(basename "$2"))"; else ok "$1"; fi
}

# kinds <file>: the rendered kinds, sorted, one per line.
kinds() { sed -n 's/^kind: //p' "$1" | sort | tr '\n' ' ' | sed 's/ $//'; }

# expect_kinds <description> <file> <kinds, sorted, space separated>
expect_kinds() {
  local got
  got="$(kinds "$2")"
  if [ "$got" = "$3" ]; then ok "$1"; else fail "$1 (want kinds '$3', got '$got')"; fi
}

# mount_of <file> <volume>: the mountPath of the volume mount named <volume> (first match).
mount_of() {
  awk -v vol="$2" '
    /volumeMounts:/ { inmounts = 1; next }
    inmounts && $0 ~ "- name: " vol "$" { want = 1; next }
    want && /mountPath:/ { print $2; exit }
    /volumes:/ { inmounts = 0 }
  ' "$1"
}

# volume_source <file> <volume>: the source key of the pod volume named <volume>
# (persistentVolumeClaim, emptyDir, ...).
volume_source() {
  awk -v vol="$2" '
    /^      volumes:/ { invols = 1; next }
    invols && $0 ~ "- name: " vol "$" { want = 1; next }
    want { sub(/^ +/, ""); sub(/:.*/, ""); print; exit }
  ' "$1"
}

# --- default values ------------------------------------------------------------------------------

if render default; then
  ok "default values render"
  d="$work/default.yaml"
  expect_kinds "default renders a Deployment and a PersistentVolumeClaim only" "$d" \
    "Deployment PersistentVolumeClaim"
  check "one replica" "$d" '^  replicas: 1$'
  check "strategy Recreate" "$d" '^    type: Recreate$'
  check "image is the 1722 plan's orchestrator image" "$d" \
    '^ +image: "ghcr\.io/jaroslawherod/micold-orchestrator:[^"]+"$'
  check "runAsNonRoot" "$d" '^ +runAsNonRoot: true$'
  check "UID 65532" "$d" '^ +runAsUser: 65532$'
  check "read-only root filesystem" "$d" '^ +readOnlyRootFilesystem: true$'
  check "no privilege escalation" "$d" '^ +allowPrivilegeEscalation: false$'
  check "all capabilities dropped" "$d" '^ +- ALL$'
  check "RuntimeDefault seccomp" "$d" '^ +type: RuntimeDefault$'
  check "no service account token" "$d" '^ +automountServiceAccountToken: false$'
  refute "no web resource while web.enabled=false" "$d" 'app\.kubernetes\.io/component: web'

  data_dir="$(sed -n 's/^ *- --data-dir=//p' "$d")"
  socket="$(sed -n 's/^ *- --socket=//p' "$d")"
  if [ "$data_dir" = /var/lib/micold ]; then ok "--data-dir is /var/lib/micold"; else fail "--data-dir is /var/lib/micold (got '$data_dir')"; fi
  if [ "$socket" = /run/micold/orchestrator.sock ]; then ok "--socket is /run/micold/orchestrator.sock"; else fail "--socket is /run/micold/orchestrator.sock (got '$socket')"; fi
  got="$(mount_of "$d" data)"
  if [ -n "$data_dir" ] && [ "$got" = "$data_dir" ]; then ok "data volume mount equals --data-dir"; else fail "data volume mount equals --data-dir (mount '$got', flag '$data_dir')"; fi
  got="$(mount_of "$d" run)"
  if [ -n "$socket" ] && [ "$got" = "$(dirname "$socket")" ]; then ok "run volume mount holds the socket"; else fail "run volume mount holds the socket (mount '$got', socket '$socket')"; fi
  got="$(mount_of "$d" tmp)"
  if [ "$got" = /tmp ]; then ok "tmp volume at /tmp"; else fail "tmp volume at /tmp (got '$got')"; fi
  got="$(volume_source "$d" data)"
  if [ "$got" = persistentVolumeClaim ]; then ok "data volume is the PVC"; else fail "data volume is the PVC (got '$got')"; fi
  for vol in run tmp; do
    got="$(volume_source "$d" "$vol")"
    if [ "$got" = emptyDir ]; then ok "$vol volume is an emptyDir"; else fail "$vol volume is an emptyDir (got '$got')"; fi
  done
  check "PVC is ReadWriteOnce" "$d" '^    - ReadWriteOnce$'
  check "PVC asks for 1Gi" "$d" '^      storage: "1Gi"$'
else
  fail "default values render: $(cat "$work/default.err")"
fi

# --- image values (FR-002) -----------------------------------------------------------------------

if render image --set image.repository=example.test/o --set image.tag=dev --set image.pullPolicy=Never; then
  check "image repository and tag are values" "$work/image.yaml" '^ +image: "example\.test/o:dev"$'
  check "pull policy is a value" "$work/image.yaml" '^ +imagePullPolicy: Never$'
else
  fail "image values render: $(cat "$work/image.err")"
fi

# --- web (FR-004) --------------------------------------------------------------------------------

if render web -f "$CHART/ci/web-enabled.yaml"; then
  w="$work/web.yaml"
  expect_kinds "web.enabled adds a web Deployment and Service" "$w" \
    "Deployment Deployment PersistentVolumeClaim Service"
  check "web image from web.image" "$w" '^ +image: "ghcr\.io/jaroslawherod/micold-web:dev"$'
  got="$(awk '/^kind: Service$/ { svc = 1 } svc && /^  selector:$/ { sel = 1; next }
    sel && /^    / { print; next } sel { exit }' "$w" | grep -c 'app\.kubernetes\.io/component: web$')"
  if [ "$got" = 1 ]; then ok "web Service selects the web pods"; else fail "web Service selects the web pods (selector has no web component)"; fi
else
  fail "web-enabled values render: $(cat "$work/web.err")"
fi

if render webnoimage --set web.enabled=true --set web.image.repository=; then
  fail "web.enabled without an image repository must fail"
else
  check "web.enabled without an image names the value" "$work/webnoimage.err" 'web[./]image[./]repository'
fi

# --- persistence off (US1-3) ---------------------------------------------------------------------

if render nopvc -f "$CHART/ci/no-persistence.yaml"; then
  expect_kinds "no PVC when state.persistence.enabled=false" "$work/nopvc.yaml" "Deployment"
  got="$(volume_source "$work/nopvc.yaml" data)"
  if [ "$got" = emptyDir ]; then ok "data volume is an emptyDir without persistence"; else fail "data volume is an emptyDir without persistence (got '$got')"; fi
  got="$(mount_of "$work/nopvc.yaml" data)"
  if [ "$got" = /var/lib/micold ]; then ok "emptyDir still mounted at the data dir"; else fail "emptyDir still mounted at the data dir (got '$got')"; fi
else
  fail "no-persistence values render: $(cat "$work/nopvc.err")"
fi

# --- state modes (FR-003, FR-016) ----------------------------------------------------------------

for mode in postgres sqlite; do
  if render "mode-$mode" --set "state.mode=$mode"; then
    fail "state.mode=$mode must fail to render"
  else
    e="$work/mode-$mode.err"
    check "state.mode=$mode says it is not available yet" "$e" "state\.mode=$mode is not available yet"
    check "state.mode=$mode names #1948" "$e" '#1948'
    check "state.mode=$mode names #1740" "$e" '#1740'
  fi
done

# The 1722 plan's value names: a renamed key is refused by the schema (additionalProperties), so
# setting the planned names must keep rendering.
if render names --set state.mode=file --set database.existingSecret=micold-db; then
  ok "state.mode and database.existingSecret are the chart's value names"
else
  fail "state.mode and database.existingSecret are the chart's value names: $(cat "$work/names.err")"
fi
if render norepo --set image.repository=; then
  fail "an empty image.repository must be refused"
else
  ok "an empty image.repository is refused"
fi
if render unknown --set database.url=x; then
  fail "an unknown value name must be refused by the schema"
else
  ok "an unknown value name is refused by the schema"
fi

echo
if [ "$failures" -ne 0 ]; then
  echo "chart-conformance: $failures of $cases check(s) failed"
  exit 1
fi
echo "chart-conformance: all $cases checks passed"
