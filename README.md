# Micold AI IDE

A local-first, AI-assisted desktop IDE for managing git worktrees and AI coding sessions with an
embedded, real terminal. A session runs [Claude Code](https://www.anthropic.com/claude-code) or
[GitHub Copilot CLI](https://github.com/features/copilot/cli), whichever you pick when you start
it.

Built in **Rust** with the **iced** GUI framework. All state lives on your machine — the app is
fully functional offline (Constitution Principle IV).

## Features

- Open a git project and manage its worktrees (one branch per line of work) from a Material
  Design sidebar — on a new branch, or on one that already exists locally or on a remote, so work
  started outside the app can be picked up inside it.
- Run multiple concurrent AI CLI sessions — `claude` or `copilot` — each in its own worktree or
  directly in the project root ("Default"), in an embedded terminal. Each session remembers which
  CLI it runs, and the two can run side by side in the same project.
- A real terminal emulator: full ANSI color + text styling, live keyboard and mouse input,
  focus-gated key routing, resize, scrollback, and copy/paste.
- Light/dark theming that follows your OS preference; configurable terminal scrollback.
- Sessions run in a background **session service**, so they survive closing — or crashing — the
  window, and reattach instantly when it comes back.
- That service can run **in a container** instead of directly on your computer: it then sees only
  the projects you registered and the credentials you allowed, under limits you set. See
  [Running the session service in a container](docs/user-guide/sandboxed-daemon.md).

## Build & run

```sh
mise run run        # run the GUI client (it spawns or attaches the session daemon itself)
mise run test       # test the whole workspace, as CI does
mise run test-core  # test only the render-free core — no GUI, much faster
```

The workspace is three crates: `micold-core` (render-free logic and the wire protocol),
`micold-client` (the iced GUI) and `micold-daemon` (the session service). `mise.toml` holds the
canonical commands; a first run in a fresh clone needs `mise trust` once.

Linux build/runtime needs the usual GUI dev libraries (X11/Wayland/xkbcommon); see
[`.github/workflows/ci.yml`](.github/workflows/ci.yml) for the exact package list.

## Documentation

User guide: [`docs/README.md`](docs/README.md) — including
[the session service](docs/daemon.md) and
[running it in a container](docs/user-guide/sandboxed-daemon.md).
Changes: [`CHANGELOG.md`](CHANGELOG.md)
(maintained by release-please and embedded in the app).

## Releases

Releases are automated with [release-please](https://github.com/googleapis/release-please) from
[Conventional Commits](https://www.conventionalcommits.org/). Each release publishes Debian
packages (`.deb`) for `amd64` and `arm64`.

## License

Apache-2.0.
