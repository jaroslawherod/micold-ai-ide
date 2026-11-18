#!/usr/bin/env bash
# scripts/container-runtime.sh (#2045): runtime detection and image loading with fake docker and
# podman on PATH. No container runtime is needed.

set -uo pipefail
ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/base" "$tmp/both" "$tmp/podman" "$tmp/none"
for t in bash env mktemp rm cat; do ln -s "$(command -v "$t")" "$tmp/base/$t"; done
mkstub() { # <dir> <name>
  printf '#!/usr/bin/env bash\necho "%s $*" >>"$STUB_LOG"\nif [ "%s" != kind ] && [ "$1" = save ]; then : >"$3"; fi\n' "$2" "$2" >"$1/$2"
  chmod +x "$1/$2"
}
for t in docker podman kind; do mkstub "$tmp/both" "$t"; done
for t in podman kind; do mkstub "$tmp/podman" "$t"; done

# probe <path-dirs> <env...>: prints "<runtime> <provider>" or the error.
probe() {
  local dirs="$1"; shift
  env -i PATH="$dirs:$tmp/base" STUB_LOG="$tmp/log" "$@" bash -c \
    ". '$ROOT/scripts/container-runtime.sh' && detect_container_runtime && echo \"\$CONTAINER_RT \${KIND_EXPERIMENTAL_PROVIDER:-}\"" 2>&1
}
check() { [ "$2" = "$3" ] && pass "$1" || fail "$1: got '$3', want '$2'"; }

check "docker wins when both are installed" "docker " "$(probe "$tmp/both")"
check "podman when docker is absent, with the kind provider" "podman podman" "$(probe "$tmp/podman")"
check "MICOLD_CONTAINER_RUNTIME overrides" "podman podman" "$(probe "$tmp/both" MICOLD_CONTAINER_RUNTIME=podman)"
check "an existing provider is kept" "podman docker" "$(probe "$tmp/podman" KIND_EXPERIMENTAL_PROVIDER=docker)"
out="$(probe "$tmp/none")"
case "$out" in *"no container runtime"*) pass "none installed is refused" ;; *) fail "none installed: $out" ;; esac
out="$(probe "$tmp/podman" MICOLD_CONTAINER_RUNTIME=docker)"
case "$out" in *"docker is not installed"*) pass "an uninstalled override is refused by name" ;; *) fail "override: $out" ;; esac

# kind_load_image: <rt> save -o <tar> <image>, then kind load image-archive, tar removed.
: >"$tmp/log"
env -i PATH="$tmp/podman:$tmp/base" STUB_LOG="$tmp/log" TMPDIR="$tmp" bash -c \
  ". '$ROOT/scripts/container-runtime.sh' && detect_container_runtime && kind_load_image c img:1"
if grep -q "^podman tag img:1 docker.io/library/img:1" "$tmp/log" \
  && grep -q "^podman save -o $tmp/micold-image-.*\.tar docker.io/library/img:1" "$tmp/log" \
  && grep -q "^kind load image-archive --name c $tmp/micold-image-.*\.tar" "$tmp/log" \
  && ! ls "$tmp"/micold-image-* >/dev/null 2>&1; then pass "image saved with the runtime, loaded from the archive, tar removed"
else fail "load: $(cat "$tmp/log")"; fi

# A failing kind load under set -e still removes the tar.
printf '#!/usr/bin/env bash\nexit 1\n' >"$tmp/kindfail"; mkdir "$tmp/kf"; cp "$tmp/kindfail" "$tmp/kf/kind"; chmod +x "$tmp/kf/kind"
env -i PATH="$tmp/kf:$tmp/podman:$tmp/base" STUB_LOG="$tmp/log" TMPDIR="$tmp" bash -c \
  "set -euo pipefail; . '$ROOT/scripts/container-runtime.sh'; detect_container_runtime; kind_load_image c img:1" && rc=0 || rc=$?
if [ "$rc" -ne 0 ] && ! ls "$tmp"/micold-image-* >/dev/null 2>&1; then pass "failed load removes the tar"
else fail "failed load: rc=$rc, $(ls "$tmp")"; fi

exit "$failures"
