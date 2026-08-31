#!/usr/bin/env bash
# Asserts `site/checks/media-budget.sh` catches what it exists to catch (feature 028, T053).
#
# The budget exists because the pictures are the point of this site and the pictures are what makes
# a page slow. A guide read over a hotel connection is read one page at a time, so the limit is per
# page and not per site: 1 MB of stills on any one page, and 3 MB for any single clip (SC-012).
#
# The rule the check must not break is that it never fixes anything. Downscaling on the author's
# behalf is the obvious convenience and the wrong one: the picture that reaches the reader would
# then differ from the picture the capture produced, nobody would have looked at the difference,
# and the failure the budget exists to surface -- a scene that got heavier -- would be papered over
# on every publication instead of once, loudly. So the last assertion here reads the fixture files
# back after a failing run and requires them byte-for-byte unchanged.
#
# Sizes are chosen to be unambiguous either side of the MB/MiB question: nothing here is within a
# factor of 1.05 of a limit, so no assertion turns on which of the two the check counts in.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

CHECK=site/checks/media-budget.sh
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
    fail "$what" "the check passed a site it must refuse"
  elif grep -qF -- "$needle" "$work/out"; then
    pass "$what"
  else
    fail "$what" "failed, but did not name \"$needle\": $(tail -8 "$work/out")"
  fi
}

# --- the fixtures ---------------------------------------------------------------------------------
#
# Rendered HTML in the shape `site/stage.sh` produces: a still is a `<figure class="media">` around
# an `<img>`, a clip is the same figure around a `<video>` with a poster and two `<source>`s. Pages
# one directory down reach the media through their own depth, which is why one fixture is nested --
# a check that only understood root-level paths would pass it for the wrong reason.

bytes() { head -c "$1" /dev/zero >"$2"; }

still_page() {
  # $1 = file, $2 = prefix to the site root, $3.. = media ids
  local file="$1" prefix="$2"
  shift 2
  {
    printf '<!doctype html><html lang="en"><head><meta charset="utf-8"><title>A page</title></head><body>\n'
    local id
    for id in "$@"; do
      printf '<figure class="media"><img src="%smedia/%s.png" alt="A picture of %s" loading="lazy"></figure>\n' \
        "$prefix" "$id" "$id"
    done
    printf '</body></html>\n'
  } >"$file"
}

clip_page() {
  # $1 = file, $2 = prefix, $3 = id
  cat >"$1" <<HTML
<!doctype html><html lang="en"><head><meta charset="utf-8"><title>A clip</title></head><body>
<figure class="media"><video controls loop muted playsinline preload="none" poster="$2media/$3.png" aria-label="A clip of $3">
<source src="$2media/$3.webm" type="video/webm">
<source src="$2media/$3.mp4" type="video/mp4">
</video></figure>
</body></html>
HTML
}

build_site() {
  # A whole site per assertion: cheap, and it keeps each one readable on its own.
  local dir="$1"
  rm -rf "$dir"
  mkdir -p "$dir/media" "$dir/user-guide"
  bytes 500000 "$dir/media/main-window.png"
  bytes 400000 "$dir/media/sidebar.png"
  bytes 300000 "$dir/media/new-worktree.png"
  bytes 2000000 "$dir/media/new-worktree.webm"
  bytes 1800000 "$dir/media/new-worktree.mp4"
  still_page "$dir/index.html" "" main-window sidebar
  clip_page "$dir/user-guide/worktrees.html" "../" new-worktree
}

printf '== the budgets (FR-015c, SC-012) ==\n'

build_site "$work/site"
expect_pass "a site inside both budgets passes" "$work/site"

build_site "$work/site"
bytes 900000 "$work/site/media/sidebar.png"
expect_fail "a page whose stills total more than 1 MB fails" "index.html" "$work/site"

# The report has to be actionable on its own: which page, which files, and how much. An author who
# is told only "too big" has to re-derive all three before they can act.
build_site "$work/site"
bytes 900000 "$work/site/media/sidebar.png"
run "$work/site" || true
for needle in "main-window.png" "sidebar.png"; do
  if grep -qF -- "$needle" "$work/out"; then
    pass "the report names $needle"
  else
    fail "the report names $needle" "$(tail -8 "$work/out")"
  fi
done
if grep -qE '1\.[34]|140[0-9]{4}' "$work/out"; then
  pass "the report gives the page's total"
else
  fail "the report gives the page's total" "$(tail -8 "$work/out")"
fi

build_site "$work/site"
bytes 4000000 "$work/site/media/new-worktree.webm"
expect_fail "a clip file over 3 MB fails" "new-worktree.webm" "$work/site"

build_site "$work/site"
bytes 4000000 "$work/site/media/new-worktree.mp4"
expect_fail "the second encoding of the same clip is measured too" "new-worktree.mp4" "$work/site"

# A poster is a still: it is what the reader downloads before deciding to play anything, so it is
# spent from the still budget of the page it is on rather than from the clip's 3 MB.
build_site "$work/site"
bytes 1400000 "$work/site/media/new-worktree.png"
expect_fail "a clip poster counts against the page's still budget" "new-worktree.png" "$work/site"

printf '== the budget is per page (SC-012) ==\n'
build_site "$work/site"
bytes 900000 "$work/site/media/main-window.png"
still_page "$work/site/user-guide/settings.html" "../" main-window
# index.html is now 1.3 MB and must fail; settings.html shows the same 900 KB picture and must not.
run "$work/site" || true
if grep -qF "index.html" "$work/out" && ! grep -qF "settings.html" "$work/out"; then
  pass "a shared picture is counted against each page separately, not summed across the site"
else
  fail "a shared picture is counted against each page separately, not summed across the site" \
    "$(tail -8 "$work/out")"
fi

printf '== nothing is shipped that no page asks for ==\n'
#
# The capture step leaves its intermediates beside its output -- a clip is encoded from a directory
# of frame PNGs, and that directory sits in the same staged media directory the renderer copies
# wholesale into the built site. Every one of those frames is already inside the .webm and the .mp4;
# published, they are megabytes the deploy carries, the reader never downloads and nobody notices,
# because no page links to them and so no per-page budget above ever counts them.
build_site "$work/site"
mkdir -p "$work/site/media/new-worktree.frames"
bytes 300000 "$work/site/media/new-worktree.frames/frame-0001.png"
bytes 300000 "$work/site/media/new-worktree.frames/frame-0002.png"
expect_fail "a media file no page references fails" "new-worktree.frames/frame-0001.png" \
  "$work/site"

# The control the rule above could pass for the wrong reason: every fixture asset IS referenced, so
# a check that simply reported everything under media/ would fail the clean site too.
build_site "$work/site"
expect_pass "a site whose every media file is referenced still passes" "$work/site"

printf '== nothing is repaired (FR-015c) ==\n'
build_site "$work/site"
bytes 900000 "$work/site/media/sidebar.png"
bytes 4000000 "$work/site/media/new-worktree.webm"
before="$(find "$work/site/media" -type f -exec sha256sum {} + | sort)"
run "$work/site" || true
after="$(find "$work/site/media" -type f -exec sha256sum {} + | sort)"
if [ "$before" = "$after" ]; then
  pass "a failing run leaves every asset byte-for-byte as it found it"
else
  fail "a failing run leaves every asset byte-for-byte as it found it" \
    "$(diff <(printf '%s\n' "$before") <(printf '%s\n' "$after") | head -6)"
fi

printf '== usage ==\n'
expect_fail "a directory that is not there fails rather than passing an empty site" "$work/nowhere" \
  "$work/nowhere"

printf '\n'
if [ "$failures" -eq 0 ]; then
  printf 'the media-budget checks: all assertions hold\n'
else
  printf '%d assertion(s) failed\n' "$failures"
fi
exit "$([ "$failures" -eq 0 ] && echo 0 || echo 1)"
