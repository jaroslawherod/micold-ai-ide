#!/usr/bin/env bash
# Asserts CI's "Test (desktop notification backends)" step fails when it runs no test (issue #572).
#
# The macOS and Windows notification backends are tested only by that step. It selected its tests
# by the substring `desktop_notify`, and `cargo test` passes on 0 tests: a renamed module would have
# left the backends untested with the step still green. The step's own `run:` block is taken from
# `ci.yml` and run here with a stand-in `cargo`, so what is checked is what CI runs.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

CI=.github/workflows/ci.yml
STEP='Test (desktop notification backends)'
failures=0

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  [ $# -gt 1 ] && printf '      %s\n' "$2"
  failures=$((failures + 1))
}

# The step's `run:` script, a one-line `run:` or a `run: |` block, without its indentation.
step_run() {
  awk -v step="- name: $STEP" '
    index($0, step) { inside = 1; next }
    inside && /^      - name:/ { exit }
    inside && /^        run: \|/ { block = 1; next }
    inside && !block && /^        run: / { sub(/^        run: /, ""); print; exit }
    inside && block {
      if ($0 ~ /^          / || $0 ~ /^$/) { sub(/^          /, ""); print } else { exit }
    }
  ' "$CI"
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin" "$work/run"
cat >"$work/bin/cargo" <<'STANDIN'
#!/usr/bin/env bash
printf '%s\n' "$*" >"$WORK/args"
printf '%s\n' "$STANDIN_OUTPUT"
exit "${STANDIN_EXIT:-0}"
STANDIN
chmod +x "$work/bin/cargo"

script="$(step_run)"

# Run the step as GitHub's `shell: bash` does (`bash -eo pipefail`) with the stand-in answering
# `$1` and exiting `$2`. Prints the step's exit code.
run_step() {
  (
    cd "$work/run"
    PATH="$work/bin:$PATH" WORK="$work" STANDIN_OUTPUT="$1" STANDIN_EXIT="$2" \
      bash -eo pipefail -c "$script" >/dev/null 2>&1
  ) && echo 0 || echo $?
}

if grep -A2 -F -- "- name: $STEP" "$CI" | grep -q "if: runner.os != 'Linux'"; then
  pass "the step runs on the macOS and Windows legs"
else
  fail "the step runs on the macOS and Windows legs" "no \`if: runner.os != 'Linux'\` under it"
fi

if grep -A3 -F -- "- name: $STEP" "$CI" | grep -q "shell: bash"; then
  pass "the step runs under bash on every leg"
else
  fail "the step runs under bash on every leg" "no \`shell: bash\`: Windows would run it in pwsh"
fi

if [ "$(run_step 'test result: ok. 3 passed; 0 failed; 0 ignored' 0)" = 0 ]; then
  pass "a run that passes tests passes the step"
else
  fail "a run that passes tests passes the step"
fi

if [ "$(run_step 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 42 filtered out' 0)" != 0 ]; then
  pass "a run that selects no test fails the step"
else
  fail "a run that selects no test fails the step" "\`cargo test\` passes on 0 tests"
fi

if [ "$(run_step 'test result: FAILED. 2 passed; 1 failed' 101)" != 0 ]; then
  pass "a run with a failing test fails the step"
else
  fail "a run with a failing test fails the step"
fi

run_step 'test result: ok. 3 passed' 0 >/dev/null
if grep -q -- '--bin micold-ai-ide shell::desktop_notify::' "$work/args" 2>/dev/null; then
  pass "the step selects the backends by their module path"
else
  fail "the step selects the backends by their module path" "cargo was run with: $(cat "$work/args" 2>/dev/null)"
fi

[ "$failures" -eq 0 ] || { printf '%d failure(s)\n' "$failures"; exit 1; }
