#!/usr/bin/env bash
# Build the staging tree the documentation site is rendered from (feature 028, T014).
#
# The published prose and the repository's prose are the same prose. What the site additionally
# needs — a version number that changes every release, figure markup for media that does not exist
# until a publication produces it, download links naming release assets — is added *here*, to a copy,
# and never to `docs/`. Writing any of it into the source tree would put release artefacts in git and
# turn every page's history into a list of version bumps (FR-001).
#
# Reads `docs/`, writes `<out>/src/`. Never writes to `docs/`, which
# `scripts/tests/site-stage.test.sh` asserts by checksumming the input across a run.
#
#   site/stage.sh [--docs DIR] [--media-manifest FILE] [--out DIR] [--version V] [--tag T]

set -euo pipefail

root="$(git rev-parse --show-toplevel)"

docs="$root/docs"
manifest="$root/site/media.toml"
out="$root/site/build"
version=""
tag=""

die() {
  printf 'stage.sh: %s\n' "$1" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --docs) docs="$2"; shift 2 ;;
    --media-manifest) manifest="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    --tag) tag="$2"; shift 2 ;;
    -h | --help) sed -n '2,12p' "$0"; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

# The workspace version is the one the release carries, so it is the one the site shows. Read rather
# than passed, so a local build without arguments still shows the truth.
if [ -z "$version" ]; then
  version="$(awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' "$root/Cargo.toml")"
fi
[ -n "$version" ] || die "could not determine the version, and none was given"
[ -n "$tag" ] || tag="v$version"

[ -d "$docs" ] || die "no such documentation directory: $docs"
[ -f "$manifest" ] || die "no such media manifest: $manifest"

# --- the manifest ------------------------------------------------------------------------------
#
# A deliberately small TOML reader: `[media.<id>]` tables of quoted scalars, which is the whole of
# contracts/media-manifest.md. Anything richer would be a parser to maintain and a second place for
# the manifest's shape to be defined.

declare -A media_kind media_alt media_caption
current=""
while IFS= read -r line || [ -n "$line" ]; do
  line="${line%$'\r'}"
  case "$line" in
    \#*) continue ;;
  esac
  if [[ "$line" =~ ^\[media\.([a-z0-9]+(-[a-z0-9]+)*)\][[:space:]]*$ ]]; then
    current="${BASH_REMATCH[1]}"
    media_kind["$current"]="still"
    media_alt["$current"]=""
    media_caption["$current"]=""
    continue
  fi
  [ -n "$current" ] || continue
  if [[ "$line" =~ ^[[:space:]]*([a-z_]+)[[:space:]]*=[[:space:]]*\"(.*)\"[[:space:]]*$ ]]; then
    key="${BASH_REMATCH[1]}"
    value="${BASH_REMATCH[2]}"
    case "$key" in
      kind) media_kind["$current"]="$value" ;;
      alt) media_alt["$current"]="$value" ;;
      caption) media_caption["$current"]="$value" ;;
    esac
  fi
done < "$manifest"

# --- the tree ----------------------------------------------------------------------------------
#
# `media/` survives a re-stage. `build.sh` captures into the staging tree and then stages, so
# clearing the whole of `src/` would delete the captures the stage is about to reference — and the
# staging would still succeed, with every figure pointing at a file that is no longer there.

mkdir -p "$out/src"
find "$out/src" -mindepth 1 -maxdepth 1 ! -name media -exec rm -rf {} +
cp -R "$docs/." "$out/src/"

# The three font files ship with the site rather than being fetched: a documentation page that
# reaches a font CDN sends every reader's address to a third party, and reads differently when that
# third party is unreachable (FR-031, SC-015).
if [ -d "$root/assets/fonts" ]; then
  mkdir -p "$out/src/fonts"
  cp "$root/assets/fonts/"*.ttf "$out/src/fonts/" 2>/dev/null || true
  cp "$root/assets/fonts/"LICENSE* "$out/src/fonts/" 2>/dev/null || true
fi

# --- substitution and directives -----------------------------------------------------------------

html_escape() {
  local s="$1"
  s="${s//&/&amp;}"
  s="${s//</&lt;}"
  s="${s//>/&gt;}"
  s="${s//\"/&quot;}"
  printf '%s' "$s"
}

# `identify` is only present on a machine that captures. Without it — and on a `--no-media` build,
# where nothing has been captured yet — the dimensions are simply left off rather than guessed: a
# wrong intrinsic size reserves the wrong space and makes the page jump as the image arrives, which
# is worse than reserving none.
dimensions() {
  local file="$1"
  [ -f "$file" ] || return 0
  command -v identify >/dev/null 2>&1 || return 0
  local wh
  wh="$(identify -format '%w %h' "$file[0]" 2>/dev/null)" || return 0
  printf ' width="%s" height="%s"' "${wh% *}" "${wh#* }"
}

figure() {
  local id="$1" prefix="$2"
  local alt caption kind
  alt="$(html_escape "${media_alt[$id]}")"
  caption="${media_caption[$id]}"
  kind="${media_kind[$id]}"
  local poster="$out/src/media/$id.png"
  local size
  size="$(dimensions "$poster")"
  local tail=""
  [ -n "$caption" ] && tail="<figcaption>$(html_escape "$caption")</figcaption>"

  if [ "$kind" = "clip" ]; then
    # `preload="none"` is load-bearing for FR-028 — nothing moving is fetched until the reader asks
    # — and the absence of `autoplay` is load-bearing for FR-015a.
    printf '<figure class="media"><video controls loop muted playsinline preload="none" poster="%smedia/%s.png" aria-label="%s"%s>\n' \
      "$prefix" "$id" "$alt" "$size"
    printf '<source src="%smedia/%s.webm" type="video/webm">\n' "$prefix" "$id"
    printf '<source src="%smedia/%s.mp4" type="video/mp4">\n' "$prefix" "$id"
    printf '</video>%s</figure>\n' "$tail"
  else
    printf '<figure class="media"><img src="%smedia/%s.png" alt="%s" loading="lazy"%s>\n%s</figure>\n' \
      "$prefix" "$id" "$alt" "$size" "$tail"
  fi
}

undeclared=0

while IFS= read -r page; do
  relative="${page#"$out/src/"}"
  # A page one directory down reaches the same media through its own depth. A single root-relative
  # path would 404 on every nested page of the published site, and only there.
  depth="$(printf '%s' "$relative" | tr -cd '/' | wc -c)"
  prefix=""
  for ((i = 0; i < depth; i++)); do prefix="../$prefix"; done

  staged="$page.staged"
  : > "$staged"
  while IFS= read -r line || [ -n "$line" ]; do
    if [[ "$line" =~ ^[[:space:]]*\<!--[[:space:]]*media:[[:space:]]*([a-z0-9]+(-[a-z0-9]+)*)[[:space:]]*--\>[[:space:]]*$ ]]; then
      id="${BASH_REMATCH[1]}"
      if [ -z "${media_kind[$id]+set}" ]; then
        printf 'stage.sh: %s references media id "%s", which %s does not declare\n' \
          "$relative" "$id" "$manifest" >&2
        undeclared=$((undeclared + 1))
        continue
      fi
      figure "$id" "$prefix" >> "$staged"
      continue
    fi
    line="${line//\{\{MICOLD_VERSION\}\}/$version}"
    line="${line//\{\{MICOLD_TAG\}\}/$tag}"
    printf '%s\n' "$line" >> "$staged"
  done < "$page"
  mv "$staged" "$page"
done < <(find "$out/src" -type f -name '*.md' | sort)

[ "$undeclared" -eq 0 ] || die "$undeclared media directive(s) name an id the manifest does not declare"

# --- the release's own downloads (FR-004a) --------------------------------------------------------
#
# A download link is the one thing on the site that can be perfectly written prose about a file that
# is not there: it reads correctly, resolves to a 404, and the page gives the reader no clue. Nothing
# in the text can settle it -- only the release's asset list can -- so the staged pages are held to
# that list here, at the one moment the tag being published is known.
#
# The list comes from `MICOLD_RELEASE_ASSETS` when the caller has it (the publishing workflow does,
# and so does the test), and otherwise from `gh`. When neither can supply it the check is *skipped
# and said so*: a local build without `gh` must not look the same as a release whose links were
# checked, so `MICOLD_SITE_STRICT` -- which the publication sets -- turns the skip into a failure.

release_assets="${MICOLD_RELEASE_ASSETS-}"
assets_known=0
if [ -n "$release_assets" ]; then
  assets_known=1
elif command -v gh >/dev/null 2>&1 &&
  release_assets="$(gh release view "$tag" --json assets --jq '.assets[].name' 2>/dev/null)"; then
  assets_known=1
fi

# Every downloads URL on every staged page, with the tag it names, after substitution.
links="$(grep -rhoE 'https://github\.com/[^/]+/[^/]+/releases/download/[^/"^)  ]+/[^)"'"'"' ]+' \
  "$out/src" 2>/dev/null | sort -u || true)"

if [ -n "$links" ]; then
  if [ "$assets_known" -eq 0 ]; then
    message="the download links name files that were not checked against $tag (no MICOLD_RELEASE_ASSETS, and \`gh\` could not answer)"
    if [ -n "${MICOLD_SITE_STRICT-}" ]; then
      die "$message"
    fi
    printf 'stage.sh: %s\n' "$message" >&2
  else
    bad=0
    while IFS= read -r link; do
      [ -n "$link" ] || continue
      file="${link##*/}"
      link_tag="${link%/*}"
      link_tag="${link_tag##*/}"
      if [ "$link_tag" != "$tag" ]; then
        printf 'stage.sh: a download link points into release %s, not %s: %s\n' "$link_tag" "$tag" "$link" >&2
        bad=$((bad + 1))
        continue
      fi
      case " $(printf '%s' "$release_assets" | tr '\n' ' ') " in
        *" $file "*) ;;
        *)
          printf 'stage.sh: release %s does not carry %s, which a page links to\n' "$tag" "$file" >&2
          bad=$((bad + 1))
          ;;
      esac
    done <<< "$links"
    [ "$bad" -eq 0 ] || die "$bad download link(s) do not match what release $tag carries"
  fi
fi

# --- the licences page (FR-008, FR-031) -----------------------------------------------------------
#
# Generated rather than committed, for the reason every generated page here is: the texts already
# exist in the repository, and a second copy under `docs/` would be a copy that can fall behind the
# licence it restates. The site owes the reader the *shipped* text, so it is assembled from the
# shipped files at staging time.
#
# It matters more here than elsewhere because the site serves the fonts itself (FR-031): a page that
# fetched them from a font host would carry no obligation at all, and the reason this project ships
# them instead is that a documentation page must not send its readers' addresses to a third party.
# Shipping them is what makes these two licences the site's own to reproduce.

licences="$out/src/licences.md"
{
  cat <<'MD'
# Licences

Micold AI IDE is published under the **Apache License 2.0**. The typefaces it ships are separate
works under their own terms, and this page carries all of them in full.

The site serves its fonts itself, from files in this repository — nothing on any page here is
fetched from a font host or a CDN. That is what puts these licences on this page: the fonts travel
with the site, so their terms do too.

## The fonts

MD
  # The mapping table is PROVENANCE.md's own, taken rather than restated: two lists of which font is
  # under which licence would be one list too many.
  awk '/^\| Font \|/, /^[[:space:]]*$/' "$root/assets/fonts/PROVENANCE.md"
  cat <<MD

The full provenance — where each file came from and how it was produced — is
[\`assets/fonts/PROVENANCE.md\`](https://github.com/Cumulocity-IoT/micold-ai-ide/blob/$tag/assets/fonts/PROVENANCE.md)
in the repository at this release.

## Apache License 2.0

This covers Micold AI IDE itself and the Material Symbols Outlined icon font. The icon font ships
its own verbatim copy at
[\`assets/fonts/LICENSE\`](https://github.com/Cumulocity-IoT/micold-ai-ide/blob/$tag/assets/fonts/LICENSE);
the text below is the project's own, which differs from it only in the appendix that names the
copyright holder.

MD
  printf '```text\n'
  cat "$root/LICENSE"
  printf '\n```\n'
  cat <<'MD'

## SIL Open Font License 1.1

This covers Roboto — `Roboto-Regular.ttf` and `Roboto-Medium.ttf`.

MD
  printf '```text\n'
  cat "$root/assets/fonts/LICENSE-Roboto-OFL.txt"
  printf '\n```\n'
} > "$licences"

# --- the generated theme partial -------------------------------------------------------------
#
# mdBook renders `theme/header.hbs` at the top of every page with the page's own context, which is
# what makes a *per-page* source link possible at all. It is generated rather than committed for the
# same reason `tokens.css` is: it carries the version, and a version in the source tree is a file
# that has to be edited by every release.

# mdBook renders the book's first page from `README.md` and calls it `index.md` everywhere
# afterwards, `{{ path }}` included. Both source links undo that rename, so the guess is asserted
# here rather than left to be discovered as a 404 on the site's most-read page.
[ -f "$docs/README.md" ] || die "docs/README.md is missing -- the source links undo mdBook's rename of it"

theme="$root/site/theme"
if [ -d "$theme" ]; then
  cat > "$theme/header.hbs" <<HBS
<!-- GENERATED by site/stage.sh. Do not edit. -->
<div class="micold-release">
  <span class="micold-release-version">$(html_escape "$version")</span>
  <a class="micold-release-source"
     href="https://github.com/Cumulocity-IoT/micold-ai-ide/blob/$(html_escape "$tag")/docs/{{#if (eq path "index.md")}}README.md{{else}}{{ path }}{{/if}}">View
    this page's source</a>
</div>
HBS
fi

printf 'stage.sh: staged %s into %s at version %s (%s)\n' "$docs" "$out/src" "$version" "$tag"
