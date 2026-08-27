#!/usr/bin/env bash
# Fails when a built page carries more media weight than a reader on a slow connection should be
# asked to download (feature 028, T056).
#
#   media-budget.sh <dir>   the built site (e.g. site/book)
#
# Contract: specs/028-docs-site-github-pages/contracts/site-checks.md
# Test: scripts/tests/media-budget.test.sh
#
# The pictures are the point of this site and the pictures are what makes it slow, so the limit is
# per page rather than per site: a guide is read one page at a time, and the page in front of the
# reader is the whole of what they are waiting for. 1 MB of stills on any one page, 3 MB for any
# single clip file (SC-012). A picture shown on two pages is counted on both, because it is
# downloaded on both.
#
# The check never repairs anything. Downscaling on the author's behalf is the obvious convenience
# and the wrong one: the published picture would then differ from the captured one with nobody
# having compared them, and a scene that quietly got heavier would be papered over on every
# publication rather than reported once. What this prints is a page, its assets and its total --
# enough to decide whether the scene should be tighter or the picture should not be there.

set -uo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: media-budget.sh <dir>   check the media weight of a built site (e.g. site/book)
USAGE
  exit 2
}

dir=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -h | --help) usage ;;
    -*) usage ;;
    *)
      [ -n "$dir" ] && usage
      dir="$1"
      ;;
  esac
  shift
done
[ -n "$dir" ] || usage

if [ ! -d "$dir" ]; then
  echo "media-budget: $dir is not a directory" >&2
  exit 2
fi

# Decimal megabytes, the unit a reader's connection is quoted in.
still_budget=1000000
clip_budget=3000000

problems=0
report() {
  printf 'media-budget: %s\n' "$1" >&2
  problems=$((problems + 1))
}
detail() { printf 'media-budget:   %s\n' "$1" >&2; }

mb() { awk -v b="$1" 'BEGIN { printf "%.1f MB", b / 1000000 }'; }

root="$(cd "$dir" && pwd)"

pages=0
while IFS= read -r page; do
  pages=$((pages + 1))
  page_dir="$(dirname "$page")"
  rel_page="${page#"$root"/}"

  stills=0
  still_list=()
  # Every locally-referenced image and every media source on the page, whichever tag carried it: a
  # `poster` is a still -- it is what the reader downloads before deciding to play anything -- and a
  # `<source>` is the clip itself. Off-origin and data: references are somebody else's check
  # (page-checks.mjs asserts there are none).
  while IFS= read -r ref; do
    case "$ref" in
      http://* | https://* | //* | data:* | "") continue ;;
    esac
    ref="${ref%%\?*}"
    ref="${ref%%#*}"
    case "$ref" in
      /*) file="$root$ref" ;;
      *) file="$page_dir/$ref" ;;
    esac
    [ -f "$file" ] || continue
    size="$(wc -c <"$file")"
    rel="${file#"$root"/}"
    case "${ref##*.}" in
      png | jpg | jpeg | gif | webp | avif | svg)
        stills=$((stills + size))
        still_list+=("$rel|$size")
        ;;
      webm | mp4 | mov)
        if [ "$size" -gt "$clip_budget" ]; then
          report "$rel_page: $rel is $(mb "$size"), over the $(mb "$clip_budget") budget for a single clip"
        fi
        ;;
    esac
  done < <(grep -oE '(src|poster)="[^"]*"' "$page" 2>/dev/null | sed 's/^[a-z]*="//; s/"$//')

  if [ "$stills" -gt "$still_budget" ]; then
    report "$rel_page: its stills total $(mb "$stills"), over the $(mb "$still_budget") budget for one page"
    for entry in "${still_list[@]}"; do
      detail "$(printf '%-52s %s' "${entry%%|*}" "$(mb "${entry##*|}")")"
    done
  fi
done < <(find "$root" -type f -name '*.html' | sort)

if [ "$pages" -eq 0 ]; then
  echo "media-budget: no .html page under $dir -- nothing was measured, which is not the same as passing" >&2
  exit 2
fi

if [ "$problems" -eq 0 ]; then
  printf 'media-budget: %d page(s) under %s -- every page inside %s of stills, every clip inside %s\n' \
    "$pages" "$dir" "$(mb "$still_budget")" "$(mb "$clip_budget")"
  exit 0
fi
printf 'media-budget: %d page(s) over budget\n' "$problems" >&2
exit 1
