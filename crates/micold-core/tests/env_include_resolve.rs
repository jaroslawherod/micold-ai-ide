//! Integration tests for the environment-include sourcing/diffing engine (feature 011,
//! contracts/env-include-resolution.md). Spawns REAL disposable subprocesses via `tempfile`-
//! written scripts — no mocking, per FR-005's "actually sourcing it, not parsing text" mandate.
//! Each platform's module only compiles (and so only runs) on its matching CI runner, mirroring
//! `tests/shell_command.rs`.

use micold_core::env_include::{resolve, EnvIncludeOutcome};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

fn write_script(dir: &tempfile::TempDir, name: &str, contents: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    path
}

/// Serialises the tests in this binary that actually spawn a shell — on Windows only.
///
/// # The flake this fixes, and why it is the harness rather than the code
///
/// Eight tests here spawn a shell, and [`resolve`] spawns **twice** per call: once to establish the
/// baseline environment, once to source the script, both out of one shared budget (BUG-003). Cargo
/// runs them in parallel, so a two-core Windows runner was starting up to sixteen cold
/// `powershell.exe` processes at once, first launch of the session, with Defender scanning each.
/// Four tests then failed together on the *baseline* — `TimedOut { diagnostic: "the timeout
/// expired while establishing the baseline environment…" }` — having never reached the script.
///
/// That contention is an artifact of the test harness, not of the thing under test: the client
/// resolves one directory at a time and never races itself. The unix module has the same shape with
/// a *tighter* five-second budget and has never flaked, because `bash` starts in milliseconds. The
/// variable is PowerShell's cold-start cost under contention, so the fix belongs here.
///
/// # Why serialising rather than raising the budgets
///
/// Because for two of these tests a bigger number provably cannot work.
/// [`a_hanging_script_costs_its_own_budget_and_not_the_baselines_as_well`] asserts
/// `elapsed < timeout * 4`, so its budget *is* the thing under test — raising it raises the ceiling
/// proportionally, while the contended cold-start cost is an additive term that scales with nothing.
/// Its margin is `3 × timeout` minus the baseline start, and no choice of `timeout` widens that.
/// Removing the contention is the only move that helps every test in the file at once.
///
/// Unix keeps running in parallel: a no-op guard there costs nothing and serialising it would slow
/// two of the three CI platforms to fix a problem neither has.
struct SpawnGuard {
    #[cfg(windows)]
    _held: std::sync::MutexGuard<'static, ()>,
}

#[cfg(windows)]
static SPAWNING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Take the spawn lock. Call it as a test's **first** statement, before any `Instant::now()`, so
/// time spent waiting for the lock never lands inside a measured window.
fn spawn_guard() -> SpawnGuard {
    SpawnGuard {
        // Poison-tolerant on purpose. A failure in this file manifests as a panic while the lock is
        // held, and a poisoned mutex would turn one real assertion failure into seven others that
        // report the poisoning instead of what they were testing.
        #[cfg(windows)]
        _held: SPAWNING.lock().unwrap_or_else(|e| e.into_inner()),
    }
}

#[cfg(not(windows))]
mod unix {
    use super::*;

    #[test]
    fn exported_variable_is_captured() {
        let _guard = spawn_guard();
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "script.sh", "export QUICKSTART_MARKER=hello\n");

        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_secs(5));

        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(vars.contains(&("QUICKSTART_MARKER".to_string(), "hello".to_string())));
    }

    #[test]
    fn nonexistent_path_is_missing_script_with_no_vars() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("does-not-exist.sh");

        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_secs(5));

        assert_eq!(outcome, EnvIncludeOutcome::MissingScript);
        assert!(vars.is_empty());
    }

    #[test]
    fn non_zero_exit_captures_diagnostic() {
        let _guard = spawn_guard();
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "script.sh", "echo 'something went wrong'\nexit 1\n");

        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_secs(5));

        assert!(vars.is_empty());
        match outcome {
            EnvIncludeOutcome::NonZeroExit { code, diagnostic } => {
                assert_eq!(code, 1);
                assert!(diagnostic.contains("something went wrong"));
            }
            other => panic!("expected NonZeroExit, got {other:?}"),
        }
    }

    #[test]
    fn hanging_script_times_out_promptly() {
        let _guard = spawn_guard();
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "script.sh", "sleep 999\n");

        let start = std::time::Instant::now();
        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_millis(300));
        let elapsed = start.elapsed();

        assert!(vars.is_empty());
        assert!(matches!(outcome, EnvIncludeOutcome::TimedOut { .. }));
        assert!(
            elapsed < Duration::from_secs(5),
            "resolve() took {elapsed:?}, expected to return promptly after the ~300ms timeout"
        );
    }

    #[test]
    fn empty_script_succeeds_with_no_vars() {
        let _guard = spawn_guard();
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "script.sh", "");

        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_secs(5));

        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(vars.is_empty());
    }

    /// BUG-001 (specs/011-env-include-script/bugs/BUG-001.md): reproduces the report that
    /// exported variables from `.bashrc` never reach the AI CLI/regular-terminal processes.
    ///
    /// `attempt_env` sources the script via `bash --noprofile --norc -c '...'` — a
    /// non-interactive shell (no `-i` flag). Debian/Ubuntu's stock `~/.bashrc` (the feature's own
    /// FR-004 default path) opens with the standard non-interactive guard:
    /// ```sh
    /// case $- in
    ///     *i*) ;;
    ///       *) return;;
    /// esac
    /// ```
    /// Because the sourcing shell is never interactive, `$-` never contains `i`, so this guard
    /// `return`s before any of the exports that follow it ever run. On a fresh install pointed at
    /// the default `~/.bashrc` (User Story 1 / SC-001's "no setup required" promise), this means
    /// *nothing* below the guard is ever captured — the feature silently no-ops for the exact
    /// out-of-the-box case it exists to solve.
    ///
    /// This test currently FAILS: `vars` comes back empty instead of containing
    /// `QUICKSTART_MARKER=hello`.
    #[test]
    fn debian_default_bashrc_guard_blocks_export_from_reaching_session() {
        let _guard = spawn_guard();
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(
            &dir,
            ".bashrc",
            "case $- in\n    *i*) ;;\n      *) return;;\nesac\nexport QUICKSTART_MARKER=hello\n",
        );

        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_secs(5));

        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(
            vars.contains(&("QUICKSTART_MARKER".to_string(), "hello".to_string())),
            "expected QUICKSTART_MARKER to be captured from the default-shaped ~/.bashrc, but \
             the non-interactive guard blocked it; captured vars: {vars:?}"
        );
    }

    /// BUG-002 (specs/011-env-include-script/bugs/BUG-002.md): reproduces the report that
    /// directory-dependent `PATH`/env contributions (the shape a version-manager hook like
    /// `mise activate bash` produces — it inspects the sourcing shell's *own* cwd for a
    /// `mise.toml`/`.tool-versions`) are only captured when the sourcing subprocess's working
    /// directory is the session's project directory, not wherever the script file itself happens
    /// to live.
    ///
    /// The script here lives in one tempdir (`script_dir`) but checks for a sentinel file in its
    /// *own current working directory* — a stand-in for a project's `mise.toml` — which is a
    /// second, independent tempdir supplied as `resolve()`'s `cwd` argument.
    #[test]
    fn directory_dependent_export_is_captured_only_when_cwd_contains_marker() {
        let _guard = spawn_guard();
        let script_dir = tempfile::tempdir().unwrap();
        let script = write_script(
            &script_dir,
            "script.sh",
            "if [ -f ./.bug002-marker ]; then export BUG002_VAR=present; fi\n",
        );

        let project_with_marker = tempfile::tempdir().unwrap();
        std::fs::File::create(project_with_marker.path().join(".bug002-marker")).unwrap();
        let (vars, outcome) = resolve(&script, project_with_marker.path(), Duration::from_secs(5));
        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(
            vars.contains(&("BUG002_VAR".to_string(), "present".to_string())),
            "expected BUG002_VAR to be captured when the sourcing cwd contains the marker file \
             (mirrors a version-manager hook keyed off cwd, e.g. mise.toml); captured vars: \
             {vars:?}"
        );

        let project_without_marker = tempfile::tempdir().unwrap();
        let (vars, outcome) = resolve(
            &script,
            project_without_marker.path(),
            Duration::from_secs(5),
        );
        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(
            !vars.contains(&("BUG002_VAR".to_string(), "present".to_string())),
            "expected BUG002_VAR to be absent when the sourcing cwd does not contain the marker; \
             captured vars: {vars:?}"
        );
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn exported_variable_is_captured() {
        let _guard = spawn_guard();
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "profile.ps1", "$env:QUICKSTART_MARKER = 'hello'\n");

        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_secs(10));

        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(vars.contains(&("QUICKSTART_MARKER".to_string(), "hello".to_string())));
    }

    #[test]
    fn nonexistent_path_is_missing_script_with_no_vars() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("does-not-exist.ps1");

        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_secs(10));

        assert_eq!(outcome, EnvIncludeOutcome::MissingScript);
        assert!(vars.is_empty());
    }

    #[test]
    fn non_zero_exit_captures_diagnostic() {
        let _guard = spawn_guard();
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(
            &dir,
            "profile.ps1",
            "Write-Output 'something went wrong'\nexit 1\n",
        );

        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_secs(10));

        assert!(vars.is_empty());
        match outcome {
            EnvIncludeOutcome::NonZeroExit { code, diagnostic } => {
                assert_eq!(code, 1);
                assert!(diagnostic.contains("something went wrong"));
            }
            other => panic!("expected NonZeroExit, got {other:?}"),
        }
    }

    #[test]
    fn hanging_script_times_out_promptly() {
        let _guard = spawn_guard();
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "profile.ps1", "Start-Sleep -Seconds 999\n");

        let start = std::time::Instant::now();
        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_millis(500));
        let elapsed = start.elapsed();

        assert!(vars.is_empty());
        assert!(matches!(outcome, EnvIncludeOutcome::TimedOut { .. }));
        assert!(
            elapsed < Duration::from_secs(10),
            "resolve() took {elapsed:?}, expected to return promptly after the timeout"
        );
    }

    #[test]
    fn empty_script_succeeds_with_no_vars() {
        let _guard = spawn_guard();
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "profile.ps1", "");

        let (vars, outcome) = resolve(&script, dir.path(), Duration::from_secs(10));

        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(vars.is_empty());
    }

    /// BUG-002 (specs/011-env-include-script/bugs/BUG-002.md): Windows equivalent of the Unix
    /// directory-dependent-PATH regression test — see that module's doc comment.
    #[test]
    fn directory_dependent_export_is_captured_only_when_cwd_contains_marker() {
        let _guard = spawn_guard();
        let script_dir = tempfile::tempdir().unwrap();
        let script = write_script(
            &script_dir,
            "profile.ps1",
            "if (Test-Path .\\.bug002-marker) { $env:BUG002_VAR = 'present' }\n",
        );

        let project_with_marker = tempfile::tempdir().unwrap();
        std::fs::File::create(project_with_marker.path().join(".bug002-marker")).unwrap();
        let (vars, outcome) = resolve(&script, project_with_marker.path(), Duration::from_secs(10));
        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(
            vars.contains(&("BUG002_VAR".to_string(), "present".to_string())),
            "expected BUG002_VAR to be captured when the sourcing cwd contains the marker file; \
             captured vars: {vars:?}"
        );

        let project_without_marker = tempfile::tempdir().unwrap();
        let (vars, outcome) = resolve(
            &script,
            project_without_marker.path(),
            Duration::from_secs(10),
        );
        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(
            !vars.contains(&("BUG002_VAR".to_string(), "present".to_string())),
            "expected BUG002_VAR to be absent when the sourcing cwd does not contain the marker; \
             captured vars: {vars:?}"
        );
    }
}

/// BUG-003 (specs/011-env-include-script/bugs/BUG-003.md): a sourced script that **backgrounds a
/// process holding the pipe** must not hold `resolve()` open past its own timeout.
///
/// `run_bounded`'s doc comment has promised this since feature 011: it kills the whole process
/// group before reading stdout, "so no orphaned grandchild can keep the pipes open", because
/// `read_to_end` waits for the last *writer* to close rather than for the child to die. The
/// promise was kept on Unix by `process_group(0)` plus `kill(-pid)`, and on Windows by nothing at
/// all — `child.kill()` reaches one process and none of its descendants.
///
/// Deliberately in the shared (non-platform) part of this file, because it is one guarantee rather
/// than two. The script differs per platform only in how a shell backgrounds something.
///
/// The bound is generous — 30× the timeout — on purpose. The failure this catches is unbounded
/// (the recorded run measured 10.8s against a 500ms budget, and would have waited for a 999-second
/// sleep had the runner allowed it), so a loose bound still separates "returned" from "blocked"
/// while leaving room for a slow cold runner. Tightening it would trade the thing being measured
/// for flakiness.
#[test]
fn a_script_that_backgrounds_a_pipe_holder_still_returns_promptly() {
    let _guard = spawn_guard();
    let dir = tempfile::tempdir().unwrap();

    #[cfg(not(windows))]
    let script = write_script(
        &dir,
        "script.sh",
        // Inherits this process's stdout, then outlives the script that started it.
        "sleep 120 &\nexport BUG003_VAR=done\n",
    );
    #[cfg(windows)]
    let script = write_script(
        &dir,
        "profile.ps1",
        // `UseShellExecute = false` with no redirection makes the child inherit our handles, which
        // is the case that matters — a job runner or version-manager helper left holding stdout.
        "$si = New-Object System.Diagnostics.ProcessStartInfo\n\
         $si.FileName = 'powershell'\n\
         $si.Arguments = '-NoProfile -Command Start-Sleep -Seconds 120'\n\
         $si.UseShellExecute = $false\n\
         [void][System.Diagnostics.Process]::Start($si)\n\
         $env:BUG003_VAR = 'done'\n",
    );

    let timeout = Duration::from_secs(2);
    let start = std::time::Instant::now();
    let (_vars, _outcome) = resolve(&script, dir.path(), timeout);
    let elapsed = start.elapsed();

    assert!(
        elapsed < timeout * 30,
        "resolve() took {elapsed:?} against a {timeout:?} timeout. A backgrounded process is \
         holding the pipe and nothing killed it, so reading to EOF is waiting for a process this \
         function never intended to wait for — the exact deadlock `run_bounded`'s doc comment says \
         it prevents (BUG-003)."
    );
}

/// BUG-003, corrected: `resolve()`'s wall clock must be bounded by the timeout it was *given*.
///
/// It was not. `resolve` establishes a baseline environment before sourcing anything, and that
/// probe carried its own hardcoded 10-second budget on both platforms — so a caller asking for
/// 500 ms could wait 10.5 s, which is exactly what CI measured (10.8 s against a 500 ms budget) and
/// what the first version of BUG-003 misread as a pipe-holding deadlock.
///
/// The two things this pins:
///
/// 1. **The budget covers the whole call**, baseline included. It is the user's
///    `env_include_timeout_secs`, and its promise is about how long opening a project can stall.
/// 2. **A baseline that did not complete is reported, not treated as an empty environment.** That
///    was the more dangerous half: diffing a real environment against nothing reports every
///    variable in it as newly set by the user's script.
#[test]
fn a_budget_too_small_to_establish_a_baseline_says_so() {
    let _guard = spawn_guard();
    let dir = tempfile::tempdir().unwrap();
    // A script that would succeed instantly given any real budget at all.
    #[cfg(not(windows))]
    let script = write_script(&dir, "script.sh", "export BUG003_TINY=yes\n");
    #[cfg(windows)]
    let script = write_script(&dir, "profile.ps1", "$env:BUG003_TINY = 'yes'\n");

    let (vars, outcome) = resolve(&script, dir.path(), Duration::from_millis(1));

    assert!(
        vars.is_empty(),
        "with no baseline to compare against, every variable in the environment looks newly set. \
         Reporting them would hand the caller the entire environment as though the script had \
         exported it: {vars:?}"
    );
    match outcome {
        EnvIncludeOutcome::TimedOut { diagnostic } => assert!(
            diagnostic.contains("baseline"),
            "the timeout is real, but the caller cannot act on it without knowing the budget went \
             before the script ever ran. Diagnostic was: {diagnostic:?}"
        ),
        other => {
            panic!("expected TimedOut once the budget cannot cover the baseline, got {other:?}")
        }
    }
}

/// The same guarantee from the outside: a hanging script does not cost the baseline's budget *plus*
/// its own.
///
/// Loose on purpose. On a fast machine the baseline costs tens of milliseconds and this passes
/// whether or not the budget is shared, so its value is on a slow runner — which is where the
/// defect was visible in the first place, and where a regression would reappear.
#[test]
fn a_hanging_script_costs_its_own_budget_and_not_the_baselines_as_well() {
    let _guard = spawn_guard();
    let dir = tempfile::tempdir().unwrap();
    #[cfg(not(windows))]
    let script = write_script(&dir, "script.sh", "sleep 120\n");
    #[cfg(windows)]
    let script = write_script(&dir, "profile.ps1", "Start-Sleep -Seconds 120\n");

    let timeout = Duration::from_secs(2);
    let start = std::time::Instant::now();
    let (_vars, outcome) = resolve(&script, dir.path(), timeout);
    let elapsed = start.elapsed();

    assert!(
        matches!(outcome, EnvIncludeOutcome::TimedOut { .. }),
        "a 120-second sleep against a 2-second budget times out; got {outcome:?}"
    );
    assert!(
        elapsed < timeout * 4,
        "resolve() took {elapsed:?} for a {timeout:?} budget. The budget is what bounds how long \
         opening a project can stall, so anything the call does beyond sourcing the script — the \
         baseline probe above all — has to come out of it rather than be added to it (BUG-003)."
    );
}

/// Every test that sources a real script takes the spawn lock (BUG-004).
///
/// Structural, because forgetting the guard is **silent**: the test passes locally, passes on unix
/// where the guard is a no-op anyway, and passes on Windows most of the time. It shows up as one
/// more process in a race that four other tests then lose — which is the failure BUG-004 is about,
/// and it took five CI re-runs and two bug reports to attribute. A ninth spawning test added
/// without the guard would put it straight back.
///
/// A test **spawns** if it writes a script and resolves it. The two `nonexistent_path_…` tests
/// resolve a path that does not exist, so `resolve()` returns `MissingScript` before touching a
/// shell; they contend for nothing and are correctly unguarded.
///
/// The needles are assembled at runtime rather than written as literals. This test reads the file
/// it is written in, so a literal `"write_script("` here would be found in its own body — the same
/// self-matching trap `service_capability_fakes.rs` records hitting, where a fixture naming a real
/// port reported itself as a live violation.
#[test]
fn every_test_that_sources_a_script_takes_the_spawn_lock() {
    let src = include_str!("env_include_resolve.rs");
    let writes = format!("{}_{}(", "write", "script");
    let guards = format!("{}_{}()", "spawn", "guard");

    let mut spawning = Vec::new();
    let mut unguarded = Vec::new();
    let mut non_spawning = Vec::new();

    for (name, body) in test_bodies(src) {
        if name == "every_test_that_sources_a_script_takes_the_spawn_lock" {
            continue;
        }
        if !body.contains(&writes) {
            non_spawning.push(name);
            continue;
        }
        spawning.push(name.clone());
        if !body.contains(&guards) {
            unguarded.push(name);
        }
    }

    assert!(
        unguarded.is_empty(),
        "these tests source a real script but do not take the spawn lock, so they race every \
         other spawning test in this binary (BUG-004): {unguarded:?}\n\nAdd `let _guard = \
         spawn_guard();` as the first statement, before any `Instant::now()`."
    );

    // Vacuity. A scan that matched nothing would pass the assertion above without looking at
    // anything, and this file's whole failure mode is a check that quietly stops checking.
    assert!(
        spawning.len() >= 14,
        "expected at least the fourteen spawning tests both platform modules declare, found {}: \
         {spawning:?}. The body scan has drifted from the source it reads.",
        spawning.len()
    );
    assert_eq!(
        non_spawning
            .iter()
            .filter(|n| n.as_str() == "nonexistent_path_is_missing_script_with_no_vars")
            .count(),
        2,
        "expected both platforms' missing-script tests to be classified as non-spawning; found \
         {non_spawning:?}. If they now write a script they need the guard like everything else."
    );
}

/// `(name, body)` for every `fn name() {` in `src`, the body matched by brace depth.
///
/// Depth-matched rather than "up to the next `}`", because a test body contains braces — a `match`
/// arm, a format string's `{elapsed:?}` — and stopping at the first one would read a fraction of
/// the body and call it whole.
fn test_bodies(src: &str) -> Vec<(String, String)> {
    let code: String = src
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut out = Vec::new();
    let mut rest = code.as_str();
    while let Some(at) = rest.find("fn ") {
        let after = &rest[at + 3..];
        let Some(paren) = after.find('(') else { break };
        let name = after[..paren].trim().to_string();
        let Some(open) = after.find('{') else { break };

        let mut depth = 0usize;
        let mut end = after.len();
        for (i, c) in after.char_indices().skip(open) {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            out.push((name, after[open..end].to_string()));
        }
        rest = &after[end.min(after.len())..];
    }
    out
}
