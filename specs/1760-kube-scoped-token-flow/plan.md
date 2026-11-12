# Implementation Plan: Scoped GitHub token reaches a Kubernetes pod daemon

**Branch**: `test/1760_scoped-token-step-11-assert-the-credential-flow-in` | **Date**: 2026-11-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1760-kube-scoped-token-flow/spec.md` (clarified; FR-008 decided by the user: per-daemon opt-in, off by default).

## Summary

A pod daemon gets no `[[github.tokens]]` today. Add, behind a per-daemon opt-in (`KubeSettings::github_tokens`, off by default, a tick in the daemon form), a client-side step in `kube_attempt` that selects the PAT-store tokens whose repository filter matches a repository of this daemon's projects (FR-009), reads each token file (owner-only check, text, size limit), puts the value into the existing startup Secret as key `github-token-<n>` (`StartRequest::secret_files`) and renders `[[github.tokens]]` entries (`name`, `token_file` = the Secret mount path, `repos`, optional `host`) into the pod's `daemon.toml` ConfigMap. The pod daemon already turns those entries into the git credential helper. The deliverable test, `kube_real_scoped_token` in `micold-client`, runs on the kind cluster against a fake GitHub API plus an HTTP git backend the pod reaches through the docker gateway, and asserts FR-003 to FR-005 with a run-unique secret.

## Technical Context

**Language/Version**: Rust (workspace toolchain), iced UI; shell for `scripts/test-kube.sh` (unchanged).

**Primary Dependencies**: existing: `micold-core` (`kube::{settings,start,manifest,names}`, `scoped_credential::PatStore`, `daemon_config::RepoPattern`, `github::GithubRepo`), `micold-client` (`daemon.rs`, `daemon_config_render.rs`, `daemon_config_delivery.rs`, `features/daemons_kube.rs`, `ui/settings/daemons.rs`), `tokio`, std `TcpListener` for the fake forge (as `kube_real_support/services.rs` does for the registry). No new crate.

**Storage**: client PAT store (`PatStore`, owner-only files, read only); `daemons.json` gains one boolean; in the cluster the existing `<name>-startup` Secret gains keys. Nothing remote is assumed (Principle IV).

**Testing**: `cargo test` core units (`micold-core` settings round trip), client units (selection, delivery, render, form draft), existing `kube::start_tests` fake cluster, the runtime-gated `kube_real_*` test (CI `kube-cluster`, `kube-real-runtime` feature), geometry/visual only for the new tick row.

**Target Platform**: Linux, macOS, Windows clients; the kind test is Linux-only.

**Project Type**: desktop app (Rust workspace).

**Performance Goals**: none; delivery adds a handful of small file reads per start.

**Constraints**: a token value never appears in the rendered `daemon.toml`, pod spec, env or client logs (FR-002); Secret limit 1 MiB shared (reuse `POD_FILE_LIMIT`).

**Scale/Scope**: a few tokens per user; one Secret.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 below.*

- [x] **I. Test-First**: every task starts with a failing test: [research.md](research.md) R8 and the layer table below; `tasks` orders tests before code.
- [x] **II. Multi-Session**: state is per daemon (the opt-in lives in that daemon's settings); every session in a pod shares one helper and sees no token (US2).
- [x] **III. Worktree Integration**: no file/VCS operation outside the daemon's existing paths; the project repositories are read from the daemon's projects (R3).
- [x] **IV. Local-First**: tokens stay in the local PAT store; leave the device only into the user's own cluster, only on explicit per-daemon opt-in (FR-008). Works offline for everything but the cluster.
- [x] **V. Rust + iced**: `github_tokens` is a `bool` field with a serde default; the delivered set is a typed `PodGithubTokens` (R4, data-model) so a token without a file path is unrepresentable.
- [x] **VI. Cross-Platform**: the PAT store and its owner-only rules are the existing abstraction; delivery code has no `cfg` of its own; the test is Linux-only by nature (kind), CI builds all three.
- [x] **VII. Documentation**: `docs/user-guide/kubernetes-daemons.md` (new tick, exposure wording, the fixed "Until you paste a GitHub token" paragraph) and `docs/user-guide/settings.md#github-tokens`, same change; quickstart row 11 of `specs/1002-sandbox-scoped-token/quickstart.md` updated.
- [x] **VIII. Reusable UI**: the tick reuses the form's existing checkbox-row builder used by the allowed-credentials list in `ui/settings/daemons.rs`; no feature-local widget.

Post-design re-check: unchanged, all PASS. Complexity Tracking: none.

## Requirement map

| Requirement | Where |
|---|---|
| FR-001 delivery | `PodGithubTokens` ([data-model.md](data-model.md)), `kube_attempt` ([contracts/pod-token-delivery.md](contracts/pod-token-delivery.md) §2) |
| FR-002 no value outside the file | contract §3; render never receives a value; unit test scans config, pod manifest and logs |
| FR-003 helper offers by filter | daemon behaviour already shipped (#1529/#1542); asserted live by the test, not re-implemented |
| FR-004 push/pull, fallback fails | test arms A and C against the HTTP git backend |
| FR-005 no env token, no `SSH_AUTH_SOCK`, no `~/.ssh` | test via new pod-inspect helper; the pod already has no SSH material (no `ssh_key_file` delivered) |
| FR-006 runtime-gated test, skipped without tooling | `kube_real_scoped_token` uses `suite_or_skip`; CI `kube-cluster` runs it with `MICOLD_KUBE_TEST_REQUIRED` |
| FR-007 harness | `kube_real_support/forge.rs` (fake API + git HTTP backend), `session_env_and_home` helper (via `work`) |
| FR-008 opt-in | `KubeSettings::github_tokens`, draft field, tick row, docs; [contracts/opt-in.md](contracts/opt-in.md) |
| FR-009 only matching tokens | `select_tokens` (contract §1) |
| FR-010 lifecycle | startup Secret is replaced/deleted by existing `startup_secret` and removed by lifecycle; contract §4 |

## Test strategy (layer per requirement)

| Layer | Covers |
|---|---|
| core unit (`kube::settings`) | opt-in default off, round trip, old JSON loads off, omitted from JSON when off |
| client unit (`daemon_config_delivery`) | selection by filter and order (FR-009), unreadable/empty/loose-mode/oversize file refuses with the entry name, no value in rendered config (FR-002), opt-in off = no entry and no key (FR-008) |
| client unit (`features/daemons_kube`) | draft tick maps to the setting both ways |
| core `kube::start_tests` | token keys land in the startup Secret, replaced at next start, deleted with the Secret when none (FR-010) |
| client UI/geometry gate | the tick row label names the exposure |
| `kube_real_scoped_token` | FR-003 to FR-006 live; secret scan of env, home and client log (SC-002/SC-003) |
| quickstart §B | visual pass of the form row only |

## Project Structure

### Documentation (this feature)

```text
specs/1760-kube-scoped-token-flow/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── pod-token-delivery.md
│   └── opt-in.md
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code (existing paths; `new` marks files that do not exist yet)

```text
crates/micold-core/src/kube/settings.rs         # KubeSettings.github_tokens (+ Raw, new())
crates/micold-core/src/kube/start.rs            # unchanged: secret_files already carries keys
crates/micold-client/src/daemon_config_render.rs     # GithubTokenSetting.host; for_pod keeps empty tokens
crates/micold-client/src/daemon_config_delivery.rs   # select_tokens, pod_github_tokens, PodStartup.files
crates/micold-client/src/daemon.rs              # kube_attempt/kube_reconnect/Dial::Kubernetes.projects, KubeEnv.tokens (PodTokenSource: pat_root, host)
crates/micold-client/src/features/daemons_kube.rs    # draft field + settings()
crates/micold-client/src/ui/settings/daemons.rs      # tick row
crates/micold-client/tests/kube_real_support/forge.rs   # new: fake forge
crates/micold-client/tests/kube_real_support/mod.rs     # session_env_and_home, start with tokens
crates/micold-client/tests/kube_real_scoped_token.rs    # new
docs/user-guide/kubernetes-daemons.md, settings.md
specs/1002-sandbox-scoped-token/quickstart.md   # row 11
```

**Structure Decision**: extend the existing delivery seam (`pod_startup` and the startup Secret) rather than a new Secret or a new wire message; the delivery stays in `micold-client` beside the other `[[github.tokens]]` code, the opt-in in `micold-core` with the other Kubernetes settings.

## Complexity Tracking

None.
