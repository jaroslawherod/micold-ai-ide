# Feature Specification: Scoped GitHub token reaches a Kubernetes pod daemon

**Feature Branch**: `test/1760_scoped-token-step-11-assert-the-credential-flow-in`

**Created**: 2026-11-12

**Status**: Draft

**Input**: User description: "Scoped-token step 11: assert the credential flow in a kind (Kubernetes) pod" (issue #1760, follow-up of #1542; closes FR-008 / US3.4 / SC-002 of `specs/1542-github-api-base-override`, Part C step 11 of `specs/1002-sandbox-scoped-token/quickstart.md`).

## Background (verified against the code, 2026-11-12)

- `sandbox_real_scoped_token` (the container arm of step 4-8 and 10) was deleted in `1e6d533e5` (#1532): it drove the `ClientMsg::GitCredentialGrant`/`GitCredentialRevoke` messages that #1530 retired. The grant path is now the daemon configuration file's `[[github.tokens]]` entries (`token_file`, optional `repos`, optional `host`). The container arm survives as `crates/micold-daemon/tests/git_credential_config.rs` and `sandbox_real_no_user_credentials`.
- A Kubernetes daemon's configuration is built by `DaemonSettings::for_pod` (`crates/micold-client/src/daemon.rs`, pod start) with an empty `github_tokens`. `pod_startup` delivers only the start script, its env file, the session extras (SSH key, known hosts, signing key, CA file, session files) and the sanitized git configuration. No client path puts a `[[github.tokens]]` entry or its token file into a pod; `GithubTokenSetting::from_store` has no non-test caller. A pod daemon therefore has no scoped credential to offer today, so step 11 cannot be asserted until one is delivered.
- The `kube_real_*` harness (`crates/micold-client/tests/kube_real_support`, CI job `kube-cluster`) has a package registry and a `git daemon` remote (`git://`, no authentication), a pod reaches both through the docker gateway. It has no HTTP git backend or fake GitHub API a pod can reach, and no helper that inspects a pod's environment or home directory.
- Finding: `docs/user-guide/sandboxed-daemon.md` tells the user a sandbox pushes with a pasted token, while no client code delivers one to a container or a pod. This spec covers the pod; the container's delivery gap is recorded as a follow-up, not fixed here.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A pod daemon pushes and pulls with the scoped token (Priority: P1)

A user with a fine-grained GitHub token stored in Settings starts a Kubernetes daemon for a project. Sessions in the pod push and pull the project's repository over HTTPS using that token, and never see the token itself.

**Why this priority**: Without delivery there is nothing to assert; this is the capability step 11 needs.

**Independent Test**: Start a pod daemon against the kind cluster with one stored token naming the project's repository, run a push and a pull from a session against a local HTTP git backend that accepts only that token; both succeed.

**Acceptance Scenarios**:

1. **Given** a stored token whose repository filter matches the project, **When** the pod daemon starts and a session pushes, **Then** the push is accepted by a backend that requires that token.
2. **Given** the same pod, **When** a session pulls the repository, **Then** the pull succeeds with the same token.
3. **Given** the pod has no stored token that applies, **When** a session pushes, **Then** it fails for want of credentials (the sandbox fallback: no inherited helper is consulted).

---

### User Story 2 - The pod holds nothing but the scoped credential (Priority: P1)

The user can rely on the pod not leaking more than the scoped token's reach: no token in any session's environment, no SSH agent and no SSH private key material in the home directory.

**Why this priority**: This is the security claim of FR-008 for the pod arm (US3.4).

**Independent Test**: In a started pod with a granted token, read a session's environment and list the pod home; neither contains the token value, `SSH_AUTH_SOCK` is unset, and `~/.ssh` does not exist.

**Acceptance Scenarios**:

1. **Given** a pod with a granted token, **When** a session's environment is read, **Then** the token's value appears in no variable and `SSH_AUTH_SOCK` is absent.
2. **Given** the same pod, **When** the home directory is listed, **Then** there is no `~/.ssh`.
3. **Given** the pod, **When** the credential helper is asked for another repository or another host, **Then** it offers nothing.

---

### User Story 3 - The kind-cluster test asserts it (Priority: P2)

A maintainer's CI job `kube-cluster` fails when the pod credential flow regresses, and reports "skipped" (not failed) where no cluster tooling exists.

**Why this priority**: It is the deliverable of #1760; it depends on Stories 1 and 2.

**Independent Test**: Run the new `kube_real_` test with the cluster tooling absent (reported skipped) and present (passes); break the delivery (revert it) and see it fail.

**Acceptance Scenarios**:

1. **Given** no `docker`/`kind`/`kubectl`, **When** the test runs, **Then** it is reported skipped.
2. **Given** the kind cluster, **When** the test runs, **Then** it asserts Stories 1 and 2 against a fake GitHub API and HTTP git backend reachable from the pod, with a secret value unique to the run.

### Edge Cases

- The token file is empty, unreadable or malformed: the pod's configuration is not delivered as valid; the start names the entry and no pod starts without it (as for the start script, #1533).
- Two stored tokens reach the same repository: the Settings order decides, as on the host.
- The token expires or is revoked while the pod runs: the pod's push fails; the application does not renew it (documented behaviour of tokens).
- The pod is stopped, deleted or the cluster context changes: no secret holding the token is left in the namespace by this client after the pod is deleted.
- Windows and macOS clients: the token file is read from the client's PAT store with the same owner-only rules (Principle VI); the test itself is Linux-only (kind).
- Two clients start the same pod at once, or the token store changes between start and first use: the pod ends with one complete delivered set (last write wins), never a mix; a token file with loose permissions on Unix is refused by the client, on Windows the ACL rule applies.
- Several sessions in one pod: every session sees the same helper and none sees the token.

## Clarifications

### Session 2026-11-12

- Q: Which stored tokens reach the pod (FR-009)? → A: only those whose repository filter matches a project of this daemon. _(agent-resolved: specs/1002-sandbox-scoped-token/spec.md#FR-005 "only the PAT of the container's own project's repository")_
- Q: When is the token removed from the cluster (FR-010)? → A: with the pod's other secrets on delete/Remove, replaced at each start, withdrawn at the next start once deleted in Settings. _(agent-resolved: docs/user-guide/kubernetes-daemons.md, resource table: startup and credential secrets removed at the next start or Remove; specs/1002-sandbox-scoped-token/spec.md#FR-008)_
- Q: May a host-stored token be placed in a Kubernetes Secret, and with what consent (FR-008)? → A: per-daemon opt-in in the daemon settings, off by default; no Secret holds a token otherwise. _(decided by user)_
- Q: Does a daemon saved before the opt-in existed deliver tokens? → A: no; the opt-in loads as off, so the pod behaves as today until the user turns it on. _(agent-resolved: specs/1002-sandbox-scoped-token/spec.md#FR-002, saved settings load without error; a new item defaults to off)_

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Starting a Kubernetes daemon MUST deliver the selected stored GitHub tokens to the pod so the pod daemon's configured tokens equal the delivered set, each with its name and repository filter, the token's value kept only in a file the daemon reads.
- **FR-002**: A delivered token's value MUST NOT appear in the pod's rendered configuration, any session's environment, the pod specification, or client logs.
- **FR-003**: The pod's git credential helper MUST offer a delivered token for a repository its filter matches and MUST offer nothing for another repository or host.
- **FR-004**: With a delivered token, a session in the pod MUST be able to push to and pull from the granted repository over HTTPS; with none that applies, a push MUST fail for want of credentials.
- **FR-005**: A session in the pod MUST have no `SSH_AUTH_SOCK` and no token value in its environment, and the pod home MUST have no `~/.ssh` directory.
- **FR-006**: A runtime-gated `kube_real_` test in `micold-client` MUST assert FR-003 to FR-005 on the kind cluster, run by CI job `kube-cluster`, and be reported skipped when the cluster tooling is absent.
- **FR-007**: The harness MUST provide, for that test, a fake GitHub API and an HTTP git backend reachable from a pod that accept only the run's token, and a way to read a pod session's environment and list its home directory.
- **FR-008**: A stored token MUST be delivered into the cluster (as a Kubernetes Secret) only when the daemon's settings carry a per-daemon opt-in for GitHub tokens, which is off by default and saved with the daemon's other settings. With it off, no Secret holds a token and the pod has no `[[github.tokens]]` entry. The opt-in's label names the exposure: anyone who can read Secrets in the namespace, and etcd, can read the token. The test enables the opt-in for its daemon.
- **FR-009**: The set of tokens delivered MUST be only those whose repository filter matches a project of this daemon, so a pod never holds a token it cannot use.
- **FR-010**: A delivered token MUST be removed from the cluster with the pod's other secrets when the pod is deleted or removed, MUST be replaced at each start, and a token deleted in Settings MUST be withdrawn at the next start.

### Key Entities

- **Stored token**: a name, a repository filter and a secret value kept owner-only on the client (the PAT store).
- **Pod credential file**: the pod-side file holding one token's value, named by that token's configuration entry.
- **Fake forge**: the test's API and git endpoint that recognises exactly the run's token.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Step 11 of the sandbox scoped-token quickstart is covered by a passing automated test (row 11 no longer UNCOVERED).
- **SC-002**: In that test a unique run secret is found in 0 session environments, 0 files of the pod home, and 0 lines of client log output.
- **SC-003**: Push and pull to the granted repository succeed, and a request for any other repository or host yields no credential, in 100% of the test's attempts.
- **SC-004**: The test is reported skipped, not failed, on a machine without the cluster tooling.

## Assumptions

- The delivery mechanism (a Kubernetes Secret key mounted read-only and named by `token_file`) follows the existing startup-secret pattern; it is a plan decision.
- The per-token `host` key lets the test point the helper at the fake forge; the shipped default stays `github.com` (#1542 FR-003/FR-004: no environment override).
- The container's missing delivery (docs promise it, code lacks it) is out of scope and recorded as a follow-up.
- Out of scope: renewing tokens, a real GitHub check (stays manual, step 7), changing the container or host arms.
