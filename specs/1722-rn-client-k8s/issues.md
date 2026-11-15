# Proposed issue changes for the web client and Kubernetes revision

**Created 2026-11-14, revision 2** (after jaro's answers). Not applied: jaro approves this list first,
then the changes are made to the GitHub issues of epic #1722 (bodies, titles, "blocked by"
relations, milestones). Labels as in `specs/orchestrator/issues.md`: `flow:feature` (or
`flow:chore`) plus `effort:high` or `effort:low`. Every body ends with the common footer:

> Plan: `specs/orchestrator/` and `specs/1722-rn-client-k8s/` (spec.md, plan.md). Implement test-first
> in its own worktree (`/speckit-autopilot #<this>`).

PR titles follow the project rule: `#<issue>: <issue title>`.

## Summary

| Issue | Action | New title |
|---|---|---|
| #1724–#1727, #1729–#1731 | keep | unchanged (#1724's M5 lands on the `RegistryStore` trait; N16 adds Postgres behind it) |
| #1732 | update | feat: JSON-over-WebSocket client API with generated TypeScript types and JSON Schema, origin checks, resume |
| #1733 | replace | feat: always-authenticated access — GitHub and Google sign-in, invites, bootstrap code, local access link |
| #1734 | update | feat: roles owner/operator/viewer, user management, append-only audit log |
| #1735 | replace | feat: React Native (Expo) web client scaffold, Material 3 theme from the tokens, sign-in, sidebar and overview |
| #1736 | update | feat: web client — live structured transcript, read-only |
| #1737 | update | feat: web client — composer: send, paste, permission prompts, slash commands, model/effort/permission pickers |
| #1738 | update | feat: web client — worktree and structured-session lifecycle; terminal sessions listed and stoppable |
| #1739 | update | feat: web client — attention list, browser notifications, quick switcher, keyboard navigation |
| #1740 | replace | feat: Kubernetes deployment — orchestrator image, Helm chart, Gateway API, LoadBalancer for daemon links, PostgreSQL, secrets as files |
| #1741 | close as not planned | the desktop app is retired instead (N13) |
| #1742 | update | feat: health, readiness and metrics endpoints, rate limits, bandwidth caps, graceful shutdown |
| N1 | new | chore: constitution 2.0.0 — the web client replaces iced; Linux-only daemons; PostgreSQL for network deployments |
| N2 | new | feat: `micold-tokens-ts` — the design tokens as a generated TypeScript theme |
| N3 | new | feat: web client — users, invites, roles, signed-in browsers; daemons list, enrollment and revoke (owner) |
| N4 | new | feat: web client image and its place in the Helm chart |
| N5 | new | feat: web client — changes and diff review |
| N6 | new | feat: web client — PR and Jira status, history and runs, plan usage, setup output, sandbox status |
| N7 | future | feat: native mobile app from the same code base (not before jaro's go-ahead) |
| N9 | new | feat: local mode — the orchestrator serves the web client on loopback with a one-time access link; user service unit |
| N10 | new | feat: Claude credentials per daemon — API key or provider file, sign-in state shown in the client; no Claude login in Micold |
| N11 | new | feat: daemon provisioning moves into the orchestrator (container, SSH host, Kubernetes pod) |
| N12 | new | feat: web client — secret settings (GitHub, Jira, environment), Kubernetes view, large editor |
| N13 | new | chore: retire the iced desktop app (remove `micold-client`, `micold-material`, packaging, docs) |
| N14 | future | feat: structured adapters for Codex, Copilot, Pi and OpenCode (after Claude Code; does not block N13) |
| N15 | future | feat: several orchestrator replicas — Postgres link directory and replica-to-replica forwarding |
| N17 | future | feat: multi-user — each user owns their daemons, projects and sessions; sharing; per-user quotas |
| N16 | new | feat: PostgreSQL store for the orchestrator (sqlx, migrations, store traits; SQLite for local mode) |

The earlier N8 (terminal view) is dropped: structured view only.

## Updated and replaced issues

### #1732 — update

- **Milestone**: M3 · **Depends on**: #1726 · **Covers**: FR-017R, FR-030
- **Body**: JSON over WebSocket at `/api/ws` for the client protocol, inside the routed envelope of
  #1723. Add `schemars` derives behind a `schema` feature of `micold-core`, a
  `micold-protocol-schema` binary writing `packages/protocol/schema.json`, and TypeScript generated
  from it (`packages/protocol/src/index.generated.ts`); `mise run protocol-schema` regenerates, the
  gate fails on drift. Each `ClientMsg` variant is classified `Read` or `Change` in one table with an
  exhaustiveness test (used by #1734). `Origin` allow-list; 20 s ping; resume by sequence after
  reconnect; routing through a `LinkDirectory` trait (in-process now, FR-046). The plan decides
  whether sidebar grouping and order are sent computed or ported with shared test vectors.
- **Acceptance**: drift check fails on a stale schema; a test client lists daemons and reads a
  structured session; a reconnect resumes with no gap or duplicate; a foreign `Origin` is refused;
  every variant is classified.

### #1733 — replace

- **Milestone**: M3 · **Depends on**: #1732, N16 · **Covers**: FR-018R, FR-031, FR-032, FR-033R, FR-037R
- **Body**: Every API request is authenticated. On a network deployment the orchestrator brokers
  sign-in with GitHub (OAuth 2.0, keyed on the numeric user id) and Google (OIDC, keyed on `sub`,
  `email_verified` required for invite matching), holding provider client secrets as mounted files.
  The client uses authorization code + PKCE against the orchestrator with allow-listed redirect URIs.
  EdDSA access tokens (15 min, key set with `kid`) in memory and in the socket's first frame, never
  the URL; rotating, hashed refresh tokens (30 days) in an `HttpOnly` cookie scoped to `/auth`. Only
  invited identities sign in; the first owner binds with a one-time bootstrap code (24 h; from a
  pre-created Secret, or generated, logged and stored hashed; `bootstrap-code --reset`). On loopback
  only, access is through a one-time local access link (N9 serves it). Binding a non-loopback
  address without a provider is refused. Users, identities, invites and tokens are stored through
  `AuthStore` (N16). `/auth/*` is rate limited. No password exists.
- **Acceptance**: no request succeeds without a token (SC-013); GitHub and Google sign-in work
  against a fake provider; an uninvited identity is refused and not stored; the bootstrap code works
  once; refresh works and a removed user's refresh fails and sockets close; `config check` validates
  providers, redirect URIs and the bind rule; no secret in logs or `Debug`.

### #1734 — update

- **Milestone**: M3 · **Depends on**: #1733 · **Covers**: FR-019R, FR-020
- **Body**: One global role per user: owner, operator, viewer (viewer default), enforced before
  routing with #1732's classification table. Audit every changing action, sign-in, refusal, invite
  and role change (actor, target, outcome; never typed text) in an append-only Postgres table (no
  `UPDATE`/`DELETE` grant), owner-readable through the API. MCP actions from #1727 are audited with
  the session as actor once #1727 lands. Per-daemon grants deferred.
- **Acceptance**: a viewer reads and every changing action is refused; an operator cannot manage
  users or read the audit; every changing action writes one entry with no typed text; a role
  downgrade applies to an open socket on its next message; a socket closes when its access token
  expires.

### #1735 — replace

- **Milestone**: M4a · **Depends on**: N1, N2, #1732, #1733 · **Covers**: FR-021R, FR-022, FR-034,
  FR-035, FR-037R
- **Body**: Expo (managed, Expo Router) TypeScript app in `apps/client/` with `react-native-web`, web
  target only; pnpm workspace with `packages/protocol/`; Node and pnpm pinned in `mise.toml`; tasks
  `client-test`, `client-web`. Paper v5 theme from N2's tokens, Roboto and Fira Mono. Sign-in page
  (configured providers only), socket client with reconnect. Sidebar and overview grid matching
  today's layout, stable order with per-row state (#1718), stacked below 600 px. Spike first: render
  a long transcript and a diff with Paper on the web and record the result, so a gap shows before the
  rest is built. CI: lint, typecheck, Jest, web export, Playwright smoke, bundle check for no terminal
  emulator (SC-007).
- **Acceptance**: the web client signs in and lists projects, worktrees and sessions of every daemon;
  a state change updates the indicator, not the position; screenshots at 1280, 800 and 390 px in
  light and dark pass a visual pass against the iced app's.

### #1736, #1737, #1738 — update

Bodies as in `specs/orchestrator/issues.md` entries 14–16, with: "web client" built from
`apps/client/`; Claude Code structured sessions first; FlashList virtualization and a
large-transcript load test (#1736); text and image paste (#1737); terminal sessions listed with
state and stoppable, never rendered (#1738, FR-045). Milestone M4b. Each one's end-to-end test runs
on Playwright. Covers: #1736 FR-023, #1737 FR-024, #1738 FR-025, FR-045.

### #1739 — update

- **Milestone**: M4b · **Depends on**: #1736 · **Covers**: FR-026, FR-036
- **Body**: the attention list and in-app snackbars as in the iced app; browser notifications while a
  tab is open; the quick switcher and command palette (Ctrl/Cmd+K); keyboard navigation and the
  shortcut sheet (#803). Remote push out of scope.

### #1740 — replace

- **Milestone**: M5 · **Depends on**: #1730, #1734, #1742, N4, N16 · **Covers**: FR-027R, FR-038, FR-039
- **Body**: Orchestrator image (distroless, non-root, read-only root, amd64+arm64, cosign-signed, on
  GHCR) and Helm chart `deploy/helm/micold`: orchestrator Deployment (`replicas: 1`, `Recreate`, no
  volume, `/tmp` `emptyDir`), database URL from `database.existingSecret` (optional CloudNativePG
  `Cluster` for demos and CI), Services (API 8080, link 8443 as `LoadBalancer` by default, ops
  9090), Gateway API `HTTPRoute` (primary) and a classic `Ingress` template, optional TLS passthrough
  (`TLSRoute`) for the link, cert-manager `Certificate`, Secrets mounted as files (OAuth client
  secrets, signing key set, CA key), ConfigMap with checksum restart, NetworkPolicies (egress to the
  database, DNS and 443 outside cluster CIDRs), optional ServiceMonitor, bootstrap code from a
  pre-created Secret or generated, in-cluster daemon StatefulSet example, compose example (with
  Postgres). Docs: install, OAuth app setup, database and CA key backup, upgrade.
- **Acceptance**: the kind CI job (Envoy Gateway, CloudNativePG) installs the chart, signs in through
  a fake provider, enrolls a daemon pod and drives a structured session through the gateway
  (SC-010); a pod delete loses nothing and clients are back within 30 s (SC-011); `helm lint` and
  `kubeconform` pass; no secret in `helm get values`, env or logs.

### #1741 — close as not planned

The desktop app is retired (N13), so it never needs to reach a remote orchestrator. Close with a
link to this plan.

### #1742 — update

- **Milestone**: M5 · **Depends on**: #1732 · **Covers**: FR-029R, FR-038
- **Body**: ops port with `/healthz`, `/readyz` (database reachable and migrated, CA, listeners),
  `/metrics` (Prometheus); per-client rate limits and per-daemon bandwidth caps (per replica, behind
  a trait); graceful SIGTERM (readiness off, reconnect notice, finish in-flight writes, exit).
- **Acceptance**: readiness is false until the database is migrated; SIGTERM ends within 30 s with
  clients reconnecting; metrics report links, clients, bytes, refusals; a client over its limit is
  slowed or refused, others are not.

## New issues

### N1 — chore: constitution 2.0.0

- **Milestone**: M3 · **Depends on**: — · **Labels**: `flow:chore`, `effort:low`
- **Body**: Principle V replaced: the UI is one TypeScript + React Native client talking only to the
  orchestrator's API through generated types; Rust for core, daemons and orchestrator; iced retired
  (jaro approved, 2026-11-14). IV amended: local mode works offline on one machine with no external
  service; an opt-in network deployment uses PostgreSQL and GitHub/Google sign-in. VI rewritten:
  daemons and orchestrator on Linux (#1897), the client on current Chromium, Firefox and Safari.
  VIII's builder-API clause becomes "typed props, React idiom". Technology constraints (GUI,
  Persistence, Distribution) updated.

### N2 — feat: `micold-tokens-ts`

- **Milestone**: M4a · **Depends on**: — · **Covers**: FR-034
- **Body**: A pure `tokens::ts::module()` beside `tokens::css` and a `micold-tokens-ts` binary writing
  `apps/client/src/theme/tokens.generated.ts` (colour roles light and dark, type scale, shape,
  elevation, motion, state layers); unit tests like `tokens/css.rs`; drift check in the gate.

### N3 — feat: web client — users, invites, roles; daemons

- **Milestone**: M4b · **Depends on**: #1734, #1735 · **Covers**: FR-032, FR-036
- **Body**: Owner screens to invite by GitHub login or Google email with a role, change roles, remove
  users, link a second identity, read the audit log, list signed-in browsers and sign one out; list
  daemons with state and Claude sign-in state, create enrollment tokens, revoke.

### N4 — feat: web client image and Helm values

- **Milestone**: M5 · **Depends on**: #1735 · **Covers**: FR-027R
- **Body**: Static image of the web export behind an unprivileged static server with CSP and cache
  headers; Deployment, Service, PodDisruptionBudget and optional HPA in the chart.

### N5 — feat: web client — changes and diff review

- **Milestone**: M6 · **Depends on**: #1736 · **Covers**: FR-040
- **Body**: the iced app's Changes view (file tree, filters, diffs, reviewed marks), then review
  replies; stacked on narrow screens.

### N6 — feat: web client — PR and Jira status, history, plan usage, setup, sandbox

- **Milestone**: M6 · **Depends on**: #1735 · **Covers**: FR-040
- **Body**: PR and Jira chips, history and parallel runs, plan usage, setup output and sandbox status,
  read from the daemons.

### N9 — feat: local mode

- **Milestone**: M4a · **Depends on**: #1733, #1735 · **Covers**: FR-021R, FR-033R
- **Body**: `micold-daemon orchestrator --serve-web` binds 127.0.0.1, serves the web bundle from the
  API's origin, prints a one-time access link, and exchanges it for an owner session cookie. A
  systemd user unit starts it on login. Stores on SQLite through N16's traits.

### N10 — feat: Claude credentials per daemon

- **Milestone**: M4b · **Depends on**: #1735 · **Covers**: FR-043
- **Body**: The daemon configuration names an Anthropic API key file or Bedrock, Vertex or Foundry
  credentials (secret by file path); the daemon passes them to the Claude Code process it starts.
  Otherwise Claude Code uses the user's own sign-in done in unmodified Claude Code on that daemon.
  The daemon reports its Claude state (`claude auth status`); the client shows it and links to the
  docs. Micold offers no Claude login and never collects, stores or relays a Claude credential; the
  docs quote Anthropic's terms and point to Anthropic for edge cases.

### N11 — feat: daemon provisioning in the orchestrator

- **Milestone**: M7 · **Depends on**: #1726, #1734 · **Covers**: FR-044
- **Body**: Move `micold_core::provision` (806) and the Kubernetes runtime (686) behind a
  `Provisioner` trait in the orchestrator: local container, SSH host, Kubernetes pod. Owner-only
  actions in the web client (add, start, stop, remove). The pod provisioner uses a ServiceAccount
  with a namespaced Role in one sessions namespace, off by default in the chart.

### N12 — feat: web client — secret settings, Kubernetes view, large editor

- **Milestone**: M7 · **Depends on**: #1734, #1735 · **Covers**: FR-044
- **Body**: GitHub and Jira tokens and environment settings written through the orchestrator to the
  daemon that uses them, never read back in full; the Kubernetes live view; the large editor.

### N13 — chore: retire the iced desktop app

- **Milestone**: M7 · **Depends on**: N5, N6, N11, N12 (not on N14: jaro, 2026-11-15) · **Labels**: `flow:chore`,
  `effort:high` · **Covers**: FR-044, SC-014
- **Body**: After one release with the web client as the default, remove `crates/micold-client`,
  `crates/micold-material`, the iced dependency, AppImage/deb packaging, desktop notifications and
  their docs; keep the tokens in `micold-core`. Close #1741.

### N16 — feat: PostgreSQL store for the orchestrator

- **Milestone**: M3 · **Depends on**: #1724 (M5 `RegistryStore` trait) · **Covers**: FR-042, FR-046
- **Body**: `sqlx` with `PgPool`; migrations in `crates/micold-daemon/migrations/` applied on start,
  forward only, refusing a newer schema; `RegistryStore`, `AuthStore`, `AuditStore` traits with
  in-memory (tests), PostgreSQL and SQLite (local mode) implementations; `tenant_id` on every table
  and `owner_user_id` on every daemon, project and session row (for later multi-user, N17);
  `database.url_file` and `database.ca_file` in the orchestrator config; a one-shot import of
  `registry.json`; CI runs the store tests against a Postgres service container and the migration
  chain from every released schema.
- **Acceptance**: the registry, users and audit survive a restart on Postgres and on SQLite; a newer
  schema is refused naming both versions; no private key is ever written to the database.

### N7, N14, N15, N17 — future

Kept as placeholders so the plan names them: native mobile app (N7), structured adapters for other
providers (N14), several orchestrator replicas (N15), multi-user ownership and sharing (N17). None is created until jaro asks.
