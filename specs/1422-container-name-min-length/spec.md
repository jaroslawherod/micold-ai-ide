# Feature Specification: Container name minimum length

**Feature Branch**: `feat/1422_container-name-enforce-a-minimum-length-docker`

**Created**: 2026-10-25

**Status**: Draft

**Input**: GitHub issue #1422: "Container name: enforce a minimum length (Docker needs 2+ characters)". Follow-up from #1116 (specs/1116-container-name-default). A defaulted container name of one character (a daemon named `a`) satisfies the 1116 charset and 63-character maximum, but Docker requires at least two characters, so container creation fails. Typed names are not length-checked either. Decide: minimum for defaulted names only or typed names too, and what the user sees when the daemon name is too short.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A one-character daemon name does not produce a container that cannot be created (Priority: P1)

A user adds a container daemon named `a` and leaves the container name empty. Today Save succeeds and the container later fails to start because Docker rejects a one-character name. Instead, Save is refused on the container name field with a message saying the container name needs at least two characters, so the user can type one.

**Why this priority**: The defect is a saved daemon that can never start; refusing at Save is the whole fix.

**Independent Test**: Open the add-daemon form, choose the container runtime, enter daemon name `a`, leave the container name empty, Save. No daemon is created and the container name field shows the minimum-length message.

**Acceptance Scenarios**:

1. **Given** daemon name `a` and an empty container name, **When** the user saves, **Then** Save is refused with a message on the container name field that the name needs at least 2 characters, and no daemon is created.
2. **Given** the refusal above, **When** the user types container name `ab` and saves, **Then** the daemon is created with container name `ab`.
3. **Given** daemon name `ab` and an empty container name, **When** the user saves, **Then** the daemon is created with container name `ab`.
4. **Given** daemon name `a!` (sanitizes to one character, `a`) and an empty container name, **When** the user saves, **Then** Save is refused with the same message, because the length is checked on the name that would be stored.

---

### User Story 2 - The field shows no placeholder for a name Save would refuse (Priority: P2)

While the daemon name is too short, the container name field does not offer it as the name Save would use.

**Why this priority**: The 1116 placeholder promises "the name Save would use"; showing `a` would promise something Save refuses.

**Independent Test**: In the add form with container runtime, type daemon name `a`: the container name field shows no placeholder. Type `ab`: the placeholder shows `ab`.

**Acceptance Scenarios**:

1. **Given** the container name is empty and the daemon name sanitizes to one character, **When** the form is shown, **Then** the placeholder is hidden.
2. **Given** the daemon name is then extended to `ab`, **When** the form updates, **Then** the placeholder shows `ab`.

---

### User Story 3 - A typed one-character container name is refused too (Priority: P2)

A user types container name `x`. Docker would reject it just the same, so Save is refused with the same message instead of storing a daemon that cannot start.

**Why this priority**: Same failure, same cause; refusing only defaulted names would leave the typed path broken. Marked for clarification below.

**Independent Test**: Choose the container runtime, enter daemon name `build`, type container name `x`, Save. Save is refused with the minimum-length message on the container name field.

**Acceptance Scenarios**:

1. **Given** container name typed as `x`, **When** the user saves, **Then** Save is refused with the minimum-length message on the container name field.
2. **Given** container name typed as ` x ` (surrounding whitespace), **When** the user saves, **Then** the length is measured after trimming, so it is refused.
3. **Given** container name typed as `xy`, **When** the user saves, **Then** the daemon is created.

---

### User Story 4 - Existing saved daemons keep working (Priority: P1)

A daemon saved earlier with a one-character container name is not changed or hidden by this feature. It is refused only if the user edits and saves it with that name still in place.

**Why this priority**: Stored data must not be rewritten or dropped silently.

**Independent Test**: Load a registry containing a daemon with container name `a`. It loads and lists unchanged. Open it for editing and Save without changes: the minimum-length message appears on the container name field.

**Acceptance Scenarios**:

1. **Given** a stored daemon with container name `a`, **When** the registry loads, **Then** the daemon is present and its container name is `a`.
2. **Given** that daemon opened for editing, **When** the user saves with the container name unchanged, **Then** Save is refused with the minimum-length message on the container name field.

---

### Edge Cases

- Empty input: an empty container name with an empty daemon name keeps the existing "A daemon needs a name." message; an empty container name whose default is empty keeps the 1116 "Enter a container name." message. The minimum-length message appears only when something non-empty but too short would be stored.
- Failure: a refusal never creates or changes a daemon.
- Length is counted in characters of the trimmed name, typed or defaulted. The charset and the 63-character maximum bound defaulted names only (1116); typed names have no charset check.
- The 63-character maximum of 1116 is unchanged; the minimum applies after truncation and stripping, so a name that sanitizes to one character is refused.
- Uniqueness (1116 FR-005) still applies to names that pass the minimum; for a too-short name the minimum-length message is shown, not a duplicate message.
- Concurrency (Principle II): the check is a pure rule on the name being saved and involves no shared state, so two sessions saving cannot disagree about it; the registry's uniqueness check at commit is unchanged.
- Cross-platform (Principle VI): the same minimum on every OS, so a daemon's validity does not depend on the platform that created it.
- A legacy one-character daemon also cannot be saved for an unrelated edit (port, image) until its container name is fixed; this is intentional, and the user fixes the name in the same save. The user learns of it on Save, not at load.
- Runtimes other than container (host, SSH, Kubernetes) have no container name and are unchanged.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Saving a container daemon whose resolved container name (defaulted or typed, trimmed) is non-empty and shorter than 2 characters MUST be refused with a message on the container name field, and MUST NOT create or change a daemon.
- **FR-002**: The refusal message MUST state that the container name needs at least 2 characters.
- **FR-003**: The default container name derived from a daemon name MUST be treated as unusable when it is shorter than 2 characters: Save refuses it per FR-001 and the placeholder (1116 FR-007) is hidden.
- **FR-004**: A typed container name MUST be checked against the same minimum as a defaulted one. [NEEDS CLARIFICATION: typed names too, or defaulted names only? Recommended: typed too, same failure and same cause.]
- **FR-005**: The placeholder MUST be shown exactly when Save would accept the default as the container name.
- **FR-006**: Daemons already stored with a one-character container name MUST load unchanged; the minimum applies only when saving.
- **FR-007**: The minimum-length check MUST NOT change the outcome for names of 2 to 63 characters, nor for other runtimes (1116 FR-009 stays true).
- **FR-008**: The message for a too-short name MUST be distinct from "Enter a container name." and name the minimum. [NEEDS CLARIFICATION: exact wording and whether a too-short daemon name is explained on the container name field only. Recommended: field only, "A container name needs at least 2 characters."]

### Key Entities

- **Container daemon**: a daemon with the container runtime; its container name is unique across daemons and at least 2 characters; a defaulted one is also restricted to a charset and at most 63 characters.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of saves of a container daemon, the stored container name is 2 to 63 characters long, or Save is refused with a message on the container name field.
- **SC-002**: Zero new daemons are saved that Docker would reject for length.
- **SC-003**: A user who hits the refusal can correct it by typing a name and saving again, with no other field changed.
- **SC-004**: No existing stored daemon is altered or dropped by loading the registry.

## Assumptions

- Docker's rule is a name of at least 2 characters (`[a-zA-Z0-9][a-zA-Z0-9_.-]+`); Podman accepts the same names. This comes from the Docker documentation, not from existing code.
- The 2-character minimum is a fixed value, not a setting.

## Out of Scope

- Rewriting or migrating stored daemons that already have a one-character container name.
- Padding or otherwise extending a too-short default (for example `a` to `a-`): the name stays predictable and the user chooses.
- Other container-name rules (charset of typed names, reserved names) beyond length.
- Defaults for other fields.
