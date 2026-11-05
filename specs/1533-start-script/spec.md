# Feature Specification: An optional start script prepares each daemon

**Feature Branch**: `feat/1533_an-optional-start-script-prepares-each-daemon`
**Created**: 2026-11-01
**Status**: Closed 2026-11-05 — shipped in PRs #1630, #1665, #1669, #1673, #1682, #1698, #1712
**Issue**: #1533 (part of #1521)
**Parent design**: `specs/daemon-config-file/` (spec.md Story 5, FR-019 to FR-024, FR-032, SC-007;
`contracts/daemon-config.md` sections `[startup]` and `[session]` precedence; research D12).
This spec narrows the parent to this issue; the parent stays the source for the full schema.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A start script runs before the daemon serves (Priority: P1)

A user gives a daemon a start script. The daemon runs it, as its own user, before it accepts any
client, on every way the daemon can be started, and what the script exports reaches the daemon and
every session it starts.

**Why this priority**: It is the feature: a sandbox gets an extra tool or variable without building
another image.

**Independent Test**: Give a daemon a script that installs a tool under `~/.local` and exports
`FOO=bar`; start it; a new session finds the tool and sees `FOO=bar`.

**Acceptance Scenarios**:

1. **Given** a script that exports `FOO=bar`, **When** the daemon starts, **Then** the daemon and
   every session see `FOO=bar`, and no client could connect before the script finished.
2. **Given** a container daemon with a script, **When** it starts at container boot and again over
   SSH, **Then** the script runs the same way on both, as the daemon's own user (never root in a
   sandbox).
3. **Given** `export_env = false`, **When** the script exports a variable, **Then** it reaches
   neither the daemon nor its sessions.
4. **Given** no `[startup]` table, **When** the daemon starts, **Then** nothing runs and nothing changes.

---

### User Story 2 - Variables reach sessions in a fixed order; a secret only if exported (Priority: P1)

A user keeps a token in an `env_file` for the script. The script can read it. The token reaches the
daemon and sessions only when the script explicitly exports it. When several settings name the same
variable, the same one always wins.

**Why this priority**: A secret must not leak to every session by accident, and an unstated order
makes the same file behave differently per path (FR-032).

**Independent Test**: An `env_file` holds `TOKEN=x`. A script that only reads `$TOKEN` leaves it out
of every session; a script that runs `export TOKEN` puts it in. An environment-include that sets
`FOO` beats a start-script `FOO`.

**Acceptance Scenarios**:

1. **Given** an `env_file` with `TOKEN=x` and a script that does not export it, **When** a session
   starts, **Then** `TOKEN` is not in its environment and not in the daemon's.
2. **Given** the same file and a script that runs `export TOKEN`, **Then** both see `TOKEN=x`.
3. **Given** the same variable in the daemon environment, the start script's exports,
   environment-include, `ai.cli[].env`, `session.env`/`secret_env` and the session's own variables,
   **Then** the later one in that order wins; environment-include beats the start script.
4. **Given** a script that exports `PATH=$HOME/.local/bin:$PATH`, **Then** the daemon and sessions see the extended PATH (the reserved names of the contract's V16 bind only `[session.env]`, `[session.secret_env]` and `ai.cli[].env`, not the script).
5. **Given** a script that exports a variable the daemon sets per session (the last tier, e.g. `MICOLD_*`), **Then** the daemon's per-session value stands.

---

### User Story 3 - `run` and `on_failure` behave as specified (Priority: P2)

A user chooses whether the script runs every start or once, and whether a failure stops the daemon
or only warns.

**Why this priority**: Installs are slow and idempotent scripts are rare; a broken script must not
always take a daemon down.

**Independent Test**: With `once_per_container`, restart twice with the script unchanged: it runs
once; edit it: it runs again. With a failing script, `refuse` leaves the daemon down and `warn`
serves.

**Acceptance Scenarios**:

1. **Given** `run = "once_per_container"` and an unchanged script, shell and env file, **When** the
   daemon restarts, **Then** the script does not run; when any of the three changed, it runs again.
   On a host or SSH daemon "once" means once per data directory.
2. **Given** a script that failed, **Then** it is not recorded as done and runs on the next start.
3. **Given** `on_failure = "refuse"` and a script that exits non-zero or times out, **Then** the
   daemon does not serve; with `warn` it serves and reports a warning.
4. **Given** `timeout_secs`, **When** the script runs longer, **Then** it and its children are
   stopped and the run counts as failed.

---

### User Story 4 - A failure is visible in the app with its output (Priority: P1)

When the script fails or times out, the user sees the exit status and the end of the script's output
in the application, whichever start path was used. Output also goes to the daemon log.

**Why this priority**: SC-007: without it a refused daemon is an unexplained dead daemon.

**Independent Test**: Make the script `echo oops; exit 1` with `refuse`; the app shows the daemon in
an error state with "oops" and exit status 1. Repeat with `warn`: the app shows a warning with the
same text.

**Acceptance Scenarios**:

1. **Given** a refused start, **Then** the daemon's error state in the app shows the exit status and
   the last 200 lines, on host, SSH, container and pod daemons.
2. **Given** a `warn` failure, **Then** the daemon serves and the app shows the same text as a warning.
3. **Given** any run, **Then** the script's output is in the daemon log under the `startup` target.

---

### User Story 5 - Settings → Daemons edits the script and says a restart is needed (Priority: P2)

In Settings → Daemons the user picks a script file or writes it in an inline editor, sets the
other `[startup]` keys, and is told that a change applies only after a restart.

**Why this priority**: The script has no other way in for a user who does not hand-write the file.

**Independent Test**: Write a script inline, save; the daemon, already running, does not re-run it,
and Settings shows "Restart to apply". Restart: it runs.

**Acceptance Scenarios**:

1. **Given** a daemon's settings, **Then** the user can pick a file or write the script inline, and
   set shell, timeout, failure policy, run policy, secret env file and export setting. An inline script is saved as a file, delivered like a picked file, and `script_file` points at it.
2. **Given** a running daemon and a changed script or `[startup]` table, **When** the config is
   delivered, **Then** the script does not re-run and the daemon reports `restart_needed`;
   Settings shows a restart notice.
3. **Given** an invalid value (timeout outside 1 to 3600, unknown policy), **Then** Settings
   refuses it with the reason before delivery.
4. **Given** a script and `env_file`, **Then** they are delivered to the daemon with the config on
   every kind (host, SSH, container, pod); no script text or secret value appears in the config file.

### Edge Cases

- Empty script file: runs, exports nothing, succeeds.
- Script file or `env_file` missing or unreadable at start: counts as a failed run, handled by `on_failure`, with that reason as the output.
- Two daemons on one host, or two starts racing (Principle II): each data directory has one run at a
  time; the `startup.done` record is written only after success and never half-written.
- Script writes no trailing newline, binary bytes, or a huge amount of output: the log and the app copy are bounded (last 200 lines).
- Script exports a variable with a newline in its value: captured intact (`env -0` diff).
- `env_file` lines that are blank, comments, or contain `$` or quotes: values are taken literally, never evaluated.
- The script backgrounds a child that holds the output pipe open: the timeout still ends the run.
- Windows host (Principle VI): the default shell is PowerShell; `export` means setting `$env:NAME`; process-group kill has a platform equivalent.
- A config reload while the daemon runs never re-runs the script.
- `[startup]` with `env_file` but no `script_file`: invalid, refused by validation (`script_file` is required in the table).
- `warn` failure: the script's partial exports are discarded; the daemon serves with its unchanged environment.
- A second start while a run holds the data directory: it waits for the run to end, then evaluates `once_per_container` against the record; it does not run the script concurrently.
- Windows: exports are captured by an equivalent of the diff (environment dumped before and after in PowerShell); where not available that is a documented limitation.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001** (parent FR-019): The `[startup]` table MUST accept `script_file` (required in the table), `shell`,
  `timeout_secs` (1 to 3600, default 300), `on_failure` (`refuse` default, `warn`), `run`
  (`every_start` default, `once_per_container`), `env_file` and `export_env` (default true). Absent
  table: no script. No script content and no secret value is inline in the config.
- **FR-002** (FR-020): The daemon MUST finish the script before it binds its listener, on every
  start path of its kind, as its own user, never as root in a sandbox.
- **FR-003** (FR-021): With `export_env` on, variables the script explicitly exports MUST reach the
  daemon and every session it starts. A value from `env_file` MUST reach them only if the script
  exports it.
- **FR-004** (FR-032): The order in which a variable's value is chosen MUST be: daemon environment
  (with start-script exports), environment-include, `ai.cli[].env`, `session.env`/`secret_env`,
  the variables the daemon sets per session (always last, never overridden, including by the script); the later wins. `export_env = false` removes the start-script tier. A script MAY extend or replace `PATH`.
  *Note (close, 2026-11-05):* the daemon applies today only the tiers that exist in its code (daemon environment with start-script exports, environment-include, the session's own variables); the `ai.cli[].env` and `session.env`/`secret_env` tiers are not applied by the daemon yet, so the order is proven only for those.
- **FR-005** (FR-019): `run = "once_per_container"` MUST skip the script while the recorded hash of
  script, shell and `env_file` matches; the record MUST be written only after success. A skipped run applies no exports.
- **FR-006** (FR-022): `timeout_secs` MUST stop the script and its children and count as a failure.
- **FR-007** (FR-022): With `refuse`, a failed run MUST keep the daemon from serving; with `warn`
  it MUST serve and report a warning.
- **FR-008** (FR-022): Script output MUST go to the daemon log; a failure MUST carry the exit
  status and the last 200 lines to the application on every start path.
- **FR-009** (FR-023): A changed script or `[startup]` table MUST NOT re-run in a running daemon,
  including on config reload; the daemon MUST report `restart_needed`.
- **FR-010** (FR-024): Settings → Daemons MUST let the user pick a script file or write it inline,
  edit the other keys, validate them before delivery, and show a restart notice for a pending change.
- **FR-011** (FR-019): The application MUST deliver the script and `env_file` with the config on
  host, SSH, container and Kubernetes daemons.
- **FR-012**: The user guide MUST describe the start script, its relation to environment-include, and that installs go to the user's home, not root.

### Key Entities

- **Start script**: the user's file, its shell, timeout, policies, optional secret env file.
- **Run record** (`startup.done`): hash of script, shell and env file, written after a success.
- **Startup result**: success, warning or failure with exit status and output tail, which the application reads.
- **Captured exports**: the variables the script exported, applied to the daemon and its sessions.

## Success Criteria *(mandatory)*

- **SC-001** (SC-007): A start script's failure is visible in the application with its output in
  100% of failed starts on every daemon kind.
- **SC-002**: A sandbox gains an extra tool and a variable from a script without building an image;
  a new session finds both on the first start.
- **SC-003**: A secret in `env_file` appears in no session unless the script exports it (0 leaks in the tests).
- **SC-004**: No client connects before the script ends (0 accepted connections during a run).
- **SC-005**: A changed script never re-runs in a running daemon and Settings says a restart is needed in 100% of cases.

## Assumptions

- The loader, reload, `restart_needed` and environment-include engine (`env -0` diff) exist (#1524, closed).
- Prerequisites #1526, #1527, #1528 (config delivery to host/SSH, container, Kubernetes) are OPEN. Daemon-side work (run, exports, failure record, precedence) and Settings need only #1524. Delivering the script and `env_file` (FR-011) and showing a failure on a kind (US4.1) land per kind as each prerequisite merges; the plan orders milestones accordingly.
- The schema, defaults and examples are the parent contract; this spec adds no key.
- The shared multi-line text editor of the Settings UI is reused for the inline editor.

## Out of Scope

- Re-running the script on reload or by a button (a restart does it).
- Running the script as root, or in a different user than the daemon.
- Editing the script's content in the config file itself.
- Per-session scripts (environment-include covers those).
- Other `daemon.toml` sections.
