#!/usr/bin/env bash
# Drives `scripts/check-user-guide-updated.sh` over crafted diffs.
#
# Constitution Principle VII says a user-facing feature is not done until its documentation
# exists, and the Documentation gate says user-facing changes MUST update the user guide in the
# same pull request. Until now the docs *build* was mechanical and that sentence was not: whether
# the guide had actually been updated was left to review, which is the half that gets skipped.
#
# Every case builds its own throwaway repository in a temp dir and deletes it afterwards, so the
# suite touches nothing real and cases cannot leak into one another. The `.gitattributes` under
# test is copied from this repository, so a change to the declaration is exercised here too -- the
# declaration is the whole rule, and a script that hard-coded paths would drift from it.
#
# Three invariants worth stating out loud:
#
#   1. **Only the user guide satisfies the gate.** A developer doc is not a substitute: the reader
#      this principle protects is the person using the application, not the person building it.
#   2. **Features, not every edit.** The word in the principle is "feature". Firing on every edit
#      under the declared paths would have blocked ten of the sixty merges before this existed,
#      eight of them bug fixes -- and a label applied that often stops being read.
#   3. **Every failure path blocks.** There is no input for which "something went wrong" means
#      "the docs are fine" -- the whole point of moving this out of review is that it stops being
#      something anyone can forget. The `docs-not-needed` label is the one way past, and it is a
#      decision somebody signs rather than an accident.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
CHECK="$ROOT/scripts/check-user-guide-updated.sh"
ATTRS="$ROOT/.gitattributes"

failures=0
cases=0

# Build a fresh repository and echo its path. The directory set mirrors the real tree closely
# enough that every pattern in the declaration has something to match.
new_repo() {
  local dir
  dir="$(mktemp -d)"
  git -C "$dir" init -q
  git -C "$dir" config user.email t@example.com
  git -C "$dir" config user.name t
  git -C "$dir" config commit.gpgsign false
  cp "$ATTRS" "$dir/.gitattributes"
  mkdir -p "$dir/docs/user-guide" "$dir/docs/development" "$dir/specs" \
    "$dir/crates/micold-core/src" "$dir/crates/micold-daemon/src" \
    "$dir/crates/micold-client/src/ui/material" "$dir/crates/micold-client/src/ui/cdk" \
    "$dir/crates/micold-client/src/ui/settings" "$dir/crates/micold-client/src/features" \
    "$dir/crates/micold-client/src/shell"
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

# run <name> <want_exit> <want_output_substring|-> <dir> <base> <head> [env assignments...]
#
# The title defaults to a feature, because that is the case the gate exists for; the cases that
# care about the title pass their own.
run() {
  local name="$1" want="$2" want_text="$3" dir="$4" base="$5" head="$6"
  shift 6
  cases=$((cases + 1))

  local out got
  out="$(cd "$dir" && env CHANGE_TITLE='feat(001): a thing' "$@" "$CHECK" "$base" "$head" 2>&1)"
  got=$?

  if [ "$got" != "$want" ]; then
    printf 'FAIL  %-52s want exit=%s got=%s\n' "$name" "$want" "$got"
    printf '%s\n' "$out" | sed 's/^/        /'
    failures=$((failures + 1))
    return
  fi
  if [ "$want_text" != "-" ] && ! printf '%s' "$out" | grep -qF -- "$want_text"; then
    printf 'FAIL  %-52s want output ~ %s\n' "$name" "$want_text"
    printf '%s\n' "$out" | sed 's/^/        /'
    failures=$((failures + 1))
    return
  fi
  printf 'ok    %-52s exit=%s\n' "$name" "$got"
}

echo "== the declaration =="

# The gate is only as good as the set of paths it fires on, and that set lives in
# `.gitattributes`, not here. Pinning the verdicts means a careless pattern edit fails in this
# suite rather than by quietly letting an undocumented screen through.
expect_attr() {
  local attr="$1" path="$2" want="$3" got
  cases=$((cases + 1))
  got="$(cd "$ROOT" && git check-attr "$attr" -- "$path" | sed "s/.*: $attr: //")"
  if [ "$got" != "$want" ]; then
    printf 'FAIL  %-52s %s want=%-12s got=%s\n' "$path" "$attr" "$want" "$got"
    failures=$((failures + 1))
  else
    printf 'ok    %-52s %s=%s\n' "$path" "$attr" "$got"
  fi
}

# What a user sees: the screens, and the behaviours they trigger.
expect_attr micold-user-facing crates/micold-client/src/ui/sidebar.rs           set
expect_attr micold-user-facing crates/micold-client/src/ui/settings_view.rs     set
expect_attr micold-user-facing crates/micold-client/src/ui/settings/daemon.rs   set
expect_attr micold-user-facing crates/micold-client/src/features/session.rs     set
expect_attr micold-user-facing crates/micold-client/src/features/worktree.rs    set

# The component library underneath them is documented for developers instead (Principle VII is
# satisfied by docs/development/component-library.md, as feature 020 already established), so a
# token tweak or a ripple fix must not demand a user-guide edit. A gate that cried wolf on every
# refactor would be labelled away within a week and stop protecting anything.
expect_attr micold-user-facing crates/micold-client/src/ui/material/button.rs   unset
expect_attr micold-user-facing crates/micold-client/src/ui/cdk/picker.rs        unset

# Everything else is code until declared otherwise -- the same default the documentation set uses.
expect_attr micold-user-facing crates/micold-core/src/lib.rs                    unspecified
expect_attr micold-user-facing crates/micold-daemon/src/server.rs               unspecified
expect_attr micold-user-facing crates/micold-client/src/shell/persist.rs        unspecified

# The other half of the rule: what counts as having documented it.
expect_attr micold-user-guide  docs/user-guide/settings.md                      set
expect_attr micold-user-guide  docs/development/component-library.md            unspecified
expect_attr micold-user-guide  README.md                                        unspecified

echo
echo "== nothing user-facing was touched =="

d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-core/src/lib.rs"; commit_in "$d" core
run "a core-only change" 0 "no user-facing paths" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'a\n' > "$d/docs/user-guide/settings.md"; commit_in "$d" docs
run "a documentation-only change" 0 "no user-facing paths" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-client/src/ui/material/button.rs"; commit_in "$d" mat
run "the component library alone" 0 "no user-facing paths" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
run "no files changed" 0 "no files changed" "$d" HEAD HEAD
rm -rf "$d"

echo
echo "== a user-facing change without the guide =="

d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-client/src/ui/sidebar.rs"; commit_in "$d" ui
run "a screen changed, guide untouched" 1 "ui/sidebar.rs" "$d" HEAD~1 HEAD
rm -rf "$d"

d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-client/src/features/session.rs"; commit_in "$d" feat
run "a behaviour changed, guide untouched" 1 "features/session.rs" "$d" HEAD~1 HEAD
rm -rf "$d"

# A developer doc is not a substitute. This is the case that makes the gate mean what Principle VII
# says rather than "some markdown was edited".
d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-client/src/ui/sidebar.rs"
printf 'a\n' > "$d/docs/development/architecture.md"; commit_in "$d" ui+devdoc
run "a developer doc does not satisfy it" 1 "ui/sidebar.rs" "$d" HEAD~1 HEAD
rm -rf "$d"

# Deleting a screen is a user-facing change too: check-attr is pure pattern matching, so a path
# that no longer exists still answers.
d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-client/src/ui/rename.rs"; commit_in "$d" add
git -C "$d" rm -q "crates/micold-client/src/ui/rename.rs"; commit_in "$d" remove
run "a deleted screen still counts" 1 "ui/rename.rs" "$d" HEAD~1 HEAD
rm -rf "$d"

echo
echo "== a user-facing change with the guide =="

d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-client/src/ui/sidebar.rs"
printf 'a\n' > "$d/docs/user-guide/worktrees-and-sessions.md"; commit_in "$d" ui+docs
run "a screen changed, guide updated" 0 "documented" "$d" HEAD~1 HEAD
rm -rf "$d"

# Three dots: what the change touches as a whole, regardless of which commit came last. Splitting
# the code and the prose across two commits is normal and must not matter.
d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-client/src/ui/toolbar.rs"; commit_in "$d" code
printf 'a\n' > "$d/docs/user-guide/settings.md"; commit_in "$d" prose
run "code and prose in separate commits" 0 "documented" "$d" HEAD~2 HEAD
rm -rf "$d"

echo
echo "== features, not every edit =="

# The word in Principle VII is "feature", and holding the gate to it is what makes it worth
# having. Over the sixty merges before this existed, firing on any edit under the declared paths
# would have blocked ten changes, eight of them bug fixes whose guide page says nothing different.
d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-client/src/ui/sidebar.rs"; commit_in "$d" ui
run "a fix is not a feature" 0 "not a feature" "$d" HEAD~1 HEAD \
  CHANGE_TITLE='fix(010): end a mutation the connection outlived (BUG-020)'
run "a refactor is not a feature" 0 "not a feature" "$d" HEAD~1 HEAD \
  CHANGE_TITLE='refactor(027): one home per setting'
run "a scoped feature is one" 1 "ui/sidebar.rs" "$d" HEAD~1 HEAD \
  CHANGE_TITLE='feat(027): give Settings a collapsible icon rail'
run "an unscoped feature is one" 1 "ui/sidebar.rs" "$d" HEAD~1 HEAD \
  CHANGE_TITLE='feat: give Settings a collapsible icon rail'
run "a breaking feature is one" 1 "ui/sidebar.rs" "$d" HEAD~1 HEAD \
  CHANGE_TITLE='feat(027)!: replace the rail'
# The prefix is a prefix. A title that merely contains the word is not a conventional-commit type,
# and a substring match here would fire on half the fix titles in this repository.
run "the word feature elsewhere in the title" 0 "not a feature" "$d" HEAD~1 HEAD \
  CHANGE_TITLE='fix(027): the feature flag was read twice'
run "a title with no type at all" 0 "not a feature" "$d" HEAD~1 HEAD \
  CHANGE_TITLE='update the sidebar'
# Only the caller can supply the title, and the caller always has one. An empty one means the
# wiring broke, which is a failure path like any other.
run "no title at all" 1 "title was not provided" "$d" HEAD~1 HEAD CHANGE_TITLE=
rm -rf "$d"

echo
echo "== the escape hatch =="

d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-client/src/ui/sidebar.rs"; commit_in "$d" ui
run "DOCS_NOT_NEEDED over a blocked diff" 0 "waived" "$d" HEAD~1 HEAD DOCS_NOT_NEEDED=true
rm -rf "$d"

d="$(new_repo)"
printf 'a\n' > "$d/crates/micold-client/src/ui/sidebar.rs"; commit_in "$d" ui
run "DOCS_NOT_NEEDED=false does not waive" 1 "ui/sidebar.rs" "$d" HEAD~1 HEAD DOCS_NOT_NEEDED=false
rm -rf "$d"

echo
echo "== every failure path blocks =="

d="$(new_repo)"
run "an all-zero base ref" 1 "base ref unavailable" \
  "$d" 0000000000000000000000000000000000000000 HEAD
rm -rf "$d"

d="$(new_repo)"
run "a base ref that does not exist" 1 "base ref unavailable" "$d" deadbeef HEAD
rm -rf "$d"

d="$(new_repo)"
run "a head ref that does not exist" 1 "head ref unavailable" "$d" HEAD deadbeef
rm -rf "$d"

# The waiver is checked before anything else, so a forced-through change cannot be blocked by a
# base ref that happens to be broken -- the mirror of how `classify-change.sh` treats `full-ci`.
d="$(new_repo)"
run "the waiver survives a broken base ref" 0 "waived" \
  "$d" 0000000000000000000000000000000000000000 HEAD DOCS_NOT_NEEDED=1
rm -rf "$d"

# Arity is its own exit code: "you called me wrong" is not "the docs are missing", and a caller
# that loses an argument must not read as a blocked change.
cases=$((cases + 1))
arity_out="$("$CHECK" 2>&1)"
arity_exit=$?
if [ "$arity_exit" = 2 ] && printf '%s' "$arity_out" | grep -qF usage; then
  printf 'ok    %-52s exit=2\n' "no arguments"
else
  printf 'FAIL  %-52s want exit=2 with usage, got exit=%s: %s\n' \
    "no arguments" "$arity_exit" "$arity_out"
  failures=$((failures + 1))
fi

echo
if [ "$failures" -ne 0 ]; then
  echo "check-user-guide-updated: $failures of $cases case(s) failed"
  exit 1
fi
echo "check-user-guide-updated: all $cases cases passed"
