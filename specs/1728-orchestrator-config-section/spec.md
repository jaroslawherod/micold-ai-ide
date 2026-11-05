# Feature Specification: `[orchestrator]` config section and `config check` rules

**Feature Branch**: `feat/1728_feat-orchestrator-config-section-and-config-check`

**Created**: 2026-11-05

**Status**: Draft

**Input**: Issue #1728 (part of #1722): extend `micold_core::daemon_config` (#1523) with an
`[orchestrator]` table (`url`, `ca_file`, `cert_file`, `key_file`, `enrollment_token_file`; secrets by
file path only). Add `micold-daemon config check` rules (URL scheme, cert and key given together,
token file or cert, never both silently) and document them in
`docs/user-guide/daemon-configuration.md`. Covers FR-011 of the orchestrator plan (#1722).

## User Scenarios & Testing

### User Story 1 - Point a daemon at an orchestrator (Priority: P1)

An operator adds an `[orchestrator]` table to `daemon.toml` naming the orchestrator's URL, the CA
that signs it, and either a client certificate and key or an enrollment token file.

**Why this priority**: Without the table nothing else in the orchestrator plan can read its
connection settings.

**Independent Test**: Parse a file with a complete `[orchestrator]` table; the resolved view shows
every key, with each secret as a path and state only.

**Acceptance Scenarios**:

1. **Given** a file with `[orchestrator]` `url`, `ca_file`, `cert_file` and `key_file`, **When** it
   is parsed, **Then** the values are available and `config check` exits 0.
2. **Given** a file with an unknown key in `[orchestrator]`, **When** it is parsed, **Then** it is
   refused with `file:line:column: unknown key ...` as for every other table.
3. **Given** a file without `[orchestrator]`, **When** it is parsed, **Then** the daemon runs as
   before and the resolved view shows no orchestrator.

### User Story 2 - Get a precise error for a wrong table (Priority: P1)

An operator who mistypes the table runs `micold-daemon config check` and is told the file, line and
column of each problem.

**Independent Test**: One test per rule feeds a file that breaks only that rule and asserts the
message names `file:line:column`.

**Acceptance Scenarios**:

1. **Given** `url` whose scheme is not `https`, **When** checked, **Then** exit 2 with an error at
   the `url` value's `file:line:column`.
2. **Given** `cert_file` without `key_file` (or the reverse), **When** checked, **Then** exit 2 with
   an error naming the missing key at the table's `file:line:column`.
3. **Given** both `enrollment_token_file` and `cert_file`/`key_file`, **When** checked, **Then**
   exit 2 with an error saying to give one: the token enrolls, the certificate is the result.
4. **Given** neither a certificate pair nor an enrollment token file, **When** checked, **Then**
   exit 2 with an error naming the choice.

Error anchors (each error is reported as `file:line:column`; exit status 2):

| Rule | Anchor |
|---|---|
| `url` missing | the `[orchestrator]` header |
| `url` not `https`, or not a valid URL | the `url` value |
| `cert_file` without `key_file`, or the reverse | the key that is present |
| token and certificate pair both given | the `enrollment_token_file` key |
| neither token nor certificate pair | the `[orchestrator]` header |
| relative or empty path | the offending value |
| wrong value type | the offending value |

### User Story 3 - Secrets never leak (Priority: P1)

**Why this priority**: The file is safe to print, diff and log only if no secret content reaches it.

**Independent Test**: Parse a table whose `key_file` and `enrollment_token_file` hold a sentinel
string; format the configuration with `Debug`, `Display` and the resolved view; assert the sentinel is
absent.

**Acceptance Scenarios**:

1. **Given** a parsed configuration, **When** any part is formatted with `Debug` or `Display`, **Then**
   no secret file content appears.
2. **Given** a named file that exists, **When** the resolved view is printed, **Then** each of
   `ca_file`, `cert_file`, `key_file` and `enrollment_token_file` shows as `path (readable, 0600)`
   (the actual mode), never content.
3. **Given** a named file that does not exist, **When** the resolved view is printed, **Then** it
   shows `path (missing)`.
4. **Given** a secret file readable by others, **When** checked with `--files`, **Then** the existing
   severity applies (warning on a host or SSH daemon, error in a container or pod; see
   "Rules for every key" and the `--files` paragraph of the user guide).

### Edge Cases

- `[orchestrator]` present but empty: the missing-`url` and the credential-choice errors are both
  reported, each at the table header.
- Empty-string `url` or path, or a wrong value type: exit 2, anchored at the value.
- A duplicate `[orchestrator]` table: refused by the TOML parser as for any table, first error only.
- A path pointing at a directory: with `--files`, an error at the value.
- `url` with credentials, a fragment, or no host: refused as an invalid URL.
- A relative path: exit 2 with `must be an absolute path`, anchored at the value, as for every path in the file.
- `--files`: a missing `ca_file`, `cert_file`, `key_file` or `enrollment_token_file` is an error;
  a secret file readable by others follows the severity in US3 scenario 4. On Windows the mode is not checked and the state shows `readable`.
- Windows paths: written as TOML literal strings, as elsewhere in the file.
- Several daemons share no state through this table (Principle II). A live reload re-reads the table
  with the rest of the file; when the new file fails the check, the daemon keeps the last valid
  configuration, as it does for every other table.

## Requirements

### Functional Requirements

- **FR-001**: The file MUST accept an `[orchestrator]` table with `url`, `ca_file`, `cert_file`,
  `key_file` and `enrollment_token_file`; the table MUST refuse any other key.
- **FR-002**: All four `*_file` keys MUST be absolute paths to files; the file MUST NOT hold a secret
  value. `ca_file` and `cert_file` hold no secret but are shown the same way.
- **FR-003**: `url` MUST be required when the table is present and MUST use the `https` scheme.
- **FR-004**: `cert_file` and `key_file` MUST be given together; one without the other is an error.
- **FR-005**: Exactly one of `enrollment_token_file` and the `cert_file`/`key_file` pair MUST be
  given; both, or neither, is an error that says which to remove or add.
- **FR-006**: Every error MUST name `file:line:column`, in the format of existing `config check` errors.
- **FR-007**: `Debug`, `Display` and the resolved view MUST NOT contain secret content; each of the
  four file keys appears as its path and state (`readable, <mode>` or `missing`) only.
- **FR-008**: `docs/user-guide/daemon-configuration.md` MUST document the table and each rule.
- **FR-009**: With `--files`, a missing named file MUST be an error, with the severity rules of US3 scenario 4.
- **FR-011**: A live reload with an invalid `[orchestrator]` table MUST keep the last valid configuration.
- **FR-010**: A file without `[orchestrator]` MUST behave exactly as before.

### Key Entities

- **Orchestrator settings**: URL, optional CA file, credential choice (certificate pair or
  enrollment token file), all by path.

## Out of Scope

Connecting to an orchestrator, enrollment itself, certificate renewal, and rendering the table from
Settings (other issues of #1722).

## Success Criteria

- **SC-001**: Each error row of the anchor table (both directions of the cert/key pair included) has a test that asserts the reported `file:line:column`.
- **SC-002**: A test asserts that no secret content appears in `Debug`, `Display` or the resolved
  view of a parsed `[orchestrator]` table.
- **SC-003**: A file without `[orchestrator]` gives the same check result and resolved view as before.
- **SC-004**: An operator can find each rule and its error in the user guide.

## Assumptions

- `ca_file` may be omitted (system trust roots are used).
- Plan documents `specs/orchestrator/` named in the issue are not on this branch; the issue text is
  the source of requirements.
