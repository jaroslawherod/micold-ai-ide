# Tasks: Scoped GitHub token reaches a Kubernetes pod daemon

**Input**: `specs/1760-kube-scoped-token-flow/` (spec, plan, research, data-model, contracts, quickstart)
**Tests**: mandatory (Constitution I): every implementation task is preceded by its failing test task. Paths are in `crates/`.
**Format**: `[ID] [P?] [Story] description with file path`

## Phase 1: Setup

- [x] T001 [US1] Reword FR-009 in `specs/1760-kube-scoped-token-flow/spec.md` from "a project of this daemon" to "a saved project of this daemon" (research R3), tick the open-markers item of `checklists/requirements.md` (clarify resolved all three).

## Phase 2: Foundational (blocks every story)

- [ ] T002 [P] Test: `KubeSettings.github_tokens` defaults off, round-trips, an old JSON without the key loads off, omitted from JSON when off, in `micold-core/src/kube/settings.rs`.
- [ ] T003 Add `KubeSettings.github_tokens: bool` (`#[serde(default, skip_serializing_if = "std::ops::Not::not")]`, Raw, `new()`) in `micold-core/src/kube/settings.rs`.
- [ ] T004 [P] Test: `GithubTokenSetting.host` renders `host = "..."` only when `Some`; the value never appears in the rendered config, in `micold-client/src/daemon_config_render.rs`.
- [ ] T005 Add `host: Option<String>` to `GithubTokenSetting` (`from_store` leaves `None`) in `micold-client/src/daemon_config_render.rs`.
- [ ] T006 [P] Test: `select_tokens` keeps Settings order, matches `Exact`/`Owner` case-insensitively, keeps an empty-`repos` token only with at least one repository, returns nothing with no repositories (FR-009), in `micold-client/src/daemon_config_delivery.rs`.
- [ ] T007 Implement `select_tokens` in `micold-client/src/daemon_config_delivery.rs`.
- [ ] T008 [P] Test: `PodGithubTokens::read` refuses an empty, non-UTF-8, over-`POD_FILE_LIMIT` file and, on Unix, a file with group/other bits, each error naming the token; `entries[i].token_file` ends with `files[i].0`; `Debug` shows no value (the value is a `SecretToken`, not a plain `String`), in `micold-client/src/daemon_config_delivery.rs`.
- [ ] T009 Implement `PodGithubTokens` (`entries`, `files`, `read`, the client-side Unix loose-mode check on token files as one `#[cfg(unix)]` helper, values held as `SecretToken`, `token_file` = `<STARTUP_ENV_MOUNT>/github-token-<n>`) in `micold-client/src/daemon_config_delivery.rs`.
- [ ] T010 [P] Test: `ProjectsDial` is the sorted, lower-cased `owner/repo` set from saved projects' `remote`, and a change of it changes the dial identity, in `micold-client/src/daemon.rs`.
- [ ] T011 Add `Dial::Kubernetes.projects: ProjectsDial` and carry it to `kube_attempt` and `kube_reconnect`, in `micold-client/src/daemon.rs`.
- [ ] T012 Add `KubeEnv.tokens: PodTokenSource { pat_root: Option<PathBuf>, host: Option<String> }`; production sets `PatStore::on_device()` and no host, in `micold-client/src/daemon.rs`.

## Phase 3: User Story 1 - A pod daemon gets the scoped token (P1) MVP

**Goal**: with the opt-in on, a pod start delivers the matching tokens; with it off nothing changes.
**Independent test**: the unit and fake-cluster tests below.

- [ ] T013 [US1] Test: opt-in off reads nothing and adds no key or entry; on, token keys reach `StartRequest::secret_files` and `[[github.tokens]]` entries reach `daemon.toml`; a bad token fails with `cannot deliver the GitHub token "<name>" to the pod: <reason>` before any cluster call; a sentinel value is absent from the rendered config, the pod manifest and captured logs (FR-002), in `micold-client/src/daemon.rs` tests.
- [ ] T014 [US1] Wire `kube_attempt`/`kube_reconnect`: when `settings.github_tokens`, `select_tokens(GithubTokenSetting::from_store(PatStore::new(env.tokens.pat_root)), projects)`, `PodGithubTokens::read`, `pod_settings.github_tokens = entries`, `delivered.files` gains the keys; apply `env.tokens.host`, in `micold-client/src/daemon.rs`.
- [ ] T015 [P] [US1] Test: token keys land in the `<name>-startup` Secret, are replaced at the next start, a token deleted in Settings disappears, the Secret is deleted when no key remains, and the first token added or last removed changes the pod's Secret volume (FR-010), in `micold-core/src/kube/start_tests.rs`; a characterization test (may be green at once, the live revert check T026 is the red evidence); fix `start.rs` only if red.
- [ ] T016 [P] [US1] Test: `KubeDaemonDraft.github_tokens` maps both ways through `of(settings)` and `settings()`, in `micold-client/src/features/daemons_kube.rs`.
- [ ] T017 [US1] Add the draft field and mapping in `micold-client/src/features/daemons_kube.rs`.
- [ ] T018 [US1] Test (geometry/UI): the tick row after **Allowed credentials** has the label `GitHub tokens (stored in a Kubernetes Secret: anyone who can read Secrets in the namespace, and etcd, can read them)` and the help line, in `micold-client/src/ui/settings/daemons.rs` tests.
- [ ] T019 [US1] Add the tick row (reuse the checkbox-row builder of the allowed-credentials list) in `micold-client/src/ui/settings/daemons.rs`, then run quickstart §B (visual-pass) for the row and add the tick wording in `docs/user-guide/kubernetes-daemons.md`.
- [ ] T020 [US1] User guide (ships in M1 with the setting, `features/**` is user-facing; word it around the `github_tokens` key in `daemons.json`, no mention of a form tick until M2): the opt-in and its exposure wording, replace the "Until you paste a GitHub token" paragraph in `docs/user-guide/kubernetes-daemons.md`; add the pod note in `docs/user-guide/settings.md#github-tokens`.

## Phase 4: User Story 2 and 3 - The pod holds only the scoped credential; the kind test asserts it (P1/P2)

**Goal**: `kube_real_scoped_token` asserts FR-003 to FR-005 live and is skipped without tooling.
**Independent test**: `mise run test-kube`; revert check of T014 turns it red.

- [ ] T021 [US3] Test (runs without a cluster; written with T023 in the same commit, the forge is its subject): the forge accepts only Basic `x-access-token:<run secret>` (constant-time compare), answers 401 with `WWW-Authenticate` otherwise, records every request's credential, in `micold-client/tests/kube_real_scoped_token.rs`.
- [ ] T022 [US3] Register `kube_real_scoped_token` as a `[[test]]` (`required-features = ["kube-real-runtime"]` as its siblings) in `micold-client/Cargo.toml`.
- [ ] T023 [US3] Add `kube_real_support/forge.rs`: fake GitHub API (`GET /api/v3/...`) plus smart-HTTP git served by `git http-backend` on `0.0.0.0`, reachable at the docker gateway `HOST_IP`; declare it in `micold-client/tests/kube_real_support/mod.rs`.
- [ ] T024 [US2] Add `session_env_and_home` (opens a terminal session via `work`, runs `env`, `ls -a ~` and a recursive `grep -r` of the home for the run secret; the token mount `/run/micold/secrets/startup` is outside the home by design, assert that in the test) and a start helper taking tokens (temp PAT store, forge host, `projects`, opt-in on) in `micold-client/tests/kube_real_support/mod.rs`.
- [ ] T025 [US2] Write `kube_real_scoped_token` with `suite_or_skip` and a run-unique secret: push (A) and pull (B) succeed; a pod with the opt-in on but only a token filtered to another repository fails the push (C), and a second daemon with the opt-in off holds no token key and fails it too (D); the helper offers nothing for another repository or host, probed with `git credential fill` in a session for each; arms C and D are extra daemons in the same run; env checked in two sessions; client log output captured through a `tracing` subscriber writing to a buffer for the run, and scanned; no secret in any session env, no `SSH_AUTH_SOCK`, no `~/.ssh`, secret in 0 home files and 0 client log lines; in `micold-client/tests/kube_real_scoped_token.rs`.
- [ ] T026 [US3] Revert check: run `mise run test-kube` (or leave to CI job `kube-cluster` when no tooling) and with the token keys of T014 commented out confirm the test fails at the push, and that `.github/workflows/ci.yml` job `kube-cluster` runs the test with `MICOLD_KUBE_TEST_REQUIRED` set; record the result in the PR body.
- [ ] T027 [US3] Mark row 11 COVERED by `kube_real_scoped_token` in `specs/1002-sandbox-scoped-token/quickstart.md`.

## Dependencies and order

Phase 2 before Phase 3 (T003, T005, T007, T009, T011, T012 feed T014). T014 before T019's row is reachable. Phase 4 needs T014 and T003. `[P]` tasks touch disjoint files.

## Implementation Strategy

MVP first: the delivery path (T001-T020) is useful alone and verified by unit and fake-cluster tests; the live test then asserts it. Tests precede code within each pair.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Delivery of scoped tokens to a pod 🎯 MVP

- **Tasks**: T001–T017, T020
- **Deliverable**: with `github_tokens` on in a daemon's saved settings, a pod start puts the matching PAT-store tokens in the `<name>-startup` Secret and `[[github.tokens]]` in the pod's `daemon.toml`; off (the default) changes nothing. The form tick lands in M2, so no UI is half-wired (milestones rule 6); the setting is only reachable through `daemons.json` until then.
- **Satisfies**: US1 acceptance scenarios 1–3 (unit level; live in M3); FR-001, FR-002, FR-008 (setting, off = nothing), FR-009, FR-010
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core kube::` and `scripts/build-lock.sh cargo test -p micold-client daemon_config_delivery`
- **Depends on**: —
- **Tier**: full

### M2 — The GitHub tokens opt-in in the daemon form

- **Tasks**: T018–T019
- **Deliverable**: Settings -> Daemons -> a Kubernetes daemon shows the **GitHub tokens** tick (off by default, label names the exposure) (the user guide, shipped in M1 with the setting, gets the tick's wording in T019).
- **Satisfies**: FR-008 (the label and help, exposure wording); Constitution VII
- **Verify**: quickstart §B; `scripts/build-lock.sh cargo test -p micold-client features::daemons_kube`
- **Depends on**: M1
- **Tier**: light

### M3 — The kind test asserts the pod credential flow

- **Tasks**: T021–T027
- **Deliverable**: `kube_real_scoped_token` runs in CI job `kube-cluster`, asserts push, pull, the no-credential fallback and the empty environment and home against a fake forge, and is skipped without tooling; row 11 of the 1002 quickstart is COVERED.
- **Satisfies**: US2 acceptance scenarios 1–3; US3 acceptance scenarios 1–2; FR-003 to FR-007; SC-001 to SC-004
- **Verify**: `mise run test-kube` (prints `SKIP` without tooling) and the `kube-cluster` CI job green on the PR; T026's result is the evidence
- **Depends on**: M1
- **Tier**: full
