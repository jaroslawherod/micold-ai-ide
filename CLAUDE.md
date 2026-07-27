# CLAUDE.md

Guidance for Claude Code when working in this repository.

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
- `mise run deb` — build the Debian `.deb` package for the host arch (installs `cargo-deb` first
  if missing).

The first `mise run <task>` in a fresh worktree/clone requires trusting the repo's `mise.toml`
once via `mise trust` (mise refuses untrusted configs by default).

mise also manages its own Rust toolchain, which is generally a *different patch release* from
whatever `rustc` is on `PATH`. Both use the same `target/` directory, and cargo fingerprints
include the compiler version — so alternating between `mise run <task>` and a bare `cargo`
invocation invalidates the other's artifacts and rebuilds every dependency (a few minutes, and a
`target/` that grows fast). Pick one and stay with it for a work session; prefer the `mise` tasks,
per above.
