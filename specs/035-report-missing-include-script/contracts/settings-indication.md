# Contract: Settings Indication and Save-Time Notification (client)

This changes 011's `contracts/settings-ui.md`, rendering step 4 ("nothing is shown" while the
feature is off), as spec 035 requires. That contract belongs to feature 011, which is Closed, so it
is not edited. This file is the delta.

## 1. Reducer (`crates/micold-client/src/features/settings.rs`)

New `Msg` variants, both routed through the existing `Message::Settings` wrapper:

```rust
/// A check of the stored script path is starting (spec 035 FR-009). Bumps the sequence number.
/// `path` and `enabled` are the stored values it checks.
ScriptPathCheckStarted { origin: CheckOrigin, path: String, enabled: bool },
/// A check finished. `result` is `None` for a blank path.
ScriptPathChecked { seq: u64, origin: CheckOrigin, result: Option<CheckedScriptPath> },
```

| # | Given | Message | Then |
|---|---|---|---|
| S1 | any | `ScriptPathCheckStarted { origin, path, enabled }` | `script_check_seq += 1`; `script_check = Pending { seq, last }`, where `last` is the previous answer, if any, and only if it has the same `path` and `enabled` (another window's save may have changed them, T3); if `origin == Saved`, `script_check_save_seq = Some(seq)` |
| S2 | `script_check_seq == seq` | `ScriptPathChecked { seq, result: Some(c) }` | `script_check = Done(c)` |
| S3 | `script_check_seq == seq` | `ScriptPathChecked { seq, result: None }` | `script_check = Idle` |
| S4 | `script_check_seq != seq` | `ScriptPathChecked { .. }` | `script_check` unchanged |
| S5 | `origin == Saved`, `script_check_save_seq == Some(seq)`, `c.state` is `NotFound{..}` or `NotReadable` | `ScriptPathChecked` | `update` returns `notifications::info(..)` of `save_notice(&c)` once (an `Outcome`, never a direct `notify_info`); `script_check_save_seq = None` (runs whether or not S2 or S4 applied) |
| S6 | `origin == Saved`, state `Present`, `Relative` or `Unchecked`, or `result: None` | `ScriptPathChecked` | no notification; `script_check_save_seq = None` if it matched |
| S7 | `origin == Opened` | `ScriptPathChecked` | never a notification (FR-007) |
| S8 | any | either message | no setting and no draft field changes (FR-008, FR-010) |

`save_notice(&CheckedScriptPath) -> Option<String>` wording (research R6); it answers `None` for the S6 states, so one match decides both whether and what to report:

- `NotFound { tilde: false }`: `The environment-include script was not found: <path>`
- `NotFound { tilde: true }`: the same, followed by ` (~ is not expanded; use a full path)`
- `NotReadable`: `The environment-include script is not a readable file: <path>`

## 2. Notice (`script_path_notice(check: &ScriptCheck, last: &EnvIncludeOutcome) -> Vec<NoticeLine>`)

A pure function in `features/settings.rs`. It returns every line shown below the timeout field,
including 011's failure note. `ui/settings/environment.rs`'s private `failure()` is removed, and
the page renders these lines in order: `Caution` through `ui::settings::caution`, `Note` through
`ui::settings::note`.

Wording keys:

- **OFF**: `Environment include is off, so no script is sourced. Turning it on will not source one until this path names a readable file.`
- **ON**: `Environment include is on, but the script cannot be sourced until this path names a readable file.`
- **TILDE**: `~ is not expanded. Use a full path.`
- **REL**: `Relative path: whether the script is found depends on each session's directory.`
- **HUNG**: `No answer within 2 seconds. The file may be on a drive that is not responding.`
- **011(o)**: 011's lines for outcome `o`, unchanged except that `MissingScript` now reads
  `Caution("Script not found: <path>")` whenever the check's path is known, and
  `Caution("Script not found")` otherwise. `NonZeroExit` and `TimedOut` keep their category
  caution and their diagnostic `Note`. `Disabled` and `Success` produce nothing.

`P` is `CheckedScriptPath.path`, `E` is `CheckedScriptPath.enabled`.

| # | `check` | `E` | `last` | Lines, in order |
|---|---|---|---|---|
| N1 | `Idle`, or `Pending { last: None }` | – | any | 011(last). `Pending { last: Some(c) }` renders as `Done(c)` (N2–N11), so a re-check does not blank the notice |
| N2 | `Done(NotFound{tilde:false})` | off | any | `Caution("Script not found: P")`, `Note(OFF)` |
| N3 | `Done(NotFound{tilde:false})` | on | `MissingScript` | `Caution("Script not found: P")`, `Note(ON)`. 011's line is merged, not repeated (FR-005) |
| N4 | `Done(NotFound{tilde:false})` | on | other | `Caution("Script not found: P")`, `Note(ON)`, 011(last) |
| N5 | `Done(NotFound{tilde:true})` | any | any | as N2–N4, with `Note(TILDE)` right after the caution |
| N6 | `Done(NotReadable)` | off | any | `Caution("Not a readable file: P")`, `Note(OFF)` |
| N7 | `Done(NotReadable)` | on | any | `Caution("Not a readable file: P")`, `Note(ON)`, 011(last), except that a `MissingScript` line is merged, not added (a path in an unreadable directory: FR-005). For a directory, that is 011's non-zero-exit note (Edge Cases: both notes) |
| N8 | `Done(Relative)` | any | any | `Note(REL)`, 011(last) |
| N9 | `Done(Unchecked)` | any | any | `Caution("Couldn't check the script path: P")`, `Note(HUNG)`, 011(last) |
| N10 | `Done(Present)` | on | `MissingScript` | `Caution("The last attempt could not find the script")`, `Note("P exists now. Save Settings or restart a session to source it.")` (FR-014) |
| N11 | `Done(Present)` | any other | any other | 011(last) |

**Interim, M1–M2.** M1 ships the feature-off rows. Until M3 lands, `script_path_notice` returns
exactly 011(last) whenever `E` is on (today's page, with no path line), so the feature-on page
never shows "Script not found" twice or loses 011's note between the merges. Until then, 011(o)
keeps 011's wording unchanged everywhere it appears (`Script not found`, with no path), in the
off-state rows N8 and N9 too. T024 makes the `MissingScript` line path-aware in M3. M3 (T024) removes that
gate and adds rows N3, N4, N5 (on), N7 and N10.

Invariants the tests assert:

- No row shows "Script not found" twice (FR-005).
- Every not-found or not-readable row names `P` and contains exactly one of OFF or ON (FR-002).
- The same `P` gives the same first caution whether `E` is on or off (SC-003).
- No row offers an action or a control (FR-008).

## 3. Triggers (shell)

`shell::env_include` splits the trigger in two, so tests can run the check without an executor:

1. `prepare_script_path_check(app: &mut App, origin: CheckOrigin) -> ScriptPathCheckJob`:
   dispatches `Message::Settings(Msg::ScriptPathCheckStarted { origin, path, enabled })` with the stored values, reads
   `seq = app.core.settings.script_check_seq`, and captures `app.env_include_script_path`,
   `app.env_include_enabled` (the stored values, research R8) and `app.caps.script_path_probe()`.
2. `ScriptPathCheckJob::run(self) -> Message`: `check_bounded(probe, path,
   SCRIPT_PATH_CHECK_BOUND)`, returned as `Msg::ScriptPathChecked { seq, origin, result }`. It is
   synchronous, and the tests call it directly.
3. `run_script_path_check(job) -> Task<Message>`: `Task::perform` over
   `tokio::task::spawn_blocking(move || job.run())`. A call site starts a check with
   `run_script_path_check(prepare_script_path_check(app, origin))`; Settings opening prepares the
   job inside `persist::open_settings`, so a test can run it synchronously. A `JoinError` becomes `result:
   Some(CheckedScriptPath { path, enabled, state: Unchecked })`.

| # | Call site | Origin | Condition |
|---|---|---|---|
| T1 | `shell::persist::on_settings_opened` | `Opened` | always. Its `Task` is returned, and the draft is seeded before the check lands (FR-006) |
| T2 | `shell::persist::apply_save`, after the write, 011's refresh and `Msg::Saved` | `Saved` | always (FR-004, FR-009), batched with any survival task |
| T3 | `shell::daemon_sync` `DaemonMsg::SettingsChanged` arm | `Opened` | only while `app.core.settings.settings_draft.is_some()` |

It is **never** called from the session-create, session-start, terminal-restart or boot paths
(FR-006, SC-004).

## 4. Capability (`shell/capabilities.rs`)

`script_path_probe: Arc<dyn ScriptPathProbe + Send + Sync>`, constructed in `Capabilities::real()`
as `StdScriptPathProbe`, with the accessor `script_path_probe(&self) -> Arc<…>` (owned, because
the consumer is a blocking task) and a `#[cfg(test)] with_script_path_probe(...)` narrowing.
The guards only see a port that is registered, so two guard edits ship with it:
`"ScriptPathProbe"` is added to `PORTS` in `crates/micold-client/tests/inventory/mod.rs`, and
`"FakeScriptPathProbe"` is added to the closed `known` fakes list in
`crates/micold-client/tests/no_concrete_implementations.rs`. Then `no_concrete_implementations.rs`
(`StdScriptPathProbe` is named only in `real()` and at its definition) and
`service_capability_fakes.rs` (the port has a core fake, and a test exercises it) must pass.
