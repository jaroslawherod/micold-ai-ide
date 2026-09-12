#!/usr/bin/env bash
# Asserts `site/checks/media-references.sh` catches what it exists to catch (feature 028, T052).
#
# Three files have to agree for a screenshot to reach a reader: the page that asks for it
# (`<!-- media: id -->`), the manifest that declares it (`site/media.toml`), and the scene script
# that produces it (`site/capture/scenes/<scene>.sh`). Each pair drifts in its own way, and only
# one of the three failures is loud:
#
#   * a directive naming an id nobody declares -- `site/stage.sh` already refuses this, but it
#     refuses it at build time, on the release branch, after the tag. The point of checking it
#     before a merge is that the author is still there.
#   * a declared id no page references -- silent. The capture runs, the picture is produced, the
#     minutes are spent, and nothing shows it. This is the one that accumulates.
#   * a `scene` naming a script that is not there -- the capture fails, but only on a machine that
#     captures, which is not the machine the rename happened on.
#
# The fourth assertion is not about drift at all. `alt` is how a reader who cannot see the picture
# reads the page (FR-014), and an empty one is worse than none: it tells a screen reader the image
# is decorative, so the reader is never told there was anything there.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

CHECK=site/checks/media-references.sh
failures=0

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  [ $# -gt 1 ] && printf '      %s\n' "$2"
  failures=$((failures + 1))
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

run() {
  "$CHECK" "$@" >"$work/out" 2>&1
}

expect_pass() {
  local what="$1"
  shift
  if run "$@"; then pass "$what"; else fail "$what" "$(tail -8 "$work/out")"; fi
}

expect_fail() {
  local what="$1" needle="$2"
  shift 2
  if run "$@"; then
    fail "$what" "the check passed a set it must refuse"
  elif grep -qF -- "$needle" "$work/out"; then
    pass "$what"
  else
    fail "$what" "failed, but did not name \"$needle\": $(tail -8 "$work/out")"
  fi
}

# --- the fixtures ---------------------------------------------------------------------------------

mkdir -p "$work/scenes"
printf '#!/usr/bin/env bash\n' >"$work/scenes/main-window.sh"
printf '#!/usr/bin/env bash\n' >"$work/scenes/new-worktree.sh"

mkdir -p "$work/docs/user-guide"
cat >"$work/docs/README.md" <<'MD'
# The guide

<!-- media: main-window-light -->

The window above is the whole application.
MD
cat >"$work/docs/user-guide/worktrees.md" <<'MD'
# Worktrees

<!-- media: main-window-dark -->

<!-- media: new-worktree -->
MD

manifest() {
  cat >"$work/media.toml" <<'TOML'
[media.main-window-light]
kind    = "still"
scene   = "main-window"
scheme  = "light"
alt     = "The application window with a project open."
caption = "The main window."

[media.main-window-dark]
kind    = "still"
scene   = "main-window"
scheme  = "dark"
alt     = "The same window in the dark theme."

[media.new-worktree]
kind    = "clip"
scene   = "new-worktree"
scheme  = "light"
alt     = "Creating a worktree, from the button to the worktree appearing in the sidebar."
TOML
}

check() {
  run --docs "$work/docs" --manifest "$work/media.toml" --scenes "$work/scenes"
}

printf '== the manifest and the pages agree (FR-022, FR-011a) ==\n'
manifest
expect_pass "a set in which every directive is declared and every entry is referenced passes" \
  --docs "$work/docs" --manifest "$work/media.toml" --scenes "$work/scenes"

manifest
printf '\n<!-- media: settings-view-light -->\n' >>"$work/docs/user-guide/worktrees.md"
expect_fail "a directive naming an id the manifest does not declare fails" "settings-view-light" \
  --docs "$work/docs" --manifest "$work/media.toml" --scenes "$work/scenes"
# ...and it says which page asked for it, because that is the file the author has to open.
if ! check && grep -qF "user-guide/worktrees.md" "$work/out"; then
  pass "and it names the page the directive is on"
else
  fail "and it names the page the directive is on" "$(tail -8 "$work/out")"
fi
sed -i '/settings-view-light/d' "$work/docs/user-guide/worktrees.md"

manifest
cat >>"$work/media.toml" <<'TOML'

[media.settings-view-light]
kind    = "still"
scene   = "main-window"
scheme  = "light"
alt     = "The settings view."
TOML
expect_fail "a declared entry no page references fails" "settings-view-light" \
  --docs "$work/docs" --manifest "$work/media.toml" --scenes "$work/scenes"

printf '== the scenes exist (FR-011a) ==\n'
manifest
sed -i 's/^scene   = "new-worktree"$/scene   = "create-worktree"/' "$work/media.toml"
expect_fail "an entry naming a scene script that is not there fails" "create-worktree" \
  --docs "$work/docs" --manifest "$work/media.toml" --scenes "$work/scenes"

printf '== every entry describes itself (FR-014) ==\n'
manifest
sed -i 's/^alt     = "The same window in the dark theme."$/alt     = ""/' "$work/media.toml"
expect_fail "an empty alt fails" "main-window-dark" \
  --docs "$work/docs" --manifest "$work/media.toml" --scenes "$work/scenes"

manifest
sed -i '/^alt     = "The same window in the dark theme."$/d' "$work/media.toml"
expect_fail "a missing alt fails the same way" "main-window-dark" \
  --docs "$work/docs" --manifest "$work/media.toml" --scenes "$work/scenes"

manifest
sed -i 's/^alt     = "The same window in the dark theme."$/alt     = "   "/' "$work/media.toml"
expect_fail "an alt of nothing but spaces fails too" "main-window-dark" \
  --docs "$work/docs" --manifest "$work/media.toml" --scenes "$work/scenes"

printf '== what is allowed ==\n'
manifest
printf '\n<!-- media: main-window-light -->\n' >>"$work/docs/user-guide/worktrees.md"
expect_pass "one entry referenced from two pages passes -- a picture may be shown twice" \
  --docs "$work/docs" --manifest "$work/media.toml" --scenes "$work/scenes"
sed -i '$d' "$work/docs/user-guide/worktrees.md"

manifest
sed -i '/^caption = "The main window."$/d' "$work/media.toml"
expect_pass "a missing caption passes -- captions are optional, alt is not" \
  --docs "$work/docs" --manifest "$work/media.toml" --scenes "$work/scenes"

printf '== the repository itself ==\n'
expect_pass "the repository's own docs/, media.toml and scenes pass"

printf '\n'
if [ "$failures" -eq 0 ]; then
  printf 'the media-reference checks: all assertions hold\n'
else
  printf '%d assertion(s) failed\n' "$failures"
fi
exit "$([ "$failures" -eq 0 ] && echo 0 || echo 1)"
