#!/usr/bin/env bash
# Drives `scripts/classify-change.sh` over crafted diffs (feature 023).
#
# Every case builds its own throwaway repository in a temp dir and deletes it afterwards, so the
# suite touches nothing real and cases cannot leak into one another. The `.gitattributes` under
# test is copied from this repository, so a change to the declaration is exercised here too.
#
# The contract these cases come from: specs/023-docs-only-ci-skip/contracts/classify-change.md.
#
# The one invariant worth stating out loud: **every failure path must land on `docs_only=false`.**
# There is no input for which "something went wrong" means "skip the build".

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
CLASSIFY="$ROOT/scripts/classify-change.sh"
ATTRS="$ROOT/.gitattributes"

failures=0
cases=0

# Build a fresh repository and echo its path.
new_repo() {
  local dir
  dir="$(mktemp -d)"
  git -C "$dir" init -q
  git -C "$dir" config user.email t@example.com
  git -C "$dir" config user.name t
  git -C "$dir" config commit.gpgsign false
  cp "$ATTRS" "$dir/.gitattributes"
  mkdir -p "$dir/docs" "$dir/specs" "$dir/crates/micold-core/src" "$dir/.github/workflows"
  printf 'seed\n' > "$dir/seed.txt"
  git -C "$dir" add -A
  git -C "$dir" commit -qm seed
  echo "$dir"
}

commit_in() {
  local dir="$1" msg="$2"
  git -C "$dir" add -A
  git -C "$dir" commit -qm "$msg"
}

# run <name> <want_docs_only> <want_reason_substring|-> <dir> <base> <head> [env assignments...]
run() {
  local name="$1" want="$2" want_reason="$3" dir="$4" base="$5" head="$6"
  shift 6
  cases=$((cases + 1))

  local out got_docs got_reason
  out="$(cd "$dir" && env "$@" "$CLASSIFY" "$base" "$head" 2>/dev/null)"
  got_docs="$(printf '%s\n' "$out" | sed -n 's/^docs_only=//p')"
  got_reason="$(printf '%s\n' "$out" | sed -n 's/^reason=//p')"

  if [ "$got_docs" != "$want" ]; then
    printf 'FAIL  %-46s want docs_only=%-5s got=%s\n' "$name" "$want" "${got_docs:-<none>}"
    failures=$((failures + 1))
    return
  fi
  if [ "$want_reason" != "-" ] && ! printf '%s' "$got_reason" | grep -qF -- "$want_reason"; then
    printf 'FAIL  %-46s want reason ~ %-22s got=%s\n' "$name" "$want_reason" "${got_reason:-<none>}"
    failures=$((failures + 1))
    return
  fi
  printf 'ok    %-46s docs_only=%-5s %s\n' "$name" "$got_docs" "$got_reason"
}

# run_pkg <name> <want_packaging> <dir> <base> <head> [env assignments...]
# The second output (#952): does the change touch a path declared `micold-packaging`? Unlike
# `docs_only`, every failure path must land on `true` here too -- a lost verdict runs the slow job.
run_pkg() {
  local name="$1" want="$2" dir="$3" base="$4" head="$5"
  shift 5
  cases=$((cases + 1))

  local got
  got="$(cd "$dir" && env "$@" "$CLASSIFY" "$base" "$head" 2>/dev/null | sed -n 's/^packaging=//p')"
  if [ "$got" != "$want" ]; then
    printf 'FAIL  %-46s want packaging=%-5s got=%s\n' "$name" "$want" "${got:-<none>}"
    failures=$((failures + 1))
    return
  fi
  printf 'ok    %-46s packaging=%s\n' "$name" "$got"
}

# --- documentation-only ------------------------------------------------------------------------

d="$(new_repo)"
printf 'a\n' > "$d/docs/page.md"; commit_in "$d" docs
run "only docs/" true "-" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
mkdir -p "$d/specs/001-x"; printf 'a\n' > "$d/specs/001-x/spec.md"; commit_in "$d" specs
run "only specs/" true "-" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'a\n' > "$d/README.md"; commit_in "$d" readme
run "README.md alone" true "-" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'a\n' > "$d/docs/gone.md"; commit_in "$d" add
git -C "$d" rm -q "docs/gone.md"; commit_in "$d" delete
run "deleted documentation file" true "-" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'a\n' > "$d/docs/a page with spaces.md"; commit_in "$d" spaced
run "documentation path containing spaces" true "-" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
run "empty diff (base == head)" true "no files changed" "$d" HEAD HEAD
rm -rf "$d"

# --- code-affecting ----------------------------------------------------------------------------

d="$(new_repo)"
# The changelog is include_str!'d into the binary, so it is a build input, not documentation.
printf 'a\n' > "$d/CHANGELOG.md"; commit_in "$d" changelog
run "CHANGELOG.md alone" false "-" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'fn a() {}\n' > "$d/crates/micold-core/src/a.rs"
for i in $(seq 1 20); do printf 'x\n' > "$d/docs/p$i.md"; done
commit_in "$d" mixed
run "one .rs plus twenty documentation files" false "-" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'x\n' > "$d/docs/first.md"; commit_in "$d" docs-first
printf 'fn a() {}\n' > "$d/crates/micold-core/src/a.rs"; commit_in "$d" code-second
run "documentation commit then code commit" false "-" "$d" HEAD~2 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'fn a() {}\n' > "$d/crates/micold-core/src/a.rs"; commit_in "$d" code-first
printf 'x\n' > "$d/docs/second.md"; commit_in "$d" docs-second
run "code commit then documentation commit" false "-" "$d" HEAD~2 HEAD
rm -rf "$d"

d="$(new_repo)"
printf '# just a comment\n' > "$d/.github/workflows/ci.yml"; commit_in "$d" wf
run "comment-only change to a workflow" false "-" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'fn a() {}\n' > "$d/crates/micold-core/src/a.rs"; commit_in "$d" add-src
git -C "$d" rm -q "crates/micold-core/src/a.rs"; commit_in "$d" del-src
run "deleted source file" false "-" "$d" HEAD~1 HEAD
rm -rf "$d"

# --- failure paths all land on code-affecting ---------------------------------------------------

d="$(new_repo)"
printf 'a\n' > "$d/docs/page.md"; commit_in "$d" docs
run "unresolvable base ref" false "base ref unavailable" "$d" origin/nope HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'a\n' > "$d/docs/page.md"; commit_in "$d" docs
run "all-zero base (new branch push)" false "base ref unavailable" "$d" \
  0000000000000000000000000000000000000000 HEAD
rm -rf "$d"

# --- escape hatch -------------------------------------------------------------------------------

d="$(new_repo)"
printf 'a\n' > "$d/docs/page.md"; commit_in "$d" docs
run "FORCE_FULL_CI over a documentation-only diff" false "forced by full-ci label" \
  "$d" HEAD~1 HEAD FORCE_FULL_CI=1
rm -rf "$d"

d="$(new_repo)"
printf 'a\n' > "$d/docs/page.md"; commit_in "$d" docs
run "FORCE_FULL_CI=false does not force" true "-" "$d" HEAD~1 HEAD FORCE_FULL_CI=false
rm -rf "$d"

# --- packaging paths (#952) ---------------------------------------------------------------------

pkg_case() { # <name> <want> <path>
  local d
  d="$(new_repo)"
  mkdir -p "$d/$(dirname "$3")"; printf 'a\n' > "$d/$3"; commit_in "$d" pkg
  run_pkg "$1" "$2" "$d" HEAD~1 HEAD
  rm -rf "$d"
}
pkg_case "packaging/windows file"           true  packaging/windows/installer.iss
pkg_case "windows-installer.sh"             true  scripts/windows-installer.sh
pkg_case "windows-install-smoke.sh"         true  scripts/windows-install-smoke.sh
pkg_case "workspace Cargo.toml"             true  Cargo.toml
pkg_case "Cargo.lock"                       true  Cargo.lock
pkg_case "release.yml"                      true  .github/workflows/release.yml
pkg_case "installer icon"                   true  assets/icon/icon.ico
pkg_case "LICENSE"                          true  LICENSE
pkg_case "remote-daemon-bundle.sh"          true  scripts/remote-daemon-bundle.sh
pkg_case "build-lock.sh"                    true  scripts/build-lock.sh
pkg_case "rust-toolchain.toml"              true  rust-toolchain.toml
pkg_case ".cargo/config.toml"               true  .cargo/config.toml
pkg_case "Rust source only"                 false crates/micold-core/src/a.rs
pkg_case "crate Cargo.toml only"            false crates/micold-core/Cargo.toml
pkg_case "documentation only"               false docs/page.md
pkg_case "ci.yml only"                      false .github/workflows/ci.yml

d="$(new_repo)"
mkdir -p "$d/packaging"; printf 'a\n' > "$d/packaging/x"; printf 'fn a() {}\n' > "$d/crates/micold-core/src/a.rs"
commit_in "$d" mixed
run_pkg "packaging among code" true "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'a\n' > "$d/docs/page.md"; commit_in "$d" docs
run_pkg "unresolvable base ref" true "$d" origin/nope HEAD
run_pkg "all-zero base" true "$d" 0000000000000000000000000000000000000000 HEAD
run_pkg "FORCE_FULL_CI over docs" true "$d" HEAD~1 HEAD FORCE_FULL_CI=1
run_pkg "FORCE_FULL_CI=false over docs" false "$d" HEAD~1 HEAD FORCE_FULL_CI=false
rm -rf "$d"

d="$(new_repo)"
run_pkg "empty diff" false "$d" HEAD HEAD
rm -rf "$d"

echo
if [ "$failures" -ne 0 ]; then
  echo "classify-change: $failures of $cases case(s) failed"
  exit 1
fi
echo "classify-change: all $cases cases passed"
