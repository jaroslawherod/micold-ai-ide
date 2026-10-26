# Contract: where a session's own installs go on a sandbox daemon (#1439, Story 5)

Requirements: FR-014, FR-015, FR-016. Extends 1289's toolchain-path contract
(`specs/1289-sandbox-project-setup/contracts/toolchain-path.md`): same scope, same spawns, same
input resolution. It changes no `PATH`.

## I1. Scope

Applies exactly where 1289's toolchain `PATH` applies: a daemon with an `offered.json` of its own
(`DaemonState::offered_clis()` is `Some`), that is an app-owned container or Kubernetes daemon. Host
and remote daemons get no variable from this contract (1289 FR-008).

## I2. Defaults

| Variable | Default | Why it is needed |
|---|---|---|
| `NPM_CONFIG_PREFIX` | `$HOME/.local` | npm's global prefix is the image's `/usr/local`, not writable by the session user; with this, `npm install -g` writes `~/.local/lib/node_modules` and links into `~/.local/bin` |
| `CARGO_INSTALL_ROOT` | `$HOME/.local` | `cargo install` puts binaries in `~/.cargo/bin`, which is not on 1289's `PATH` (mise shims it only after a reshim); with this, they go to `~/.local/bin` |
| `GOBIN` | `$HOME/.local/bin` | `go install` puts binaries in `~/go/bin`, off `PATH` |

`$HOME` is the spawn's `HOME`, resolved as 1289's contract resolves it (the resolved environment
first, else the daemon's own). Without a `HOME`, none is set.

Each is set only when neither the resolved environment (environment-include) nor the daemon's own
environment sets it to a non-empty value; for `NPM_CONFIG_PREFIX`, the lower-case
`npm_config_prefix` counts as set too (npm reads both). A value already set is kept unchanged.

## I3. Left on their own defaults

The daemon sets none of these; each tool's default already lies in the home and, for those with
binaries, on 1289's `PATH`:

| Tool | Default location | Binaries |
|---|---|---|
| mise | `~/.local/share/mise` (data), `~/.config/mise/config.toml` (`mise use -g`) | shims folder (1289) |
| rustup / cargo | `~/.rustup`, `~/.cargo` | through mise's shims |
| uv | `~/.local/share/uv` (tools, Pythons) | `~/.local/bin` |
| pip `--user` (mise-installed Python) | `~/.local` | `~/.local/bin` |

Setting `MISE_DATA_DIR`, `CARGO_HOME`, `RUSTUP_HOME` or `XDG_*` here would change the input of 1289's
shims rule and override a project's or user's own choice, for no gain.

## I4. Reach

Every spawn the daemon makes through its spawn environment (`spawn_env_for`): sessions, terminals,
setup runs and the tool servers the AI CLI starts. The AI CLI provisioning step is not a daemon spawn
and passes its own `--prefix` (`crates/micold-core/src/provision/ai_cli.rs:231`), so it is
unaffected. An AI CLI's own npm self-update, run inside a session, now reaches `~/.local` and can
leave a second copy in `~/.local/bin` (only OpenCode's updater is disabled,
`crates/micold-core/src/provider.rs:2093`). This is accepted as harmless: the offered CLI's bin
folders precede `~/.local/bin` on `PATH` (1289's toolchain-path contract), so the offered copy runs,
and `DaemonState::only_offered` reports only offered CLIs (spec edge case "An AI CLI updates itself
through npm").

## I5. Persistence (FR-016)

Nothing in this contract stores state. What makes the installs stay is where the home lives, which
this feature does not change: a container daemon's home is the host folder
`<daemon state dir>/sandbox-home`, bind-mounted read-write, so it outlives a restart and a replaced
container; a Kubernetes daemon's home is `/data/home` on its PersistentVolumeClaim, which only
removing the daemon deletes. Citations: spec Assumptions; research R12.
