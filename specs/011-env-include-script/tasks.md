# Tasks: Environment-Include Script

**Input**: Design documents from `/specs/011-env-include-script/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md (all present)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY for every genuinely pure/testable unit of logic. The sourcing/diffing engine
(`src/env_include.rs`) needs no `iced`/`portable-pty`, so — unlike prior features' real-PTY glue —
it gets **real, non-mocked** integration tests (spawning actual disposable `bash`/`powershell.exe`
processes via `tempfile`), per FR-005's explicit "actually sourcing it, not parsing text" mandate.
Only the binary-only wiring in `src/main.rs` (`boot()`, `launch_spec()`,
`ensure_attached_process`) and the GUI rendering in `src/ui/settings_form.rs` have no practical
automated test in this codebase today — same precedent as feature 010's `main.rs`/`ui/` glue.

**Documentation**: Per Constitution Principle VII, every user-facing user story ships its
user-guide update (`docs/user-guide/settings.md`) in the same change.

**Cross-platform**: Per Constitution Principle VI, the one platform-varying piece (which
interpreter sources the script, and the default path) is isolated behind
`default_env_include_path` + the two subprocess-invocation call sites (research R1/R6/R7) and
covered by CI on Linux, macOS, and Windows via `#[cfg(windows)]`/`#[cfg(not(windows))]`-split
tests, mirroring `tests/shell_command.rs`.

**Organization**: Tasks are grouped by user story (spec.md priorities: US1 P1, US2 P2, US3 P3).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1 / US2 / US3, per spec.md
- File paths are exact

---

## Phase 1: Setup

**Purpose**: Register the one new module every later task depends on.

- [X] T001 Create `src/env_include.rs` (empty module skeleton) and add `pub mod env_include;` to
  `src/lib.rs` (alongside the other core modules — not gui-gated, since this module only needs
  `std::process`, per research R4).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The sourcing/diffing engine (`src/env_include.rs`) and the persisted settings shape
every user story sits on top of. No user story is independently testable until this phase is
green.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests for Foundational (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementation.

- [X] T002 [P] Write failing pure tests in `tests/env_include_diff.rs` (NEW,
  `--no-default-features`): `parse_env_dump` correctly splits NUL-delimited `KEY=VALUE` pairs
  (including a value that itself contains `=`); `diff_env(baseline, attempt)` reports keys that
  are new or changed in `attempt`, and does **not** report a key present in `baseline` but absent
  from `attempt` (research R3, contracts/env-include-resolution.md).
- [X] T003 [P] Write failing integration tests in `tests/env_include_resolve.rs` (NEW, real
  subprocess via `tempfile`-written scripts, `#[cfg(not(windows))]`/`#[cfg(windows)]` split
  mirroring `tests/shell_command.rs`): a script exporting a new variable → `Success` with that
  variable in the returned `Vec`; a nonexistent path → `MissingScript` with an empty `Vec`; a
  script that `exit 1`s (after printing something) → `NonZeroExit { code: 1, .. }` with that
  output captured as the diagnostic; a script that sleeps past a short test timeout → `TimedOut`,
  returning at approximately the timeout bound, not after the sleep completes; an empty script →
  `Success` with an empty `Vec` (contracts/env-include-resolution.md's full test-obligations list).
- [X] T004 [P] Write failing tests in `tests/settings_roundtrip.rs` (extend existing file): the
  three new fields (`env_include_enabled`, `env_include_script_path`, `env_include_timeout_secs`)
  default correctly when absent from a pre-011 JSON document; `SETTINGS_VERSION` is `3`; a full
  roundtrip through `StoredSettings::from_settings`/`into_settings` preserves all three
  (contracts/settings-schema-addition.md).
- [X] T005 [P] Write failing tests in `tests/settings_env_include.rs` (NEW, mirrors
  `tests/settings_scrollback.rs`'s structure): `clamp_env_include_timeout` clamps to `1..=60`
  seconds (e.g. `0` → `1`, `5` → `5`, `999` → `60`); an out-of-range persisted timeout is clamped
  on load; a corrupt settings file degrades to defaults for all three new fields, same as it
  already does for `scrollback_lines`.
- [X] T006 [P] Write failing per-platform tests in `tests/shell_command.rs` (extend, reusing its
  existing `#[cfg(windows)]`/`#[cfg(not(windows))]` module split): `default_env_include_path(home)`
  returns `<home>/.bashrc` on Unix and `<home>/Documents/WindowsPowerShell/profile.ps1` on Windows
  (research R7).

### Implementation for Foundational

- [X] T007 [P] Implement `EnvIncludeOutcome` enum (`Disabled`, `Success`, `MissingScript`,
  `NonZeroExit { code: i32, diagnostic: String }`, `TimedOut { diagnostic: String }`) in
  `src/env_include.rs` (data-model.md). First edit to this new file.
- [X] T008 Implement pure `parse_env_dump(bytes: &[u8]) -> HashMap<String, String>` and
  `diff_env(baseline: &HashMap<String, String>, attempt: &HashMap<String, String>) ->
  Vec<(String, String)>` in `src/env_include.rs` (same file as T007 — sequential, not `[P]`);
  makes T002 pass (research R3/R6).
- [X] T009 Implement the impure `resolve(path: &Path, timeout: Duration) -> (Vec<(String, String)>,
  EnvIncludeOutcome)` orchestrator in `src/env_include.rs` (same file as T007/T008 — sequential):
  Rust-side existence check short-circuits to `MissingScript` with no subprocess spawned; baseline
  via `bash --noprofile --norc -c 'env -0'` (Unix) / the PowerShell equivalent (Windows, research
  R6); attempt via the `out=$(source "$1" 2>&1); status=$?; printf '%s' "$out" >&2; env -0; exit
  $status` wrapper (research R1), spawned with `Command::spawn()` and polled via `try_wait()`
  bounded by `timeout` (research R2), `kill()`+`TimedOut` on expiry; depends on T007, T008; makes
  T003 pass (contracts/env-include-resolution.md).
- [X] T010 Implement pure `merge_with_term(vars: &[(String, String)]) -> Vec<(String, String)>` in
  `src/env_include.rs` (same file — sequential): appends the hardcoded `("TERM",
  "xterm-256color")` pair *last*, so it always wins on key collision regardless of whether `vars`
  itself already contains a `"TERM"` entry (FR-009). Add a pure unit test alongside T002 asserting
  this precedence explicitly (both when `vars` has no `TERM` and when it has a conflicting one).
- [X] T011 [P] Add the three new fields to `Settings`/`StoredSettings` in `src/settings.rs`, each
  `#[serde(default = "...")]`; bump `SETTINGS_VERSION` `2 → 3` and extend its doc comment (matching
  the existing "bumped to 2 when scrollback_lines was added" convention); makes T004 pass
  (contracts/settings-schema-addition.md). Different file from T007–T010 — safe to start in
  parallel with that stream.
- [X] T012 Add `clamp_env_include_timeout`, `DEFAULT_ENV_INCLUDE_ENABLED`/
  `DEFAULT_ENV_INCLUDE_TIMEOUT_SECS` (`= 10`)/`MIN_ENV_INCLUDE_TIMEOUT_SECS` (`= 1`)/
  `MAX_ENV_INCLUDE_TIMEOUT_SECS` (`= 60`) consts, and
  `default_env_include_path(home: Option<&Path>) -> PathBuf` (research R7) to `src/settings.rs`
  (same file as T011 — sequential); makes T005/T006 pass.
- [X] T013 Add `EnvIncludeSnapshot { vars: Vec<(String, String)>, outcome: EnvIncludeOutcome }`
  and a new `App.env_include: EnvIncludeSnapshot` field in `src/main.rs` (gui-only), computed once
  in `boot()`: read the three settings, and when enabled with a non-blank path call
  `env_include::resolve(path, timeout)` (else `Disabled` with empty `vars`, no subprocess spawned)
  — depends on T009, T011, T012. *(No practical automated test — `boot()`/`App` are binary-only,
  matching feature 010's `main.rs`-glue precedent; validated by quickstart.md.)*

**Checkpoint**: `cargo test --no-default-features` and `cargo test --features gui` both pass; the
app builds and runs exactly as before — `App.env_include` is computed at startup but nothing yet
reads it in a spawn call site, so there is no user-visible behavior change yet.

---

## Phase 3: User Story 1 - Sessions automatically pick up my shell environment (Priority: P1) 🎯 MVP

**Goal**: Both the `claude` AI CLI process and the regular-terminal shell process are launched
with the variables resolved from the include script, merged with the hardcoded `TERM` pair.

**Independent Test**: Add an exported variable and a `PATH` prepend to the configured include
script (default `~/.bashrc`); open a new session; confirm the variable and the `PATH` change are
visible in both the AI CLI process's environment and the regular-terminal shell's environment for
that session.

### Tests for User Story 1

> No new automated test: T003 (Foundational) already proves `resolve()` captures variables
> correctly per platform, and T010 already proves `merge_with_term`'s TERM-wins precedence. This
> phase's job is wiring those already-tested pure/impure units into `src/main.rs`'s two spawn call
> sites, which — like all `main.rs` process-spawning glue in this codebase (feature 010's `T018`
> precedent) — has no practical automated test; validated by `quickstart.md` Scenario 1.

### Implementation for User Story 1

- [X] T014 [US1] In `src/main.rs`'s `launch_spec()`, replace the hardcoded
  `env: vec![("TERM", "xterm-256color".to_string())]` with
  `env: env_include::merge_with_term(&app.env_include.vars)`, threading `app`'s resolved snapshot
  into `launch_spec`'s parameters (its call site in `ensure_attached_process`'s `AiCli` branch
  already has `app` in scope) — depends on T013. **Confirm FR-010 while making this edit**:
  `portable_pty::CommandBuilder::env(k, v)` overrides a same-named variable already present in the
  inherited environment (not just adds new ones) — this is what makes a captured value win over
  the app process's own pre-existing environment for that key (FR-010), not merely an assumption.
  If this codebase's `portable-pty` version behaves differently, add an explicit override here.
- [X] T015 [US1] In `ensure_attached_process`'s `TerminalMode::Regular` branch (`src/main.rs`),
  replace the hardcoded `let env = vec![("TERM", "xterm-256color".to_string())];` with
  `let env = env_include::merge_with_term(&app.env_include.vars);` — depends on T013; same file as
  T014, so sequential (not `[P]`) despite neither depending on the other's output.
- [X] T016 [P] [US1] Add an "Environment include" section to `docs/user-guide/settings.md`
  describing the default auto-include behavior (on by default, default path per platform, that
  both the AI CLI and regular-terminal sessions see the identical resolved set) — Principle VII.
  Different file from T014/T015 — safe to run in parallel with them.

**Checkpoint**: A fresh install with no prior settings auto-includes the default script into both
process kinds with no user action; `quickstart.md` Scenario 1 passes. MVP demoable.

---

## Phase 4: User Story 2 - Configure or turn off environment-include (Priority: P2)

**Goal**: The enabled flag, script path, and timeout are visible and editable as one grouped
section in the existing Settings interface; saving a change refreshes the resolved snapshot
immediately.

**Independent Test**: Open Settings, change the script path to a different file (or clear/disable
the feature), save, open a new session, and confirm the new path was used (or that no include
script ran).

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementation.

- [X] T017 [P] [US2] Write failing pure tests in `tests/app_state.rs` (extend existing file,
  `--no-default-features`): `Message::SettingsEnvIncludeEnabledToggled`/`PathChanged`/
  `TimeoutChanged` each update the corresponding `SettingsDraft` field and leave the others
  untouched (contracts/settings-ui.md).

### Implementation for User Story 2

- [X] T018 [US2] Extend `SettingsDraft` in `src/app.rs` with `env_include_enabled: bool`,
  `env_include_script_path: String`, `env_include_timeout: String`; add the three new `Message`
  variants and their pure reducers — depends on T011; makes T017 pass
  (contracts/settings-ui.md).
- [X] T019 [US2] In `src/main.rs`'s `Message::SettingsOpened` handler, seed the three new draft
  fields from the app's current settings values, alongside the existing
  `draft.scrollback_lines = app.scrollback_lines.to_string()` line — depends on T018.
- [X] T020 [US2] Implement `refresh_env_include(app: &mut App)` in `src/main.rs`: reads the
  current enabled/path/timeout, calls `env_include::resolve()` (or sets `Disabled` with empty
  `vars` when off or the path is blank — no subprocess spawned), and replaces `app.env_include`
  wholesale — depends on T013.
- [X] T021 [US2] Extend `Message::SettingsSaved`'s handler in `src/main.rs`: parse and validate
  the timeout text field the same way the existing scrollback value is (reject with `draft.error`
  and keep the overlay open on parse failure or out-of-range value — never silently substitute a
  default); on success, persist all three new fields via the existing `store.save(...)` call, then
  call `refresh_env_include(app)` (T020) — depends on T018, T020; same file as T019 (sequential).
- [X] T022 [US2] Add the grouped "Environment include" sub-section to
  `src/ui/settings_form.rs::modal`: a new `style::checkbox(r)` helper (alongside the existing
  `style::input`/`style::filled`/`style::outlined`), a `checkbox("Enabled", ...)` bound to
  `Message::SettingsEnvIncludeEnabledToggled`, and two `text_input`s (path, timeout) bound to their
  respective `Message` variants — ordered/spaced as one visually distinct group, separate from the
  scrollback field (FR-015) — depends on T018 (contracts/settings-ui.md).
- [X] T023 [P] [US2] Extend `docs/user-guide/settings.md`'s "Environment include" section (T016)
  with the three fields themselves (default values, that the path accepts any string with no
  format validation, that the timeout is clamped) and the persistence caveat (only the flag/path/
  timeout are ever saved — captured values and diagnostics never are, SC-005) — Principle VII.

**Checkpoint**: A user can view, edit, and disable all three settings from the existing Settings
interface, grouped together; a saved change takes effect on the next session without an app
restart; `quickstart.md` Scenarios 2 and 4 pass.

---

## Phase 5: User Story 3 - Broken or slow include scripts never block a session (Priority: P3)

**Goal**: A missing, failing, or hanging script never delays or fails session launch beyond the
configured timeout, and the most recent failure is visibly discoverable and recoverable without
restarting the app.

**Independent Test**: Point the configured path at a nonexistent file, then at a script that
`exit 1`s partway through, then at a script that hangs; in all three cases, confirm session launch
still completes promptly and the session is otherwise usable, and that the failure is visible and
recoverable via the existing per-session restart control.

### Tests for User Story 3

> No new automated test: T003 (Foundational) already proves `resolve()`'s four-way categorization
> — including `TimedOut` firing at the configured bound rather than waiting out a hung script — and
> T013/T020 already guarantee a failed resolution yields a usable (if empty) `vars`, so session
> launch degrading gracefully falls out of the existing type-level design rather than being a new
> testable property. This phase's remaining work — one line of `main.rs` glue and a conditional UI
> block — has no practical automated test (binary-only / GUI rendering); validated by
> `quickstart.md` Scenario 3.

### Implementation for User Story 3

- [X] T024 [US3] In `src/main.rs`'s `Message::TerminalRestartRequested` handler, call
  `refresh_env_include(app)` (T020) before the existing `ensure_attached_process(app, id)` call —
  depends on T020 (spec Clarifications: the manual restart control also triggers a fresh
  re-source).
- [X] T025 [US3] In `src/ui/settings_form.rs::modal`, add a read-only failure-diagnostic block
  rendered only when `app.env_include.outcome` is `MissingScript`/`NonZeroExit`/`TimedOut`: the
  failure category as a short label ("Script not found" / "Exited with an error" / "Timed out")
  followed by the diagnostic text (when present) — depends on T022 (contracts/settings-ui.md,
  FR-012/FR-013).
- [X] T026 [P] [US3] Extend `docs/user-guide/settings.md` with the failure-visibility behavior
  (where the failure note appears, what each category label means, that captured script output is
  shown verbatim for troubleshooting) and the restart-control recovery path (fix the script, use
  the session's existing restart control to refresh without restarting the app) — Principle VII.

**Checkpoint**: Every failure mode (missing/non-zero/timeout) is bounded, visible in Settings, and
recoverable via the existing restart control. All `quickstart.md` scenarios pass.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that span all three stories.

- [X] T027 [P] Cross-cutting documentation review: confirm `docs/user-guide/settings.md`'s new
  "Environment include" section (T016/T023/T026) reads consistently with the existing "Terminal
  scrollback limit" section; update any docs index/navigation that lists Settings sections. Also
  add `docs/user-guide/settings.md` (and the other currently-uncovered `worktrees-and-sessions.md`/
  `icons.md`) to `.github/workflows/ci.yml`'s docs-check job's file-existence list — it previously
  only checked three unrelated files, so it would not have caught a regression to the very file
  this feature's docs live in. Done: moved the shared "Click Save/Cancel" line to the end of the
  file (it governs the whole dialog, not just scrollback); updated `docs/README.md`'s Settings
  summary line to mention environment-include; `ci.yml`'s docs-check job already covered
  `settings.md` from the `/speckit-analyze` remediation pass.
- [X] T028 Verify `cargo build`/`cargo test` (both `--no-default-features` and `--features gui`)
  plus `cargo clippy --features gui --lib --bins -- -D warnings` pass on Linux, macOS, and Windows
  (Principle VI). `tests/env_include_resolve.rs`'s and `tests/shell_command.rs`'s
  `#[cfg(windows)]` branches compile and run for real only on Windows CI. Verified on Linux (this
  environment): `cargo fmt --check`, `cargo build`/`--features gui`, `cargo test
  --no-default-features --all-targets` (52 binaries) and `cargo test --features gui` (54, includes
  `main.rs`'s own `#[cfg(test)]` module), and `cargo clippy --no-default-features --all-targets` /
  `--features gui --all-targets -- -D warnings` — all clean, 0 failures, 0 warnings. Fixed two
  `App { .. }` literals in `main.rs`'s internal test module that `clippy --all-targets` (but not
  plain `cargo build`) compiles, and applied `cargo fmt`. macOS/Windows are CI-only in this
  environment (no local toolchain here; the existing CI matrix is also Linux-only today — a
  pre-existing gap, not introduced by this feature).
- [X] T029 Run `quickstart.md` end-to-end (all 4 scenarios + the Windows variant) as final manual
  validation. `cargo run --features gui` launches cleanly in this environment (`DISPLAY=:1`
  present) for a 5s smoke test with zero errors/panics. Full interactive click-through wasn't
  performed (no GUI-automation tooling in this session, matching feature 010's `T030` precedent);
  confirmed each scenario against the implementation by inspection instead: Scenario 1 (fresh
  install auto-includes) ⟸ `boot()`'s `resolve_env_include` call (T013) + FR-004 defaults
  (T011/T012) + `merge_with_term` wired into both `launch_spec` (T014) and
  `ensure_attached_process`'s `Regular` branch (T015) — real-subprocess-tested by
  `tests/env_include_resolve.rs::exported_variable_is_captured`. Scenario 2 (reconfigure/disable,
  grouped fields) ⟸ T018's draft fields + T022's grouped UI + T021's validate/save/refresh —
  pure-tested by `tests/app_state.rs`'s env-include reducer tests. Scenario 3 (broken/slow
  scripts, failure visibility, restart-recovery) ⟸ T009's `MissingScript`/`NonZeroExit`/`TimedOut`
  categorization (real-subprocess-tested) + T025's conditional failure block + T024's
  restart-triggered refresh. Scenario 4 (persistence never leaks) ⟸ `Settings`/`StoredSettings`
  structurally has only the 3 new fields (T011) — captured vars/diagnostics live only on
  `App.env_include`, never serialized — confirmed by `tests/settings_roundtrip.rs`'s roundtrip
  tests exercising exactly those 3 fields. Windows variant ⟸ T006/T009's `#[cfg(windows)]`
  branches, which will compile and run for real the next time CI executes on a Windows runner. A
  hands-on interactive pass (actually opening Settings, toggling fields, watching a real session's
  environment) is recommended before merging.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately.
- **Foundational (Phase 2)**: Depends on Setup (needs `src/env_include.rs` to exist as a module).
  BLOCKS all user stories.
- **User Stories (Phase 3+)**: All depend on Foundational (Phase 2) completion.
  - US1 (P1) needs only Foundational.
  - US2 (P2) needs only Foundational (independent of US1's `src/main.rs` edits — different
    functions, `launch_spec()`/`ensure_attached_process` vs. `SettingsOpened`/`SettingsSaved`).
  - US3 (P3) depends on US2's T020 (`refresh_env_include`) existing — build US2 before US3.
- **Polish (Phase 6)**: Depends on all three user stories being complete.

### User Story Dependencies

- **US1 (P1)**: Foundational only.
- **US2 (P2)**: Foundational only (implementable in parallel with US1 by a second developer).
- **US3 (P3)**: Foundational + US2's T020 (`refresh_env_include`).

### Parallel Opportunities

- Within Foundational: T002–T006 (tests) in parallel (5 different files); then T007 (starts the
  `env_include.rs` stream) and T011 (starts the `settings.rs` stream) in parallel — but T008/T009/
  T010 (same file as T007) and T012 (same file as T011) are each sequential within their own
  stream, not parallel with their stream's own earlier tasks.
- T013 waits on T009 + T011 + T012 (needs the engine and the settings defaults/consts).
- Once Foundational is green, US1 and US2 can proceed in parallel (different `src/main.rs`
  functions and different files overall) by two developers; US3 must wait for US2's T020.
- T014/T015 (US1, same file) are sequential; T016 (docs) can run in parallel with them.
- T019/T020/T021 (US2, same file `src/main.rs`) are sequential; T018 (`app.rs`) and T022
  (`settings_form.rs`) can each proceed in parallel with that stream once T018 itself is done;
  T023 (docs) in parallel with any of them.
- T024 (`main.rs`) and T025 (`settings_form.rs`) can proceed in parallel once T020/T022 are done
  respectively; T026 (docs) in parallel with both.

---

## Parallel Example: Foundational Tests

```bash
# Launch all Foundational test-writing tasks together:
Task: "Write failing pure tests in tests/env_include_diff.rs"
Task: "Write failing integration tests in tests/env_include_resolve.rs"
Task: "Write failing tests in tests/settings_roundtrip.rs (extend)"
Task: "Write failing tests in tests/settings_env_include.rs"
Task: "Write failing per-platform tests in tests/shell_command.rs (extend)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational (CRITICAL — blocks everything).
3. Complete Phase 3: User Story 1.
4. **STOP and VALIDATE**: `quickstart.md` Scenario 1 — a fresh install auto-includes the default
   script into both process kinds.
5. Demo if ready — this alone delivers the feature's headline value.

### Incremental Delivery

1. Setup + Foundational → sourcing/diffing engine and settings schema ready, no user-visible
   change yet.
2. Add US1 → both processes see the resolved environment → validate Scenario 1 (MVP!).
3. Add US2 → Settings UI to configure/disable → validate Scenarios 2 and 4.
4. Add US3 → failure visibility + restart-triggered recovery → validate Scenario 3.
5. Polish → cross-platform verification + full quickstart pass.

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together.
2. Once Foundational is done:
   - Developer A: User Story 1 (`launch_spec`/`ensure_attached_process` merge)
   - Developer B: User Story 2 (Settings UI + persistence)
3. Developer C starts User Story 3 once Developer B's T020 (`refresh_env_include`) lands.

---

## Notes

- [P] tasks = different files, no dependencies.
- [Story] label maps task to specific user story for traceability.
- Foundational's T007–T010 (`src/env_include.rs`) and T011–T012 (`src/settings.rs`) are two
  independent same-file-sequential streams — parallel *across* streams, sequential *within* each.
- The sourcing/diffing engine (T002/T003/T007–T010) gets real, non-mocked subprocess tests —
  unlike prior features' PTY glue, `std::process::Command` needs no `gui` feature, so this is
  fully exercisable under `cargo test --no-default-features` (mirrors `tests/shell_command.rs`'s
  existing real-per-platform-branch precedent, not a new exception).
- US2 and US3 share `refresh_env_include` (T020) — build US2's T020 before US3's T024, even though
  they're different priorities in different phases.
- Commit after each task or logical group; verify tests fail before implementing, then pass after.
