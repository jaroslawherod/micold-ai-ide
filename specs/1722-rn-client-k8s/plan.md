# Plan: a React Native web client that replaces the desktop app, and the orchestrator on Kubernetes

**Date**: 2026-11-14 (revision 2, after jaro's answers) | **Spec**: [spec.md](spec.md) |
**Issues**: [issues.md](issues.md) | **Epic**: #1722 (revises its M3–M5; base plan `specs/orchestrator/`)

## Summary

The Micold user interface moves to the web. One TypeScript client, written with React Native (Expo
and `react-native-web`) so that a mobile app can be added later from the same code, runs in the
browser and, once it reaches parity, **replaces the iced desktop app** (jaro, 2026-11-14). Rust stays
for everything that is not UI: `micold-core`, the daemons and the orchestrator.

The client offers the **structured view only** and supports **Claude Code first**. It talks only to
the orchestrator, over the JSON-over-WebSocket API of #1732 with TypeScript types generated from the
Rust protocol. Its Material 3 theme is generated from the same tokens as today's desktop app, and its
screens follow today's desktop layout.

Because the web interface may be published on a network, **every access needs authentication**:
GitHub or Google sign-in through the orchestrator on a network deployment, and a one-time local
access link on a loopback-only one. Three global roles control what a signed-in user can do. The
Claude credentials that sessions use never pass through the web client or the orchestrator (see
[Claude credentials](#claude-credentials)).

The orchestrator ships as an image plus a Helm chart built for Kubernetes: one replica in v1 with its
state in PostgreSQL, designed so that it can scale out later ([Scalability path](#scalability-path)), a
stateless web image, Gateway API routing, mTLS daemon links on their own Service, secrets as files,
and health, readiness and metrics endpoints.

## Architecture

```text
 Browser (web client;                          Kubernetes cluster (owner's)
 mobile later)                     ┌──────────────────────────────────────────────────────────┐
 ┌───────────────────────┐  HTTPS  │ Gateway  micold.example.com                              │
 │ apps/client (Expo RN, │────────▶│   /            → Service web  → Deployment web (static)  │
 │  web build)           │  WSS    │   /api, /auth  → Service orch → Deployment orchestrator  │
 │  - M3 theme (tokens)  │────────▶│                          (1 replica now) ──▶ PostgreSQL  │
 │  - @micold/protocol   │         │ Service orch-link :8443 (LoadBalancer, mTLS untouched)   │
 └───────────────────────┘         │ Service orch-ops  :9090 /healthz /readyz /metrics        │
                                   │                                                          │
 GitHub OAuth / Google OIDC ◀──────│ orchestrator: sign-in broker, roles, audit, router       │
                                   │                                                          │
 Daemons anywhere ─ mTLS link ────▶│ orch-link        In-cluster daemon pods ─ Service DNS ──▶│
 (laptop, SSH host, container)     └──────────────────────────────────────────────────────────┘
   each daemon runs Claude Code with the owner's own Claude credentials (never via the client)

 Local use (no cluster): `micold-daemon orchestrator --serve-web` on 127.0.0.1, the same web bundle,
 opened through a one-time local access link; daemons as today.
```

- **Sessions stay in the daemons** (unchanged rule of `specs/orchestrator/`). The orchestrator is a
  router plus the owner of the registry, CA, users, invites, refresh tokens and audit log.
- **The client never reaches a daemon**; every request goes through the orchestrator, which
  authorizes it by role and audits it.
- **No unauthenticated access**, also on a laptop: a loopback-only orchestrator still requires the
  local access link (FR-033R), because any local process or user can reach a loopback port.

### Client (`apps/client/`)

| Concern | Choice |
|---|---|
| Framework | Expo SDK (managed workflow) with Expo Router; `react-native-web`; **only the web target is built now**. Native targets stay possible because the code is React Native; platform files (`*.web.tsx`) only where the web needs them |
| Language | TypeScript, `strict` |
| Material 3 | `react-native-paper` v5 (MD3 components and theme) for the base components; Micold-specific components (transcript cards, session strip, overview card, diff view) in `apps/client/src/components/`, one shared library (Principle VIII applied to the client) |
| Theme | `apps/client/src/theme/tokens.generated.ts`, produced by a new `micold-tokens-ts` binary beside `micold-tokens-css` (`crates/micold-core/src/bin/`), read into a Paper `MD3Theme` for light and dark; Roboto and Fira Mono from `assets/fonts/` |
| Protocol | `packages/protocol/` (`@micold/protocol`): TS types + JSON Schema generated from the Rust types (see #1732 below), and a small hand-written client: socket, reconnect with backoff, resume by sequence, request/response correlation |
| State | Zustand stores per feature (sessions, transcript, composer, attention), fed by the socket; no server cache library, since the socket pushes state |
| Lists | `@shopify/flash-list` for the transcript and long lists (virtualized; the desktop's large-transcript CPU issue #1683 is the warning) |
| Markdown and code | `react-native-markdown-display` plus a syntax highlighter |
| Auth | Authorization code + PKCE against the orchestrator; refresh token in an `HttpOnly` cookie, access token in memory only |
| Notifications | In-app snackbar and attention list; browser `Notification` while a tab is open |
| Tests | Jest + React Native Testing Library (units), Playwright against the web build (end to end, keyboard navigation included) |
| Builds | `expo export --platform web` into a static bundle: served by the orchestrator in local mode and by the web image on Kubernetes |

The workspace uses pnpm; Node and pnpm are pinned in `mise.toml`, so `mise run client-test` and
`mise run client-web` join the existing tasks.

### Feature parity matrix

jaro: "Initially assume similar features and look as current rust ui", then "the web interface will
replace rust" and "structured view only". So every desktop surface either moves to the web client or
is dropped on purpose; nothing stays desktop-only once the iced app is retired. Inventory from
`crates/micold-client/src/`.

| Desktop surface (module) | Web client | Notes |
|---|---|---|
| Sidebar: projects, worktrees, sessions, groups (`ui/sidebar.rs`, `micold-core/src/sidebar_groups.rs`) | v1 | Grouping and stable order come from `micold-core` rules; the orchestrator sends them computed or the rule is ported with shared test vectors (decided in #1732's plan) |
| Overview grid of cards (`ui/overview.rs`, `ui/material/overview_card.rs`) | v1 | One column below 600 px |
| Structured transcript, insights, subagent threads, usage (`ui/transcript/`) | v1 | Claude Code first |
| Composer, paste, attachments (`ui/composer.rs`) | v1 | Text and image paste |
| Permission prompts and ask-user answers (`ui/transcript/permission.rs`) | v1 | |
| Mode, model and effort pickers (`ui/mode_control.rs`) | v1 | |
| Confirmations (`ui/confirm_*.rs`) | v1 | Those of the actions the client offers |
| Worktree create, rename, delete (`ui/worktree_form.rs`) | v1 | GitHub and Jira issue filters in v1.1 |
| Session start, stop; session strip (`features/session_strip.rs`) | v1 | Structured Claude Code sessions; see "Terminal sessions" below |
| Attention and notifications (`features/attention.rs`) | v1 | While a tab is open |
| Quick switcher, command palette (`ui/quick_switcher.rs`, `features/command_palette.rs`) | v1 | Ctrl/Cmd+K |
| Keyboard navigation (#803), shortcut sheet | v1 | Same shortcuts where the browser allows |
| Appearance settings (light, dark, system) | v1 | Per browser |
| Users, invites, roles, audit (new) | v1 | Owner only |
| Daemons list, enrollment token, revoke (`features/daemons.rs`) | v1 | |
| Changes and diff review (`ui/changes.rs`, `ui/material/diff_view.rs`, `review_*`) | v1.1 | Read-only first, then review replies |
| PR status, Jira status chips (`features/pr_status.rs`, `features/jira_status.rs`) | v1.1 | Read from the daemon; tokens stay on the daemon |
| History and parallel runs (`features/history.rs`, `features/runs.rs`, `ui/parallel_dialog.rs`) | v1.1 | |
| Plan usage (`features/plan_usage.rs`) | v1.1 | |
| Setup output, project setup (`features/setup.rs`) | v1.1 | Streamed from the daemon |
| Sandbox status (`features/sandbox.rs`) | v1.1 | |
| Settings: environment, GitHub and Jira tokens, project | v2 (before retirement) | Edited through the orchestrator, stored on the daemon that uses them, never shown back in full; owner only |
| Daemon provisioning: add container, SSH and pod daemons, start them (`features/daemons_kube.rs`, `features/daemons_start.rs`, `micold_core::provision`) | v2 (before retirement) | Moves from the desktop app to the orchestrator: see [Retiring the iced client](#retiring-the-iced-client) |
| Kubernetes live view (`kube/`) | v2 | Through the orchestrator's provisioner |
| Large editor (`ui/large_editor.rs`) | v2 | |
| Terminal view (`ui/terminal.rs`), terminal settings | **dropped** | Structured view only (jaro). See below |

**Terminal sessions.** Without a terminal view, a terminal session has no UI once the iced app is
gone. The client lists existing terminal sessions with their state and lets the user stop them, and
starts only structured sessions. Today the structured mode exists for Claude Code (features 938–944),
so "Claude Code first" and "structured only" agree; other providers (Codex, Copilot, Pi, OpenCode)
need a structured adapter each before the web client can drive them; they ship later (decision 8).

### Network API (#1732, revised)

- JSON over WebSocket at `/api/ws`, one socket per client, multiplexing requests, responses and
  pushed events; HTTP only for `/auth/*` and the ops port.
- **Schema generation**: no generator exists in the workspace today (`schemars`, `ts-rs` and
  `specta` are absent). Add `schemars` derives behind a `schema` cargo feature of `micold-core` on
  the protocol types, a `micold-protocol-schema` binary that writes `packages/protocol/schema.json`,
  and `json-schema-to-typescript` to emit `index.generated.ts`. A `mise run protocol-schema` task
  regenerates both; the gate fails on drift (as the token CSS does).
- The JSON form is the existing serde form of `ClientMsg` and `DaemonMsg` (already JSON for control
  messages, `protocol/codec.rs`), wrapped in the routed envelope of #1723 with the client's
  sequence numbers, so nothing is translated by hand.
- Authorization happens before routing: each `ClientMsg` variant maps to `Read` or `Change` (a
  table in `micold-core` with a test that every variant is classified, so a new variant cannot ship
  unclassified).
- Origin allow-list, per-socket rate limit (#1742), 20 s ping, resume by sequence after reconnect.

### Sign-in (#1733, replaced)

```text
client ──(1) /auth/start?provider=github&code_challenge=…&redirect_uri=…──▶ orchestrator
orchestrator ──(2) redirect to GitHub/Google authorize (its own client id, state, PKCE)──▶ provider
provider ──(3) callback /auth/callback/github?code=…──▶ orchestrator: exchange code, read identity,
                 check invite or bootstrap, create or load user, audit
orchestrator ──(4) redirect to client redirect_uri with a one-time orchestrator code──▶ client
client ──(5) POST /auth/token (code + code_verifier) ──▶ access token (15 min) + refresh token
```

- Redirect URIs: the web origin (`https://micold.example.com/auth/done`), in an allow-list. A
  native app later adds a reverse-domain private-use scheme (RFC 8252 §7.1).
- Local mode: no provider; the one-time local access link ([Local mode](#local-mode-no-cluster))
  yields the same cookie and tokens for the owner.
- Socket authentication: browsers cannot set headers on a WebSocket, so every client sends the
  access token in the socket's first frame (never in the URL). The orchestrator keeps the user table
  in memory and checks the role on every message, so a role change applies at once; the socket
  closes when its token expires (the client refreshes and reconnects, resuming by sequence) or when
  the user is removed.
- Web refresh cookie: `HttpOnly`, `Secure`, `SameSite=Strict`, `Path=/auth`; `/auth/refresh` also
  checks `Origin`. `/auth/*` is rate limited.
- Access tokens: JWT (EdDSA) signed with a key from a key set with `kid`; the key set is a mounted
  file, a new key is added beside the old one, and the old one keeps verifying until its tokens
  expire (Helm alone cannot rotate a key; the docs give the steps).
- Google: invites match an email only when `email_verified` is true.
- Storage: users, identities, invites, bootstrap code and refresh-token hashes in PostgreSQL
  behind an `AuthStore` trait ([Storage](#storage-postgresql)).
- Provider setup is the owner's: a GitHub OAuth App and/or a Google OAuth client; either may be
  absent, and the client shows only the configured buttons.

### Roles (#1734, revised)

| Action | owner | operator | viewer |
|---|---|---|---|
| List and read projects, worktrees, sessions, transcripts, changes | ✓ | ✓ | ✓ |
| Send input, answer prompts, change model/effort/mode, start/stop sessions | ✓ | ✓ | — |
| Create, rename, delete worktrees | ✓ | ✓ | — |
| Invite, change role, remove users; read the audit log | ✓ | — | — |
| Create enrollment tokens, revoke daemons | ✓ | — | — |

The client hides what the role cannot do; the orchestrator refuses it regardless.

### Kubernetes deployment (#1740, replaced; #1742 revised)

| Concern | Design |
|---|---|
| Images | `ghcr.io/jaroslawherod/micold-orchestrator` (the `micold-daemon` binary, `orchestrator` subcommand; distroless `cc` base, UID 65532, read-only root FS) and `ghcr.io/jaroslawherod/micold-web` (static bundle behind a small static server, e.g. `nginx-unprivileged`). Multi-arch amd64 and arm64, built and signed (cosign) by the release workflow |
| Workloads | orchestrator: Deployment, `replicas: 1` in v1, no volume (state in PostgreSQL), an `emptyDir` at `/tmp` because the root filesystem is read-only, `strategy: Recreate` while `replicas` is 1 so two orchestrators never hold daemon links at once; web: Deployment, 2 replicas, HPA optional, PodDisruptionBudget |
| State | PostgreSQL (bring your own, or CloudNativePG; the chart takes `database.existingSecret` with the URL): registry, enrollments, users, invites, refresh-token hashes, audit. The CA private key and the signing key set are Secrets. Nothing about sessions. Backup: the database's backups plus the CA key Secret (losing the CA key means every daemon re-enrolls) |
| Why one replica | Each daemon holds one link to one orchestrator and the link directory is in memory in v1; more replicas need the Postgres `LinkDirectory` and replica-to-replica forwarding ([Scalability path](#scalability-path)) |
| Ingress | One host for web and API: `/` → web, `/api`, `/auth` → orchestrator with WebSocket idle timeouts raised. Gateway API `HTTPRoute` is the primary path (ingress-nginx is retired upstream, best-effort maintenance ended March 2026); a classic `Ingress` template stays for other controllers (Traefik, HAProxy, cloud ones), with their timeout annotations documented |
| Daemon links | mTLS must reach the orchestrator untouched. Default: a `LoadBalancer` Service on 8443 (works on every cloud and with MetalLB). Option: TLS passthrough through a Gateway `TLSRoute` (experimental channel in many implementations) or a controller's passthrough mode, whose prerequisites (for example a controller flag) the chart cannot set and the docs list. Terminating TLS at the ingress and forwarding the client certificate in a header is rejected (spoofable once anything else can reach the Service) |
| TLS | cert-manager `Certificate` for the web/API host (issuer is a chart value); the link host serves the orchestrator's own CA-signed certificate (#1729), which daemons pin |
| Secrets | Kubernetes Secrets mounted as files under `/run/secrets/micold/` (GitHub and Google client secrets, JWT signing key, optional CA key import); the orchestrator config names them by path (#1521 rule). Works with External Secrets / Sealed Secrets unchanged. The bootstrap code is either taken from a Secret the owner created before install (`bootstrap.existingSecret`), or generated by the orchestrator on first start against an empty database, printed to its log and stored only as a hash; `micold-daemon orchestrator bootstrap-code --reset` (run with `kubectl exec`) issues a new one. No Job and no Kubernetes API access is needed |
| Config | A ConfigMap with the orchestrator TOML (public URL, allowed origins, redirect URIs, providers, limits, `database.url_file`); a checksum annotation restarts the pod on change |
| Probes | `startupProbe` and `readinessProbe` on `/readyz` (database reachable and migrated, CA loaded, listeners bound), `livenessProbe` on `/healthz` (event loop responsive), all on the ops port 9090, which no ingress routes |
| Metrics | `/metrics` (Prometheus text): links online/offline, clients by kind, messages and bytes per daemon, auth refusals, rate-limit hits; optional ServiceMonitor |
| Shutdown | SIGTERM: readiness fails, links and clients told to reconnect, in-flight writes finished, exit; `terminationGracePeriodSeconds: 30`. With no volume, a lost node only delays the replacement pod by the eviction timeout |
| Network | NetworkPolicies: web pod accepts from ingress only; orchestrator accepts 8080 from ingress, 8443 from anywhere (daemons), 9090 from the monitoring namespace; egress to the database, to DNS and to 443 outside the cluster CIDRs (standard NetworkPolicy has no hostnames; Cilium or Calico FQDN policies for GitHub and Google are optional values) |
| Security | `runAsNonRoot`, `readOnlyRootFilesystem`, `allowPrivilegeEscalation: false`, all capabilities dropped, `seccompProfile: RuntimeDefault`; no service account token mounted |
| In-cluster daemons | A documented example StatefulSet for a standing daemon (spec 686's image) that enrolls with a token Secret and links to `micold-orchestrator-link.<ns>.svc:8443` (FR-039). Daemon pods created by the orchestrator's pod provisioner (N11) link the same way |
| Scaling | Vertical for the orchestrator in v1 (requests 100m/128Mi, limits a chart value; tens of daemons and clients per replica); horizontal later ([Scalability path](#scalability-path)); horizontal for web now |
| Local and compose | A `docker compose` file (orchestrator, web, Postgres) for one server stays as a small example; plain local mode needs neither (SQLite, Q-F) |
| CI | A `kind` job: build both images, `helm install` with Envoy Gateway as the Gateway API implementation, CloudNativePG for the database, a pre-created bootstrap Secret and a fake OAuth provider (a small OIDC mock in the test), enroll a daemon pod, run the Playwright smoke test through the gateway; `helm lint` and `kubeconform` on every chart change |

### Claude credentials

jaro asked that the user log in to Claude to get a token that is passed to the orchestrator, the
daemons and the sessions. Anthropic's terms do not allow that design. The Claude Code
[legal and compliance page](https://code.claude.com/docs/en/legal-and-compliance) (read 2026-11-14)
says: "Anthropic does not permit third-party developers to offer Claude.ai login into their own
applications, or to route requests through Free, Pro, or Max plan credentials on behalf of their
users. Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session
tokens — sign-in to a Claude account must complete through Anthropic's own flow." It also says the
Claude Code binary must not be modified and must keep its own sign-in methods.

So Micold neither offers "Sign in with Claude" nor carries a Claude token from the browser through
the orchestrator. Two routes stay open, and the plan supports both (jaro chose both, 2026-11-14):

| Route | How | Billing |
|---|---|---|
| **API key or cloud provider** (clearly permitted) | The owner puts an Anthropic API key (Claude Console), or Amazon Bedrock / Google Vertex / Microsoft Foundry credentials, into a file named by the daemon's configuration (#1521: secrets by file path); on Kubernetes, a Secret mounted only into daemon pods. The daemon passes it to the Claude Code process it starts. The orchestrator and the web client never read it | Per token, to the key owner |
| **The user's own Claude subscription** | The user signs in to the **unmodified Claude Code** on each daemon through Claude Code's own flow (for example `claude` → `/login` over SSH or `kubectl exec`, once per daemon volume). The token stays in Claude Code's own store on that daemon; Micold never collects, stores or relays it. The web client only shows whether a daemon's Claude Code is signed in, from `claude auth status` | The user's plan |

Not chosen: storing a `claude setup-token` value in the orchestrator and handing it to daemons. That
is Micold storing and relaying a subscription token, which the terms rule out. Whether a token the
user writes into their own daemon's secret file is acceptable is unclear from the page; the docs will
point to Anthropic's sales contact for that case rather than recommend it.

### Local mode (no cluster)

`micold-daemon orchestrator --serve-web` binds 127.0.0.1, serves the web bundle from the same origin
as the API, and prints a one-time local access link (`http://127.0.0.1:<port>/?access=<code>`), as
Jupyter does. Opening it exchanges the code for a session cookie and an owner session. GitHub and
Google sign-in are needed only when the orchestrator is bound to a non-loopback address, and binding
one without them configured is refused by `config check`. A user service unit
(`micold-orchestrator.service`, systemd user) replaces "the desktop app starts the orchestrator".

### Retiring the iced client

The web client replaces the iced app only after the v2 rows of the parity matrix ship. Until then
the iced app stays in maintenance (bug fixes, no new features), so a user is never left without a
way to create or provision a daemon. What moves:

- **Daemon provisioning** moves from the desktop app (`micold_core::provision`, feature 806, and the
  Kubernetes runtime of spec 686) into the orchestrator, behind a `Provisioner` trait: local
  container, SSH host, Kubernetes pod. This reverses the base spec's "out of scope: the orchestrator
  provisioning daemons". On Kubernetes, the pod provisioner needs a ServiceAccount with a namespaced
  Role (pods, PVCs, Secrets in one sessions namespace). It is off by default; standing daemons from
  the chart's example need no Kubernetes API access.
- **Settings that hold secrets** (GitHub and Jira tokens, environment) are written through the
  orchestrator to the daemon that uses them and never read back in full.
- **Notifications** become browser notifications; desktop notifications (`shell/desktop_notify/`)
  go with the iced app.
- **Removal**: one chore deletes `crates/micold-client`, `crates/micold-material`, the iced
  dependency, the AppImage/deb packaging and their docs, after a release in which the web client is
  the default. The design tokens stay in `micold-core` (they feed the web theme and the docs site).

### Storage: PostgreSQL

jaro (2026-11-14): "make orchestrator use some relational db as storage, assume postgresql".

- **One relational store** for everything the orchestrator owns: daemon registry, enrollment tokens,
  certificate serials and revocations, users, identities, invites, bootstrap code, refresh-token
  hashes, audit log, and (from M7) provisioned-daemon records. Every table carries `tenant_id`.
- **Rust side**: `sqlx` (async, compile-time-checked queries, `PgPool`), migrations in
  `crates/micold-daemon/migrations/` applied on start by `sqlx::migrate!`, forward only; the
  orchestrator refuses a database whose schema is newer than it knows. Stores stay behind traits
  (`RegistryStore`, `AuthStore`, `AuditStore`) with an in-memory implementation for unit tests and a
  Postgres one tested against a real Postgres in CI (a service container).
- **#1724's JSON registry file** is replaced: its M5 ("registry survives a restart") lands on the
  `RegistryStore` trait as planned, and the Postgres implementation follows in #1733's milestone; a
  one-shot import reads an existing `registry.json`.
- **Secrets stay out of the database**: the CA private key and the token signing key set are files
  (Kubernetes Secrets); the database holds only certificate serials, hashes and public data.
- **Audit log** is append-only (no `UPDATE`/`DELETE` grant for the orchestrator's role on that table;
  retention by a scheduled partition drop the owner configures).
- **Connection**: `database.url_file` in the orchestrator config (a file holding the URL with its
  password, #1521 rule) and `database.ca_file` for TLS to the server; pool size a config value.
- **On Kubernetes**: bring your own Postgres (managed cloud database, or an operator such as
  CloudNativePG in the cluster); the chart takes an existing Secret with the URL and, for kind CI
  and demos only, can install a CloudNativePG `Cluster`. The orchestrator then needs **no volume**:
  it becomes a Deployment, which is the main step of the [Scalability path](#scalability-path).
- **Local mode**: see Q-F. Recommendation: a local Postgres is not required; the same traits get a
  SQLite implementation (`sqlx` supports both) for the loopback-only orchestrator, so the
  offline, single-machine case keeps working with no server process (Principle IV).
- **Backup**: the database's own backups (managed service, or CloudNativePG's scheduled backups);
  the CA key Secret backed up separately.

### Scalability path

v1 runs one orchestrator replica (jaro: "initially yes but we should keep in mind to make it
scalable"). These choices keep the way to several replicas open:

- **State in PostgreSQL**: every replica shares the one database, so no replica owns state.
- **No session state in the orchestrator**: sessions stay in daemons, so a replica holds only
  connections, which are cheap to rebuild.
- **Stateless tokens**: access tokens are JWTs any replica can verify with the shared key set;
  refresh tokens live in Postgres.
- **A link directory seam**: routing goes through a `LinkDirectory` trait ("which replica holds the
  link of daemon D"). v1's implementation is in-process. A later one keeps the directory in Postgres
  or Redis and forwards a request to the replica holding the link (replica-to-replica over the
  internal Service); clients reconnect anywhere and resume by sequence. Postgres `LISTEN/NOTIFY`
  is enough for that directory at this scale, so no Redis is needed.
- **Rate limits** are per replica in v1; the same trait boundary allows a shared counter later.
- **What scaling changes on Kubernetes**: `replicas` above 1 on the Deployment, a headless Service
  for replica-to-replica forwarding, a PodDisruptionBudget, and the `LinkDirectory` Postgres
  implementation. v1 already runs as a Deployment with no volume, `replicas: 1`.

## Constitution Check

| Principle | Status |
|---|---|
| I. Test-First | Kept: each client issue is TDD with Jest/RNTL and Playwright; Rust parts as today |
| II. Multi-Session | Kept: sessions stay in daemons; the client shows many sessions; input serialized by the daemon |
| III. Worktree Integration | Kept: worktree actions are the daemon's existing ones, routed |
| IV. Local-First (NON-NEGOTIABLE) | **Amended**: local mode still works offline on one machine with no external service (loopback orchestrator on SQLite, local access link, daemons as today). An opt-in network deployment stores its state in PostgreSQL and uses GitHub or Google for sign-in. N1 adds that clause |
| V. Rust + iced Stack | **Replaced** (jaro approved, 2026-11-14): the UI is one TypeScript + React Native client talking only to the orchestrator's API through generated types; Rust stays for core, daemons and orchestrator; iced is retired with the desktop app. MAJOR bump, issue N1 |
| VI. Cross-Platform Parity | Rewritten by N1: daemons and orchestrator on Linux (#1897); the client on current Chromium, Firefox and Safari; native mobile later |
| VII. Documentation | Kept: user guide pages for the client, sign-in, roles, local mode and the Helm chart in the same changes |
| VIII. Reusable UI Components | Applied to the client: one component library in `apps/client/src/components/`; the builder-API clause (iced idiom) becomes "typed props, React idiom" |

"Technology, Storage & Licensing Constraints" is amended with them: GUI (React Native web client),
Persistence (an opt-in network orchestrator stores its state in PostgreSQL; local mode in SQLite) and Distribution
(Linux binaries for daemon and orchestrator, the web bundle, container images).

## Milestones and dependencies

Done: #1723, #1728; #1724 M1–M4 merged (M5 store, M6 pending). Open in M1/M2: #1724, #1725, #1726,
#1727, #1729, #1730, #1731. Numbers `N1`… are proposed new issues ([issues.md](issues.md)).

| Milestone | Delivers | Issues | Depends on | Estimate |
|---|---|---|---|---|
| **M1** (unchanged) | Local orchestrator | #1724, #1725, #1726, #1727 | — | in progress |
| **M2** (unchanged) | mTLS daemons anywhere | #1729, #1730, #1731 | #1724, #1728 | 4–6 weeks |
| **M3 — Network API, storage, auth, roles** | PostgreSQL store, authorized JSON API, GitHub/Google sign-in, local access link, roles, audit | N1 (constitution), N16 (PostgreSQL store), #1732 (revised), #1733 (replaced), #1734 (revised) | #1726 (routing); N16 on #1724's M5 trait; #1733 on N16 | 5–7 weeks |
| **M4a — Client foundation** | Expo web app, M3 theme, sign-in, sidebar and overview, local mode serving, CI | N2 (tokens to TS), #1735 (replaced), N9 (local mode) | N1, #1732, #1733 | 3–4 weeks |
| **M4b — Structured Claude Code sessions** | Transcript, composer, prompts, pickers, lifecycle, attention, quick switcher, admin screens, Claude credentials status | #1736, #1737, #1738, #1739, N3, N10 | M4a, #1734 | 5–7 weeks |
| **M5 — Kubernetes** | Images, Helm chart, gateway and link Service, probes, metrics, kind CI | #1742 (first), #1740 (replaced), N4 (web image) | #1742 on #1732; #1740 on #1730, #1734, #1742, N4; N4 on #1735 | 3–4 weeks |
| **M6 — Parity v1.1** | Changes review, PR/Jira chips, history and runs, plan usage, setup output, sandbox status | N5, N6 | M4b | 4–6 weeks |
| **M7 — Parity v2 and retirement** | Provisioning in the orchestrator, secret settings, Kubernetes view, large editor; then the iced app removed | N11, N12, N13 | M6; N13 on N11, N12 | 5–8 weeks |
| **Later** | Native mobile app; structured adapters for other providers; HA orchestrator | N7, N14, N15 | jaro's go-ahead | — |

```text
#1724 ─▶ #1725 ─▶ #1726 ─┬─▶ #1727
                         └─▶ #1732 ─▶ #1733 ─▶ #1734
#1724 (M5 store trait) ─▶ N16 ─▶ #1733      #1732 ─▶ #1742
#1728 ✓ ─▶ #1729 ─▶ #1730 ─▶ #1731
N1, N2, #1732, #1733 ─▶ #1735 ─▶ N9
#1734, #1735 ─▶ N3 ;  #1735 ─▶ N10
#1735 ─▶ #1736 ─▶ #1737 ;  #1736 ─▶ #1739 ;  #1735 ─▶ #1738
#1736 ─▶ N5 ;  #1735 ─▶ N6 ;  #1735 ─▶ N4
#1730, #1734, #1742, N4 ─▶ #1740
#1726, #1734 ─▶ N11 ;  #1734 ─▶ N12 ;  N5, N6, N11, N12 ─▶ N13
```

M4a and M2 can run in parallel once #1732 and #1733 land; M5 can start on #1742 as soon as #1732
exists. The critical path is #1724 → #1725 → #1726 → #1732 → #1733 → #1735 → #1736 → #1737, then
M6 and M7 to retirement.

## Research and rejected alternatives

| Decision | Chosen | Rejected, and why |
|---|---|---|
| Client stack | Expo + `react-native-web`, web target only for now | Plain React web app (simpler and fewer web gaps, but a mobile app later would mean a second codebase; jaro asked for React Native); Flutter (not React Native); keeping iced and adding a web client (two UIs to keep in step, now unneeded) |
| M3 components | `react-native-paper` v5 | Tamagui or NativeWind (not Material 3); hand-built components only; Material Web components (web only, would block mobile later) |
| Theme source | Generate from `micold-core` tokens | Paper's own `MD3LightTheme` with a seed (drifts from today's look, ignores our shape and type scale) |
| Schema | `schemars` → JSON Schema → TS | `ts-rs`/`specta` (TS only, no schema for validation in tests and tooling); hand-kept types (drift) |
| Wire | JSON over WebSocket | gRPC-web (needs a proxy, no gain for a pushed event stream); postcard in WASM |
| Network sign-in | Orchestrator brokers GitHub and Google, issues its own tokens | Client talks to providers directly; a separate IdP (Dex, Keycloak: one more stateful service); oauth2-proxy at the gateway (no roles, no per-message WebSocket authorization) |
| Claude login as the app's sign-in | Not offered | Not permitted by Anthropic's terms (see [Claude credentials](#claude-credentials)) |
| Local access | One-time local access link on loopback | No auth on loopback (any local process could drive sessions); GitHub/Google even locally (breaks offline use, Principle IV) |
| Permissions | Three global roles | Per-daemon grants now (deferred, tenant id kept) |
| Storage | PostgreSQL (jaro), `sqlx`, behind store traits; SQLite for local mode | JSON files and SQLite on a PVC (no path to several replicas); an ORM such as Diesel or SeaORM (sync API or more layers than the queries need); Redis for the link directory (Postgres `LISTEN/NOTIFY` is enough) |
| Replicas | One in v1, Deployment with no volume | Several now (needs the link directory first) |
| Web serving | Orchestrator serves the bundle in local mode; a separate static image on Kubernetes | One way only (local mode would need a second server, or the stateful pod would serve static files in the cluster) |
| Daemon mTLS through the gateway | LoadBalancer by default, TLS passthrough as an option | Gateway terminates and forwards the client certificate in a header (spoofable from inside the cluster) |

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Constitution IV, V and VI must change | Blocks all client work | N1 first (approved in principle by jaro) |
| Replacing the desktop app before parity | Users lose provisioning, settings or review | Iced app in maintenance until M7's parity rows ship; retirement is its own issue (N13) |
| Terminal sessions lose their UI | After retirement, Codex, Copilot, Pi and OpenCode cannot be driven until their structured adapters ship | Accepted by jaro (decision 8): Claude Code first; terminal sessions listed and stoppable; adapters later (N14); the release notes of the retirement say so |
| `react-native-web` and Paper gaps (focus rings, keyboard navigation, hover, text selection in long transcripts, diff view) | Web feels less capable than the iced app | Platform files where needed; Playwright keyboard tests; spike in #1735 to check the transcript and a diff view before committing |
| Large transcripts in the browser | Jank, memory | FlashList virtualization, server-side page of history plus live tail, a load test with the transcript that caused #1683 |
| Claude credential policy | A design that relays Claude tokens would break Anthropic's terms | No Claude login in Micold; API key or the user's own sign-in in unmodified Claude Code per daemon |
| Single orchestrator replica | Downtime during upgrades (seconds); on node loss, down until the pod is force-deleted | Sessions keep running in daemons; clients resume by sequence; node-loss runbook; [Scalability path](#scalability-path) |
| Gateways differ (passthrough, WebSocket timeouts); ingress-nginx retired upstream | Chart works on one cluster, not another | Gateway API first, classic `Ingress` template second, link on a `LoadBalancer` by default, kind CI with Envoy Gateway |
| Google refuses redirect URIs on private domains | No Google sign-in on a `.lan` deployment | Documented; GitHub works; a public web host name |
| Postgres is one more service for self-hosters | Harder first install | Bring-your-own (managed DB or CloudNativePG); the chart can install a demo CloudNativePG cluster; local mode needs none |
| Schema migrations on upgrade | A failed or newer migration blocks start | Forward-only migrations tested in CI from every released schema; refuse a newer schema; docs say to back up first |
| Pod provisioner needs Kubernetes API access | A compromised orchestrator can create pods | Off by default; a namespaced Role limited to one sessions namespace; NetworkPolicies |
| The orchestrator reads all traffic | A compromised cluster sees code and prompts | Documented; E2E encryption stays future work; audit log; NetworkPolicies |
| OAuth misconfiguration by self-hosters | Cannot sign in | `config check` validates providers and redirect URIs; docs for creating both OAuth apps |
| `ClientMsg` is large (~76 variants) and grows | Unclassified or unhandled variants on the client | Authorization table with an exhaustiveness test; generated discriminated unions with `never` checks in TS |

## Owner decisions (2026-11-14)

1. Constitution amendment: **yes**, and the web interface **replaces** the Rust UI.
2. **Structured view only**; no terminal view.
3. The web interface may be published on a network, so access **always requires authentication**;
   Claude Code is supported first. (The "log in to Claude and pass the token" part is replaced by
   the [Claude credentials](#claude-credentials) design, see Q-A.)
4. **Web only** for now; the mobile app is future work.
5. **One orchestrator replica** for now, designed to scale later.
6. Storage in **PostgreSQL**.
7. Claude credentials (Q-A): **both routes**, an API key or cloud credential file per daemon, or the
   user's own sign-in in unmodified Claude Code on that daemon; Micold never handles the token.
8. Other providers (Q-B, 2026-11-15): **no**, they do not block the web client or the iced app's
   retirement. Focus on delivering the UI for Claude Code; Codex, Copilot, Pi and OpenCode ship later
   (N14).

## Open questions for jaro

- **Q-C — Password-free sign-in**: GitHub and Google for network access, the local access link for
  loopback, no password account. OK?
- **Q-D — Per-daemon grants**: three global roles for "basic permissions". OK?
- **Q-F — Local mode storage**: PostgreSQL for every network deployment. For the loopback-only
  local mode, SQLite through the same traits (no server to run), or Postgres everywhere (one code
  path, but a local Postgres to install)? Recommendation: SQLite locally.
- **Q-E — Names**: the image namespace (`ghcr.io/jaroslawherod/…`) and the public host name pattern
  in the chart's examples.

Also planned without a question (say if you disagree): an owner screen listing signed-in browsers
with sign-out per browser (N3), linking a GitHub and a Google identity to one user, and
accessibility to WCAG 2.2 AA with screen-reader labels on every control.
