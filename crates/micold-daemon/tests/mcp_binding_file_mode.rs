//! Feature 034 (FR-007, SC-009; U30–U33): a session's binding file carries its bearer credential, so
//! only the user who runs the service may read it. `owner_only::write` creates the directory
//! and the file owner-only: modes `0700` / `0600` on Unix, a protected DACL whose one ACE is the
//! current user's on Windows.

use micold_core::owner_only::write as write_owner_only;

#[path = "../../micold-core/tests/support/file_security.rs"]
mod file_security;

const CONTENT: &[u8] = br#"{"mcpServers":{}}"#;

#[cfg(unix)]
mod unix {
    use super::file_security::{mode, set_mode};
    use super::*;

    #[test]
    fn the_created_directory_is_owner_only() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(mode(&dir), 0o700, "the binding directory must be 0700");
    }

    #[test]
    fn the_written_file_is_owner_only() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        let path = write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(path, dir.join("a.json"));
        assert_eq!(mode(&path), 0o600, "the binding file must be 0600");
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
    }

    #[test]
    fn rewriting_replaces_the_bytes_and_stays_owner_only() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        write_owner_only(&dir, "a.json", b"a much longer first version of the file").unwrap();
        let path = write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
        assert_eq!(mode(&path), 0o600);
    }

    #[test]
    fn an_existing_wider_file_is_narrowed_on_rewrite() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.json");
        std::fs::write(&path, b"old").unwrap();
        set_mode(&path, 0o644);
        write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(mode(&path), 0o600);
    }
}

#[cfg(windows)]
mod windows {
    use super::file_security::read_dacl;
    use super::*;

    #[test]
    fn the_written_file_has_a_protected_dacl_for_the_current_user_only() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        let path = write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
        let dacl = read_dacl(&path);
        assert!(dacl.protected, "the binding file's DACL must be protected");
        assert_eq!(dacl.ace_count, 1, "exactly one ACE");
        assert!(
            dacl.first_ace_allows_current_user,
            "the one ACE must allow the current user"
        );
    }

    #[test]
    fn rewriting_keeps_the_owner_only_dacl() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        write_owner_only(&dir, "a.json", b"first").unwrap();
        let path = write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
        let dacl = read_dacl(&path);
        assert!(dacl.protected);
        assert_eq!(dacl.ace_count, 1);
        assert!(dacl.first_ace_allows_current_user);
    }
}
