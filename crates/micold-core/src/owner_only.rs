//! Files and directories that only the user running the process can read (feature 041, R8).

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

/// Make `dir` exist and be owner-only.
pub fn ensure_dir(_dir: &Path) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}

/// Write `bytes` to `dir/file`, owner-only, and return the file's path.
pub fn write(dir: &Path, file: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    use std::io::Write;
    write_with(dir, file, |out| out.write_all(bytes))
}

/// [`write`], with the content written by `fill`.
pub fn write_with(
    _dir: &Path,
    _file: &str,
    _fill: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<PathBuf> {
    Err(io::ErrorKind::Unsupported.into())
}
