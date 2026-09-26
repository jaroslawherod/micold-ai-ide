# Feature Specification: Run a session on the Pi coding agent

**Feature Branch**: `feat/support-of-pi-agent`

**Created**: 2026-09-03

**Status**: Closed 2026-09-14 — implemented and shipped in PR #300 (merged 2026-09-13); all 61 tasks
in [tasks.md](./tasks.md) are done. Clarified 2026-09-03 and 2026-09-12 (10 questions, see
[Clarifications](#clarifications)). Quickstart §B, §C and §D are recorded in [evidence/](./evidence/);
§B and §D ran in the application on Xvfb with lavapipe, not on a real GPU. They found two defects,
both fixed and re-run in the application: a running Pi row showed Pi's terminal title instead of its
name (PR #320), and resuming a Pi session into an image without `pi` gave advice that could not work
(PR #333, re-run in PR #335). Still open, and Pi's own output: every new Pi session opens with a
`No project session found` warning.

**Reopened**: 2026-09-18 for [BUG-001](./bugs/BUG-001.md). A `pi` installed with `npm install -g`
under a Node version manager was not offered, because availability was checked against the session
service's own `PATH` rather than the environment sessions are spawned with. FR-003b added.

**Input**: User description: "add support for https://pi.dev/"

## Context

Feature 026 generalised the AI-CLI seam so that a session records which AI coding CLI backs it, and
landed two providers behind that seam: Claude Code and GitHub Copilot. Its SC-007 stated the point
of the exercise in as many words — *"adding a third AI CLI later requires changes in one place
only"*. This is that third CLI.

Pi is an AI coding agent distributed as the `pi` command. It is the first provider this application
adds that it did not design its seam around, so the feature has a second purpose beyond the CLI
itself: it is the first real test of whether 026's claim holds. Where it does not, the seam is what
changes — never the call sites.

## Clarifications

### Session 2026-09-03

- Q: Pi's only *reported* busy/idle signal (`turn_start`, `turn_end`, `agent_settled`) reaches code
  loaded into Pi as an extension, and Pi's own documentation says extensions run with the user's
  full system permissions. Supply one, infer the signal from Pi's conversation file, or ship Pi
  without a badge? → A: Supply one, scoped to this application's own launches. The badge is how a
  user running three concurrent sessions knows which is waiting for them, and a Pi row that never
  reports is a row that has to be opened to be read — which is the problem the badge exists to
  solve. Inferring from the file was rejected because "quiet" is not "settled": Pi writes nothing
  while it is thinking, so an inferred badge would report idle at precisely the moment the claim
  matters, violating the conservative rule the other two CLIs are held to. Dropping the badge was
  rejected while a working mechanism is available, though feature 026's precedent that a badge may
  degrade is kept as the failure behaviour rather than the design (FR-012d). The permission cost is
  real and is answered by bounding it rather than by accepting it silently: the code is supplied at
  launch and never installed (FR-012a), it may do nothing but report activity for its own session
  (FR-012b), and the user guide says in as many words that it is there (FR-012c).

- Q: Should a Pi session's conversation live in Pi's own session store, or in a store the
  application keeps for itself? → A: Pi's own store, with the application deriving the conversation's
  location from its own session id. One store is what makes FR-015 and FR-016 mean anything: a conversation
  started here is resumable by a bare `pi` in a terminal, one started in a terminal is discovered
  here, and the durable close is marked where it survives the loss of the application's own record.
  It is also the shape both existing providers already have — the application derives paths inside
  the CLI's own storage from its own session id rather than relocating the CLI. Relocating Pi to an
  application-owned directory was rejected because it splits the store in two: the application's
  sessions become invisible to the user's own `pi`, and discovery has to read the default store
  anyway or find nothing. See FR-005b for what happens if Pi will not accept a path for a
  conversation that does not exist yet.

- Q: Can a user decline the activity component that reports Pi's busy/idle signal? → A: Yes, through
  a setting, on by default. The degradation path already exists — FR-012d requires a session whose
  reporting does not load to run normally with an unknown badge — so honouring a refusal costs a
  setting and reuses behaviour the spec already demands. On by default because the alternative makes
  the common case the degraded one: a user who never opens Settings would get a Pi row that never
  moves, with nothing on screen to say why. Having no opt-out at all was rejected as making
  FR-012c's disclosure inert: telling someone this application runs code inside their agent, while
  leaving them no recourse but to choose a different CLI, is not a choice.

- Q: What should happen when a Pi conversation the application has running is opened at the same
  time by the user's own `pi` in a terminal? → A: Detect it and say so. The application checks
  whether a conversation is already live before opening it — its own sessions, and as far as it can
  tell anyone else's — because FR-005a deliberately put the conversation in a store the user shares,
  and losing turns out of the user's own history is the worst outcome available here. Doing nothing
  was rejected: the collision the application can most easily cause is the one between two of its own
  sessions, and it would then cause it. Guarding only its own was rejected as protecting the case
  the user is least likely to hit while ignoring the one FR-005a just made reachable.

  The known cost of detecting a foreign process — that liveness for something this application did
  not start is inferred, and a stale indicator would otherwise refuse a session the user has no way
  to release — is answered by splitting the two cases rather than accepting it. Its own sessions are
  known, so that case is a refusal (FR-006a). Anything else is inferred, so that case is a warning
  the user can proceed past (FR-006b), and being unable to tell is never treated as a positive
  (FR-006c). A guess never becomes a lock.

- Q: Should the application refuse to offer Pi when the installed `pi` is older than the version its
  features need? → A: No. Pi is offered on the same terms the other two CLIs are — it resolves on
  `PATH` where sessions run, or it does not — and an incompatible one is caught by the reporting
  paths that already exist: a start that cannot address its conversation is reported with a reason
  (FR-005), and a badge that cannot load degrades to unknown (FR-012d). Both MUST name the installed
  version, so the user can connect the failure to its cause (FR-003a). A declared minimum was
  rejected on what it would cost the seam: availability is a deliberately cheap path lookup shared by
  every provider, and turning it into a process spawn for one of them is the special case FR-020
  forbids — while the version number itself would go stale and refuse working builds the application
  had not heard of. The published image pins a known-good `pi` (FR-017), so the default experience
  never meets this question at all.

- Q: What user-facing name should Pi carry in menus and sentences — the Settings default, the
  override list, failure messages? → A: "Pi Coding Agent". The seam carries two registers and the
  spec had pinned only one: the sidebar and terminal-bar label is the command name (`pi`, FR-009),
  and this is the other. "Pi Coding Agent" is parallel to "Claude Code" and "GitHub Copilot", which
  both give a product name rather than a bare command, and it survives a menu where "Pi" on its own
  reads as a symbol or an abbreviation — it also gives a user who has not heard of it something to
  search for. Recorded as FR-001a.

### Session 2026-09-12

- Q: When a Pi session runs inside the sandbox, where should the activity component the application
  loads into Pi come from? → A: With the application's own session service. The service is what
  launches Pi, and it is present wherever sessions run — on the host under host placement, inside
  the published image under sandboxed placement — so the component is already where it is needed and
  no placement adds an obligation to anything else. Shipping it into the image as its own pinned
  artifact was rejected because it makes the badge a property of the image: a substituted image
  carrying `pi` would run sessions correctly and lose activity reporting silently, and FR-018's
  report is about a CLI an image lacks, not a capability it quietly dropped. Restricting the badge
  to host placement was rejected as a parity the spec would then have to unsay — feature 027 put
  sessions in the sandbox on the same terms as on the host. Recorded as FR-017a.

- Q: Should the application's session identity be carried in Pi's user-facing conversation name, or
  only in where the conversation is stored? → A: Only in where it is stored. The application derives
  the conversation's location from its own session id and never writes the name field, which belongs
  to the user and is what FR-011 reads. This is what both existing providers already do — a path
  inside the CLI's own store, derived from the session id, and nothing written that the user reads —
  so it needs no new capability from the seam. Carrying the identity in the name field was rejected
  because the two requirements then collide: FR-011 says the sidebar shows the name Pi recorded, so
  every conversation this application started would be labelled with a UUID, and the fallback chain
  FR-011 spends three sentences on would never be reached. Namespacing the identity inside the name
  field and stripping it for display was rejected as the same collision with a parser in front of
  it: the user's own `pi` would still show the marker, and a name the user then edits becomes an
  identity the application cannot resolve. Recorded in FR-005a and FR-011.

- Q: Where should the setting that declines the activity component apply — to the whole application,
  to each project, or to each session as it is started? → A: Application-wide, one switch, held where
  the default-CLI setting is held. The component is identical for every launch and does not vary with
  the project or the session, so a per-project or per-session copy multiplies the control without
  changing what is being consented to — and a consent control the user has to find in three places is
  one they cannot answer with confidence. Per-project was rejected on that ground rather than on
  cost: a user wary of one repository is wary of the mechanism, not of the repository, and the
  component reads no conversation content in any project (FR-012b). Per-session was rejected as
  putting a security decision in the path of every session start, where it becomes a prompt to click
  through. Recorded in FR-012e.

- Q: When a project with hundreds of recorded Pi conversations is opened, how much of each
  conversation may the application read to produce that row's label? → A: A bounded prefix, and no
  more. Pi writes a conversation as one record that grows for its whole life, so a rule that reads
  each record to find a label makes opening a busy project cost more the longer people have worked
  in it — a cost that arrives gradually, on the projects the user cares about most, and looks like
  the application getting slower rather than like a rule anyone chose. A bound keeps a row's cost
  constant and still has every row labelled by the time the sidebar is read. Deferring the read
  until a row is shown was rejected as trading a bounded cost for a visible one: the user would open
  a project to placeholders that resolve as they scroll, and the sidebar's job is to be readable
  without interaction. Reading whatever it takes was rejected because FR-015's existing bound counts
  conversations and says nothing about their size, which is the dimension that actually grows.
  Recorded in FR-011 and FR-015.

- Q: When a user forks a Pi conversation in their own terminal, should the fork appear in the sidebar
  as its own session? → A: Yes, as an ordinary independent session, with the application never
  reading Pi's parentage. Forking is a normal Pi workflow and the fork is a conversation the user can
  resume with a bare `pi`, so leaving it out reintroduces exactly the gap FR-015 exists to close —
  something that exists in Pi and not here. Listing only the newest fork of a lineage was rejected
  because deciding which one that is means reading the session tree this feature declines to surface,
  and because "superseded" is the application's guess about work the user may well still be holding
  open. Listing only conversations with no parent was rejected on the same two grounds, more
  strongly: it hides the conversation the user was most recently in. The sidebar cost is real and is
  accepted — a fork is a conversation, and the row for it is the truth about the worktree. Recorded
  in FR-015.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start and resume a session on Pi (Priority: P1)

A developer who uses Pi picks it the way they already pick Claude Code or GitHub Copilot: as their
default in Settings, or as a one-off override at the moment they start a session. The session opens
with `pi` running in that session's own directory. Closing the application and reopening it later
resumes the same Pi conversation, not a fresh one, and not the wrong CLI.

**Why this priority**: This is the feature. Without it, nothing else here has anything to describe.
A user who can start a Pi session and come back to it tomorrow has the whole value of the change,
even if every row in the sidebar still looked the same.

**Independent Test**: With `pi` installed, set the default to Pi, start a session, confirm `pi` is
the process in the session's terminal and that its working directory is the session's. Restart the
application and confirm the same conversation comes back.

**Acceptance Scenarios**:

1. **Given** Pi is installed and the default is Claude Code, **When** the user starts a session
   through the per-session override and picks Pi, **Then** the session runs `pi` and records Pi as
   its CLI.
2. **Given** the default has been set to Pi, **When** the user starts a session with the primary
   control, **Then** the session runs `pi` without any further choice.
3. **Given** a Pi session exists, **When** the application, the session service, or the machine is
   restarted, **Then** the session still records Pi and resuming it resumes that same conversation.
4. **Given** a Pi session whose conversation Pi no longer holds, **When** the user resumes it,
   **Then** the session is reported as failed, naming the reason, and nothing is started under that
   session's identity.
5. **Given** Pi is not installed where sessions run, **When** the user opens the CLI choice,
   **Then** Pi is not offered as a selectable option.
5a. **Given** an installed `pi` too old to do what a session needs, **When** the user starts a
   session on it, **Then** the failure is reported with a reason that names the installed version,
   and the CLI choice itself was not gated on a version probe.
5b. **Given** `pi` is reachable only through the environment a session is started in — for example
   installed with `npm install -g` under a Node version manager that the environment-include
   script activates — and not through the session service's own `PATH`, **When** the user opens
   the CLI choice, **Then** Pi is offered, and a session started on it runs `pi` (FR-003b).
6. **Given** a user who never selects Pi, **When** they use the application as before, **Then**
   nothing about their sessions, settings, or sidebar has changed.

---

### User Story 2 - Pi works in the sandboxed runtime without installing anything (Priority: P2)

A developer running sessions in the application's sandbox picks Pi and it starts, because the image
the project publishes already carries it. They never install `pi` themselves, and they are never
offered a CLI the sandbox cannot run.

**Why this priority**: Feature 027 put the AI CLIs inside the published image and made that a
checked obligation over every variant the application offers, not a list. The moment Pi becomes an
offered CLI, a sandboxed user who picks it gets a session that can never start its agent — unless
the image ships it. The two land together or the feature is broken for everyone on the sandbox.

**Independent Test**: Build the published image, start a Pi session inside it with nothing installed
on the host, and confirm `pi` runs as the uid the sandbox uses.

**Acceptance Scenarios**:

1. **Given** the default published image and sandboxed placement, **When** the user starts a Pi
   session, **Then** it starts without the user having installed anything into the sandbox.
2. **Given** a user-substituted image that does not carry `pi`, **When** the user selects an image
   or selects a CLI, **Then** the application reports that the image must provide `pi`, naming the
   CLI and the image — and does not discover this at session start.
3. **Given** sandboxed placement, **When** the application decides whether Pi is available, **Then**
   the answer comes from inside the sandbox, not from the host the client runs on.
4. **Given** a user-substituted image that carries `pi`, **When** the user runs a Pi session in it,
   **Then** the activity badge behaves exactly as it does on the published image, the image having
   been required to carry nothing beyond `pi` for that to hold.

---

### User Story 3 - Tell Pi sessions apart from the others at a glance (Priority: P3)

A developer running Claude Code, Copilot and Pi side by side on the same project reads the sidebar
and knows, without opening anything, which row is which CLI, what each conversation is about, and
which one is working versus waiting for them.

**Why this priority**: Concurrent sessions across CLIs is the reason multi-provider support exists.
Three CLIs make the identification problem worse than two did, and a row that cannot be identified
has to be opened to be understood. It is not P1 only because a Pi session is useful before the
sidebar is legible.

**Independent Test**: Create sessions on all three CLIs in one project and confirm each row names
its CLI as text, carries the conversation's own title where the CLI has recorded one, and shows the
same activity badge vocabulary as the other two.

**Acceptance Scenarios**:

1. **Given** sessions on three CLIs, **When** the user reads the sidebar, **Then** each row carries
   a short text label with that CLI's command name — `claude`, `copilot`, `pi`.
1a. **Given** the user opens the Settings default or the per-session override list, **When** they
   read the options, **Then** Pi is named "Pi Coding Agent" there, not `pi`.
2. **Given** a Pi session is open, **When** the user looks at the terminal bar's pinned AI tab,
   **Then** it names `pi` as text beside the glyph.
3. **Given** Pi has recorded a name for a conversation, **When** the sidebar renders that row,
   **Then** the row shows that name.
4. **Given** Pi has recorded no name for a conversation, **When** the sidebar renders that row,
   **Then** it falls back as FR-011 requires and never fails the session.
5. **Given** a supervised Pi session that starts working, **When** the user looks at its row,
   **Then** the badge reads as working, in the same presentation the other two CLIs use.
6. **Given** a supervised Pi session that is thinking and writing nothing, **When** the user looks
   at its row, **Then** the badge still reads as working and never as idle.
7. **Given** a user who wants to know what this application runs inside their agent, **When** they
   read the user guide, **Then** it states that a Pi session loads a component of this application
   into Pi, what it reports, and what it does not touch.

---

### User Story 4 - Find Pi conversations the application did not start (Priority: P4)

A developer who has been running `pi` by hand in a worktree opens that project in the application
and finds those conversations listed as sessions, on the same terms as the ones they started here.
Closing a session they do not want stays closed.

**Why this priority**: Reconciliation exists so the sidebar tells the truth about what is in a
worktree, and a third CLI invisible to it reintroduces exactly the gap that check was added to
close. It is last because it concerns history that already exists rather than the work in front of
the user.

**Independent Test**: Run `pi` by hand in a project directory, open the project in the application,
and confirm the conversation appears as a Pi session. Close it, reopen the project, and confirm it
does not come back.

**Acceptance Scenarios**:

1. **Given** conversations `pi` recorded for a location outside the application, **When** the user
   opens or reopens that project, **Then** those conversations are listed as Pi sessions without the
   user asking.
2. **Given** a discovered Pi session the user closes or removes, **When** they reopen the project,
   **Then** it is not rediscovered.
3. **Given** a location with a large number of recorded Pi conversations, **When** the project
   opens, **Then** all of them are listed — no cap by count, no cut-off by age.
4. **Given** a discovered Pi session the application is not running, **When** the user reads its
   badge, **Then** it reads as unknown rather than idle or working, and the application observes
   nothing for it.
5. **Given** a conversation the user forked inside Pi's own terminal, **When** they open the project,
   **Then** the fork is listed as its own Pi session beside the conversation it came from, with
   nothing in either row depicting the relationship between them.

---

### Edge Cases

- **Pi installed but not authenticated.** Starting is not the same as being usable. The session
  starts and Pi reports its own state in its own terminal; the application does not pre-flight
  credentials or present the CLI's authentication problem as its own failure.
- **Pi's conversation store is unreadable, absent, or in an unrecognised shape.** Discovery
  contributes nothing, the title falls back, the durable-close check answers "not closed", and a
  project open never fails.
- **The installed `pi` is present but too old for what a session needs.** It is still offered — the
  application does not judge versions (FR-003a) — and the resulting failure names the version, so
  the user is pointed at the upgrade rather than left guessing.
- **`pi` is installed through a Node version manager (mise, nvm, fnm, volta).** Its directory is
  on `PATH` only once the shell startup file has run, so the session service, started from the
  desktop, does not see it in its own environment. Sessions do see it, through the
  environment-include script (feature 011). Pi is offered on the strength of the environment
  sessions actually get (FR-003b). With environment-include turned off, that environment is the
  session service's own, and Pi is not offered, consistently with what a session would find.
- **A session's recorded CLI is Pi, but `pi` has since been uninstalled.** The application reports
  which CLI is missing and offers the CLIs that are available, exactly as it already does for a
  missing default, and leaves the stored choice untouched.
- **Pi's conversation for a session is deleted while the session is open.** The running session is
  unaffected; a later resume takes the failure path in Scenario 1.4 rather than starting fresh.
- **Two sessions on the same worktree, one Pi and one another CLI.** Both run concurrently and
  neither sees the other's conversation, configuration, or state.
- **The user asks to open a Pi conversation one of their own sessions is already running.** Refused,
  naming the session that has it (FR-006a). Nothing is started, and the running session is untouched.
- **The user has `pi` open in a terminal on a conversation they then open here.** Detected as far as
  the application can tell, warned about, and permitted if the user goes ahead (FR-006b). The
  application does not decide for them.
- **A conversation looks live because of an indicator left by a process that has since died.** The
  user is warned and proceeds; nothing is blocked (FR-006b). This is the failure mode the warning was
  chosen over a refusal to avoid.
- **The application cannot tell whether a conversation is live.** It opens, with no warning
  (FR-006c). Silence about an unanswerable question is preferred to a caution the user cannot act on.
- **A user forks a conversation inside Pi's own terminal.** The fork is listed as its own session,
  like any other recorded conversation, and the original keeps its row (FR-015). Closing either does
  not close the other, and the relationship between them is Pi's, not shown here.
- **A recorded Pi conversation that the application cannot address under its own session identity.**
  Whatever mapping the application uses between its session and Pi's record, a record it cannot
  address is listed or skipped consistently — never listed and then unresumable.
- **A user turns the activity setting off while a Pi session is already running.** The running
  session is unaffected — it keeps reporting until it ends; the setting governs launches, not live
  sessions, so nothing is torn out from under a session in progress.
- **Pi refuses, or fails, to load the application's activity component.** The session starts and
  runs normally; its badge reads unknown for the session's life. The user is not shown a broken
  session, and nothing about the start is reported as a failure.
- **A substituted image carries `pi` but was not built by this project.** Activity reporting still
  works: the component travels with the session service rather than the image (FR-017a), so the only
  way a Pi session loses its badge is the user's own setting (FR-012e) or a load failure.
- **A user upgrades with sessions and settings written before Pi existed.** Everything loads
  unchanged; nothing is relabelled and no session restarts.

## Requirements *(mandatory)*

### Functional Requirements

**Offering and choosing**

- **FR-001**: Pi MUST be offered as a selectable AI CLI everywhere the application already offers a
  choice between CLIs — the Settings default, the per-session override at the point of creation, and
  the list shown when a stored default cannot be found — on identical rules to the existing CLIs.
- **FR-001a**: Pi MUST carry two distinct names, as every provider does. Where the application
  **offers a choice or names a failure** — the Settings default, the per-session override list, the
  missing-CLI message — it MUST read **"Pi Coding Agent"**, matching the register "Claude Code" and
  "GitHub Copilot" occupy. Where it **labels** a session — the sidebar row and the terminal bar — it
  MUST read `pi`, the command name (FR-009, FR-010). Neither register may be used in place of the
  other.
- **FR-002**: The initial default MUST remain Claude Code. Adding Pi MUST NOT change any existing
  session, any stored setting, or the behaviour a user sees if they never select it.
- **FR-003**: The application MUST NOT offer Pi when `pi` cannot be found where sessions actually
  run — inside the sandbox under sandboxed placement, on the host under host placement.
- **FR-003a**: Availability MUST remain the presence check every provider gets: the application MUST
  NOT declare a minimum Pi version, MUST NOT probe the installed version to decide whether to offer
  Pi, and MUST NOT make deciding availability cost more for Pi than for any other CLI. An installed
  `pi` too old for what this application needs is caught where it fails — FR-005's start failure and
  FR-012d's badge degradation — and each of those reports MUST name the installed version, so the
  user can act on a cause rather than a symptom.
- **FR-003b**: "Where sessions actually run" (FR-003) means **the environment a session is spawned
  with**, not the session service's own process environment. The presence check MUST resolve the
  command against the `PATH` a session started in that directory would receive: the session
  service's `PATH` with the environment-include result (feature 011) applied when that feature is
  enabled. Where a choice is offered for a known directory — the per-session override and the
  missing-CLI list at the point of creation — that directory's environment is used. Where no
  directory is in play — the Settings default — the user's home directory is used. The rule
  applies to every provider, not to Pi alone (FR-019, FR-021), and to both placements, since each
  resolves its environment where its sessions run. The environment MUST be the same resolution
  a spawn in that directory uses, shared with it rather than computed a second way, so that a CLI is
  offered exactly when a session started on it would find it. Resolving that environment MUST NOT
  block the session service's handling of other requests.

**Running**

- **FR-004**: Starting a session recorded as Pi MUST launch `pi` in that session's own working
  directory, in that session's own terminal, fully isolated from every other session.
- **FR-005**: Resuming a session recorded as Pi MUST resume that session's own Pi conversation. When
  Pi no longer holds it, the application MUST report the failure with a sentence naming the reason
  and MUST NOT start a fresh conversation under that session's identity.
- **FR-005a**: A Pi session's conversation MUST live in Pi's own session store for that working
  directory — not in a store belonging to the application — and MUST be addressed by the
  application's own session identity, so the correspondence between a session and its conversation
  is derived rather than remembered. A conversation started in this application MUST therefore be
  resumable by the user's own `pi` outside it, and the application MUST NOT relocate, duplicate, or
  shadow Pi's store. That identity MUST be carried by **where** the conversation is stored and not
  by the name Pi records for it: the application MUST NOT write Pi's conversation name, which
  belongs to the user and is what FR-011 reads.
- **FR-005b**: If Pi will not accept an application-chosen location for a conversation that does not
  yet exist, the application MUST record the correspondence between its session and Pi's own record
  in its own store, and MUST still leave the conversation in Pi's store. Two things are NOT available
  as fallbacks: splitting the conversation into a store of the application's own, because the single
  store is the requirement; and writing the identity into Pi's conversation name, because that field
  is the user's and is what the sidebar reads (FR-005a, FR-011). Only the means of addressing the
  conversation may change.
- **FR-006**: Sessions backed by Pi MUST run concurrently with sessions backed by the other CLIs,
  with no shared filesystem, configuration, or in-memory state between them.
- **FR-006a**: The application MUST NOT run two of its own sessions on one Pi conversation. An
  attempt to open a conversation another of its sessions already has running MUST be refused with a
  sentence naming the session that holds it, and MUST NOT start a second process against that
  conversation. This case is **known**, not inferred — the application started the other session —
  so it is a refusal.
- **FR-006b**: Before opening a Pi conversation, the application MUST make a best-effort check for
  any other process already working on it, including a `pi` the user runs themselves. When one is
  detected, the user MUST be warned — naming the conversation and what the risk is — and MUST be
  able to proceed anyway. This case is **inferred**, so it MUST NOT be a refusal: an indicator left
  behind by a process that has since died would otherwise block a session the user has no way to
  release.
- **FR-006c**: Being unable to determine whether a conversation is live MUST be treated as *not
  live*. The application MUST NOT warn on an absence of evidence, and MUST NOT let a detection it
  cannot make prevent, delay, or degrade opening a session.
- **FR-006d**: The check MUST NOT make opening a session cost more as a project accumulates
  conversations: it concerns the one conversation being opened, MUST NOT scan a location's history,
  and MUST NOT run on a timer or for sessions that are not being opened.
- **FR-007**: The application MUST NOT modify Pi's user-level or project-level configuration. Any
  per-session configuration Pi needs MUST be supplied to that launch alone, and MUST NOT persist as
  a change to what the user's own `pi` does outside the application.
- **FR-008**: A session's recorded CLI MUST survive application restart, session-service restart,
  and machine reboot, and MUST resume on the CLI it was created with.

**Showing**

- **FR-009**: The sidebar MUST identify a Pi session by a short text label carrying the command name
  `pi`, in the same place and the same form the other CLIs use. Identification MUST NOT depend on
  colour alone, a glyph alone, or a tooltip.
- **FR-010**: An open Pi session's terminal bar MUST name `pi` as text beside the glyph on its
  pinned AI tab, by the same command name FR-009 uses.
- **FR-011**: A Pi session's sidebar label MUST use the name Pi has recorded for that conversation
  when Pi has recorded one. Pi records a conversation name only when asked to, so when none exists
  the label MUST fall back to the same summary Pi's own session picker falls back to; when neither
  is available it MUST fall back to the existing neutral placeholder. A missing or unreadable name
  MUST NOT fail the session. No name the application itself wrote may occupy that field (FR-005a),
  so what a user reads on a Pi row is always Pi's or their own. Producing a label MUST read a
  bounded amount from the start of Pi's record and no more, so labelling a row costs the same
  whether its conversation is a minute or a year old; where the bound is reached without a usable
  label, the fallback chain above applies.
- **FR-012**: The sidebar MUST show the busy/idle activity badge for supervised Pi sessions, in the
  same presentation and the same vocabulary the other two CLIs use. The signal MUST be **reported by
  Pi**, not inferred from the size or mtime of its files: the badge changes when Pi says a turn
  started and when Pi says it has settled, so a session that is thinking without writing still reads
  as working.
- **FR-012a**: Obtaining that signal requires the application to supply Pi with code of its own, and
  that code MUST be scoped to the launches this application makes. It MUST be supplied at launch
  only; the application MUST NOT install it into Pi's user-level or project-level extension
  directories, MUST NOT require the user to trust a project for it, and MUST leave the user's own
  `pi` — run outside this application — entirely unaffected. FR-007 governs this and is not relaxed
  by it. The component MUST travel with the application's own session service rather than with the
  environment a session runs in: the service is what launches Pi and is present wherever sessions
  run, so a Pi session MUST report activity identically under host and sandboxed placement, and
  supplying the component MUST NOT add anything to what a sandbox image has to carry (FR-017a).
- **FR-012b**: The supplied code MUST do nothing but report activity for the session it was launched
  with. It MUST NOT read, alter, or transmit conversation content, MUST NOT reach any destination
  other than the application that launched it, and MUST NOT be a general extension point that later
  features can put other work into. Its full extent MUST be reviewable in one place.
- **FR-012c**: Because Pi grants an extension the user's full system permissions, the user guide MUST
  state plainly that starting a Pi session loads a component of this application into Pi, what it
  does, and what it does not do (FR-022). A user MUST be able to find out what runs inside their
  agent without reading the source, and the disclosure MUST name the setting of FR-012e as the way
  to decline.
- **FR-012e**: A setting MUST let the user stop the application supplying that component. It MUST be
  **on** by default, so the badge works without the user configuring anything. When it is off, Pi
  MUST be launched without the component and the session's badge MUST read unknown — the same
  degradation FR-012d already specifies — with the session otherwise unchanged. The setting MUST
  persist across restarts, MUST apply to sessions started after it changes, and MUST NOT alter the
  recorded CLI of any existing session. It MUST be **application-wide** — one switch, held where the
  default-CLI setting is held — and MUST NOT be duplicated per project or per session: the component
  does not vary with either, so neither does the decision about it.
- **FR-012f**: Turning the setting off MUST NOT be presented as a fault, and a Pi session running
  without the component MUST NOT be shown as failed, degraded, or in need of attention. Its badge
  reads unknown for the same reason a discovered session's does (FR-013), and nothing else about the
  row differs.
- **FR-012d**: A Pi session whose activity reporting fails to load, or stops reporting, MUST still
  run. The badge degrades to unknown for that session; it MUST NOT read as idle, and the session
  MUST NOT fail, refuse to start, or be presented as broken.
- **FR-013**: A Pi session the application has discovered but is not running MUST read as unknown
  rather than idle or working, and MUST NOT cause the application to observe that session's storage.
- **FR-014**: Observing Pi for activity MUST be event-driven in the application's own scheduling: no
  polling timer, no periodic wakeup, and no per-idle-session work of its own, on any of the three
  supported platforms.

**Persisting and rediscovering**

- **FR-015**: The application MUST discover Pi conversations recorded for a location that it has no
  record of — including ones the user started by running `pi` themselves, which FR-005a's single
  store makes visible to it — and list them as Pi sessions. Discovery MUST run on every project open and reopen, MUST
  apply the same rules the other CLIs get — no cap by count, no cut-off by age — and its cost MUST
  be proportional to the number of locations, not the number of conversations in them — and MUST
  NOT grow with how long those conversations are (FR-011). Every conversation Pi has recorded for a
  location MUST be listed on the same terms: the application MUST NOT read Pi's parentage and MUST
  NOT distinguish a conversation forked from another from one started fresh, so a fork is an ordinary
  session row and nothing the user can resume with a bare `pi` is absent here.
- **FR-016**: Closing or removing a Pi session MUST be durably suppressed in a way that survives the
  loss of the application's own record, so a closed session is not rediscovered on a later open. The
  suppression MUST NOT delete, truncate, or otherwise destroy the user's conversation: the store is
  shared with the user's own `pi` (FR-005a), and closing a row in this application is not permission
  to remove history from it.

**The sandbox**

- **FR-017**: The image this project publishes MUST ship `pi` at a pinned version, together with
  whatever runtime it needs, so a user on the default image starts a Pi session without installing
  anything into the sandbox. Credentials are excluded: shipping a CLI is not shipping an
  authenticated one.
- **FR-017a**: An image's obligation MUST remain `pi` and its runtime, and nothing further. The
  activity component of FR-012a MUST NOT become a second thing an image ships, pins, or is checked
  for — so a user-substituted image that carries `pi` MUST give a Pi session the same badge the
  published image does. No image MUST be able to run Pi sessions correctly while silently losing
  activity reporting.
- **FR-018**: A user-substituted image that does not carry `pi` MUST be reported where the user
  selects an image and where they select a CLI — naming the CLI and the image — and MUST NOT be
  discovered at session start.

**Keeping the seam honest**

- **FR-019**: Every Pi-specific detail — the command, its launch arguments, where it stores
  conversations, how a name is read, how a closed session is marked, how activity is observed — MUST
  be reached through the existing single provider seam. No session, storage, sidebar, or terminal
  code may name Pi, and the existing automated check that enforces this MUST pass unchanged.
- **FR-020**: Where Pi cannot be expressed through the seam as it stands, the seam MUST be what
  changes. Pi MUST NOT be accommodated by a special case above the seam, by a conditional on which
  CLI a session runs, or by a behaviour available to Pi sessions and to no other CLI's.
- **FR-021**: The application MUST NOT gain a second place that enumerates the CLIs it supports. Any
  surface that lists them — menus, availability checks, the image's own contents check — MUST derive
  the list from the one declaration, so a fourth provider is found by the same checks rather than
  silently omitted from them.

**Documentation**

- **FR-022**: The user guide and README MUST describe Pi as a supported CLI in the same change,
  including how to select it, what its sidebar label reads, and any behaviour where a Pi session
  differs from a Claude Code or Copilot one. Three things MUST be stated outright rather than left
  to be inferred: that a Pi session's conversation lives in Pi's own store and is therefore
  resumable outside this application (FR-005a); that a Pi session loads a component of this
  application into Pi, what it reports, and the setting that declines it (FR-012c, FR-012e); and
  what the application does when it finds a conversation already in use, and that the warning is
  advisory rather than a guarantee (FR-006b, FR-006c).

**Bugfix**: 2026-09-18 — BUG-001. FR-003b added: availability is decided against the environment a
session is spawned with (session-service `PATH` plus the environment-include result), not the
session service's bare process `PATH`. SC-006a amended to allow the one shared, cached environment
resolution that spawns already perform. US1 scenario 5b and an edge case added. See
[bugs/BUG-001.md](./bugs/BUG-001.md).

### Key Entities

- **AI CLI choice**: The named CLI a session is bound to. Gains a third member, carrying both of the
  names FR-001a distinguishes — "Pi Coding Agent" for menus and sentences, `pi` for labels. It is persisted, it
  is what the Settings default and the per-session override select, and it is the single declaration
  every list of CLIs derives from.
- **Session**: Unchanged in shape. It records which CLI backs it and resumes on that CLI.
- **Pi conversation record**: What `pi` itself writes for a working directory — the thing discovery
  reads, a name is taken from (in bounded part, FR-011), and a durable close is marked against.
  Activity is **not** derived from it — that signal is reported by Pi itself (FR-012). Owned by Pi,
  read best-effort by the application, never depended on for correctness.
- **Sandbox image**: The published runtime a session runs inside. Its contract is that it carries
  every CLI the application offers; adding Pi adds an obligation to it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user with Pi installed starts a Pi session in the same number of interactions the
  existing CLIs cost — one when it is their default, two when it is an override — with no trip to
  Settings in between.
- **SC-002**: 100% of Pi sessions resume on Pi, with their own conversation, across application
  restart, session-service restart, and machine reboot.
- **SC-003**: A user who never selects Pi observes no change whatsoever: every session, setting, and
  sidebar row that existed before the upgrade behaves identically after it, with no loss,
  relabelling, or restart.
- **SC-004**: Given a project with sessions on all three CLIs, a user identifies each row's CLI from
  the sidebar as rendered, without opening a session, without relying on colour, and at any contrast
  setting.
- **SC-005**: A supervised Pi session that starts working is shown as working within 1 second, on
  the same terms the other two CLIs are measured on, including while Pi is thinking and writing
  nothing. Where reporting fails, the session reads as unknown — never as idle-while-working — and
  keeps running.
- **SC-004a**: Pi's two names never cross registers: no menu, setting, or failure sentence shows
  `pi` where a product name belongs, and no sidebar row or terminal bar shows "Pi Coding Agent" where
  a command label belongs.
- **SC-004b**: No Pi row in the sidebar is ever labelled with an identifier of the application's
  own: a conversation the application started and one the user started by hand are labelled by the
  same rule, and a user reading either sees a name Pi or they recorded.
- **SC-005a**: A user who turns the activity setting off starts a Pi session that runs identically
  in every respect except its badge, which reads unknown; nothing in the interface reports the
  session or the application as being in an error state. The switch is found in exactly one place
  and governs every project and every Pi session started after it.
- **SC-006**: With Pi sessions idle, the application schedules zero polling timers and zero periodic
  wakeups of its own, and a project holding hundreds of discovered-but-unsupervised Pi sessions
  schedules no observation work at all — verified structurally rather than by measurement.
- **SC-006b**: Opening a project costs the same whether its recorded Pi conversations are minutes or
  years old: the work of labelling a row does not grow with the length of that row's conversation,
  and every row is labelled by the time the sidebar is readable — verified structurally rather than
  by measurement.
- **SC-005b**: A user opening a Pi conversation that another process is working on is told before
  anything starts, and can still proceed. No detection this application cannot make results in a
  blocked session, and no stale indicator leaves a conversation permanently unopenable.
- **SC-006a**: Deciding whether Pi is available costs exactly what deciding it for the other two CLIs
  costs — ~~no process is spawned and no version is read to answer it~~ no version is read, no CLI is
  spawned, and the only process ever spawned is the environment-include resolution a session spawn
  in that directory already performs, shared and cached with it — and, where no directory is in play
  (the Settings default, answered for the home directory per FR-003b), that one home-directory
  resolution, cached the same way (FR-003b; struck 2026-09-18,
  BUG-001: "no process" made the check blind to a CLI a session would find). Where an old `pi`
  does fail, the report names the installed version.
- **SC-001a**: A user whose `pi` is reachable only through the environment-include script — the
  ordinary result of `npm install -g` under a Node version manager — is offered Pi in the Settings
  default and the per-session override, and starts a session on it, with no change to their
  installation (FR-003b, BUG-001).
- **SC-007**: On the default published image, a Pi session starts with nothing installed by the user
  into the sandbox, and the image's own contents check covers Pi without naming it.
- **SC-007a**: A Pi session under sandboxed placement reports activity exactly as one on the host —
  same badge, same vocabulary, same timing — and the image's contents check gains nothing beyond
  `pi` to look for.
- **SC-008**: Adding Pi changes no session, storage, sidebar, or terminal code: the existing check
  that fails when such code names a specific CLI passes unchanged with three providers present.
- **SC-009**: A fourth CLI added after this one is found by every check this feature leaves behind —
  availability, image contents, and the one-place guard — without any of them being edited to
  mention it.
- **SC-010**: The user guide and README describe three CLIs, and the documentation build and link
  checks pass in the same change.

## Assumptions

- **Scope is parity, not Pi's distinctive features.** "Support for Pi" means Pi as a third
  selectable AI CLI with the same capabilities the existing two have. Pi's tree-structured sessions
  and their branching, forking, cloning, export and share commands remain available inside Pi's own
  terminal and are **not** surfaced in this application's interface. Pi's RPC, print/JSON and SDK
  modes are likewise out of scope: sessions run the interactive CLI in a terminal, as they do today.
- **The application owns session identity, as it already does.** A session's identity is the
  application's, and FR-005a makes Pi's record for that conversation addressable from it. Whether Pi
  accepts an application-chosen location for a new conversation is a research question for the
  design phase; FR-005b settles what happens either way, so the answer changes the mechanism and not the
  behaviour.
- **Pi is installed by the user on the host, and by the project in the sandbox.** The application
  does not install, update, or manage a host `pi`; it finds one or reports its absence, and does not
  judge its version (FR-003a). "Finds" means in the environment sessions are spawned with, which is
  where a version-manager install lives (FR-003b, BUG-001). Inside the
  published image the project pins the version, as it already does for the other two CLIs.
- **Pi's conversation store is read, never written by the application.** The conversations in it are
  written by Pi itself (FR-005a); apart from the durable close marker of FR-016, the application
  treats Pi's storage as read-only and best-effort: anything missing or unparseable
  degrades a feature, never fails a session or a project open. The activity component of FR-012a is
  not an exception to this — it is supplied to a launch, not written into Pi's directories.
- **Cross-platform parity applies unchanged.** Pi must work on Linux, macOS and Windows, and no
  Pi-specific code may branch on the host operating system directly.
- **Nothing leaves the machine.** Pi's own network use is Pi's; this feature adds no off-device
  transmission of its own, and the application remains functional offline apart from what the CLI
  itself needs.
- **The published image's base already provides what Pi needs.** The image is built on a Node base
  because the existing CLIs are Node programs; Pi is distributed the same way, so it is assumed to
  need no new runtime. If it does, the image gains it rather than Pi being excluded.

## Out of Scope

- Surfacing Pi's session tree, branching, forking, or export in the application's own interface. A
  forked conversation is still listed like any other (FR-015); what is out of scope is depicting the
  relationship, not the conversation.
- Driving Pi through its RPC, print/JSON or SDK modes instead of an interactive terminal.
- Pi extensions, skills, packages, or custom model providers as an application-managed concern. The
  single component of FR-012a is the one exception, and it is bounded by FR-012b: it reports
  activity and does nothing else, and this feature does not open a general extension point.
- Managing, installing, or updating a host installation of `pi`.
- Any change to how Claude Code or GitHub Copilot sessions behave.
