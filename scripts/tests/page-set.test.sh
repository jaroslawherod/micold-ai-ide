#!/usr/bin/env bash
# Asserts `site/checks/page-set.sh` catches what it exists to catch (feature 028, T051).
#
# The check holds two things together that drift apart quietly. The first is the page set: `docs/`
# is the source and `docs/SUMMARY.md` is the table of contents, and the renderer reads the second.
# A page nobody listed is a page nobody can reach -- it is not a broken link, so the link check
# passes, and the page is simply absent from the published site. A listed page that was renamed is
# the mirror image: the render fails, or worse, renders a stub.
#
# The second is the theme. `site/theme/css/site.css` is meant to draw the site out of the design
# tokens the application itself is drawn from, so that changing a token changes both. A single
# literal -- one colour, one type size, one corner radius -- looks harmless in review and is exactly
# how that stops being true: it survives the next token change, and the site slowly stops matching
# the product it documents. So the check reads values, not properties, and it reads only what the
# token set actually has an opinion about (SC-014): colours, type sizes, corner radii, elevations
# and motion durations. Widths and spacing are not on that list and must not be flagged.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

CHECK=site/checks/page-set.sh
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
    fail "$what" "the check passed a tree it must refuse"
  elif grep -qF -- "$needle" "$work/out"; then
    pass "$what"
  else
    fail "$what" "failed, but did not name \"$needle\": $(tail -8 "$work/out")"
  fi
}

# --- the fixtures ---------------------------------------------------------------------------------
#
# Shaped like `docs/`: a handful of pages at the root, a subdirectory of them, and a `SUMMARY.md`
# that lists every one. Small enough to read, structured enough to break in each of the ways below.

make_docs() {
  local dir="$1"
  mkdir -p "$dir/user-guide"
  printf '# The guide\n' >"$dir/README.md"
  printf '# Installing\n' >"$dir/install.md"
  printf '# Settings\n' >"$dir/user-guide/settings.md"
  cat >"$dir/SUMMARY.md" <<'MD'
# Summary

[The guide](README.md)
[Installing](install.md)

# User guide

- [Settings](user-guide/settings.md)
MD
}

make_css() {
  # The control: every value that the token set owns comes through a variable, including one local
  # custom property that forwards a token rather than restating its value.
  cat >"$1" <<'CSS'
:root {
    --code-font-size: var(--micold-type-body-medium-size);
}

.page {
    color: var(--micold-color-on-surface);
    background: var(--micold-color-surface);
    font-size: var(--micold-type-body-large-size);
    max-width: 80ch;
    padding: 24px 32px;
    border-radius: var(--micold-shape-corner-medium);
    box-shadow: var(--micold-elevation-1);
    transition: box-shadow var(--micold-motion-duration-short-4) var(--micold-motion-easing-standard);
}

code {
    font-size: var(--code-font-size);
}
CSS
}

make_docs "$work/docs-good"
make_css "$work/site-good.css"

# A page that exists and is listed nowhere. The commonest way a documentation set loses a page:
# the file was added, the table of contents was not.
cp -R "$work/docs-good" "$work/docs-unlisted"
printf '# Icons\n' >"$work/docs-unlisted/user-guide/icons.md"

# The mirror image: an entry naming a file that is not there.
cp -R "$work/docs-good" "$work/docs-missing"
printf -- '- [Icons](user-guide/icons.md)\n' >>"$work/docs-missing/SUMMARY.md"

# A page the build generates rather than the repository carrying -- `licences.md` is the real one.
# It is listed, it is absent from the source tree, and it must be tolerated only because it was
# named as generated. A blanket tolerance would swallow the assertion above.
cp -R "$work/docs-good" "$work/docs-generated"
printf -- '[Licences](licences.md)\n' >>"$work/docs-generated/SUMMARY.md"

printf '== the page set (FR-023, SC-002) ==\n'
expect_pass "a documentation set whose pages and table of contents agree passes" \
  --docs "$work/docs-good" --css "$work/site-good.css"
expect_fail "a page no SUMMARY entry names fails" "user-guide/icons.md" \
  --docs "$work/docs-unlisted" --css "$work/site-good.css"
expect_fail "an entry naming a file that is not there fails" "user-guide/icons.md" \
  --docs "$work/docs-missing" --css "$work/site-good.css"
expect_pass "an entry for a page the build generates passes when it is declared as generated" \
  --docs "$work/docs-generated" --css "$work/site-good.css" --generated licences.md
expect_fail "the same entry fails when it is not declared -- the tolerance is per page, not blanket" \
  "licences.md" --docs "$work/docs-generated" --css "$work/site-good.css" --generated nothing.md
expect_pass "SUMMARY.md itself needs no entry of its own" \
  --docs "$work/docs-good" --css "$work/site-good.css"

printf '== the theme is drawn from the tokens (SC-008, SC-014) ==\n'

css_case() {
  # $1 = the name of the assertion, $2 = the needle, $3 = the offending declaration
  local what="$1" needle="$2" decl="$3" file="$work/case.css"
  make_css "$file"
  printf '\n.offender {\n    %s\n}\n' "$decl" >>"$file"
  expect_fail "$what" "$needle" --docs "$work/docs-good" --css "$file"
}

css_case "a hex colour fails" "#1a2b3c" "color: #1a2b3c;"
css_case "an rgb() colour fails" "rgb(" "background: rgb(30 30 30 / 60%);"
css_case "an hsl() colour fails" "hsl(" "border-color: hsl(210, 40%, 30%);"
css_case "a literal type size fails" "font-size" "font-size: 15px;"
css_case "a literal corner radius fails" "border-radius" "border-radius: 6px;"
css_case "a literal elevation fails" "box-shadow" "box-shadow: 0 1px 2px var(--micold-color-shadow);"
css_case "a literal motion duration fails" "150ms" "transition: opacity 150ms ease;"

# Prose is not code. `site.css` explains itself at length -- two of its comments discuss mdBook's
# own root font-size, and one names a hex colour to say why the theme does not. A check that read
# those would be a check nobody could keep passing without deleting the explanations.
cat >"$work/comment.css" <<'CSS'
/* mdBook's root font-size is 62.5%, so 1rem is 10px; the tokens are in rem for that reason.
   The default theme paints its own #ffffff behind the page, with a 4px radius and a 120ms fade. */
.page {
    color: var(--micold-color-on-surface);
}
CSS
expect_pass "a colour, a size, a radius and a duration inside a comment pass" \
  --docs "$work/docs-good" --css "$work/comment.css"

# Not everything with a number in it is a token. The token set has no opinion about how wide the
# content column is or how much padding surrounds it, and a check that flagged those would push
# authors towards inventing tokens to silence it.
cat >"$work/spacing.css" <<'CSS'
.page {
    max-width: 80ch;
    padding: 24px 32px;
    margin: 0 auto;
    border-width: 1px;
    z-index: 3;
}
CSS
expect_pass "widths, padding and other untokenised values pass" \
  --docs "$work/docs-good" --css "$work/spacing.css"

printf '== the repository itself ==\n'
# The point of the two above: the real files must pass, today, unchanged.
expect_pass "the repository's own docs/ and site.css pass"

printf '\n'
if [ "$failures" -eq 0 ]; then
  printf 'the page-set checks: all assertions hold\n'
else
  printf '%d assertion(s) failed\n' "$failures"
fi
exit "$([ "$failures" -eq 0 ] && echo 0 || echo 1)"
