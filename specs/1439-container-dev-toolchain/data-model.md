# Data model: Container sessions can build a mise project (#1439)

No new persisted app state, no settings field, no protocol message. Three things carry data.

## Pinned mise release (spec Key Entity)

- **Where**: `packaging/basic/mise.sha256` (new), inside the hashed directory.
- **Fields**: `version` (from the file names, e.g. `2026.10.19`), `sha256` per architecture
  (`linux-x64`, `linux-arm64`).
- **Rules**: exactly two lines, `sha256sum` format; one version on both lines; 64 lowercase hex.
  Read by the `Containerfile` (B3), the guard test (B4), the real-runtime test (`mise --version`
  equals `version`) and named in `packaging/basic/README.md` (FR-013).
- **Lifecycle**: changed by hand when mise is updated (README "Update mise"); every change moves
  `BASIC_IMAGE_CONTENT_HASH` and goes through 806's hash, publish and pin steps.

## Sandbox home toolchain (spec Key Entity)

- **Where**: the sandbox home of a container daemon (`$MICOLD_HOME`) or pod (`/data/home`).
- **Folders**: `~/.local/bin`, the mise shims folder (B5), mise's data (`~/.local/share/mise`),
  rustup's `~/.rustup` and cargo's `~/.cargo` (mise's rust backend defaults), mise's cache
  (`~/.cache/mise`).
- **Also**: `~/.config/mise` (`mise use -g`), `~/.local/lib/node_modules` (`npm install -g`, via
  `NPM_CONFIG_PREFIX`), `~/.local/share/uv` (uv tools), binaries of `cargo install` and `go install`
  in `~/.local/bin` (contract [sandbox-install-env.md](contracts/sandbox-install-env.md)).
- **Rules**: created empty by the entrypoint (the first two) before any session; filled by a
  setup command or by a session itself (FR-014); never removed by the app while the daemon exists;
  shared by every worktree and session on the daemon; persists across container restarts, container
  replacement and pod recreation (FR-005, FR-016).

## Setup turn (daemon, memory only)

- **Where**: `DaemonState` in `crates/micold-daemon/src/state.rs`, one per daemon process.
- **Shape**: `Arc<tokio::sync::Mutex<()>>`; held by at most one setup run (contract S2).
- **Per-run additions** in `setup.rs`'s `Entry`: a way to wake a waiting run when **Stop setup**,
  a delete or a daemon stop ends it (a `watch`/`Notify` the stop path signals), so S3's "at once"
  holds. `SetupState` is unchanged: waiting is `Running`.
- **Transitions** (a run on a sandbox daemon):

  ```text
  start ──► waiting (Running, line printed) ──turn──► shell running (Running) ──► Succeeded | Failed
     │            │                                         │
     │            └──stop / delete / daemon stop──► Stopped | Interrupted (no spawn)
     └──turn free──► shell running
  ```
