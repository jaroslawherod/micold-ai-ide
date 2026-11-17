# Feature Specification: React Native (Expo) web client scaffold, Material 3 theme, sign-in, sidebar and overview

**Feature Branch**: `claude/project-thread-hzt36f`

**Created**: 2026-11-16

**Status**: Closed 2026-11-17 — shipped in PRs #2009, #2013, #2015, #2018, #2020

**Input**: GitHub issue #1735 "feat: React Native (Expo) web client scaffold, Material 3 theme from the tokens, sign-in, sidebar and overview" (part of #1722, milestone M4a; covers FR-021R, FR-022, FR-034, FR-035, FR-037R of `specs/1722-rn-client-k8s/spec.md`). Added requirements from the coordinator: build against a **mocked** orchestrator API behind a TypeScript interface; internationalization from the start; use the tokens module of #1950. Labels: effort:high, flow:feature.

## Terms

- **Client**: the Expo (managed, Expo Router) TypeScript app in `apps/client/`, built for the web only with `react-native-web`.
- **API boundary**: the TypeScript interface `OrchestratorApi` (in `packages/protocol/`) through which the Client reads and changes everything it shows. Two implementations satisfy it: the **mock** (this feature) and the **real socket client** (later, when #1732 lands).
- **Location row**, **stable order**, **card state**: as in `specs/1718-sidebar-stable-order/spec.md#Terms`.
- **Overview card**: one location's card in the Overview grid, as in the iced app (`ui/overview.rs`).
- **Daemon**: a registered host the orchestrator lists; every project, worktree and session belongs to exactly one.

## What is mocked, and what is real

The orchestrator's WebSocket API (#1732) and its sign-in endpoints (#1733) are not built when this feature ships: #1732 waits on #1726, and neither has a spec or code in the repository (checked 2026-11-16). Therefore:

| Piece | In this feature | Later |
|---|---|---|
| `OrchestratorApi` interface and its TypeScript types | **Real**, hand-written here as a stand-in for the types #1732 generates from the Rust definitions (constitution V requires generated types; this stand-in is replaced, not kept) | #1732 generates them; the interface is aligned and the stand-in deleted |
| Lists of daemons, projects, worktrees, sessions and their states, and live state changes | **Mocked**: a deterministic in-memory fixture with a scriptable state-change feed | Real socket client of #1732: socket, reconnect with backoff, resume by sequence |
| Sign-in, provider list, session (access token), sign-out | **Mocked**: a mock auth service offering fixture providers (GitHub and Google configured, a third unconfigured) and accepting a fixed test identity | Real authorization code + PKCE flow of #1733 with the `HttpOnly` refresh cookie |
| Reconnect behaviour | **Real** logic over a transport interface, tested against a fake transport that drops and restores | Same logic over the real WebSocket |
| Design tokens, theme, layout, i18n, routing, CI | **Real** | |

The mock is selected by one build-time switch and is never reachable in a production export (SC-006). Wherever the Client shows data from the mock, nothing in the UI claims it is live.

## Clarifications

### Session 2026-11-16

- Q: Which locales ship in v1? → A: English (source, default) plus Polish as the one real second locale, which proves runtime switching, plural rules and persistence; the pseudo-locale is for tests only; more languages are added by translators later. _(agent-resolved: specs/1735-rn-client-scaffold/spec.md#User Story 5; the issue requires i18n from the start and FR-015 needs a real switch target; a translation is not otherwise in scope, so Polish strings are kept minimal to the screens of this feature)_
- Q: Does the mock ship in the deployed bundle? → A: No. The mock is in development, test and CI builds only; a production export has none (SC-006). A separate, explicit demo flag may include it for a labelled demo build; it is off by default, and the production bundle check runs on builds without it. _(agent-resolved: specs/1735-rn-client-scaffold/spec.md#What is mocked, and what is real)_ in the deployed static bundle behind a "demo" switch, or only in development, test and CI builds? The proposal is dev, test and CI only (SC-006).]

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Sign in with a configured provider (Priority: P1)

A person opens the Client, sees only the providers the owner configured, signs in with one, and lands on the Overview. Without a session every other route sends them to sign-in.

**Why this priority**: nothing else is reachable without it, and FR-037R sets the token rule everything later relies on.

**Independent Test**: with the mock auth service configured for GitHub only, load the app signed out: the sign-in page shows one button; pressing it ends on the Overview; reloading the page returns to sign-in because the access token lived in memory only.

**Acceptance Scenarios**:

1. **Given** the mock reports GitHub and Google as configured and a third provider as not, **When** the sign-in page loads, **Then** exactly two buttons show, named by the provider.
2. **Given** no provider is configured, **When** the sign-in page loads, **Then** it shows a message that sign-in is not set up, with no button, and nothing crashes.
3. **Given** a signed-out visitor, **When** they open any other route directly, **Then** they land on sign-in and, after signing in, return to the route they asked for.
4. **Given** a signed-in person, **When** they reload the tab, **Then** they are signed out and nothing token-shaped is in `localStorage` or `sessionStorage` (FR-037R: access token in memory only).
5. **Given** the provider refuses or the sign-in is cancelled, **When** the page returns, **Then** a readable error shows with a retry, and no session exists.

---

### User Story 2 - The sidebar lists every daemon's projects, worktrees and sessions in a stable order (Priority: P1)

A signed-in person sees, in a sidebar, the projects, worktrees and sessions of every daemon, each row carrying its state. When a session changes state the row's indicator changes; the row does not move.

**Why this priority**: this is the acceptance criterion of the issue and the reason for #1718.

**Independent Test**: load the fixture with two daemons; record the row identities and indices; drive every state change through the mock feed; the order is identical and the indicator of the changed row differs.

**Acceptance Scenarios**:

1. **Given** the fixture has two daemons with projects, worktrees and sessions, **When** the sidebar loads, **Then** every project, worktree and session of both daemons is listed, each location under its daemon and project.
2. **Given** a worktree's session goes working, then waiting, then idle, **When** each change arrives, **Then** that row's state indicator changes (glyph or text, not colour alone) and its position is unchanged.
3. **Given** a daemon is offline in the fixture, **When** the sidebar renders, **Then** its rows stay listed, marked offline, and no other daemon's rows move.
4. **Given** the list is empty (no daemons, or a daemon with no projects), **When** the sidebar renders, **Then** an empty state with a message shows.
5. **Given** two state changes arrive in one batch, **When** applied, **Then** the final state of each row is shown and order is unchanged.
6. **Given** the fixture holds structured and terminal sessions, **When** the sidebar renders, **Then** terminal sessions are listed with their state exactly like structured ones, the kind shown by a glyph or text.

---

### User Story 3 - The Overview grid matches today's layout and stacks on narrow screens (Priority: P1)

The Overview shows one card per location with its state, in the desktop app's arrangement on wide screens; below 600 px the sidebar and the grid stack into one column.

**Why this priority**: FR-035 and the visual pass.

**Independent Test**: render at 1280, 800 and 390 px, light and dark; assert the column count and the stacking at the 600 px breakpoint, and capture the six screenshots for the visual pass.

**Acceptance Scenarios**:

1. **Given** a viewport of 1280 px, **When** the Overview renders, **Then** the sidebar sits beside a multi-column card grid in the iced app's arrangement.
2. **Given** a viewport of 599 px or narrower, **When** the Overview renders, **Then** the layout is one column with the sidebar stacked above the grid; at 600 px it is the wide layout.
3. **Given** the card order, **When** a state changes, **Then** the card's state shows and its position is unchanged (same rule as the sidebar).
4. **Given** the six screenshots (1280, 800, 390 px; light and dark), **When** compared with the iced app's in the visual pass, **Then** each passes the checks of the pass's record.

---

### User Story 4 - The theme comes from the tokens and follows light and dark (Priority: P2)

A person sees Material 3 colours, type, shape and elevation identical to the desktop app, in light or dark, following the system setting unless they choose.

**Why this priority**: FR-034; looks only, no new capability.

**Independent Test**: unit test that every Paper `MD3Theme` colour equals the matching `tokens.generated.ts` value in both schemes; the generated file is imported, never copied.

**Acceptance Scenarios**:

1. **Given** the generated tokens, **When** the theme is built, **Then** each Paper colour role equals the token of the same role, light and dark.
2. **Given** the system prefers dark, **When** the Client loads, **Then** it renders dark; switching the system setting while open switches the theme without a reload.
3. **Given** the text uses Roboto and code uses Fira Mono, **When** the page loads, **Then** the fonts are the bundled ones, not a network fetch from a third party.

---

### User Story 5 - Every string is translatable and the locale is switchable (Priority: P1)

A person changes the interface language from a control in the Client and every user-visible string changes, with no reload and no string left in the source language.

**Why this priority**: the coordinator requires i18n from the start; retrofitting a catalog later touches every component.

**Independent Test**: switch to the test pseudo-locale: every visible string on sign-in, sidebar and Overview is transformed; a lint check fails when a component contains a user-visible literal.

**Acceptance Scenarios**:

1. **Given** the message catalog, **When** the locale is switched, **Then** all visible text on every screen of this feature changes, including empty states, errors, accessibility labels and state names.
2. **Given** a locale chosen, **When** the person reloads, **Then** the choice persists (a preference, not a token) and, with none chosen, the browser's language is used, falling back to English.
3. **Given** a message missing in the chosen locale, **When** rendered, **Then** the English text shows and the gap is reported in development.
4. **Given** counts, dates and relative times, **When** rendered, **Then** they use the locale's plural rules and formats.
5. **Given** a locale of right-to-left script is added later, **When** selected, **Then** layout mirrors (the Client uses logical start/end, never left/right, for spacing).

---

### User Story 6 - A developer can build, test and gate the Client (Priority: P1)

A developer runs `mise run client-test` and `mise run client-web`; CI runs lint, typecheck, Jest, the web export, a Playwright smoke test and the bundle check.

**Why this priority**: every later client issue (#1736 onward) builds on this loop.

**Independent Test**: on a clean checkout, the two tasks and the CI job succeed; adding a terminal emulator dependency or a literal UI string makes CI fail.

**Acceptance Scenarios**:

1. **Given** a clean checkout, **When** `mise run client-test` runs, **Then** lint, typecheck and Jest pass.
2. **Given** the same, **When** `mise run client-web` runs, **Then** a static web bundle is produced.
3. **Given** the bundle, **When** the bundle check scans it and the dependency list, **Then** it finds no terminal emulator (SC-007) and fails naming the match otherwise.
4. **Given** the exported bundle, **When** the Playwright smoke test runs, **Then** it signs in through the mock, sees the sidebar and the Overview, and a state change moves no row.
5. **Given** `tokens.generated.ts` drifts from `micold-core`, **When** CI runs, **Then** the existing drift check (#1950) still fails first.

---

### User Story 7 - The spike: a long transcript and a diff render with Paper on the web (Priority: P1)

Before the rest is built, a spike renders a long transcript and a diff with Paper on the web and records what works and what does not, so a gap shows early.

**Why this priority**: it de-risks #1736 and the diff view (M6); it ships no user feature. It is P1 because it must precede the sidebar and Overview work (ordering is fixed in FR-020, not by priority).

**Independent Test**: the spike page renders fixed fixtures (a 5,000-entry transcript, a 2,000-line diff) in the web export; the recorded result in `specs/1735-rn-client-scaffold/spike.md` states frame cost, scroll behaviour, selection and copy, and the gaps found.

**Acceptance Scenarios**:

1. **Given** the fixtures, **When** the spike page is exported and opened in Chromium, **Then** `spike.md` records measured numbers (render time, scroll smoothness, memory) and a verdict per surface.
2. **Given** a gap, **When** recorded, **Then** it names the follow-up issue or the decision (for example virtualization with FlashList, owned by #1736).
3. **Given** the spike is done, **When** the scaffold ships, **Then** the spike page is not routed in a production export.

---

### Edge Cases

- **Empty input**: no daemons, a daemon with no projects, a project with no worktrees, a worktree with no session: each shows an empty state, no crash.
- **Failure**: the mock feed or the transport drops: the Client shows a connection banner (reconnecting, with attempt count, then offline) and keeps the last list visible, marked stale; on return it resumes and the order is unchanged.
- **Concurrency (Principle II)**: sessions of two daemons change state at once, or one daemon's rows arrive while another's are being removed: rows are keyed by daemon, project and location identity, never by index, so a change to one never re-keys another; the final list is order-independent of arrival.
- **Browser differences (Principle VI)**: the smoke and visual tests run on Chromium, Firefox and WebKit (Safari); a missing `Intl` feature, `matchMedia` for colour scheme, or font-loading API degrades to English, light and system fonts rather than failing.
- **Viewport**: exactly 600 px, below 320 px, browser zoom to 200 percent, and resize while open all keep content reachable without horizontal scroll.
- **Long names**: a very long project, worktree or session name truncates with its full text available (title or accessible label), without moving the state indicator.
- **Auth**: an expired or missing session mid-use returns to sign-in without losing the open route; two tabs signed in separately never share a token.
- **Locale**: a locale code the catalog lacks falls back to the nearest language, then English; strings 40 percent longer than English (pseudo-locale) neither overflow nor clip the indicator.
- **Reduced motion**: with `prefers-reduced-motion` the state indicator changes without animation.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001** (FR-021R): The Client MUST be one Expo (managed workflow, Expo Router) TypeScript (`strict`) codebase in `apps/client/`, with `react-native-web`, producing only a static web export, in a pnpm workspace that includes `packages/protocol/`.
- **FR-002**: Node and pnpm versions MUST be pinned in `mise.toml`, and `mise run client-test` and `mise run client-web` MUST exist; `client-test` MUST run lint, typecheck and Jest, `client-web` the web export.
- **FR-003** (FR-034): The Client's Material 3 theme (react-native-paper v5 `MD3Theme`, light and dark) MUST be built from `apps/client/src/theme/tokens.generated.ts`, which the Client MUST import and MUST NOT copy or edit; Roboto and Fira Mono MUST be bundled with the Client.
- **FR-004**: The Client MUST read and change all data only through the `OrchestratorApi` interface; no component, store or route MAY import the mock or a socket directly.
- **FR-005**: This feature MUST supply a mock implementation of `OrchestratorApi` and of the auth service, per the *What is mocked* table, selectable only by a build-time switch, absent from a production export (SC-006).
- **FR-006** (FR-037R): The access token MUST be held in memory only and MUST NOT be written to `localStorage`, `sessionStorage`, `IndexedDB` or a non-`HttpOnly` cookie. The refresh-cookie half of FR-037R is the orchestrator's and the real auth client's (#1733) and is out of scope until then; the interface MUST NOT expose a refresh token to script.
- **FR-007**: The sign-in page MUST list only providers the auth service reports as configured, show a "not set up" state when there are none, handle refusal and cancellation, and return the person to the route they asked for.
- **FR-008**: Routes other than sign-in MUST require a session; with none they MUST redirect to sign-in.
- **FR-009**: The socket-side client logic (connection state, reconnect with backoff and jitter, resume by sequence, request/response correlation) MUST be implemented over a transport interface in `packages/protocol/` and be tested with a fake transport; it MUST surface a connection state the UI shows (connected, reconnecting with attempt, offline).
- **FR-010** (FR-022): The sidebar MUST list the projects, worktrees and sessions of every daemon the API reports, grouped by daemon and project, each row keyed by identity and carrying its state.
- **FR-011**: Rows MUST keep the stable order of `specs/1718-sidebar-stable-order/spec.md`; a state change MUST update the row's indicator and MUST NOT change its position, in the sidebar and in the Overview. The state indicator MUST differ by glyph or text, not colour alone.
- **FR-012** (FR-035): The Overview MUST render one card per location in the iced app's arrangement on wide screens, and the layout MUST stack into one column below 600 px.
- **FR-013**: An offline or stale daemon MUST stay listed, marked as such, with its last known rows.
- **FR-014**: Every user-visible string, including accessibility labels, empty states, errors, state names and the title of the page, MUST come from a message catalog (ICU-style messages with plural and format support); a lint rule MUST fail CI on a user-visible literal in a component.
- **FR-015**: The catalog MUST ship English and Polish (plus a test-only pseudo-locale). The locale MUST be switchable at runtime from a control in the Client without a reload, persisted as a preference, defaulting to the browser's language and falling back to English; a missing message MUST fall back to English.
- **FR-016**: The Client MUST use logical (start/end) layout properties so a right-to-left locale mirrors without code change.
- **FR-017**: The Client MUST NOT include a terminal emulator (FR-045); CI MUST fail when the bundle or the dependency list contains one (SC-007 of `specs/orchestrator/`).
- **FR-018**: CI MUST run lint, typecheck, Jest, the web export, a Playwright smoke test (in Chromium, Firefox and WebKit) of sign-in, sidebar, Overview and the order-stable state change, and the bundle check, on every change to `apps/client/` or `packages/protocol/`.
- **FR-019**: The Client MUST meet the accessibility baseline of the iced app's pass: keyboard-reachable sign-in, sidebar and locale control, visible focus, names on every control, and contrast of the theme's roles.
- **FR-020**: A spike MUST come first and be recorded in `specs/1735-rn-client-scaffold/spike.md` (long transcript and diff with Paper on the web, measured), before the sidebar and Overview are built; its page MUST NOT be reachable in a production export.
- **FR-021**: Screenshots at 1280, 800 and 390 px in light and dark MUST be captured by the Playwright run (in Chromium, the reference browser for the visual pass) and pass the visual pass against the iced app's.

### Key Entities

- **Daemon**: id, display name, connection status (online, offline, needs-update).
- **Project**: id, name, owning daemon.
- **Worktree (location)**: id, name, branch, owning project; the Default location of a project is one of them.
- **Session**: id, name, owning location, kind (structured, terminal), state (working, waiting, idle, error, stopped).
- **AuthProvider**: id, display name, configured (boolean).
- **ConnectionState**: connected, reconnecting (attempt), offline.
- **Message catalog**: locale, message id, message.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the fixture of 2 daemons, 6 projects, 30 worktrees and 60 sessions, every item appears in the sidebar, and across 100 scripted state changes the order of rows is identical to the initial order in 100 percent of checks.
- **SC-002**: A state change reaches the visible indicator within 100 ms of the mock's feed delivering it (measured in the automated tests of FR-018), and no row index changes.
- **SC-003**: The Overview shows one column at every width below 600 px and the wide layout at 600 px and above, at all three tested widths in light and dark (6 screenshots), all passing the visual pass.
- **SC-004**: 100 percent of user-visible strings on the screens of this feature come from the catalog: the lint check reports 0 literals, and the pseudo-locale run shows 0 untransformed strings.
- **SC-005**: A clean checkout runs the client test and web export tasks of FR-002 to success, and the CI client job completes in under 10 minutes.
- **SC-006**: A production export contains 0 references to the mock modules (the bundle check of FR-018) and 0 token-shaped values in web storage after sign-in (test).
- **SC-007**: The bundle check finds 0 terminal emulator packages in the export and the dependency list; adding one makes it fail.
- **SC-008**: The spike records, for a 5,000-entry transcript and a 2,000-line diff, render time and scroll behaviour with a verdict, before the sidebar work starts.
- **SC-009**: Diff coverage of the guarded client code is at least 80 percent, the repository's merge gate.

## Assumptions

- #1949 (constitution 2.0.0) and #1950 (`apps/client/src/theme/tokens.generated.ts` and its drift check) are merged; `apps/client/` holds only that tokens file today, so this feature creates the Expo app around it.
- #1732 and #1733 are not available; the mock stands in until the real client replaces it (a follow-up task, not this feature's).
- The state names and stable order come from #1718's rules; the orchestrator is expected to send them computed (parity matrix note), so the Client does not port `sidebar_groups.rs` beyond keeping the received order.
- English is the source locale; the catalog uses a standard ICU-compatible library chosen in the plan.
- Chromium, Firefox and WebKit are the supported browsers (Principle VI); native targets are out of scope.
- Transcript, composer, session start/stop, worktree create, attention, quick switcher and settings are later issues (#1736 onward).

## Out of scope

- The real WebSocket API, generated protocol types and the real auth flow (#1732, #1733), the HttpOnly refresh cookie and its server side.
- Serving the bundle from the orchestrator or the Kubernetes web image (FR-021R's serving half, #1954 and M5).
- Stopping a session (and starting one) is deferred to #1736 onward.
- Transcript, composer, paste, permission prompts, attention list, quick switcher, command palette, users and invites, daemon management.
- Native (iOS, Android) builds, and any language beyond English and Polish.
- Retiring the iced app.
