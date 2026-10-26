#!/usr/bin/env bash
# Drives `scripts/basic-image-hash.sh` (feature 806, FR-002, FR-013).
#
# The basic image is published under the content hash of `packaging/basic/`, and the client pins
# that hash in `crates/micold-core/src/sandbox/basic_image.rs`. So the hash has to be a function of
# the files alone (not of the order they were written in), has to move when any byte moves, and
# `--check` has to tell a maintainer whether the pin matches the tree. Every case that edits files
# works on a temporary copy, through `MICOLD_BASIC_DIR` and `MICOLD_BASIC_PIN_FILE`.
#
# The last case reads the real Containerfile: the image is the same for every user, so nothing that
# identifies one (a key, a token, the daemon, an installed AI CLI) may be built into it.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/basic-image-hash.sh"
BASIC="$ROOT/packaging/basic"
PIN_SOURCE="$ROOT/crates/micold-core/src/sandbox/basic_image.rs"

failures=0
cases=0

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n      %s\n' "$1" "$2"
  failures=$((failures + 1))
}

# hash_of <dir>: the script's output for that tree.
hash_of() { MICOLD_BASIC_DIR="$1" "$SCRIPT" 2>&1; }

# copy_basic: a temporary copy of packaging/basic. Echoes its path.
copy_basic() {
  local d
  d="$(mktemp -d)"
  cp -R "$BASIC/." "$d/"
  echo "$d"
}

# pin_file <hash>: a temporary Rust file pinning <hash> the way basic_image.rs does.
pin_file() {
  local f
  f="$(mktemp)"
  printf 'pub const BASIC_IMAGE_CONTENT_HASH: &str = "%s";\n' "$1" >"$f"
  echo "$f"
}

# --- the hash ----------------------------------------------------------------------------------

cases=$((cases + 1))
first="$(hash_of "$BASIC")"
second="$(hash_of "$BASIC")"
if ! printf '%s' "$first" | grep -Eqx '[0-9a-f]{16}'; then
  fail "prints 16 lowercase hex" "got: $first"
elif [ "$first" != "$second" ]; then
  fail "prints 16 lowercase hex" "two runs disagree: $first then $second"
else
  pass "prints 16 lowercase hex, the same twice in a row"
fi

cases=$((cases + 1))
reordered="$(mktemp -d)"
# The same files, written in reverse name order, so creation order and directory order differ.
for f in $(cd "$BASIC" && find . -type f | LC_ALL=C sort -r); do
  mkdir -p "$reordered/$(dirname "$f")"
  cp "$BASIC/$f" "$reordered/$f"
done
got="$(hash_of "$reordered")"
if [ "$got" != "$first" ]; then
  fail "does not depend on the order the files were written in" "want $first, got $got"
else
  pass "does not depend on the order the files were written in"
fi
rm -rf "$reordered"

cases=$((cases + 1))
changed="$(copy_basic)"
printf ' ' >>"$changed/sshd_config"
got="$(hash_of "$changed")"
if [ "$got" = "$first" ] || ! printf '%s' "$got" | grep -Eqx '[0-9a-f]{16}'; then
  fail "changes when one byte of one file changes" "unchanged tree $first, one byte added: $got"
else
  pass "changes when one byte of one file changes"
fi

cases=$((cases + 1))
renamed="$(copy_basic)"
mv "$renamed/README.md" "$renamed/README.txt"
got="$(hash_of "$renamed")"
if [ "$got" = "$first" ]; then
  fail "changes when a file is renamed" "both $got"
else
  pass "changes when a file is renamed"
fi
rm -rf "$renamed"

# --- --check -----------------------------------------------------------------------------------

cases=$((cases + 1))
pin="$(pin_file "$first")"
output="$(MICOLD_BASIC_DIR="$BASIC" MICOLD_BASIC_PIN_FILE="$pin" "$SCRIPT" --check 2>&1)"
status=$?
if [ "$status" -ne 0 ] || [ "$output" != "CURRENT" ]; then
  fail "--check prints CURRENT when the pin matches the tree" "exit $status, output: $output"
else
  pass "--check prints CURRENT when the pin matches the tree"
fi

cases=$((cases + 1))
drifted="$(hash_of "$changed")"
output="$(MICOLD_BASIC_DIR="$changed" MICOLD_BASIC_PIN_FILE="$pin" "$SCRIPT" --check 2>&1)"
status=$?
if [ "$status" -ne 0 ] || [ "$output" != "DRIFT $first $drifted" ]; then
  fail "--check prints DRIFT <pinned> <tree>, exit 0, when they differ" \
    "want 'DRIFT $first $drifted' exit 0, got exit $status: $output"
else
  pass "--check prints DRIFT <pinned> <tree>, exit 0, when they differ"
fi
rm -f "$pin"
rm -rf "$changed"

cases=$((cases + 1))
output="$("$SCRIPT" --check 2>&1)"
status=$?
pinned="$(grep -o 'BASIC_IMAGE_CONTENT_HASH: &str = "[0-9a-f]*"' "$PIN_SOURCE" 2>/dev/null | grep -o '"[0-9a-f]*"' | tr -d '"')"
if [ "$status" -ne 0 ] || [ "$output" != "CURRENT" ]; then
  fail "the repository's pin matches packaging/basic" \
    "exit $status, output: $output (pinned '$pinned' in $PIN_SOURCE; set it to the tree's hash)"
else
  pass "the repository's pin matches packaging/basic"
fi

# --- the Containerfile carries no credential, daemon or AI CLI ---------------------------------

# refusals <Containerfile>: what in it would build a key, credential, token, the daemon or an AI
# CLI into the image. Prints nothing when there is none. Instructions only: comments may name what
# is deliberately absent.
refusals() {
  local instructions copies bad_copy npm
  instructions="$(grep -v '^[[:space:]]*#' "$1")"
  copies="$(printf '%s\n' "$instructions" | grep -Ei '^[[:space:]]*(COPY|ADD)[[:space:]]' || true)"
  bad_copy="$(printf '%s\n' "$copies" | grep -Ei 'key|credential|\.ssh|micold-daemon|token' || true)"
  # Any package-manager or piped-script install: what the client installs must not be baked in.
  npm="$(printf '%s\n' "$instructions" \
    | grep -Ei '(npm[[:space:]]+(install|i|ci|add)([[:space:]]|\\|$))|npx[[:space:]]|pnpm[[:space:]]|yarn[[:space:]]+(global|add)|(curl|wget)[^|]*\|[[:space:]]*(ba)?sh' || true)"
  if [ -n "$bad_copy" ]; then
    echo "copies: $bad_copy"
  elif [ -n "$npm" ]; then
    echo "installs: $npm"
  fi
}

cases=$((cases + 1))
containerfile="$BASIC/Containerfile"
if [ ! -f "$containerfile" ]; then
  fail "Containerfile copies in no key, credential, daemon or token, and installs no AI CLI" \
    "$containerfile does not exist"
else
  problem="$(refusals "$containerfile")"
  if [ -n "$problem" ]; then
    fail "Containerfile copies in no key, credential, daemon or token, and installs no AI CLI" \
      "$problem"
  else
    pass "Containerfile copies in no key, credential, daemon or token, and installs no AI CLI"
  fi
fi

# Each refusal still refuses next to the mise step (1439, contract B4): one instruction added to a
# copy of the real Containerfile.
while IFS= read -r line; do
  cases=$((cases + 1))
  bad="$(copy_basic)"
  printf '%s\n' "$line" >>"$bad/Containerfile"
  if [ -z "$(refusals "$bad/Containerfile")" ]; then
    fail "the guard refuses: $line" "refusals printed nothing"
  else
    pass "the guard refuses: $line"
  fi
  rm -rf "$bad"
done <<'LINES'
COPY id_ed25519 /root/key
ADD credentials.json /etc/credentials.json
COPY .ssh /root/.ssh
COPY micold-daemon /usr/local/bin/micold-daemon
COPY gh_token /etc/token
RUN npm install -g @anthropic-ai/claude-code
RUN npx some-cli
RUN pnpm add -g some-cli
RUN yarn global add some-cli
RUN curl -fsSL https://example.invalid/install.sh | sh
RUN wget -qO- https://example.invalid/install.sh | bash
LINES

# --- the pinned mise release (1439, FR-002, FR-013, contract B3/B4) ------------------------------
# The image downloads mise and checks it against `mise.sha256`, in the same RUN, so a download that
# does not match the pin fails the build. `mise.sha256` is exactly two `sha256sum` lines of one
# version, and the README names that version.

# mise_problems <dir>: what is wrong with the mise pin in a copy of packaging/basic. Prints nothing
# when nothing is.
mise_problems() {
  local dir="$1" runs downloads unchecked lines versions version
  # One logical instruction per line: comments dropped, `\` continuations joined.
  runs="$(grep -v '^[[:space:]]*#' "$dir/Containerfile" | sed -e ':a' -e '/\\$/N; s/\\\n/ /; ta')"
  downloads="$(printf '%s\n' "$runs" | grep -E 'github\.com/jdx/mise/releases/download' || true)"
  if [ -z "$downloads" ]; then
    echo "Containerfile does not download mise from github.com/jdx/mise/releases"
    return
  fi
  unchecked="$(printf '%s\n' "$downloads" | grep -Ev 'sha256sum[[:space:]]+-c.*mise\.sha256' || true)"
  if [ -n "$unchecked" ]; then
    echo "a RUN downloads mise without 'sha256sum -c' of mise.sha256: $unchecked"
    return
  fi
  if [ ! -f "$dir/mise.sha256" ]; then
    echo "mise.sha256 does not exist"
    return
  fi
  lines="$(wc -l <"$dir/mise.sha256" | tr -d ' ')"
  if [ "$lines" != 2 ]; then
    echo "mise.sha256 has $lines lines, want 2"
    return
  fi
  if grep -Evq '^[0-9a-f]{64}  mise-v[0-9]+\.[0-9]+\.[0-9]+-linux-(x64|arm64)$' "$dir/mise.sha256"; then
    echo "mise.sha256 has a line that is not '<64 lowercase hex>  mise-v<version>-linux-(x64|arm64)'"
    return
  fi
  if ! grep -q -- '-linux-x64$' "$dir/mise.sha256" || ! grep -q -- '-linux-arm64$' "$dir/mise.sha256"; then
    echo "mise.sha256 does not have one linux-x64 and one linux-arm64 line"
    return
  fi
  versions="$(sed -E 's/^[0-9a-f]{64}  mise-v(.*)-linux-(x64|arm64)$/\1/' "$dir/mise.sha256" | sort -u)"
  if [ "$(printf '%s\n' "$versions" | wc -l | tr -d ' ')" != 1 ]; then
    echo "mise.sha256 names more than one version: $(printf '%s ' $versions)"
    return
  fi
  version="$versions"
  if ! grep -qF "$version" "$dir/README.md"; then
    echo "README.md does not name the pinned mise version $version"
  fi
}

cases=$((cases + 1))
problem="$(mise_problems "$BASIC")"
if [ -n "$problem" ]; then
  fail "the real tree pins mise: checked download, two-line mise.sha256, README names it" "$problem"
else
  pass "the real tree pins mise: checked download, two-line mise.sha256, README names it"
fi

# mise_refuses <name> <dir>: the case passes when mise_problems finds something in <dir>.
mise_refuses() {
  cases=$((cases + 1))
  if [ -z "$(mise_problems "$2")" ]; then
    fail "the mise pin check refuses $1" "mise_problems printed nothing"
  else
    pass "the mise pin check refuses $1"
  fi
  rm -rf "$2"
}

# The first hex digit of every line, upper-cased; a third line; the arm64 line at another version.
bad="$(copy_basic)"
sed -i.bak -e '/sha256sum/s/sha256sum[[:space:]]*-c/true/' "$bad/Containerfile" && rm -f "$bad/Containerfile.bak"
mise_refuses "a download without the checksum check" "$bad"

bad="$(copy_basic)"
cat "$bad/mise.sha256" <(head -n1 "$bad/mise.sha256") >"$bad/m" && mv "$bad/m" "$bad/mise.sha256"
mise_refuses "a mise.sha256 with three lines" "$bad"

bad="$(copy_basic)"
sed -i.bak -E 's/^([0-9a-f]{63})[0-9a-f]/\1F/' "$bad/mise.sha256" && rm -f "$bad/mise.sha256.bak"
mise_refuses "a mise.sha256 with uppercase hex" "$bad"

bad="$(copy_basic)"
sed -i.bak -E '/-linux-arm64$/s/mise-v[0-9]+\.[0-9]+\.[0-9]+-/mise-v1.2.3-/' "$bad/mise.sha256" && rm -f "$bad/mise.sha256.bak"
mise_refuses "a mise.sha256 with two versions" "$bad"

bad="$(copy_basic)"
version="$(sed -nE 's/^[0-9a-f]{64}  mise-v(.*)-linux-x64$/\1/p' "$BASIC/mise.sha256" 2>/dev/null)"
if [ -n "$version" ]; then
  grep -vF "$version" "$bad/README.md" >"$bad/r" && mv "$bad/r" "$bad/README.md"
fi
mise_refuses "a README that does not name the pinned version" "$bad"

# --- the basic image installs the tools an agent session expects (1435) -------------------------
# `gh` for PR and issue work from inside a session, plus the everyday tools a script or an AI CLI
# reaches for. The apt-get install list is what builds them in, so this reads it.

cases=$((cases + 1))
instructions="$(grep -v '^[[:space:]]*#' "$BASIC/Containerfile")"
missing=""
for pkg in gh curl wget jq ripgrep make python3; do
  printf '%s\n' "$instructions" | grep -Eq "^[[:space:]]+$pkg([[:space:]]|\\|$)" || missing="$missing $pkg"
done
if [ -n "$missing" ]; then
  fail "Containerfile installs gh, curl, wget, jq, ripgrep, make and python3" "missing:$missing"
else
  pass "Containerfile installs gh, curl, wget, jq, ripgrep, make and python3"
fi

# --- no image sets XDG_DATA_HOME (1286) ---------------------------------------------------------
# Every process in the container inherits an image-wide ENV, and `/var/lib` is not writable by the
# session user, so XDG-aware tools (mise) fail. The daemon is pointed at its state directory through
# MICOLD_DATA_DIR instead.

for image in basic sandbox; do
  cases=$((cases + 1))
  file="$ROOT/packaging/$image/Containerfile"
  instructions="$(grep -v '^[[:space:]]*#' "$file")"
  if printf '%s\n' "$instructions" | grep -q 'XDG_DATA_HOME'; then
    fail "the $image image does not set XDG_DATA_HOME" "$file sets it"
  elif ! printf '%s\n' "$instructions" | grep -q '^ENV MICOLD_DATA_DIR=/var/lib/micold-ai-ide$'; then
    fail "the $image image points the daemon at its state directory with MICOLD_DATA_DIR" \
      "$file has no 'ENV MICOLD_DATA_DIR=/var/lib/micold-ai-ide'"
  else
    pass "the $image image sets MICOLD_DATA_DIR, not XDG_DATA_HOME"
  fi
done

printf '\n%d case(s), %d failure(s)\n' "$cases" "$failures"
[ "$failures" -eq 0 ]
