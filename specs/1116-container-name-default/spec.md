# Feature Specification: Container daemon name defaults to the daemon name

**Feature Branch**: `feat/1116_container-daemon-default-the-container-name-to-the`

**Created**: 2026-10-23

**Status**: Draft

**Input**: GitHub issue #1116: "Container daemon: default the container name to the daemon name when none is specified". When creating a container daemon the container name is required ("Enter a container name."). If it is not specified, use the daemon's name as the container name instead of blocking Save.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Save a container daemon without typing a container name (Priority: P1)

A user adds a container daemon, types only a daemon name and leaves the container name field empty. Save works, and the daemon's container is named after the daemon.

**Why this priority**: This is the whole request; the container name is today the one field the user must invent although the daemon name already identifies the daemon.

**Independent Test**: Open the add-daemon form, choose the container runtime, enter the name `build box`, leave the container name empty, Save. The daemon is created and its stored container name is `build-box`.

**Acceptance Scenarios**:

1. **Given** the container runtime is chosen, the daemon name is `devbox` and the container name is empty, **When** the user saves, **Then** the daemon is created with container name `devbox` and no "Enter a container name." message appears.
2. **Given** the container name field is empty, **When** the form is shown, **Then** the field is no longer marked required and shows the name that would be used (the sanitized daemon name) as placeholder or hint.
3. **Given** the user typed a container name `my-ctr`, **When** they save, **Then** the container name is `my-ctr`, not the daemon name.

---

### User Story 2 - The resolved name is fixed at save (Priority: P2)

After saving, the container name is a stored value. Renaming the daemon later does not rename the container.

**Why this priority**: A container runtime holds state under the container name; a silent rename would orphan it.

**Independent Test**: Save a daemon `devbox` with an empty container name, rename the daemon to `devbox2`, save. The container name is still `devbox`.

**Acceptance Scenarios**:

1. **Given** a saved container daemon whose container name was defaulted to `devbox`, **When** the user renames the daemon to `devbox2` and saves, **Then** the container name is still `devbox`.
2. **Given** that same daemon is opened for editing, **When** the form is shown, **Then** the container name field shows `devbox` as a typed value, not as a placeholder.

---

### User Story 3 - A daemon name that is not a valid container name is made valid (Priority: P2)

Daemon names are free text; container names are limited to the runtime's charset. The default is the daemon name converted to a valid container name.

**Why this priority**: Without it, names with spaces or punctuation would make the default unusable.

**Independent Test**: Save daemons named `Build Box!`, `日本語`, `  web  ` and `_web.` with an empty container name. The stored container names are `Build-Box`, (refused), `web` and `web`.

**Acceptance Scenarios**:

1. **Given** daemon name `Build Box!` and an empty container name, **When** saved, **Then** the container name consists only of letters, digits, `_`, `.` and `-`, starts with a letter or digit, and is derived from the name (`Build-Box`). A name `_web.` gives `web`.
2. **Given** a daemon name with no usable character (e.g. `日本語` or `!!!`) and an empty container name, **When** the user saves, **Then** Save is refused with a message on the container name field asking the user to enter one.
3. **Given** a daemon name with surrounding whitespace, **When** saved with an empty container name, **Then** the surrounding whitespace is ignored (`  web  ` gives `web`).

---

### User Story 4 - Uniqueness is still enforced (Priority: P1)

The defaulted name goes through the same check against other daemons' container names as a typed one.

**Why this priority**: Two daemons sharing a container would corrupt each other.

**Independent Test**: With a daemon whose container name is `devbox`, add a second container daemon named `devbox` with an empty container name.

**Acceptance Scenarios**:

1. **Given** another daemon already uses container name `devbox`, **When** the user saves a new daemon named `devbox` with an empty container name, **Then** Save is refused with the existing "That container is already used by daemon …" message on the container name field and no daemon is created.
2. **Given** the refusal above, **When** the user types a different container name and saves, **Then** the daemon is created.

---

### Edge Cases

- Container name filled with only whitespace: treated as empty (defaults).
- Daemon name also empty: the existing "A daemon needs a name." message wins; no container name is derived.
- Sanitizing yields a name equal to another daemon's container name (`a b` and `a-b`): refused as a duplicate, never auto-suffixed.
- Sanitized name very long: truncated to the runtime's length limit; if truncation collides, refused as a duplicate.
- Concurrency (Principle II): two sessions saving daemons whose names sanitize to the same container name: the registry's uniqueness check applies at commit, so one save wins and the other is refused with the duplicate message.
- Cross-platform (Principle VI): the same sanitization on every OS, so a daemon's container name does not depend on the platform that created it.
- Daemon name empty while the container name is empty: the placeholder is hidden.
- Runtimes other than container (host, SSH, Kubernetes) have no container name and are unchanged.
- Existing saved daemons keep their stored container name unchanged.

## Clarifications

### Session 2026-10-23

- Q: On a collision of the defaulted name, refuse or suffix? → A: Refuse with the existing duplicate message on the container name field; no suffix, so the name stays predictable. _(agent-resolved: specs/1116-container-name-default/spec.md#Assumptions, FR-005, FR-010; crates/micold-client/src/features/daemons.rs `ContainerInUse` already reports on `container_name`)_
- Q: Length limit for the defaulted name? → A: 63 characters, a conservative cap below the runtimes' limits; it only bounds defaulted names. _(agent-resolved: specs/1116-container-name-default/spec.md#Assumptions, FR-002)_
- Q: Are typed names charset-checked too? → A: No; typed names keep today's validation (non-empty only, `invalid_field("container_name")`), per the issue's scope. _(agent-resolved: specs/1116-container-name-default/spec.md#Out of Scope, FR-004, FR-011; crates/micold-client/src/features/daemons.rs `invalid_field`)_

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Saving a container daemon with an empty or whitespace-only container name MUST succeed when the daemon name yields a valid container name, and MUST store that name as the container name.
- **FR-002**: The default MUST be the daemon name, trimmed, with every character outside ASCII letters, digits, `_`, `.` and `-` replaced by `-`, runs of replaced characters collapsed to one, then truncated to 63 characters, then stripped of leading characters that are not a letter or digit and of trailing `-`, `_` and `.`. Example: `_web.` gives `web`.
- **FR-003**: When the default would be empty, Save MUST be refused with a message on the container name field asking for a name.
- **FR-004**: A typed container name MUST be used as typed (trimmed) and MUST NOT be replaced by the default.
- **FR-005**: The uniqueness check against other daemons' container names MUST apply to the resolved name, with the existing message and field.
- **FR-006**: The resolved name MUST be stored with the daemon; renaming the daemon later MUST NOT change it.
- **FR-007**: The container name field MUST NOT be flagged as required, and when empty MUST show the name that Save would use, updating as the daemon name is edited.
- **FR-008**: Editing a saved container daemon MUST show its stored container name as the field's value.
- **FR-009**: Daemons of other runtimes and already saved daemons MUST be unaffected.
- **FR-010**: A duplicate refusal of a defaulted name MUST be shown on the container name field, not on another field.
- **FR-011**: A defaulted name MUST never be stored unless it matches the charset and length of FR-002; typed names keep today's validation.

### Key Entities

- **Container daemon**: a daemon with the container runtime; has a name (free text) and a container name (restricted charset, unique across daemons).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can create a container daemon by filling in only its name, with one fewer required field than before.
- **SC-002**: For every daemon name left to default, the stored container name is either valid for the runtime or Save is refused with a message on the field; zero defaulted saves store an invalid container name.
- **SC-003**: In 100% of renames of a defaulted daemon, the container name is unchanged.
- **SC-004**: No save ever produces two daemons with the same container name.

## Assumptions

- The charset `[a-zA-Z0-9][a-zA-Z0-9_.-]*` comes from the Docker and Podman documentation, not from existing code; 63 characters is a deliberately conservative limit chosen here. Nothing validates this today, so this feature adds the check for defaulted names only.
- A collision on the defaulted name is reported, not resolved by appending a suffix, so the container name stays predictable.
- The placeholder text mechanism for form fields already exists; reusing it is acceptable.

## Out of Scope

- Renaming an existing container when its daemon is renamed.
- Defaults for other fields (port, image).
- Changing validation of typed container names beyond what exists today.
