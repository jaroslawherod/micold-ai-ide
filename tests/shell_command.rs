//! T004 — pure shell-command resolution (feature 010, research R3,
//! contracts/shell-process.md). `default_shell_command` branches on `cfg!(windows)`
//! internally, so the Unix-branch tests run for real on Linux/macOS CI and the Windows-branch
//! tests run for real on Windows CI (Principle VI) — each `#[cfg]`-gated group only compiles
//! (and so only runs) on its matching platform, exercising the actually-compiled branch rather
//! than a simulated one.

use micold_ai_ide::terminal::default_shell_command;

#[cfg(not(windows))]
mod unix {
    use super::default_shell_command;
    use micold_ai_ide::settings::default_env_include_path;
    use std::path::{Path, PathBuf};

    #[test]
    fn prefers_non_empty_shell_env() {
        assert_eq!(
            default_shell_command(Some("/usr/bin/zsh"), None),
            "/usr/bin/zsh"
        );
    }

    #[test]
    fn falls_back_when_shell_env_absent() {
        assert_eq!(default_shell_command(None, None), "/bin/sh");
    }

    #[test]
    fn falls_back_when_shell_env_empty() {
        assert_eq!(default_shell_command(Some(""), None), "/bin/sh");
    }

    #[test]
    fn ignores_comspec() {
        assert_eq!(
            default_shell_command(None, Some("C:\\Windows\\System32\\cmd.exe")),
            "/bin/sh"
        );
    }

    // Feature 011 (environment-include), research R7.
    #[test]
    fn default_env_include_path_is_bashrc_under_home() {
        assert_eq!(
            default_env_include_path(Some(Path::new("/home/alice"))),
            PathBuf::from("/home/alice/.bashrc")
        );
    }

    #[test]
    fn default_env_include_path_falls_back_when_home_absent() {
        assert_eq!(default_env_include_path(None), PathBuf::from(".bashrc"));
    }
}

#[cfg(windows)]
mod windows {
    use super::default_shell_command;
    use micold_ai_ide::settings::default_env_include_path;
    use std::path::{Path, PathBuf};

    #[test]
    fn prefers_non_empty_comspec_env() {
        assert_eq!(
            default_shell_command(None, Some("C:\\Windows\\System32\\cmd.exe")),
            "C:\\Windows\\System32\\cmd.exe"
        );
    }

    #[test]
    fn falls_back_when_comspec_absent() {
        assert_eq!(default_shell_command(None, None), "cmd.exe");
    }

    #[test]
    fn falls_back_when_comspec_empty() {
        assert_eq!(default_shell_command(None, Some("")), "cmd.exe");
    }

    #[test]
    fn ignores_shell_env() {
        assert_eq!(default_shell_command(Some("/usr/bin/zsh"), None), "cmd.exe");
    }

    // Feature 011 (environment-include), research R6/R7.
    #[test]
    fn default_env_include_path_is_powershell_profile_under_home() {
        assert_eq!(
            default_env_include_path(Some(Path::new(r"C:\Users\alice"))),
            PathBuf::from(r"C:\Users\alice\Documents\WindowsPowerShell\profile.ps1")
        );
    }

    #[test]
    fn default_env_include_path_falls_back_when_home_absent() {
        assert_eq!(
            default_env_include_path(None),
            PathBuf::from("Documents\\WindowsPowerShell\\profile.ps1")
        );
    }
}
