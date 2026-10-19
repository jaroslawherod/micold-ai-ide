# Implementation Plan: Scoped, expiring git credential for sandboxed daemons

**Branch**: `claude/project-thread-2cdrmk` | **Date**: 2026-10-19 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1002-sandbox-scoped-token/spec.md` (issue #1002)

**Decided (ledger D7, jaro, 2026-10-19)**: the credential source is a **fine-grained PAT per
repository that the user creates by hand and pastes into Settings** (GitHub's shortest expiry is 1 day;
no automatic renewal). The GitHub App design was rejected and its M5 abandoned. Everything specific
to the source sits behind one seam (research R3), marked **[source]** below. D19 records what that
changes in FR-008–FR-011 and in M4's 1-hour assumptions.

## Summary

Container and Kubernetes daemons stop receiving the user's git identity: the *SSH agent*, *Git
credentials* and *SSH key* items are removed (old saved settings load, are ignored and dropped,
with a one-time notice), and the shared `~/.gitconfig` is sanitized of helpers and embedded tokens.
In their place the client delivers, for each repository a daemon's project pushes to, the
fine-grained PAT the user pasted for that repository (checked: fine-grained, has an expiry, pushes to
that repo, reaches no other private repo where visible), over the existing authenticated protocol.
The daemon keeps it on an in-memory filesystem inside the container, and a `micold-daemon
git-credential` helper, made the only helper by a command-scope git environment, hands it to git for
that repository only, over HTTPS (SSH-form remotes rewritten). The client does not renew: it
warns 24 h before the PAT's expiry, withdraws it at expiry, delivers a replacement in place, never
revokes the user's PAT on GitHub, and when no usable PAT exists the daemon runs without one and its
status says which project and why.

## Technical Context

**Language/Version**: Rust (stable, workspace toolchain)

**Primary Dependencies**: existing `ureq` (client HTTP), `serde_json`. No new dependency (no `ring`,
no `base64`: nothing is signed, R4 superseded).

**Storage**: the pasted PATs owner-only in `paths::secrets_dir()` (new) = sibling of `data_dir()` (research R5;
no OS keychain crate in the workspace); in memory on the client while delivering, and on a tmpfs /
memory `emptyDir` in the container only (research R6). Nothing new under `data_dir()`, nothing in the
settings document.

**Testing**: `cargo test` — core unit tests (pure), `micold-core/tests/sandbox_credentials.rs` and
`sandbox_argv.rs` (FR-014, container), `kube/start_tests.rs` with `kube-fake` (FR-014, Kubernetes),
a new `micold-daemon/tests/scoped_git_credential.rs` running real `git` (FR-015), the real-runtime
sandbox suite for a live inspection (off by default), quickstart Part C for live GitHub. The new settings rows (*GitHub access tokens* section,
status rows) are compositions of existing shared components with no new geometry; their reducer
behaviour is tested in the client, and their look gets one `visual-pass` run (quickstart Part C
steps 2, 12 in M5; the status rows' pass, steps 4, 8, 10, needs a connected daemon and runs in the close unit with T056; a 1-day token shows the expiry warning from the day it is pasted) — no new geometry gate.

**Target Platform**: client on Linux, macOS, Windows; the container is Linux in every case.

**Project Type**: desktop application (iced client) + daemon + render-free core library.

**Performance Goals**: checking a PAT is ≤ 2 HTTPS round trips per repository, at paste and at each
delivery; daemon start is never blocked on it (credentials arrive after `Attach`).

**Constraints**: fail closed (FR-012); no project-hosted service (D6); a PAT enters only the
container of a daemon serving its repository, is never logged, never leaves the device otherwise, and is never on a host-mounted path (FR-005, R5); offline → daemons start without credentials
(constitution IV).

**Scale/Scope**: a handful of daemons × a handful of repositories per user.

## Constitution Check

*Pre-design and post-design: both PASS.*

- [x] **I. Test-First**: every decision is in pure core (PAT checks and parsing, stored file,
  schedule (`AtExpiry`), git env, sanitizer, helper parsing, migration, status texts) or the daemon (helper,
  file store, message handling), each with tests first. Client glue (HTTP transport over `ureq`,
  settings rows, subscription wiring) is thin and follows the Principle I exception, covered by
  quickstart Part C; the transport's status-to-reason mapping is in core and tested.
- [x] **II. Multi-Session**: credentials are per daemon and per repository; sessions, and daemons, using one repository share its stored PAT (spec edge case); no session state added.
- [x] **III. Worktree Integration**: worktrees of a project share its `origin`, so they share its
  credential; no worktree lifecycle change.
- [x] **IV. Local-First**: the PATs stay on the device; network calls go only to GitHub, only for a
  sandboxed daemon serving a GitHub project, after the user pasted a PAT for that repository (explicit opt-in);
  offline the feature degrades to "no credential" and everything else works.
- [x] **V. Rust + iced**: yes. `Grant` makes a credential without contents/metadata
  unrepresentable; `ScopedCredential` can only be built through the narrowing check.
- [x] **VI. Cross-Platform**: core is OS-agnostic; the one platform fact (where `secrets_dir()`
  lands) is a `directories`-derived path tested for all three layouts; the container side is Linux
  everywhere. The removed agent-socket path was the only Unix-specific credential mount.
- [x] **VII. Documentation**: `docs/user-guide/sandboxed-daemon.md`, `kubernetes-daemons.md` and
  `settings.md` updated in the same change: creating and pasting a PAT, the removed items, statuses,
  the no-renewal and leak-until-expiry notes.
- [x] **VIII. Reusable UI**: the *GitHub access tokens* section and the status rows use
  the existing settings field, button and status-row components; no new widget.

## Design overview

```text
client (device)                                    sandbox (container / pod)
┌───────────────────────────────────────┐          ┌─────────────────────────────────────┐
│ Settings › GitHub access tokens [src] │          │ micold-daemon                        │
│   pasted PAT per repo → secrets_dir() │          │  ├ GitCredentialTargets  ───────────►│ (to client)
│ PatSource  [source]                   │◄─────────┤  │  project → origin push URL         │
│   impl CredentialSource (seam)        │ protocol │  ├ GitCredentialGrant/Revoke ──────► │ /run/micold/git/<o>/<r>  (tmpfs, 0600)
│ Schedule (core): at expiry, no renewal│─────────►│  └ git-credential get  ◄── git       │
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
3. **[source] Pasted PAT** — `scoped_credential/pat.rs` (shape check, probe and header parsing,
   HTTP-status → reason, stored-file format, `ScopedCredential::pasted`); `paths::secrets_dir()`;
   client `shell/scoped_credential.rs` (`PatSource` over `ureq`, the per-daemon wiring), `scoped_credential/pat_store.rs` (new, `PatStore`), *GitHub
   access tokens* section in the client's settings feature (contracts/pat-source.md §1). M4 rework
   (in M5): `MAX_LIFETIME_SECS` → 366 days (core and the daemon's grant check), `RenewPolicy::AtExpiry`
   and `Schedule::refresh` in `schedule.rs`, `CredentialStatus::expires_within`, `PatExpired`.
4. **Protocol** — `DaemonMsg::GitCredentialTargets`, `ClientMsg::GitCredentialGrant`,
   `ClientMsg::GitCredentialRevoke`; version bump (contracts/protocol.md).
5. **Daemon** — targets reporting (`ops.rs`/`state.rs`, on attach and project/remote change),
   file store under `/run/micold/git` with a test root, `git-credential` subcommand in `main.rs`
   beside `status-line`/`remote`.
6. **Runtimes** — container argv: `--tmpfs /run/micold/git…` and the `git_env` block; pod manifest:
   memory `emptyDir` at `/run/micold/git`; `daemon_env` merges `git_env`.
7. **Client wiring** — per daemon `Schedule` (`AtExpiry`) driven by the subscription clock while
   connected; PAT checks off the UI thread; grants/revokes sent on the daemon's connection; copies
   withdrawn on daemon stop/delete (the PAT is not revoked at GitHub); status rows with the 24 h
   expiry warning (FR-013, FR-008); the PAT permission guidance text at the point of pasting (FR-011;
   no per-daemon toggle).
8. **Docs** — the three user-guide pages: `settings.md` gains *GitHub access tokens* (creating the
   fine-grained PAT step by step with the permissions and a short expiry, pasting, replacing, the 24 h
   warning, removing, and that removing does not delete it on GitHub); `sandboxed-daemon.md` and
   `kubernetes-daemons.md` gain the statuses and remedies, that pushes use the pasted per-repository
   token over HTTPS (SSH remotes rewritten), the no-renewal and leaked-until-expiry notes;
   `docs/daemon.md` gains the new messages.

### Requirement traceability

| FR / SC | Where | Tested by |
|---|---|---|
| FR-001 | area 1; contracts/git-in-container.md §4 | core `sandbox_credentials`/`sandbox_argv`, `kube::start_tests`, real-runtime sandbox suite |
| FR-002 | area 1; data-model §9 | core unit (legacy profile/settings read, dropped on save) |
| FR-003 | area 1; data-model §9, research R10 | core unit (notice raised once); client reducer test (notice shown, dismissed) |
| FR-004 | area 1; research R7; contracts/git-in-container.md §1, §3 | core unit (sanitizer, env block); daemon `scoped_git_credential` (planted helpers do not run) |
| FR-005 | areas 2–3; research R2–R3, R5; contracts/pat-source.md §1, §3 | core unit (stored PAT → credential for its repo only); client reducer test (paste, replace, remove); quickstart C |
| FR-006 | area 6; research R6; data-model §10 | core argv/manifest tests (tmpfs/memory, not a host path); daemon unit (file mode, path) |
| FR-007 | areas 5–6; contracts/git-in-container.md §1–2 | daemon `scoped_git_credential` (push/pull, SSH-form rewrite, other repo/host empty) |
| FR-008 | area 7; data-model §8; research R8 | core unit (`AtExpiry`: no renewal, `refresh`, expiry → `PatExpired`, `expires_within` 24 h); client reducer test (warning shown) |
| FR-009 | data-model §2; contracts/pat-source.md §2 | core unit (classic/no-expiry/expired/refused/other-private-repo PATs rejected; `pasted` refuses past and > cap expiry); daemon unit (grant check cap) |
| FR-010 | area 7; data-model §8; contracts/pat-source.md §4 | core unit (`withdraw` on removed/expired/refused); client wiring test T043 (`PatSource::revoke` does nothing, no request to GitHub, stop/delete drops copies, reconnect withdraws); daemon unit (revoke removes file) |
| FR-011 | area 7; contracts/pat-source.md §1 | client reducer test (help text names the permissions and the leak consequence; no toggle exists) |
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
| M3 | **[source]** pasted-PAT source, settings section, client wiring, status, guidance text, docs | completes US2/US3 |

Superseded by tasks.md, which cuts these finer (six milestones, ledger D11): the sanitized gitconfig and `git_env` move to M2, and the **[source]** work is milestone M5.

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
│   └── pat-source.md             # [source]
└── tasks.md                      # tasks unit
```

### Source Code

```text
crates/micold-core/src/
├── scoped_credential/            # on main since M1-M4; M5 changes mod.rs, schedule.rs, fake.rs
│   ├── mod.rs                    # Grant, SecretToken, ScopedCredential, CredentialSource, NoCredentialReason, CredentialStatus (M5: MINTED/MAX lifetime, `pasted`, PatExpired, docs of `Grant::permissions` and `CredentialSource::revoke` that say "installation token"/"within an hour")
│   ├── schedule.rs               # renewal state machine (M5: RenewPolicy, refresh, withdraw)
│   ├── fake.rs                   # FakeSource (M5: MINTED_LIFETIME_SECS)
│   ├── git_env.rs, gitconfig.rs   # unchanged
│   ├── helper.rs                 # (M5: one doc comment says "installation token")
│   ├── pat.rs                    # NEW [source] checks, parsing, stored-file format
│   └── pat_store.rs              # NEW [source] PatStore over secrets_dir()
├── sandbox/{mod.rs, argv.rs}     # removal, tmpfs, env
├── kube/{credentials.rs, settings.rs, manifest.rs}
├── daemons.rs                    # MigrationNotice::RemovedGitCredentials
├── paths.rs                      # secrets_dir()   NEW [source]
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

- **A PAT lives as long as the user made it** (research R2, R8): at least a day, up to a year. A
  PAT in a sandbox is exposed to code running there for that long, and the client cannot narrow or
  revoke it. Mitigations: FR-009 refuses classic and no-expiry tokens, the settings text recommends
  a short expiry and one repository, and the 24 h warning. Accepted by jaro with D7.
- **No renewal**: a PAT that expires while the daemon runs ends pushes until the user pastes a new one
  (US3-4), by design.
- **Container git older than 2.31** (no `GIT_CONFIG_COUNT`) in a user-chosen image: the helper reset
  would not apply. The basic image's git is newer; a custom image with an older git gets a
  status (`NoCredentialReason::GitTooOld`) from the daemon's start-up check (`git --version`, sent
  as `GitCredentialTargets.git_too_old`), and no credential is delivered to it.
- **API facts unverified offline** (M5 builds on fixtures; the close unit's live run, T056, confirms them): the expiry header on `GET /repos/{o}/{r}`, `permissions.push`
  in that body for a fine-grained token, `403`/`404` meaning "does not reach", and that
  `GET /user/repos?visibility=private` is limited to a fine-grained token's repositories are
  GitHub documentation as the author recalls it; M5 builds the checks on fixtures and quickstart
  Part C step 3 confirms them live, with the documented fallbacks (contracts/pat-source.md §2).

## Complexity Tracking

| Item | Why needed | Simpler alternative rejected because |
|---|---|---|
| (none; the `ring`/`base64` row of the App design is withdrawn with D7) | — | — |
| New protocol messages + version bump | delivering and replacing a credential without a restart, both runtimes alike | env vars cannot be replaced; `exec` is a second channel per runtime (R6) |
| `secrets_dir()` outside `data_dir()` | the legacy default sandbox mounts all of `data_dir()` | under `data_dir()` the PATs would be visible in that sandbox (R5) |
