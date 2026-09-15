---
name: reclaim-disk
description: Use when the disk is filling up, a build fails with "No space left on device", or target-shared/ or a worktree's target dir has grown large — reclaims cargo build artifacts across the shared target dir and every worktree's private one with `mise run sweep`.
---

# Reclaim disk from cargo target dirs

Sharing `target-shared/` removes the *multiplication* — one directory instead of one per branch —
but only for builds that go through `mise run`, and not the growth: every branch that builds here
leaves artifacts behind and cargo never collects them, so `target-shared/` creeps up on a disk that
has run out once already. Bare-`cargo` worktree dirs put the multiplication back, quietly, which is
how that disk reached zero bytes free. `mise run sweep` bounds all of them.

`mise run sweep` reclaims space in **every** target dir this repo accumulates — the shared one and
each worktree's private one — dropping artifacts unused for 7 days (installs `cargo-sweep` first if
missing). It walks `git worktree list` via `scripts/sweep-targets.sh` and lets each checkout resolve
its own directory.

- It refuses to run while a `cargo` build is, since the oldest artifacts in a shared directory are
  usually dependencies a live build is still linking against; `SWEEP_FORCE=1` overrides.
- `SWEEP_ARGS` replaces the default, e.g. `SWEEP_ARGS='--dry-run --time 7'` to preview, or
  `SWEEP_ARGS='--maxsize 50GB'` to bound each directory by size instead of age.
