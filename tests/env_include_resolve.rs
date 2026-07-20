//! Integration tests for the environment-include sourcing/diffing engine (feature 011,
//! contracts/env-include-resolution.md). Spawns REAL disposable subprocesses via `tempfile`-
//! written scripts — no mocking, per FR-005's "actually sourcing it, not parsing text" mandate.
//! Each platform's module only compiles (and so only runs) on its matching CI runner, mirroring
//! `tests/shell_command.rs`.

use micold_ai_ide::env_include::{resolve, EnvIncludeOutcome};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

fn write_script(dir: &tempfile::TempDir, name: &str, contents: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    path
}

#[cfg(not(windows))]
mod unix {
    use super::*;

    #[test]
    fn exported_variable_is_captured() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "script.sh", "export QUICKSTART_MARKER=hello\n");

        let (vars, outcome) = resolve(&script, Duration::from_secs(5));

        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(vars.contains(&("QUICKSTART_MARKER".to_string(), "hello".to_string())));
    }

    #[test]
    fn nonexistent_path_is_missing_script_with_no_vars() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("does-not-exist.sh");

        let (vars, outcome) = resolve(&script, Duration::from_secs(5));

        assert_eq!(outcome, EnvIncludeOutcome::MissingScript);
        assert!(vars.is_empty());
    }

    #[test]
    fn non_zero_exit_captures_diagnostic() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "script.sh", "echo 'something went wrong'\nexit 1\n");

        let (vars, outcome) = resolve(&script, Duration::from_secs(5));

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
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "script.sh", "sleep 999\n");

        let start = std::time::Instant::now();
        let (vars, outcome) = resolve(&script, Duration::from_millis(300));
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
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "script.sh", "");

        let (vars, outcome) = resolve(&script, Duration::from_secs(5));

        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(vars.is_empty());
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn exported_variable_is_captured() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "profile.ps1", "$env:QUICKSTART_MARKER = 'hello'\n");

        let (vars, outcome) = resolve(&script, Duration::from_secs(10));

        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(vars.contains(&("QUICKSTART_MARKER".to_string(), "hello".to_string())));
    }

    #[test]
    fn nonexistent_path_is_missing_script_with_no_vars() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("does-not-exist.ps1");

        let (vars, outcome) = resolve(&script, Duration::from_secs(10));

        assert_eq!(outcome, EnvIncludeOutcome::MissingScript);
        assert!(vars.is_empty());
    }

    #[test]
    fn non_zero_exit_captures_diagnostic() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(
            &dir,
            "profile.ps1",
            "Write-Output 'something went wrong'\nexit 1\n",
        );

        let (vars, outcome) = resolve(&script, Duration::from_secs(10));

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
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "profile.ps1", "Start-Sleep -Seconds 999\n");

        let start = std::time::Instant::now();
        let (vars, outcome) = resolve(&script, Duration::from_millis(500));
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
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "profile.ps1", "");

        let (vars, outcome) = resolve(&script, Duration::from_secs(10));

        assert_eq!(outcome, EnvIncludeOutcome::Success);
        assert!(vars.is_empty());
    }
}
