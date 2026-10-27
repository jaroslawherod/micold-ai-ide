# Implementation Plan: One configuration file for every daemon

**Branch**: `docs/daemon-config-file-plan` | **Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/daemon-config-file/spec.md`

> **PLANNING ONLY.** This plan is split into GitHub issues ([issues.md](issues.md)); each issue is
> implemented later in its own worktree and its own spec-kit run, test-first. No source file changes
> in this branch.

## Summary

Every daemon kind (host process, SSH host, container, Kubernetes pod) starts from one TOML file,
`daemon.toml`, that the application renders from Settings and delivers next to the daemon (or a user
writes by hand). It replaces about a dozen `MICOLD_*` start-up variables, three per-kind env lists
and the `GitCredentialGrant` push protocol. Secrets are named by file path only. The daemon watches
the file and its secret files and applies GitHub tokens, AI CLI entries, session env and projects
live; listener, auth, storage and the start script need a restart, which the app shows.

Technical approach (research.md):
- typed schema in `micold-core::daemon_config` (serde + `toml`, `deny_unknown_fields`, versioned) —
  one type for the client renderer, the daemon loader and `micold-daemon config check` (D1–D3);
- the daemon's credential helper becomes a stateless `micold-daemon git-credential` that reads the
  config, the token file and a reach cache (`GET /user/repos`) at each call — multi-repository PATs
  with an optional `repos` filter, most-specific wins (D6, D7);
- a `gh` shim on the session `PATH` sets `GH_TOKEN` per repository (D8);
- `[[ai.cli]]` holds deployment settings per CLI (enabled, version, args, model, base URL, sign-in,
  API key file, settings files); live UI settings stay in `settings.json` (D9);
- `[startup]` runs a user script inside `micold-daemon` before it listens, on every start path,
  capturing exports with the environment-include engine (D12);
- per-kind delivery: host data dir; SSH copy over the existing provisioning channel; container bind
  mounts; pod ConfigMap + directory Secret mounts (D10).

Bug found while verifying, fixed first and independently: the pod daemon refuses GitHub token
grants (research §1.1). The handover's second bug is already fixed by #1438 (§1.2).

## Technical Context

**Language/Version**: Rust (workspace toolchain, edition 2024).

**Primary Dependencies**: `serde`, `toml` 1.x (already in `Cargo.lock` transitively; becomes a
direct dependency of `micold-core`), `notify` 8.2 (already in `micold-daemon`), `ureq` 3.4 (workspace
dependency of the client today; new direct dependency of the daemon for reach discovery, research
D6), `protocol::hashing::sha256_hex` for fingerprints, the client's existing kube API for the
ConfigMap.

**Storage**: files only — `daemon.toml`, secret files (`0600`), `<data_dir>/github-reach.json`,
`<data_dir>/startup.done`, `<data_dir>/startup-failure.json`, `<data_dir>/startup-env.json`.

**Testing**: `cargo test` via `mise run test-core` (schema, parse, render goldens, matching,
env-diff), daemon integration tests with a fake forge (`MICOLD_GIT_CREDENTIAL_BASE` stays as the
test override), `mise run test-sandbox` for container delivery, `scripts/tests/*.test.sh` for the
entrypoint, kube manifest goldens.

**Target Platform**: Linux, macOS, Windows clients; daemons on Linux/macOS hosts, Windows host
process, Linux containers and pods.

**Project Type**: desktop app (iced) + daemon + shared core library.

**Performance Goals**: a changed token is in use within 5 s (SC-004); the credential helper answers
from cache without a network call on a hit; config load < 10 ms.

**Constraints**: no secret value in any file, log or diagnostic (SC-005); offline: with a reach
cache and no network, helper keeps answering for known repositories; the app and
its provisioned daemons run the same build (spec Assumptions), so no compatibility period for the
retired messages; the schema version guards hand-written files.

**Scale/Scope**: 4 daemon kinds, ~45 config keys, 15 issues plus an epic (issues.md).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: PASS. Every issue's tasks start with failing tests; the schema and renderer
  are pure functions with golden files per kind; matching and env-diff are unit-tested in core; the
  planning branch itself has no code.
- [x] **II. Multi-Session Support**: PASS. Config is per daemon; per-session state (env, `GH_TOKEN`,
  chosen token) is resolved at session spawn or per command and never shared between sessions.
- [x] **III. Worktree Integration**: PASS. Token choice is by the repository's remote, so every
  worktree of a repository gets the same token; `[[projects]]` names project roots, worktrees stay
  app-managed.
- [x] **IV. Local-First Storage**: PASS. The file and secrets live on the daemon's filesystem. The
  only new network call is `GET /user/repos` with a token the user pasted for GitHub (explicit
  opt-in, as the current reach check); it is cached for offline use.
- [x] **V. Rust + iced**: PASS. Schema types make invalid states unrepresentable: `[listen]` is an
  enum (socket | tcp), `idle_stop` an enum, CLI ids an enum, secrets a `SecretPath` newtype with no
  `Display` of contents.
- [x] **VI. Cross-Platform Parity**: PASS with a note. The file and loader are the same on all
  platforms; default paths come from the existing `paths.rs`; the start script's default shell is
  PowerShell on Windows (as environment-include); owner-only checks are unix-only, as today
  (`owner_only.rs`). Container and pod kinds are Linux-only by nature.
- [x] **VII. Documentation**: PASS. Each issue updates the user-guide page it changes
  (`sandboxed-daemon.md`, `kubernetes-daemons.md`, `remote-daemons.md`, `settings.md`) and a new
  `daemon-configuration.md` reference ships with the schema issue (FR-018).
- [x] **VIII. Reusable UI Components**: PASS. Settings uses the existing settings rows, the shared
  multi-line text editor for the start script, and the shared file picker; the read-only config view
  reuses the code/markup viewer. No new feature-local widgets.

Post-design re-check: PASS — no deviation; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/daemon-config-file/
├── handover.md          # Brief and verified-later findings
├── spec.md              # What and why
├── plan.md              # This file
├── research.md          # Verified code facts, decisions D1–D12, open questions
├── data-model.md        # Entities and rules
├── contracts/
│   └── daemon-config.md # The TOML schema, every key, an example per kind
├── quickstart.md        # End-to-end validation per kind
├── tasks.md             # Tasks grouped by future issue
├── issues.md            # One section per GitHub issue
└── checklists/requirements.md
```

### Source Code (touched by the future issues, not by this branch)

```text
crates/micold-core/src/
├── daemon_config/          # NEW: types, parse, validate, matching (issues 2, 9)
├── env_include.rs          # generalised run_and_diff for the start script (issue 12)
├── scoped_credential/      # PAT store without repo, reach parsing (issue 8)
├── kube/                   # ConfigMap + directory Secret manifests (issue 7)
├── sandbox/argv.rs         # config + secrets mounts replace the -e list (issue 6)
└── provider.rs, provision/ # [[ai.cli]] mapping (issue 11)

crates/micold-daemon/src/
├── main.rs                 # --config, start script before the runtime (issues 3, 12)
├── config_watch.rs         # NEW: notify watcher, live apply (issue 3)
├── git_credential.rs       # stateless helper subcommand, reach cache (issue 9)
├── gh_shim.rs              # NEW: gh-token subcommand + shim (issue 10)
├── state.rs                # session env layering (issues 11, 12)
└── server.rs               # retire GitCredential* (issue 9)

crates/micold-client/src/
├── daemon_config_render.rs # NEW: Settings → daemon.toml (issue 4)
├── daemon.rs, shell/       # delivery per kind; Settings → Daemons (issues 5–7, 11, 12, 14)
└── kube/                   # pod delivery (issue 7)

packaging/basic/entrypoint.sh  # read [ssh] dir from the file (issue 6)
docs/user-guide/               # daemon-configuration.md (new) and the kind pages
```

**Structure Decision**: existing three-crate layout; the schema is in `micold-core` because both the
client (render, check before delivery) and the daemon (load) need the same type.

## Delivery order

Bug (issue 1) any time. Then: schema (2) → loader + reload (3) and renderer (4) → delivery per
kind (5 host/SSH, 6 container, 7 pod) → tokens (8) → helper (9) → `gh` (10); AI CLIs (11), start
script (12), projects (13) and the config view (14) in parallel after delivery → retire variables
(15, last). Each delivery issue removes that kind's env list in
the same change, so no kind is fed by both at once (research D10, D11).

## Complexity Tracking

None.
