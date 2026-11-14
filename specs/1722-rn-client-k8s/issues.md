# Proposed issue changes for the React Native client and Kubernetes revision

**Created 2026-11-14.** Not applied: jaro approves this list first, then the changes are made to the
GitHub issues of epic #1722 (bodies, titles, "blocked by" relations, milestones M3–M5). Labels as in
`specs/orchestrator/issues.md`: `flow:feature` (or `flow:chore`) plus `effort:high` or `effort:low`.
Every body ends with the common footer:

> Plan: `specs/orchestrator/` and `specs/1722-rn-client-k8s/` (spec.md, plan.md). Implement test-first
> in its own worktree (`/speckit-autopilot #<this>`).

PR titles follow the project rule: `#<issue>: <issue title>`.

## Summary

| Issue | Action | New title |
|---|---|---|
| #1724–#1727, #1729–#1731 | keep | unchanged |
| #1732 | update | feat: JSON-over-WebSocket client API with generated TypeScript types and JSON Schema, origin checks, resume |
| #1733 | replace (rewrite body and title) | feat: sign-in with GitHub and Google through the orchestrator, invites, bootstrap code, short-lived tokens |
| #1734 | update | feat: roles owner/operator/viewer, user management, audit log |
| #1735 | replace | feat: React Native (Expo) client scaffold for web and Android, Material 3 theme, sign-in, sidebar and overview |
| #1736 | update | feat: client — live structured transcript, read-only |
| #1737 | update | feat: client — composer: send, paste, permission prompts, slash commands, model/effort/permission pickers |
| #1738 | update | feat: client — worktree and structured-session lifecycle; terminal sessions listed but not openable |
| #1739 | update | feat: client — attention list, in-app and local notifications, quick switcher |
| #1740 | replace | feat: Kubernetes deployment — orchestrator image, Helm chart, ingress, TLS passthrough for daemon links, secrets as files |
| #1741 | update | feat: the desktop app connects to a remote orchestrator after GitHub or Google sign-in |
| #1742 | update | feat: health, readiness and metrics endpoints, rate limits, bandwidth caps, graceful shutdown |
| N1 | new | chore: amend the constitution for a React Native client and Linux-only desktop |
| N2 | new | feat: `micold-tokens-ts` — the design tokens as a generated TypeScript theme |
| N3 | new | feat: client — users, invites and roles; daemons list and revoke (owner) |
| N4 | new | feat: web client image and its place in the Helm chart |
| N5 | new | feat: client — changes and diff review, read-only |
| N6 | new | feat: client — PR and Jira status, history, plan usage |
| N7 | new | chore: mobile release — Android signed builds, iOS via EAS Build, store listings (after Q4) |
| N8 | new, optional | feat: client — terminal view (only if Q1 = yes) |

## Updated and replaced issues

### #1732 — update

- **Milestone**: M3 · **Depends on**: #1726 · **Covers**: FR-017R, FR-030
- **Body**: JSON over WebSocket at `/api/ws` for the client protocol, inside the routed envelope of
  #1723. Add `schemars` derives behind a `schema` feature of `micold-core`, a
  `micold-protocol-schema` binary writing `packages/protocol/schema.json`, and TypeScript generated
  from it (`packages/protocol/src/index.generated.ts`); `mise run protocol-schema` regenerates, the
  gate fails on drift. Each `ClientMsg` variant is classified `Read` or `Change` in one table with an
  exhaustiveness test (used by #1734). Localhost only unless configured; `Origin` allow-list; 20 s
  ping; resume by sequence after reconnect. The plan decides whether sidebar grouping and order are
  sent computed or ported with shared test vectors.
- **Acceptance**: drift check fails on a stale schema; a test client lists daemons and reads a
  structured session; a reconnect resumes with no gap or duplicate; a foreign `Origin` is refused;
  every variant is classified.

### #1733 — replace

- **Milestone**: M3 · **Depends on**: #1732, #1724 (M5 store) · **Covers**: FR-018R, FR-031, FR-032, FR-033, FR-037
- **Body**: The orchestrator brokers sign-in with GitHub (OAuth 2.0, identity from `/user` keyed on
  the numeric id) and Google (OIDC, keyed on `sub`); it holds the provider client secrets as mounted
  files. Clients use authorization code + PKCE against the orchestrator with allow-listed redirect
  URIs (web origin, the reverse-domain scheme `io.github.jaroslawherod.micold:/` per RFC 8252,
  desktop loopback). It issues EdDSA access tokens (15 min, key set with `kid`) and rotating, hashed
  refresh tokens (30 days); the access token goes in the socket's first frame, never the URL. Only
  invited identities sign in (Google: `email_verified` only); the first owner binds with a one-time
  bootstrap code (24 h, from a pre-created Secret or generated, logged and written to the volume;
  `bootstrap-code --reset` CLI). Users, identities, invites, bootstrap and refresh tokens live in
  SQLite behind a new `AuthStore` trait with forward-only migrations. `/auth/*` is rate limited.
  Local socket clients stay owner without sign-in. No password exists.
- **Acceptance**: against a fake provider, GitHub and Google sign-in yield tokens; an uninvited
  identity is refused and not stored; the bootstrap code works once; an expired access token is
  refused and refresh works; a removed user's refresh fails and open sockets close; `config check`
  validates providers and redirect URIs; no secret in logs or `Debug`.

### #1734 — update

- **Milestone**: M3 · **Depends on**: #1733 (no longer #1727) · **Covers**: FR-019R, FR-020
- **Body**: One global role per user: owner, operator, viewer (viewer default), enforced before
  routing with the classification table of #1732 and the action table of `plan.md`. Audit every
  changing action, sign-in, refusal, invite and role change (actor, target, outcome; never typed
  text) in SQLite, owner-readable through the API. MCP actions from #1727 are audited with the
  session as actor once #1727 lands. Per-daemon grants deferred.
- **Acceptance**: a viewer reads and every changing action is refused; an operator cannot manage
  users or read the audit; every changing action writes one entry with no typed text; a role
  downgrade applies to an open socket on its next message; a socket closes when its access token
  expires.

### #1735 — replace

- **Milestone**: M4a · **Depends on**: N1, N2, #1732, #1733 · **Covers**: FR-021R, FR-022, FR-034,
  FR-035, FR-037, FR-041
- **Body**: Expo (managed, Expo Router) TypeScript app in `apps/client/` with `react-native-web`;
  pnpm workspace with `packages/protocol/`; a first-start screen for the orchestrator URL (FR-041); Node and pnpm pinned in `mise.toml`; tasks `client-test`,
  `client-web`, `client-android`. Paper v5 theme from N2's tokens, Roboto and Fira Mono. Sign-in
  screen (configured providers only), secure token storage, socket client with reconnect. Sidebar and
  overview grid matching the desktop layout, stable order with per-row state (#1718), one column
  below 600 px. CI: lint, typecheck, Jest, web export, Android debug build, Playwright smoke, bundle
  check for no terminal emulator (SC-007).
- **Acceptance**: web and Android builds sign in and list projects, worktrees and sessions of every
  daemon; a state change updates the indicator, not the position; screenshots at 1280, 800 and
  390 px in light and dark pass a visual pass against the desktop's.

### #1736, #1737, #1738 — update

Bodies as in `specs/orchestrator/issues.md` entries 14–16, with: "web client" becomes "client (web
and Android)"; FlashList virtualization and a large-transcript load test (#1736); image paste on
web and an image picker on mobile (#1737); confirmations as the desktop's (#1738). Milestone M4b.
Each one's end-to-end test runs on Playwright and on Maestro. Covers: #1736 FR-023, #1737 FR-024,
#1738 FR-025.

### #1739 — update

- **Milestone**: M4b · **Depends on**: #1736 · **Covers**: FR-026, FR-036
- **Body**: the attention list and in-app snackbars as on the desktop; browser notifications on web
  and `expo-notifications` local notifications on mobile while the app runs; the quick switcher
  (Ctrl/Cmd+K on web, a search screen on mobile). Remote push out of scope (Q3).

### #1740 — replace

- **Milestone**: M5 · **Depends on**: #1730, #1734, #1742, N4 · **Covers**: FR-027R, FR-038, FR-039
- **Body**: Orchestrator image (distroless, non-root, read-only root, amd64+arm64, cosign-signed, on
  GHCR) and Helm chart `deploy/helm/micold`: StatefulSet (1 replica) + PVC, Services (API 8080, link
  8443, ops 9090), Gateway API `HTTPRoute` (primary) and a classic `Ingress` template, link on a
  `LoadBalancer` Service by default or TLS passthrough (`TLSRoute`) as an option, `ReadWriteOncePod`
  volume and `/tmp` `emptyDir`, cert-manager `Certificate`, Secrets mounted as files, ConfigMap with
  checksum restart, NetworkPolicies (egress 443 outside cluster CIDRs), optional ServiceMonitor,
  bootstrap code from a pre-created Secret or the volume, in-cluster daemon StatefulSet example,
  compose example. Docs: install, OAuth app setup, backup of the volume and the CA key, upgrade.
- **Acceptance**: the kind CI job installs the chart, signs in through a fake provider, enrolls a
  daemon pod and drives a structured session through the ingress (SC-010); a pod delete loses
  nothing and clients are back within 30 s (SC-011); `helm lint` and `kubeconform` pass; no secret
  in `helm get values`, env or logs.

### #1741 — update

- **Depends on**: #1733 · **Covers**: FR-028 · **Body**: as before, sign-in through the system browser with a loopback
  redirect and PKCE; tokens in the OS keyring.

### #1742 — update

- **Milestone**: M5 · **Depends on**: #1732 (no longer #1740) · **Covers**: FR-029R, FR-038
- **Body**: ops port with `/healthz`, `/readyz` (store, CA, listeners), `/metrics` (Prometheus);
  per-client rate limits and per-daemon bandwidth caps; graceful SIGTERM (readiness off, reconnect
  notice, flush, exit).
- **Acceptance**: readiness is false until the store opens; SIGTERM ends within 30 s with clients
  reconnecting; metrics report links, clients, bytes, refusals; a client over its limit is slowed or
  refused, others are not.

## New issues

### N1 — chore: amend the constitution for a React Native client and Linux-only desktop

- **Milestone**: M3 · **Depends on**: — · **Labels**: `flow:chore`, `effort:low`
- **Body**: Constitution 2.0.0: Principle IV gains an explicit, opt-in remote-deployment clause (a
  remote orchestrator keeps its own store on its volume; the desktop stays offline-capable); Principle V allows a TypeScript + React Native client that talks only
  to the orchestrator's network API with generated types; VI scoped to Linux for the desktop (#1897)
  and web, Android, iOS for the client; VIII applies to the client's own component library in React's
  idiom; Technology constraints (GUI, Persistence, Distribution) updated. Owner approval required
  (Governance).

### N2 — feat: `micold-tokens-ts`

- **Milestone**: M4a · **Depends on**: — · **Covers**: FR-034
- **Body**: A pure `tokens::ts::module()` beside `tokens::css` and a `micold-tokens-ts` binary writing
  `apps/client/src/theme/tokens.generated.ts` (colour roles light and dark, type scale, shape,
  elevation, motion, state layers); unit tests like `tokens/css.rs`; drift check in the gate.

### N3 — feat: client — users, invites, roles; daemons list and revoke

- **Milestone**: M4b · **Depends on**: #1734, #1735 · **Covers**: FR-032, FR-036
- **Body**: Owner screens to invite by GitHub login or Google email with a role, change roles, remove
  users, read the audit log, list signed-in devices and sign one out; list daemons with state,
  create enrollment tokens, revoke.

### N4 — feat: web client image and Helm values

- **Milestone**: M5 · **Depends on**: #1735
- **Covers**: FR-027R · **Body**: Static image of the web export behind an unprivileged static server with CSP and cache
  headers; Deployment, Service, PodDisruptionBudget and optional HPA in the chart.

### N5 — feat: client — changes and diff review, read-only

- **Milestone**: M6 · **Depends on**: #1736 · **Covers**: FR-040 · **Body**: the desktop's Changes view (file tree,
  filters, diffs) on wide screens; file list then diff on phones.

### N6 — feat: client — PR and Jira status, history, plan usage

- **Milestone**: M6 · **Depends on**: #1735 · **Covers**: FR-040 · **Body**: the desktop's PR and Jira chips, history
  and runs lists, and plan usage, read from the daemons.

### N7 — chore: mobile release

- **Milestone**: M6 · **Depends on**: #1739, Q4 · **Body**: signed Android release builds on Linux
  CI; iOS through EAS Build if approved; store listings, privacy text, deep-link domains.

### N8 — feat: client — terminal view (optional)

- **Depends on**: Q1 = yes, #1737 · **Body**: xterm.js on web and in a WebView on mobile, fed by the
  orchestrator relaying terminal output; SC-007 removed.
