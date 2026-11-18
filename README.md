# Micold AI IDE

[![CI](https://github.com/jaroslawherod/micold-ai-ide/actions/workflows/ci.yml/badge.svg)](https://github.com/jaroslawherod/micold-ai-ide/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/jaroslawherod/micold-ai-ide)](https://github.com/jaroslawherod/micold-ai-ide/releases/latest)
[![License](https://img.shields.io/github/license/jaroslawherod/micold-ai-ide)](LICENSE)

**Run several AI coding agents side by side, each in its own git worktree, from the web client.**

Micold gives every line of work its own branch, worktree and real terminal, and keeps the agent
session running in the background — close the browser, or crash it, and the sessions are still there
when you come back.

- **Any agent, your choice per session** — [Claude Code](https://www.anthropic.com/claude-code),
  [GitHub Copilot CLI](https://github.com/features/copilot/cli),
  [Pi Coding Agent](https://pi.dev/), [Codex](https://github.com/openai/codex) or
  [OpenCode](https://opencode.ai/). Mix them in the same project.
- **One worktree per task** — start from a new branch or pick up an existing one; parallel agents
  never trample each other's files.
- **Sessions that survive** — a background session service owns the terminals; the window just
  attaches.
- **Sandboxable** — run the session service in a container so agents see only the projects and
  credentials you allow.
- **Local-first** — a Rust daemon and orchestrator with a web client, no account;
  all state stays on your machine and local mode works offline.

**Documentation: <https://jaroslawherod.github.io/micold-ai-ide/>** — the user guide, published from this repository on every release.

## Features

- Open a git project and manage its worktrees (one branch per line of work) from a Material
  Design sidebar — on a new branch, or on one that already exists locally or on a remote, so work
  started outside the app can be picked up inside it.
- Run multiple concurrent AI CLI sessions — `claude`, `copilot`, `pi`, `codex` or `opencode` — each in its own worktree
  or directly in the project root ("Default"), in an embedded terminal. Each session remembers which
  CLI it runs, and they can run side by side in the same project.
- A real terminal emulator: full ANSI color + text styling, live keyboard and mouse input,
  focus-gated key routing, resize, scrollback, and copy/paste.
- Sessions run in a background **session service**, so they survive closing — or crashing — the
  client, and reattach instantly when it comes back.
- That service can run **in a container** instead of directly on your computer: it then sees only
  the projects you registered and the credentials you allowed, under limits you set. See
  [Running the session service in a container](docs/user-guide/sandboxed-daemon.md).

## Download

Linux is the only supported platform: macOS and Windows support was dropped in
[#1897](https://github.com/jaroslawherod/micold-ai-ide/issues/1897). The desktop application and its
`.deb` and `.rpm` packages were retired in #1959; the web client is the client, and the daemon and
orchestrator are built from source or run from their container images. See
[installing](https://jaroslawherod.github.io/micold-ai-ide/install.html).

## Build from source

```sh
mise run daemon     # run the session daemon
mise run test       # test the whole workspace, as CI does
mise run test-core  # test only the render-free core — no GUI, much faster
```

The workspace is `micold-core` (render-free logic and the wire protocol), `micold-daemon` (the
session service and the orchestrator) and `micold-e2e`; the web client is `apps/client`. `mise.toml` holds the
canonical commands; a first run in a fresh clone needs `mise trust` once.

## Documentation

The published site is <https://jaroslawherod.github.io/micold-ai-ide/>: the same documentation
this repository carries, at the latest release, at the latest release.

The sources are here, and are what a pull request edits — the site is built from them, never the
other way round: [`docs/README.md`](docs/README.md), including
[installing](docs/install.md),
[the session service](docs/daemon.md) and
[running it in a container](docs/user-guide/sandboxed-daemon.md).
How the site is built and published: [`docs/development/docs-site.md`](docs/development/docs-site.md).
Changes: [`CHANGELOG.md`](CHANGELOG.md)
(maintained by release-please and embedded in the app).

## Releases

Releases are automated with [release-please](https://github.com/googleapis/release-please) from
[Conventional Commits](https://www.conventionalcommits.org/). Each release publishes Debian
packages (`.deb`) for `amd64` and `arm64`, RPM packages for `x86_64` and `aarch64`, and
republishes the documentation site above.

## License

Apache-2.0.
