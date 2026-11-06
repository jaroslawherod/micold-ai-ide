# Feature Specification: Test-only base-URL override for the GitHub API and git host

**Feature Branch**: `feat/1542_base-url-override-for-the-github-api-and-git-host`

**Created**: 2026-11-06

**Status**: Draft

**Input**: Issue #1542 (from #1147; refs #1002): the scoped-token PAT check reaches GitHub only at the
literal `https://api.github.com`, so quickstart Part C of #1002 cannot run against a local fake. Add a
test-only override and turn Part C into automated checks that CI runs.

## User Scenarios & Testing *(mandatory)*

The users here are maintainers and CI: they need Part C of the scoped-token quickstart to run without
a real GitHub account or a real fine-grained token.

### User Story 1 - Token checks run against a fake GitHub API in the normal suite (Priority: P1)

A maintainer runs the ordinary test suite. A local fake GitHub API stands in for `api.github.com`; the
client's token check is pointed at it by a test-only override and the Part C steps that need only the
API run end to end: adding a token (2), the API facts the checks rely on (3), removing and re-pasting
a token (8), expiry and the expiry warning (10), and the Settings help text (12).

**Why this priority**: it removes the blocker named in #1147 and runs on every CI run, with no
container runtime.

**Independent Test**: start the fake API with canned repository, expiry-header, permission and
private-listing replies; run each step's check through the real client token path; assert the
outcomes of the quickstart step.

**Acceptance Scenarios**:

1. **Given** the override points at a fake API answering a valid fine-grained token for `you/scratch`,
   **When** the token is pasted for that repository, **Then** it is stored and the expiry date the
   fake reported is shown.
2. **Given** the fake API answers a classic-shaped token, a token for another repository (404), a
   token with no expiry, or a token that also reaches a second private repository, **When** each is
   pasted, **Then** each is refused with its reason and nothing is stored.
3. **Given** a stored token, **When** it is removed and pasted again, **Then** the credential is
   withdrawn and granted again without restarting the daemon.
4. **Given** a token whose reported expiry is within 24 hours (and, separately, already past),
   **When** the check and schedule run, **Then** the expiry warning (resp. the expired state and
   deletion of the token file) appears.
5. **Given** the paste form, **When** the help text is read, **Then** it names *Contents: Read and
   write* and *Metadata: Read*, the sandbox-can-use-everything and cannot-narrow statements, the
   leaked-token and *Remove* statements, and the *Pull requests* sentence.

---

### User Story 2 - The override cannot weaken the production check (Priority: P1)

A maintainer or reviewer needs confidence that the override is a test seam only: a shipped build
checks tokens against `https://api.github.com` exactly as before and a token is never sent anywhere
else.

**Why this priority**: the check is a security control (the token is scoped to one repository).

**Independent Test**: with no override set, the URLs the check builds are byte-for-byte the current
ones; in a release configuration an override value has no effect.

**Acceptance Scenarios**:

1. **Given** no override, **When** the probe and reach URLs are built, **Then** they are exactly the
   current `https://api.github.com/...` URLs.
2. **Given** a production (non-test) build, **When** the means FR-004 settles on is supplied (if an
   environment variable: set in the environment), **Then** it has no effect on the base, and no
   setting, file or UI exposes it. (Exact form depends on the FR-004 answer.)
3. **Given** an override in a test build, **When** a token check runs, **Then** the token goes only
   to the overridden base, never to `api.github.com`.

---

### User Story 3 - Container and kind run the credential flow, gated on a runtime (Priority: P2)

CI's existing `sandbox-runtime` job runs a runtime-gated test: a container daemon and a Kubernetes
(kind) daemon each serve a clone, are granted the scoped credential from the fake API and the daemon
git helper's base override, and the Part C steps 4-8, 10 and 11 are asserted.

**Why this priority**: it needs a container runtime and kind, so it cannot run in the normal suite;
off by default like the other sandbox tests.

**Independent Test**: `mise run test-sandbox` with a runtime installed runs it; without one it is
skipped, not failed.

**Acceptance Scenarios**:

1. **Given** a container daemon with the credential for `you/scratch`, **When** a session pushes and
   pulls, **Then** both succeed, no token or SSH agent variable is in its environment and `~/.ssh` is
   absent (steps 4, 5).
2. **Given** the same daemon, **When** it asks for another repository, **Then** it gets no credential;
   the token file holds the token (steps 6, 7; the real-GitHub `/hooks` 403 stays manual).
3. **Given** the token is removed, **Then** push fails and, after re-pasting, succeeds without a
   daemon restart (step 8); **Given** the token expires, **Then** the file is gone and push fails
   until a new token is pasted (step 10).
4. **Given** a kind-cluster daemon, **When** steps 4-5 are repeated, **Then** they hold (step 11).
5. **Given** no container runtime or kind, **When** the test runs, **Then** it is skipped.

### Edge Cases

- Override set but empty: treated as unset.
- Override with a trailing slash or a path prefix: URLs stay well-formed.
- Override unreachable (nothing listening): the token is refused as source unreachable, nothing stored.
- Fake API on a random free port, several sessions or tests in parallel: no shared fixed port.
- Per-test isolation: two tests with different fakes in one process or in parallel sessions must not see each other's base (a process-global variable would race).
- Malformed override (not a URL, or a scheme the fake cannot serve): the check fails as source unreachable, never falls back to sending the token to `api.github.com`.
- Windows/macOS: the normal-suite tests run on every CI platform; the runtime-gated test is Linux only.
- GitHub Enterprise hosts keep their `<host>/api/v3` base; the override replaces the default only for tests.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The token check MUST accept a test-only API base that replaces `https://api.github.com`
  for both the probe and the reach requests.
- **FR-002**: With no override, the requests MUST be identical to today's.
- **FR-003**: The override MUST NOT be reachable from any user-facing setting, stored file or UI.
- **FR-004**: The override MUST be supplied to the code under test and, for the runtime-gated test, to the built client or daemon. [NEEDS CLARIFICATION: is the override compiled only into test builds, or an environment variable honoured by shipped builds too (as `MICOLD_GIT_CREDENTIAL_BASE` is in the daemon)? A test-build-only seam is safer; an env var also lets the runtime job drive a built client.]
- **FR-005**: A token MUST be sent only to the base in force; a test MUST show it is not sent to `api.github.com` when overridden.
- **FR-006**: A reusable fake GitHub API MUST serve the repository probe (expiry header, push permission, 404), and the private-repository listing, with per-test canned replies.
- **FR-007**: The normal suite MUST cover Part C steps 2, 3, 8, 10 and 12 through the fake API.
- **FR-010**: The "not live" table in the #1002 quickstart MUST be updated to name the new tests.
- **FR-011**: The fake API MUST run on every CI platform (Linux, macOS, Windows) on loopback.
- **FR-008**: A runtime-gated test MUST cover steps 4-8, 10 and 11 for a container daemon and a kind daemon, run by the existing `sandbox-runtime` CI job, skipped (reported as skipped, not failed) when no runtime is available.
- **FR-009**: Steps that need real GitHub (1, the real-API facts of step 3, the `/hooks` 403 of step 7) MUST remain documented as manual. Step 9 (SSH-agent notice) is already covered by an existing test and is out of scope.

### Key Entities

- **API base override**: a test-only value naming the server the token check calls instead of GitHub.
- **Fake GitHub API**: a local server with canned replies for the probe and listing requests.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 5 of the 12 Part C steps (2, 3, 8, 10, 12) are asserted by tests in the normal suite, with no network access beyond loopback.
- **SC-002**: Steps 4-8, 10 and 11 are asserted by one runtime-gated test in the `sandbox-runtime` CI job (8 and 10 are also covered by the fake-API tests; 4, 5, 6, 7 and 11 are new coverage).
- **SC-003**: With no override, 0 differences in the URLs and headers the check sends (asserted by test).
- **SC-004**: In a shipped build, 0 user-reachable ways (setting, file, UI) to change the base.

## Assumptions

- Real-GitHub-only facts (step 1, API fact verification, `/hooks` 403) stay manual under #1147.
- The fake API is plain HTTP on loopback; the override therefore must allow a non-HTTPS base in tests.
- The git-host half of the title is already covered by the daemon helper's `MICOLD_GIT_CREDENTIAL_BASE`; this feature adds the API half.
- Out of scope: any user-facing GitHub Enterprise setting, changes to the daemon helper's existing override, and the visual pass of status rows (T055).
