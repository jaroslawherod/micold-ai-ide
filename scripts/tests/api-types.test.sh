#!/usr/bin/env bash
# The client API drift check (#1732, SC-004): `scripts/api-types.sh --check` passes on the checked-in
# TypeScript and fails, naming the file and the regeneration command, on an edited or a missing
# copy. The Rust side (schema and FR-012 vectors) is checked by crates/micold-core/tests/client_api_*.
# Needs `pnpm install` (json2ts); without it this test skips, as in CI's Rust lint job, and CI's web
# client job runs it.

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

if [ ! -x packages/protocol/node_modules/.bin/json2ts ]; then
	echo "skip  json2ts is not installed (pnpm install)"
	exit 0
fi

committed=packages/protocol/src/generated/client-api.ts
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

scripts/api-types.sh --check --ts-only >/dev/null
echo "ok    the checked-in TypeScript matches the schema"

cp "$committed" "$tmp/edited.ts"
echo "// hand edit" >>"$tmp/edited.ts"
for file in "$tmp/edited.ts" "$tmp/missing.ts"; do
	if out="$(API_TYPES_TS="$file" scripts/api-types.sh --check --ts-only)"; then
		echo "FAIL  $file passed the drift check"
		exit 1
	fi
	case "$out" in
	*"DRIFT: $file"*"scripts/api-types.sh"*) echo "ok    $(basename "$file") fails, naming the file and the command" ;;
	*) echo "FAIL  message lacks file or command: $out"; exit 1 ;;
	esac
done
