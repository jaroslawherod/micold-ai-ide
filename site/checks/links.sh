#!/usr/bin/env bash
# Fails when a link inside the documentation does not resolve (feature 028, T040).
#
#   links.sh --sources [<dir>]   the Markdown, before a merge   (default: docs)
#   links.sh --built  [<dir>]    the rendered HTML, before a deploy (default: site/book)
#
# Contract: specs/028-docs-site-github-pages/contracts/site-checks.md
# Test: scripts/tests/links.test.sh
#
# The two modes are not the same check run twice. `--sources` reads what an author wrote, where a
# broken link is a typo somebody can still see; `--built` reads what a reader gets, where the
# failure modes belong to the renderer -- a chapter missing from `SUMMARY.md`, a heading whose
# generated id is not the one the link guessed. A tree can pass either one and fail the other, so
# both run, at the two moments where the fix is cheapest.
#
# Fragments are checked, not just file names. A link that opens the right page at the wrong place
# leaves the reader looking for the paragraph they were sent to, and that failure is invisible to
# any check that stops at the path.
#
# Nothing off this machine is ever fetched (`--offline`). A publication that fails because
# somebody else's site is down, or slow, or behind a bot wall is a publication whose red run means
# nothing -- and the failure arrives months after the link was written, in a run that changed
# something else. External links are the reader's problem to report, not the pipeline's to guess at.

set -uo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: links.sh --sources [<dir>]   check the Markdown sources (default: docs)
       links.sh --built [<dir>]     check a built site (default: site/book)
USAGE
  exit 2
}

mode=""
dir=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --sources | --built)
      [ -n "$mode" ] && usage
      mode="${1#--}"
      ;;
    -h | --help) usage ;;
    -*) usage ;;
    *)
      [ -n "$dir" ] && usage
      dir="$1"
      ;;
  esac
  shift
done
[ -n "$mode" ] || usage

lychee="lychee"
command -v lychee >/dev/null 2>&1 || lychee="$HOME/.cargo/bin/lychee"
if [ ! -x "$lychee" ] && ! command -v "$lychee" >/dev/null 2>&1; then
  echo "links: lychee is not installed -- run \`cargo install lychee\`" >&2
  exit 2
fi

# One page of the site is not in `docs/`: `site/stage.sh` assembles the licences page at build time
# from the licence files the application actually ships, so `SUMMARY.md` names a file that exists
# only in a staged tree. Excluding it here is not a hole -- `--built` reads the staged tree, where
# the page is real, and fails if it is not.
exclude=()
case "$mode" in
  sources)
    dir="${dir:-docs}"
    ext="md"
    exclude=(--exclude 'licences\.md$')
    ;;
  built) dir="${dir:-site/book}" ext="html" ;;
esac

if [ ! -d "$dir" ]; then
  echo "links: $dir is not a directory" >&2
  exit 2
fi

# The inputs are listed here rather than handed to lychee as a directory: a built site also holds
# the search index, the stylesheets and the images, and a link checker reading those finds things
# that look like links inside minified JavaScript. Only the pages are pages.
mapfile -t pages < <(find "$dir" -type f -name "*.$ext" | sort)
if [ "${#pages[@]}" -eq 0 ]; then
  echo "links: no .$ext file under $dir -- nothing was checked, which is not the same as passing" >&2
  exit 2
fi

# `--root-dir` is what a root-relative href resolves against: the site's own root, not the
# filesystem's. Without it `/user-guide/settings.html` is looked for at `/user-guide/settings.html`
# on this machine, which is nobody's page.
"$lychee" \
  --offline \
  --include-fragments \
  --no-progress \
  --root-dir "$(cd "$dir" && pwd)" \
  ${exclude[@]+"${exclude[@]}"} \
  -- "${pages[@]}"
status=$?

if [ "$status" -eq 0 ]; then
  printf 'links: %d %s page(s) under %s -- every internal link and fragment resolves\n' \
    "${#pages[@]}" "$mode" "$dir"
fi
exit "$status"
