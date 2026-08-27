#!/usr/bin/env bash
# Produce every image the manifest declares (feature 028, T022).
#
#   site/capture/capture.sh [--out DIR] [--manifest FILE] [--only ID] [--list]
#
# `site/media.toml` is the list of what the site shows; this is what makes those files exist. The
# manifest maps each id to a scene and a scheme, so the run order here is by *scene*, not by id: a
# scene launches the application, which takes seconds, and several ids usually come from one screen.
# Each scene is therefore run once per scheme it is asked for, and writes every id it owns.
#
# The last step is the one that matters. A capture run that quietly produced eleven of twelve images
# leaves the twelfth figure pointing at a file that is not there -- a broken image on a published
# page, which nobody sees until a reader does. So every declared id is checked for afterwards, and a
# missing one fails the run (FR-011a, SC-004).

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
manifest="$root/site/media.toml"
out="$root/site/build/src/media"
only=""
list=0

die() {
  printf 'capture.sh: %s\n' "$1" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --manifest) manifest="$2"; shift 2 ;;
    --only) only="$2"; shift 2 ;;
    --list) list=1; shift ;;
    -h | --help) sed -n '2,17p' "$0"; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -f "$manifest" ] || die "no such media manifest: $manifest"

# The same deliberately small TOML reader `stage.sh` uses -- `[media.<id>]` tables of quoted
# scalars. Two readers of one file shape is the price of not adding a TOML dependency to a shell
# pipeline; `contracts/media-manifest.md` is the shape both of them implement.
declare -A media_kind media_scene media_scheme
ids=()
current=""
while IFS= read -r line || [ -n "$line" ]; do
  line="${line%$'\r'}"
  case "$line" in
    \#*) continue ;;
  esac
  if [[ "$line" =~ ^\[media\.([a-z0-9]+(-[a-z0-9]+)*)\][[:space:]]*$ ]]; then
    current="${BASH_REMATCH[1]}"
    ids+=("$current")
    media_kind["$current"]="still"
    media_scene["$current"]=""
    media_scheme["$current"]="light"
    continue
  fi
  [ -n "$current" ] || continue
  if [[ "$line" =~ ^[[:space:]]*([a-z_]+)[[:space:]]*=[[:space:]]*\"(.*)\"[[:space:]]*$ ]]; then
    case "${BASH_REMATCH[1]}" in
      kind) media_kind["$current"]="${BASH_REMATCH[2]}" ;;
      scene) media_scene["$current"]="${BASH_REMATCH[2]}" ;;
      scheme) media_scheme["$current"]="${BASH_REMATCH[2]}" ;;
    esac
  fi
done < "$manifest"

if [ -n "$only" ]; then
  [ -n "${media_kind[$only]+set}" ] || die "$manifest does not declare \"$only\""
  ids=("$only")
fi

if [ "$list" -eq 1 ]; then
  for id in ${ids[@]+"${ids[@]}"}; do
    printf '%s\t%s\t%s\t%s\n' "$id" "${media_kind[$id]}" "${media_scene[$id]}" "${media_scheme[$id]}"
  done
  exit 0
fi

# A manifest with no entries yet is not an error: the entries arrive with the pages that reference
# them, and until then the site builds and renders with no media at all.
if [ "${#ids[@]}" -eq 0 ]; then
  printf 'capture.sh: %s declares no media -- nothing to capture\n' "$manifest" >&2
  exit 0
fi

# --- the run order -------------------------------------------------------------------------------

runs=()
for id in "${ids[@]}"; do
  scene="${media_scene[$id]}"
  [ -n "$scene" ] || die "\"$id\" declares no scene"
  [ -f "$root/site/capture/scenes/$scene.sh" ] || die "\"$id\" names a scene that does not exist: site/capture/scenes/$scene.sh"
  case "${media_scheme[$id]}" in light | dark) ;; *) die "\"$id\" declares an unknown scheme: ${media_scheme[$id]}" ;; esac
  key="$scene:${media_scheme[$id]}"
  case " ${runs[*]-} " in *" $key "*) ;; *) runs+=("$key") ;; esac
done

mkdir -p "$out"

# One display for the whole run. Starting it per scene would work and would cost a second each time;
# the reason it is shared is that the state directory beneath it is where the pinned binaries live,
# and building them once is the point.
env_file="$("$root/site/capture/display.sh" start)"
# shellcheck disable=SC1090
. "$env_file"
trap '"$root/site/capture/display.sh" stop >/dev/null 2>&1 || true' EXIT

failed=()
for run in "${runs[@]}"; do
  scene="${run%:*}"
  scheme="${run##*:}"
  printf 'capture.sh: %s (%s)\n' "$scene" "$scheme" >&2
  if ! "$root/site/capture/scenes/$scene.sh" --out "$out" --scheme "$scheme"; then
    failed+=("$scene ($scheme)")
  fi
done

# --- what the manifest promised ------------------------------------------------------------------

missing=()
for id in "${ids[@]}"; do
  [ -f "$out/$id.png" ] || { missing+=("$id.png"); continue; }
  # A clip's poster is its first frame, so the still above is necessary but not sufficient: the
  # frames the encoder turns into the clip have to be there too.
  if [ "${media_kind[$id]}" = "clip" ]; then
    if [ ! -d "$out/$id.frames" ] || [ -z "$(ls -A "$out/$id.frames" 2>/dev/null)" ]; then
      missing+=("$id.frames/")
    fi
  fi
done

if [ "${#failed[@]}" -ne 0 ]; then
  printf 'capture.sh: scene failed: %s\n' "${failed[@]}" >&2
fi
if [ "${#missing[@]}" -ne 0 ]; then
  printf 'capture.sh: declared but not produced: %s\n' "${missing[@]}" >&2
fi
if [ "${#failed[@]}" -ne 0 ] || [ "${#missing[@]}" -ne 0 ]; then
  exit 1
fi

printf 'capture.sh: %d media in %s\n' "${#ids[@]}" "$out" >&2
