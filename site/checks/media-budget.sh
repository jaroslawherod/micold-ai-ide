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
# The third rule is not a budget at all: nothing may be published under `media/` that no page
# references. Such a file is invisible to the budgets above -- no page links to it, so it is on no
# page's total -- and the deploy carries it anyway. See the rule itself, below, for the case that
# put it here.
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

# Two pages reach the same picture by different relative paths -- `media/x.png` from the root and
# `../media/x.png` one directory down -- so the paths have to be resolved before they can be
# compared with what is on disk. `realpath` and `readlink -f` are not both present everywhere; `cd`
# is.
declare -A referenced=()
canon() {
  printf '%s/%s\n' "$(cd "$(dirname "$1")" && pwd -P)" "$(basename "$1")"
}

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
    referenced["$(canon "$file")"]=1
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

# Every page has now said what it wants, so anything left under media/ is weight nobody asked for.
# The capture step is where it comes from: a clip is encoded out of a directory of frame PNGs that
# is written beside the encodes, in the same staged directory the renderer copies wholesale into the
# built site. Those frames are already inside the .webm and the .mp4, so publishing them costs
# megabytes on every deploy that no reader ever downloads -- and no per-page budget above can catch
# it, because catching it there would require a page to link to them, which is the very thing that
# is not happening.
if [ -d "$root/media" ]; then
  orphans=()
  orphan_bytes=0
  while IFS= read -r file; do
    [ -n "${referenced["$(canon "$file")"]:-}" ] && continue
    size="$(wc -c <"$file")"
    orphan_bytes=$((orphan_bytes + size))
    orphans+=("${file#"$root"/}|$size")
  done < <(find "$root/media" -type f | sort)

  if [ "${#orphans[@]}" -gt 0 ]; then
    report "$(printf '%d file(s) under media/ totalling %s are published but no page references them' \
      "${#orphans[@]}" "$(mb "$orphan_bytes")")"
    # Enough of them to recognise the shape -- a whole `.frames` directory reads the same in five
    # lines as in seventy -- and a count for the rest.
    for entry in "${orphans[@]:0:5}"; do
      detail "$(printf '%-52s %s' "${entry%%|*}" "$(mb "${entry##*|}")")"
    done
    [ "${#orphans[@]}" -gt 5 ] && detail "... and $(( ${#orphans[@]} - 5 )) more"
  fi
fi

if [ "$problems" -eq 0 ]; then
  printf 'media-budget: %d page(s) under %s -- every page inside %s of stills, every clip inside %s, nothing published unreferenced\n' \
    "$pages" "$dir" "$(mb "$still_budget")" "$(mb "$clip_budget")"
  exit 0
fi
printf 'media-budget: %d problem(s) in %d page(s) under %s\n' "$problems" "$pages" "$dir" >&2
exit 1
