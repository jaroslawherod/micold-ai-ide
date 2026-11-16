#!/usr/bin/env bash
# Install the Kubernetes tools the chart and kind jobs use, at pinned versions, each download
# checked against its pinned sha256 before anything is installed (#1981, FR-005, FR-009).
#
#   install-kube-tools.sh <tool>...       tools: helm kubeconform
#   install-kube-tools.sh --print-pins    one line per tool: <tool> <version> <sha256> <url>
#
# KUBE_TOOLS_DEST is the install directory (default $HOME/.local/bin). KUBE_TOOLS_URL_<TOOL>
# (upper case) replaces a tool's download URL, for the shell test's local file; the pinned sha256
# still applies, so it cannot install anything but the pinned bytes. Linux amd64 only.
#
# The versions and sha256 values come from the upstream release checksum files:
#   helm        https://get.helm.sh/helm-<version>-linux-amd64.tar.gz.sha256sum
#   kubeconform https://github.com/yannh/kubeconform/releases/download/<version>/CHECKSUMS
# M3 (#1984) adds kind and kubectl here, moving their pins out of ci.yml's kube-cluster job.

set -euo pipefail

HELM_VERSION=v3.19.0
HELM_SHA256=a7f81ce08007091b86d8bd696eb4d86b8d0f2e1b9f6c714be62f82f96a594496
KUBECONFORM_VERSION=v0.7.0
KUBECONFORM_SHA256=c31518ddd122663b3f3aa874cfe8178cb0988de944f29c74a0b9260920d115d3

tools=(helm kubeconform)

version_of() { case "$1" in helm) echo "$HELM_VERSION" ;; kubeconform) echo "$KUBECONFORM_VERSION" ;; esac; }
sha_of() { case "$1" in helm) echo "$HELM_SHA256" ;; kubeconform) echo "$KUBECONFORM_SHA256" ;; esac; }
url_of() {
  case "$1" in
    helm) echo "https://get.helm.sh/helm-${HELM_VERSION}-linux-amd64.tar.gz" ;;
    kubeconform) echo "https://github.com/yannh/kubeconform/releases/download/${KUBECONFORM_VERSION}/kubeconform-linux-amd64.tar.gz" ;;
  esac
}
# The path of the binary inside the tarball.
member_of() { case "$1" in helm) echo linux-amd64/helm ;; kubeconform) echo kubeconform ;; esac; }

known() { local t; for t in "${tools[@]}"; do [ "$t" = "$1" ] && return 0; done; return 1; }

usage() {
  echo "usage: $0 <tool>... | --print-pins   (tools: ${tools[*]})" >&2
  exit 2
}

[ "$#" -gt 0 ] || usage
if [ "$1" = --print-pins ]; then
  for t in "${tools[@]}"; do echo "$t $(version_of "$t") $(sha_of "$t") $(url_of "$t")"; done
  exit 0
fi
for t in "$@"; do known "$t" || { echo "install-kube-tools: unknown tool '$t' (known: ${tools[*]})" >&2; exit 2; }; done

if [ "$(uname -s)-$(uname -m)" != Linux-x86_64 ]; then
  echo "install-kube-tools: only Linux amd64 is pinned, this is $(uname -s)-$(uname -m)" >&2
  exit 1
fi

dest="${KUBE_TOOLS_DEST:-$HOME/.local/bin}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

for t in "$@"; do
  override="KUBE_TOOLS_URL_$(tr '[:lower:]' '[:upper:]' <<<"$t")"
  url="${!override:-$(url_of "$t")}"
  want="$(sha_of "$t")"
  echo "install-kube-tools: $t $(version_of "$t") from $url"
  if ! curl -sSLfo "$work/$t.tar.gz" "$url"; then
    echo "install-kube-tools: $t: download failed: $url" >&2
    exit 1
  fi
  got="$(sha256sum "$work/$t.tar.gz" | cut -d' ' -f1)"
  if [ "$got" != "$want" ]; then
    echo "install-kube-tools: $t: sha256 mismatch: want $want, got $got; nothing installed" >&2
    exit 1
  fi
  tar -xzf "$work/$t.tar.gz" -C "$work" "$(member_of "$t")"
  mkdir -p "$dest"
  install -m 0755 "$work/$(member_of "$t")" "$dest/$t"
  echo "install-kube-tools: $t installed in $dest"
done
