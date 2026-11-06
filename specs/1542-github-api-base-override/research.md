# Research: #1542

## R1 Injected base, not env var
Decision: `ApiBase` argument of `check`/`check_added`, default `https://api.github.com`. Rationale: spec FR-004; parallel tests would race on a process-global var; `Transport` is already injected. Rejected: `MICOLD_GITHUB_API_BASE` env var (shipped code would honour it, races); `cfg(test)`-only var (not visible to integration tests or the daemon test); a setting/file (FR-003).

## R2 Where the fake lives
Decision: std-only `TcpListener` fake in `crates/micold-client/tests/support/fake_github.rs`, shared with the daemon test via `#[path]`. Rejected: a new dev-crate (workspace churn for ~150 lines); `httpmock`/`wiremock` (new dependency, license review); reuse of `fake_gh.rs` example (models the `gh` CLI, not REST).

## R3 Runtime test home
Decision: `micold-daemon` test under `sandbox-real-runtime` (existing CI job and `test-sandbox` target pick it up). The client check is `micold-core`, callable in-process from the daemon test with a small `Transport`. Rejected: a client-crate test (not in the `test-sandbox` package list `-p micold-core -p micold-daemon`); a new CI job.

## R4 Plain HTTP base
Decision: `ApiBase` accepts any string URL; `ureq` follows its scheme. Production passes the default constant, so HTTPS stays. Rejected: validating https-only (blocks the loopback fake).

## R5 Kind
Decision: kind steps reuse the container flow with a kind daemon; gated on `kind` and the runtime being installed, else skipped (reported skipped).

## R6 Git host for the runtime test
Decision: reuse the existing local git HTTP backend of `git_credential_grant.rs`/`git_credential_pod.rs` (daemon tests) via `MICOLD_GIT_CREDENTIAL_BASE`. Rejected: a second git server in the fake (duplication).

## R5 addendum
Rejected for kind: a separate kind-only test (duplicates the container flow).
