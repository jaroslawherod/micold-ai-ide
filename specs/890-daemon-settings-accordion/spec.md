# Feature Specification: Daemon settings in titled sections, every daemon kind, one look

**Feature Branch**: `claude/project-thread-rwlog8`

**Created**: 2026-10-18

**Status**: Draft

**Input**: Issue #890, "Group daemon settings sections in an accordion (new material component)":
with #804 the container (Session service) settings live in each daemon's form in Settings › Daemons,
in several sections, which makes the form long; place them in an accordion, introducing a reusable
one in the material components (collapsible section with a header, expanded/collapsed state,
keyboard and focus handling), then use it in the daemon form. The user added when starting this
run: "do UI refinment and unification, split it for proper sections" — also refine and unify the
daemon form's look (spacing, field styles, headers) across sections and split the settings into
proper, well-named sections, delivered as separate milestones, each its own PR.

## Context: what already exists

Feature 804's amendment M4 (tracked under this issue) already delivered part of the issue:

- a shared titled collapsible section component in the material components (header with an
  open/closed chevron, pointer and Space/Enter toggling, focus ring, light and dark themes, closed
  sections offer no focus stops, listed in the component showcase), built on the existing
  downward-revealing panel;
- the **local container** daemon's container settings grouped in five such sections — Container,
  Credentials, Network, Limits, Sessions — with Container open on arrival, a section forced open
  while a validation error stands in it, and toggling never touching a value.

What remains, and what this feature covers:

- The rest of a container daemon's form (name, where it runs, container name, port, connect at
  start) sits outside any section, above and below the five.
- The **Kubernetes** form is one long flat column under plain muted labels (Kubernetes, Resources,
  Placement and storage, Allowed credentials) with no sections; the **SSH** and **host** forms are
  flat too. The same idea (what credentials reach the sandbox, resource limits) is named
  differently per kind.
- Helper text is placed inconsistently: some fields carry their explanation as the field's own
  supporting text, others have a separate free-floating line above or below them; spacing between
  a heading and its fields, and between groups, differs by kind.
- The section header shows nothing of its contents while closed, so a user must open a section to
  see what it holds.

## Clarifications

### Session 2026-10-18

- Q: Does a required field shown as missing because the user left it empty (feature 937 FR-006), not only after a refused save, keep its section open? → A: Yes. Any required error shown on a field (left empty, or a refused save) counts as a shown error under FR-004. _(agent-resolved: specs/937-required-field-markers/spec.md#FR-006; crates/micold-core/src/required.rs#FieldRequirement::Missing)_
- Q: Should the Kubernetes "Image" section take the container daemon's "Container" name, since both hold the image and AI CLIs? → A: No. FR-003's shared names are Connection, Credentials, Resources and Sessions; the container's "Container" section also chooses the runtime, which a pod lacks, and keeps 804's name. _(agent-resolved: specs/804-session-service-in-daemons/spec.md#FR-012)_
- Q: What exact summary text does each section show? → A: The plan's UI contract fixes each section's summary; the spec keeps FR-014's rule and examples. _(agent-resolved: deferred to plan)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Every daemon kind's form is split into well-named sections (Priority: P1)

A user adding or editing a daemon of any kind (host, container, SSH, Kubernetes) sees, under the
form's title, the fields that identify the daemon, then the rest of its settings as a short list of
titled collapsible sections named for what they control. The same concept carries the same section
name in every kind that has it.

**Why this priority**: this is the issue's core ask ("split it for proper sections") applied to the
forms 804 left flat; the Kubernetes form is today the longest form in the app.

**Independent Test**: open the add form, switch *Runs in* through each kind, and check the section
list each kind shows against FR-002; open and close each section with the pointer and the keyboard;
save a daemon of each kind and reopen it.

**Acceptance Scenarios**:

1. **Given** the add-daemon form, **When** *Runs in* is set to any kind, **Then** the name and *Runs
   in* fields stay at the top outside any section, and every other setting of that kind is inside
   exactly one titled section listed in FR-002, in that order.
2. **Given** a daemon form of any kind, **When** it opens, **Then** its first section is open and the
   others are closed.
3. **Given** a Kubernetes daemon form, **When** the user opens the section that controls what
   reaches the pod, **Then** it is named the same as the container daemon's equivalent section
   ("Credentials"), and the resource fields' section carries the same name in both kinds.
4. **Given** a field with a validation error in a closed section of any kind, **When** the form shows
   that error, **Then** the section holding it is open.
5. **Given** values typed in any section, **When** that section is closed and opened again, or the
   form is saved, **Then** the values are kept and saved exactly as before this feature.

---

### User Story 2 - One look across all daemon forms (Priority: P2)

A user moving between daemon kinds sees one consistent form: the same spacing between fields, the
same gap between sections, the same header style, and each field's explanation in the same place.

**Why this priority**: the user's "UI refinement and unification" ask; it is polish on top of US1.

**Independent Test**: open the form for each kind with every section open, and measure field
spacing, section gaps and helper-text placement against FR-006 to FR-009 (geometry checks), then a
visual pass in both themes.

**Acceptance Scenarios**:

1. **Given** any two daemon forms, **When** compared, **Then** the vertical gap between consecutive
   fields within a section is the same, and the gap between sections is the same.
2. **Given** a field that has an explanation, **When** shown, **Then** the explanation is the field's
   own supporting text directly under it, not a separate line elsewhere — except an explanation that
   concerns a whole section, which is the first line of that section's body.
3. **Given** a group of checkboxes inside a section (AI CLIs, allowed credentials), **When** shown,
   **Then** the group has a short label above it in one shared style, and the checkboxes align to the
   same left edge as the section's text fields.
4. **Given** the Kubernetes context chooser, **When** contexts are found in the kubeconfig, **Then**
   the user picks one from a select above the *Context* field, as the SSH form's host select sits
   above its *Host* field, rather than from a column of full-width buttons, and can still type a
   context by hand in the *Context* field.
5. **Given** light and dark themes, **When** the forms are shown, **Then** both follow the theme with
   no fixed colours.

---

### User Story 3 - A closed section says what it holds (Priority: P3)

A user scanning a long form sees, on each closed section's header, a one-line summary of its current
settings (for example "Docker · Basic image" or "No outbound connections"), and a mark when a field
inside it needs attention, so they open only the section they need.

**Why this priority**: refines the shared component so the accordion shortens the form without
hiding what it holds; useful to any later settings view that adopts the component.

**Independent Test**: in the component showcase and in a daemon form, check a closed section with a
summary, an open one (summary hidden), and one with a required field still empty.

**Acceptance Scenarios**:

1. **Given** a section given a summary, **When** it is closed, **Then** the summary shows on one line
   under or beside the title, cut with an ellipsis if too long; **When** it is open, **Then** the
   summary is not shown.
2. **Given** a section without a summary, **When** shown, **Then** it looks exactly as today.
3. **Given** a section holding a required field that is still empty, **When** it is closed, **Then**
   its header shows the same required marker the form's fields use.
4. **Given** the component showcase, **When** opened, **Then** the section is listed closed with a
   summary, open, and closed with the marker.

---

### Edge Cases

- Switching *Runs in* to another kind shows that kind's sections in their default state (first
  open); fields of the kind left behind are not shown and are dropped on save exactly as today.
- SSH has a single section (Connection): it is open on arrival and, like every section, can be
  closed; the header keeps the SSH form consistent with the others.
- Two sections each holding an error, or a required field showing its required error (left empty,
  or after a refused save): both are open.
- Kubernetes contexts failed to load: the failure message shows inside the section that holds the
  context field; typing a context by hand still works.
- No contexts found or not yet loaded: the context field alone shows, as today.
- A summary that depends on a value being typed updates as the user types.
- Narrow window: headers, summaries and fields wrap or ellipsize; nothing overflows the form.
- Sessions running on the daemon being edited (Principle II): opening, closing or editing sections
  does nothing to them; only a save applies changes, as today.
- Platforms (Principle VI): the forms and sections look and behave the same on macOS, Linux and
  Windows; a kind not offered on a platform (e.g. no local container runtime) shows no sections for
  it, as the *Runs in* choices already decide today.

## Requirements *(mandatory)*

### Functional Requirements

**Sections**

- **FR-001**: Every daemon form MUST show, under its title and required-field legend, the daemon's
  name and *Runs in* fields outside any section, then every other setting of the chosen kind inside
  exactly one titled collapsible section, then the save and cancel actions outside any section.
- **FR-002**: The sections per kind MUST be, in this order:
  - **Container**: Connection (container name, port, connect at start), Container (runtime, image,
    AI CLIs), Credentials, Network, Resources (the limits), Sessions (survive-logout).
  - **Kubernetes**: Connection (context, namespace, connect at start), Image (image, AI CLIs),
    Credentials (allowed credentials and their files), Resources (requests and limits), Placement
    and storage (node selector, tolerations, volume size, storage class).
  - **SSH**: Connection (host chooser, host, user, port, key file, connect at start).
  - **Host**: Connection (connect at start), Sessions (the survive-logout note).
- **FR-003**: The same concept MUST have the same section name in every kind: "Connection",
  "Credentials", "Resources", "Sessions". The container daemon's "Limits" section is renamed
  "Resources".
- **FR-004**: When a form opens or *Runs in* changes, the kind's first section MUST be open and the
  others closed. A section holding the field of a shown validation error MUST be open (as 804
  FR-012 does for container sections, extended to every section of every kind). A required field
  showing its required error (left empty by the user, or after a refused save; feature 937 FR-006)
  counts as a shown error: every section holding such a field is open.
- **FR-005**: Opening or closing a section MUST NOT change any field value, the draft, or what is
  saved; every field's validation and save behaviour MUST stay as it is today.

**Unified look**

- **FR-006**: Within a section, consecutive controls MUST be separated by one shared vertical gap,
  the same in every section and every kind; consecutive sections MUST be separated by one shared
  gap.
- **FR-007**: An explanation that concerns one field MUST be that field's own supporting text. An
  explanation that concerns the whole section MUST be the first line of the section's body, in one
  shared muted style. No free-floating explanation line between two fields. Cautions and warnings stay directly
  under the control they concern (e.g. the unshared sign-in caution under its share, 804 FR-004g; a
  node-selector or toleration error under its editor).
- **FR-008**: A group of checkboxes, and a multi-line editor (node selector, tolerations), MUST have
  a group label above it in one shared style, and align with the section's other controls. An
  explanation of the whole group (e.g. the AI CLI note, "Nothing reaches the pod unless it is ticked
  here.") goes directly under that group label.
- **FR-009**: The Kubernetes contexts found in the kubeconfig MUST be offered in a select placed
  above the *Context* text field, shown only when contexts were found, following the SSH form's host
  select over its *Host* field; choosing one fills the *Context* field, which stays for typing. It
  replaces today's one button per context. No new combined select-and-type component is added.
- **FR-010**: The plain muted group labels the daemon forms use today as section headings MUST be
  gone from the daemon forms; every heading in them is a section header.

**Shared section component**

- **FR-011**: The shared titled section MUST accept an optional one-line summary shown only while
  closed, ellipsized when longer than the header's width, in a muted style under the title.
- **FR-012**: The shared titled section MUST accept an optional required marker shown on its header
  while closed, in the same style as the form fields' required marker.
- **FR-013**: The summary and marker MUST NOT change the header's keyboard, pointer and focus
  behaviour; a section without them MUST look as it does today. Both MUST follow the light and dark
  themes.
- **FR-014**: The daemon forms MUST give each section a summary of its current values where one fits
  in a line (for example, Container: runtime and image kind; Network: the posture; Resources: the
  limits set, or "Runtime defaults"), and the required marker when a required field inside it is
  empty.
- **FR-015**: The component showcase MUST list the section closed with a summary, open, and closed
  with the required marker.

**Delivery**

- **FR-016**: The work MUST ship as separate milestones, each its own pull request: (1) the shared
  section's summary and marker (US3's component part), (2) every kind's form split into the FR-002
  sections (US1), (3) the unified look and the summaries in the daemon forms (US2, FR-014).
- **FR-017**: The user guide pages that describe a daemon's form MUST name the sections as FR-002
  does.

### Key Entities

- **Form section**: a titled, collapsible group of a daemon form's fields; has a name, an open/closed
  state held by the form for as long as it is shown (not persisted), an optional summary and an
  optional required marker.
- **Daemon kind**: host, container, SSH, Kubernetes; decides which sections the form shows.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every daemon kind, 100% of its settings other than name and *Runs in* sit inside a
  titled section; no daemon form shows a plain group label as a heading.
- **SC-002**: With default section states, a Kubernetes daemon form fits its section list and actions
  in one window height of 800 logical pixels, where today it needs several.
- **SC-003**: The gap between consecutive fields and the gap between sections each take a single
  value across all four kinds (geometry checks).
- **SC-004**: Every daemon of every kind saved before this feature opens, edits and saves with the
  same stored values afterwards (no stored-format change).
- **SC-005**: A user can reach and toggle every section with the keyboard alone, and Tab never stops
  on a field inside a closed section.
- **SC-006**: A visual pass of all four forms, in both themes, records no inconsistent spacing,
  misplaced helper text or clipped header.

## Assumptions

- The shared titled section component from feature 804 M4 is the issue's "new material component";
  it is extended (summary, marker), not rebuilt. Its keyboard, focus and reveal behaviour stays.
- Which sections are open is view state for as long as the form is shown, not persisted (as in 804).
- The remote-daemon repository step and the daemon list rows are out of scope; only the add/edit
  form changes.
- No settings are added, removed or change meaning; the stored daemon registry format is unchanged.
- Adopting the section component in other settings views (GitHub, Jira, Environment…) is out of
  scope; the component stays reusable for them.
- The milestone order can change at planning if a later milestone's work turns out to be needed
  earlier, but each stays its own PR.
