# Feature Specification: macOS Package

**Feature Branch**: `feat/package-for-macos`

**Created**: 2026-08-27

**Status**: Draft

**Input**: User description: "Prepare a package for macos which should cover the specifics for that system"

## Overview

The application already builds and passes its tests on macOS — CI runs a `macos-latest` job on every
change — but no macOS *artifact* is ever produced. A release publishes Debian packages and nothing
else, so a Mac user has no way to obtain the application other than installing a Rust toolchain and
compiling it. The project constitution's Distribution constraint ("Every release MUST provide builds
for Linux, macOS, and Windows") is therefore unmet on two of three platforms.

This feature closes the macOS half of that gap. It is not a port: the code is already portable, and
the platform's quirks that reach the code — the 103-byte socket-path budget, the Cmd-key bindings —
are already handled. What is missing is everything that stands between a compiled binary and a Mac
user double-clicking an application: the shape macOS expects an application to have, the container it
is delivered in, a first launch whose outcome is known before the user reaches it, and the
documentation that tells them what to do.

"The specifics for that system" is the substance of the work. Debian packaging assumes a package
manager, a filesystem hierarchy, and a service supervisor; macOS has none of those and instead has a
self-contained application folder, drag-to-install, an operating-system-enforced trust check on first
launch, and its own answers for background work, updates, and removal. Each of those is a separate
requirement below.

Windows packaging is deliberately out of scope — it is the same shape of gap and deserves its own
feature rather than being bolted onto this one.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Install and run on a Mac without building it (Priority: P1)

A developer on a Mac hears about the application, opens the project's releases page, downloads the
macOS download, and installs it the way every other Mac application is installed: open the download,
drag the application into Applications, launch it from Launchpad or Spotlight. The application opens,
shows its own icon and name, lets them open a git project, and starts an AI session in the embedded
terminal — with no Rust toolchain, no compiler, no command line, and no separate service to install
or start.

**Why this priority**: Without this, macOS is not a supported platform in any practical sense. Every
other story in this feature refines an experience that does not exist until this one lands, and it is
the single story that, delivered alone, makes the application obtainable by Mac users.

**Independent Test**: On a Mac that has never had the project's source, toolchain, or dependencies,
download the released macOS artifact, install it with the standard gesture, launch it, open a git
repository, and start a session. Delivers a working application to a population that previously had
none.

**Acceptance Scenarios**:

1. **Given** a Mac with no developer tooling installed, **When** the user opens the downloaded macOS
   artifact, **Then** they are presented with the application alongside a clear target for installing
   it, and the standard drag gesture installs it.
2. **Given** the application installed in the standard applications location, **When** the user
   launches it from Launchpad, Spotlight, or the Finder, **Then** the window opens showing the
   application's own icon in the Dock and its own name in the menu bar — not a generic placeholder
   and not a file name.
3. **Given** the freshly launched application, **When** the user opens a git project and starts an AI
   session, **Then** the session service starts automatically without the user installing, enabling,
   or configuring anything.
4. **Given** a running session, **When** the user closes the application window and reopens the
   application, **Then** the session is still running and reattaches — the same behaviour the
   application already promises on Linux.
5. **Given** the installed application, **When** its contents are inspected, **Then** the
   development-only component showcase is not present.

---

### User Story 2 - First launch is not blocked or frightening (Priority: P1)

The user double-clicks the newly installed application for the first time. macOS refuses to open
software it downloaded from the internet unless it can attribute it to a developer Apple knows, and
this project does not sign with such an identity — so the refusal is expected. What must not happen
is that the refusal is a surprise: the user has already been told, on the download page and in the
documentation, exactly which dialog they will see and the one gesture that clears it. They perform it
once, the application opens, and it never asks again.

**Why this priority**: This is the same priority as US1 because on macOS it is not a separate step —
it is part of the first launch. An artifact the operating system refuses to open is not an
installable artifact, and a user who hits an *unexplained* refusal does not retry, they conclude the
download is broken or malicious and leave. The refusal itself is acceptable; being unprepared for it
is not.

**Independent Test**: On a Mac that has never run the application, download the released artifact
through a web browser (so the operating system marks it as downloaded), install and launch it, and
record every dialog shown. Compare against the documented expectation, follow the documented steps,
and confirm the application opens and that a second launch is clean. Delivers a first launch whose
outcome is known in advance rather than discovered.

**Acceptance Scenarios**:

1. **Given** the artifact downloaded through a web browser, **When** the user launches the installed
   application for the first time, **Then** the dialog shown matches what the macOS installation
   documentation predicts, including its wording and the reason behind it.
2. **Given** that dialog, **When** the user follows the documented steps, **Then** the application
   opens on the first attempt, and every subsequent launch opens with no further prompting.
3. **Given** a user who would rather not clear the block by hand, **When** they consult the
   documentation, **Then** it offers building from source as the alternative and says plainly why the
   released artifact is not attributable to a known developer.
4. **Given** the released artifact, **When** its signature is checked with the operating system's own
   verification, **Then** it carries the signature this feature claims it carries, verified
   automatically at release time rather than by someone remembering to look.

---

### User Story 3 - Every release ships a macOS download, automatically (Priority: P2)

A maintainer merges the release pull request. Without any further action, the published release
carries a macOS download beside the existing Debian packages, built from exactly that release's
sources and labelled with that release's version.

**Why this priority**: A one-off hand-built artifact satisfies US1 exactly once and then rots. Making
it automatic is what turns "macOS was packaged" into "macOS is supported". It is P2 rather than P1
only because the first release can be produced by hand to prove US1 while this is being built.

**Independent Test**: Run the release process for a version and confirm the published release
contains a macOS artifact whose reported version matches the tag, without a human having built or
uploaded it. Delivers ongoing macOS availability rather than a single snapshot.

**Acceptance Scenarios**:

1. **Given** a release is created, **When** the release process completes, **Then** the published
   release includes a macOS download alongside the Debian packages.
2. **Given** a published macOS download, **When** the user checks the application's version in the
   About dialog, **Then** it matches the release it was published under.
3. **Given** the release process, **When** the macOS artifact cannot be produced, **Then** the
   release stays an unpublished draft rather than publishing without it, the failure is visible, and
   re-running the failed step is enough to publish it complete.

---

### User Story 4 - Works on both kinds of Mac (Priority: P2)

A user on an Apple silicon Mac and a user on an Intel Mac download the same release and both get an
application that runs natively — full speed, no compatibility layer, no prompt to install one, and no
guessing which of several downloads is theirs.

**Why this priority**: Getting this wrong sends half the audience to a download that will not run, or
runs slowly under emulation. It is separable from US1 (which can be proven on one machine) but must
land before the platform can be called supported.

**Independent Test**: Run the same released artifact on an Apple silicon Mac and on an Intel Mac and
confirm both launch and run without a compatibility layer. Delivers coverage of the whole macOS
population rather than the maintainer's own machine.

**Acceptance Scenarios**:

1. **Given** an Apple silicon Mac, **When** the user installs and runs the released artifact, **Then**
   it runs natively, with no prompt to install a compatibility layer.
2. **Given** an Intel Mac running a supported macOS version, **When** the user installs and runs the
   released artifact, **Then** it runs natively.
3. **Given** a Mac running a macOS version older than the stated minimum, **When** the user tries to
   launch the application, **Then** they are told the requirement rather than seeing an unexplained
   failure, and the minimum is stated on the download page and in the documentation.

---

### User Story 5 - Update and remove it the Mac way (Priority: P3)

A user who already has the application installed downloads a newer release and replaces the installed
copy with the same drag gesture; their projects, settings, and sessions are still there afterwards.
Later, a user who no longer wants the application drags it to the Trash, and the documentation tells
them exactly what — if anything — is left behind and how to remove it.

**Why this priority**: Neither is required to get value from the application, but both are part of the
platform's expected contract, and getting update wrong (losing settings, or leaving a stale background
service running against a new application) produces confusing bug reports.

**Independent Test**: Install version N, create a project and a session, install version N+1 over it,
confirm state survives and the running service is the new one; then remove the application and confirm
the documented leftovers are the actual leftovers. Delivers a complete install/update/remove lifecycle.

**Acceptance Scenarios**:

1. **Given** an installed application with saved projects and settings, **When** the user replaces it
   with a newer release, **Then** projects, settings, and session history are preserved.
2. **Given** a session service left running by the previous version, **When** the newer version is
   launched, **Then** the version mismatch is resolved without the user having to find and stop a
   background process by hand.
3. **Given** an installed application, **When** the user drags it to the Trash, **Then** no background
   process is left running indefinitely, and the documentation lists every location that still holds
   user data along with how to remove it.

---

### User Story 6 - Know what happens when you log out (Priority: P3)

A user with long-running AI sessions logs out of macOS at the end of the day. The sessions stop —
macOS offers no unprivileged way for the application to keep a background service alive across a
logout, and this feature does not invent one. What the user gets instead is a straight answer: the
macOS documentation states that sessions survive closing the window but not logging out, and points
at the one thing that does survive it — running the session service in a container, which already
works on macOS and is the platform's own answer, since the container runtime is a service macOS keeps
running on its own.

**Why this priority**: The behaviour is unchanged, so nothing here blocks a user from working. It is
in scope because the macOS documentation this feature writes would otherwise be silent on a question
every user of long-running sessions eventually asks, and because the existing documentation's
statements about macOS must be checked against what this package actually ships.

**Independent Test**: Read the macOS documentation and confirm it answers "what happens to my
sessions when I log out?" without the reader having to infer it; then verify the claim by starting a
session, logging out, and logging back in. Delivers a documented, verified boundary instead of an
undiscovered one.

**Acceptance Scenarios**:

1. **Given** the macOS documentation, **When** a user looks for logout behaviour, **Then** it states
   that sessions do not survive logging out when the service runs directly on the computer, and why.
2. **Given** that same documentation, **When** the user wants sessions to survive logout, **Then** it
   points them at the container placement and confirms it provides this on macOS.
3. **Given** the installed application, **When** the user inspects what it registered with the system,
   **Then** it registered nothing that runs in the background — installing the application starts no
   permanent component and enables no login item.

---

### Edge Cases

- **The user runs the application straight from the download without installing it.** macOS may run a
  downloaded application from a randomised, read-only location, and that copy is discarded when the
  download is ejected. The application must recognise this and say so, pointing at the install
  gesture — never run in a way that looks installed, and never half-work.
- **The companion session service is not found beside the application.** The application locates the
  service next to itself; if the packaged layout separates them, every session silently fails to
  start. This must be verified on the packaged artifact, not just on a development build.
- **A stale service from a previous version is still running when a newer application launches.** The
  two refuse to talk to each other, and the user sees a refusal with two version numbers.
- **The user's home directory path is unusually long**, pushing the service's communication path past
  the platform's hard limit. The application already detects this and reports it; the documentation
  must say what the user can do about it.
- **The user opens the application before granting the file-access permissions macOS asks for**, or
  declines them, and then opens a project in a location the operating system protects. The failure
  must be attributable to the permission, not presented as a broken project.
- **The container placement is selected but the container runtime is not installed**, which on macOS
  is an extra application the user must obtain separately.
- **The user has both an old hand-built copy and the released application**, and the two disagree about
  where state lives.
- **The download is opened on a Mac older than the stated minimum**, or on a Mac of the other
  architecture than the one it was built for.
- **The development-only showcase, or any other internal tool, is accidentally included** in the
  shipped artifact.
- **The user clears the first-launch block on the copy still inside the mounted download** rather
  than on the installed one, and then finds the installed copy blocked again. The documented gesture
  must be stated against the installed copy, in that order.
- **The user updates to a newer release and meets the first-launch block a second time**, because the
  block is per-copy and a new download is a new copy. The documentation must say so rather than
  letting it read as a regression.
- **The macOS artifact fails while the Linux ones succeed.** Publishing is immutable, so a release
  that goes out incomplete can never be corrected; the release must instead wait, which means a
  failure here holds up every platform and someone has to notice.

## Requirements *(mandatory)*

### Functional Requirements

#### The shape of a macOS application

- **FR-001**: The application MUST be delivered as a self-contained macOS application that a user can
  move, copy, and run as a single item, with no separate installation step for its companion session
  service.
- **FR-002**: The application MUST present its own name and its own icon everywhere macOS shows an
  application's identity: the Dock, the menu bar, the Finder, Launchpad, Spotlight, and the
  force-quit list.
- **FR-003**: The packaged application MUST declare its version, and that version MUST match both the
  release it was published under and the version the application's own About dialog reports.
- **FR-004**: The packaged application MUST support the two most recent macOS releases, and no
  older. It MUST declare that floor, and the declaration MUST be enforced by the operating system
  rather than left to the user to check. The floor is a maintained value rather than a derived one —
  nothing available to this project knows when Apple has shipped a major release — so the
  requirement is on how it is maintained: the value MUST be recorded in exactly one place, the
  documentation MUST state that same value and an automated check MUST enforce the agreement so the
  two can never drift apart, and re-checking the value against Apple's current releases MUST be a
  named step in the developer documentation for producing a release. At the time of writing the
  value is macOS 15 (Sequoia).
- **FR-005**: The companion session service MUST be located and started automatically by the packaged
  application, exactly as it is on Linux, with no user action and no reliance on the service being on
  the user's command-line path.
- **FR-006**: The packaged application MUST NOT contain the development-only component showcase, and
  this MUST be enforced by an automated check on the packaging definition rather than by review — the
  same guarantee the Debian package already carries.
- **FR-007**: The packaged application MUST NOT require the user to install a runtime, framework,
  toolchain, or system library that does not ship with a stock macOS installation, for anything the
  application does by default.

#### Delivery

- **FR-008**: The macOS download MUST use the platform's conventional delivery container and present
  the platform's conventional install gesture, so that a user familiar with macOS needs no
  instructions to install it.
- **FR-009**: The download's file name MUST identify the application, its version, and — where a user
  must choose between downloads — which Mac it is for.
- **FR-010**: Each published release MUST include the macOS download alongside the existing Debian
  packages, produced by the automated release process from that release's sources.
- **FR-011**: A release whose macOS artifact failed to build or upload MUST NOT be published at all.
  It MUST remain an unpublished draft until every supported platform's artifact is attached, and the
  failure MUST be visible rather than silently producing a Linux-only release. Recovery MUST be a
  re-run of the failed step followed by publication, with no artifact lost and no version number
  consumed.

#### Trust and first launch

- **FR-012**: The released artifact MUST carry an ad-hoc signature, so that macOS treats it as a
  well-formed application whose code has not been tampered with since it was built, rather than as an
  unidentifiable or damaged one.
- **FR-013**: The first launch of a freshly downloaded copy MUST have a known outcome that the
  documentation predicts exactly: the operating system blocks it because it cannot attribute it to a
  registered developer, and one documented gesture clears that block permanently for that copy. An
  outcome the documentation does not predict is a defect.
- **FR-014**: The documented gesture that clears the block MUST be one a non-technical user can
  perform entirely through the operating system's own interface, without a terminal. Across the
  supported floor (FR-004) it MUST be a single gesture, not one per version; a command-line
  equivalent MAY be documented as an alternative, never as the only route.
- **FR-015**: The signature on the released artifact MUST be verified automatically as part of
  producing it, so that a regression that ships an unsigned or malformed artifact — which macOS
  reports as "damaged", indistinguishable from a corrupt download — is caught at release time and not
  by the first user.
- **FR-016**: Producing the released artifact MUST NOT require any Apple Developer account,
  certificate, or release secret. A contributor MUST be able to produce an artifact equivalent in
  trust status to the released one.
- **FR-017**: The release process MUST be structured so that adding Developer ID signing and Apple
  notarization later is a matter of supplying credentials and one additional step, without reshaping
  how the artifact is built or delivered.
- **FR-018**: The application MUST remain fully functional after the user clears the block — in
  particular it MUST still be able to start shells, AI command-line tools, and git as child processes,
  and MUST still render its interface.
- **FR-019**: When the application is launched from the delivery container rather than from where the
  user installed it — including the randomised read-only location macOS may relocate it to — it MUST
  recognise that, say so in one sentence, and point at the install gesture instead of continuing. It
  MUST NOT run in a way that looks installed, because the copy is discarded on eject, the trust
  gesture performed on it does not carry to the installed copy, and nothing done in it can be found
  again.

#### Platform coverage

- **FR-020**: The release MUST cover both Apple silicon and Intel Macs, either with a single download
  that runs natively on both or with clearly-labelled per-architecture downloads.
- **FR-021**: Whichever coverage approach is taken, a user MUST NOT have to determine their Mac's
  architecture in order to choose correctly; if there is a choice, the download page and the
  documentation MUST make it unambiguous.

#### Lifecycle

- **FR-022**: Replacing an installed copy with a newer one MUST preserve the user's projects,
  settings, and session history.
- **FR-023**: When a newer application meets a session service left running by an older version, the
  mismatch MUST be resolved without the user having to find and stop a background process by hand.
- **FR-024**: Removing the application MUST leave no process running indefinitely, and the
  documentation MUST list every location that still holds user data and how to remove it.
- **FR-025**: Installing the application MUST NOT register, enable, or start any always-running
  background component — no login item, no launch agent, nothing that outlives the user's login
  session. The session service the application starts stays tied to the login session, as it does
  today.
- **FR-026**: The documentation MUST state explicitly that on macOS sessions survive closing the
  window but not logging out when the service runs directly on the computer, say why, and point at
  the container placement as the supported way to survive logout on macOS.

#### Permissions

- **FR-027**: The packaged application MUST ask only for the protected locations it actually needs,
  at the moment it needs them, rather than requesting broad access up front. Each request MUST carry
  an explanation stating why the application needs that location, in the user's terms.
- **FR-028**: When a session fails because a macOS permission was not granted, the application MUST
  attribute the failure to the permission and say how to grant it, rather than reporting a generic
  error.
- **FR-029**: Because the embedded terminal runs tools the project cannot enumerate in advance, the
  documentation MUST additionally describe a one-time broad grant the user can choose to give, so a
  user whose work sits in protected locations can stop being interrupted mid-session. It MUST be
  presented as optional, with what it grants stated plainly, and MUST NOT be a prerequisite for
  installing or running the application.

#### Documentation

- **FR-030**: The user guide MUST gain macOS installation documentation covering: obtaining the
  download, installing it, exactly what happens on first launch, the minimum macOS version, which Mac
  it runs on, granting permissions — including the optional broad grant of FR-029 — updating,
  removing, and where user data lives.
- **FR-031**: The project's README MUST stop describing releases as Debian-only and MUST state what
  each release provides for macOS.
- **FR-032**: The existing documentation statements about macOS — that logout survival is not
  supported directly on the computer, and that the container placement provides it — MUST be checked
  against what this feature ships and left in agreement with it.
- **FR-033**: Developer documentation MUST describe how the macOS artifact is produced, how a
  contributor reproduces it locally, and what would have to change to sign and notarize it with an
  Apple Developer identity later.

#### Verification

- **FR-034**: The packaging definition MUST be covered by automated checks that run on every change —
  at minimum, that the shipped item list is complete, that it excludes internal tools, and that the
  application and its companion service are laid out so that the application can find the service.
- **FR-035**: Producing the macOS artifact MUST be reproducible by a single documented command in the
  project's task runner, consistent with how the Debian package is produced.
- **FR-036**: The packaged application MUST be assembled and shown to actually start on macOS — not
  merely to have been assembled — on every change able to affect what is built or packaged. This
  MUST reuse the macOS build that already runs, and MAY be limited to the runner's own architecture
  with no delivery container, so that it costs little enough to run every time.
- **FR-037**: The remaining steps that only a release performs — covering the second architecture
  (FR-020), producing the delivery container (FR-008), and verifying the signature (FR-015) — MUST
  be exercised by the release process itself, and a failure in any of them MUST hold the release as
  a draft under FR-011 rather than publishing without them.

### Key Entities

- **macOS application bundle**: The self-contained item the user drags to Applications. Holds the
  application, its companion session service, its icon, its identity (name, version, minimum system
  version), and its permission explanations.
- **Delivery container**: The file the user downloads, which presents the bundle together with the
  install gesture. It is a means of transport, not a place to run the application from — FR-019 makes
  that explicit to the user.
- **Ad-hoc signature**: The signature the artifact carries. It proves the code has not been altered
  since it was built, but names no developer, so macOS cannot attribute the application to anyone.
  Needs no account and no secret, which is why a contributor's build and the released build are
  equivalent.
- **Quarantine flag**: The mark macOS puts on anything downloaded through a browser. It is what
  triggers the first-launch block, it is per-copy, and clearing it once is what US2's documented
  gesture does.
- **Download-page trust notice**: The statement, alongside the download itself, of what the first
  launch will do and how to proceed — the thing that turns an alarming block into an expected step.
- **Release artifact set**: The complete set of downloads a release publishes; incomplete on macOS
  today, and what FR-010 and FR-011 make whole.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A Mac user who has never seen the project goes from the releases page to a running
  application in under 5 minutes, without reading anything beyond the download page, and every step
  they take — including clearing the first-launch block — is one that page told them to expect.
- **SC-002**: 100% of published releases from this feature onward include a macOS download; zero
  releases are published claiming macOS support without carrying it.
- **SC-003**: On a Mac with no developer tooling of any kind, the application launches, opens a git
  project, and runs an AI session — measured on a machine that has never had the project's source or
  toolchain.
- **SC-004**: First launch on a freshly downloaded copy shows exactly the dialog the documentation
  predicts, and the documented gesture resolves it on the first attempt; every launch after that is
  clean.
- **SC-005**: The application runs natively on both Apple silicon and Intel Macs; 0% of users are
  prompted to install a compatibility layer.
- **SC-006**: The development-only showcase, and any other internal tool, is absent from 100% of
  shipped macOS artifacts, verified automatically on every change rather than by inspection.
- **SC-013**: A change that breaks the packaged application's ability to assemble or start is caught
  by the pull request that introduces it, not by the release it would otherwise block.
- **SC-007**: Installing a newer release over an older one preserves 100% of the user's projects,
  settings, and session history, with zero manual steps.
- **SC-008**: Removing the application leaves zero processes running, and every remaining piece of
  user data appears in the documented list.
- **SC-009**: Session behaviour on macOS matches Linux for everything the application promises other
  than logout survival: sessions start, survive closing the window, and reattach.
- **SC-010**: A contributor can produce a macOS artifact with one documented command, with no Apple
  Developer account and no release secret, and it is equivalent in trust status to the released one.
- **SC-011**: A user who reads the macOS documentation can answer, without inferring it, what happens
  to their sessions when they close the window, quit the application, and log out.
- **SC-012**: Installing the application registers zero background components with the operating
  system — nothing runs before the user launches the application, and nothing outlives their login
  session.

## Assumptions

- **Scope is packaging, not porting.** The application already builds and passes its test suite on
  macOS in CI, and the platform differences that reach the code (the socket-path budget, the Cmd-key
  bindings, theme detection) are already handled. This feature adds the artifact, the trust, the
  lifecycle, and the documentation around that working build. Any code change it needs is confined to
  packaging definitions, build tooling, the release process, and whatever the packaged layout forces.
- **Windows packaging is a separate feature.** The same gap exists there and is acknowledged, but
  bundling both would double the surface and halve the scrutiny each receives.
- **Both architectures are covered by a single universal download**, because it removes the user's
  need to know which Mac they have (FR-021) at the cost of a larger download and a longer release
  build. Per-architecture downloads remain an acceptable alternative under FR-020 if the release build
  proves impractical, provided the labelling makes the choice unambiguous.
- **The supported floor is the two most recent macOS releases** (FR-004) — a floor that moves with
  Apple rather than a fixed version number, so it never claims coverage of releases nothing tests.
  Two consequences are relied on elsewhere in this spec: the block-clearing gesture is the same on
  every supported version, because the older Finder-based bypass predates the floor (FR-014); and
  Intel coverage (FR-020) is inherently time-limited, since macOS support for Intel Macs ends within
  a few releases and the floor will eventually rise past it. Dropping the Intel half of FR-020 when
  that happens is a follow-up, not a defect in this feature.
- **The container placement is unchanged by this feature.** Running the session service in a container
  on macOS already works through the platform's container runtime, and it remains the documented
  answer for logout survival.
- **Distribution is through the project's own releases page**, not the Mac App Store. The App Store's
  sandbox is incompatible with an application whose purpose is running arbitrary command-line tools
  against the user's own git repositories.
- **No auto-update mechanism is in scope.** Updating means downloading the new release and replacing
  the application, which is what FR-022 covers.
- **The existing Debian packaging, its systemd units, and the Linux logout-survival flow are
  untouched.** This feature adds a platform; it does not reshape the one that works.
- **The user data locations the application already uses on macOS are correct and unchanged.** This
  feature documents them (FR-024) rather than moving them.

## Clarifications

### Session 2026-08-27

- **Q: How far should macOS trust go for the released artifact?**
  **A: Ad-hoc signed.** No Apple Developer account, no certificate, no release secret. macOS still
  blocks the first launch of a downloaded copy because it cannot attribute the application to a
  registered developer, so the documentation carries that weight: it predicts the dialog and gives the
  one gesture that clears it. Decided FR-012 through FR-018, the shape of US2, and SC-004.
  Developer ID signing with notarization is the better first launch and remains the intended
  destination — FR-017 requires the release process be built so that reaching it later is a matter of
  supplying credentials, not a redesign.

- **Q: Should the macOS package close the logout-survival gap by shipping an opt-in background
  registration?**
  **A: No — keep pointing at the container placement.** macOS behaviour is unchanged: sessions survive
  closing the window, not logging out. The application registers nothing that runs in the background
  and installing it starts nothing. Decided US6, FR-025, and FR-026. This keeps the feature a
  drag-to-install application rather than one that also introduces an always-running component with
  its own install, upgrade, and removal lifecycle — and the container placement already provides
  logout survival on macOS through the container runtime, which macOS keeps running on its own.

- Q: Which is the oldest macOS version the packaged application will support? → A: The two most
  recent macOS releases — a floor that moves with Apple, which today means macOS 15 (Sequoia) and
  newer.

- Q: When the macOS artifact fails to build or upload, what should happen to the rest of that
  release? → A: Hold the whole release — it stays an unpublished draft until every platform's
  artifact is attached, and a re-run publishes it with nothing lost.

- Q: How much of the macOS packaging should be exercised on every pull request, rather than only
  when a release is cut? → A: Assemble and smoke-test the bundle on every pull request, single
  architecture and no disk image, reusing the macOS build job that already runs; the universal
  build, the disk image, and the signature check stay in the release job.

- Q: When the user opens a project in a folder macOS protects, how should the application ask for
  access? → A: Ask per folder, at the moment of use, each request explaining why — and document a
  one-time broad grant as an optional escape hatch for users whose work sits in protected locations
  or who hit prompts mid-session.

- Q: When the user launches the application straight from the mounted download instead of installing
  it, should it work from there or stop and tell them to install it? → A: Detect it and refuse with a
  one-sentence instruction pointing at the install gesture — it never half-works, and the user does
  not end up trusting a copy that vanishes on eject.
