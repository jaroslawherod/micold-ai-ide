# Micold AI IDE

Run AI coding sessions on several branches at once. Micold AI IDE gives each piece of work its own
worktree and runs an AI CLI session beside the code, in a background service (the daemon) that keeps
every session alive, driven from the web client through the orchestrator.

[**Install**](install.md) · **Read the user guide** ·
[Source on GitHub](https://github.com/jaroslawherod/micold-ai-ide)

The desktop application was retired in #1959; the pages that described it are gone.

Documentation ships in the same change as the code it describes and is verified in CI, so what
follows describes the version you are reading it from (constitution, Principle VII).

## User Guide

- [The web client and the local stack](user-guide/web-client.md)
- [Tools for the AI in your sessions](user-guide/agent-tools.md) — what an assistant can do through
  the app's tool server, and which AI CLIs report their sub-agent runs (Claude Code does; Copilot,
  Pi, Codex and OpenCode have no known signal, and `claude --bg` is not covered).
- [The daemon configuration file](user-guide/daemon-configuration.md) — every key of a daemon's
  `daemon.toml`, its default per kind, the rules a file must meet, and `micold-daemon config check`.
- [Running the session service in a container](user-guide/sandboxed-daemon.md) — turning the
  container placement on, which runtimes work (and how Podman differs), what the container can and
  cannot see, credentials, limits, network posture, working offline, what happens across restarts
  and reboots, and what to do when it will not start.

## Installing

- [Installing Micold AI IDE](install.md) — building the daemon from source (Linux is the only
  supported platform).

## Development

- [The CI pipeline](development/ci-pipeline.md) — why a change that touches only documentation or
  specs skips the build entirely, where the documentation set is declared, the single status check
  the default branch requires and the two properties that keep it honest, how to force a full run,
  and what to do when the pipeline surprises you.
- [Diff guardrails](development/diff-guardrails.md) — changed-line coverage (merge-blocking) and mutation testing
  (advisory), what they cover, the two switches in `ci.yml`, and the opt-in `mise run` tasks.
- [The documentation site](development/docs-site.md) — how a publication works: the five steps a
  build runs in, what each pre-merge and pre-deploy check catches, what triggers a publication and
  how to republish without cutting a release, what a failed publication leaves behind, and why the
  fonts ship whole.

## The session service (daemon)

- [The Micold session daemon](daemon.md) — the background service that hosts your sessions so they
  survive closing (or crashing) the window: what survives and what doesn't, instant reattach and
  bounded scrollback, project/worktree operations running through the service, unattended crash
  supervision, one-window-per-project with deliberate takeover and half-open-connection detection,
  what a version mismatch looks like and how restart-and-resume behaves, why a service running
  directly on this computer does not survive a logout and what does, where the service logs, and **where the service runs** — the placement model (on this computer, in
  a container, or reserved for remote), why the container is reached over authenticated loopback TCP
  rather than a socket, and who answers "is this a git repository?" once it is. Also how a daemon behind SSH is provisioned (feature 806): the one start sequence, the basic
  image, the AI CLI install and its relay.
