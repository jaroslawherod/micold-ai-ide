#!/usr/bin/env bash
#
# Build the repository half of feature 018's reference scene (FR-039b, quickstart.md §B8).
#
# The scene the frame-time figures are taken on is "a repository with 20 worktrees in the sidebar,
# the sidebar expanded, one running terminal session, a context menu open over a dialog". Only the
# first of those is a repository; the rest are things a person does in the running application. This
# script builds the repository, exactly and repeatably, and then tells you the remaining steps.
#
# Repeatably is the point. SC-018 compares three figures taken on the same machine across a change
# that alters what the sidebar renders, so the rows have to be identical between runs — a scene hand-
# built twice is two different scenes, and the difference lands in the number without appearing in it.
#
# It is also the fixture quickstart.md's Prerequisites asks for ("a dozen worktrees across several
# conventional-commit types"): the rows below cover all ten types, with and without issue keys, plus
# an untyped row, an over-long row, and one orphaned directory, so the tag-colour, ellipsis, filter
# and health-tag checks all have something to look at.
#
# Usage:
#   scripts/reference-scene.sh [TARGET_DIR] [--force]
#
#   TARGET_DIR  where to build it (default: ~/micold-reference-scene)
#   --force     replace TARGET_DIR if it already exists
#
# Or: mise run fixture

set -euo pipefail

TARGET_DEFAULT="$HOME/micold-reference-scene"
TARGET=""
FORCE=0

for arg in "$@"; do
    case "$arg" in
        --force) FORCE=1 ;;
        -h|--help)
            sed -n '3,26p' "$0" | sed 's|^# \{0,1\}||'
            exit 0
            ;;
        -*)
            echo "unknown option: $arg" >&2
            exit 2
            ;;
        *)
            if [ -n "$TARGET" ]; then
                echo "give at most one target directory (got '$TARGET' and '$arg')" >&2
                exit 2
            fi
            TARGET="$arg"
            ;;
    esac
done
TARGET="${TARGET:-$TARGET_DEFAULT}"

# The worktree rows, as directory names under `.claude/worktrees/`.
#
# The branch for each is the directory name with the first `-` turned into `/`, which is exactly the
# convention `micold_core::naming::derive` produces (`{type}-{ticket}-{name}` ->
# `{type}/{ticket}-{name}`). Deriving it rather than listing it twice is what keeps the two from
# drifting apart.
#
# Chosen so the sidebar has something of each kind to render:
#   - all ten Conventional-Commits types, so every tag colour appears
#   - some with a Jira-style issue key (`abc-101` -> tag `ABC-101`), some without
#   - one name with no known type at all, which lands in the "untyped" filter bucket
#   - one name long enough to force ellipsis in the row
WORKTREES=(
    feat-abc-101-terminal-scrollback
    feat-abc-102-session-restore
    fix-abc-103-cursor-drift
    fix-focus-ring-clipping
    chore-mid-914-bump-dependencies
    chore-tidy-imports
    docs-mid-915-architecture-notes
    docs-readme-refresh
    refactor-mid-916-split-the-reducer
    refactor-grid-cache-ownership
    test-mid-917-daemon-restart-coverage
    test-golden-snapshots
    build-mid-918-debian-packaging
    build-strip-release-symbols
    ci-mid-919-windows-runner
    perf-mid-920-glyph-atlas-reuse
    style-mid-921-material-tokens
    feat-a-very-long-worktree-name-that-should-ellipsize-in-the-sidebar
    spike-websocket-transport
)

# A directory under `.claude/worktrees/` that git does not know about. The application surfaces it as
# a row with a health tag rather than hiding it, so it belongs in a fixture meant to exercise what the
# sidebar draws. It is a row like any other, so it counts toward the 20.
ORPHAN="fix-orphaned-directory"

# What FR-039b asks for. Asserted at the end rather than assumed: a fixture that quietly built 19 rows
# would move the figure without ever looking wrong.
EXPECTED_ROWS=20

# Deliberately no agent-owned worktree (`agent-<hex>` / `worktree-agent-<hex>`). Those are hidden from
# the sidebar unless revealed, so one would add a row to the repository without adding a row to the
# scene — and the count that matters here is what the sidebar actually draws.

# --- checks ------------------------------------------------------------------------------------

command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 1; }

if [ -e "$TARGET" ]; then
    if [ "$FORCE" -ne 1 ]; then
        echo "$TARGET already exists. Re-run with --force to replace it." >&2
        exit 1
    fi
    # `--force` means `rm -rf` on a path someone typed, so refuse the handful that would be a
    # catastrophe rather than a rebuild. Not a general safety net — it cannot be one — but these are
    # the slips that actually happen: a stray argument, an unset variable expanding to nothing.
    resolved="$(cd "$TARGET" 2>/dev/null && pwd -P || echo "$TARGET")"
    if [ "$resolved" = "/" ] || [ "$resolved" = "$HOME" ]; then
        echo "refusing to remove $resolved" >&2
        exit 1
    fi
    # A directory that is neither empty nor something this script built is far more likely to be a
    # mistyped path than a stale fixture. The marker is written into every scene it creates.
    if [ -n "$(ls -A "$resolved" 2>/dev/null)" ] && [ ! -e "$resolved/.micold-reference-scene" ]; then
        echo "$resolved is not empty and was not built by this script (no .micold-reference-scene marker)." >&2
        echo "Refusing to delete it. Remove it yourself if that is really what you want." >&2
        exit 1
    fi
    # `git worktree add` leaves admin files inside the parent repo, but every worktree here lives
    # under TARGET itself, so removing TARGET removes the whole fixture and nothing else.
    echo "removing existing $TARGET"
    rm -rf "$TARGET"
fi

mkdir -p "$TARGET"
# Resolve symlinks now and use the canonical path throughout. `micold_core::worktree::reconcile`
# matches each git-reported worktree path against `<project>/.claude/worktrees` by exact parent
# comparison, with no canonicalisation — so if git reports a resolved path and the project was opened
# through a symlinked one, every row is silently discarded and the sidebar comes up empty.
TARGET="$(cd "$TARGET" && pwd -P)"

# --- build -------------------------------------------------------------------------------------

echo "building the reference scene in $TARGET"

git init -q -b main "$TARGET"
cd "$TARGET"
# The marker `--force` looks for before deleting anything. Written first, so an interrupted build
# still leaves a directory this script is willing to clean up on the next run.
echo "Built by scripts/reference-scene.sh — safe to delete." > .micold-reference-scene
# Set locally so the fixture does not depend on the machine's global git identity, and does not
# inherit a signing key that would make each commit prompt.
git config user.name "Micold Reference Scene"
git config user.email "reference-scene@micold.invalid"
git config commit.gpgsign false

cat > README.md <<'README'
# Micold reference scene

Generated by `scripts/reference-scene.sh`. This repository exists only to hold the worktrees that
feature 018's frame-time measurement is taken against (FR-039b, quickstart.md §B8).

Nothing here is real work. Delete it whenever.
README
# The marker is committed rather than left untracked, so the fixture's worktrees all come up clean
# — an untracked file in every one of the 20 rows is noise in a scene meant to be looked at.
git add README.md .micold-reference-scene
git commit -q -m "chore: seed the reference scene"

for dir in "${WORKTREES[@]}"; do
    branch="${dir/-//}"   # first `-` only: feat-abc-101-x -> feat/abc-101-x
    git worktree add -q -b "$branch" ".claude/worktrees/$dir" HEAD
done

mkdir -p ".claude/worktrees/$ORPHAN"
cat > ".claude/worktrees/$ORPHAN/README.md" <<'ORPHANED'
Not registered with git on purpose: the application surfaces an unregistered directory under
`.claude/worktrees/` as a worktree row carrying a health tag, and the reference scene should
exercise that path like any other row.
ORPHANED

# --- verify ------------------------------------------------------------------------------------
#
# A fixture that silently built the wrong scene is worse than one that failed: the figure it produces
# looks exactly like a good one.

registered="$(git worktree list --porcelain | grep -c '^worktree ' || true)"
# `git worktree list` counts the main checkout too, which is not a row in the sidebar.
registered_worktrees=$((registered - 1))

on_disk="$(find .claude/worktrees -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')"
rows=$((registered_worktrees + 1))   # registered rows + the orphan

fail=0
if [ "$registered_worktrees" -ne "${#WORKTREES[@]}" ]; then
    echo "expected ${#WORKTREES[@]} registered worktrees, git reports $registered_worktrees" >&2
    fail=1
fi
if [ "$on_disk" -ne "$rows" ]; then
    echo "expected $rows directories under .claude/worktrees/, found $on_disk" >&2
    fail=1
fi
if [ "$rows" -ne "$EXPECTED_ROWS" ]; then
    echo "the scene has $rows rows, but FR-039b's baseline scene is $EXPECTED_ROWS" >&2
    fail=1
fi
if [ "$fail" -ne 0 ]; then
    echo "the reference scene was NOT built correctly — do not record a figure from it" >&2
    exit 1
fi

# --- what to do next ---------------------------------------------------------------------------

cat <<NEXT

Built $rows sidebar rows ($registered_worktrees git worktrees + 1 orphaned directory) in:

  $TARGET

The repository half of the scene is done. The rest happens in the running application — compose it
before taking any figure (quickstart.md §B8):

  1. Open $TARGET as the project.
  2. Expand the sidebar; confirm all $EXPECTED_ROWS rows are there.
  3. Start one terminal session and leave it running.
  4. Open a dialog, then open a context menu over it. Leave both on screen.

Then, with the scene composed and NOT touching the window:

  MICOLD_FRAME_PROBE=300 mise run run

It discards 30 warm-up frames, counts 300, prints one line and exits. Paste that line whole into the
matching slot in specs/018-material3-visual-system/quickstart.md §B8.

For T000z this must be done BEFORE the palette task (T000f) lands — the pre-change build does not
exist afterwards, and SC-018 needs all three figures from the same machine.

To remove: rm -rf $TARGET
NEXT
