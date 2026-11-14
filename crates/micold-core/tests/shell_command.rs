//! T004 — pure shell-command resolution (feature 010, research R3,
//! contracts/shell-process.md): `$SHELL`, or `/bin/sh` when it is unset or empty.

use micold_core::terminal::default_shell_command;

mod unix {
    use super::default_shell_command;
    use micold_core::settings::default_env_include_path;
    use std::path::{Path, PathBuf};

    #[test]
    fn prefers_non_empty_shell_env() {
        assert_eq!(default_shell_command(Some("/usr/bin/zsh")), "/usr/bin/zsh");
    }

    #[test]
    fn falls_back_when_shell_env_absent() {
        assert_eq!(default_shell_command(None), "/bin/sh");
    }

    #[test]
    fn falls_back_when_shell_env_empty() {
        assert_eq!(default_shell_command(Some("")), "/bin/sh");
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
