# Implementation Plan: Environment-Include Script

**Branch**: `011-env-include-script` | **Date**: 2026-07-20 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/011-env-include-script/spec.md`

## Summary

Both spawn call sites (`launch_spec()` for the `claude` AI CLI process, and the
`TerminalMode::Regular` branch of `ensure_attached_process` for the regular-terminal shell) today
build their `env: Vec<(String, String)>` from nothing but a hardcoded `TERM=xterm-256color` pair.
This feature resolves the user's normal shell environment by actually sourcing a configured rc
script (default `~/.bashrc` on Linux/macOS, the PowerShell profile on Windows) in a real,
disposable, timeout-bounded shell process, diffing its resulting environment against a clean
baseline, and merging the captured variables into both call sites' `env` vector — with the
enabled flag, script path, and timeout persisted as a new grouped Settings section, and captured
values/diagnostics kept in memory only (never on disk).

**Technical approach**: A new pure/core module `src/env_include.rs` owns the sourcing/diffing
engine (categorizing outcomes as `Success`, `MissingScript`, `NonZeroExit`, or `TimedOut`),
callable without the `gui` feature since it only needs `std::process::Command`. Three new fields
extend `Settings`/`StoredSettings` (`src/settings.rs`), following the exact
`#[serde(default)]`-on-missing-field pattern `scrollback_lines` already established. `App`
(`src/main.rs`) gains one shared, never-persisted resolved-environment snapshot, computed once at
`boot()` (mirroring the existing synchronous `discover_worktrees`/`GitCli` precedent already run
inside `boot()` and inside `update()`), and refreshed by exactly two triggers: a `SettingsSaved`
that touched the enabled/path/timeout fields, or a `TerminalRestartRequested` for any session.
`settings_form.rs` gains a grouped sub-section (checkbox + two text inputs + a read-only failure
diagnostic) using only iced's existing built-in widgets, styled the same way the rest of the form
already is — no new shared Material component is needed.

## Technical Context

**Language/Version**: Rust, edition 2021, no MSRV change. Sourcing/diffing uses `std::process::
Command` (already available; no dependency on `portable-pty` or `iced`), so the engine lives in
the render-free core and is exercised by `cargo test --no-default-features` (`mise run test`).

**Primary Dependencies**: `libc` (Unix-only, `[target.'cfg(unix)'.dependencies]`) — added during
implementation for a direct `kill(2)` syscall to terminate a whole process group on timeout
(research R2; a sourced rc file may background a process that outlives a single-process kill).
Already present transitively; this pins it as a direct dependency. Discovered necessary only after
shelling out to the `kill(1)` binary proved unreliable in the sandboxed development environment —
see research R2's implementation note. `tempfile` (already a `[dev-dependencies]` entry, used by
`settings_scrollback.rs`'s temp-store tests) is reused to write real disposable include scripts for
integration tests.

**Storage**: Local-first (Principle IV). Three new fields on the existing
`Settings`/`StoredSettings` JSON document (`src/settings.rs`) — `env_include_enabled: bool`,
`env_include_script_path: String`, `env_include_timeout_secs: u64` — all `#[serde(default)]`, plus
a `SETTINGS_VERSION` bump `2 → 3` (doc-comment-only; nothing branches on the number, matching how
`scrollback_lines`'s addition was recorded). The resolved environment values and any failure
diagnostic text are held only in memory on `App` — never written to this document or any log
file (FR-008/FR-013, SC-005).

**Testing**: `cargo test --no-default-features` covers: the pure env-diff/parse functions, the
`Settings`/`StoredSettings` roundtrip and clamping for the three new fields (extending
`settings_roundtrip.rs`/`settings_scrollback.rs`'s existing pattern), and the default-path
resolution function (extending `shell_command.rs`'s `#[cfg(windows)]` / `#[cfg(not(windows))]`
split). A new integration test spawns a real disposable `bash`/`powershell.exe` process against a
`tempfile`-written script to exercise the actual sourcing engine end-to-end (success, missing
script, non-zero exit, timeout) — each platform's branch only compiles (and so only runs) on its
matching CI runner, exactly like `shell_command.rs` already does for `default_shell_command`.
GUI-gated tests (`--features gui`) cover the new `SettingsDraft` fields and `settings_form.rs`
wiring. Manual end-to-end validation via `quickstart.md`.

**Target Platform**: Desktop — Linux, macOS, Windows (Principle VI, CI on all three). The
interpreter choice (`bash --noprofile --norc` vs. `powershell.exe`) and the default script path
are the only platform-varying pieces, isolated behind one function pair mirroring
`default_shell_command`'s existing `cfg!(windows)` split (research R1/R7).

**Project Type**: Desktop application (single Rust project; render-free lib core + gui binary) —
unchanged from every prior feature.

**Performance Goals**: Resolving the include script must never block session launch by more than
the configured timeout (10s default, SC-003). Once resolved, every actual session/terminal launch
reads the already-cached in-memory snapshot with no additional I/O (FR-007) — the sourcing cost is
paid at most twice per app run under normal use (once at boot, once on an explicit refresh
trigger), not per session.

**Constraints**: App functionality stays fully offline/local-first; the include script itself may
do anything (including network calls, e.g. a version-manager init block), but the app's own
behavior around it stays bounded and local. `EnvIncludeOutcome` is an enum so an invalid
combination (e.g. "failed but has no diagnostic text") is unrepresentable at the type level
instead of guarded by a runtish flag combination.

**Scale/Scope**: One shared resolved-environment snapshot per app run (not per session, per spec
Key Entities) — this is app-level configuration resolution, not new per-session state, so it does
not change feature 006's existing "handful of concurrent sessions" scale assumption.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: The env-diff/parse functions, the `Settings` roundtrip
  and clamping, and the default-path resolution are pure or std-only and land in the render-free
  core — tested first under `--no-default-features`, exactly like `default_shell_command` and
  `clamp_scrollback` were for features 006/010. The actual sourcing engine (spawns a real
  disposable process) gets a real, non-mocked integration test (per FR-005's explicit "actually
  sourcing it... not by parsing text" mandate) using `tempfile`, mirroring how this codebase
  already tests `default_shell_command` for real per-platform behavior rather than mocking `cfg!`.
- [x] **II. Multi-Session Support**: The resolved-environment snapshot is deliberately **not**
  per-session state — it is app-level configuration resolution shared by every session by design
  (spec Key Entities), the same way `scrollback_lines` is a single `App` field applied uniformly
  to every newly spawned terminal. No session's data leaks into another session; sessions remain
  independently addressable and unaffected by this feature.
- [x] **III. Worktree Integration**: Unaffected — the include script resolves environment
  *variables*, not `cwd`; every session's worktree/Default-root resolution is untouched.
  `LaunchSpec.cwd` and `spawn_shell_pty`'s `cwd` parameter are not touched by this feature.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: Only the enabled flag, script path, and
  timeout are persisted, to the existing local JSON settings file. Captured environment values
  and failure diagnostic text (which may contain secrets the script printed) are held only in
  memory for the running app instance (FR-008/FR-013) and never written to disk, a log file, or
  transmitted anywhere — strictly stronger than the baseline requirement.
- [x] **V. Rust + iced Stack**: Rust + `std::process` (+ `libc` on Unix, for a single `kill(2)`
  syscall — research R2) only; no new GUI framework. `EnvIncludeOutcome` is an enum precisely so
  "failed with no diagnostic" or "succeeded but has a failure category" cannot be constructed.
- [x] **VI. Cross-Platform Parity**: The one OS-varying behavior — which interpreter sources the
  script, and the default path — is isolated behind a small function pair mirroring
  `default_shell_command`'s existing `cfg!(windows)` split (research R1/R7), covered by CI on all
  three platforms (bash ships on the Linux/macOS runners already used; `powershell.exe` ships
  built into every Windows runner — no new CI setup required).
- [x] **VII. Documentation First-Class**: `docs/user-guide/settings.md` gains an "Environment
  include" section (default, range/behavior, persistence caveat, failure display, restart-refresh)
  in the same change, mirroring the existing "Terminal scrollback limit" section's structure;
  verified by the CI docs build.
- [x] **VIII. Reusable UI Component Foundation**: `settings_form.rs` already builds its fields
  from iced's own built-in `text_input`/`button`/`column`/`container` widgets styled via the
  shared `style::` helpers, not custom Material-wrapped structs per field — the new checkbox and
  two text inputs follow that exact existing precedent (a new `style::checkbox(r)` helper
  alongside the existing `style::input`/`style::filled`/`style::outlined`), so no one-off widget
  is forked and no existing shared primitive is duplicated. The Modal wrapper itself (`Modal`,
  already a builder-API component) is reused unchanged.

**Result: PASS — no violations. Complexity Tracking left empty.**

## Project Structure

### Documentation (this feature)

```text
specs/011-env-include-script/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── env-include-resolution.md   # sourcing/diffing/timeout/categorization contract
│   ├── settings-schema-addition.md # new persisted fields, defaults, back-compat
│   └── settings-ui.md              # grouped Settings-modal fields, failure display, refresh triggers
├── checklists/
│   └── requirements.md  # (from /speckit-specify + /speckit-clarify)
└── tasks.md              # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── env_include.rs     # NEW (pure/core, not gui-gated): EnvIncludeOutcome enum, pure env-dump
│                       #   parse + diff functions, and the impure resolve() orchestrator (stat
│                       #   the path, spawn baseline + attempt subprocesses with a timeout,
│                       #   categorize the outcome) — contracts/env-include-resolution.md
├── settings.rs         # extend: Settings/StoredSettings gain env_include_enabled,
│                       #   env_include_script_path, env_include_timeout_secs (all
│                       #   #[serde(default)]); SETTINGS_VERSION 2 → 3; new
│                       #   DEFAULT_ENV_INCLUDE_* consts + clamp_env_include_timeout; a
│                       #   default_env_include_path(home) helper mirroring
│                       #   terminal.rs::default_shell_command's platform split
│                       #   (contracts/settings-schema-addition.md)
├── app.rs              # extend: SettingsDraft gains env_include_enabled/path/timeout editable
│                       #   fields; Message gains SettingsEnvIncludeEnabledToggled/PathChanged/
│                       #   TimeoutChanged; pure reducers mirror SettingsScrollbackChanged
├── main.rs             # extend: App gains a resolved-environment snapshot (Vec<(String,String)>
│                       #   + EnvIncludeOutcome), computed once in boot() (mirroring the existing
│                       #   synchronous discover_worktrees/GitCli precedent); launch_spec() and
│                       #   ensure_attached_process's Regular branch merge it in (TERM hardcoded
│                       #   last, so it always wins per FR-009); Message::SettingsSaved and
│                       #   Message::TerminalRestartRequested both call a new
│                       #   refresh_env_include(app) helper before proceeding
└── ui/
    └── settings_form.rs # extend: a grouped "Environment include" sub-section (checkbox + two
                          #   text inputs, new style::checkbox(r) helper) plus a read-only
                          #   failure-diagnostic block shown when the last resolution failed
                          #   (contracts/settings-ui.md)

tests/
├── env_include_diff.rs     # NEW (pure): env-dump parsing + baseline/attempt diffing
├── env_include_resolve.rs  # NEW (integration, real subprocess via tempfile): success / missing
│                            #   script / non-zero exit / timeout, per-platform #[cfg] split
│                            #   mirroring shell_command.rs
├── settings_roundtrip.rs   # extend: new fields' serde default/roundtrip/back-compat
├── settings_scrollback.rs  # (unchanged; new fields get their own assertions alongside, or a
│                            #   new settings_env_include.rs sibling — task-level choice)
└── shell_command.rs        # extend or sibling: default_env_include_path per-platform tests

docs/user-guide/
└── settings.md          # extend: new "Environment include" section (Principle VII)
```

**Structure Decision**: Preserve the render-free-core + gui-binary layout unchanged. The new
sourcing/diffing engine is pure/std-only, so it lands as a new core module (`src/env_include.rs`)
rather than gui-gated code, keeping it testable under `cargo test --no-default-features` exactly
like `terminal.rs`'s `default_shell_command`. Persistence extends the existing `Settings` file in
place (no new store, no schema migration machinery). The UI addition is one grouped sub-section
inside the existing Settings modal — no new modal, no new shared component. (One small,
Unix-only crate — `libc` — was added during implementation for process-group cleanup; see
Technical Context and research R2.)

## Complexity Tracking

*No constitution violations — no entries.*
