//! Finding this workspace's `micold-daemon` build from a test executable (bug 1863).
//!
//! The toolchain puts a test executable in `target/<profile>/deps/` or, on newer ones, in
//! `target/<profile>/build/<pkg>/<hash>/out/`, so a fixed number of `parent()` steps lands in the
//! wrong directory. The build is `target/<profile>/micold-daemon`: look in every ancestor of the
//! executable instead.

use std::path::{Path, PathBuf};

/// The `micold-daemon` file in the nearest ancestor directory of `exe` that has one.
pub fn built_daemon_above(exe: &Path) -> Option<PathBuf> {
    exe.ancestors()
        .skip(1)
        .map(|dir| dir.join("micold-daemon"))
        .find(|candidate| candidate.is_file())
}

/// [`built_daemon_above`] for the running test executable.
#[allow(dead_code)] // the lookup test includes this file but only uses the pure half
pub fn built_daemon() -> Option<PathBuf> {
    built_daemon_above(&std::env::current_exe().ok()?)
}
