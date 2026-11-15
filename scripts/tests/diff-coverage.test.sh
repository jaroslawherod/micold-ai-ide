#!/usr/bin/env bash
# Drives scripts/diff-coverage.py (#1931) over throwaway repositories and hand-written lcov files:
# the changed executable lines of in-scope files are checked against the lcov hits.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/diff-coverage.py"
REAL_CONF="$ROOT/scripts/guardrails-scope.conf"
failures=0
pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  [ $# -gt 1 ] && printf '      %s\n' "$2"
  failures=$((failures + 1))
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
repo="$work/repo"
mkdir -p "$repo"
git -C "$repo" init -q
git -C "$repo" config user.email t@example.com
git -C "$repo" config user.name t
git -C "$repo" config commit.gpgsign false

cat >"$work/scope.conf" <<'CONF'
[settings]
threshold = 80

[rust]
crates/micold-daemon/src/**
crates/micold-core/src/orchestrator/**

[web]
web/src/**
CONF

ten() { for i in 1 2 3 4 5 6 7 8 9 10; do echo "line $i"; done; }
mkdir -p "$repo/crates/micold-daemon/src" "$repo/crates/micold-core/src/orchestrator" \
  "$repo/crates/micold-client/src" "$repo/web/src"
for f in crates/micold-daemon/src/a.rs crates/micold-core/src/orchestrator/o.rs \
  crates/micold-core/src/other.rs crates/micold-client/src/c.rs web/src/w.ts; do ten >"$repo/$f"; done
git -C "$repo" add -A && git -C "$repo" commit -qm base
git -C "$repo" tag base
# the change: lines 3-7 of every file are rewritten (5 changed lines each)
for f in crates/micold-daemon/src/a.rs crates/micold-core/src/orchestrator/o.rs \
  crates/micold-core/src/other.rs crates/micold-client/src/c.rs web/src/w.ts; do
  sed -i '3,7s/$/ changed/' "$repo/$f"
done
git -C "$repo" add -A && git -C "$repo" commit -qm change

# lcov helper: record <file> <covered lines...> -- lines 3-7 are executable, hits 1 for the listed
# lines and 0 for the rest of 3-7.
lcov_for() { # <file> <hit lines, space separated>
  local file="$1" hits=" $2 " n
  echo "SF:$repo/$file"
  for n in 3 4 5 6 7; do
    case "$hits" in *" $n "*) echo "DA:$n,4" ;; *) echo "DA:$n,0" ;; esac
  done
  echo "end_of_record"
}

run() { (cd "$repo" && python3 "$SCRIPT" --config "$work/scope.conf" --base base "$@" 2>&1); }

{ lcov_for crates/micold-daemon/src/a.rs "3 4 5 6 7"; } >"$work/full.lcov"
out="$(run --lcov "$work/full.lcov")"; rc=$?
[ "$rc" -eq 0 ] && printf '%s' "$out" | grep -q '100' && pass "fully covered changed lines pass" \
  || fail "fully covered changed lines pass" "rc=$rc $out"

{ lcov_for crates/micold-daemon/src/a.rs "3 4"; } >"$work/low.lcov"
out="$(run --lcov "$work/low.lcov")"; rc=$?
[ "$rc" -eq 1 ] && printf '%s' "$out" | grep -q 'crates/micold-daemon/src/a.rs' \
  && printf '%s' "$out" | grep -q '5-7' && pass "40% covered fails and names the uncovered lines" \
  || fail "40% covered fails and names the uncovered lines" "rc=$rc $out"

out="$(run --lcov "$work/low.lcov" --threshold 40)"; rc=$?
[ "$rc" -eq 0 ] && pass "--threshold overrides the config" || fail "--threshold overrides the config" "rc=$rc $out"

{ lcov_for crates/micold-daemon/src/a.rs "3 4 5 6"; } >"$work/edge.lcov"
out="$(run --lcov "$work/edge.lcov")"; rc=$?
[ "$rc" -eq 0 ] && pass "exactly 80% meets the threshold" || fail "exactly 80% meets the threshold" "rc=$rc $out"

{ lcov_for crates/micold-client/src/c.rs ""; lcov_for crates/micold-core/src/other.rs ""; } >"$work/out.lcov"
out="$(run --lcov "$work/out.lcov")"; rc=$?
[ "$rc" -eq 0 ] && pass "uncovered lines outside the scope are ignored" \
  || fail "uncovered lines outside the scope are ignored" "rc=$rc $out"

{ lcov_for crates/micold-core/src/orchestrator/o.rs "3"; } >"$work/orch.lcov"
out="$(run --lcov "$work/orch.lcov")"; rc=$?
[ "$rc" -eq 1 ] && pass "an orchestrator file is in scope" || fail "an orchestrator file is in scope" "rc=$rc $out"

{ lcov_for web/src/w.ts "3"; } >"$work/web.lcov"
out="$(run --lcov "$work/web.lcov" --lang web)"; rc=$?
[ "$rc" -eq 1 ] && pass "--lang selects another language's glob list" || fail "--lang selects another language's glob list" "rc=$rc $out"
out="$(run --lcov "$work/web.lcov")"; rc=$?
[ "$rc" -eq 0 ] && pass "default language does not see web files" || fail "default language does not see web files" "rc=$rc $out"

printf 'SF:%s/crates/micold-daemon/src/a.rs\nDA:1,1\nend_of_record\n' "$repo" >"$work/none.lcov"
out="$(run --lcov "$work/none.lcov")"; rc=$?
[ "$rc" -eq 0 ] && pass "no changed executable line passes" || fail "no changed executable line passes" "rc=$rc $out"

out="$(run --lcov "$work/low.lcov" --summary "$work/summary.md")"
grep -q 'crates/micold-daemon/src/a.rs' "$work/summary.md" 2>/dev/null \
  && pass "--summary writes the markdown report" || fail "--summary writes the markdown report"

out="$(cd "$repo" && python3 "$SCRIPT" --config "$work/scope.conf" --print-globs rust 2>&1)"
[ "$out" = "$(printf 'crates/micold-daemon/src/**\ncrates/micold-core/src/orchestrator/**')" ] \
  && pass "--print-globs lists a language's globs" || fail "--print-globs lists a language's globs" "$out"

out="$(run --lcov "$work/missing.lcov")"; rc=$?
[ "$rc" -eq 2 ] && pass "a missing lcov file is a usage error, not a pass" || fail "a missing lcov file is a usage error, not a pass" "rc=$rc $out"

globs="$(cd "$ROOT" && python3 "$SCRIPT" --print-globs rust 2>&1)"
{ printf '%s' "$globs" | grep -qx 'crates/micold-daemon/src/\*\*' \
  && printf '%s' "$globs" | grep -qx 'crates/micold-core/src/orchestrator/\*\*' \
  && ! printf '%s' "$globs" | grep -q 'client\|material'; } \
  && pass "the checked-in scope covers daemon and orchestrator, not the desktop app" \
  || fail "the checked-in scope covers daemon and orchestrator, not the desktop app" "$globs"

[ "$failures" -eq 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "all passed"
