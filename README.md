# Micold AI IDE

A local-first, AI-assisted desktop IDE for managing [Claude Code](https://www.anthropic.com/claude-code)
worktrees and sessions with an embedded, real terminal.

Built in **Rust** with the **iced** GUI framework. All state lives on your machine — the app is
fully functional offline (Constitution Principle IV).

## Features

- Open a git project and manage its worktrees (one branch per line of work) from a Material
  Design sidebar — on a new branch, or on one that already exists locally or on a remote, so work
  started outside the app can be picked up inside it.
- Run multiple concurrent `claude` sessions, each in its own worktree or directly in the
  project root ("Default"), in an embedded terminal.
- A real terminal emulator: full ANSI color + text styling, live keyboard and mouse input,
  focus-gated key routing, resize, scrollback, and copy/paste.
- Light/dark theming that follows your OS preference; configurable terminal scrollback.

## Build & run

```sh
cargo run --features gui        # run the GUI app
cargo test --no-default-features # render-free logic core (no GUI needed)
```

Linux build/runtime needs the usual GUI dev libraries (X11/Wayland/xkbcommon); see
[`.github/workflows/ci.yml`](.github/workflows/ci.yml) for the exact package list.

## Documentation

User guide: [`docs/README.md`](docs/README.md). Changes: [`CHANGELOG.md`](CHANGELOG.md)
(maintained by release-please and embedded in the app).

## Releases

Releases are automated with [release-please](https://github.com/googleapis/release-please) from
[Conventional Commits](https://www.conventionalcommits.org/). Each release publishes Debian
packages (`.deb`) for `amd64` and `arm64`.

## License

Apache-2.0.
