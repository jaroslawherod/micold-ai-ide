# Implementation Plan: Test-only base-URL override for the GitHub API

**Branch**: `feat/1542_base-url-override-for-the-github-api-and-git-host` | **Date**: 2026-11-06 | **Spec**: [spec.md](spec.md)

## Summary

The token check in `micold-core` (`scoped_credential/pat.rs`) builds its URLs from the literal
`https://api.github.com`. Add an `ApiBase` value the callers pass into `check` and `check_added`
(FR-004: an injected parameter, no env var, setting or file). Production callers pass
`ApiBase::default()` (byte-identical URLs). Tests build a base from a loopback fake. A reusable fake
GitHub API (plain HTTP, random port) backs normal-suite tests for Part C steps 2, 3, 8, 10, 12 and a
runtime-gated test in `micold-daemon` for steps 4-8, 10, 11. The #1002 quickstart "not live" table is
updated.

## Technical Context

**Language/Version**: Rust (workspace edition), no new crate dependencies (the fake uses `std::net`; the client path uses the existing `ureq` 3).
**Primary Dependencies**: `micold-core::scoped_credential::pat` (`Transport`, `check`, `check_added`, `REACH_URL`, `probe_url`); `micold-client::shell::scoped_credential` (`UreqTransport`, `PatSource`, `check_and_store`); `micold-daemon::git_credential` (`BASE_ENV`).
**Storage**: N/A (the PAT store is untouched).
**Testing**: `cargo test` (core unit, client integration), `sandbox-real-runtime` feature tests run by `mise run test-sandbox` / CI `sandbox-runtime`.
**Target Platform**: Linux, macOS, Windows for the normal-suite tests; Linux only for the runtime-gated one.
**Project Type**: desktop app workspace (core, client, daemon).
**Performance Goals**: none.
**Constraints**: loopback only; no new user-reachable setting; with the default base the requests are unchanged.
**Scale/Scope**: one parameter threaded through 2 functions and 2 call sites, one fake, ~5 test files.

## Constitution Check

- [x] **I. Test-First**: every task writes the failing test first (URL-shape unit tests, then the fake-API step tests, then the runtime test).
- [x] **II. Multi-Session**: no session state; the base is a per-call argument, so parallel tests cannot race (no process global).
- [x] **III. Worktree**: N/A, no VCS or worktree operation changes.
- [x] **IV. Local-First**: the fake is loopback; the default base is unchanged; nothing new leaves the device.
- [x] **V. Rust + iced**: Rust only; `ApiBase` is a newtype so a bare string cannot be passed where a base is wanted; no UI change.
- [x] **VI. Cross-Platform**: fake uses `std::net` on loopback; normal-suite tests run on all three CI platforms; the runtime test is `cfg`/feature-gated Linux.
- [x] **VII. Documentation**: no user-facing change; the #1002 quickstart table is updated (FR-010).
- [x] **VIII. UI Foundation**: N/A, no UI.

Post-design re-check: unchanged, PASS. Complexity Tracking: none.

## Project Structure

```text
specs/1542-github-api-base-override/
├── plan.md  research.md  data-model.md  quickstart.md
└── contracts/api-base.md

crates/micold-core/src/scoped_credential/pat.rs     # ApiBase, check/check_added take &ApiBase
crates/micold-core/src/scoped_credential/reach.rs   # existing api_base(host) (GHE) unchanged
crates/micold-client/src/shell/scoped_credential.rs # PatSource field + check_and_store param
crates/micold-client/tests/support/fake_github.rs   # shared fake (new), include!/#[path]
crates/micold-client/tests/scoped_token_fake_api.rs # steps 2, 3, 8, 10, 12 (new)
crates/micold-daemon/tests/sandbox_real_scoped_token.rs # steps 4-8, 10, 11 (new, gated)
specs/1002-sandbox-scoped-token/quickstart.md       # "not live" table
```

## Design by requirement

| Requirement | Where |
|---|---|
| FR-001, FR-004 | `ApiBase` param on `pat::check` / `pat::check_added`; `probe_url` and the reach URL are built from it. See contracts/api-base.md. |
| FR-002, SC-003 | `ApiBase::default()` yields exactly `https://api.github.com`; core unit test asserts the old URL strings and `REACH_URL` equality. |
| FR-003, SC-004 | Production callers (`PatSource::new`, Settings `check_github_token`) pass the default; no env/file/setting read; the only non-default base is built in tests (`ApiBase::new`); a client test asserts the production constructors use the default. |
| FR-005 | Fake records request URLs/Authorization; a test asserts the token only reached the fake, and a recording `Transport` shows no `api.github.com` URL when a base is given. |
| FR-006, FR-011 | `fake_github.rs`: `std::net::TcpListener` on `127.0.0.1:0`, per-test canned routes (probe reply with expiry header/permissions/404, private listing). |
| FR-007 | `scoped_token_fake_api.rs` drives `shell::scoped_credential::check_and_store` and `PatSource` with `UreqTransport` against the fake; step 12 asserts the Settings help text. |
| FR-008, FR-009 | `sandbox_real_scoped_token.rs` in `micold-daemon`, `sandbox-real-runtime` feature: calls the core check in-process with the fake's base, grants the credential to container and kind daemons with `MICOLD_GIT_CREDENTIAL_BASE`; skipped when no runtime. Run by `test-sandbox` (CI `sandbox-runtime`, `daemon_runtime` path filter). |
| FR-010 | quickstart table edit. |

Edge cases: empty override treated as default (`ApiBase::new` normalizes empty to default); trailing
slash trimmed; malformed base yields `SourceUnreachable` from the transport and never falls back.

## Test strategy

- core unit (`pat.rs` tests): default URLs identical, trailing slash, empty, path prefix.
- client integration with the fake: steps 2, 3, 8, 10, 12, and FR-005.
- daemon runtime-gated: steps 4-8, 10, 11 (skipped without runtime/kind).
- No geometry or visual pass (no UI change).
