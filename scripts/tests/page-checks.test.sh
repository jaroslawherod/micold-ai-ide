#!/usr/bin/env bash
# Asserts `site/checks/page-checks.mjs` catches what it exists to catch (feature 028, T024/T025/T039).
#
# This is the only check in the publication that looks at a *rendered* page. Everything else --
# the token contrast test, the staging test, the link check -- reads text. Rendered contrast, a
# reachable name, whether the install link is on the first screen, whether every page can be reached
# from every other, what the site's own search box answers, and whether the browser went to another
# host to draw the page are all properties that only exist once a browser has run the CSS and the
# JavaScript, so they are checked in a browser or not at all.
#
# The fixtures are pages that fail: a home page whose install link sits below the fold, a page that
# fetches a stylesheet from another host, a page with an unnamed image and unreadable text, a
# navigation that buries a page three steps down, and a search box that answers with the wrong page.
# A check that passes everything looks exactly like a check that works, so the test asserts on the
# failures and keeps one passing fixture as the control.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

CHECKS=site/checks/page-checks.mjs
failures=0

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  [ $# -gt 1 ] && printf '      %s\n' "$2"
  failures=$((failures + 1))
}

# The check runs on Node, which `mise.toml` pins for exactly this reason. A machine without it is
# not a machine where the check passed -- it is one where it did not run, and the two must not look
# alike from the outside.
node_cmd=(node)
if ! command -v node >/dev/null 2>&1; then
  if command -v mise >/dev/null 2>&1 && mise exec node@20 -- node --version >/dev/null 2>&1; then
    node_cmd=(mise exec node@20 -- node)
  else
    printf 'page-checks: no Node 20 -- run `mise install node@20` (the check cannot be run without it)\n' >&2
    exit 1
  fi
fi

if [ ! -d site/checks/node_modules ]; then
  printf 'page-checks: site/checks/node_modules is missing -- run `npm install` in site/checks\n' >&2
  exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# --- the fixtures ---------------------------------------------------------------------------------
#
# Deliberately plain HTML rather than a built book: the check reads a rendered page, and the smallest
# page that renders is the one where a failure can only be the fixture's own.
#
# Every page carries the two pieces of furniture the real site has on every page -- the navigation
# and the search box -- because two of the assertions read exactly those. A fixture without them is
# not a smaller site, it is a different one. The ids are mdBook's own (`mdbook-sidebar`,
# `mdbook-searchbar`, `mdbook-searchresults`): the check has to find them on the real site, so the
# fixtures must not offer it an easier target than the thing it is checking.

page_head() {
  cat <<'HTML'
<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Micold AI IDE</title>
<style>
  :root { color-scheme: light; }
  body { background: #ffffff; color: #1a1c1e; font-family: sans-serif; margin: 0; padding: 16px; }
  a { color: #14468c; }
  nav ul { list-style: none; padding: 0; display: flex; gap: 16px; }
  #mdbook-searchresults { list-style: none; padding: 0; }
</style>
</head><body>
HTML
}

# The navigation every control page carries: one link per page, so every page is one step from every
# other. `$1` is the path back to the site root ("" at the root, "../" one level down).
page_nav() {
  local up="$1"
  cat <<HTML
<nav id="mdbook-sidebar" aria-label="Site navigation"><ul class="chapter">
<li><a href="${up}index.html">Micold AI IDE</a></li>
<li><a href="${up}install.html">Install</a></li>
<li><a href="${up}user-guide/index.html">User guide</a></li>
</ul></nav>
HTML
}

# The search box, and a search that actually answers. `$1` is the path back to the root, `$2` is an
# optional line of JavaScript that rearranges the hits -- which is how the failing fixture is built:
# the box works, the index is right, and the wrong page is put first.
page_search() {
  local up="$1" rearrange="${2:-}" extra="${3:-}"
  cat <<HTML
<form role="search"><input type="search" id="mdbook-searchbar" aria-label="Search this book"></form>
<ul id="mdbook-searchresults"></ul>
<script>
(function () {
  var index = [
    { title: 'Micold AI IDE', href: '${up}index.html' },
    { title: 'Install', href: '${up}install.html' },
    { title: 'User guide', href: '${up}user-guide/index.html' }${extra}
  ];
  var bar = document.getElementById('mdbook-searchbar');
  var out = document.getElementById('mdbook-searchresults');
  bar.addEventListener('input', function () {
    var q = bar.value.trim().toLowerCase();
    var hits = q === '' ? [] : index.filter(function (e) {
      return e.title.toLowerCase().indexOf(q) !== -1;
    });
    ${rearrange}
    out.innerHTML = hits.map(function (e) {
      return '<li><a href="' + e.href + '">' + e.title + '</a></li>';
    }).join('');
  });
})();
</script>
HTML
}

# The control. Everything the home page has to have, on the first screen, with nothing fetched from
# anywhere else.
mkdir -p "$work/good/user-guide"
{
  page_head
  page_nav ""
  page_search ""
  cat <<'HTML'
<h1>Micold AI IDE</h1>
<p>A desktop workbench for running AI coding sessions across several git worktrees at once.</p>
<p><a href="install.html">Install</a> &middot; <a href="user-guide/index.html">User guide</a></p>
<img src="shot.png" alt="The application with a project open" width="40" height="30">
</body></html>
HTML
} > "$work/good/index.html"
{ page_head; page_nav "../"; page_search "../"
  printf '<h1>User guide</h1><p>Body.</p></body></html>\n'; } > "$work/good/user-guide/index.html"
{ page_head; page_nav ""; page_search ""
  printf '<h1>Install</h1><p>Body.</p></body></html>\n'; } > "$work/good/install.html"
printf 'x' > "$work/good/shot.png"

# Below the fold. The install link is real, reachable and correct -- and a visitor never sees it,
# which is the whole of SC-001.
#
# The sidebar here is closed, the state mdBook renders it in until the reader opens it. That is the
# only honest way to write this fixture: an open sidebar lists the install page *on the first screen*
# and the reader has seen the link, wherever the body puts it. So the page that fails is the one
# where neither the sidebar nor the first screen of the body offers the link.
cp -R "$work/good" "$work/below-fold"
{
  page_head
  page_nav "" | sed 's/ aria-label="Site navigation">/ aria-label="Site navigation" hidden>/'
  page_search ""
  cat <<'HTML'
<h1>Micold AI IDE</h1>
<p>A desktop workbench for running AI coding sessions across several git worktrees at once.</p>
<img src="shot.png" alt="The application with a project open" width="40" height="30">
<div style="height: 2400px"></div>
<p><a href="install.html">Install</a> &middot; <a href="user-guide/index.html">User guide</a></p>
</body></html>
HTML
} > "$work/below-fold/index.html"

# Another host. One line in a stylesheet is all it takes to send every reader's address to a third
# party and to make the page read differently when that third party is unreachable.
cp -R "$work/good" "$work/off-origin"
{
  page_head
  page_nav ""
  page_search ""
  cat <<'HTML'
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Roboto&display=swap">
<h1>Micold AI IDE</h1>
<p>A desktop workbench for running AI coding sessions across several git worktrees at once.</p>
<p><a href="install.html">Install</a> &middot; <a href="user-guide/index.html">User guide</a></p>
</body></html>
HTML
} > "$work/off-origin/index.html"

# Unnamed and unreadable. An image with no alternative text and body text at 1.8:1 -- both of them
# things the markup alone cannot settle, because the contrast is the rendered result of two rules.
cp -R "$work/good" "$work/inaccessible"
{
  page_head
  page_nav "../"
  page_search "../"
  cat <<'HTML'
<h1>User guide</h1>
<p style="color:#bfc4c9; background:#ffffff">The worktree list shows every session.</p>
<img src="../shot.png" width="40" height="30">
</body></html>
HTML
} > "$work/inaccessible/user-guide/index.html"

# A navigation that is a corridor rather than a map: the home page opens onto a chain of pages, each
# of which links on to the next and back to the home page, and to nothing else. Every page is
# reachable -- follow it far enough -- and the last one is three steps from the front door, which is
# the distance at which a reader stops following (FR-023a, SC-006).
#
# It is the control site with the chain added, so the home page still has everything the first-screen
# assertion asks for and the search still answers for every page: the *only* thing wrong here is how
# far apart the pages are.
cp -R "$work/good" "$work/deep-nav"
# The three extra pages, as the search index sees them. `$1` is the path back to the site root, so
# the entries read the same from every page -- the same shape the control's own index has.
chain_entries() {
  local up="$1"
  cat <<HTML
,
    { title: 'Alpha', href: '${up}alpha.html' },
    { title: 'Beta', href: '${up}beta.html' },
    { title: 'Gamma', href: '${up}gamma.html' }
HTML
}

chain_page() {
  local file="$1" title="$2" onward="$3" onward_title="$4"
  {
    page_head
    cat <<HTML
<nav id="mdbook-sidebar" aria-label="Site navigation"><ul class="chapter">
<li><a href="index.html">Micold AI IDE</a></li>
<li><a href="${onward}">${onward_title}</a></li>
</ul></nav>
HTML
    page_search "" "" "$(chain_entries "")"
    printf '<h1>%s</h1><p>Body.</p></body></html>\n' "$title"
  } > "$work/deep-nav/$file"
}
chain_page alpha.html Alpha beta.html Beta
chain_page beta.html Beta gamma.html Gamma
chain_page gamma.html Gamma index.html "Micold AI IDE"
{
  page_head
  cat <<'HTML'
<nav id="mdbook-sidebar" aria-label="Site navigation"><ul class="chapter">
<li><a href="index.html">Micold AI IDE</a></li>
<li><a href="install.html">Install</a></li>
<li><a href="user-guide/index.html">User guide</a></li>
<li><a href="alpha.html">Alpha</a></li>
</ul></nav>
HTML
  page_search "" "" "$(chain_entries "")"
  cat <<'HTML'
<h1>Micold AI IDE</h1>
<p>A desktop workbench for running AI coding sessions across several git worktrees at once.</p>
<p><a href="install.html">Install</a> &middot; <a href="user-guide/index.html">User guide</a></p>
<img src="shot.png" alt="The application with a project open" width="40" height="30">
</body></html>
HTML
} > "$work/deep-nav/index.html"
{ page_head; page_nav "../"; page_search "../" "" "$(chain_entries "../")"
  printf '<h1>User guide</h1><p>Body.</p></body></html>\n'; } > "$work/deep-nav/user-guide/index.html"
{ page_head; page_nav ""; page_search "" "" "$(chain_entries "")"
  printf '<h1>Install</h1><p>Body.</p></body></html>\n'; } > "$work/deep-nav/install.html"

# A search box that answers, promptly and confidently, with the wrong page. The commonest way a
# documentation search fails is not an error -- it is a plausible first result that is not the page
# the reader asked for, and only a query whose right answer is known can tell the two apart.
cp -R "$work/good" "$work/bad-search"
{
  page_head
  page_nav ""
  page_search "" "hits = [index[1]].concat(hits.filter(function (e) { return e.href !== index[1].href; }));"
  cat <<'HTML'
<h1>Micold AI IDE</h1>
<p>A desktop workbench for running AI coding sessions across several git worktrees at once.</p>
<p><a href="install.html">Install</a> &middot; <a href="user-guide/index.html">User guide</a></p>
<img src="shot.png" alt="The application with a project open" width="40" height="30">
</body></html>
HTML
} > "$work/bad-search/index.html"

# --- clips, and what a page may not do with one (feature 028, T066) --------------------------------
#
# A clip on this site is a poster with a play control: it holds still until the reader presses it
# (FR-015a) and fetches no video bytes before that (FR-028). Both are one attribute away from being
# wrong, and neither shows up in a link check, a budget check or a contrast check -- the page is
# perfectly valid, it just starts moving at a reader who did not ask.
#
# So there are three fixtures: the figure written correctly, the same figure with `autoplay`, and the
# same figure with a preload that fetches. The correct one is the control, because a check that
# refused every video would pass this test and take the clips off the site.

clip_figure() {
  # $1 = the attributes under test, $2 = the path back to the site root
  local attrs="$1" up="${2:-}"
  cat <<HTML
<figure class="media"><video controls loop muted playsinline ${attrs}
  poster="${up}media/open-project.png" aria-label="A project opening in the main area" width="320" height="200">
  <source src="${up}media/open-project.webm" type="video/webm">
  <source src="${up}media/open-project.mp4" type="video/mp4">
</video><figcaption>Opening a project.</figcaption></figure>
HTML
}

clip_site() {
  # $1 = the fixture directory, $2 = the attributes on the <video>
  cp -R "$work/good" "$1"
  mkdir -p "$1/media"
  # Real files, so that a fetch the page should not make is a fetch that succeeds. A 404 would fail
  # the page for a reason that is not the one under test.
  cp "$work/good/shot.png" "$1/media/open-project.png"
  printf 'x' > "$1/media/open-project.webm"
  printf 'x' > "$1/media/open-project.mp4"
  {
    page_head
    page_nav "../"
    page_search "../"
    printf '<h1>User guide</h1><p>Opening a project shows its worktrees.</p>\n'
    clip_figure "$2" "../"
    printf '</body></html>\n'
  } > "$1/user-guide/index.html"
}

clip_site "$work/clip-good" 'preload="none"'
clip_site "$work/clip-autoplay" 'autoplay preload="none"'
clip_site "$work/clip-preload" 'preload="auto"'

run() {
  local dir="$1"
  shift
  "${node_cmd[@]}" "$CHECKS" --site "$dir" "$@" >"$work/out" 2>&1
}

expect_pass() {
  local what="$1" dir="$2"
  if run "$dir"; then pass "$what"; else fail "$what" "$(tail -5 "$work/out")"; fi
}

expect_fail() {
  local what="$1" dir="$2" needle="$3"
  if run "$dir"; then
    fail "$what" "the check passed a page it must refuse"
  elif grep -qiF -- "$needle" "$work/out"; then
    pass "$what"
  else
    fail "$what" "failed, but did not name \"$needle\": $(tail -5 "$work/out")"
  fi
}

printf '== the control ==\n'
expect_pass "a home page with everything on the first screen passes" "$work/good"

printf '== the first screen (FR-023a, SC-001) ==\n'
expect_fail "an install link below the fold fails the home page" "$work/below-fold" "install"

printf '== off-origin (FR-027a, SC-015) ==\n'
expect_fail "a stylesheet from another host fails" "$work/off-origin" "fonts.googleapis.com"

printf '== WCAG 2.2 AA (FR-031, SC-013) ==\n'
expect_fail "an image with no alternative text fails" "$work/inaccessible" "image-alt"
expect_fail "text that cannot be read against its background fails" "$work/inaccessible" "color-contrast"

printf '== the navigation (FR-023a, SC-006) ==\n'
expect_fail "a page three steps down the navigation fails" "$work/deep-nav" "gamma.html"

printf '== search (FR-026, SC-006) ==\n'
expect_fail "a search that answers with the wrong page first fails" "$work/bad-search" "user guide"

printf '== clips hold still until the reader asks (FR-015a, FR-028, SC-011) ==\n'
expect_pass "a clip with a play control and preload=\"none\" passes" "$work/clip-good"
expect_fail "a clip that plays on its own fails" "$work/clip-autoplay" "autoplay"
expect_fail "a clip that fetches its video before the reader asks fails" "$work/clip-preload" "preload"

printf '\n'
if [ "$failures" -eq 0 ]; then
  printf 'the rendered-page checks: all assertions hold\n'
else
  printf '%d assertion(s) failed\n' "$failures"
fi
exit "$([ "$failures" -eq 0 ] && echo 0 || echo 1)"
