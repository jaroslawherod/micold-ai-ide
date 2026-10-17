# Feature Specification: Required-field markers in forms

**Feature Branch**: `claude/project-thread-rkhzho`

**Created**: 2026-10-17

**Status**: Draft

**Input**: Issue #937, "Forms show which fields are required, following Material Design". Forms in
the app do not show which fields are required; the user finds out only when validation fails. Mark
required fields in every form, following the Material Design rules for required fields in text
fields and forms: mark them consistently, explain the marker once per form, show an error state
with supporting text when a required field is left empty, do not rely on colour alone, and do it in
the material components so every form gets it. Acceptance: every required field in the app's forms
(e.g. Settings › Daemons, create worktree, create session) is visibly marked as required; the marker
and its explanation follow the Material Design guidance and are consistent across forms; an empty
required field shows the error state with supporting text.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See which fields must be filled before submitting (Priority: P1)

A user opens a form (the add-worktree form and the session it starts, the add-daemon form in
Settings › Daemons, the run-in-parallel dialog, a rename dialog) and can tell, before typing anything,
which fields the form will not accept empty.

**Why this priority**: This is the problem the issue reports. Today the user learns that a field is
required only from a failed submit.

**Independent Test**: Open each form in the inventory (FR-009) and check, without submitting, that
every required field carries the required marker in its label and that no optional field does.

**Acceptance Scenarios**:

1. **Given** the add-worktree form is open, **When** the user looks at it, **Then** every field the
   form will not accept empty (e.g. Name) carries the required marker in its label, and Ticket,
   which is optional, does not.
2. **Given** Settings › Daemons with a new daemon draft, **When** the user looks at the form,
   **Then** Name and every other field the daemon cannot be saved without carry the marker, and Port
   (a free port is chosen when empty) does not.
3. **Given** a field is required only for one choice of another field (e.g. the SSH host when the
   daemon runs over SSH), **When** the user switches that choice, **Then** the marker appears or
   disappears with it.
4. **Given** a field shows the marker, **When** the user reads its label as plain text (screen
   reader, monochrome display, any colour scheme), **Then** the field is still identifiable as
   required, because the marker is a character in the label, not only a colour.

---

### User Story 2 - Understand what the marker means (Priority: P2)

A user who has never seen the marker reads, once per form, what it means.

**Why this priority**: Material Design asks for the convention to be stated; without it the marker
is a symbol the user has to guess at. It adds no value without Story 1.

**Independent Test**: Open each form that has at least one required field and check that it shows
the explanation exactly once, and that a form without required fields shows none.

**Acceptance Scenarios**:

1. **Given** a form with one or more required fields, **When** it is open, **Then** the form shows
   one explanation of the marker, "* required", in the same wording and the same position
   relative to its fields as every other form.
2. **Given** a form with no required fields (e.g. Settings › Appearance), **When** it is open,
   **Then** no explanation is shown.

---

### User Story 3 - An empty required field says what is wrong (Priority: P2)

A user leaves a required field empty and the field itself shows the error state, with supporting
text saying it is required, instead of a separate message elsewhere in the form.

**Why this priority**: The third acceptance criterion of the issue. It replaces a message the user
has to connect to a field with one on the field.

**Independent Test**: In each form, leave one required field empty, trigger the error (FR-006), and
check that the field shows the error state with its supporting text replaced by the required
message, and that the message names the problem in words.

**Acceptance Scenarios**:

1. **Given** the add-worktree form with an empty Name, **When** the error state is triggered
   (FR-006), **Then** the Name field shows the error state and its supporting text reads that the
   field is required, and the form is not submitted.
2. **Given** a required field in the error state, **When** the user types a value into it, **Then**
   the error state clears without the field losing focus or the typed text.
3. **Given** a field that already shows another validation error (e.g. a port that is not a
   number), **When** it is not empty, **Then** that error is shown, not the required message.

---

### Edge Cases

- **Whitespace only**: a required field holding only spaces counts as empty wherever the form's own
  validation trims the value; the marker and the error follow the form's validation, never a
  different rule.
- **Conditionally required fields**: a field required only for some value of another field shows
  the marker only while it is required, and a hidden field never counts toward the form's
  explanation.
- **Fields that are not text fields**: a required select (e.g. a daemon or AI CLI picker with no
  default) carries the same marker and error state as a text field.
- **Several required fields empty at once**: a submit attempt puts every empty required field into
  the error state together, not only the first one the form's validation reports.
- **A field stops being required while in the error state** (e.g. the user switches a daemon from
  SSH back to local): its marker and its required error go away with the requirement.
- **Pre-filled required field**: a required field that opens with a value keeps its marker; the
  marker says what the form needs, not what is missing.
- **Submit failure that is not about emptiness** (a branch collision, an unreachable daemon): shown
  as today; this feature does not change it.
- **Several sessions and windows** (Principle II): each form's marker and error state belong to its
  own draft; an error in one window's or session's form does not appear in another's.
- **Cross-platform** (Principle VI): the marker is a text character rendered with the field's label
  font on Linux and macOS alike, with no platform-specific glyph or font fallback.
- **Label floated or resting**: the marker stays part of the label in both positions of a filled
  field's label and at every text scale the app supports.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every required field in every form MUST carry a required marker as part of its label:
  an asterisk ("*") after the label text. The app uses this one convention in every form; it does
  not switch to marking optional fields in forms where most fields are required, because the issue
  asks for markers that are consistent across forms.
- **FR-002**: A field MUST be marked required exactly when the form refuses to proceed with that
  field empty, given the current values of the form's other fields.
- **FR-003**: The marker MUST NOT depend on colour alone: it MUST be a character in the label, still
  present when the label is read as text.
- **FR-004**: Every form that has at least one required field MUST show one explanation of the
  marker, reading "* required", above the form's first field, in every form. A form with no
  required field MUST NOT show it.
- **FR-005**: A required field left empty MUST show the field's error state with supporting text
  saying that the field is required; the message replaces the field's supporting text, sits on the
  field rather than elsewhere in the form, and states the problem in words, not by colour alone.
- **FR-006**: The required error MUST appear on every empty required field when the user attempts to
  submit the form, and MUST stay while the field remains empty and required. It MUST NOT appear on
  a form the user has just opened and not yet interacted with. [NEEDS CLARIFICATION: Should an
  empty required field also show the error when the user leaves it after focusing it, before any
  submit attempt? Default if unanswered: no, submit attempt only.]
- **FR-007**: The error state MUST clear as soon as the field holds a value, without the field
  losing focus, cursor position or typed text.
- **FR-008**: The marker, the explanation and the required error MUST be provided by the shared
  form components, so a form gets them by declaring a field required, not by drawing its own
  marker, explanation or message.
- **FR-009**: The forms covered are exactly the forms with input fields: the add-worktree form (with
  the fields that start its first session: daemon, AI CLI, first prompt), the run-in-parallel
  dialog, the project and worktree rename dialogs, and the Settings sections Daemons (the
  add/edit-daemon form and the repository binding), Environment, Terminal, GitHub, Jira and the
  container fields of a daemon. In each, the required fields are those FR-002 picks out (e.g. the
  run-in-parallel prompt, type, name and base branch; the daemon's name; a rename's new name); the
  plan MUST list each form with its required fields. A form in which FR-002 picks out no field
  (e.g. Settings › Appearance) gets neither marker nor explanation.
- **FR-010**: A field's own validation error other than emptiness (e.g. an invalid port) MUST take
  precedence over the required error while the field is not empty, and forms MUST keep their
  existing non-emptiness errors.
- **FR-011**: The component showcase MUST show a required field in its resting, filled and error
  states and the marker explanation, so the convention can be checked without opening a form.

### Key Entities

- **Required field**: a form field the form will not accept empty under the current values of its
  other fields. Attributes: its label, whether it is required now, whether it is empty, and whether
  its required error is showing.
- **Marker explanation**: the one line per form that says what the marker means.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of the forms in the FR-009 inventory, every field the form rejects empty
  carries the marker, and no field it accepts empty does.
- **SC-002**: A user can name every required field of any covered form before submitting it, on
  the first look, without triggering a validation error.
- **SC-003**: Every covered form with a required field shows exactly one marker explanation, and
  all of them read the same and sit in the same place, above the form's first field.
- **SC-004**: In every covered form, submitting with each required field empty in turn puts that
  field, and only fields that are empty and required, into the error state with the required
  message.
- **SC-005**: With colour removed (greyscale), every marker and every required error is still
  recognisable from its text.
- **SC-006**: No covered form draws its own marker, explanation or required message: all come from
  the shared components.

## Assumptions

- "Required" is defined by each form's existing validation: this feature makes existing rules
  visible and does not make a field required or optional that is not so today.
- Forms that are pickers or confirmations with no input fields (bind daemon, attach existing,
  confirm dialogs) have no required fields and are out of scope.
- Errors from the daemon or the file system after a submit (collisions, unreachable hosts) are out
  of scope and keep their current presentation.
- The issue's "create session" form is the session part of the add-worktree form (daemon, AI CLI,
  first prompt); the app has no separate create-session form today.
- Translation of the wording is out of scope; the app ships in English.
