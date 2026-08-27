#!/usr/bin/env bash
# Asserts `site/checks/links.sh` catches what it exists to catch (feature 028, T038).
#
# The check runs twice in a publication and the two runs are not the same check. `--sources` reads
# the Markdown before a merge, where a broken link is cheap to fix and the author is still looking
# at it; `--built <dir>` reads the rendered HTML before a deploy, where the failure modes are the
# renderer's own -- a chapter that never made it into `SUMMARY.md`, a heading whose generated id is
# not the one an author guessed. A tree can pass one and fail the other, so both are driven here.
#
# The third assertion is the one that is easy to lose: no external URL is ever fetched. A
# publication that fails because somebody else's site is down is a publication nobody trusts, and
# the failure arrives months after the link was written. Every fixture below therefore carries a
# link to a host that *cannot* resolve -- `.invalid` is reserved for exactly this (RFC 2606) -- and
# the control asserts the check passes anyway. If the check ever starts reaching the network, that
# assertion fails on the first run rather than on the first outage.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

CHECK=site/checks/links.sh
failures=0

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  [ $# -gt 1 ] && printf '      %s\n' "$2"
  failures=$((failures + 1))
}

if ! command -v lychee >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/lychee" ]; then
  printf 'links: lychee is not installed -- run `cargo install lychee` (the check cannot be run without it)\n' >&2
  exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# --- the source fixtures --------------------------------------------------------------------------
#
# Shaped like `docs/`: a small set of Markdown pages that link to each other by relative path, the
# way a documentation set does. Nothing here is a book -- the source check runs before anything is
# rendered, which is the point of having it.

mkdir -p "$work/src-good"
cat >"$work/src-good/index.md" <<'MD'
# The guide

Start with [installing it](install.md), then read about [worktrees](guide/worktrees.md).
The [session section](guide/worktrees.md#sessions) covers the terminal.

The project is discussed [elsewhere](https://micold.invalid/forum) as well.
MD
cat >"$work/src-good/install.md" <<'MD'
# Installing

Back to [the guide](index.md).
MD
mkdir -p "$work/src-good/guide"
cat >"$work/src-good/guide/worktrees.md" <<'MD'
# Worktrees

## Sessions

A session runs in a worktree. See [installing it](../install.md).
MD

# A link to a page that is not there. The commonest way a documentation set breaks: the page was
# renamed and the six links to it were not.
cp -R "$work/src-good" "$work/src-missing-page"
cat >"$work/src-missing-page/index.md" <<'MD'
# The guide

Start with [installing it](installation.md).

The project is discussed [elsewhere](https://micold.invalid/forum) as well.
MD

# A fragment that names no heading. The link resolves, the page opens, and the reader lands at the
# top of it wondering which part they were sent to -- which is why a link check that stops at the
# file name is only half a check.
cp -R "$work/src-good" "$work/src-missing-fragment"
cat >"$work/src-missing-fragment/index.md" <<'MD'
# The guide

The [session section](guide/worktrees.md#running-a-session) covers the terminal.

The project is discussed [elsewhere](https://micold.invalid/forum) as well.
MD

# --- the built fixtures ---------------------------------------------------------------------------
#
# The same three shapes in rendered HTML, because the renderer is between the author and the reader
# and it has failure modes of its own.

built_page() {
  # $1 = title, $2 = body, $3 = the path back to the site root ("" at the root, "../" one level down)
  cat <<HTML
<!doctype html><html lang="en"><head><meta charset="utf-8"><title>$1</title></head>
<body><nav><a href="${3:-}index.html">The guide</a></nav>
$2
</body></html>
HTML
}

mkdir -p "$work/built-good/guide"
built_page "The guide" '<h1 id="the-guide">The guide</h1>
<p><a href="install.html">Installing</a>, <a href="guide/worktrees.html">worktrees</a>,
the <a href="guide/worktrees.html#sessions">session section</a>,
and <a href="https://micold.invalid/forum">elsewhere</a>.</p>' >"$work/built-good/index.html"
built_page "Installing" '<h1 id="installing">Installing</h1><p><a href="index.html">Back</a>.</p>' \
  >"$work/built-good/install.html"
built_page "Worktrees" '<h1 id="worktrees">Worktrees</h1><h2 id="sessions">Sessions</h2>
<p><a href="../install.html">Installing</a>.</p>' "../" >"$work/built-good/guide/worktrees.html"

cp -R "$work/built-good" "$work/built-missing-page"
built_page "The guide" '<h1 id="the-guide">The guide</h1>
<p><a href="installation.html">Installing</a>, and <a href="https://micold.invalid/forum">elsewhere</a>.</p>' \
  >"$work/built-missing-page/index.html"

cp -R "$work/built-good" "$work/built-missing-fragment"
built_page "The guide" '<h1 id="the-guide">The guide</h1>
<p>The <a href="guide/worktrees.html#running-a-session">session section</a>,
and <a href="https://micold.invalid/forum">elsewhere</a>.</p>' \
  >"$work/built-missing-fragment/index.html"

# --- the assertions ---------------------------------------------------------------------------------

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
    fail "$what" "the check passed a tree it must refuse"
  elif grep -qiF -- "$needle" "$work/out"; then
    pass "$what"
  else
    fail "$what" "failed, but did not name \"$needle\": $(tail -8 "$work/out")"
  fi
}

printf '== the sources, before a merge (FR-021) ==\n'
expect_pass "a documentation set whose internal links resolve passes" --sources "$work/src-good"
expect_fail "a link to a page that does not exist fails" "installation.md" \
  --sources "$work/src-missing-page"
expect_fail "a fragment that names no heading fails" "running-a-session" \
  --sources "$work/src-missing-fragment"

printf '== the built site, before a deploy (FR-005, SC-003) ==\n'
expect_pass "a rendered site whose internal links resolve passes" --built "$work/built-good"
expect_fail "a rendered link to a page that does not exist fails" "installation.html" \
  --built "$work/built-missing-page"
expect_fail "a rendered fragment that names no heading fails" "running-a-session" \
  --built "$work/built-missing-fragment"

printf '== the network is never reached (FR-021) ==\n'
# Both controls above carry a link to `micold.invalid`, a host that by definition cannot resolve.
# They passed, so nothing tried to fetch it -- but a check that merely *tolerated* the failure would
# pass too, and would still be reaching the network on every publication. So the control is run once
# more and its own output is read: the external host must not appear in it at all.
if run --sources "$work/src-good" && ! grep -qiF "micold.invalid" "$work/out"; then
  pass "no external URL is fetched -- an unresolvable host is not even reported on"
else
  fail "no external URL is fetched" "$(tail -8 "$work/out")"
fi

printf '== the mode is required ==\n'
expect_fail "neither mode fails with usage" "usage" "$work/built-good"

printf '\n'
if [ "$failures" -eq 0 ]; then
  printf 'the link checks: all assertions hold\n'
else
  printf '%d assertion(s) failed\n' "$failures"
fi
exit "$([ "$failures" -eq 0 ] && echo 0 || echo 1)"
