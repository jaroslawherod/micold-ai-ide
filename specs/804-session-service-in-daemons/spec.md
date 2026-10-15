# Feature Specification: Session service settings in the Daemons manager

**Feature Branch**: `feat/804_move-the-session-service-settings-container`

**Created**: 2026-10-15

**Status**: Draft

**Input**: User description: "Move the Session service settings (container runtime, limits) into the Daemons manager (#804)"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Configure a daemon's container settings from the daemon manager (Priority: P1)

A user opens Settings, then Daemons, selects a daemon and finds, with that daemon, where its
sessions run, the container runtime, what the sandbox may reach and its limits. They no longer
visit a second section.

**Why this priority**: it is the whole point of the move; the settings belong to a daemon.

**Independent Test**: open Settings > Daemons, edit a daemon's container runtime and a limit, save,
and see the daemon restart with the new values.

**Acceptance Scenarios**:

1. **Given** a container daemon, **When** the user opens its entry in Daemons, **Then** the
   placement, container runtime, image, credential shares, network posture and limits are shown and
   editable for that daemon.
2. **Given** two daemons with different limits, **When** the user views each, **Then** each shows
   its own values and editing one leaves the other unchanged.
3. **Given** a change to these settings, **When** the user saves, **Then** the same
   restart-on-save behaviour applies as before the move.

---

### User Story 2 - The standalone section is gone (Priority: P1)

The settings rail no longer lists Session service, and nothing points to it.

**Why this priority**: two homes for one setting is the problem being fixed.

**Independent Test**: open Settings and inspect the rail and every text that named the old section.

**Acceptance Scenarios**:

1. **Given** the Settings window, **When** the user looks at the rail, **Then** there is no
   Session service entry.
2. **Given** a limit is exceeded, **When** the sandbox hint is shown, **Then** it names the
   Daemons manager as where to raise the limit.
3. **Given** any daemon shares a credential, **When** the user is in any section, **Then** the
   Daemons rail entry shows the *Sharing* badge, and shows none when no daemon shares.
4. **Given** the old limits hint text, **When** a limit is hit, **Then** it names Settings >
   Daemons, and no other user-facing text names Session service.

---

### User Story 3 - Existing settings carry over (Priority: P2)

A user upgrading keeps every value they had set.

**Why this priority**: no one should have to reconfigure.

**Independent Test**: start with a settings file made before the change; the Daemons manager shows
the same values.

**Acceptance Scenarios**:

1. **Given** settings saved before this change, **When** the app starts, **Then** the daemon(s)
   show the same placement, runtime, shares, limits and sign-out survival (rule per FR-005).
2. **Given** the user guide, **When** read, **Then** it describes the daemon manager as the place
   for these settings and has no Session service section.

### Edge Cases

- No daemons configured: the manager offers Add daemon; there is nothing to configure per daemon.
- A host daemon: container-only settings are shown but stay inert, and are kept while the form is
  open if the user switches the daemon back to a container. A saved host daemon stores none (plan R4).
- Invalid value (for example a limit out of range): the error shows beside the field in the
  daemon's form and blocks saving, as before.
- Several windows or sessions running: saving restarts only the edited daemon.
- Remote (SSH) daemon: container settings do not apply and are not shown for it (FR-011); its
  form keeps the host, user, port and key file it has today.
- Windows, macOS and Linux behave the same; only the container runtimes available on each differ.
- Saving a daemon's settings restarts that daemon; its running sessions are affected as they were
  by a restart-on-save before. Other daemons and their sessions are untouched.
- Unsaved edits in two daemons' forms at once are kept independently; saving one does not discard
  the other.
- Removing a daemon discards its container settings; renaming it keeps them.
- No section name is persisted, so nothing remembered can point at the removed section.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Users MUST be able to set, for each daemon, every field the Session service section
  has today, from Settings > Daemons: where sessions run, container runtime, image source,
  reference and archive, credential shares (with the notice for an unshared sign-in), network
  posture and its warning, limits, and whether sessions outlive sign-out.
- **FR-002**: The settings rail MUST NOT list a Session service section.
- **FR-003**: Each daemon MUST keep its own values; editing one daemon MUST NOT change another.
- **FR-004**: Saving these settings MUST restart or re-apply the affected daemon with the same
  effect on its running sessions as the Session service section had before; which control saves
  is FR-010.
- **FR-005**: Values saved before the change MUST appear unchanged: the existing global values
  belong to the one migrated daemon (id 1) only, as feature 491 already migrates them; no other
  daemon is seeded. A daemon added later starts from the defaults, not a copy.
- **FR-006**: All user-facing text that pointed at Settings > Session service MUST point at the
  Daemons manager.
- **FR-007**: The rail MUST still signal, from any section, that credentials are being shared.
- **FR-008**: Validation errors for these fields MUST show beside the field and block saving.
- **FR-009**: The user guide MUST describe the new location and drop the Session service section.
- **FR-010**: The container fields MUST be saved by the Daemons form's own save, with its existing
  restart confirmation when the daemon is running; they MUST NOT keep a separate page Save.
- **FR-011**: For a daemon whose runtime is SSH, the container fields MUST NOT be shown and its
  an unedited entry's stored settings MUST be left as they were; switching a container daemon to SSH
  and saving drops its container settings (plan R4).

### Key Entities

- **Daemon**: a configured session service with a name, placement and the container settings.
- **Container settings**: runtime, image, credential shares, network posture, limits of a daemon.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can change a daemon's container runtime and a limit without leaving
  Settings > Daemons (zero other sections visited).
- **SC-002**: The settings rail lists no Session service entry, and no user-facing text names it.
- **SC-003**: 100% of values saved before the change appear unchanged afterwards.
- **SC-004**: The user guide has no stale reference to Session service.

## Clarifications

### Session 2026-10-15

- Q: Do existing global values seed every daemon or only the migrated one, and does a later daemon copy? → A: only the migrated daemon (id 1); a later daemon starts from defaults. _(agent-resolved: specs/491-multi-session-daemons/research.md#R5-migration)_
- Q: Which save do the fields use? → A: the Daemons form's own save with its running-daemon confirmation. _(agent-resolved: crates/micold-client/src/features/daemons.rs `save`)_
- Q: Do container settings apply to an SSH daemon? → A: no; SSH settings are a target only, so the fields are hidden. _(agent-resolved: specs/489-ssh-remote-daemon/spec.md; crates/micold-client/src/features/daemons.rs `DaemonRuntime::Ssh`)_

## Assumptions

- The "Session service" is the single legacy entry; after the move its settings are those of
  the daemon that entry was migrated to.
- Fields, defaults and validation are unchanged; only their location changes.

## Out of Scope

- New container settings, new runtimes, changes to what a limit means.
- Remote daemon behaviour beyond the clarification above.
