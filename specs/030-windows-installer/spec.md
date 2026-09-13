# Feature Specification: Windows Installation Package

**Feature Branch**: `feat/windows-package`

**Created**: 2026-09-13

**Status**: Draft

**Input**: User description: "the app should support installation package for windows same as currently support dep for linux and macos"

## Context

Today every release carries a Debian package for Linux (64-bit Intel/AMD and 64-bit Arm), built
and attached by the release pipeline before the release is published. The package installs the
application, the session service beside it, a menu entry with the application's icon, and removes
cleanly. Windows has no package: the install guide tells Windows users to clone the repository and
build from source with a Rust toolchain and the Visual Studio Build Tools. That contradicts the
project's own distribution rule: every release has to ship builds for Linux, macOS, and Windows.
macOS is being packaged in parallel by its own feature (`028-macos-package`, PR #284). That feature
adds an unsigned disk image, a per-platform install page, a per-change CI packaging check, and a
release rule that publishes only complete artifact sets.

This feature gives Windows users the same experience Linux users already have: download one file
from the release, install it, and start the application from the Start menu.

The Windows build compiles and passes its core tests, but it cannot run a session yet when the
session service runs on the user's own computer. Planning found three gaps, all deferred since
feature 010 (task T083/W5):

- **No connection.** The service's local endpoint on Windows is a stub that always fails, so the
  application cannot connect to a service it started.
- **No restart.** "Restart service" cannot stop a running service on Windows.
- **Console flashes.** Background commands (git, the environment-include script, the container
  runtime) would each flash a console window once the application runs without a console of its
  own.

Packaging an application whose sessions all fail would not meet User Story 1, so closing these gaps
is part of this feature (decided 2026-09-13).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Install from a release download (Priority: P1)

A Windows developer who has never built Rust software opens the install guide, downloads the
Windows installer for the current release, runs it, and starts Micold AI IDE from the Start menu.
They open a project and start a session. Nothing had to be compiled and no developer toolchain had
to be installed.

**Why this priority**: This is the whole feature. Without it, Windows users either give up or go
through a from-source build that needs several gigabytes of tooling. Each later story assumes the
application was installed this way.

**Independent Test**: On a clean Windows machine with no Rust or Visual Studio tooling, download
the installer from a release, run it, launch the application from the Start menu, open a git
repository, and start a shell session that shows output.

**Acceptance Scenarios**:

1. **Given** a clean Windows machine and a published release, **When** the user downloads the
   Windows installer linked from the install guide and runs it, **Then** installation completes
   without requiring any developer tooling and the application appears in the Start menu with its
   own icon.
2. **Given** the application was just installed, **When** the user launches it from the Start menu,
   **Then** the main window opens and no console window appears alongside it.
3. **Given** the application is running, **When** the user opens a project and starts a session,
   **Then** the session service starts on its own, the session runs, and no console window appears
   for the service either.
4. **Given** the application is installed, **When** the user opens Help → About, **Then** the
   version shown matches the release the installer was downloaded from.
5. **Given** the installer, **When** its contents are inspected, **Then** it does not contain the
   development-only component showcase.

---

### User Story 2 - Upgrade and uninstall cleanly (Priority: P2)

A Windows user with an older release installed runs the newer release's installer, which replaces
the old version in place. Later they remove the application through Windows' standard "Installed
apps" list, and the program files and shortcuts go away.

**Why this priority**: A package that installs but cannot be upgraded or removed leaves users with
stale versions or leftover files. That puts it below what the Linux package offers (install over the
top, `apt remove`). It comes second because a first install is useful even before upgrades are
polished.

**Independent Test**: Install release N, then install release N+1 over it. Confirm only one entry
appears in "Installed apps" and About shows N+1. Uninstall it and confirm the program files and
Start menu entry are gone while the user's saved projects and settings remain.

**Acceptance Scenarios**:

1. **Given** an older version is installed, **When** the user runs a newer version's installer,
   **Then** the installed version is replaced, exactly one entry for the application remains in the
   system's installed-apps list, and the user's projects and settings carry over.
2. **Given** the application is installed, **When** the user uninstalls it from the system's
   installed-apps list, **Then** the program files, Start menu entry, and installed-apps entry are
   removed.
3. **Given** the application was uninstalled, **When** the user reinstalls any version, **Then** their
   previously saved projects and settings are still present, because uninstalling never deletes
   user data.
4. **Given** the application or its session service is running, **When** the user starts an upgrade
   or uninstall, **Then** the installer tells them the application is in use and that running
   sessions will end. It proceeds only once they confirm or close the application. It never leaves
   a half-replaced installation behind.

---

### User Story 3 - Every release ships it automatically (Priority: P2)

The maintainer merges a release pull request. The release pipeline builds the Windows installer
alongside the Linux packages, attaches it to the release, and publishes the release only after it
is attached. The documentation site links to the new installer.

**Why this priority**: If the installer is built by hand, sooner or later a release goes out
without it, and the project's releases are immutable once published. It is P2 rather than P1
because a manually attached installer still serves Story 1, but it is required before the feature
counts as done.

**Independent Test**: Cut a release and confirm, without any manual step, that the published
release carries the Windows installer, that the install guide on the documentation site links to
it, and that the link downloads the file.

**Acceptance Scenarios**:

1. **Given** a release is being cut, **When** the pipeline runs, **Then** the Windows installer is
   built and attached to the draft release before the release is published.
2. **Given** the Windows installer fails to build or attach, **When** the pipeline reaches the
   publish step, **Then** the release is not published, just as a failed Linux package holds it
   back today.
3. **Given** a release is published, **When** the documentation site is published from it, **Then**
   the install guide's Windows section links to that release's installer, and the site's existing
   check that every download link names a real release asset covers the Windows link too.
4. **Given** a maintainer working on a Windows machine, **When** they run the project's standard
   packaging task, **Then** they get the same Windows installer locally that the pipeline would
   produce, the same way the Linux package can be built locally today.

---

### User Story 4 - Install guide describes the Windows package (Priority: P3)

A Windows user reading the install guide finds a Windows section like the Linux one: which file to
download, how to install it, what it installs, how to upgrade and remove it, and what they still
need beside it (their own AI CLI). The from-source instructions stay available but are no longer
the only route on Windows.

**Why this priority**: Documentation ships with the feature (Principle VII), but it only has value
once the installer exists.

**Independent Test**: Read the published install guide's Windows section and follow it on a clean
machine from download to a running session without consulting any other page.

**Acceptance Scenarios**:

1. **Given** the published install guide, **When** a Windows user reads it, **Then** it no longer
   says there is no packaged build for Windows. It names the download, the SmartScreen warning and
   how to continue past it, the install and removal steps, and what is installed.
2. **Given** the install guide, **When** a user reads what the session service can and cannot do
   on Windows, **Then** it restates the existing limit that sessions survive closing the window but
   not logging out, unless the service runs in a container.

### Edge Cases

- **User without administrator rights**: installation must succeed for a standard user; see FR-004.
- **Unrecognised publisher warning**: the installer is unsigned (FR-010), so Windows SmartScreen
  warns before running it. The user must be able to continue with the steps the install guide
  names, and nothing about the installed application may depend on a signature being present.
- **Download blocked outright**: some managed or policy-restricted machines refuse unsigned
  installers entirely. The install guide must point those users to the from-source build instead of
  implying the installer will always run.
- **Application or session service running during upgrade/uninstall**: on Windows a running program
  cannot be replaced. The installer must detect this and act as described in Story 2, scenario 4,
  instead of failing partway.
- **Session service left running from the old version after an upgrade**: the new client must not
  end up talking to a service from the previous version. The upgrade either stops the old service
  or the client's existing version-mismatch handling replaces it.
- **Unsupported processor architecture**: running the installer on an architecture no package was
  built for must fail with a clear message, not install something that cannot start.
- **AI CLI not installed**: the installer does not bundle or install Claude Code or Copilot CLI. A
  session that asks for one that is missing behaves as it does on other platforms, and the install
  guide lists them as prerequisites.
- **Install location contains spaces or non-ASCII characters** (for example a user profile name
  with accents): the application and its session service must still start and find each other.
- **Two Windows accounts signed in at once** (fast user switching, or a shared machine): each gets
  its own session service, and neither can connect to the other's (FR-021).
- **Session service killed or crashed**: the next time the application needs the service it
  starts a new one. A stale endpoint or single-instance marker left behind must not block it.
- **Installer re-run for the same version**: repairs or reinstalls without creating a second
  installed-apps entry.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every release MUST provide a Windows installer as a downloadable release asset, next
  to the existing Linux packages.
- **FR-002**: The Windows installer MUST install both the application and its session service,
  placed so the application finds the service without any configuration from the user.
- **FR-003**: The installer MUST add a Start menu entry that launches the application and shows the
  application's icon. The installed executable MUST also show that icon.
- **FR-004**: Installation MUST NOT require administrator rights. A standard Windows user MUST be
  able to install, upgrade, and uninstall the application for their own account.
- **FR-005**: Launching the installed application, and the session service it starts, MUST NOT open
  a console window.
- **FR-006**: The installer MUST register the application in Windows' standard installed-apps list
  with its name, version, and publisher, and uninstalling from there MUST remove all installed
  program files and shortcuts.
- **FR-007**: Uninstalling MUST NOT delete the user's projects, settings, or other application data.
- **FR-008**: Installing a newer version over an older one MUST replace it in place, leaving exactly
  one installed-apps entry and keeping user data.
- **FR-009**: When the application or its session service is running, upgrade and uninstall MUST
  detect it, tell the user that running sessions will end, and proceed only after confirmation or
  once the application is closed. They MUST NOT leave a partially replaced installation.
- **FR-010**: The installer MUST be published unsigned: no certificate, signing secret, or paid
  signing service is involved, so a contributor who builds it locally gets an artifact that is
  trusted exactly as much as the released one. The install guide MUST show the "Windows protected
  your PC" warning a downloaded unsigned installer triggers and name the steps to continue
  ("More info" → "Run anyway"). Code signing may come in a later feature.
- **FR-011**: The installer MUST NOT enable anything that starts automatically at login. This
  matches the Linux package, which ships its service units but does not enable them.
- **FR-012**: The installer MUST NOT include the development-only component showcase. The existing
  automated check that keeps the showcase out of the Linux package MUST cover the Windows package
  as well.
- **FR-013**: The version reported by the installed application and shown in the installed-apps list
  MUST equal the release version the installer was built for.
- **FR-014**: The release pipeline MUST build the Windows installer and attach it to the draft
  release. The release MUST NOT be published unless that step succeeded.
- **FR-015**: Windows installers MUST be provided for 64-bit Intel/AMD (x64) and 64-bit Arm (ARM64),
  the same two architectures the Linux package covers.
- **FR-016**: Maintainers MUST be able to build the Windows installer locally through the project's
  standard task runner, the same way the Linux package is built today.
- **FR-017**: The install guide MUST gain a Windows section with download links for the release,
  install, upgrade, and removal steps, a list of what is installed, and prerequisites. The
  documentation site's download-link check MUST verify the Windows links against the release's
  actual assets.
- **FR-018**: Every change that could affect the build MUST, in continuous integration on Windows,
  build the Windows package, install it, and confirm the installed application starts its session
  service. The release job must not be the first place a broken package shows up. This mirrors the
  per-change packaging check the macOS feature adds.
- **FR-019**: macOS packaging is out of scope. It is delivered by its own feature
  (`028-macos-package`, PR #284). This feature MUST fit alongside it, not compete with it: the
  Windows artifacts join the same "publish only a complete set of artifacts" release rule, the same
  showcase-exclusion check, and the same per-platform install-guide structure, whichever of the two
  features merges first.
- **FR-020**: On Windows, the application MUST be able to start, connect to, and reconnect to its
  session service running on the user's own computer, with no container involved. This is the
  same behaviour Linux and macOS have today: closing the window leaves sessions running, and
  reopening the window reattaches to them.
- **FR-021**: On Windows, only the user who started the session service MUST be able to connect to
  it. Another account on the same machine, including one that knows where the endpoint is, MUST be
  refused. This is the protection Linux and macOS get from an owner-only directory.
- **FR-022**: On Windows, at most one session service MUST run per user, and starting a second one
  MUST find and reuse the first, as on the other platforms.
- **FR-023**: On Windows, "Restart service" (offered when the application and its service are
  different versions) MUST stop the running service and start a matching one. The installer's
  upgrade path MUST be able to stop the running service too.
- **FR-024**: No background command the application or its session service runs on Windows (version
  control, the environment-include script, the container runtime) MUST open a visible console
  window. The terminals of the sessions themselves are unaffected, since they already render
  inside the application.
- **FR-025**: The Windows leg of continuous integration MUST exercise FR-020 to FR-023 on a real
  Windows machine. Compiling them is not enough.

### Key Entities

- **Windows installer**: A single downloadable file per release and architecture. Its attributes
  are the release version, the target architecture, and its publisher identity. It is attached to
  the release beside the Linux packages.
- **Installation**: What exists on a user's machine after installing: program files (application
  and session service), a Start menu entry, and an installed-apps registration. It is separate from
  **user data** (projects, settings, session state), which outlives it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a clean Windows machine with no developer tooling, a user goes from opening the
  install guide to a running session in under 5 minutes, excluding download time.
- **SC-002**: 100% of releases published after this feature carries a Windows installer for each
  supported architecture, and no release is ever published without one.
- **SC-003**: After uninstalling, none of the application's program files, shortcuts, or
  installed-apps entries remain, and 100% of the user's projects and settings are still present.
- **SC-004**: Upgrading from the previous release to the next one requires no manual cleanup and
  leaves exactly one installed version.
- **SC-005**: Launching the application from the Start menu and starting a session shows no console
  window at any point.
- **SC-006**: The Windows install section of the guide lets a first-time user complete installation
  without consulting any other page, and it no longer states that Windows has no packaged build.
- **SC-007**: Every code-affecting change in continuous integration builds, installs, and
  launches the Windows package, so a packaging break is caught before a release is cut.
- **SC-008**: On Windows, closing the window during a running session and reopening it within 10
  minutes shows the same session still running, with its output intact. This matches the existing
  behaviour on Linux and macOS.
- **SC-009**: A second Windows account on the same machine cannot connect to the first account's
  session service in 100% of attempts.

## Assumptions

- "Same as the Linux package" means parity in user experience (one download, standard install,
  menu entry with icon, clean upgrade and removal, built by the release pipeline). It does not mean
  a Linux-style package format.
- The default install is per-user, into the user's own programs location, with no administrator
  rights needed. A machine-wide install for all users is not required for this feature.
- Distribution through Windows package managers or stores (for example winget or the Microsoft
  Store) is out of scope. The installer is a release asset, like the `.deb`.
- The session service keeps its current Windows behaviour: it survives closing the window but not
  logging out, except in the container placement. Surviving logout on Windows is not added here.
- The installer does not add the application to the command-line path. It is a Start-menu
  application, and the install guide can say where the executable lives for users who want to add
  it themselves.
- ~~The application code already works on Windows, so this feature is only packaging.~~ This turned
  out to be false during planning, as the Context section describes. The Windows session service
  runtime (FR-020 to FR-025) is in scope. The container placement is not changed by this feature.
- The existing application icon assets, including a Windows icon file, are used as they are.
- **Dependency on feature 028 (macOS package, PR #284)**: that PR rewrites the release workflow,
  the showcase-exclusion check, and the install documentation layout, and adds a
  complete-artifact-set release check. Whichever feature merges second must rebase onto the other's
  version of those files. Planning should read PR #284's release-artifacts contract so the Windows
  jobs extend it rather than restate it.
- The Linux package's own name and contents are not changed by this feature.
- The install guide's from-source build instructions stay in place for users who prefer them.
