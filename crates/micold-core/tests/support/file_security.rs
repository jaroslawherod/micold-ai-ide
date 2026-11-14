//! Read back what protects a file: the mode bits on Unix, the DACL on Windows. Included with
//! `#[path]` by `owner_only.rs` (core), `mcp_binding_file_mode.rs` and `windows_pipe_acl.rs`
//! (daemon); each uses a different part, so unused items are expected.
#![allow(dead_code)]

#[cfg(unix)]
pub use unix::{mode, set_mode};

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    pub fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    pub fn set_mode(path: &Path, mode: u32) {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
}
