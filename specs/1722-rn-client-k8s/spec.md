# Feature Specification: A React Native web client that replaces the desktop app, and the orchestrator on Kubernetes

**Feature Branch**: `claude/project-thread-zmzag6`

**Created**: 2026-11-14

**Status**: Draft, revision 2 (planning only). Owner answers of 2026-11-14 applied; all owner
questions answered (see [plan.md](plan.md#owner-decisions-2026-11-14)).

**Input**: Owner request (jaro, 2026-11-14): "I would like you to do planning of migration into web
interface that was mentioned for orchestrator. It should use react native to support web and mobile
app, use material 3 theme. Should support login via GitHub or Google and have basic permission
support. It should be considered to run on k8s cluster." Follow-ups (jaro, 2026-11-14): "Initially
assume similar features and look as current rust ui"; answers to the first round of questions (see
Clarifications); "make orchestrator use some relational db as storage, assume postgresql".

> **Planning only.** This revises milestones M3–M5 of epic #1722 (`specs/orchestrator/`) and adds the
> retirement of the iced desktop app. It changes no source file. Each issue proposed in
> [issues.md](issues.md) is planned and implemented later in its own worktree with
> `/speckit-autopilot #<issue>`, after jaro approves the list. Nothing here creates or edits a GitHub
> issue.

## What this revises

| Topic | `specs/orchestrator/` (2026-11-05) | This revision (2026-11-14) |
|---|---|---|
| UI | iced desktop app stays; a TypeScript web client is added | One React Native (Expo, `react-native-web`) client, web build only for now, that **replaces** the iced app once it reaches parity |
| Look | not specified | Material 3 from today's design tokens (seed `#6750A4`, Roboto), light and dark, today's layout |
| Features | web: structured view only | Today's desktop features (parity matrix in [plan.md](plan.md#feature-parity-matrix)), structured view only, Claude Code first |
| Access | OIDC plus a local owner account with a password | Always authenticated: GitHub or Google on a network deployment, a one-time local access link on loopback; no passwords |
| Claude credentials | not covered | API key / cloud provider credential, or the user's own sign-in in unmodified Claude Code, per daemon; never through Micold ([plan.md](plan.md#claude-credentials)) |
| Permissions | owner / operator / viewer with per-daemon grants | owner / operator / viewer, one global role per user |
| Storage | JSON and files on one volume | PostgreSQL (SQLite for loopback-only local mode) |
| Deployment | compose and Helm examples | Kubernetes first: images, Helm chart, Gateway API, LoadBalancer for daemon mTLS links, secrets as files, probes, metrics; one replica now, built to scale out |
| Daemon provisioning | the desktop app; the orchestrator never provisions | moves into the orchestrator before the iced app is retired |

M1 (#1723–#1727) and M2 (#1728–#1731) are unchanged and remain prerequisites.

## Clarifications

### Session 2026-11-14

- Q: React Native setup? → A: Expo (managed, Expo Router) with `react-native-web`. _(agent-decided)_
- Q: Material 3? → A: `react-native-paper` v5, theme generated from `crates/micold-core/src/tokens/`.
  _(agent-decided)_
- Q: Which features first? → A: Same features and look as the current Rust UI (jaro).
- Q: Amend the constitution for a React Native client? → A: Yes, and the web interface **replaces**
  the Rust UI (jaro: "ad 1 yes, change that the web interface will replace rust").
- Q: Terminal view? → A: **Structured view only** (jaro).
- Q: Sign-in? → A: The web interface may be published on a network, so it always requires
  authentication; support Claude Code first (jaro). jaro also asked to "login into claude to obtain a
  token which should be passed to orchestrator, daemon and sessions"; Anthropic's terms forbid a
  third-party app offering Claude.ai login or storing or relaying Claude tokens, so access uses
  GitHub or Google (network) or a local access link (loopback), and Claude credentials stay on each
  daemon (plan, "Claude credentials"). jaro chose both routes: an API key or cloud credential file,
  or the user's own sign-in in unmodified Claude Code on the daemon.
- Q: Mobile? → A: Future; **web only** for now (jaro).
- Q: One orchestrator replica? → A: Yes initially, keeping it scalable (jaro).
- Q: Storage? → A: A relational database, **PostgreSQL** (jaro).
- Q: Who may sign in on a network deployment? → A: Only invited GitHub or Google identities; the
  first owner binds with a one-time bootstrap code. _(agent-decided)_
- Q: Basic permissions? → A: Three global roles: owner, operator, viewer. _(confirmed by jaro, 2026-11-15)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Nobody reaches my sessions without signing in (Priority: P1)

The owner runs the orchestrator on their cluster at a public host name. Opening the URL shows a
sign-in page with GitHub and Google. The owner signs in with GitHub and enters the bootstrap code;
an invited colleague signs in with Google and gets the viewer role; a stranger is refused. On a
laptop, the loopback orchestrator opens only through the one-time link it prints.

**Independent Test**: Against an orchestrator with a fake OAuth/OIDC provider, every API call
without a valid token is refused; GitHub and Google sign-in work; an uninvited identity is refused;
a loopback orchestrator refuses a browser without the access link.

**Acceptance Scenarios**:

1. **Given** a fresh deployment, **When** the first person signs in and enters the bootstrap code,
   **Then** they become owner and the code is spent; without it nobody becomes owner.
2. **Given** an invited Google email, **When** that person signs in with Google, **Then** they get the
   role on the invite (viewer by default).
3. **Given** an identity that was not invited, **When** it signs in, **Then** it is refused with "not
   invited" and no user record is created.
4. **Given** a signed-in browser, **When** the access token expires, **Then** the client refreshes it
   silently; a removed user's refresh is refused and the client returns to sign-in.
5. **Given** a loopback-only orchestrator, **When** a browser opens it without the access link,
   **Then** it is refused; with the link, it gets an owner session.
6. **Given** an orchestrator bound to a non-loopback address with no provider configured, **When** it
   starts, **Then** it refuses to start and `config check` names the problem.

---

### User Story 2 - The same app, the same look, in the browser (Priority: P1)

The user opens the web client and sees today's desktop layout: sidebar of projects, worktrees and
sessions, the overview grid of cards and the session view, in today's Material 3 colours, light or
dark. Below 600 px the screens stack.

**Independent Test**: Screenshots of the overview and a structured session at 1280, 800 and 390 px
in light and dark match the iced app's colour roles and type scale (tokens compared by a test,
layout by a visual pass).

**Acceptance Scenarios**:

1. **Given** the tokens change, **When** the client is built, **Then** its theme changes with them;
   CI fails if the generated theme file is stale.
2. **Given** the OS is in dark mode, **When** the client opens with "follow system", **Then** it is
   dark.
3. **Given** a narrow window, **When** the user opens a session, **Then** the list gives way to the
   session with a back action, and nothing scrolls sideways.

---

### User Story 3 - Read and drive a structured Claude Code session (Priority: P1)

As `specs/orchestrator/` User Story 4: list projects, worktrees and sessions in a stable order with
state, open a structured session, read the live transcript, answer a permission prompt, paste and
send, switch model, effort and permission mode, start and stop sessions and worktrees. A viewer sees
no composer and no changing action.

**Independent Test**: SC-006 of `specs/orchestrator/` from the web client.

**Acceptance Scenarios**: those of `specs/orchestrator/` User Story 4, plus:

1. **Given** the network drops for a minute, **When** it returns, **Then** the client reconnects and
   resumes the transcript by sequence number with nothing lost or duplicated.
2. **Given** a terminal session, **When** it is listed, **Then** it shows its state and can be
   stopped, and no terminal is rendered.
3. **Given** a daemon whose Claude Code is not signed in and has no API key, **When** the user tries
   to start a session there, **Then** the client says so and points to the docs; it never asks for a
   Claude password or token.

---

### User Story 4 - Everything the desktop app does, then no desktop app (Priority: P2)

The user reviews a worktree's changes, sees PR and Jira status, reads history, adds an SSH or pod
daemon and edits its settings, all from the web client. Once that works, the iced app is removed.

**Independent Test**: Every v1.1 and v2 row of the parity matrix has an end-to-end test on the web
client; the retirement issue removes the iced crates and the gate still passes.

**Acceptance Scenarios**:

1. **Given** a worktree with changes, **When** the user opens Changes, **Then** the tree and diffs
   match the iced app's.
2. **Given** the owner adds an SSH host or a Kubernetes pod daemon from the web client, **When** it is
   created, **Then** the orchestrator provisions it and it links and appears online.
3. **Given** a secret setting (a GitHub or Jira token), **When** the owner saves it, **Then** it is
   stored on the daemon that uses it and is never shown back in full.

---

### User Story 5 - Run it on my Kubernetes cluster (Priority: P2)

The owner runs `helm install` with a host name, a gateway, a database Secret and GitHub and Google
OAuth client secrets, and gets a web URL, a daemon-link endpoint and a bootstrap code. Daemons on
their laptop, on SSH hosts and in the cluster enroll and link over mTLS.

**Independent Test**: In a kind cluster in CI (Envoy Gateway, CloudNativePG), install the chart with
a pre-created bootstrap-code Secret, sign in through a fake provider, enroll a daemon pod, drive a
structured session through the gateway.

**Acceptance Scenarios**:

1. **Given** the chart installed, **When** the pod is ready, **Then** `/readyz` is 200 only after the
   database is reachable and migrated, the CA is loaded and the listeners are up.
2. **Given** an orchestrator pod restart, **When** the new pod starts, **Then** registry, users,
   invites and audit log are intact (they are in PostgreSQL), and daemons and clients reconnect on
   their own.
3. **Given** the gateway terminates TLS for the web and API host, **When** a daemon links, **Then** its
   mTLS reaches the orchestrator untouched (LoadBalancer Service, or TLS passthrough).
4. **Given** the secrets, **When** the chart runs, **Then** every secret reaches the pod as a mounted
   file and none appears in a ConfigMap, an env var, a log or `helm get values`.
5. **Given** a daemon pod in the same cluster, **When** it starts with an enrollment token Secret,
   **Then** it links through the in-cluster Service.

### Edge Cases

- GitHub is OAuth 2.0, not OIDC: the identity is read from the GitHub API and keyed on the numeric
  user id (logins can be renamed).
- Google: keyed on `sub`; an invite's email matches only once and only with `email_verified: true`.
  Google refuses redirect URIs on private domains (for example `.lan`): such a deployment offers
  GitHub only or uses a public host name.
- A browser cannot set an `Authorization` header on a WebSocket: the access token goes in the first
  frame, never in the URL.
- The owner loses their only identity: `micold-daemon orchestrator bootstrap-code --reset`, run with
  shell access to the orchestrator, issues a new code.
- The bootstrap code leaks before use: it expires (default 24 h) and can be reset.
- The database is unreachable: readiness fails, the API refuses with a clear error, daemons keep
  their sessions running, links retry.
- A database schema newer than the binary: the orchestrator refuses to start and names both versions.
- Two tabs signed in as one user: each is its own client and its own window for window exclusivity.
- A gateway with a 60 s idle timeout: the client pings every 20 s; the chart documents timeouts.
- A daemon of an older protocol: shown as needing an update.
- Codex, Copilot, Pi and OpenCode sessions have no structured mode today: listed with state and
  stoppable only, until a structured adapter ships later (jaro, 2026-11-15: focus on Claude Code; the
  others do not block retiring the iced app).

## Requirements *(mandatory)*

Requirement numbers continue those of `specs/orchestrator/spec.md`; an `R` suffix marks a
requirement that replaces the one with the same number there.

### Network API and storage (M3)

- **FR-017R**: The orchestrator MUST offer JSON over WebSocket (TLS on a network) for the client
  protocol, with TypeScript types and a JSON Schema generated from the Rust protocol types and
  checked in CI for drift. It MUST check `Origin` against a configured list and resume a client's
  streams by sequence number after a reconnect.
- **FR-030**: Binary terminal frames MUST NOT be sent on the JSON API.
- **FR-042**: The orchestrator MUST store its state (registry, enrollments, certificate serials and
  revocations, users, identities, invites, bootstrap code hash, refresh-token hashes, audit log) in
  PostgreSQL, through store traits, with forward-only migrations applied on start; it MUST refuse a
  newer schema. Private keys MUST stay out of the database. A loopback-only local mode MAY use
  SQLite through the same traits (jaro, 2026-11-15).

### Access and permissions (M3)

- **FR-018R**: On a network deployment users MUST sign in with GitHub (OAuth 2.0) or Google (OIDC),
  brokered by the orchestrator (authorization code + PKCE from the client; provider secrets only on
  the orchestrator). No password is stored. The orchestrator MUST issue its own access tokens
  (default 15 min, key set with `kid`) and rotating refresh tokens (hashed, default 30 days,
  revocable). `/auth/*` MUST be rate limited.
- **FR-031**: The first owner MUST be bound by a one-time, expiring bootstrap code.
- **FR-032**: Only invited identities MAY sign in; the owner MUST be able to invite, change role and
  remove users, which revokes their refresh tokens and closes their sockets. A socket MUST close when
  its access token expires and MUST see a role change on its next message.
- **FR-033R**: Every API request MUST be authenticated. A loopback-only orchestrator MUST grant
  access only through a one-time local access link printed at start; an orchestrator bound to any
  other address MUST refuse to start without a sign-in provider.
- **FR-019R**: Every API request MUST be authorized by the user's global role (owner, operator,
  viewer; viewer by default).
- **FR-020**: unchanged (audit every changing action, never the typed text), plus every sign-in,
  refusal, invite and role change; the audit table is append-only.
- **FR-043**: Micold MUST NOT offer Claude.ai sign-in and MUST NOT collect, store or relay Claude
  subscription credentials. Each daemon MUST run Claude Code either with an API key or cloud
  provider credential named by its configuration (secret by file path) or with the user's own sign-in
  done in the unmodified Claude Code on that daemon; the client MUST show each daemon's Claude Code
  sign-in state.

### Client (M4, M6, M7)

- **FR-021R**: The client MUST be one React Native (Expo) TypeScript codebase; only the web build is
  produced now. The orchestrator MUST serve the bundle in local mode; on Kubernetes a separate
  static image serves it.
- **FR-034**: The client theme MUST be generated from `crates/micold-core/src/tokens/`; CI MUST fail
  when the generated file drifts.
- **FR-035**: The client MUST follow today's desktop layout on wide screens and stack below 600 px.
- **FR-022 – FR-026**: unchanged, for the web client; structured sessions of Claude Code first.
- **FR-036**: v1 MUST offer the parity matrix's v1 rows (including quick switcher, attention,
  appearance, users and invites, daemons list and revoke).
- **FR-040**: M6 MUST offer the v1.1 rows (changes and diff review, PR and Jira status, history and
  runs, plan usage, setup output, sandbox status).
- **FR-044**: M7 MUST offer the v2 rows (daemon provisioning through the orchestrator, secret
  settings, Kubernetes view, large editor); only then MAY the iced app be removed.
- **FR-045**: The client MUST NOT include a terminal emulator; terminal sessions MUST be listed with
  state and be stoppable.
- **FR-037R**: The refresh token MUST live in an `HttpOnly`, `Secure`, `SameSite=Strict` cookie scoped
  to `/auth`; the access token only in memory; never `localStorage`.

### Kubernetes deployment (M5)

- **FR-027R**: The orchestrator MUST ship as a non-root, read-only-root, multi-arch image and the web
  bundle as a second static image. A Helm chart MUST install both with: a Deployment (one replica
  now) for the orchestrator with no volume, a database connection from an existing Secret (and an
  optional CloudNativePG cluster for demos and CI), a Deployment for the web image, Services,
  Gateway API routes (and a classic Ingress template) for web and API, a LoadBalancer Service (or
  TLS passthrough) for daemon links, secrets as files, NetworkPolicies, and an optional
  ServiceMonitor.
- **FR-029R**: The orchestrator MUST expose `/healthz`, `/readyz` and `/metrics` on a port no gateway
  routes, and enforce per-client rate limits and per-daemon bandwidth caps.
- **FR-038**: On SIGTERM the orchestrator MUST stop accepting, tell links and clients to reconnect,
  finish in-flight writes and exit within the grace period.
- **FR-039**: A daemon in the same cluster MUST be able to link through the in-cluster Service with
  an enrollment token from a Secret.
- **FR-046**: The design MUST keep scaling out possible without a data migration: no session state in
  the orchestrator, all orchestrator state in PostgreSQL, routing behind a `LinkDirectory` trait.

### Key Entities

- **Identity**: provider (`github` | `google`), stable provider id, login or email.
- **User**: id, tenant id, identities, role, created, last seen.
- **Invite**, **Bootstrap code**, **Refresh token**, **Local access code**: as in plan.md.
- **Daemon Claude state**: signed in (subscription), API key or provider configured, or neither.
- **Generated protocol package** and **generated theme**.

## Success Criteria *(mandatory)*

- **SC-008**: The web client drives a structured Claude Code session end to end (sign in, open,
  send, answer a prompt, start a session) in CI with Playwright.
- **SC-009**: A token change in `crates/micold-core/src/tokens/` reaches the client with no manual
  step; a stale generated theme fails CI.
- **SC-010**: `helm install` on a kind cluster with a host name, a database and two OAuth secrets
  yields a working sign-in and a linked daemon (CI job).
- **SC-011**: An orchestrator pod restart loses no user, invite, enrollment or audit entry; clients
  are back within 30 s with no manual step.
- **SC-012**: No password, no provider client secret and no Claude credential exists in the client
  bundle, browser storage or the orchestrator database.
- **SC-013**: No API call succeeds without authentication, on loopback or on a network.
- **SC-014**: After retirement, the repository builds and passes the gate with no iced dependency.
- SC-006 and SC-007 of `specs/orchestrator/` stand.

## Assumptions

- This is a planning revision, so requirements and success criteria name technologies (Expo, Helm,
  PostgreSQL, kind) on purpose; the owner asked for them by name. Each implementation issue writes
  its own technology-free spec and requirements checklist.
- The client lives in this repository (`apps/client/`, `packages/protocol/`, pnpm workspace).
- Single tenant, owner-run deployments; a hosted service is out of scope.
- The client talks only to the orchestrator, never to a daemon.

## Out of scope (future work)

- Native mobile apps (the React Native code base keeps them possible).
- A terminal view.
- Several active orchestrator replicas (designed for, not built).
- Multi-user ownership (each user their own daemons, projects and sessions), per-daemon or
  per-project grants (designed for: `tenant_id` and `owner_user_id` on every row); SSO beyond GitHub
  and Google.
- Remote push notifications.
- End-to-end encryption between client and daemon.
