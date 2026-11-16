#!/usr/bin/env bash
# Drives scripts/install-kube-tools.sh (#1981, FR-005, FR-009) with local file URLs: no network.
#
# A download whose sha256 differs from the pin must abort before anything is installed, and every
# pin must be a full sha256. The real downloads are proven by the `kube-chart` CI job, which runs
# the script against the upstream URLs.

set -uo pipefail

ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/install-kube-tools.sh"

failures=0
cases=0
ok() { cases=$((cases + 1)); printf 'ok    %s\n' "$1"; }
fail() { cases=$((cases + 1)); failures=$((failures + 1)); printf 'FAIL  %s\n' "$1"; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# A tarball shaped like helm's, whose bytes cannot match the pin.
mkdir -p "$work/src/linux-amd64" "$work/dest"
printf '#!/bin/sh\necho fake helm\n' >"$work/src/linux-amd64/helm"
chmod +x "$work/src/linux-amd64/helm"
tar -C "$work/src" -czf "$work/helm.tar.gz" linux-amd64

out="$(KUBE_TOOLS_DEST="$work/dest" KUBE_TOOLS_URL_HELM="file://$work/helm.tar.gz" \
  "$SCRIPT" helm 2>&1)"
rc=$?
if [ "$rc" -ne 0 ]; then ok "checksum mismatch exits non-zero"; else fail "checksum mismatch exits non-zero (exit 0)"; fi
if [ ! -e "$work/dest/helm" ]; then ok "nothing installed on a mismatch"; else fail "nothing installed on a mismatch"; fi
if grep -q 'helm' <<<"$out" && grep -qi 'sha256' <<<"$out"; then ok "mismatch message names the tool and sha256"; else fail "mismatch message names the tool and sha256 (got: $out)"; fi

out="$(KUBE_TOOLS_DEST="$work/dest" KUBE_TOOLS_URL_KUBECONFORM="file://$work/missing.tar.gz" \
  "$SCRIPT" kubeconform 2>&1)"
rc=$?
if [ "$rc" -ne 0 ] && [ ! -e "$work/dest/kubeconform" ]; then ok "failed download exits non-zero, installs nothing"; else fail "failed download exits non-zero, installs nothing (exit $rc)"; fi

out="$(KUBE_TOOLS_DEST="$work/dest" "$SCRIPT" nosuchtool 2>&1)"
rc=$?
if [ "$rc" -eq 2 ] && grep -q 'nosuchtool' <<<"$out"; then ok "unknown tool is a usage error naming it"; else fail "unknown tool is a usage error naming it (exit $rc: $out)"; fi

out="$("$SCRIPT" --print-pins 2>&1)"
rc=$?
for tool in helm kubeconform; do
  line="$(grep "^$tool " <<<"$out")"
  if [ "$rc" -eq 0 ] && grep -qE "^$tool v[0-9]+\.[0-9]+\.[0-9]+ [0-9a-f]{64} https://" <<<"$line"; then
    ok "$tool has a version, a full sha256 and an https URL"
  else
    fail "$tool has a version, a full sha256 and an https URL (got: '$line')"
  fi
done

echo
if [ "$failures" -ne 0 ]; then
  echo "install-kube-tools: $failures of $cases case(s) failed"
  exit 1
fi
echo "install-kube-tools: all $cases cases passed"
