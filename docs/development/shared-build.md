# The shared target directory and build lock

`mise run` tasks go through `scripts/build-lock.sh`, which exports `CARGO_TARGET_DIR` to
`target-shared/` beside the main checkout, so every worktree's `mise run` build compiles into that
one directory. `scripts/build-lock.sh --print-target-dir` resolves the path.

## A bare `cargo` does not share it

`.cargo/config.toml` sets `target-dir = "target-shared"`, a path relative to the config file's own
directory. That file is checked in, so every worktree has its own copy, and cargo's
closest-config-wins resolution picks it. Without the export above, `cargo build` in a worktree
compiles into a `target-shared/` beside the **worktree**. Confirm which one applies with
`cargo metadata --format-version 1 --no-deps | jq -r .target_directory`. The sharing is a property
of the `mise run` wrapper, not of the config, which is why `mise run sweep` sweeps each worktree's
directory too.

## Why one directory

Cargo takes an exclusive lock on the target directory, so a second build prints `Blocking waiting
for file lock on build directory` and waits instead of running alongside the first. `jobs = 4` caps
a single cargo process but does not compose across worktrees: four agents building at once meant
sixteen jobs, which oversubscribed RAM, spilled to a swap file on the same NVMe as the target dirs,
and left the machine stalled on I/O rather than short of CPU. Pointing `CARGO_TARGET_DIR` somewhere
private to skip the wait restores that pile-up.

## The build lock

`scripts/build-lock.sh` also takes a lock in the shared git dir and names the holder while you
wait. `MICOLD_NO_BUILD_LOCK=1` skips it for a one-off run; cargo's own lock still applies. The
interactive tasks (`run`, `showcase`, `daemon`) skip it by design, since they stay in the foreground
for as long as the app is open.

Target dirs grow without bound; `mise run sweep` reclaims them.
