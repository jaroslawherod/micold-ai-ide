# Feature Specification: Environment-Include Script

**Feature Branch**: `feat/optional-include`

**Created**: 2026-07-20

**Status**: Draft

**Input**: User description: "Add an optional environment-include script that runs automatically, by default, and supplies environment variables to both the `claude` AI CLI process and the regular-terminal shell process for every session. Today, `LaunchSpec.env` (used for `claude`, `src/terminal.rs`) and `spawn_shell_pty`'s `env` parameter (used for the regular shell, `src/ui/terminal.rs`) are both populated with nothing but a hardcoded `TERM=xterm-256color` pair (`main.rs`), so neither process reliably picks up the user's normal shell environment. Add a new app-level setting (enabled + script path, default ON, default path the platform's conventional rc file) exposed in the Settings modal. Resolve the script by actually sourcing it in a real, disposable shell process and diffing the resulting environment against a clean baseline, then merge into the same `env: Vec<(String, String)>` used by both existing spawn call sites."

## Clarifications

### Session 2026-07-20

- Q: How much diagnostic detail should the failure indication (FR-013) include, given a failing script may have printed secret-bearing output before failing? → A: Full diagnostic, including the script's captured stdout/stderr output from the failed attempt, is shown to the user — accepted as a deliberate tradeoff favoring debuggability over the risk of the script's own output containing secrets.
- Q: Besides the once-per-app-run cache and the Settings-save refresh, is there another way to force a fresh re-source (e.g. after fixing a broken script) without restarting the whole app? → A: Yes — the existing per-session manual restart control (shown when that session's process isn't running) also triggers a fresh re-source, in addition to the Settings-save refresh.
- Q: What should bound how long sourcing the script is allowed to run before being treated as hung (FR-012)? → A: A user-configurable timeout setting (FR-003), defaulting to 10 seconds (FR-004), editable from the Settings interface grouped alongside the other environment-include fields (FR-015).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Sessions automatically pick up my shell environment (Priority: P1)

A developer has their normal shell environment set up the way they like it — a version manager (nvm/pyenv/rbenv) prepending to `PATH`, exported API keys, proxy settings, and so on. When they open a new session in the app — whether they're talking to the AI CLI or working in the regular terminal — those variables are already there, without the developer having to re-export anything by hand or launch the app from an already-configured terminal.

**Why this priority**: This is the core problem statement: today the AI CLI process never runs any rc file at all (it isn't spawned through an interactive shell), and the regular-terminal shell only inherits whatever the app process itself happened to start with (which is empty/minimal when launched from a desktop launcher). Without this, every other part of the feature is moot.

**Independent Test**: Add an exported variable and a `PATH` prepend to the configured include script (e.g. the default `~/.bashrc`); open a new session; confirm the variable and the `PATH` change are visible in both the AI CLI process's environment and the regular-terminal shell's environment for that session.

**Acceptance Scenarios**:

1. **Given** a fresh install with no prior settings, **When** the user opens their first session, **Then** the AI CLI process and the regular-terminal shell process both see variables sourced from the platform's default include script (e.g. `~/.bashrc` on Linux/macOS), with no setup required from the user.
2. **Given** environment-include is enabled and resolved successfully, **When** the user opens a session in AI CLI mode and then switches that same session to regular-terminal mode (or vice versa), **Then** both processes see the identical set of resolved variables.
3. **Given** the configured script exports a variable not otherwise present in the app's own environment, **When** a session is launched, **Then** that variable is present in the spawned process's environment.

---

### User Story 2 - Configure or turn off environment-include (Priority: P2)

A developer wants to point the feature at a different script than the default, or turn it off entirely (for example, because their rc file is slow, or because they don't want its side effects applied to app-spawned sessions).

**Why this priority**: The feature reads and executes a file outside the app's control; some users will want to redirect or disable that by design, and the feature description explicitly requires this to be a user-facing setting, not a hidden always-on behavior.

**Independent Test**: Open Settings, change the script path to a different file (or clear/disable the feature), save, open a new session, and confirm the new path was used (or that no include script ran).

**Acceptance Scenarios**:

1. **Given** the Settings interface is open, **When** the user changes the environment-include script path and saves, **Then** subsequent new sessions resolve their environment from the newly configured path.
2. **Given** the Settings interface is open, **When** the user disables environment-include and saves, **Then** subsequent new sessions launch with only the app's existing hardcoded environment (no script is sourced).
3. **Given** environment-include is currently disabled, **When** the user re-enables it and saves, **Then** subsequent new sessions resolve their environment from the configured script again.
4. **Given** the Settings interface is open, **When** the user views the environment-include controls, **Then** the enabled flag, script path, and timeout fields appear grouped together as one related set, separate from unrelated settings (theme, scrollback limit); changing the timeout and saving takes effect on the next sourcing attempt.

---

### User Story 3 - Broken or slow include scripts never block a session (Priority: P3)

A developer's configured script is missing, contains an error, or is unusually slow (or hangs, e.g. on a stalled network call inside a version-manager init block). The developer still expects to be able to open a session promptly.

**Why this priority**: This is a robustness guarantee, not new user-facing value — but it's what makes it safe to turn the feature on by default for every user, matching the existing precedent that settings loading never crashes or blocks the app.

**Independent Test**: Point the configured path at a nonexistent file, then at a script that `exit 1`s partway through, then at a script that hangs (e.g. `sleep 999`); in all three cases, confirm session launch still completes promptly and the session is otherwise usable.

**Acceptance Scenarios**:

1. **Given** the configured script path does not exist, **When** a session is launched, **Then** the session still launches promptly with the app's existing (non-script) environment, and the failure is visible to the user in a non-blocking way (e.g. a status note in the Settings interface).
2. **Given** the configured script exits with a non-zero status partway through, **When** a session is launched, **Then** the session still launches promptly, using whatever partial or baseline environment is available, and the failure is surfaced the same non-blocking way.
3. **Given** the configured script hangs indefinitely, **When** a session is launched, **Then** the app does not wait indefinitely — sourcing is abandoned after a bounded delay, the session launches with whatever environment is otherwise available, and the timeout is surfaced the same non-blocking way.
4. **Given** a previous include attempt failed and the user has since fixed the underlying script, **When** the user invokes that session's existing manual restart control, **Then** the script is re-sourced fresh (not reused from the stale cached snapshot) before the session relaunches.

---

### Edge Cases

- Configured script path is empty/blank after the user clears it in Settings: treated the same as disabled (no script sourced).
- Script produces no environment differences at all (e.g. an empty file): sessions launch with only the app's existing hardcoded environment; not treated as a failure.
- A variable sourced from the script has the same name as the app's hardcoded `TERM` value: the hardcoded `TERM` value wins (terminal emulation depends on it).
- A variable sourced from the script has the same name as a variable already present in the app process's own inherited environment: the script-sourced value wins, since the goal is to reflect the user's normal interactive-shell setup rather than whatever the app process happened to inherit.
- The user changes the setting (path, enabled flag, or timeout) while sessions are already running: already-running session processes are unaffected; the new resolution applies only to sessions/terminals spawned after the change.
- The configured script itself prints to stdout/stderr as a side effect of being sourced (common in rc files, e.g. a version-manager banner): this output MUST NOT be forwarded into the session's terminal display or otherwise interfere with the eventual session's own process I/O — it is only a means to observe the resulting environment (or, on failure, to populate the FR-013 diagnostic).
- On Windows, the equivalent default include mechanism is the user's PowerShell profile script rather than a bashrc-style file (see FR-018).
- Invoking one session's manual restart control refreshes the single shared resolved-environment snapshot for the whole app (FR-007), but only the restarted session's next launch picks up the fresh values immediately — other sessions whose processes are already running keep their existing process's environment until they too are restarted or relaunched.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide an app-level setting that controls whether environment-include is active ("enabled"), alongside the existing theme and scrollback-limit settings.
- **FR-002**: System MUST provide an app-level setting for the path to the script to include, editable independently of the enabled flag.
- **FR-003**: System MUST provide an app-level setting for how long, in seconds, sourcing the script is allowed to run before being treated as hung (the timeout), editable independently of the enabled flag and the path.
- **FR-004**: The enabled flag MUST default to on, the script-path setting MUST default to the platform's conventional interactive-shell startup file, and the timeout setting MUST default to 10 seconds — so a fresh install with no existing settings file auto-includes the default script with no user action (matches the existing missing-field-defaults pattern used for the scrollback limit).
- **FR-005**: System MUST resolve the script's effect on environment variables by actually executing/sourcing it in a real, disposable shell process and comparing the resulting environment against a clean baseline environment — not by parsing the script's text — so that conditionals, sourced sub-files, `PATH`-mutating loops, and version-manager init blocks all resolve correctly.
- **FR-006**: The AI CLI session process and the regular-terminal shell process MUST both be launched with the identical set of variables resolved from the include script for a given app run (no divergence between the two processes' resolved environments).
- **FR-007**: The include script MUST be (re-)sourced once per application run and cached for the remainder of that run. It MUST also be re-sourced fresh — refreshing the same shared cached snapshot used by every session — whenever either of the following occurs: (a) the user saves a change to the enabled flag, script path, or timeout in the Settings interface, or (b) the user invokes the existing per-session manual restart control (shown when that session's currently-attached process isn't running) for any session. This keeps sourcing cost off of every individual session/terminal launch, while giving the user two ways to force a refresh — via Settings or via a session restart — without restarting the whole app.
- **FR-008**: Environment-variable values captured from sourcing the script MUST NOT be persisted to disk (including the settings file); only the enabled flag, the script path, and the timeout are persisted. Captured values exist only in memory for the running app instance and are re-resolved from scratch on the next app run (or the next FR-007 refresh trigger).
- **FR-009**: When a variable captured from the script has the same name as the app's existing hardcoded `TERM` value, the hardcoded `TERM` value MUST take precedence.
- **FR-010**: When a variable captured from the script has the same name as a variable already present in the app process's own inherited environment (excluding `TERM`), the script-sourced value MUST take precedence.
- **FR-011**: If the configured script is missing, fails partway through (non-zero exit), or produces no usable environment diff, session launch MUST proceed unaffected — using whatever environment is otherwise available — rather than failing the launch or blocking it, matching the existing precedent that a missing/corrupt settings file degrades to defaults rather than erroring.
- **FR-012**: Sourcing the script MUST be bounded by the configured timeout (FR-003/FR-004), so a script that hangs (e.g. on a stalled network call or waiting on interactive input) cannot block or indefinitely delay session launch; on timeout, resolution is abandoned for that attempt and treated the same as any other include failure (FR-011).
- **FR-013**: When the most recent attempt to resolve the include script failed (missing script, non-zero exit, or timeout), the system MUST surface a non-blocking, user-visible indication of that failure (e.g. a status note in the Settings interface) — it MUST NOT be silent, and it MUST NOT interrupt or delay session launch. The indication MUST include the failure category (missing script / non-zero exit / timeout) plus the script's own captured stdout/stderr output from that failed attempt, to aid troubleshooting; this diagnostic text is held only in memory alongside the rest of the resolved-environment state (FR-008) and MUST NOT be written to the persisted settings file or any other on-disk log.
- **FR-014**: Users MUST be able to view and change all three parts of the environment-include setting (enabled flag, script path, and timeout) from the existing Settings interface, alongside the existing theme and scrollback-limit controls.
- **FR-015**: The Settings interface MUST present the environment-include setting's fields (enabled flag, script path, timeout) grouped together as one related set, visually distinct from unrelated settings (theme, scrollback limit).
- **FR-016**: Changing the environment-include setting (enabling/disabling, or changing the path or timeout) MUST apply to sessions/terminal processes spawned after the change; it MUST NOT modify or restart already-running session processes, matching the existing precedent for how a changed scrollback limit applies only to newly spawned terminals.
- **FR-017**: On Linux and macOS, the script MUST be sourced using the bash interpreter specifically (independent of the user's own configured login/interactive shell), since the default script path (`~/.bashrc`) is bash syntax; a user-supplied custom path is sourced the same way.
- **FR-018**: On Windows, the default script path MUST be the user's PowerShell profile script location, sourced via the PowerShell interpreter; a user-supplied custom path on Windows is sourced the same way. The feature (enabled flag + path + timeout settings) is present and configurable on Windows on the same terms as on Linux/macOS.
- **FR-019** *(added — BUG-001)*: On Linux/macOS, sourcing the script (FR-005/FR-017) MUST behave as an interactive shell for the purpose of any interactive-only guard the script itself checks (e.g., the standard `case $- in *i*) ;; *) return;; esac` pattern used by Debian/Ubuntu's stock `~/.bashrc`), so that the platform's default script path (FR-004) resolves its real exports out of the box — without requiring the user to edit, relocate, or replace their existing rc file — per User Story 1's Acceptance Scenario 1 and SC-001.

### Key Entities

- **Environment-Include Setting**: The persisted, user-configurable part of this feature — an enabled/disabled flag, a script-file path, and a timeout (seconds). Lives alongside the existing theme and scrollback-limit settings, is grouped together as one related set in the Settings interface (FR-015), and follows the same persistence and default-on-missing-field behavior.
- **Resolved Environment Snapshot**: The in-memory (never persisted) set of name/value pairs captured from sourcing the configured script during the current app run. A single shared snapshot is used by every session; it is recomputed once per app run and refreshed by either a Settings save or any session's manual restart control (FR-007); discarded when the app exits.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a fresh install with no prior settings, a user's version-manager `PATH` entries and exported shell variables are available in a newly opened session's AI CLI process and regular-terminal process without the user performing any manual setup.
- **SC-002**: A user can change the environment-include script path, or turn the feature off, entirely from within the app's Settings interface, without editing any file outside the app.
- **SC-003**: When the configured script is missing, broken, or hangs, opening a session is never delayed by more than the configured timeout (10 seconds by default), and otherwise takes no perceptibly longer than opening a session with environment-include turned off.
- **SC-004**: For any given session, 100% of the environment variables resolved from the include script are present identically in both the AI CLI process and the regular-terminal process.
- **SC-005**: Inspecting the app's persisted settings file never reveals any captured environment-variable value or failure diagnostic text (only the enabled flag, the configured script path, and the configured timeout are found there).
- **SC-006**: When the most recent include attempt failed, a user can discover that fact — including the failure category and the script's own captured output — from within the app (e.g. the Settings interface) without needing to inspect logs or files.
- **SC-007**: A user can locate and adjust all three environment-include settings (enabled flag, script path, timeout) as one grouped set in the Settings interface, without hunting for them among unrelated settings.

## Assumptions

- The default script path on Linux/macOS is `~/.bashrc`; Windows' default is the user's PowerShell profile script location, per FR-018.
- "Session" here covers both of the app's existing terminal modes for a session (AI CLI / `claude`, and the regular terminal), consistent with how both already share the same underlying `env` mechanism today.
- The "clean baseline environment" used for diffing (FR-005) is the environment the shell process would have before the script runs — the sourcing process does not need to be sandboxed from the filesystem or network, only compared against for the purpose of capturing what the script *changed*, not for security isolation.
- ~~Non-interactive sourcing of an interactive-oriented rc file (e.g. a `.bashrc` that early-exits when it detects a non-interactive shell) is a known limitation of "source it for real" resolution; this spec does not require working around scripts that intentionally skip their own setup in non-interactive contexts.~~ **Superseded (BUG-001)**: the platform default script path (`~/.bashrc` on Linux/macOS, FR-004) is itself exactly such an interactive-oriented file — Debian/Ubuntu's stock `~/.bashrc` opens with a `case $- in *i*) ;; *) return;; esac` guard that returns before any of its own exports run when sourced non-interactively. Because FR-004 mandates defaulting to this exact file, resolution MUST satisfy that guard for the *default* path, per new FR-019 — otherwise User Story 1's "no setup required" promise (SC-001) fails out of the box on the two most common Linux distros. A user-supplied custom script that guards on some *other*, unrelated condition remains out of scope, per the original assumption.
- No new user-facing surface is required beyond the existing Settings interface; the failure indication in FR-013 can reuse that same surface rather than introducing a new notification system.
- Like the existing scrollback-limit setting (which clamps to a sane numeric range), the timeout setting is expected to be clamped to a sane range (e.g. a few seconds to under a minute) rather than accepting arbitrary values; the exact bounds are a planning-phase detail, not a scope decision.

**Bugfix**: 2026-07-21 — BUG-001 Sourcing the default `~/.bashrc` never reached any of its own exports on Debian/Ubuntu, because the sourcing shell ran non-interactively and the stock file's own interactive guard (`case $- in *i*) ;; *) return;; esac`) returned before anything else executed — silently defeating User Story 1's "no setup required" promise for the exact default path FR-004 configures. FR-019 added (sourcing MUST satisfy the script's own interactive-guard check on Linux/macOS); the Assumptions bullet disclaiming non-interactive-guard scripts is superseded/narrowed to scripts that guard on something other than interactivity. See `bugs/BUG-001.md`.
