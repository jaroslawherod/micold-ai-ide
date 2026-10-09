# Feature Specification: Codex CLI and OpenCode as Session Providers

**Feature Branch**: `feat/488-codex-opencode-providers`

**Created**: 2026-10-09

**Status**: Draft

**Input**: User description: "Implement GitHub issue #488: Add Codex CLI and OpenCode as session providers. Sessions can run `claude`, `copilot` or `pi`. Users of other terminal coding agents, such as OpenAI Codex CLI and OpenCode, cannot pick them. Add both, following the existing provider pattern. For each: detection of the installed binary, the start command, resume of a previous conversation where the CLI supports it, session naming from the first turn, and activity detection where the CLI exposes enough signal. Sandboxed runtime: install or mount the CLI in the image, and share its sign-in the same way as for the existing providers. Document each provider in the user guide. Acceptance: each new provider can be selected when starting a session and a session remembers it; a provider whose CLI is not installed is shown as unavailable with the reason; activity shows `Unknown` rather than a wrong state when the CLI gives no signal."

## Terms

- **Provider**: an AI CLI that Micold runs in a session. Today Claude Code, GitHub Copilot and Pi; this feature adds **Codex** (OpenAI Codex CLI, command `codex`) and **OpenCode** (command `opencode`).
- **Unavailable**: a provider whose command is not found on the `PATH` a session would be spawned with.
- **Activity**: the busy / idle / awaiting-input badge of a session; `Unknown` when no reliable signal exists.
- **Sandboxed runtime**: running a session's CLI inside the container image instead of on the host.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start a Codex or OpenCode session (Priority: P1)

A user with `codex` or `opencode` installed starts a session in a worktree and picks that CLI, as they would pick Claude Code, Copilot or Pi. The CLI opens in the embedded terminal in the worktree. The session remembers its provider, so after an app restart it is still a Codex or OpenCode session.

**Why this priority**: it is the whole point of the issue; everything else refines it.

**Independent Test**: with a stand-in `codex` (and `opencode`) executable on the `PATH`, start a session choosing it, and check that the stand-in ran in the worktree; restart the daemon and check the session still reports that provider.

**Acceptance Scenarios**:

1. **Given** `codex` is installed, **When** the user starts a session and chooses Codex, **Then** a terminal opens running `codex` in the worktree and the session row is labelled `codex`.
2. **Given** `opencode` is installed, **When** the user chooses OpenCode, **Then** the same holds with `opencode`.
3. **Given** a running Codex session, **When** the app and daemon restart, **Then** the session is listed with provider Codex.
4. **Given** the Settings default provider is set to Codex or OpenCode, **When** a session is started without an override, **Then** it runs that CLI.
5. **Given** the MCP `create_session` tool is called with `ai_cli` set to `codex` or `opencode`, **Then** it starts that provider like any other.

---

### User Story 2 - See why a provider cannot be used (Priority: P1)

A user without `codex` or `opencode` installed opens the provider choice. The provider is shown as unavailable, with the reason (the command was not found), and cannot be started. Installing the CLI makes it available without restarting the app.

**Why this priority**: an acceptance criterion; it prevents a silent failed launch.

**Independent Test**: with neither executable on the `PATH`, list providers and check both are unavailable with a reason; add one and list again.

**Acceptance Scenarios**:

1. **Given** `opencode` is not on the `PATH`, **When** the user lists providers, **Then** OpenCode is shown as unavailable, naming the missing command.
2. **Given** an unavailable provider, **When** the user tries to start a session with it (including through MCP), **Then** the start is refused with the same reason and no terminal is created.
3. **Given** the CLI is installed afterwards, **Then** the provider becomes available on the next listing.

---

### User Story 3 - Resume and name a session (Priority: P2)

A user closes the app during a Codex or OpenCode conversation and later restarts the session. Where the CLI supports resuming, the previous conversation continues rather than a fresh one starting. A session that has had a first turn is named from it, as for the existing providers, instead of staying "untitled".

**Why this priority**: valuable, but the session works without it, and each depends on what the CLI records on disk.

**Independent Test**: with a stand-in CLI that records a conversation in the CLI's own store layout, restart the session and check the resume launch; check the name derived from the first turn.

**Acceptance Scenarios**:

1. **Given** a session whose CLI recorded a conversation, **When** the session is started again, **Then** the CLI is launched to resume that conversation.
2. **Given** a session with no recorded conversation, **When** it is started again, **Then** it starts fresh.
3. **Given** a conversation with a first user turn, **When** the session has no name, **Then** it is labelled from that turn.
4. **Given** a CLI that offers no resume, **Then** the user guide says so and a restarted session starts fresh without an error.

---

### User Story 4 - Activity that is honest (Priority: P2)

A user watching the sidebar sees busy / idle / awaiting-input for Codex or OpenCode sessions only where the CLI gives a reliable signal. Where it gives none, the badge reads `Unknown`, never a guessed state.

**Why this priority**: an acceptance criterion, but `Unknown` is the safe default, so shipping with it is already correct.

**Independent Test**: run a session of a CLI with no signal source and check the badge stays `Unknown` through output and silence.

**Acceptance Scenarios**:

1. **Given** a CLI for which no activity source is implemented, **When** it prints and goes quiet, **Then** the badge stays `Unknown`.
2. **Given** a CLI with an implemented activity source, **When** it works then waits, **Then** the badge follows busy then idle.

---

### User Story 5 - Use them in the sandbox (Priority: P3)

A user who runs sessions in the sandboxed runtime finds `codex` and `opencode` in the image, and the host sign-in of each is shared into the container the way it is for the existing providers, so there is no second login.

**Why this priority**: sandbox users only; host sessions are complete without it.

**Independent Test**: build the image, run each CLI's version command inside it, and check the sign-in files are visible in the container and read-only or writable exactly as for the existing providers.

**Acceptance Scenarios**:

1. **Given** the sandbox image, **When** a Codex or OpenCode session starts in the sandbox, **Then** the CLI is found and starts.
2. **Given** the user signed in on the host, **Then** the sandboxed session is signed in without a second login.
3. **Given** a host with no sign-in files, **Then** the sandbox still starts and the CLI shows its own login prompt.

---

### User Story 6 - Documentation (Priority: P3)

The user guide documents each new provider: how it is detected, what a session does on restart, how it is named, what its activity badge shows, tool-server support, and sandbox sign-in.

**Independent Test**: grep the user guide for each provider and each topic.

**Acceptance Scenarios**:

1. **Given** the user guide, **Then** every place that lists the providers lists Codex and OpenCode, and each has a section stating what is and is not supported.

### Edge Cases

- **Neither CLI installed**: both unavailable with reasons; existing providers unaffected.
- **CLI installed under a version manager** (npm global under mise or nvm): found the way `pi` and `copilot` are, using the session's own `PATH`.
- **Store missing or unreadable**: no recorded conversation, so the session starts fresh; never an error, never fails a project open.
- **Many concurrent sessions** of one CLI in one worktree (Principle II): each keeps its own identity and resumes its own conversation, never another's.
- **CLI mints its own conversation ids** and cannot be given the app's session id: the app still resumes the right conversation (see FR-006).
- **Provider removed after sessions exist**: the session is listed with its remembered provider, shown unavailable, and starting it fails with the reason.
- **Platforms** (Principle VI): macOS, Linux and Windows differences in the `PATH`, executable names (`codex.cmd`, `opencode.exe`) and store locations are handled as for the existing providers.
- **Sign-in files differ per platform** (for instance a keychain instead of a file): the sandbox shares only what exists and never fails because a file is absent.
- **A first prompt typed before the CLI is ready**, or into a folder-trust question: handled as for the existing providers, or the provider says it cannot take an injected first prompt.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The set of providers MUST include Codex and OpenCode, each selectable wherever a provider is chosen: the new-session choice, the Settings default and the MCP `create_session` `ai_cli` parameter.
- **FR-002**: A session MUST remember its provider across app and daemon restarts, and across upgrades from sessions recorded before this feature, which keep their provider unchanged.
- **FR-003**: The app MUST report each provider's availability by looking for its command on the `PATH` a session would be spawned with, and MUST show an unavailable provider with the reason, not hide it. Availability MUST be re-evaluated on each listing, never cached for the app's lifetime.
- **FR-004**: Starting a session with an unavailable provider MUST fail before any terminal is created, with the reason, from the app and from MCP.
- **FR-005**: A fresh session MUST launch the provider's command in the session's worktree; a session label MUST show the command name (`codex`, `opencode`).
- **FR-006**: Where the CLI can resume a conversation, a restarted session MUST resume the conversation that session had, found from the CLI's own store, and MUST NOT resume another session's conversation. Where it cannot, the session MUST start fresh and the user guide MUST say so.
- **FR-007**: Reading a provider's store MUST be best-effort: a missing, unreadable or unrecognised record yields no conversation, never an error.
- **FR-008**: A session without a name MUST be labelled from its first user turn where the CLI records one, reading a bounded prefix only.
- **FR-009**: Activity MUST be reported only from a reliable CLI-provided signal; for a provider with none the badge MUST read `Unknown` and MUST NOT show busy, idle or awaiting-input.
- **FR-010**: A session's close or removal MUST be recorded so discovery does not offer it again, as for the existing providers.
- **FR-011**: The provider's tool-server (MCP) support MUST be declared: bound to the app's tool server where the CLI supports it, otherwise the session starts unbound and the reason is logged.
- **FR-012**: The provider MUST declare how a first prompt is injected safely (readiness signal and folder-trust behaviour), so `create_session` with a `prompt` never types into a trust question.
- **FR-013**: The sandbox image MUST contain both CLIs, and a sandboxed session MUST share each CLI's sign-in from the host the way existing providers do, writing nothing the existing providers would not write.
- **FR-014**: The user guide MUST list Codex and OpenCode wherever it lists providers and document, per provider: detection, restart behaviour, naming, activity, tool-server support and sandbox sign-in.
- **FR-015**: Existing providers' behaviour MUST NOT change; adding a provider touches the provider seam and its consumers, not a per-CLI conditional elsewhere.
- **FR-016**: The wire protocol MUST stay compatible: an older client meeting a session of a new provider MUST NOT crash, and any version bump follows the repository's protocol rules. [NEEDS CLARIFICATION: does the provider list in the wire schema break old clients, i.e. is a protocol version bump acceptable?]

### Key Entities

- **Provider**: identity, user-facing name, command, availability, launch shape (fresh / resume), store location, naming source, activity source, tool-server support, first-prompt readiness, folder trust.
- **Session**: unchanged, except its remembered provider can now be Codex or OpenCode.
- **Conversation record**: the CLI's own on-disk record the app reads to detect, resume and name.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can start a Codex session and an OpenCode session in a worktree in the same steps as for Claude Code, and each is running its CLI with its first output shown in the terminal within the same budget as Claude Code.
- **SC-002**: 100% of sessions started with a new provider still report that provider after a restart.
- **SC-003**: With a CLI missing, 100% of start attempts through the app and MCP are refused with a reason naming the missing command, and no terminal is created.
- **SC-004**: For a provider with no activity signal, the badge is `Unknown` in 100% of observed samples.
- **SC-005**: In a fresh sandbox image, both CLIs report a version, and a host-signed-in user needs no second login for either.
- **SC-006**: Every provider list in the user guide includes both new providers.
- **SC-007**: Existing providers' tests pass unchanged.

## Assumptions

- **Open (needs verification in plan, against the shipped CLIs)**: how each CLI resumes (Codex: `codex resume <id>` from `~/.codex/sessions`; OpenCode: `--session <id>` or `--continue` from its data directory), whether either accepts an externally chosen conversation id, what each records for naming, and which signal could drive activity. These shape FR-006, FR-008 and FR-009 but not the user-visible contract: the fallback for each is fresh start, no label, `Unknown`.
- **Sign-in locations** (to verify): Codex under `~/.codex` (`CODEX_HOME`), OpenCode under its data directory; shared as the existing providers' are.
- The existing provider seam is sufficient; no new provider capability is invented unless the plan shows a gap.
- A first-turn name is shown only if the CLI's record carries the turn in a form readable without running the CLI.

## Out of Scope

- Other CLIs (Gemini CLI, Aider and the like).
- Installing the CLIs on the host for the user; only the sandbox image carries them.
- Attaching a provider's existing worktrees, which feature 582 covers, including discovery of a new provider's store.
- Changing how Activity states are derived; only the source per provider.
- Signing in on the user's behalf.
