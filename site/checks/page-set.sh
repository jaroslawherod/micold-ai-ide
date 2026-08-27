#!/usr/bin/env bash
# Fails when the page set and the table of contents disagree, or when the theme stops being drawn
# from the design tokens (feature 028, T054).
#
#   page-set.sh [--docs <dir>] [--css <file>] [--generated <name>]...
#
# Contract: specs/028-docs-site-github-pages/contracts/site-checks.md
# Test: scripts/tests/page-set.test.sh
#
# Two failures, both quiet, both caught here because this is a step in CI's `docs` job -- the one
# job feature 023 keeps running on a documentation-only change, and so the only place a check over
# documentation is guaranteed to run at all.
#
# The first is the page set. `docs/SUMMARY.md` is what the renderer reads, so a page nobody listed
# is a page nobody can reach: the link check passes, the build passes, and the page is simply not
# on the site. The reverse -- an entry naming a file that was renamed -- is louder but arrives at
# render time, on the release branch, after the tag.
#
# The second is the theme. `site/theme/css/site.css` exists to draw the site out of the same tokens
# the application is drawn from, so that one token change moves both. One literal is enough to end
# that: it survives the next token change silently, and the site drifts away from the product it
# documents a colour at a time. So values are read, not property names -- `--code-font-size:
# var(--micold-type-body-medium-size)` is a token forwarded under a shorter name, not a literal --
# and only the five kinds of value the token set has an opinion about are read at all. Widths,
# spacing and z-order are nobody's token, and flagging them would push authors into inventing
# tokens to silence a check.

set -uo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: page-set.sh [--docs <dir>] [--css <file>] [--generated <name>]...
       --docs       the documentation sources        (default: docs)
       --css        the site stylesheet              (default: site/theme/css/site.css)
       --generated  a SUMMARY entry whose file the build produces, repeatable
                    (default: licences.md)
USAGE
  exit 2
}

docs="docs"
css="site/theme/css/site.css"
generated=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    --docs)
      [ "$#" -ge 2 ] || usage
      docs="$2"
      shift
      ;;
    --css)
      [ "$#" -ge 2 ] || usage
      css="$2"
      shift
      ;;
    --generated)
      [ "$#" -ge 2 ] || usage
      generated+=("$2")
      shift
      ;;
    -h | --help) usage ;;
    *) usage ;;
  esac
  shift
done
[ "${#generated[@]}" -gt 0 ] || generated=("licences.md")

[ -d "$docs" ] || {
  echo "page-set: $docs is not a directory" >&2
  exit 2
}
[ -f "$css" ] || {
  echo "page-set: $css is not a file" >&2
  exit 2
}

summary="$docs/SUMMARY.md"
[ -f "$summary" ] || {
  echo "page-set: $summary is not there -- the renderer has no table of contents to read" >&2
  exit 2
}

problems=0
report() {
  printf 'page-set: %s\n' "$1" >&2
  problems=$((problems + 1))
}

# --- the page set ---------------------------------------------------------------------------------

# Every `](path.md)` in the table of contents, relative to `docs/`. mdBook also accepts entries with
# no link at all -- `- [Not written yet]()` -- which are placeholders rather than pages and name no
# file to check.
declare -A listed=()
while IFS= read -r target; do
  [ -n "$target" ] || continue
  case "$target" in
    http://* | https://* | \#*) continue ;;
  esac
  listed["${target%%#*}"]=1
done < <(grep -oE '\]\([^)]+\.md[^)]*\)' "$summary" | sed 's/^](//; s/)$//')

declare -A is_generated=()
for name in "${generated[@]}"; do is_generated["$name"]=1; done

# A listed page that is not there. `licences.md` is the standing example of the tolerated case:
# `site/stage.sh` assembles it at build time from the licence files the application ships, so it is
# real in a staged tree and absent from this one. The tolerance is per name on purpose -- a blanket
# one would swallow every rename this check exists to catch.
for target in $(printf '%s\n' "${!listed[@]}" | sort); do
  if [ ! -f "$docs/$target" ] && [ -z "${is_generated[$target]+set}" ]; then
    report "SUMMARY.md lists $target, which is not in $docs/ (if the build produces it, pass --generated $target)"
  fi
done

# A page that is there and is listed nowhere. SUMMARY.md is the table of contents, not a chapter,
# so it does not list itself.
while IFS= read -r page; do
  rel="${page#"$docs"/}"
  [ "$rel" = "SUMMARY.md" ] && continue
  if [ -z "${listed[$rel]+set}" ]; then
    report "$docs/$rel has no SUMMARY.md entry -- it renders into nothing a reader can reach"
  fi
done < <(find "$docs" -type f -name '*.md' | sort)

# --- the theme --------------------------------------------------------------------------------------

# Comments are prose. `site.css` explains its own decisions at length, and two of those explanations
# quote the very values the rules below refuse; a check that read them would be a check nobody could
# keep passing without deleting the reasoning.
css_problems="$(
  awk '
    function literal_length(v) { return v ~ /[0-9]+(\.[0-9]+)?(px|pt|rem|em|ex|ch|vh|vw|%)/ }
    function duration(v)       { return v ~ /[0-9]+(\.[0-9]+)?m?s([^a-z]|$)/ }
    function say(why)          { printf "%s|%s: %s -- %s\n", NR, prop, shown, why }
    # A declaration, not a selector: a name, a colon, and a value. `#mdbook-menu-bar` and
    # `@media (min-width: 700px)` are neither, and are not read.
    function check(text,   colon, value) {
      if (!match(text, /^[ \t]*(--)?[A-Za-z][-A-Za-z0-9]*[ \t]*:/)) return
      colon = index(text, ":")
      prop  = substr(text, 1, colon - 1); sub(/^[ \t]+/, "", prop); sub(/[ \t]+$/, "", prop)
      shown = substr(text, colon + 1);    sub(/^[ \t]+/, "", shown); sub(/[ \t;]+$/, "", shown)

      # Variable *names* carry digits -- `--micold-elevation-1` is not a one-pixel offset -- so the
      # names come out before anything counts digits. A fallback inside `var(--x, 4px)` deliberately
      # stays: it is a literal, and it is the one that gets used when the token is missing.
      value = shown
      gsub(/--[-A-Za-z0-9]+/, "", value)

      if (value ~ /#[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]/ || value ~ /rgba?\(/ || value ~ /hsla?\(/)
        say("a literal colour; use a --micold-color-* variable")
      if (prop ~ /font-size$/ && literal_length(value))
        say("a literal type size; use a --micold-type-*-size variable")
      if (prop ~ /border(-[a-z]+)*-radius$/ && literal_length(value))
        say("a literal corner radius; use a --micold-shape-corner-* variable")
      if (prop ~ /box-shadow$/ && value ~ /[0-9]/)
        say("a literal elevation; use a --micold-elevation-* variable")
      if (prop ~ /^(transition|animation)/ && duration(value))
        say("a literal duration; use a --micold-motion-duration-* variable")
    }
    {
      # Strip comments, keeping the line numbering: what is reported has to be what an author can
      # open the file at.
      line = $0; out = ""
      while (length(line) > 0) {
        if (incomment) {
          p = index(line, "*/")
          if (p == 0) { line = "" } else { line = substr(line, p + 2); incomment = 0 }
        } else {
          p = index(line, "/*")
          if (p == 0) { out = out line; line = "" }
          else { out = out substr(line, 1, p - 1); line = substr(line, p + 2); incomment = 1 }
        }
      }
      # One line can hold several declarations -- `.page { color: #fff; margin: 0 }` is legal CSS,
      # and reading only the first thing on the line would let a pasted one-line rule put a literal
      # into the theme unremarked. Braces and semicolons are what separate a selector from the
      # declarations it opens and one declaration from the next, so the line is cut on them and
      # every piece is read.
      n = split(out, piece, /[{};]/)
      for (i = 1; i <= n; i++) check(piece[i])
    }
  ' "$css"
)"

if [ -n "$css_problems" ]; then
  while IFS='|' read -r line what; do
    report "$css:$line: $what"
  done <<<"$css_problems"
fi

if [ "$problems" -eq 0 ]; then
  pages="$(find "$docs" -type f -name '*.md' | wc -l)"
  printf 'page-set: %d page(s) under %s, all listed; %s draws every tokenised value from a variable\n' \
    "$((pages - 1))" "$docs" "$css"
  exit 0
fi
printf 'page-set: %d problem(s)\n' "$problems" >&2
exit 1
