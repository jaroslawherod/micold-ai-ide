# Implementation Plan: Scoped, short-lived git credential for sandboxed daemons

**Branch**: `claude/project-thread-2cdrmk` | **Date**: 2026-10-19 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1002-sandbox-scoped-token/spec.md` (issue #1002)

**Provisional**: the credential source is a user-registered GitHub App (ledger D7, pending the
user's answer). Everything specific to it sits behind one seam (research R3); the parts a switch
would replace are marked **[source]** below.

## Summary

Container and Kubernetes daemons stop receiving the user's git identity: the *SSH agent*, *Git
credentials* and *SSH key* items are removed (old saved settings load, are ignored and dropped,
with a one-time notice), and the shared `~/.gitconfig` is sanitized of helpers and embedded tokens.
In their place the client mints, on the user's device, a GitHub App installation token per
repository a daemon's project pushes to — contents read/write and metadata read, plus pull requests
and issues when the daemon opts in — and delivers it over the existing authenticated protocol. The
daemon keeps it on an in-memory filesystem inside the container, and a `micold-daemon
git-credential` helper, made the only helper by a command-scope git environment, hands it to git for
that repository only, over HTTPS (SSH-form remotes rewritten). The client renews 10 minutes before
the 1-hour expiry, revokes on stop/delete, and when no credential can be had the daemon runs without
one and its status says which project and why.

## Technical Context

**Language/Version**: Rust (stable, workspace toolchain)

**Primary Dependencies**: existing `ureq` (client HTTP), `serde_json`; `base64` (workspace, today
client-only) added to `micold-core`; `ring` 0.17 added to the root `[workspace.dependencies]` and to
`micold-core` for RS256 (research R4). No new crate in `Cargo.lock`.

**Storage**: the App ID in the settings document; the App private key owner-only in
`secrets_dir()` = sibling of `data_dir()` (research R5); tokens in memory on the client and on a
tmpfs / memory `emptyDir` in the container only (research R6). Nothing new under `data_dir()`.

**Testing**: `cargo test` — core unit tests (pure), `micold-core/tests/sandbox_credentials.rs` and
`sandbox_argv.rs` (FR-014, container), `kube/start_tests.rs` with `kube-fake` (FR-014, Kubernetes),
a new `micold-daemon/tests/scoped_git_credential.rs` running real `git` (FR-015), the real-runtime
sandbox suite for a live inspection (off by default), quickstart Part C for live GitHub. The new settings rows (GitHub App section, grant toggles,
status rows) are compositions of existing shared components with no new geometry; their reducer
behaviour is tested in the client, and their look gets one `visual-pass` run (quickstart Part C
steps 2, 3, 9) — no new geometry gate.

**Target Platform**: client on Linux, macOS, Windows; the container is Linux in every case.

**Project Type**: desktop application (iced client) + daemon + render-free core library.

**Performance Goals**: minting adds ≤ 2 HTTPS round trips per repository per hour; daemon start is
never blocked on it (credentials arrive after `Attach`).

**Constraints**: fail closed (FR-012); no project-hosted service (D6); the App key never enters a
container or a host-mounted path (FR-005, R5); offline → daemons start without credentials
(constitution IV).

**Scale/Scope**: a handful of daemons × a handful of repositories per user.

## Constitution Check

*Pre-design and post-design: both PASS.*

- [x] **I. Test-First**: every decision is in pure core (JWT, request/response, narrowing,
  schedule, git env, sanitizer, helper parsing, migration, status texts) or the daemon (helper,
  file store, message handling), each with tests first. Client glue (HTTP transport over `ureq`,
  settings rows, subscription wiring) is thin and follows the Principle I exception, covered by
  quickstart Part C; the transport's status-to-reason mapping is in core and tested.
- [x] **II. Multi-Session**: credentials are per daemon and per repository; sessions of one daemon
  share their daemon's credential for a repository (spec edge case); no session state added.
- [x] **III. Worktree Integration**: worktrees of a project share its `origin`, so they share its
  credential; no worktree lifecycle change.
- [x] **IV. Local-First**: the key stays on the device; network calls go only to GitHub, only for a
  sandboxed daemon serving a GitHub project, after the user set up their own App (explicit opt-in);
  offline the feature degrades to "no credential" and everything else works.
- [x] **V. Rust + iced**: yes. `Grant` makes a credential without contents/metadata
  unrepresentable; `ScopedCredential` can only be built through the narrowing check.
- [x] **VI. Cross-Platform**: core is OS-agnostic; the one platform fact (where `secrets_dir()`
  lands) is a `directories`-derived path tested for all three layouts; the container side is Linux
  everywhere. The removed agent-socket path was the only Unix-specific credential mount.
- [x] **VII. Documentation**: `docs/user-guide/sandboxed-daemon.md`, `kubernetes-daemons.md` and
  `settings.md` updated in the same change: setting up the App, the removed items, statuses,
  the client-must-be-running renewal note.
- [x] **VIII. Reusable UI**: the *GitHub App* section, the two grant toggles and the status rows use
  the existing settings field, file-picker, toggle and status-row components; no new widget.

## Design overview

```text
client (device)                                    sandbox (container / pod)
┌───────────────────────────────────────┐          ┌─────────────────────────────────────┐
│ Settings › GitHub App  [source]       │          │ micold-daemon                        │
│   app id, key → secrets_dir()         │          │  ├ GitCredentialTargets  ───────────►│ (to client)
│ GithubAppSource  [source]             │◄─────────┤  │  project → origin push URL         │
│   impl CredentialSource (seam)        │ protocol │  ├ GitCredentialGrant/Revoke ──────► │ /run/micold/git/<o>/<r>  (tmpfs, 0600)
│ Schedule (core): renew at exp−10 min  │─────────►│  └ git-credential get  ◄── git       │
│ CredentialStatus per daemon project   │          │ env: GIT_CONFIG_COUNT block (R7)     │
└───────────────────────────────────────┘          └─────────────────────────────────────┘
```

### Changes by area

1. **Removal** (FR-001…FR-004) — `sandbox/mod.rs` (`CredentialShare`, `RawProfile`,
   `CredentialLayout`, `MountSet::build_for`), `kube/credentials.rs` (`KubeCredential`,
   `daemon_env`, `every_kind`), `kube/settings.rs` (raw `allowed`), `daemons.rs`
   (`MigrationNotice::RemovedGitCredentials`), client `features/daemons.rs`,
   `features/daemons_kube.rs`, `features/daemon_sections.rs` (items and their messages go; notice
   shown). Sanitized gitconfig: `scoped_credential/gitconfig.rs`, used by `shell/sandbox.rs`
   (written into the daemon state dir and mounted where `~/.gitconfig` was) and by the Kubernetes
   secret builder.
2. **Seam and types** — new `micold-core/src/scoped_credential/{mod.rs, schedule.rs, git_env.rs,
   gitconfig.rs, helper.rs}`: `Grant`, `SecretToken`, `ScopedCredential`, `CredentialSource`,
   `NoCredentialReason`, `CredentialStatus`, `Schedule`, `git_env`, `sanitize_gitconfig`, helper
   request/answer.
3. **[source] GitHub App** — `scoped_credential/github_app.rs` (PEM, JWT, request bodies, response
   parsing, HTTP-status → reason); `paths::secrets_dir()`; client `shell/scoped_credential.rs`
   (`GithubAppSource` over `ureq`), settings section in the client's settings feature;
   `GithubAppSettings` as `settings.github_app` in the settings document (`micold-core/src/settings.rs`).
4. **Protocol** — `DaemonMsg::GitCredentialTargets`, `ClientMsg::GitCredentialGrant`,
   `ClientMsg::GitCredentialRevoke`; version bump (contracts/protocol.md).
5. **Daemon** — targets reporting (`ops.rs`/`state.rs`, on attach and project/remote change),
   file store under `/run/micold/git` with a test root, `git-credential` subcommand in `main.rs`
   beside `status-line`/`remote`.
6. **Runtimes** — container argv: `--tmpfs /run/micold/git…` and the `git_env` block; pod manifest:
   memory `emptyDir` at `/run/micold/git`; `daemon_env` merges `git_env`.
7. **Client wiring** — per daemon `Schedule` driven by the subscription clock while connected;
   minting off the UI thread; grants/revokes sent on the daemon's connection; revocation on daemon
   stop/delete; status rows (FR-013); per-daemon grant toggles (FR-011) with their help text.
8. **Docs** — the three user-guide pages; `docs/daemon.md` gains the new messages.

### Requirement traceability

| FR / SC | Where | Tested by |
|---|---|---|
| FR-001 | area 1; contracts/git-in-container.md §4 | core `sandbox_credentials`/`sandbox_argv`, `kube::start_tests`, real-runtime sandbox suite |
| FR-002 | area 1; data-model §9 | core unit (legacy profile/settings read, dropped on save) |
| FR-003 | area 1; data-model §9, research R10 | core unit (notice raised once); client reducer test (notice shown, dismissed) |
| FR-004 | area 1; research R7; contracts/git-in-container.md §1, §3 | core unit (sanitizer, env block); daemon `scoped_git_credential` (planted helpers do not run) |
| FR-005 | areas 2–3; research R2–R3; contracts/github-app-source.md | core unit (request body, JWT, narrowing); quickstart C |
| FR-006 | area 6; research R6; data-model §10 | core argv/manifest tests (tmpfs/memory, not a host path); daemon unit (file mode, path) |
| FR-007 | areas 5–6; contracts/git-in-container.md §1–2 | daemon `scoped_git_credential` (push/pull, SSH-form rewrite, other repo/host empty) |
| FR-008 | area 7; data-model §8 | core unit (`Schedule` due at exp−10 min, retries) |
| FR-009 | data-model §2 | core unit (`accept` refuses missing or > 1 h expiry) |
| FR-010 | area 7; data-model §8; contracts/github-app-source.md §2 | core unit (`Schedule` revokes on stop/delete); daemon unit (revoke removes file) |
| FR-011 | areas 1, 7; data-model §1 | core unit (grant → permissions body); client reducer test (toggles, default off) |
| FR-012 | data-model §6, §8 | core unit (every reason → no grant); daemon starts without targets granted |
| FR-013 | research R11; data-model §6–7 | core unit (message texts); client reducer test (status rows set/cleared) |
| FR-014 | research R12 | named tests above for both runtimes |
| FR-015 | research R12 | daemon `scoped_git_credential` + core narrowing tests; quickstart C live check |
| SC-001…SC-005 | — | as their FRs; SC-002/SC-003/SC-004 live in quickstart C |

### Suggested milestones (for the tasks unit)

| M | Deliverable | Independently mergeable because |
|---|---|---|
| M1 | Removal + sanitized gitconfig + `git_env` with no helper target yet + notice + FR-014 tests + docs of the removal | US1 alone is the security guarantee (spec: "a sandbox that cannot push is safe") |
| M2 | Seam, `Schedule`, helper subcommand, tmpfs/emptyDir, protocol messages, daemon store, FR-015 test, `FakeSource` | pushes work with any source; no real source wired |
| M3 | **[source]** GitHub App source, settings section, client wiring, status, grant toggles, docs | completes US2/US3 |

## Project Structure

### Documentation (this feature)

```text
specs/1002-sandbox-scoped-token/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── protocol.md
│   ├── git-in-container.md
│   └── github-app-source.md      # [source]
└── tasks.md                      # tasks unit
```

### Source Code

```text
crates/micold-core/src/
├── scoped_credential/            # new
│   ├── mod.rs                    # Grant, SecretToken, ScopedCredential, CredentialSource, NoCredentialReason, CredentialStatus
│   ├── schedule.rs               # renewal state machine
│   ├── git_env.rs                # GIT_CONFIG_COUNT block
│   ├── gitconfig.rs              # sanitizer
│   ├── helper.rs                 # credential-helper request/answer + file format
│   └── github_app.rs             # [source] PEM, JWT, requests, responses
├── sandbox/{mod.rs, argv.rs}     # removal, tmpfs, env
├── kube/{credentials.rs, settings.rs, manifest.rs}
├── daemons.rs                    # MigrationNotice::RemovedGitCredentials
├── paths.rs                      # secrets_dir()   [source]
├── settings.rs                   # settings.github_app: GithubAppSettings   [source]
└── protocol/messages.rs, version.rs
crates/micold-core/tests/{sandbox_credentials.rs, sandbox_argv.rs}
crates/micold-daemon/src/{main.rs, ops.rs, state.rs, git_credential.rs (new)}
crates/micold-daemon/tests/scoped_git_credential.rs (new)
crates/micold-client/src/
├── shell/{sandbox.rs, scoped_credential.rs (new, [source] transport + wiring)}
└── features/{daemons.rs, daemons_kube.rs, daemon_sections.rs, settings section [source]}
docs/user-guide/{sandboxed-daemon.md, kubernetes-daemons.md, settings.md}, docs/daemon.md
```

**Structure Decision**: the existing three-crate layout; all decision logic in `micold-core`
(render-free, `mise run test-core`), process wiring in the daemon, glue in the client.

## Risks

- **Renewal needs the client** (research R8): a daemon left running without its client loses push
  after ≤ 1 h. This is the fail-closed behaviour US3-4 describes, documented, not hidden.
- **Container git older than 2.31** (no `GIT_CONFIG_COUNT`) in a user-chosen image: the helper reset
  would not apply. The basic image's git is newer; a custom image with an older git gets a status
  warning from the daemon's start-up check (`git --version`), and no credential is delivered to it.
- **D7 reversal**: B (pasted PAT) replaces the **[source]** parts and reworks FR-005/008/009/010 —
  the M3 milestone only, if M1/M2 have merged.

## Complexity Tracking

| Item | Why needed | Simpler alternative rejected because |
|---|---|---|
| `ring` (new root workspace dependency) and `base64` added to `micold-core` | RS256 for the App JWT | already in `Cargo.lock` (via `kube`/`rustls`); `jsonwebtoken`/`rsa` add crates and, for `rsa`, an open timing advisory (R4) |
| New protocol messages + version bump | renewal without restart, both runtimes alike | env vars cannot renew; `exec` is a second channel per runtime (R6) |
| `secrets_dir()` outside `data_dir()` | the legacy default sandbox mounts all of `data_dir()` | under `data_dir()` the key would be visible in that sandbox (R5) |
