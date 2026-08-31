# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Response style: be concise

Skip greetings, preambles, and recaps. Do not open with phrases like "I'd be happy to help",
"Great question", or "Sure, I can do that". Do not close with a restatement of what you just did
unless the user asks for a summary. Answer directly: for a question, give the answer; for a task,
make the change and report only what changed and any follow-up the user needs to take. Output code
without wrapping explanation unless the user asks how or why. Prefer one line over a paragraph, and
a paragraph over a bulleted essay, when either communicates the same information.

## Use `mise` tasks, not raw `cargo` commands

Prefer `mise run <task>` over invoking `cargo` directly — the tasks in `mise.toml` are the
canonical way to build, test, and run this project, so use them instead of rediscovering the
right `cargo` invocation each time:

- `mise run run` — run the GUI client (`cargo run -p micold-client`); it spawns/attaches the
  session daemon itself.
- `mise run daemon` — run the session daemon on its own (`cargo run -p micold-daemon`), for when
  you need it in the foreground instead of auto-spawned.
- `mise run test` — test the **whole** workspace (core + client + daemon), matching CI
  (`cargo test --workspace`).
- `mise run test-core` — test only the render-free core (`cargo test -p micold-core
  --all-targets`); no GUI, no iced, so it is much faster for logic-only changes.
- `mise run build` — build the release GUI binary (`cargo build --release -p micold-client`).
- `mise run image` — build the `:dev` sandbox image from the working tree; `mise run test-sandbox`
  then runs the real-runtime sandbox suite against it (both crates, release, one at a time). Those
  tests are off by default, so `mise run test` does not need a container runtime installed.
- `mise run deb` — build the Debian `.deb` package for the host arch (installs `cargo-deb` first
  if missing).
- `mise run sweep` — reclaim space in **every** target dir this repo accumulates — the shared one
  and each worktree's private one (see below) — dropping artifacts unused for 7 days (installs
  `cargo-sweep` first if missing). It walks `git worktree list` via `scripts/sweep-targets.sh` and
  lets each checkout resolve its own directory. It refuses to run while a `cargo` build is, since
  the oldest artifacts in a shared directory are usually dependencies a live build is still linking
  against; `SWEEP_FORCE=1` overrides. `SWEEP_ARGS` replaces the default, e.g.
  `SWEEP_ARGS='--dry-run --time 7'` to preview, or `SWEEP_ARGS='--maxsize 50GB'` to bound each
  directory by size instead of age.

The first `mise run <task>` in a fresh worktree/clone requires trusting the repo's `mise.toml`
once via `mise trust` (mise refuses untrusted configs by default).

mise's Rust toolchain and whatever `rustc` is on `PATH` used to resolve to different patch
releases, and because cargo fingerprints include the compiler version, alternating between
`mise run <task>` and a bare `cargo` rebuilt every dependency. `rust-toolchain.toml` now pins both
entry points to `stable`, so they agree and you can move between them freely.

## One target directory, one build at a time

`mise run` tasks go through `scripts/build-lock.sh`, which exports `CARGO_TARGET_DIR` to
`target-shared/` beside the main checkout — so every worktree's `mise run` build compiles into that
one directory. Build output is there, not in `target/`; `scripts/build-lock.sh --print-target-dir`
resolves the path.

**A bare `cargo` does not share it.** `.cargo/config.toml` sets `target-dir = "target-shared"`, a
path relative to the config file's own directory — and that file is checked in, so every worktree
has its own copy and cargo's closest-config-wins resolution picks *it*. Without the export above,
`cargo build` in a worktree compiles into a `target-shared/` beside the **worktree**. Confirm which
one applies with `cargo metadata --format-version 1 --no-deps | jq -r .target_directory`. So the
sharing is a property of the `mise run` wrapper, not of the config; that is a further reason to
prefer the tasks, and why `mise run sweep` has to sweep each worktree's directory too.

Sharing it is what keeps this machine usable, and the mechanism is cargo's own: it takes an
exclusive lock on the target directory, so a second build prints `Blocking waiting for file lock on
build directory` and waits instead of running alongside the first. `jobs = 4` caps a single cargo
process but does not compose across worktrees — four agents building at once meant sixteen jobs,
which oversubscribed RAM, spilled to a swap file on the same NVMe as the target dirs, and left the
machine stalled on I/O rather than short of CPU.

**So expect to wait behind another worktree's build, and leave it that way.** Pointing
`CARGO_TARGET_DIR` somewhere private to skip the wait restores the pile-up.

`mise run` tasks also pass through `scripts/build-lock.sh`, which takes a lock in the shared git
dir and names the holder while you wait. `MICOLD_NO_BUILD_LOCK=1` skips that lock for a one-off
run; cargo's own lock still applies. The interactive tasks (`run`, `showcase`, `daemon`) skip it by
design, since they stay in the foreground for as long as the app is open.

`.cargo/config.toml` also caps rust-lld at `--threads=4`; it otherwise sizes its pool from the CPU
count, which put ~68 linker threads on one NVMe queue.

Sharing removes the *multiplication* — one directory instead of one per branch — but only for
builds that go through `mise run`, and not the growth: every branch that builds here leaves
artifacts behind and cargo never collects them, so `target-shared/` creeps up on a disk that has
run out once already. Bare-`cargo` worktree dirs put the multiplication back, quietly, which is how
that disk reached zero bytes free. `mise run sweep` bounds all of them.
