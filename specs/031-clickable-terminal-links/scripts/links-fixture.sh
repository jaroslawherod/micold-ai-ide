#!/usr/bin/env bash
# The quickstart §B.0 fixture for feature 031: prints one of each link the visual pass hovers and
# clicks, and creates the files its file links point at. Run it inside a terminal session.
set -euo pipefail

dir=/tmp/031-fixture
mkdir -p "$dir/folder"
printf 'The 031 fixture file.\n' >"$dir/readme.txt"
printf '#!/bin/sh\necho "run.sh ran: it should have been revealed, not run"\n' >"$dir/run.sh"
chmod +x "$dir/run.sh"

# An OSC 8 run: $1 is the declared URI, $2 the visible text.
osc8() {
	printf '\e]8;;%s\e\\%s\e]8;;\e\\' "$1" "$2"
}

printf 'See https://example.com/docs/page.html for details.\n'
printf '(https://en.wikipedia.org/wiki/Rust_(programming_language))\n'
printf 'mailto:team@example.com\n'
osc8 https://example.com/manual docs
printf '  '
osc8 https://example.com/other other
printf '\n'
printf 'file://%s%s/readme.txt\n' "$(hostname)" "$dir"
printf 'file://%s/run.sh\n' "$dir"
printf 'file://%s/folder\n' "$dir"
printf 'a line long enough that the terminal soft-wraps https://example.com/a/very/long/path/%s/end\n' \
	"$(printf 'segment-%02d/' $(seq 1 12) | sed 's#/$##')"
