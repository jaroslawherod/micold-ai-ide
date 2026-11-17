#!/usr/bin/env bash
# Dockerfile.web (#1732 M5, FR-022): the web client's static export, built for the address given in
# the EXPO_PUBLIC_ORCHESTRATOR_URL build arg, served on 8731 as a non-root user. The static checks
# run everywhere; with a working docker (MICOLD_DOCKER_TESTS=1) the image is built and must serve
# index.html carrying the URL.

set -uo pipefail
ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
F="$ROOT/Dockerfile.web"
failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }

[ -f "$F" ] || { fail "no Dockerfile.web"; exit 1; }
grep -q '^ARG EXPO_PUBLIC_ORCHESTRATOR_URL=ws://127.0.0.1:8730/api/ws$' "$F" && pass "the URL build arg defaults to the local stack" || fail "no URL build arg with the default"
grep -q 'EXPO_PUBLIC_ORCHESTRATOR_URL="\$EXPO_PUBLIC_ORCHESTRATOR_URL" pnpm --filter @micold/client web:export' "$F" && pass "the export is built with the arg" || fail "export does not take the arg"
grep -q '^EXPOSE 8731$' "$F" && pass "exposes 8731" || fail "no EXPOSE 8731"
grep -q '^USER 65532:65532$' "$F" && pass "runs as 65532" || fail "not USER 65532:65532"
n="$(grep -c '^FROM .*@sha256:[0-9a-f]\{64\}' "$F")"; t="$(grep -c '^FROM ' "$F")"
[ "$n" = "$t" ] && [ "$t" -ge 2 ] && pass "every base is digest-pinned" || fail "unpinned base ($n of $t)"
grep -q 'SERVE_HOST=0.0.0.0' "$F" && pass "the server listens on every interface in the pod" || fail "SERVE_HOST not set"

if [ "${MICOLD_DOCKER_TESTS:-0}" = 1 ]; then
  tag=micold-web:test url=ws://127.0.0.1:9999/api/ws
  docker build -q -f "$F" --build-arg EXPO_PUBLIC_ORCHESTRATOR_URL=$url -t $tag "$ROOT" >/dev/null || fail "docker build"
  id="$(docker run -d --read-only --tmpfs /tmp -p 127.0.0.1::8731 $tag)"
  port="$(docker port "$id" 8731 | head -1 | sed 's/.*://')"
  sleep 2
  curl -fs "http://127.0.0.1:$port/" | grep -q html && pass "the image serves index.html on 8731" || fail "nothing served"
  docker run --rm --entrypoint sh $tag -c "grep -rlq 127.0.0.1:9999 /srv/web" && pass "the export carries the URL" || fail "URL not in the export"
  docker rm -f "$id" >/dev/null
else
  echo "skip  image build (set MICOLD_DOCKER_TESTS=1 with docker)"
fi
exit "$failures"
