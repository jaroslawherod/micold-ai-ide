# Sourced by scripts/local-stack-kind.sh and scripts/e2e-kind.sh (#2045): one container runtime for
# building images and loading them into kind. MICOLD_CONTAINER_RUNTIME names it (docker or podman);
# otherwise docker when installed, else podman. Podman also exports KIND_EXPERIMENTAL_PROVIDER=podman
# unless the caller set it, so kind uses the same runtime.
#
#   detect_container_runtime   sets CONTAINER_RT; returns 1 and prints why on stderr when none found
#   kind_load_image <cluster> <image>   `<rt> save` to a tar, then `kind load image-archive`
#                                       (`kind load docker-image` shells out to `docker save`)

detect_container_runtime() {
  CONTAINER_RT="${MICOLD_CONTAINER_RUNTIME:-}"
  if [ -z "$CONTAINER_RT" ]; then
    if command -v docker >/dev/null 2>&1; then CONTAINER_RT=docker
    elif command -v podman >/dev/null 2>&1; then CONTAINER_RT=podman
    else
      echo "no container runtime: install docker or podman (or set MICOLD_CONTAINER_RUNTIME)" >&2
      return 1
    fi
  fi
  command -v "$CONTAINER_RT" >/dev/null 2>&1 || {
    echo "container runtime $CONTAINER_RT is not installed (MICOLD_CONTAINER_RUNTIME)" >&2
    return 1
  }
  if [ "$CONTAINER_RT" = podman ]; then
    export KIND_EXPERIMENTAL_PROVIDER="${KIND_EXPERIMENTAL_PROVIDER:-podman}"
  fi
}

kind_load_image() {
  local cluster="$1" image="$2" tar rc
  tar="$(mktemp "${TMPDIR:-/tmp}/micold-image-XXXXXX.tar")" || return 1
  "$CONTAINER_RT" save -o "$tar" "$image" && kind load image-archive --name "$cluster" "$tar"
  rc=$?
  rm -f "$tar"
  return $rc
}
