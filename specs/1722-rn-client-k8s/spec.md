# Feature Specification: A React Native client for web and mobile, and the orchestrator on Kubernetes

**Feature Branch**: `claude/project-thread-zmzag6`

**Created**: 2026-11-14

**Status**: Draft (planning only, awaiting owner decisions in [plan.md](plan.md#open-questions-for-jaro))

**Input**: Owner request (jaro, 2026-11-14): "I would like you to do planning of migration into web
interface that was mentioned for orchestrator. It should use react native to support web and mobile
app, use material 3 theme. Should support login via GitHub or Google and have basic permission
support. It should be considered to run on k8s cluster." Follow-up (jaro, 2026-11-14): "Initially
assume similar features and look as current rust ui."

> **Planning only.** This revises milestones M3–M5 of epic #1722 (`specs/orchestrator/`). It changes
> no source file. Each issue proposed in [issues.md](issues.md) is planned and implemented later in
> its own worktree with `/speckit-autopilot #<issue>`, after jaro approves the list. Nothing here
> creates or edits a GitHub issue.

## What this revises

`specs/orchestrator/spec.md` planned M3 (network API, users, roles), M4 (a browser-only TypeScript
client under `web/`, structured view only) and M5 (cloud deployment with compose and Helm examples).
Where this spec differs, it wins:

| Topic | `specs/orchestrator/` (2026-11-05) | This revision (2026-11-14) |
|---|---|---|
| Client stack | TypeScript web app under `web/`, framework open | React Native (Expo, `react-native-web`): one codebase for web, Android and iOS |
| Look | not specified | Material 3, the desktop app's own tokens (seed `#6750A4`, Roboto), light and dark |
| Features | structured view only | the desktop app's features where a phone or browser can do them (parity matrix in [plan.md](plan.md#feature-parity-matrix)); terminal view is an open question |
| Sign-in | OIDC plus a local owner account with a password | GitHub and Google only; no passwords; owner bound by a one-time bootstrap code |
| Permissions | owner / operator / viewer with per-daemon grants | owner / operator / viewer as one global role per user; per-daemon grants deferred |
| Deployment | compose and Helm examples, one volume | Kubernetes first: image, Helm chart, ingress, daemon mTLS links on their own Service (LoadBalancer or TLS passthrough), secrets as files, probes, metrics, one active replica |

M1 (#1723–#1727) and M2 (#1728–#1731) are unchanged and remain prerequisites.

## Clarifications

### Session 2026-11-14

- Q: Which React Native setup? → A: Expo (managed workflow, Expo Router) with `react-native-web` for
  the browser build. One TypeScript codebase; platform files (`*.web.tsx`, `*.native.tsx`) only where
  a platform differs. _(agent-decided; alternatives in [plan.md](plan.md#research-and-rejected-alternatives))_
- Q: How is Material 3 done? → A: `react-native-paper` v5 (its MD3 theme), with every colour,
  type, shape, elevation and motion value generated from `crates/micold-core/src/tokens/`, the same
  source the desktop app and the docs site (`micold-tokens-css`) use. No hand-kept copy. _(agent-decided)_
- Q: Which features first? → A: Same features and look as the current Rust UI, wherever the
  platform allows (jaro, 2026-11-14). The parity matrix says which desktop surface ships in which
  milestone and which stay desktop-only.
- Q: GitHub or Google, with or without the local owner account? → A: Both GitHub and Google; the
  local owner account with a password is **replaced**. The first owner is bound by a one-time
  bootstrap code the orchestrator prints on first start (and writes to a file / Kubernetes Secret);
  whoever signs in with GitHub or Google and enters it becomes owner. No password is stored
  anywhere. _(agent-decided, as the brief allowed; reason: no password store to secure, and a
  self-hosted deployment still needs a GitHub or Google OAuth app only, which both offer free)_
- Q: Does the local desktop setup now need a GitHub or Google login? → A: No. A local orchestrator
  reached over its local socket trusts the local OS user as owner, as today. Sign-in applies only to
  the network API (FR-017 binds it to localhost unless configured). _(agent-decided)_
- Q: Who may sign in? → A: Only identities the owner invited (a GitHub login or a Google email);
  anyone else gets "not invited" and nothing is stored about them beyond an audit entry.
  _(agent-decided)_
- Q: What does "basic permission support" mean? → A: Three global roles per user: owner (everything,
  users, invites, audit, daemon revocation), operator (drive sessions and worktrees), viewer (read
  only). Per-daemon and per-project grants are deferred; every record already carries a tenant id.
  _(agent-decided)_
- Q: Is the orchestrator stateless on Kubernetes? → A: It stores no session (sessions stay in the
  daemons), but it does own the registry, CA, users, invites, refresh tokens and audit log. v1 runs
  **one active replica** with a persistent volume (StatefulSet, `replicas: 1`); storage sits behind
  store traits (the `RegistryStore` that #1724's M5 plans, and a new `AuthStore`), which keeps a later
  move to Postgres and several replicas open. The web bundle is a separate,
  stateless image that scales freely. _(agent-decided; HA is an open question)_
- Q: Push notifications on mobile? → A: Out of scope for v1 (as Web Push was): attention shows while
  the app is in the foreground, plus local notifications while it runs. Remote push is an open
  question. _(agent-decided)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Sign in from a phone or a browser with GitHub or Google (Priority: P1)

The owner deployed the orchestrator to their cluster. On their phone they open the Micold app (or
the web URL), tap "Sign in with GitHub", approve, and see their projects. A colleague they invited by
Google email signs in with Google and gets the viewer role.

**Independent Test**: Against an orchestrator with a fake OIDC/OAuth provider, a web build and an
Android build each sign in with GitHub and with Google; an uninvited identity is refused.

**Acceptance Scenarios**:

1. **Given** a fresh deployment, **When** the first person signs in and enters the bootstrap code,
   **Then** they become owner and the code is spent; without the code nobody becomes owner.
2. **Given** an invited Google email, **When** that person signs in with Google, **Then** they get the
   role on the invite (viewer by default).
3. **Given** an identity that was not invited, **When** it signs in, **Then** it is refused with "not
   invited" and no user record is created.
4. **Given** a signed-in app, **When** the access token expires, **Then** the app refreshes it without
   asking the user; a revoked user's refresh is refused and the app returns to sign-in.
5. **Given** the mobile app, **When** sign-in finishes in the system browser, **Then** the app receives
   the result through its redirect and no provider secret is ever in the app.

---

### User Story 2 - The same app, the same look, on every screen size (Priority: P1)

The user opens the web client on a laptop and sees the desktop app's layout: sidebar of projects,
worktrees and sessions, the overview grid of cards, and the session view, in the desktop app's
Material 3 colours, light or dark. On a phone the same screens stack: list, then session.

**Independent Test**: Screenshots of the overview and a structured session at 1280 px (web), 800 px
and 390 px (phone) in light and dark match the desktop app's colour roles and type scale (token
values compared by a test, layout by a visual pass).

**Acceptance Scenarios**:

1. **Given** the desktop tokens change, **When** the client is built, **Then** its theme changes with
   them; CI fails if the generated theme file is stale.
2. **Given** the OS is in dark mode, **When** the app starts with "follow system", **Then** it is dark,
   as the desktop app is.
3. **Given** a phone-width screen, **When** the user opens a session, **Then** the list gives way to
   the session with a back action, and nothing scrolls sideways.

---

### User Story 3 - Read and drive a structured session (Priority: P1)

As `specs/orchestrator/` User Story 4: list projects, worktrees and sessions in a stable order with
state, open a structured session, read the live transcript, answer a permission prompt, paste and
send, switch model, effort and permission mode, start and stop sessions and worktrees. A viewer sees
no composer and no changing action.

**Independent Test**: As SC-006 of `specs/orchestrator/`, from the web build and the Android build.

**Acceptance Scenarios**: those of `specs/orchestrator/` User Story 4, plus:

1. **Given** the phone app goes to the background for five minutes, **When** it returns, **Then** it
   reconnects and resumes the transcript by sequence number with nothing lost or duplicated.
2. **Given** an operator and a viewer, **When** the viewer opens the same session, **Then** they see
   the transcript update live and no input control.

---

### User Story 4 - Review changes and see issue and PR status (Priority: P2)

The user opens a worktree's changes from the web client, reads the diff tree and the file diffs as in
the desktop review view, and sees the worktree's GitHub PR and Jira status chips.

**Independent Test**: A worktree with changes shows the same file tree and diff as the desktop app.

**Acceptance Scenarios**:

1. **Given** a worktree with changes, **When** the user opens Changes, **Then** the tree and diffs
   match the desktop app's for that worktree.
2. **Given** a worktree linked to a PR, **When** its row is shown, **Then** the PR state chip matches
   the desktop app's.

---

### User Story 5 - Run it on my Kubernetes cluster (Priority: P2)

The owner runs `helm install micold deploy/helm/micold` with a hostname, a cert-manager issuer, and
GitHub and Google OAuth client secrets. They get a web URL, a daemon-link endpoint and a bootstrap
code (in the pod log and in a file on the orchestrator's volume, or taken from a Secret they created
beforehand). Daemons on their laptop, on SSH hosts and in the same cluster enroll and link over mTLS.

**Independent Test**: In a kind cluster in CI, install the chart with a pre-created bootstrap-code
Secret, sign in through a fake provider, enroll a daemon pod, drive a structured session through the
ingress.

**Acceptance Scenarios**:

1. **Given** the chart installed, **When** the pod is ready, **Then** `/readyz` is 200 only after the
   store, CA and listeners are up, and `/healthz` stays 200 while the process can serve.
2. **Given** a pod restart or node drain, **When** the new pod starts, **Then** registry, users,
   invites and audit log are intact, and daemons and clients reconnect on their own.
3. **Given** the ingress terminates TLS for the web and API host, **When** a daemon links, **Then** its
   mTLS reaches the orchestrator untouched (TLS passthrough or a dedicated LoadBalancer Service).
4. **Given** the secrets, **When** the chart runs, **Then** every secret reaches the pod as a mounted
   file (per #1521) and none appears in a ConfigMap, an env var, a log or `helm get values`.
5. **Given** a daemon pod in the same cluster, **When** it starts with an enrollment token Secret,
   **Then** it links to the orchestrator's in-cluster Service without leaving the cluster.

### Edge Cases

- GitHub is OAuth 2.0, not OIDC: it has no ID token. The orchestrator reads the user through the
  GitHub API (`/user`, `/user/emails`) and keys the identity on the numeric user id, never the login
  (logins can be renamed).
- A Google account's email changes: the identity is keyed on the `sub` claim; the email on the invite
  matches only the first sign-in, and only when Google reports `email_verified: true`.
- Google refuses OAuth redirect URIs on non-public domains (for example `.lan`): a deployment on a
  private name can offer GitHub only, or use a public name for the web host.
- A browser cannot set an `Authorization` header on a WebSocket: the access token goes in the
  socket's first frame, never in the URL (ingress logs would record it).
- A node is lost (not drained): the StatefulSet pod stays `Terminating` until someone force-deletes
  it, so SC-011's 30 s holds for restarts and drains only; the docs say what to do on node loss.
- The owner loses access to their only identity: a new bootstrap code is issued by a CLI command run
  inside the pod (`micold-daemon orchestrator bootstrap-code --reset`), which needs cluster access.
- The bootstrap code leaks before use: it expires (default 24 h) and is printed again on restart.
- Two browser tabs and a phone signed in as one user: each is its own client and its own window for
  window exclusivity (`specs/orchestrator/` edge case "a web client counts as a window").
- An ingress with a 60 s idle timeout drops the WebSocket: the client pings every 20 s; the chart sets
  the ingress timeouts it can and documents the rest.
- A mobile network change (Wi-Fi to cellular) drops the socket: reconnect with backoff and resume by
  sequence number; an action in flight reports an unknown outcome and the app re-reads.
- iOS cannot be built on Linux CI (jaro dropped macOS, 2026-11-14): iOS builds need a hosted macOS
  builder (EAS Build); see open questions.
- A daemon with an older protocol: shown as needing an update, as in the desktop app.

## Requirements *(mandatory)*

Requirement numbers continue those of `specs/orchestrator/spec.md`; an `R` suffix marks a
requirement that replaces the one with the same number there.

### Network API (M3)

- **FR-017R**: The orchestrator MUST offer JSON over WebSocket (TLS) for the client protocol, with
  TypeScript types and a JSON Schema generated from the Rust protocol types (`ClientMsg`, `DaemonMsg`,
  `StructuredEvent` and every type they reach) and checked in CI for drift. It MUST listen on
  localhost only unless configured, check the `Origin` header against a configured list, and resume a
  client's streams by sequence number after a reconnect.
- **FR-030**: Binary frames (terminal grid, postcard) MUST NOT be sent on the JSON API unless the
  terminal view is approved (open question Q1).

### Sign-in and permissions (M3)

- **FR-018R**: Users MUST sign in with GitHub (OAuth 2.0) or Google (OIDC), brokered by the
  orchestrator: the client runs authorization code + PKCE against the orchestrator, the orchestrator
  runs it against the provider and holds the provider client secrets. No password is stored. The
  orchestrator MUST issue its own access tokens (default 15 min, signed with a key from a key set
  with `kid`, so a new key can be added while the old one still verifies) and rotating refresh
  tokens (stored hashed, default 30 days, revocable). `/auth/*` MUST be rate limited.
- **FR-031**: The first owner MUST be bound by a one-time, expiring bootstrap code; no default
  credential exists.
- **FR-032**: Only invited identities (GitHub login or Google email, matched once, then bound to the
  provider's stable id) MAY sign in; the owner MUST be able to invite, change role, and remove users,
  which revokes their refresh tokens and closes their sockets. A socket MUST be closed when its
  access token expires (the client refreshes and reconnects) and MUST see a role change on its next
  message.
- **FR-019R**: Every API request MUST be authorized by the user's global role: owner, operator,
  viewer; viewer by default. Per-daemon grants are deferred.
- **FR-020**: unchanged (audit every changing action, never the typed text), plus every sign-in,
  refusal, invite and role change.
- **FR-033**: A local orchestrator reached over its local socket MUST keep treating the local OS user
  as owner; sign-in applies to the network API only.

### Client (M4)

- **FR-021R**: The client MUST be one React Native (Expo) TypeScript codebase that builds a web
  bundle, an Android app and an iOS app; the orchestrator does not need to serve the bundle.
- **FR-034**: The client theme MUST be generated from `crates/micold-core/src/tokens/` (colour roles
  for light and dark, type scale, shape, elevation, motion, state layers); CI MUST fail when the
  generated file drifts.
- **FR-035**: The client MUST follow the desktop app's layout on wide screens (sidebar, overview grid,
  session area) and adapt it to one column below 600 px wide.
- **FR-022 – FR-026**: unchanged, for every platform build.
- **FR-036**: The client MUST offer the desktop surfaces the parity matrix marks for v1 (quick
  switcher, attention list, appearance settings, users and invites, daemons list and revoke).
- **FR-040**: The client SHOULD then offer the v1.1 surfaces of the parity matrix (changes and diff
  review, PR and Jira status, history and runs, plan usage); they ship in M6, not v1.
- **FR-041**: The client MUST ask for the orchestrator's URL on first start (no built-in server), and
  keep a list of servers signed in to.
- **FR-037**: The mobile apps MUST keep the refresh token in the platform keystore (Keychain,
  Android Keystore via `expo-secure-store`); the web build MUST keep it in an `HttpOnly`, `Secure`,
  `SameSite=Strict` cookie, never in `localStorage`.

### Kubernetes deployment (M5)

- **FR-027R**: The orchestrator MUST ship as a non-root, read-only-root-filesystem, multi-arch
  (amd64, arm64) image, and the web bundle as a second, static image. A Helm chart (`deploy/helm/micold`)
  MUST install both with: a StatefulSet (one replica) and a PVC for the orchestrator, a Deployment for
  the web image, Services, an Ingress (or Gateway API routes) for web and API, a TLS-passthrough
  route or LoadBalancer Service for daemon links, secrets mounted as files, NetworkPolicies, and a
  ServiceMonitor when Prometheus Operator is present.
- **FR-029R**: The orchestrator MUST expose `/healthz` (liveness), `/readyz` (readiness) and
  `/metrics` (Prometheus) on a separate port that the ingress does not route, plus per-client rate
  limits and per-daemon bandwidth caps.
- **FR-038**: On SIGTERM the orchestrator MUST stop accepting, tell every daemon link and client to
  reconnect, flush the store and exit within the termination grace period.
- **FR-039**: A daemon in the same cluster MUST be able to link through the orchestrator's in-cluster
  Service with an enrollment token from a Secret; the chart documents a daemon StatefulSet example.
- **FR-028**: unchanged (desktop app connects to a remote orchestrator after sign-in), using the
  system browser and a loopback redirect.

### Key Entities

- **Identity**: provider (`github` | `google`), stable provider id, display login or email.
- **User**: id, tenant id, identities, role, created, last seen.
- **Invite**: provider, login or email, role, created by, expires, used.
- **Bootstrap code**: hash, expires, used.
- **Refresh token**: hash, user, client kind (web, android, ios, desktop), created, last used,
  revoked.
- **Generated protocol package**: TypeScript types and JSON Schema for the client protocol.
- **Generated theme**: the tokens as a TypeScript module the client theme reads.

## Success Criteria *(mandatory)*

- **SC-008**: One codebase produces the web bundle and the Android app in CI on Linux; the same
  end-to-end test (sign in, open a structured session, send, answer a prompt) passes on both.
- **SC-009**: A token change in `crates/micold-core/src/tokens/` reaches the client with no manual
  step; a stale generated theme fails CI.
- **SC-010**: `helm install` on a kind cluster, with only a hostname, an issuer and two OAuth secrets
  set, yields a working sign-in and a linked daemon (CI job).
- **SC-011**: A pod restart loses no user, invite, enrollment or audit entry; clients are back within
  30 s with no manual step.
- **SC-012**: No password and no provider client secret exists in any client build or browser
  storage.
- SC-006 and SC-007 of `specs/orchestrator/` stand (SC-007 only while the terminal view is out).

## Assumptions

- This is a planning revision of an existing epic, so its requirements and success criteria name
  technologies (Expo, Helm, kind) on purpose; the owner asked for them by name. Each implementation
  issue writes its own technology-free spec and requirements checklist.

- The desktop app stays the iced client (Principle V) and the primary tool; the React Native client
  is an additional client. The constitution needs an amendment for it (see plan).
- The React Native client lives in the repository (`apps/client/`), with its own CI jobs; the Rust
  gate (`mise run gate`) gains only the drift checks of the generated files.
- Single tenant, owner-run deployments; a hosted service is out of scope.
- The orchestrator is the only thing the client talks to; the client never reaches a daemon.

## Out of scope (future work)

- Several active orchestrator replicas (HA) and an external database.
- Per-daemon or per-project grants, groups, SSO beyond GitHub and Google.
- Remote push notifications (APNs, FCM, Web Push).
- The orchestrator creating daemons (pods) by itself.
- End-to-end encryption between client and daemon.
