#!/usr/bin/env bash
# Fails when the pages, the media manifest and the scene scripts stop agreeing (feature 028, T055).
#
#   media-references.sh [--docs <dir>] [--manifest <file>] [--scenes <dir>]
#
# Contract: specs/028-docs-site-github-pages/contracts/site-checks.md
# Test: scripts/tests/media-references.test.sh
#
# A screenshot reaches a reader only if three files agree: the page asks for it with a
# `<!-- media: id -->` directive, `site/media.toml` declares the id, and a scene script produces it.
# Each pair drifts on its own, and the drift is only loud in one direction.
#
# `site/stage.sh` already refuses a directive naming an undeclared id -- but it refuses it during a
# build, which is a release branch and a tag later. Held here instead, the author who wrote the
# directive is still looking at the page.
#
# The other direction is silent, and it is the one that accumulates: a declared entry no page
# references is captured on every publication, costs the minutes and the megabytes, and appears
# nowhere. Nothing fails, so nobody finds out.
#
# A `scene` naming a script that is not there fails only on a machine that captures, which is not
# the machine the rename happened on. And an empty `alt` is not a small omission: it tells a screen
# reader the image is decorative, so the reader is not told there was a picture at all (FR-014).

set -uo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: media-references.sh [--docs <dir>] [--manifest <file>] [--scenes <dir>]
       --docs      the documentation sources  (default: docs)
       --manifest  the media manifest         (default: site/media.toml)
       --scenes    the scene scripts           (default: site/capture/scenes)
USAGE
  exit 2
}

docs="docs"
manifest="site/media.toml"
scenes="site/capture/scenes"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --docs)
      [ "$#" -ge 2 ] || usage
      docs="$2"
      shift
      ;;
    --manifest)
      [ "$#" -ge 2 ] || usage
      manifest="$2"
      shift
      ;;
    --scenes)
      [ "$#" -ge 2 ] || usage
      scenes="$2"
      shift
      ;;
    -h | --help) usage ;;
    *) usage ;;
  esac
  shift
done

[ -d "$docs" ] || {
  echo "media-references: $docs is not a directory" >&2
  exit 2
}
[ -f "$manifest" ] || {
  echo "media-references: $manifest is not a file" >&2
  exit 2
}
[ -d "$scenes" ] || {
  echo "media-references: $scenes is not a directory" >&2
  exit 2
}

problems=0
report() {
  printf 'media-references: %s\n' "$1" >&2
  problems=$((problems + 1))
}

# The same deliberately small TOML reader `site/stage.sh` uses: `[media.<id>]` tables of quoted
# scalars, which is the whole of contracts/media-manifest.md. Two readers of one format is already
# one too many; a third, richer one would be a parser to keep in step with a build.
declare -A media_scene=() media_alt=() media_has_alt=()
order=()
current=""
while IFS= read -r line || [ -n "$line" ]; do
  line="${line%%$'\r'}"
  [[ "$line" =~ ^[[:space:]]*# ]] && continue
  [[ "$line" =~ ^[[:space:]]*$ ]] && continue
  if [[ "$line" =~ ^\[media\.([a-z0-9]+(-[a-z0-9]+)*)\][[:space:]]*$ ]]; then
    current="${BASH_REMATCH[1]}"
    order+=("$current")
    media_scene["$current"]=""
    media_alt["$current"]=""
    continue
  fi
  [ -n "$current" ] || continue
  if [[ "$line" =~ ^[[:space:]]*([a-z_]+)[[:space:]]*=[[:space:]]*\"(.*)\"[[:space:]]*$ ]]; then
    case "${BASH_REMATCH[1]}" in
      scene) media_scene["$current"]="${BASH_REMATCH[2]}" ;;
      alt)
        media_alt["$current"]="${BASH_REMATCH[2]}"
        media_has_alt["$current"]=1
        ;;
    esac
  fi
done <"$manifest"

if [ "${#order[@]}" -eq 0 ]; then
  echo "media-references: $manifest declares nothing -- an empty manifest is not the same as a checked one" >&2
  exit 2
fi

# --- every directive names a declared id ------------------------------------------------------------

declare -A referenced=()
while IFS=: read -r page line text; do
  id="$(printf '%s' "$text" | sed -E 's/^[[:space:]]*<!--[[:space:]]*media:[[:space:]]*//; s/[[:space:]]*-->[[:space:]]*$//')"
  referenced["$id"]=1
  if [ -z "${media_scene[$id]+set}" ]; then
    report "$page:$line references media id \"$id\", which $manifest does not declare"
  fi
done < <(grep -rnE '^[[:space:]]*<!--[[:space:]]*media:[[:space:]]*[a-z0-9-]+[[:space:]]*-->[[:space:]]*$' \
  --include='*.md' "$docs" 2>/dev/null || true)

# --- every declared entry is referenced, has a scene, and describes itself --------------------------

for id in "${order[@]}"; do
  if [ -z "${referenced[$id]+set}" ]; then
    report "$manifest declares \"$id\", which no page references -- it would be captured on every publication and shown nowhere"
  fi

  scene="${media_scene[$id]}"
  if [ -z "$scene" ]; then
    report "$manifest declares \"$id\" with no scene -- nothing would produce it"
  elif [ ! -f "$scenes/$scene.sh" ]; then
    report "$manifest declares \"$id\" with scene \"$scene\", but $scenes/$scene.sh is not there"
  fi

  alt="${media_alt[$id]}"
  if [ -z "${media_has_alt[$id]+set}" ]; then
    report "$manifest declares \"$id\" with no alt -- a reader who cannot see it is told nothing was there"
  elif [ -z "${alt//[[:space:]]/}" ]; then
    report "$manifest declares \"$id\" with an empty alt -- that marks the picture decorative, which it is not"
  fi
done

if [ "$problems" -eq 0 ]; then
  printf 'media-references: %d entry(ies) in %s, each referenced, each with a scene and an alt\n' \
    "${#order[@]}" "$manifest"
  exit 0
fi
printf 'media-references: %d problem(s)\n' "$problems" >&2
exit 1
