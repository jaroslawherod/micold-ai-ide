//! The application's per-user directories, resolved in one place.

use std::path::PathBuf;

use directories::ProjectDirs;

/// The application tuple `directories` derives every per-user location from. It MUST stay stable
/// across releases (storage-schema contract).
pub const APP_ID: &str = "micold-ai-ide";

/// The application's directories, or `None` when no home directory can be determined.
pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", APP_ID)
}

/// The per-user data directory (XDG / Application Support / roaming AppData).
pub fn data_dir() -> Option<PathBuf> {
    project_dirs().map(|dirs| dirs.data_dir().to_path_buf())
}

/// `name` under [`data_dir`].
pub fn data_subdir(name: &str) -> Option<PathBuf> {
    data_dir().map(|dir| dir.join(name))
}
