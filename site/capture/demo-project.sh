#!/usr/bin/env bash
# The fabricated project every screenshot is taken of (feature 028, T019).
#
# The application is a workspace for someone's own code, so a screenshot of it is a screenshot of
# whatever repository was open. Pointing the capture at a real checkout would publish a home
# directory, real branch names and, on this project's own screenshots, a customer's name -- to a
# public site, permanently, in an image nobody greps (FR-013).
#
# So the capture opens a project that does not exist: fabricated name, fabricated branches,
# fabricated commits, all under the run's own temporary directory. It is built here rather than
# committed because a checked-in git repository inside a git repository is a nuisance, and because
# the state has to be *identical on every run* -- git stamps a commit with the wall clock and the
# committer's identity, so both are pinned. Two publications that differ only in a timestamp inside
# a PNG are two publications whose diff tells nobody anything (FR-011b).
#
#   site/capture/demo-project.sh <dir> [--state <data-home>]
#
# Writes the project to <dir> and the catalogue the application reads at startup to
# <data-home>/micold-ai-ide/projects.json -- `$XDG_DATA_HOME` when the capture display has set one,
# and `<dir>-state` otherwise. The catalogue is what lets the capture open a project without a CLI:
# the client has no command line to speak of, so the way to start it on a project is to have the
# project already in its catalogue.

set -euo pipefail

die() {
  printf 'demo-project.sh: %s\n' "$1" >&2
  exit 1
}

dir=""
state="${XDG_DATA_HOME:-}"

while [ $# -gt 0 ]; do
  case "$1" in
    --state) state="$2"; shift 2 ;;
    -h | --help) sed -n '2,23p' "$0"; exit 0 ;;
    -*) die "unknown argument: $1" ;;
    *) [ -z "$dir" ] || die "only one project directory, got $dir and $1"; dir="$1"; shift ;;
  esac
done

[ -n "$dir" ] || die "usage: demo-project.sh <dir> [--state <data-home>]"

rm -rf "$dir"
mkdir -p "$dir"
dir="$(cd "$dir" && pwd -P)"
[ -n "$state" ] || state="$dir-state"
name="$(basename "$dir")"

# Every commit below runs through this. It pins the identity, the clock and the configuration:
# `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` pointed at nothing is what stops the machine's own
# `user.name`, `commit.gpgsign`, `core.hooksPath` or `init.defaultBranch` reaching a commit and
# making one machine's capture differ from another's.
demo_git() {
  GIT_CONFIG_GLOBAL=/dev/null \
  GIT_CONFIG_SYSTEM=/dev/null \
  GIT_AUTHOR_NAME="Robin Alder" \
  GIT_AUTHOR_EMAIL="robin@aurora.invalid" \
  GIT_COMMITTER_NAME="Robin Alder" \
  GIT_COMMITTER_EMAIL="robin@aurora.invalid" \
  GIT_AUTHOR_DATE="$commit_date" \
  GIT_COMMITTER_DATE="$commit_date" \
    git -C "$dir" -c advice.detachedHead=false "$@"
}

# The dates march forward by a day per commit so the history reads like work rather than like a
# script, while staying the same on every run. `.invalid` is the reserved TLD (RFC 2606): the
# address in the log cannot be someone's real one.
commit_date="2026-01-12T09:14:00+00:00"
next_day() {
  commit_date="$(TZ=UTC date -u -d "$commit_date + 1 day" +%Y-%m-%dT%H:%M:%S+00:00)"
}

demo_git init -q -b main

# --- the project's own content -------------------------------------------------------------------
#
# Plausible, small, and boring on purpose: it appears in the file tree and the diff view of several
# screenshots, so it has to survive being read by a curious reader without becoming the subject.

mkdir -p "$dir/src" "$dir/docs" "$dir/tests"

cat >"$dir/README.md" <<'EOF'
# Aurora Fleet

Route planning and telemetry for a small fleet of delivery vehicles.

- `src/` — the service
- `docs/` — how it is operated
- `tests/` — the acceptance suite

Run the service with `cargo run`, and the acceptance suite with `cargo test`.
EOF

cat >"$dir/src/main.rs" <<'EOF'
//! Aurora Fleet: accepts telemetry from vehicles and answers route questions about them.

mod routes;
mod telemetry;

fn main() -> std::io::Result<()> {
    let fleet = telemetry::Fleet::load("fleet.json")?;
    println!("tracking {} vehicles", fleet.len());
    routes::serve(fleet)
}
EOF

cat >"$dir/src/telemetry.rs" <<'EOF'
//! Vehicle positions, as reported, with the gaps left visible.

pub struct Fleet {
    vehicles: Vec<Vehicle>,
}

pub struct Vehicle {
    pub id: String,
    pub last_seen: Option<u64>,
}

impl Fleet {
    pub fn load(_path: &str) -> std::io::Result<Self> {
        Ok(Self { vehicles: Vec::new() })
    }

    pub fn len(&self) -> usize {
        self.vehicles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vehicles.is_empty()
    }

    /// Vehicles that have not reported inside the window, in the order they went quiet.
    pub fn stale(&self, now: u64, window: u64) -> Vec<&Vehicle> {
        let mut stale: Vec<&Vehicle> = self
            .vehicles
            .iter()
            .filter(|v| v.last_seen.is_none_or(|seen| now.saturating_sub(seen) > window))
            .collect();
        stale.sort_by_key(|v| v.last_seen);
        stale
    }
}
EOF

cat >"$dir/src/routes.rs" <<'EOF'
//! The question the fleet is actually asked: what should this vehicle do next?

use crate::telemetry::Fleet;

pub fn serve(fleet: Fleet) -> std::io::Result<()> {
    if fleet.is_empty() {
        eprintln!("no vehicles are reporting; refusing to plan");
    }
    Ok(())
}
EOF

cat >"$dir/docs/operations.md" <<'EOF'
# Operating Aurora Fleet

The service is stateless. Restarting it loses nothing except in-flight requests.

## A vehicle stops reporting

`stale()` lists them oldest first. A vehicle that has never reported has no `last_seen` at all and
sorts first, which is deliberate: a vehicle that was never heard from is a worse problem than one
that went quiet an hour ago.
EOF

cat >"$dir/tests/routes.rs" <<'EOF'
#[test]
fn an_empty_fleet_is_not_planned_for() {
    // Placeholder: the acceptance suite lives here.
}
EOF

cat >"$dir/.gitignore" <<'EOF'
/target/
/.claude/
fleet.json
EOF

demo_git add -A
demo_git commit -q -m "Track a fleet and answer route questions about it"
next_day

cat >>"$dir/docs/operations.md" <<'EOF'

## Telemetry gaps

A gap is not an error. The window is a policy, and the policy lives with the operator, not here.
EOF
demo_git add -A
demo_git commit -q -m "Write down what a telemetry gap means"
next_day

# --- the side branches, and a worktree for each --------------------------------------------------
#
# The worktree list is the application's left-hand column and the subject of several of the
# screenshots, so the demonstration project has to have worktrees -- a project with none shows an
# empty-state sentence where the feature should be, which is what the first capture of the main
# window did publish.
#
# The names follow the application's own derivation (`crates/micold-core/src/naming.rs`): a
# worktree the app creates lands in `.claude/worktrees/${type}-${ticket}-${name}` on branch
# `${type}/${ticket}-${name}`. Fabricating them by hand in any other shape would show visitors a
# naming convention the app would never produce.

demo_git checkout -q -b feat/AF-114-route-planner
cat >>"$dir/src/routes.rs" <<'EOF'

/// The next stop for a vehicle, or `None` while its position is unknown.
pub fn next_stop(_vehicle: &str) -> Option<String> {
    None
}
EOF
demo_git add -A
demo_git commit -q -m "Sketch the next-stop question"
next_day

demo_git checkout -q main
demo_git checkout -q -b fix/AF-121-telemetry-drift
sed -i 's/now.saturating_sub(seen) > window/now.saturating_sub(seen) >= window/' "$dir/src/telemetry.rs"
demo_git add -A
demo_git commit -q -m "Treat a vehicle exactly at the window as stale"
next_day

demo_git checkout -q main
demo_git checkout -q -b docs/AF-118-operations-guide
cat >>"$dir/docs/operations.md" <<'EOF'

## Restarting the service

Restarting is safe at any time. Vehicles retry, and the window is measured from the last report,
not from the last restart.
EOF
demo_git add -A
demo_git commit -q -m "Say what a restart costs"
next_day

demo_git checkout -q main

# `git worktree add` on an existing branch, which is what the application does once the names above
# are derived. The main checkout stays on `main` and appears in the list as the default worktree.
demo_worktree() {
  demo_git worktree add -q ".claude/worktrees/$1" "$2"
}
demo_worktree feat-AF-114-route-planner  feat/AF-114-route-planner
demo_worktree fix-AF-121-telemetry-drift fix/AF-121-telemetry-drift
demo_worktree docs-AF-118-operations-guide docs/AF-118-operations-guide

# --- the catalogue -------------------------------------------------------------------------------
#
# `$XDG_DATA_HOME/micold-ai-ide/projects.json` is where the daemon looks for the known-projects
# list, and `store.rs` defines its shape. Seeding it is what makes the capture possible at all: the
# client has no CLI, so "open this project" cannot be said on a command line -- it has to already be
# in the catalogue, and active, before the window opens.

catalog_dir="$state/micold-ai-ide"
mkdir -p "$catalog_dir"

json_escape() {
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g'
}

cat >"$catalog_dir/projects.json" <<EOF
{
  "schema_version": 1,
  "last_active": "$(json_escape "$dir")",
  "projects": [
    {
      "path": "$(json_escape "$dir")",
      "display_name": "$(json_escape "$name")",
      "is_git_repo": true
    }
  ]
}
EOF

printf 'demo-project.sh: %s (catalogue in %s)\n' "$dir" "$catalog_dir" >&2
