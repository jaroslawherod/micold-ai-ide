# Tasks: Test-only base-URL override for the GitHub API

**Input**: `specs/1542-github-api-base-override/` (spec.md, plan.md, research.md, data-model.md, contracts/api-base.md, quickstart.md)
**Tests**: required (Constitution I): each test task precedes the code it covers.
**User guide**: no user-facing change; the `feat` PRs carry the `docs-not-needed` label.

Format: `[ID] [P?] [Story] description with file path`. `[P]` = disjoint files, no dependency.

## Phase 1: Foundational (blocks both stories)

- [x] T001 [US2] Core unit tests in `crates/micold-core/src/scoped_credential/pat.rs` (`mod tests`): `ApiBase::default()` and `ApiBase::new("")` give exactly `https://api.github.com/repos/o/r` and a reach URL equal to `REACH_URL`; trailing slash trimmed; path prefix kept well-formed; a recording `Transport` sees only URLs under a given base (FR-005); a malformed base surfaces as `SourceUnreachable` from the transport, never a fallback. Red first (does not compile).
- [x] T002 [US2] Add `ApiBase` (newtype, `Default`, `new`, `probe_url`, `reach_url`) and an `&ApiBase` parameter on `pat::check` and `pat::check_added` in `crates/micold-core/src/scoped_credential/pat.rs`; keep `REACH_URL`; update the existing `pat` tests' calls (FR-001, FR-002, FR-004).
- [x] T003 [US2] Client test in `crates/micold-client/src/shell/scoped_credential/tests.rs`: `PatSource::new` / `PatSource::on_device` and the Settings path use the default base, with a recording transport showing `https://api.github.com/...` URLs (FR-003, SC-003, SC-004). Red first.
- [x] T004 [US2] Thread the base: `PatSource` field (default in `new`/`on_device`) and `check_and_store` parameter in `crates/micold-client/src/shell/scoped_credential.rs`; `check_github_token` in `crates/micold-client/src/shell/settings.rs` passes `ApiBase::default()`; update the callers in `shell/scoped_credential/tests.rs`.

## Phase 2: User Story 1 - Token checks against a fake API in the normal suite (P1) 🎯 MVP

- [x] T005 [P] [US1] Fake GitHub API `crates/micold-client/tests/support/fake_github.rs`: `std::net::TcpListener` on `127.0.0.1:0`, per-test canned routes (probe reply with `github-authentication-token-expiration` header, push permission, 404, private listing), records `(path, Authorization)`, `base() -> ApiBase` (FR-006, FR-011). Its self-test (reply, recorded request) is the first test in `scoped_token_fake_api.rs` (T007).
- [x] T006 [P] [US1] Mark `crates/micold-client/tests/support/fake_github.rs` `micold-daemon-runtime` in `.gitattributes` and add a matching `dr_case` line in `scripts/tests/classify-change.test.sh`.
- [x] T007 [US1] Create `crates/micold-client/tests/scoped_token_fake_api.rs` together with its `[[test]] scoped_token_fake_api` entry in `crates/micold-client/Cargo.toml` (`autotests = false`); first the fake's self-test, then step 2 and 3 through `check_and_store` with `UreqTransport` against the fake: valid token stored with the reported expiry; classic shape, other repository (404), no expiry, token reaching a second private repository each refused with its reason and nothing stored (US1 scenarios 1, 2).
- [x] T008 [US1] Same file: step 8 (remove then re-paste: withdrawn and granted again via `PatSource` without a daemon restart), step 10 (expiry within 24 h warns; past expiry is expired and the token file deleted), FR-005 (the token reaches only the fake; an unreachable base is `SourceUnreachable` and stores nothing), step 12 (Settings help text names *Contents: Read and write*, *Metadata: Read*, the sandbox and *Remove* statements and the *Pull requests* sentence; reuse the existing help-text assertion in `features_settings` if one exists) (US1 scenarios 3-5, US2 scenario 3).
- [x] T009 [US1] Update the "Covered by (not live)" table in `specs/1002-sandbox-scoped-token/quickstart.md`: rows 2, 3, 8, 10, 12 name the new tests; rows 1, 3 (real-API facts) and 7 (`/hooks` 403) stay manual, 9 out of scope; drop the "no override" sentence made false by this feature (FR-009, FR-010).

**Checkpoint**: `scripts/build-lock.sh cargo test -p micold-client --test scoped_token_fake_api` and `mise run test-core` pass.

## Phase 3: User Story 3 - Container and kind credential flow, runtime-gated (P2)

- [ ] T010 [US3] Runtime-gated `crates/micold-daemon/tests/sandbox_real_scoped_token.rs` (`#![cfg(all(feature = "sandbox-real-runtime", unix))]`, includes the fake via `#[path = "../../micold-client/tests/support/fake_github.rs"]`): runs `pat::check` in-process with the fake's base through `UreqTransport` (if not `pub` to the daemon, a 10-line ureq `Transport` local to the test), grants the credential to a container daemon and a kind daemon with `MICOLD_GIT_CREDENTIAL_BASE` on the existing local git HTTP backend of `git_credential_grant.rs`; asserts steps 4-8 and 10, and step 11 for kind; skipped (reported) without runtime or kind (FR-008, US3). Register `[[test]] sandbox_real_scoped_token` in `crates/micold-daemon/Cargo.toml` like `sandbox_real_no_user_credentials`.
- [ ] T011 [US3] Update `specs/1002-sandbox-scoped-token/quickstart.md` rows 4-7, 11 (and 8, 10) to name `sandbox_real_scoped_token`; step 7's `/hooks` 403 stays manual.

**Checkpoint**: with a runtime, `mise run image && mise run test-sandbox` runs it; without, it is skipped.

## Dependencies & order

T001 -> T002 -> T003 -> T004 -> T005 -> T007 (T006 any time after T005's path is fixed, in parallel) -> T008 -> T009. T010 needs T002 and T005; T011 needs T010.

## Implementation Strategy

MVP = M1: the base parameter plus the fake-API tests, which delivers the maintainers' blocker removal and the
production-unchanged guarantee. M2 adds the runtime-gated test.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — ApiBase parameter and fake-API tests for Part C 🎯 MVP

- **Tasks**: T001–T009
- **Deliverable**: the token check takes an injected `ApiBase` (default unchanged) and a reusable fake GitHub API backs normal-suite tests for Part C steps 2, 3, 8, 10, 12; the #1002 quickstart table names them.
- **Satisfies**: US1 acceptance scenarios 1–5; US2 acceptance scenarios 1–3; FR-001–FR-007, FR-009–FR-011; SC-001, SC-003, SC-004
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test scoped_token_fake_api`
- **Depends on**: —
- **Tier**: full

### M2 — Runtime-gated container and kind credential test

- **Tasks**: T010–T011
- **Deliverable**: `sandbox_real_scoped_token` asserts Part C steps 4-8, 10, 11 for a container and a kind daemon in `mise run test-sandbox` (CI `sandbox-runtime`), skipped without a runtime.
- **Satisfies**: US3 acceptance scenarios 1–5; FR-008; SC-002
- **Verify**: `mise run image && mise run test-sandbox` (skips without runtime/kind); `git grep -n sandbox_real_scoped_token specs/1002-sandbox-scoped-token/quickstart.md`
- **Depends on**: M1
- **Tier**: full
