# GitHub issues for "an orchestrator for every daemon, and a web client"

**Created 2026-11-05.** Epic #1722; each entry below is one issue. Sub-issues of the epic carry GitHub "blocked by" relations matching "Depends on". Labels: `flow:feature` plus `effort:high` or `effort:low`. Milestones:

- [M1 — Local orchestrator (MVP)](https://github.com/jaroslawherod/micold-ai-ide/milestone/1): issues #1723, #1724, #1725, #1726, #1727
- [M2 — Daemons anywhere, with mTLS](https://github.com/jaroslawherod/micold-ai-ide/milestone/2): issues #1728, #1729, #1730, #1731
- [M3 — Network API, users, authorization](https://github.com/jaroslawherod/micold-ai-ide/milestone/3): issues #1732, #1733, #1734
- [M4 — Web client (structured view only)](https://github.com/jaroslawherod/micold-ai-ide/milestone/4): issues #1735, #1736, #1737, #1738, #1739
- [M5 — Cloud deployment](https://github.com/jaroslawherod/micold-ai-ide/milestone/5): issues #1740, #1741, #1742

| Entry | Issue | Milestone | Title |
|---|---|---|---|
| 0 | #1722 | — | Epic: an orchestrator for every daemon, and a web client |
| 1 | #1723 | M1 | contract: daemon-qualified refs and routed envelope (protocol bump) |
| 2 | #1724 | M1 | feat: `micold-daemon orchestrator` — registry, daemon link, heartbeat, offline |
| 3 | #1725 | M1 | feat: daemons dial the orchestrator (host directly; container, SSH and pod through the reverse forward) |
| 4 | #1726 | M1 | feat: the desktop app talks only to the orchestrator (aggregated lists, routed ops and streams, starts it when absent) |
| 5 | #1727 | M1 | feat: MCP tools across daemons (per-daemon opt-out, confirmations routed to the app) |
| 6 | #1728 | M2 | feat: `[orchestrator]` config section and `config check` rules |
| 7 | #1729 | M2 | feat: built-in CA, enrollment tokens and CSR, short-lived certs, renewal, revocation |
| 8 | #1730 | M2 | feat: daemon link over TLS WebSocket — reconnect, resumable streams |
| 9 | #1731 | M2 | feat: the app enrolls the daemons it manages (private key unreadable by sessions) |
| 10 | #1732 | M3 | feat: JSON-over-WebSocket API with a schema generated from the Rust types, localhost-only by default |
| 11 | #1733 | M3 | feat: user auth — OIDC plus a local owner account, short-lived tokens |
| 12 | #1734 | M3 | feat: roles owner/operator/viewer, per-daemon grants, audit log |
| 13 | #1735 | M4 | feat: web client scaffold under `web/`, login, served by the orchestrator, stable-order list with per-row state (see #1718) |
| 14 | #1736 | M4 | feat: web client — live structured transcript, read-only |
| 15 | #1737 | M4 | feat: web client — composer: send, paste (see #1721), permission prompts, slash commands, model/effort/permission pickers |
| 16 | #1738 | M4 | feat: web client — worktree and structured-session lifecycle; terminal sessions listed but not openable |
| 17 | #1739 | M4 | feat: web client — attention notifications while the tab is open |
| 18 | #1740 | M5 | feat: orchestrator image with compose and Helm examples, TLS, persistent volume |
| 19 | #1741 | M5 | feat: the desktop app connects to a remote orchestrator after sign-in |
| 20 | #1742 | M5 | feat: rate limits, bandwidth caps, health and metrics |

Common footer for every body:

> Plan: `specs/orchestrator/` (spec.md, issues.md). Implement test-first in its own worktree
> (`/speckit-autopilot #<this>`).

---

## 0. Epic: an orchestrator for every daemon, and a web client

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: —
- **Depends on**: —
- **Body**:
  An orchestrator that routes every daemon, and a web client. The orchestrator (`micold-daemon orchestrator`) is a router, not a store: each daemon keeps its sessions, worktrees and catalog and dials out to it (over the app's SSH reverse forward locally, over a TLS WebSocket with mTLS in the cloud). The desktop app talks only to the orchestrator; MCP tools see every daemon's sessions of the caller's project; a web client offers the structured view only (no terminal emulator). Single tenant, tenant id in every record; E2E encryption and Web Push are out of scope. Milestones M1–M5; issues 1–20.

## 1. contract: daemon-qualified refs and routed envelope (protocol bump)

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M1 — Local orchestrator (MVP)
- **Depends on**: —
- **Covers**: FR-004, FR-006
- **Body**:
  Add to `micold_core::protocol` a daemon id, daemon-qualified refs `(daemon id, local ref)` for session, worktree and project, and a routed envelope that wraps any client message with its target daemon and a logical connection id (and a sequence number for later resume). Bump `PROTOCOL_VERSION`; document it in `docs/daemon.md`. The plan decides how daemons declare "same project" (the tools identify a project by path today).
  Acceptance:
  - Every session, worktree and project row type carries a daemon id; a qualified ref round-trips unchanged.
  - The routed envelope round-trips through the codec; a frame without a known target is rejected.
  - `PROTOCOL_VERSION` is bumped and the handshake refusal names both versions.
  - `docs/daemon.md` and a contract under `specs/orchestrator/contracts/` describe the envelope and the project-identity rule.

## 2. feat: `micold-daemon orchestrator` — registry, daemon link, heartbeat, offline

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M1 — Local orchestrator (MVP)
- **Depends on**: 1 (#1723)
- **Covers**: FR-001, FR-002, FR-003, FR-010
- **Body**:
  New `micold-daemon orchestrator` mode that hosts no sessions. It accepts daemon links, keeps the registry (id, kind, name, protocol version, capabilities, health, last seen, tenant id) in a persistent store, heartbeats each link and marks a silent daemon offline. An older-protocol daemon is refused with both versions named and shown as needing an update.
  Acceptance:
  - The orchestrator starts, accepts a link and lists the daemon with its fields.
  - A link that stops answering heartbeats shows offline; its return shows online.
  - An older-protocol daemon is refused naming both versions; other daemons are unaffected.
  - Two links claiming one daemon id: the second is refused.
  - Restarting the orchestrator keeps the registry; no daemon loses a session.

## 3. feat: daemons dial the orchestrator (host directly; container, SSH and pod through the reverse forward)

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M1 — Local orchestrator (MVP)
- **Depends on**: 2 (#1724)
- **Covers**: FR-003, FR-010
- **Body**:
  Every daemon dials out and keeps one multiplexed link. The host daemon connects to the local orchestrator directly; container, SSH-host and pod daemons connect through a long-lived `ssh -R` reverse forward the app holds (the 806 install relay is only a precedent). The link reconnects on its own. Confirm in the plan that pods have an sshd for the reverse path.
  Acceptance:
  - Host, container, SSH-host and pod daemons each appear online in the registry.
  - Dropping the app's SSH connection shows those daemons offline; reconnecting brings them back without a restart.
  - Daemons keep running their sessions while the orchestrator is down and re-link when it returns.

## 4. feat: the desktop app talks only to the orchestrator (aggregated lists, routed ops and streams, starts it when absent)

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M1 — Local orchestrator (MVP)
- **Depends on**: 3 (#1725)
- **Covers**: FR-005, FR-006, FR-007, FR-010
- **Body**:
  The app opens one connection, to the orchestrator, and starts the local one when none runs (as it starts the host daemon). List requests return the union with a named reason for every daemon not asked; every other request and every stream (terminal output, structured events, progress) is routed to the owning daemon. Terminal and structured views, worktree and session operations, takeover and notifications keep working.
  Acceptance:
  - The existing client acceptance suites pass through the orchestrator (SC-002).
  - Routing adds at most 5 ms p95 per request and one frame of terminal-output delay (SC-003).
  - With the orchestrator absent, the app starts it; if it stops, sessions keep running and the app settles on the authoritative state.
  - A list names every offline daemon and why; a request in flight when a link drops reports an unknown outcome and the app re-reads.

## 5. feat: MCP tools across daemons (per-daemon opt-out, confirmations routed to the app)

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M1 — Local orchestrator (MVP)
- **Depends on**: 4 (#1726)
- **Covers**: FR-008, FR-009
- **Body**:
  The MCP tools list, read, start and type into the sessions and worktrees of every daemon serving the caller's project, via the orchestrator; refs are daemon-qualified. The scope never widens beyond the project. Each daemon has a setting hiding its sessions from other daemons' agents. Confirmations for actions on another daemon's session reach the user in the app as today.
  Acceptance:
  - `list_sessions` from a host session returns the sessions of a host and a container daemon of one project, each naming its daemon (SC-001).
  - An offline daemon is named as offline in the answer, not omitted silently.
  - With the opt-out on, the daemon's sessions are not listed and an action on one is refused with the same `not_found` as a missing one.
  - `send_session_input` to a session on another daemon asks for confirmation in the app exactly as today.
  - A target outside the caller's project stays `not_found`.

## 6. feat: `[orchestrator]` config section and `config check` rules

- **Labels**: `flow:feature`, `effort:low`
- **Milestone**: M2 — Daemons anywhere, with mTLS
- **Depends on**: —
- **Covers**: FR-011
- **Body**:
  Extend `micold_core::daemon_config` (#1523) with `[orchestrator]`: `url`, `ca_file`, `cert_file`, `key_file`, `enrollment_token_file`; secrets by file path only. Add `micold-daemon config check` rules (URL scheme, cert and key given together, token file or cert, never both silently) and document them in `docs/user-guide/daemon-configuration.md`.
  Acceptance:
  - The section parses; unknown keys are refused as elsewhere in the file.
  - Each rule has a test naming file:line:col.
  - No secret content appears in any `Debug`/`Display`; the resolved view hides it.

## 7. feat: built-in CA, enrollment tokens and CSR, short-lived certs, renewal, revocation

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M2 — Daemons anywhere, with mTLS
- **Depends on**: 2 (#1724), 6 (#1728)
- **Covers**: FR-012, FR-013, FR-014
- **Body**:
  The orchestrator runs a CA behind an interface (an external CA can replace it), issues one-time expiring enrollment tokens (default 1 hour, revocable unused), signs CSRs and records each daemon. The key is generated on the daemon. Certificates default to 24 h and renew over the link without a restart; a revocation list by daemon id and serial refuses connect and renew. The daemon id is taken from the certificate only. The docs say how to back up the CA key.
  Plan step (buy vs build): evaluate an existing CA such as step-ca behind the CA interface before writing one; also check Teleport, headscale, NetBird and Nebula for enrollment and revocation. See spec.md, "Prior art".
  Acceptance:
  - A valid token yields a certificate and is spent; reuse and expiry are refused.
  - A certificate renews before expiry without a restart.
  - A revoked daemon is disconnected, cannot reconnect or renew, and shows as revoked.
  - The daemon pins the orchestrator CA and refuses another.
  - The identity a daemon claims after connecting is ignored; the certificate decides.

## 8. feat: daemon link over TLS WebSocket — reconnect, resumable streams

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M2 — Daemons anywhere, with mTLS
- **Depends on**: 3 (#1725), 7 (#1729)
- **Covers**: FR-014, FR-015
- **Body**:
  Carry the daemon link over a TLS WebSocket with mTLS, alongside the reverse-forward carrier of M1. Reconnect with backoff and resume streams by sequence number so a client that stayed connected to the orchestrator sees nothing lost or duplicated.
  Plan step (prior art): compare Piko, Towonel and selftunnel for the dial-out and reconnect pattern; none carries our routed envelope or resumable streams. See spec.md, "Prior art".
  Acceptance:
  - A daemon on another network enrolls and links over mTLS.
  - After a 60 s network drop no structured event or terminal output is lost or duplicated for a connected client (SC-005).
  - Reconnect backs off and gives up no session.
  - A client without a valid certificate cannot open a link.

## 9. feat: the app enrolls the daemons it manages (private key unreadable by sessions)

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M2 — Daemons anywhere, with mTLS
- **Depends on**: 8 (#1730)
- **Covers**: FR-016
- **Body**:
  The app renders `[orchestrator]` into each managed daemon's configuration (host, SSH, container, pod, per #1526–#1528), delivers the enrollment token, and keeps the daemon's private key readable by the daemon process only, never by its sessions (including sandbox and pod sessions).
  Acceptance:
  - A daemon enrolls from one token and one configuration file and no other step (SC-004).
  - An agent run in a container or pod session cannot read the daemon's private key.
  - Settings → Daemons shows each daemon's enrollment state and can revoke it.

## 10. feat: JSON-over-WebSocket API with a schema generated from the Rust types, localhost-only by default

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M3 — Network API, users, authorization
- **Depends on**: 4 (#1726)
- **Covers**: FR-017
- **Body**:
  A network API on the orchestrator: JSON over WebSocket (TLS), exposing the client protocol, with the schema (JSON Schema/TypeScript types) generated from the Rust protocol types and checked in CI against drift. It listens on localhost only unless configured.
  Acceptance:
  - The schema is generated by a `mise` task; CI fails when the checked-in output drifts.
  - The API listens on localhost by default; binding elsewhere needs explicit configuration and TLS.
  - A test client lists daemons and reads a structured session through it.

## 11. feat: user auth — OIDC plus a local owner account, short-lived tokens

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M3 — Network API, users, authorization
- **Depends on**: 10 (#1732)
- **Covers**: FR-018
- **Body**:
  Sign-in by OIDC or, for self-hosting, a local owner account created on first run (no default password). API tokens are short-lived and refreshable. Every stored record carries a tenant id.
  Acceptance:
  - The first sign-in sets up the owner account; no default credential exists.
  - An OIDC login yields a short-lived token; an expired token is refused.
  - Every record written carries a tenant id.

## 12. feat: roles owner/operator/viewer, per-daemon grants, audit log

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M3 — Network API, users, authorization
- **Depends on**: 11 (#1733), 5 (#1727)
- **Covers**: FR-019, FR-020
- **Body**:
  Authorize every request per user, daemon and action. Roles owner, operator, viewer (viewer by default for an added user), with per-daemon grants. Every action that changes a session or worktree (input, start, stop, create, delete, permission answer) is audited with actor, target and outcome, never the typed text. MCP actions are audited with the session as actor.
  Acceptance:
  - A viewer can read but every changing action is refused.
  - A grant on one daemon does not extend to another.
  - Each changing action writes an audit entry; typed text never appears in it.
  - The audit log is readable by the owner only.

## 13. feat: web client scaffold under `web/`, login, served by the orchestrator, stable-order list with per-row state (see #1718)

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M4 — Web client (structured view only)
- **Depends on**: 12 (#1734)
- **Covers**: FR-021, FR-022
- **Body**:
  A TypeScript application under `web/` (the plan picks the framework) that shares only the generated schema, served by the orchestrator, with sign-in and the project/worktree/session list in a stable order with a per-row state indicator (see #1718: rows must not jump between groups). No terminal emulator in the bundle.
  Acceptance:
  - A browser signs in and lists projects, worktrees and sessions of every daemon.
  - A state change updates the row's indicator but not its position.
  - A CI check fails if the bundle contains a terminal emulator (SC-007).
  - A terminal session is shown with its state and a note that it opens only in the desktop app.

## 14. feat: web client — live structured transcript, read-only

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M4 — Web client (structured view only)
- **Depends on**: 13 (#1735)
- **Covers**: FR-023
- **Body**:
  Render a structured session's live transcript from `StructuredEvents` with the desktop app's content: messages, tool calls, subagent threads and usage.
  Acceptance:
  - Opening a session shows the same transcript as the desktop app.
  - New events append live; reconnecting resumes without gaps or duplicates.
  - A viewer sees no composer or changing action.

## 15. feat: web client — composer: send, paste (see #1721), permission prompts, slash commands, model/effort/permission pickers

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M4 — Web client (structured view only)
- **Depends on**: 14 (#1736)
- **Covers**: FR-024
- **Body**:
  The composer: send, paste (see #1721), answering permission prompts, slash commands, and the model, effort and permission-mode pickers, matching the desktop structured view.
  Acceptance:
  - A message sent from the browser reaches the session and shows in the desktop app.
  - Pasting text puts it at the cursor.
  - A permission prompt answered in the browser shows as answered in every client.
  - Slash commands and the three pickers work as in the desktop app.
  - Input from the browser and the desktop at once is serialized by the daemon.

## 16. feat: web client — worktree and structured-session lifecycle; terminal sessions listed but not openable

- **Labels**: `flow:feature`, `effort:low`
- **Milestone**: M4 — Web client (structured view only)
- **Depends on**: 13 (#1735)
- **Covers**: FR-025
- **Body**:
  Create, start and stop worktrees and structured sessions from the browser; terminal sessions are listed with state and are not openable.
  Acceptance:
  - A structured session can be started in a worktree from a browser with no desktop app running (SC-006).
  - Create/stop/delete follow the role rules and are audited.
  - Selecting a terminal session opens no terminal.

## 17. feat: web client — attention notifications while the tab is open

- **Labels**: `flow:feature`, `effort:low`
- **Milestone**: M4 — Web client (structured view only)
- **Depends on**: 14 (#1736)
- **Covers**: FR-026
- **Body**:
  Show "needs you" attention (permission prompts, finished sessions) as in-page and browser notifications while the tab is open. Web Push is out of scope.
  Acceptance:
  - A permission prompt raises a notification in an open tab.
  - No notification is sent to a closed tab.
  - Answering elsewhere clears it.

## 18. feat: orchestrator image with compose and Helm examples, TLS, persistent volume

- **Labels**: `flow:feature`, `effort:high`
- **Milestone**: M5 — Cloud deployment
- **Depends on**: 8 (#1730), 12 (#1734)
- **Covers**: FR-027
- **Body**:
  A container image of the orchestrator, documented compose and Helm examples, TLS, and one persistent volume for the registry, CA, users and audit log. Restart keeps all of it.
  Acceptance:
  - The documented example deploys and a daemon enrolls over the internet.
  - Restarting the orchestrator loses no registry, user, enrollment or audit data; daemons and clients reconnect on their own.
  - Docs cover CA key backup and recovery.

## 19. feat: the desktop app connects to a remote orchestrator after sign-in

- **Labels**: `flow:feature`, `effort:low`
- **Milestone**: M5 — Cloud deployment
- **Depends on**: 11 (#1733)
- **Covers**: FR-028
- **Body**:
  Settings lets the user choose a remote orchestrator and sign in; the app then uses it instead of the local one. Cloud use is opt-in; the local default is unchanged.
  Acceptance:
  - After sign-in the app lists the remote daemons and operates them.
  - Signing out returns to the local orchestrator.
  - The default install never contacts a remote orchestrator.

## 20. feat: rate limits, bandwidth caps, health and metrics

- **Labels**: `flow:feature`, `effort:low`
- **Milestone**: M5 — Cloud deployment
- **Depends on**: 18 (#1740)
- **Covers**: FR-029
- **Body**:
  Per-client rate limits, per-daemon bandwidth caps, and health and metrics endpoints.
  Acceptance:
  - A client or daemon over its limit is slowed or refused with a reason; others are unaffected.
  - Health and metrics endpoints report link counts, bytes and errors.
