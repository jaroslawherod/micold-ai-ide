#!/usr/bin/env bash
# Build the documentation site (feature 028, T017).
#
# One script, five steps, in this order and no other:
#
#   1. emit   the theme's variables from the application's design tokens
#   2. capture the screenshots and clips the manifest declares
#   3. stage  a copy of `docs/` with the version substituted and media directives expanded
#   4. render the staged copy with mdBook
#   5. check  the built site, before anything is deployed
#
# The order is the argument. The theme cannot be edited into disagreement with the application
# because it is regenerated here; the media cannot go stale because it is captured here; and nothing
# reaches a reader that has not been through step 5, because the workflow that publishes runs this
# script and deploys only what it leaves behind (FR-018).
#
#   site/build.sh                     # everything
#   site/build.sh --no-media          # skip capture -- the local iteration loop (quickstart A3)
#   site/build.sh --checks-only       # re-run the checks against the site already in site/book
#
# `mise run site-build` and `mise run site-check` are the same two entry points.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
site="$root/site"
book="$site/book"
staging="$site/build"

capture=1
render=1
version=""
tag=""

die() {
  printf 'build.sh: %s\n' "$1" >&2
  exit 1
}

step() {
  printf '\n== %s\n' "$1"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --no-media) capture=0; shift ;;
    --checks-only) render=0; capture=0; shift ;;
    --version) version="$2"; shift 2 ;;
    --tag) tag="$2"; shift 2 ;;
    -h | --help) sed -n '2,21p' "$0"; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

if [ "$render" -eq 1 ]; then
  # --- 1. the theme ------------------------------------------------------------------------------
  #
  # Generated on every build, from the same token set the application renders with, so the site
  # cannot drift into looking like a different product (FR-030). It is written through a temporary
  # file: a failed emit that truncated the stylesheet would leave a site that builds and looks
  # unstyled, which is a worse failure than not building at all.

  step "theme: emitting site/theme/css/tokens.css from the design tokens"
  mkdir -p "$site/theme/css"
  "$root/scripts/build-lock.sh" cargo run --quiet -p micold-core --bin micold-tokens-css \
    >"$site/theme/css/tokens.css.new"
  mv "$site/theme/css/tokens.css.new" "$site/theme/css/tokens.css"

  # --- 2. the media ------------------------------------------------------------------------------
  #
  # Captured from the application built from this same checkout. --no-media skips it for local
  # iteration on prose or theme; every published build captures, because a screenshot that was not
  # produced by the release it ships with is a screenshot of a different program (FR-011).

  if [ "$capture" -eq 1 ]; then
    step "media: capturing every entry in site/media.toml"
    [ -x "$site/capture/capture.sh" ] ||
      die "site/capture/capture.sh is not there yet -- pass --no-media to build without it"
    # `site/build/src/media` survives a re-stage, on purpose -- it is where the capture writes and
    # the stage reads. That is also how a picture from a previous run could be published as if it
    # were this one's, so the moment this run started capturing is recorded and step 5 requires
    # every published file to be newer than it.
    mkdir -p "$staging"
    : >"$staging/.capture-stamp"
    "$site/capture/capture.sh" --out "$staging/src/media"
  else
    step "media: skipped (--no-media) -- pages will show figures with no file behind them"
  fi

  # --- 3. staging --------------------------------------------------------------------------------

  step "stage: copying docs/ into site/build/src with the version and media expanded"
  stage_args=()
  [ -n "$version" ] && stage_args+=(--version "$version")
  [ -n "$tag" ] && stage_args+=(--tag "$tag")
  "$site/stage.sh" --out "$staging" "${stage_args[@]+"${stage_args[@]}"}"

  # --- 4. the render -----------------------------------------------------------------------------

  step "render: mdbook build"
  command -v mdbook >/dev/null 2>&1 ||
    die "mdbook is not installed -- see site/README.md for the toolchain"
  mdbook build "$site"
fi

[ -d "$book" ] || die "there is no built site at $book -- run without --checks-only first"

# --- 5. the checks -------------------------------------------------------------------------------
#
# The pre-deploy set from contracts/site-checks.md, in the order the workflow runs them. Each is a
# separate program that exits non-zero and says what failed; none of them repairs anything.
#
# A check that is not present yet is reported and skipped, so this script stays runnable while the
# set is still being written -- but MICOLD_SITE_STRICT=1 makes a missing check fatal, and the
# publishing workflow sets it. Otherwise "the check was not there" and "the check passed" would look
# the same from the outside, which is the one thing a gate must never allow.

strict="${MICOLD_SITE_STRICT:-}"
missing=0

# `page-checks.mjs` is a Node program with a `#!/usr/bin/env node` line, and Node is not a thing this
# repository otherwise needs: CI installs it for the docs job, and a developer machine has it through
# mise (`node = "20"` in mise.toml). Running this script straight from a shell is the case in
# between -- mise has installed Node but has not put it on PATH -- so the directory is added here
# rather than letting the check die with `env: node: No such file or directory`, which says nothing
# about what to do next.
if ! command -v node >/dev/null 2>&1 && command -v mise >/dev/null 2>&1; then
  node_bin="$(mise where node@20 2>/dev/null || true)/bin"
  [ -x "$node_bin/node" ] && PATH="$node_bin:$PATH" && export PATH
fi

run_check() {
  local what="$1"
  shift
  if [ ! -x "$1" ]; then
    printf 'build.sh: %s is not present (%s) -- skipping\n' "$what" "$1" >&2
    missing=1
    return 0
  fi
  printf -- '-- %s\n' "$what"
  "$@"
}

step "check: the built site"

# --- what the manifest promised, in the site a reader would get ----------------------------------
#
# `capture.sh` already refuses a run that produced fewer files than the manifest declares, and this
# says the same thing one stage later, about the built site rather than the staging tree. The two
# are not the same assertion: between them lie the stage and the render, and a figure can lose its
# file to either. A missing picture is not a small defect on this site -- the pictures are what it
# is for -- so it is never published as a gap (FR-011a, SC-004).
#
# `-nt` is the second half. Without it the assertion is satisfied by whatever a previous run left
# behind, which is exactly the failure that looks like success: a page showing last month's window,
# captured from a build nobody is running any more.
media_missing=0
media_stale=0
stamp="$staging/.capture-stamp"
while IFS=$'\t' read -r id kind _scene _scheme; do
  [ -n "$id" ] || continue
  files=("$book/media/$id.png")
  [ "$kind" = "clip" ] && files+=("$book/media/$id.webm" "$book/media/$id.mp4")
  for file in "${files[@]}"; do
    if [ ! -f "$file" ]; then
      printf 'build.sh: %s declares "%s", and %s is not in the built site\n' \
        "site/media.toml" "$id" "${file#"$book"/}" >&2
      media_missing=$((media_missing + 1))
    elif [ "$capture" -eq 1 ] && [ ! "$file" -nt "$stamp" ]; then
      printf 'build.sh: %s is older than this run'"'"'s capture -- it is a file from a previous build\n' \
        "${file#"$book"/}" >&2
      media_stale=$((media_stale + 1))
    fi
  done
done < <("$site/capture/capture.sh" --list)

if [ "$media_missing" -ne 0 ] || [ "$media_stale" -ne 0 ]; then
  die "$media_missing declared media file(s) missing from the built site, $media_stale left over from a previous run"
fi
printf -- '-- media completeness\n'

run_check "internal links" "$site/checks/links.sh" --built "$book"
run_check "media budget" "$site/checks/media-budget.sh" "$book"
run_check "page checks" "$site/checks/page-checks.mjs" "$book"

if [ "$missing" -eq 1 ] && [ -n "$strict" ]; then
  die "a check listed above is missing and MICOLD_SITE_STRICT is set -- refusing to call this built"
fi
if [ "$capture" -eq 0 ] && [ -n "$strict" ]; then
  die "--no-media leaves the media from whatever ran last, and MICOLD_SITE_STRICT is set -- a publication captures"
fi

printf '\nBuilt %s\n' "$book"
