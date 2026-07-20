# Quickstart: Validating the Environment-Include Script

Manual end-to-end validation for the scenarios in `spec.md`'s User Stories. Run the GUI with
`mise run run` (per `CLAUDE.md`). See `contracts/env-include-resolution.md`,
`contracts/settings-schema-addition.md`, and `contracts/settings-ui.md` for the underlying
mechanism this exercises.

## Prerequisites

- A Linux or macOS machine with `bash` on `PATH` (or Windows with `powershell.exe`).
- No prior `settings.json` for a true "fresh install" check (back up and remove
  `~/.local/share/micold-ai-ide/settings.json` or platform equivalent — see
  `JsonFileSettingsStore::default_location`).

## Scenario 1 — Fresh install auto-includes the default script (User Story 1, SC-001)

1. Ensure `~/.bashrc` (Linux/macOS) exports something identifiable, e.g. add
   `export QUICKSTART_MARKER=hello` and `export PATH="$HOME/.quickstart-bin:$PATH"`.
2. Remove any existing `settings.json` (see Prerequisites).
3. Launch the app, open a session, switch to AI CLI mode.
4. From the AI CLI's shell prompt (or by asking `claude` to print its environment), confirm
   `QUICKSTART_MARKER=hello` and the `PATH` prepend are present.
5. Switch the same session to Regular Terminal mode; run `echo $QUICKSTART_MARKER` and
   `echo $PATH` — confirm identical values (FR-006, SC-004).

**Expected**: both processes see the variable and `PATH` change, with zero manual setup.

## Scenario 2 — Reconfigure or disable (User Story 2, SC-002/SC-007)

1. Open **Settings**. Confirm the "Environment include" fields (Enabled checkbox, script path,
   timeout) appear grouped together, visually separate from the scrollback field (FR-015).
2. Change the script path to a different file containing `export QUICKSTART_MARKER=changed`, and
   Save.
3. Open a new session; confirm `QUICKSTART_MARKER=changed` (not `hello`) is present.
4. Reopen Settings, uncheck **Enabled**, Save.
5. Open another new session; confirm `QUICKSTART_MARKER` is absent entirely.
6. Re-check **Enabled**, Save; confirm a new session sees the variable again.
7. Reopen Settings, leave **Enabled** checked but clear the script path field entirely, and Save.
   Confirm this behaves identically to disabling — no script is sourced, and a new session sees no
   captured variables (spec.md Edge Cases: a blank path is treated the same as disabled).

**Expected**: each Settings change takes effect on the next new session, without an app restart.

## Scenario 3 — Broken/slow scripts never block a session (User Story 3, SC-003/SC-006)

1. Set the script path to a nonexistent file. Open Settings — confirm a non-blocking failure
   note reading "Script not found" (no diagnostic text, since no subprocess ran —
   `contracts/env-include-resolution.md`).
2. Set the script path to a file containing `exit 1` (optionally after printing something to
   stdout/stderr first). Save. Confirm Settings shows "Exited with an error" plus the script's own
   printed output as the diagnostic (FR-013's full-diagnostic tradeoff).
3. Set the script path to a file containing `sleep 999`. Set the timeout to a small value (e.g.
   `2`) and Save. Time how long Settings takes to reflect the failure — expect it to land at
   approximately the configured timeout, not 999 seconds (SC-003).
4. In all three cases, open a session anyway — confirm it launches promptly and is otherwise fully
   usable (no hang, no crash).
5. Fix the script (make it valid again), then use that session's existing manual **restart**
   control (shown when its attached process isn't running). Confirm the failure note in Settings
   clears and the freshly-sourced variable is now present — without restarting the whole app
   (spec Clarifications, restart-triggered refresh).

**Expected**: every failure mode is bounded, visible, and recoverable without an app restart.

## Scenario 4 — Persistence never leaks captured values (SC-005)

1. After Scenario 1 or 2 has resolved successfully, quit the app.
2. Inspect `settings.json` directly (`JsonFileSettingsStore::default_location()`'s path).
3. Confirm it contains only `env_include_enabled`, `env_include_script_path`, and
   `env_include_timeout_secs` — never `QUICKSTART_MARKER` or any other captured variable name/value,
   and never the failure diagnostic text from Scenario 3.

**Expected**: the persisted document is exactly the three settings fields — nothing captured.

## Windows variant

Repeat Scenario 1 using `%USERPROFILE%\Documents\WindowsPowerShell\profile.ps1` (create it if
absent) with `$env:QUICKSTART_MARKER = "hello"` instead of `export`, and confirm the same
cross-process consistency (research R6).
