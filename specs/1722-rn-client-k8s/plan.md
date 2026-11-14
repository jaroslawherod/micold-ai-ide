# Plan: React Native client for web and mobile, and the orchestrator on Kubernetes

**Date**: 2026-11-14 | **Spec**: [spec.md](spec.md) | **Issues**: [issues.md](issues.md) |
**Epic**: #1722 (revises its M3–M5; base plan `specs/orchestrator/`)

## Summary

The desktop iced app stays. A second client, written once in TypeScript with React Native (Expo and
`react-native-web`), runs in browsers, on Android and on iOS. It talks only to the orchestrator, over
the JSON-over-WebSocket API of #1732, whose TypeScript types are generated from the Rust protocol.
Its Material 3 theme is generated from the same tokens as the desktop app, and its screens follow
the desktop layout. Users sign in with GitHub or Google through the orchestrator (no passwords), and
get one of three roles. The orchestrator ships as an image plus a Helm chart built for Kubernetes:
one stateful replica with a volume, a stateless web image, an ingress for web and API, TLS
passthrough for daemon mTLS links, secrets as files, and health, readiness and metrics endpoints.

## Architecture

```text
 Browser / Android / iOS                       Kubernetes cluster (owner's)
 ┌───────────────────────┐   HTTPS   ┌──────────────────────────────────────────────────────────┐
 │ apps/client (Expo RN) │──────────▶│ Ingress  micold.example.com                              │
 │  - screens, M3 theme  │           │   /            → Service web  → Deployment web (static)  │
 │  - @micold/protocol   │   WSS     │   /api, /auth  → Service orch → StatefulSet orchestrator │
 │    (generated TS)     │──────────▶│                                   (1 replica, PVC)       │
 └───────────────────────┘           │ TLS passthrough link.micold.example.com                  │
                                     │   → Service orch-link :8443 (mTLS, no termination)       │
 Desktop app (iced) ─ local socket ──│   (local orchestrator, or remote after sign-in, #1741)   │
                                     │ Service orch-ops :9090 /healthz /readyz /metrics (no ing)│
 GitHub OAuth / Google OIDC ◀────────│ orchestrator brokers sign-in, holds client secrets       │
                                     │                                                          │
 Daemons anywhere ─ mTLS WSS link ──▶│ link host        In-cluster daemon pods ─ Service DNS ──▶│
 (laptop, SSH host, container)       └──────────────────────────────────────────────────────────┘
```

- **Sessions stay in the daemons** (unchanged rule of `specs/orchestrator/`). The orchestrator is a
  router plus the owner of the registry, CA, users, invites, refresh tokens and audit log.
- **The client never reaches a daemon**; every request goes through the orchestrator, which
  authorizes it by role and audits it.
- **Local use is unchanged**: the desktop app starts a local orchestrator and talks over the local
  socket with no sign-in (FR-033).

### Client (`apps/client/`)

| Concern | Choice |
|---|---|
| Framework | Expo SDK (managed workflow) with Expo Router; `react-native-web` for the browser build |
| Language | TypeScript, `strict` |
| Material 3 | `react-native-paper` v5 (MD3 components and theme) for the base components; Micold-specific components (transcript cards, session strip, overview card, diff view) in `apps/client/src/components/`, one shared library (Principle VIII applied to the client) |
| Theme | `apps/client/src/theme/tokens.generated.ts`, produced by a new `micold-tokens-ts` binary beside `micold-tokens-css` (`crates/micold-core/src/bin/`), read into a Paper `MD3Theme` for light and dark; Roboto and Fira Mono from `assets/fonts/` |
| Protocol | `packages/protocol/` (`@micold/protocol`): TS types + JSON Schema generated from the Rust types (see #1732 below), and a small hand-written client: socket, reconnect with backoff, resume by sequence, request/response correlation |
| State | Zustand stores per feature (sessions, transcript, composer, attention), fed by the socket; no server cache library, since the socket pushes state |
| Lists | `@shopify/flash-list` for the transcript and long lists (virtualized; the desktop's large-transcript CPU issue #1683 is the warning) |
| Markdown and code | `react-native-markdown-display` plus a highlighter on web; plain monospace on native at first |
| Auth | `expo-auth-session` (PKCE, system browser) against the orchestrator; `expo-secure-store` on native, an `HttpOnly` cookie on web |
| Notifications | in-app snackbar and attention list everywhere; `expo-notifications` local notifications while the app runs; browser `Notification` on web |
| Tests | Jest + React Native Testing Library (units), Playwright against the web build (end to end), Maestro against an Android emulator (end to end) |
| Builds | web: `expo export --platform web` into a static image; Android: Gradle (`expo prebuild`) on Linux CI; iOS: EAS Build (hosted macOS), see Q4 |

The workspace uses pnpm; versions are pinned and managed through `mise` (`node`, `pnpm` tools), so
`mise run client-test`, `client-web`, `client-android` join the existing tasks.

### Feature parity matrix

jaro: "Initially assume similar features and look as current rust ui". Inventory from
`crates/micold-client/src/`; v1 means the first M4 release.

| Desktop surface (module) | Client | Notes |
|---|---|---|
| Sidebar: projects, worktrees, sessions, groups (`ui/sidebar.rs`, `micold-core/src/sidebar_groups.rs`) | v1 | Grouping and stable order come from `micold-core` rules; the orchestrator sends them computed or the rule is ported with shared test vectors (decided in #1732's plan) |
| Overview grid of cards (`ui/overview.rs`, `ui/material/overview_card.rs`) | v1 | One column on phones |
| Structured transcript, insights, subagent threads, usage (`ui/transcript/`) | v1 | |
| Composer, paste, attachments (`ui/composer.rs`) | v1 | Image paste on web; picker on mobile |
| Permission prompts and ask-user answers (`ui/transcript/permission.rs`) | v1 | |
| Mode, model and effort pickers (`ui/mode_control.rs`) | v1 | |
| Confirmations (`ui/confirm_*.rs`) | v1 | Only those of actions the client offers |
| Worktree create, rename, delete (`ui/worktree_form.rs`) | v1 | GitHub and Jira issue filters in v1.1 |
| Session start, stop; session strip (`features/session_strip.rs`) | v1 | Structured sessions; terminal sessions listed only (Q1) |
| Attention and notifications (`features/attention.rs`) | v1 | Foreground only (no remote push, Q3) |
| Quick switcher (`ui/quick_switcher.rs`) | v1 | Ctrl/Cmd+K on web, search screen on mobile |
| Appearance settings (light, dark, system) | v1 | Per device |
| Changes and diff review (`ui/changes.rs`, `ui/material/diff_view.rs`, `review_*`) | v1.1 | Read-only first; review replies later |
| PR status, Jira status chips (`features/pr_status.rs`, `features/jira_status.rs`) | v1.1 | Read from the daemon; tokens stay on the daemon |
| History and parallel runs (`features/history.rs`, `features/runs.rs`) | v1.1 | |
| Plan usage (`features/plan_usage.rs`) | v1.1 | |
| Users, invites, roles, audit (new) | v1 | Owner only; web first |
| Daemons list and revoke (`features/daemons.rs`) | v1 | List, state, revoke, enrollment token; creating container, SSH or pod daemons stays desktop-only |
| Terminal view (`ui/terminal.rs`) | Q1 | Out by default (epic: structured only) |
| Settings: environment, terminal, GitHub and Jira tokens, project setup | desktop only | Secrets and host paths belong to the machine running the daemon |
| Large editor, sandbox status, setup output, Kubernetes live view | desktop only | Revisit after v1.1 |
| Keyboard navigation (#803) | v1 on web | Same shortcuts where the browser allows |

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

- Redirect URIs: the web origin (`https://micold.example.com/auth/done`), the app scheme
  (`io.github.jaroslawherod.micold:/auth/done`) for Android and iOS, and `http://127.0.0.1:<port>/` for the desktop app
  (#1741). The orchestrator keeps an allow-list of them.
- Native redirect: a reverse-domain private-use scheme (RFC 8252 §7.1), since a self-hosted server
  cannot use universal or app links on the owner's own domain.
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
- Storage: users, identities, invites, bootstrap code and refresh-token hashes in a SQLite file on
  the orchestrator volume, behind a new `AuthStore` trait (with an in-memory implementation for
  tests), shaped like the `RegistryStore` that #1724's M5 plans. Principle IV allows SQLite; the
  registry JSON file stays as #1724 ships it. Schema migrations run forward on start, the store
  refuses a newer schema than it knows, and the chart's upgrade notes say to snapshot the volume
  first.
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
| Workloads | orchestrator: StatefulSet, `replicas: 1`, PVC `ReadWriteOncePod` (falls back to `ReadWriteOnce` where the CSI driver lacks it; 1 GiB default) at `/var/lib/micold`, an `emptyDir` at `/tmp` because the root filesystem is read-only; web: Deployment, 2 replicas, HPA optional, PodDisruptionBudget. The volume must be block or local storage: SQLite on NFS is unsupported and the docs say so |
| State | On the PVC: registry JSON (#1724), CA key and certs (#1729), SQLite for users, invites, tokens and audit. Nothing about sessions. Backup: a documented `kubectl cp` of the volume, or a VolumeSnapshot; CA key backup is the one that matters (losing it means every daemon re-enrolls) |
| Why one replica | Each daemon holds one link to one orchestrator and routing state is in memory; two replicas would need a link directory and replica-to-replica forwarding. Deferred (Q5); the store traits keep a Postgres store possible |
| Ingress | One host for web and API: `/` → web, `/api`, `/auth` → orchestrator with WebSocket idle timeouts raised. Gateway API `HTTPRoute` is the primary path (ingress-nginx is retired upstream, best-effort maintenance ended March 2026); a classic `Ingress` template stays for other controllers (Traefik, HAProxy, cloud ones), with their timeout annotations documented |
| Daemon links | mTLS must reach the orchestrator untouched. Default: a `LoadBalancer` Service on 8443 (works on every cloud and with MetalLB). Option: TLS passthrough through a Gateway `TLSRoute` (experimental channel in many implementations) or a controller's passthrough mode, whose prerequisites (for example a controller flag) the chart cannot set and the docs list. Terminating TLS at the ingress and forwarding the client certificate in a header is rejected (spoofable once anything else can reach the Service) |
| TLS | cert-manager `Certificate` for the web/API host (issuer is a chart value); the link host serves the orchestrator's own CA-signed certificate (#1729), which daemons pin |
| Secrets | Kubernetes Secrets mounted as files under `/run/secrets/micold/` (GitHub and Google client secrets, JWT signing key, optional CA key import); the orchestrator config names them by path (#1521 rule). Works with External Secrets / Sealed Secrets unchanged. The bootstrap code is either taken from a Secret the owner created before install (`bootstrap.existingSecret`), or generated by the orchestrator, printed to its log and written to `/var/lib/micold/bootstrap-code` (`kubectl exec … cat`); no Job and no Kubernetes API access is needed |
| Config | A ConfigMap with the orchestrator TOML (public URL, allowed origins, redirect URIs, providers, limits); a checksum annotation restarts the pod on change |
| Probes | `startupProbe` and `readinessProbe` on `/readyz` (store opened, CA loaded, listeners bound), `livenessProbe` on `/healthz` (event loop responsive), all on the ops port 9090, which no ingress routes |
| Metrics | `/metrics` (Prometheus text): links online/offline, clients by kind, messages and bytes per daemon, auth refusals, rate-limit hits; optional ServiceMonitor |
| Shutdown | SIGTERM: readiness fails, links and clients told to reconnect, store flushed, exit; `terminationGracePeriodSeconds: 30`; the StatefulSet replaces the pod in place, so the volume never has two writers. On node loss the pod stays `Terminating` until force-deleted: documented, and a risk below |
| Network | NetworkPolicies: web pod accepts from ingress only; orchestrator accepts 8080 from ingress, 8443 from anywhere (daemons), 9090 from the monitoring namespace; egress to DNS and to 443 outside the cluster CIDRs (standard NetworkPolicy has no hostnames; Cilium or Calico FQDN policies for GitHub and Google are optional values) |
| Security | `runAsNonRoot`, `readOnlyRootFilesystem`, `allowPrivilegeEscalation: false`, all capabilities dropped, `seccompProfile: RuntimeDefault`; no service account token mounted |
| In-cluster daemons | A documented example StatefulSet for a standing daemon (spec 686's image) that enrolls with a token Secret and links to `micold-orchestrator-link.<ns>.svc:8443` (FR-039). The desktop app's pod runtime (686) keeps working and may link through the external host |
| Scaling | Vertical for the orchestrator (requests 100m/128Mi, limits a chart value; tens of daemons and clients per replica); horizontal for web |
| Local and compose | A `docker compose` file for one machine stays as a small example (FR-027 of the base plan), generated from the same config |
| CI | A `kind` job: build both images, `helm install` with Envoy Gateway as the Gateway API implementation, a pre-created bootstrap Secret and a fake OAuth provider (a small OIDC mock in the test), enroll a daemon pod, run the Playwright smoke test through the gateway; `helm lint` and `kubeconform` on every chart change |

## Constitution Check

| Principle | Status |
|---|---|
| I. Test-First | Kept: each client issue is TDD with Jest/RNTL, Playwright and Maestro; Rust parts as today |
| II. Multi-Session | Kept: sessions stay in daemons; the client shows many sessions; input serialized by the daemon |
| III. Worktree Integration | Kept: worktree actions are the daemon's existing ones, routed |
| IV. Local-First (NON-NEGOTIABLE) | **Amended**, not broken in spirit: the desktop app and local orchestrator still work offline with no sign-in, and sessions stay on the daemon's machine. But "all application state on the local filesystem" and "no separate server process" (Technology constraints, Persistence) do not hold for an opt-in remote orchestrator on a cluster volume, or for a mobile client. N1 adds an explicit, opt-in remote-deployment clause |
| V. Rust + iced Stack | **Violated** by a TypeScript/React Native client. Needs an amendment (MAJOR bump, 2.0.0): "the desktop application is Rust + iced; a remote client for web and mobile MAY be TypeScript + React Native, talking only to the orchestrator's network API with generated types; no other GUI framework". Proposed as issue N1, before any client code |
| VI. Cross-Platform Parity | Text is out of date after #1897 (Linux only). The amendment scopes it: the desktop app targets Linux; the client targets web, Android and iOS, with iOS gated on Q4 |
| VII. Documentation | Kept: user guide pages for the client, sign-in, roles and the Helm chart in the same changes; the chart's `values.yaml` documented and checked |
| VIII. Reusable UI Components | Applied to the client: one component library in `apps/client/src/components/`; the builder-API clause is iced-specific and the amendment says the client follows React's own idiom (typed props) instead |

"Technology, Storage & Licensing Constraints" is amended with Principles IV, V and VI: GUI (iced for
the desktop, React Native for the remote client), Persistence (an opt-in remote orchestrator may keep
its own store on its volume) and Distribution (Linux desktop builds, web bundle, Android and, after
Q4, iOS).

## Milestones and dependencies

Done: #1723, #1728; #1724 M1–M4 merged (M5 store, M6 pending). Open in M1/M2: #1724, #1725, #1726,
#1727, #1729, #1730, #1731. Numbers `N1`… are proposed new issues ([issues.md](issues.md)).

| Milestone | Delivers | Issues | Depends on | Estimate |
|---|---|---|---|---|
| **M1** (unchanged) | Local orchestrator | #1724, #1725, #1726, #1727 | — | in progress |
| **M2** (unchanged) | mTLS daemons anywhere | #1729, #1730, #1731 | #1724, #1728 | 4–6 weeks |
| **M3 — Network API, sign-in, roles** | Authorized JSON API, GitHub/Google sign-in, roles, audit | N1 (constitution), #1732 (revised), #1733 (replaced), #1734 (revised) | #1726 (routing); #1733 on #1724's M5 store; #1734 no longer waits for #1727 | 4–6 weeks |
| **M4a — Client foundation** | Expo app on web and Android, M3 theme, sign-in, sidebar and overview, CI | N2 (tokens to TS), #1735 (replaced) | N1, #1732, #1733 | 3–4 weeks |
| **M4b — Structured sessions** | Transcript, composer, prompts, pickers, lifecycle, attention, quick switcher, users screen | #1736, #1737, #1738, #1739 (revised), N3 (admin screens) | M4a, #1734 | 5–7 weeks |
| **M5 — Kubernetes** | Images, Helm chart, gateway and link Service, probes, metrics, kind CI | #1742 (revised, first), #1740 (replaced), N4 (web image) | #1742 on #1732; #1740 on #1730 (mTLS link), #1734, #1742, N4; N4 on #1735 | 3–4 weeks |
| **M5b** | Desktop app against a remote orchestrator | #1741 (revised: loopback PKCE) | #1733 | 1–2 weeks |
| **M6 — Parity v1.1 and mobile release** | Changes review, PR/Jira chips, history, plan usage; store builds | N5, N6, N7 (mobile release) | M4b; N7 on Q4 | 4–6 weeks |
| **Optional** | Terminal view in the client | N8 | Q1 = yes; M4b | 2–3 weeks |

```text
#1724 ─▶ #1725 ─▶ #1726 ─┬─▶ #1727
                         └─▶ #1732 ─▶ #1733 ─▶ #1734
#1724 (M5 store) ─▶ #1733      #1732 ─▶ #1742
#1728 ✓ ─▶ #1729 ─▶ #1730 ─▶ #1731
N1, N2, #1732, #1733 ─▶ #1735
#1734, #1735 ─▶ N3
#1735 ─▶ #1736 ─▶ #1737 ;  #1736 ─▶ #1739 ─▶ N7 ;  #1735 ─▶ #1738
#1736 ─▶ N5 ;  #1735 ─▶ N6 ;  #1735 ─▶ N4
#1730, #1734, #1742, N4 ─▶ #1740
```

M4a and M2 can run in parallel once #1732 and #1733 land; M5 can start on #1742 as soon as #1732
exists. The critical path is #1724 → #1725 → #1726 → #1732 → #1733 → #1735 → #1736 → #1737.

## Research and rejected alternatives

| Decision | Chosen | Rejected, and why |
|---|---|---|
| RN setup | Expo managed + Expo Router + `react-native-web` | Bare React Native CLI (no web story, more native config to own); separate React web app + RN app (two codebases, against the request); Flutter (not React Native); Tauri/iced on mobile (iced has no production mobile target) |
| M3 components | `react-native-paper` v5 | Tamagui or NativeWind (not Material 3); hand-built components only (months of work Paper already covers); Material Web components (web only) |
| Theme source | Generate from `micold-core` tokens | Paper's own `MD3LightTheme` with a seed (drifts from the desktop, ignores our shape and type scale) |
| Schema | `schemars` → JSON Schema → TS | `ts-rs`/`specta` (TS only, no schema for validation in tests and tooling); hand-kept types (drift, rejected in the base spec) |
| Wire | JSON over WebSocket | gRPC-web (needs a proxy, no gain for a pushed event stream); postcard in WASM (base spec rejected it) |
| Auth broker | Orchestrator brokers GitHub/Google, issues its own tokens | Client talks to providers directly (provider tokens in the app, GitHub has no ID token to verify); a separate IdP such as Dex or Keycloak (one more stateful service to run, overkill for one owner); oauth2-proxy at the ingress (cannot express roles or reach WebSocket per-message authorization, and breaks the native apps' PKCE flow) |
| Local owner account | Replaced by bootstrap code + GitHub/Google | Password account (a password store to secure, reset flows, brute-force limits) |
| Permissions | Three global roles | Per-daemon grants now (more UI and tests for a single-owner tool; deferred with tenant id kept) |
| State on K8s | One replica + PVC (SQLite + JSON files) | Postgres now (another dependency for every self-hoster); several replicas now (needs cross-replica routing) |
| Web serving | Separate static image | Bundle served by the orchestrator (couples releases, makes the stateful pod serve static files, base FR-021) — kept possible for the local orchestrator via a flag later |
| Daemon mTLS through ingress | TLS passthrough or LoadBalancer | Ingress terminates and forwards the client certificate in a header (spoofable from inside the cluster) |

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Constitution V and VI must change | Blocks all client work | N1 first, small, owner-approved |
| Two UI stacks to keep in step | Parity drifts; double work for each feature | Generated protocol and theme; shared rule vectors from `micold-core` (sorting, grouping) checked in both test suites; parity matrix in the docs, updated per feature |
| `react-native-web` and Paper gaps (focus rings, keyboard navigation, hover, text selection in long transcripts) | Web feels less native than the desktop | Platform files where needed; Playwright keyboard tests; spike in #1735 |
| Large transcripts on phones | Jank, memory | FlashList virtualization, server-side page of history plus live tail, a load test with the transcript that caused #1683 |
| iOS builds need macOS | No iOS app from Linux CI | EAS Build (hosted), or web-only on iOS as a PWA (Q4) |
| App store review and accounts | Release delay, cost (Apple $99/yr, Google $25) | Q4; web and sideloaded Android first |
| Single orchestrator replica | Downtime during upgrades (seconds); on node loss, down until the pod is force-deleted | Sessions keep running in daemons; clients resume by sequence; node-loss runbook in the docs; HA deferred (Q5) |
| Ingress controllers differ (passthrough, WebSocket timeouts); ingress-nginx retired upstream | Chart works on one cluster, not another | Gateway API first, classic `Ingress` template second, link on a `LoadBalancer` by default, kind CI with Envoy Gateway |
| Google refuses redirect URIs on private domains | No Google sign-in on a `.lan` deployment | Documented; GitHub works; a public web host name |
| SQLite on a network filesystem | Corrupt store | Docs require block or local storage |
| The orchestrator reads all traffic | A compromised cluster sees code and prompts | Documented; E2E encryption stays future work; audit log; NetworkPolicies |
| OAuth misconfiguration by self-hosters | Cannot sign in | `micold-daemon orchestrator config check` validates providers and redirect URIs; docs with screenshots for creating both OAuth apps |
| `ClientMsg` is large (~76 variants) and grows | Unclassified or unhandled variants on the client | Authorization table with an exhaustiveness test; generated discriminated unions with `never` checks in TS |

## Open questions for jaro

1. **Q1 — Terminal view**: you asked for the same features as the desktop app; the epic said
   structured only. Recommendation: structured only in v1 (no xterm.js), terminal sessions listed
   with state; a terminal view (xterm.js on web, xterm.js in a WebView on mobile) as optional issue N8
   after v1.
2. **Q2 — Local owner account**: I replaced it with GitHub/Google plus a one-time bootstrap code, and
   kept "no sign-in over the local socket". Keep a password account for air-gapped self-hosting?
   Recommendation: no.
3. **Q3 — Mobile push**: out of v1 (foreground and local notifications only). Add Expo push
   (APNs/FCM through Expo's service, which sees notification text) or self-run FCM later?
   Recommendation: later, with notification text limited to "a session needs you".
4. **Q4 — iOS and stores**: build iOS with EAS Build (hosted macOS) and publish to the App Store and
   Play Store, or ship web + sideloaded Android first? Recommendation: web + Android APK first, stores
   in M6.
5. **Q5 — High availability**: one orchestrator replica with a volume in v1. Is a few seconds of
   reconnect during an upgrade acceptable? Recommendation: yes; HA with Postgres later if needed.
6. **Q6 — Per-daemon grants**: dropped to three global roles for "basic permissions". OK?
7. **Q7 — Where the code lives**: `apps/client/` and `packages/protocol/` in this repository (pnpm
   workspace, own CI jobs). Recommendation: same repository, so generated files and the Rust types
   change in one PR.
8. **Q8 — Android dynamic colour**: use the wallpaper colours (Material You) on Android 12+, or always
   the desktop seed `#6750A4`? Recommendation: the desktop seed, so every client looks the same.
9. **Q9 — Linked identities**: may one person attach both a GitHub and a Google identity to one user?
   Recommendation: yes, from their own profile screen after sign-in.
10. **Q10 — Names**: the image namespace (`ghcr.io/jaroslawherod/…`), the app id
    (`io.github.jaroslawherod.micold`) and, if stores are approved, the publisher account.

Also planned without a question (say if you disagree): an owner screen listing signed-in devices
with sign-out per device (N3), and accessibility to WCAG 2.2 AA on web with screen-reader labels on
every control (in each client issue's acceptance).
