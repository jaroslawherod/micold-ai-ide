# Implementation Plan: Report a Missing Environment-Include Script

**Branch**: `fix/github-issues` | **Date**: 2026-09-29 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/035-report-missing-include-script/spec.md`

## Summary

Settings says nothing about a stored environment-include script path that names no file while
the feature is off (issue #435). With the feature on, 011 already shows "Script not found" after a
failed resolution attempt. This feature adds a **script path check**: a stat-and-open probe of the
*stored* path. It never runs the script. It classifies the path as present, not found, not a
readable file, relative (not checked) or could not be checked. It runs in the client each time
Settings opens and after every save, never on the session-launch path, and is bounded at 2 s.
The Environment page shows the result above 011's failure note, in either state of the feature,
with wording that agrees with 011's note (the two merge when they state the same fact). A save
that leaves a missing path posts an info notification naming it. Nothing is persisted, no
recovery is offered, and nothing outside Settings reports it.

Technical approach (research.md R1–R9): the classification and the bounded check are pure core
code in a new `micold_core::script_path_check` module behind a `ScriptPathProbe` capability (real
`StdScriptPathProbe` and a fake). The result lives in the client's render-free `features/settings`
state, and its reducer drops stale results and returns the save-time notification as an `Outcome`. The wording is a
pure function beside `missing_cli_notice`. The shell (`shell/persist.rs`, `shell/daemon_sync.rs`)
only starts the check on a blocking task. `ui/settings/environment.rs` renders the notice lines with
the existing `caution` and `note` primitives.

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise` (workspace edition as in `Cargo.toml`)

**Primary Dependencies**: iced 0.14 (client GUI; `Task::perform`), tokio 1.53
(`task::spawn_blocking`, already a client dependency), `std::fs` only for the probe. No new crates.

**Storage**: None added. `settings.json` keeps only `env_include_enabled`,
`env_include_script_path` and `env_include_timeout_secs` (FR-010, 011 FR-008).

**Testing**: `cargo test` via `mise run test-core` (core classification, bounded check, real probe
against temp files) and `mise run gate` (client reducer, notice wording, shell handlers in
`src/main_tests.rs`). Rendering is quickstart §B's visual pass (Constitution I exception).

**Target Platform**: Desktop client on Linux, macOS and Windows.

**Project Type**: Desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`).

**Performance Goals**: Settings content renders without waiting for the check. The indication
appears within 2 s of opening Settings (SC-004). The probe is one `metadata` call and one `open`.

**Constraints**: No path check on the session-launch path (FR-006, SC-004). The check never
executes or sources the script (FR-003). No change to 011's sourcing, cache or timeout.

**Scale/Scope**: One app-level setting. About 13 files touched (including 3 guard or test files), 1 new core module, and 1 user guide
section.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How the plan satisfies it |
|---|---|---|
| **I. Test-First** | PASS | Every decision is in tested code written test-first: classification and the 2 s bound in core (`crates/micold-core/tests/script_path_check.rs`), stale-result and notification rules in the settings reducer (`crates/micold-client/tests/features_settings.rs`, asserting on the `Outcome`s `update` returns), wording in the pure `script_path_notice`, and the shell triggers (open, save, launch never checks) in `src/main_tests.rs` with the fake probe. Only the lines in `ui/settings/environment.rs` that push already-computed notice lines into the page fall under the GUI-glue exception. Quickstart §B validates them. |
| **II. Multi-Session** | PASS | The check is about one app-level setting and holds no per-session state. It never runs during a launch and never blocks a session. Each window checks for itself when its Settings opens. A result for an older request or another path is dropped (sequence number), so a save in another window (`DaemonMsg::SettingsChanged`) cannot leave a stale answer showing. |
| **III. Worktree Integration** | PASS (N/A) | No file or VCS operation on worktrees. A relative path is deliberately not checked because its answer depends on the session's directory (spec Edge Cases). |
| **IV. Local-First** | PASS | A local `std::fs` probe. No network and no remote service. Nothing is persisted or transmitted. |
| **V. Rust + iced** | PASS | Rust and iced only. `ScriptPathState` is an enum, so "not found but readable" cannot be built, and `Option<CheckedScriptPath>` makes "blank path has a result" unrepresentable. |
| **VI. Cross-Platform** | PASS | `std::fs::metadata`, `File::open` and `Path::is_absolute` behave the same way on all three OSes. The core does not branch on the OS. The real-probe tests run on all three CI platforms; only the chmod-unreadable case is `cfg(unix)`, and a directory covers "not a readable file" everywhere. The UNC and stalled-mount hang is covered by the bound, and tested with a fake that never answers. |
| **VII. Documentation** | PASS | `docs/user-guide/settings.md` ("Environment" / "If the script fails", and the "Script path" bullet that now says a path is only checked when used) is updated in the milestone that ships the behaviour (FR-013). |
| **VIII. Reusable UI** | PASS | No new widget. The notice renders through the existing `ui::settings::caution` and `ui::settings::note` primitives already used for 011's failure note. |

No violations. Complexity Tracking is empty.

**Post-design re-check (after Phase 1)**: unchanged. The contracts add one core module, one
capability and one reducer message, and no widget, persisted field or dependency.

## Project Structure

### Documentation (this feature)

```text
specs/035-report-missing-include-script/
├── plan.md              # This file
├── research.md          # Phase 0: decisions R1–R9 with rejected alternatives
├── data-model.md        # Phase 1: ScriptPathState, CheckedScriptPath, ScriptCheck, notice lines
├── quickstart.md        # Phase 1: §A automated checks, §B visual pass
├── tdd/                 # test-list.md and cycle-log.md (speckit-tdd-plan)
├── contracts/
│   ├── script-path-check.md     # core API: classify, probe capability, bounded check
│   └── settings-indication.md   # client: triggers, reducer message, notice wording, notification
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
crates/micold-core/
├── src/lib.rs                         # + pub mod script_path_check
├── src/script_path_check.rs           # NEW: ScriptPathState, classify, ScriptPathProbe,
│                                      #      StdScriptPathProbe, FakeScriptPathProbe, check_bounded
└── tests/script_path_check.rs         # NEW: classification, real probe on temp files, the bound

crates/micold-client/
├── src/features/settings.rs           # + ScriptCheck state, Msg::ScriptPathChecked,
│                                      #   script_path_notice(), save-time notification
├── src/shell/capabilities.rs          # + script_path_probe capability (real() names StdScriptPathProbe)
├── src/shell/persist.rs               # on_settings_opened / apply_save start the check
├── src/shell/daemon_sync.rs           # SettingsChanged re-checks while Settings is open
├── src/shell/env_include.rs           # + prepare_script_path_check(app, origin) -> job; run_script_path_check(job) -> Task<Message>
├── src/main.rs                        # pass the check into the view (routing needs no change:
│                                      #   shell/settings.rs's catch-all forwards pure Msgs)
├── src/ui/mod.rs, src/ui/settings_view.rs   # thread the check through to the page
├── src/ui/settings/environment.rs     # render script_path_notice lines (caution/note) after the timeout field
├── src/main_tests.rs                  # shell trigger tests with the fake probe
├── tests/features_settings.rs         # reducer + wording tests
├── tests/inventory/mod.rs             # + "ScriptPathProbe" in PORTS
└── tests/no_concrete_implementations.rs  # + "FakeScriptPathProbe" in the known fakes

docs/user-guide/settings.md            # FR-013
```

**Structure Decision**: Existing workspace layout. Decision logic goes to `micold-core` (the
classification and the bound, which need no iced) and to the client's render-free
`features/settings.rs` (state, reducer, wording). The shell starts the work and the UI renders it,
following 021's capability pattern and 028's feature encapsulation. The daemon is not touched. Its
per-directory silent failure is #454, out of scope.

## Test strategy (which layer tests each requirement)

| Requirement | Layer | Where |
|---|---|---|
| FR-001 (readable-file definition, literal `~`, relative not checked), FR-011 (blank) | core unit | `micold-core/tests/script_path_check.rs`: `classify` with the fake probe, and the real probe on a temp file, a missing path, a directory and a `cfg(unix)` mode-000 file |
| FR-003 (never runs the script) | core unit | the real probe on a script whose execution would create a marker file; the marker is absent afterwards. The fake resolver records no call. |
| FR-006 bound (2 s → could not be checked), fast error → not readable | core unit | `check_bounded` with a fake probe that blocks; `classify` with `answering(Unreadable)` for a fast error |
| FR-002, FR-005, FR-014, Edge Cases wording | client unit (pure) | `tests/features_settings.rs`: `script_path_notice` for each (state × enabled × last outcome) row in contracts/settings-indication.md |
| FR-004 (save-time notification, every save, not for relative/unchecked/present) | client reducer | `tests/features_settings.rs`: `Msg::ScriptPathChecked` with `CheckOrigin::Saved` vs `Opened`, asserting on the returned `Outcome::NotificationRaised` |
| FR-009 (reflects the file now, after every save), stale results dropped (II) | client reducer + shell | reducer: sequence check. `main_tests.rs`: open and save each start a check with the stored path |
| FR-006 / SC-004 (no check on launch, Settings not blocked) | shell | `main_tests.rs`: a session launch makes no probe call (fake probe call count); `on_settings_opened` returns with the draft seeded and the check pending |
| FR-007, FR-008 (Settings only, no recovery) | client reducer | `Opened`-origin results post no notification. The notice has no action. The reducer changes no setting. |
| FR-010 / SC-005 (not persisted) | shell | `main_tests.rs`: after a save with a missing path, the written `Settings` equals the validated form. `Settings` gains no field (`micold-core/tests/settings_roundtrip.rs` unchanged). |
| FR-012 (three OSes) | CI | core tests run on the Linux, macOS and Windows jobs |
| FR-013 | docs | `docs/user-guide/settings.md`; CI user-guide gate |
| Rendering (order, primitives, both states) | quickstart §B | `visual-pass` skill against the client on Xvfb |

## Complexity Tracking

No Constitution Check violations.
