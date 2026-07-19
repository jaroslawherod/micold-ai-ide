# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Use `mise` tasks, not raw `cargo` commands

Prefer `mise run <task>` over invoking `cargo` directly — the tasks in `mise.toml` are the
canonical way to build, test, and run this project, so use them instead of rediscovering the
right `cargo` invocation each time:

- `mise run run` — run the GUI application (`cargo run --features gui`).
- `mise run test` — test the render-free logic core, matching CI (`cargo test
  --no-default-features --all-targets`).
- `mise run build` — build the release GUI binary.
- `mise run deb` — build the Debian `.deb` package for the host arch.

The first `mise run <task>` in a fresh worktree/clone requires trusting the repo's `mise.toml`
once via `mise trust` (mise refuses untrusted configs by default). mise also manages its own Rust
toolchain, in its own target directory — so the first `mise run` in a new worktree does a full
dependency rebuild (a few minutes) even if `cargo build` has already been run directly there.
