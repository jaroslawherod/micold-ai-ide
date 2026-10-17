//! Feature 041 (R8, FR-020; U55–U59): saved terminal history is readable by the user who runs the
//! service and by nobody else, from the moment it is created. `owner_only::write` and
//! `owner_only::ensure_dir` make the directory and the file owner-only: modes `0700` / `0600` on
//! Unix, a protected DACL whose one ACE is the current user's on Windows. A write goes through a
//! temporary file in the same directory that is renamed over the old file.

use std::io::Write;
use std::path::{Path, PathBuf};

use micold_core::owner_only;

#[path = "support/file_security.rs"]
mod file_security;

const CONTENT: &[u8] = b"the second, shorter version";
const FILE: &str = "a.history";

/// Every entry of `dir`, sorted.
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    found.sort();
    found
}

#[test]
fn write_with_stores_what_the_fill_wrote_and_returns_the_path() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("terminal-history");
    let path = owner_only::write_with(&dir, FILE, |out| {
        out.write_all(b"one ")?;
        out.write_all(b"two")
    })
    .unwrap();
    assert_eq!(path, dir.join(FILE));
    assert_eq!(std::fs::read(&path).unwrap(), b"one two");
    assert_eq!(entries(&dir), [path], "only the file is left");
}

/// U59: a write that fails after the temporary file exists removes it.
#[test]
fn a_fill_that_fails_returns_its_error_and_leaves_no_temporary_file() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("terminal-history");
    owner_only::write(&dir, FILE, b"the earlier file").unwrap();
    let error = owner_only::write_with(&dir, FILE, |out| {
        out.write_all(b"half of a fi")?;
        Err(std::io::Error::other("the disk is full"))
    })
    .unwrap_err();
    assert_eq!(error.to_string(), "the disk is full");
    assert_eq!(entries(&dir), [dir.join(FILE)], "no temporary file");
    assert_eq!(
        std::fs::read(dir.join(FILE)).unwrap(),
        b"the earlier file",
        "the earlier file is whole"
    );
}

/// U59: the rename cannot replace a directory that is not empty.
#[test]
fn a_rename_that_fails_returns_the_error_and_leaves_no_temporary_file() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("terminal-history");
    let in_the_way = dir.join(FILE);
    std::fs::create_dir_all(&in_the_way).unwrap();
    std::fs::write(in_the_way.join("inner"), b"x").unwrap();
    owner_only::write(&dir, FILE, CONTENT).unwrap_err();
    assert_eq!(entries(&dir), [in_the_way], "no temporary file");
}

#[cfg(unix)]
mod unix {
    use super::file_security::{mode, set_mode};
    use super::*;
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    /// U55.
    #[test]
    fn write_creates_the_directory_0700_and_the_file_0600() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("terminal-history");
        let path = owner_only::write(&dir, FILE, CONTENT).unwrap();
        assert_eq!(path, dir.join(FILE));
        assert_eq!(mode(&dir), 0o700, "the directory must be 0700");
        assert_eq!(mode(&path), 0o600, "the file must be 0600");
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
        assert_eq!(entries(&dir), [path], "only the file is left");
    }

    /// U55: the old file is replaced, not written into, so a reader that has it open keeps the
    /// whole old content.
    #[test]
    fn write_replaces_an_existing_file_through_a_rename() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("terminal-history");
        let first = b"a much longer first version of the file";
        let path = owner_only::write(&dir, FILE, first).unwrap();
        let old_inode = std::fs::metadata(&path).unwrap().ino();
        let mut old = std::fs::File::open(&path).unwrap();

        owner_only::write(&dir, FILE, CONTENT).unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
        assert_eq!(mode(&path), 0o600);
        assert_ne!(
            std::fs::metadata(&path).unwrap().ino(),
            old_inode,
            "the name holds a new file"
        );
        let mut still = Vec::new();
        old.read_to_end(&mut still).unwrap();
        assert_eq!(still, first, "the old file was not written into");
        assert_eq!(entries(&dir), [path], "only the file is left");
    }

    /// U55.
    #[test]
    fn write_narrows_an_existing_wider_file_and_directory() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("terminal-history");
        std::fs::create_dir_all(&dir).unwrap();
        set_mode(&dir, 0o755);
        let path = dir.join(FILE);
        std::fs::write(&path, b"old").unwrap();
        set_mode(&path, 0o644);
        owner_only::write(&dir, FILE, CONTENT).unwrap();
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&path), 0o600);
    }

    /// U56.
    #[test]
    fn ensure_dir_creates_a_missing_directory_0700() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("data").join("terminal-history");
        owner_only::ensure_dir(&dir).unwrap();
        assert!(dir.is_dir());
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(entries(&dir), Vec::<PathBuf>::new());
    }

    /// U56.
    #[test]
    fn ensure_dir_tightens_an_existing_looser_directory() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("terminal-history");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("kept"), b"x").unwrap();
        for looser in [0o755, 0o777, 0o750, 0o701] {
            set_mode(&dir, looser);
            owner_only::ensure_dir(&dir).unwrap();
            assert_eq!(mode(&dir), 0o700, "from {looser:o}");
        }
        assert_eq!(std::fs::read(dir.join("kept")).unwrap(), b"x");
    }

    /// U58 (FR-020).
    #[test]
    fn the_temporary_file_is_0600_before_any_content_is_written() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("terminal-history");
        let mut seen = None;
        owner_only::write_with(&dir, FILE, |out| {
            let found = entries(&dir);
            assert_eq!(found.len(), 1, "only the temporary file: {found:?}");
            assert_ne!(
                found[0],
                dir.join(FILE),
                "the content is not under the name yet"
            );
            let metadata = std::fs::metadata(&found[0]).unwrap();
            seen = Some((metadata.permissions().mode() & 0o777, metadata.len()));
            out.write_all(CONTENT)
        })
        .unwrap();
        assert_eq!(
            seen,
            Some((0o600, 0)),
            "mode and length when the fill starts"
        );
    }

    /// U59. The directory is the user's own and not looser than `0700`, so it is left as it is and
    /// the temporary file cannot be created in it.
    #[test]
    fn write_into_a_read_only_directory_returns_the_error_and_leaves_no_temporary_file() {
        // SAFETY: `geteuid` reads the process's credentials and cannot fail.
        if unsafe { libc::geteuid() } == 0 {
            return; // root writes through a read-only directory
        }
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("terminal-history");
        std::fs::create_dir_all(&dir).unwrap();
        set_mode(&dir, 0o500);
        let result = owner_only::write(&dir, FILE, CONTENT);
        let left = entries(&dir);
        let mode_after = mode(&dir);
        set_mode(&dir, 0o700);
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert_eq!(left, Vec::<PathBuf>::new(), "no temporary file");
        assert_eq!(mode_after, 0o500, "the directory was not made writable");
    }
}

#[cfg(windows)]
mod windows {
    use super::file_security::{read_dacl, Dacl};
    use super::*;

    /// The DACL every directory and file of this module must have.
    const OWNER_ONLY: Dacl = Dacl {
        protected: true,
        ace_count: 1,
        first_ace_allows_current_user: true,
    };

    /// U57.
    #[test]
    fn the_written_file_and_its_directory_have_a_protected_dacl_for_the_current_user_only() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("terminal-history");
        let path = owner_only::write(&dir, FILE, CONTENT).unwrap();
        assert_eq!(path, dir.join(FILE));
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
        assert_eq!(read_dacl(&path), OWNER_ONLY, "the file");
        assert_eq!(read_dacl(&dir), OWNER_ONLY, "the directory");
    }

    /// U57.
    #[test]
    fn rewriting_replaces_the_bytes_and_keeps_the_owner_only_dacl() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("terminal-history");
        owner_only::write(&dir, FILE, b"a much longer first version of the file").unwrap();
        let path = owner_only::write(&dir, FILE, CONTENT).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
        assert_eq!(read_dacl(&path), OWNER_ONLY);
        assert_eq!(entries(&dir), [path], "only the file is left");
    }

    /// U57.
    #[test]
    fn ensure_dir_gives_a_missing_and_an_existing_directory_the_owner_only_dacl() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("data").join("terminal-history");
        owner_only::ensure_dir(&missing).unwrap();
        assert!(missing.is_dir());
        assert_eq!(read_dacl(&missing), OWNER_ONLY, "a created directory");

        // Made by someone else: it inherits the temporary directory's entries.
        let existing = root.path().join("existing");
        std::fs::create_dir_all(&existing).unwrap();
        assert_ne!(read_dacl(&existing), OWNER_ONLY, "an inherited DACL");
        owner_only::ensure_dir(&existing).unwrap();
        assert_eq!(read_dacl(&existing), OWNER_ONLY, "an existing directory");
    }

    /// U58 (FR-020).
    #[test]
    fn the_temporary_file_has_the_owner_only_dacl_before_any_content_is_written() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("terminal-history");
        let mut seen = None;
        owner_only::write_with(&dir, FILE, |out| {
            let found = entries(&dir);
            assert_eq!(found.len(), 1, "only the temporary file: {found:?}");
            assert_ne!(
                found[0],
                dir.join(FILE),
                "the content is not under the name yet"
            );
            seen = Some((read_dacl(&found[0]), std::fs::metadata(&found[0])?.len()));
            out.write_all(CONTENT)
        })
        .unwrap();
        assert_eq!(
            seen,
            Some((OWNER_ONLY, 0)),
            "DACL and length when the fill starts"
        );
    }
}
