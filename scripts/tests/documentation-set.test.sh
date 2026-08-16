#!/usr/bin/env bash
# Asserts the documentation-set declaration in `.gitattributes` classifies paths as intended.
#
# The declaration is the single source of truth for "which paths cannot affect what is built"
# (feature 023, FR-003). Two consumers read it through the same matcher, `git check-attr`:
# `scripts/classify-change.sh` (which paths did this change touch) and
# `crates/micold-core/tests/documentation_is_not_read.rs` (which paths may no test read). This
# test pins the verdicts so neither consumer has to guess, and so a careless pattern edit fails
# here rather than by silently skipping a build.
#
# `set` means documentation. Both `unset` and `unspecified` mean code -- the default is code, so a
# path stays code until someone declares otherwise.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

failures=0

expect() {
  local path="$1" want="$2" got
  # `--` guards paths that could look like options; check-attr is pure pattern matching, so a path
  # that no longer exists (a deleted file) still answers correctly.
  got="$(git check-attr micold-docs -- "$path" | sed 's/.*: micold-docs: //')"
  if [ "$got" != "$want" ]; then
    printf 'FAIL  %-46s want=%-12s got=%s\n' "$path" "$want" "$got"
    failures=$((failures + 1))
  else
    printf 'ok    %-46s %s\n' "$path" "$got"
  fi
}

echo "== documentation (skippable) =="
expect docs/user-guide/settings.md                set
expect docs/development/component-library.md      set
expect specs/023-docs-only-ci-skip/spec.md        set
expect specs/001-app-shell-about/spec.md          set
expect README.md                                  set
expect CLAUDE.md                                  set
expect LICENSE                                    set
expect dialog-list.png                            set
expect .claude/skills/visual-pass/SKILL.md        set

echo
echo "== deleted paths still classify (check-attr never touches the worktree) =="
expect docs/gone/removed-page.md                  set
expect specs/999-removed/spec.md                  set

echo
echo "== compiled into the binary, so NOT documentation =="
# crates/micold-core/src/metadata.rs does `include_str!("../../../CHANGELOG.md")`: the changelog is
# embedded so the app can show a "what's new" view offline. Changing it changes the built artifact,
# which is exactly what FR-004 excludes. The `-micold-docs` line must therefore come after `/*.md`.
expect CHANGELOG.md                               unset

echo
echo "== code (everything not declared) =="
expect crates/micold-core/src/lib.rs              unspecified
expect crates/micold-client/tests/showcase_glue.rs unspecified
expect .github/workflows/ci.yml                   unspecified
expect Cargo.toml                                 unspecified
expect Cargo.lock                                 unspecified
expect rust-toolchain.toml                        unspecified
expect scripts/build-lock.sh                      unspecified
expect scripts/classify-change.sh                 unspecified
expect assets/fonts/Roboto-Regular.ttf            unspecified
expect packaging/micold-ai-ide.desktop            unspecified
expect .gitattributes                             unspecified
# Markdown is documentation only at the repository root; a .md that is part of a crate is not.
expect crates/micold-core/README.md               unspecified
# Only `skills/` under `.claude`; the rest of that directory is the app's runtime state.
expect .claude/worktrees/feat-x/src/lib.rs        unspecified
expect .claude/settings.local.json                unspecified

echo
if [ "$failures" -ne 0 ]; then
  echo "documentation-set: $failures failure(s)"
  exit 1
fi
echo "documentation-set: all assertions passed"
