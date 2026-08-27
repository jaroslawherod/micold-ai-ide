#!/usr/bin/env bash
# Asserts `site/stage.sh` builds the staging tree the site is rendered from (feature 028, T009).
#
# Staging exists so the published prose and the repository's prose are the same prose. The site
# needs three things the repository must not carry: a version number that changes every release,
# figure markup for media that does not exist until a publication produces it, and download links
# that name release assets. Writing any of those into `docs/` would put a release artefact into the
# source tree and make every page's history a list of version bumps.
#
# So the property under test is not "the output looks right" but "the input is untouched, and the
# output differs from it in exactly the declared ways".
#
# Fixture-driven, like `documentation-set.test.sh` beside it: the failures a check exists to catch
# are the ones no real tree contains, so the tree is built here rather than found.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

STAGE=site/stage.sh
failures=0

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  [ $# -gt 1 ] && printf '      %s\n' "$2"
  failures=$((failures + 1))
}

check() {
  local what="$1" want="$2" got="$3"
  if [ "$got" = "$want" ]; then pass "$what"; else fail "$what" "want=$want got=$got"; fi
}

contains() {
  local what="$1" file="$2" needle="$3"
  if grep -qF -- "$needle" "$file"; then pass "$what"; else fail "$what" "not in $file: $needle"; fi
}

absent() {
  local what="$1" file="$2" needle="$3"
  if grep -qF -- "$needle" "$file"; then fail "$what" "still in $file: $needle"; else pass "$what"; fi
}

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/docs/user-guide"

cat > "$fixture/docs/SUMMARY.md" <<'MD'
# Summary

[Home](README.md)

- [Deep page](user-guide/deep.md)
MD

cat > "$fixture/docs/README.md" <<'MD'
# Home

This is version {{MICOLD_VERSION}}, released as {{MICOLD_TAG}}.

<!-- media: probe-still -->

<!-- media: probe-clip -->

Trailing prose.
MD

cat > "$fixture/docs/user-guide/deep.md" <<'MD'
# Deep page

<!-- media: probe-still -->
MD

cat > "$fixture/media.toml" <<'MD'
[media.probe-still]
kind   = "still"
scene  = "probe"
scheme = "light"
alt    = "A probe still, described for a reader who cannot see it."
caption = "The probe still."

[media.probe-clip]
kind   = "clip"
scene  = "probe"
scheme = "light"
alt    = "A probe clip, described for a reader who cannot see it."
caption = "The probe clip."
MD

before="$(find "$fixture/docs" -type f -exec sha256sum {} + | sort)"

echo "== a clean stage =="
if "$STAGE" --docs "$fixture/docs" --media-manifest "$fixture/media.toml" \
   --out "$fixture/out" --version 9.9.9 --tag v9.9.9 > "$fixture/stage.log" 2>&1; then
  pass "stage.sh succeeds over the fixture"
else
  fail "stage.sh succeeds over the fixture" "$(tail -3 "$fixture/stage.log")"
fi

after="$(find "$fixture/docs" -type f -exec sha256sum {} + | sort)"
check "the source prose is not edited in place" "$before" "$after"

for page in README.md user-guide/deep.md SUMMARY.md; do
  if [ -f "$fixture/out/src/$page" ]; then
    pass "staged $page"
  else
    fail "staged $page" "missing from $fixture/out/src"
  fi
done

echo
echo "== substitution =="
contains "the version is substituted" "$fixture/out/src/README.md" "This is version 9.9.9"
contains "the tag is substituted" "$fixture/out/src/README.md" "released as v9.9.9"
absent "no placeholder survives" "$fixture/out/src/README.md" "{{MICOLD_VERSION}}"

echo
echo "== the media directive =="
absent "the directive line is consumed" "$fixture/out/src/README.md" "<!-- media: probe-still -->"
contains "a figure is emitted" "$fixture/out/src/README.md" '<figure class="media">'
contains "the still is referenced" "$fixture/out/src/README.md" 'src="media/probe-still.png"'
contains "alt text comes from the manifest" "$fixture/out/src/README.md" \
  'alt="A probe still, described for a reader who cannot see it."'
contains "the caption comes from the manifest" "$fixture/out/src/README.md" \
  "<figcaption>The probe still.</figcaption>"
contains "surrounding prose survives" "$fixture/out/src/README.md" "Trailing prose."
# A page one directory down resolves the same media through its own depth. `media/probe-still.png`
# on that page would resolve to `user-guide/media/…` and 404 on the published site, which is the
# kind of break that only shows up after deployment.
contains "a nested page reaches media through its own depth" \
  "$fixture/out/src/user-guide/deep.md" 'src="../media/probe-still.png"'

echo
echo "== a clip directive (FR-015a, FR-015b, FR-028) =="
# A clip is the one figure that can *do* something on its own, and both of the things it must not do
# are single attributes. `autoplay` starts motion nobody asked for; a preload that fetches downloads
# a video to a reader who never presses play, on a connection that may be paying for it. The check
# in `site/checks/page-checks.mjs` catches either in the rendered HTML -- this catches them one step
# earlier, in the markup this script writes, where the attribute list is actually decided.
contains "a clip is a video element" "$fixture/out/src/README.md" "<video controls loop muted playsinline"
contains "nothing is fetched before the reader presses play" "$fixture/out/src/README.md" \
  'preload="none"'
absent "a clip never plays by itself" "$fixture/out/src/README.md" "autoplay"
contains "the poster is the clip's own first frame" "$fixture/out/src/README.md" \
  'poster="media/probe-clip.png"'
# Two sources, WebM first: a browser takes the first type it can decode, and the VP9 is the smaller
# of the two on flat interface frames (FR-015c).
contains "the WebM is offered first" "$fixture/out/src/README.md" \
  '<source src="media/probe-clip.webm" type="video/webm">'
contains "the H.264 is offered as well" "$fixture/out/src/README.md" \
  '<source src="media/probe-clip.mp4" type="video/mp4">'
# A video has no `alt`, so the description the manifest carries has to arrive as a label instead --
# without it the clip is unannounced to a reader using a screen reader (FR-014).
contains "the description reaches a screen reader" "$fixture/out/src/README.md" \
  'aria-label="A probe clip, described for a reader who cannot see it."'
contains "the clip's caption comes from the manifest" "$fixture/out/src/README.md" \
  "<figcaption>The probe clip.</figcaption>"

echo
echo "== a directive naming an id the manifest does not declare =="
cat > "$fixture/docs/user-guide/deep.md" <<'MD'
# Deep page

<!-- media: not-declared -->
MD
if "$STAGE" --docs "$fixture/docs" --media-manifest "$fixture/media.toml" \
   --out "$fixture/out" --version 9.9.9 --tag v9.9.9 > "$fixture/undeclared.log" 2>&1; then
  fail "an undeclared id fails the stage" "stage.sh exited 0"
else
  pass "an undeclared id fails the stage"
fi
if grep -qF "not-declared" "$fixture/undeclared.log"; then
  pass "the failure names the id"
else
  fail "the failure names the id" "$(tail -3 "$fixture/undeclared.log")"
fi

echo
echo "== the download links on the install page (FR-004a) =="
# A download link is the one thing on the site that can be *correct prose about the wrong file*: it
# reads perfectly, resolves to a 404, and nothing about the page says so. The release's own asset
# list is the only thing that settles it, so staging is given that list and holds the page to it.
cat > "$fixture/docs/user-guide/deep.md" <<'MD'
# Deep page

[the package](https://github.com/Cumulocity-IoT/micold-ai-ide/releases/download/{{MICOLD_TAG}}/micold-client_{{MICOLD_VERSION}}-1_amd64.deb)
MD

if MICOLD_RELEASE_ASSETS="micold-client_9.9.9-1_amd64.deb micold-client_9.9.9-1_arm64.deb" \
   "$STAGE" --docs "$fixture/docs" --media-manifest "$fixture/media.toml" \
   --out "$fixture/out" --version 9.9.9 --tag v9.9.9 > "$fixture/assets.log" 2>&1; then
  pass "a link naming an asset the release carries stages"
else
  fail "a link naming an asset the release carries stages" "$(tail -3 "$fixture/assets.log")"
fi

if MICOLD_RELEASE_ASSETS="micold-client_9.9.9-1_arm64.deb" \
   "$STAGE" --docs "$fixture/docs" --media-manifest "$fixture/media.toml" \
   --out "$fixture/out" --version 9.9.9 --tag v9.9.9 > "$fixture/missing-asset.log" 2>&1; then
  fail "a link naming an asset the release does not carry fails the stage" "stage.sh exited 0"
else
  pass "a link naming an asset the release does not carry fails the stage"
fi
if grep -qF "micold-client_9.9.9-1_amd64.deb" "$fixture/missing-asset.log"; then
  pass "the failure names the file"
else
  fail "the failure names the file" "$(tail -3 "$fixture/missing-asset.log")"
fi

# A link into a *different* tag's downloads is the other half of the same mistake: it points at a
# real file, and at the wrong release -- so a reader on the current version's page downloads an old
# one and nothing tells either of them.
cat > "$fixture/docs/user-guide/deep.md" <<'MD'
# Deep page

[the package](https://github.com/Cumulocity-IoT/micold-ai-ide/releases/download/v1.0.0/micold-client_9.9.9-1_amd64.deb)
MD
if MICOLD_RELEASE_ASSETS="micold-client_9.9.9-1_amd64.deb" \
   "$STAGE" --docs "$fixture/docs" --media-manifest "$fixture/media.toml" \
   --out "$fixture/out" --version 9.9.9 --tag v9.9.9 > "$fixture/wrong-tag.log" 2>&1; then
  fail "a link into another release's downloads fails the stage" "stage.sh exited 0"
else
  pass "a link into another release's downloads fails the stage"
fi

echo
if [ "$failures" -eq 0 ]; then
  echo "site/stage.sh: all assertions hold"
else
  echo "site/stage.sh: $failures assertion(s) failed"
  exit 1
fi
