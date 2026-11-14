//! Files and directories that only the user running the process can read (feature 041, R8,
//! FR-020; first written for the tool-server binding files of feature 034, FR-007).
//!
//! `0700` / `0600` on Unix, a protected DACL whose only ACE grants the current user full access on
//! Windows (`D:P(A;;GA;;;<sid>)`, as the daemon's pipe has). A file is written to a temporary file
//! in the same directory, owner-only before any content reaches it, and renamed over the old one:
//! the content is never readable under a wider protection, and the name holds the previous whole
//! file or the new whole one. The temporary file is synced to disk before the rename, and on Unix
//! the directory after it (contracts/saved-history-file.md §3).

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

/// Make `dir` exist and be owner-only, creating its missing parents the same way.
///
/// An existing directory loses what it grants to anyone but its owner. The owner's own bits are
/// left as they are on Unix, so a directory the user made read-only stays read-only.
pub fn ensure_dir(dir: &Path) -> io::Result<()> {
    imp::ensure_dir(dir)
}

/// Write `bytes` to `dir/file` so that only the user running the process can read it, making `dir`
/// owner-only as [`ensure_dir`] does, and return the file's path.
pub fn write(dir: &Path, file: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    use std::io::Write;
    write_with(dir, file, |out| out.write_all(bytes))
}

/// [`write`], with the content written by `fill` into the temporary file, which is empty and
/// already owner-only when `fill` is called.
///
/// When `fill` or a later step fails, the temporary file is removed, the error is returned and
/// `dir/file` is what it was.
pub fn write_with(
    dir: &Path,
    file: &str,
    fill: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<PathBuf> {
    ensure_dir(dir)?;
    let path = dir.join(file);
    let tmp = dir.join(format!(".{file}.tmp"));
    // Left by a run that was killed between the creation and the rename.
    let _ = std::fs::remove_file(&tmp);
    let written = (|| {
        let mut out = imp::create_owner_only(&tmp)?;
        fill(&mut out)?;
        // On disk before the name points at it, so a power loss leaves the old whole file or the
        // new whole one under the name (FR-006).
        out.sync_all()?;
        drop(out);
        std::fs::rename(&tmp, &path)
    })();
    if let Err(error) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(error);
    }
    imp::sync_dir(dir);
    Ok(path)
}

#[cfg(unix)]
mod imp {
    use std::fs::{DirBuilder, File, OpenOptions, Permissions};
    use std::io;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
    use std::path::Path;

    pub(super) fn ensure_dir(dir: &Path) -> io::Result<()> {
        DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
        // An existing directory keeps its mode through `create`; take away what group and others
        // have.
        let mode = std::fs::metadata(dir)?.permissions().mode();
        if mode & 0o077 != 0 {
            std::fs::set_permissions(dir, Permissions::from_mode(mode & 0o700))?;
        }
        Ok(())
    }

    /// A new, empty file at `path` with mode `0600`.
    pub(super) fn create_owner_only(path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
    }

    /// Put the rename itself on disk. The file is whole under its name by now, so a filesystem
    /// that cannot sync a directory does not fail the write.
    pub(super) fn sync_dir(dir: &Path) {
        let _ = File::open(dir).and_then(|dir| dir.sync_all());
    }
}
