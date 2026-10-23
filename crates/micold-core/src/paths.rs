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

/// The variable that points the daemon at its state directory directly (1286). A container image
/// sets this instead of `XDG_DATA_HOME`, which every process in the container would inherit and
/// which names a directory the session user cannot write (`mise` and other XDG-aware tools then
/// fail under `/var/lib`).
pub const DATA_DIR_ENV: &str = "MICOLD_DATA_DIR";

fn data_dir_override() -> Option<PathBuf> {
    override_from(std::env::var_os(DATA_DIR_ENV))
}

fn override_from(value: Option<std::ffi::OsString>) -> Option<PathBuf> {
    value.filter(|value| !value.is_empty()).map(PathBuf::from)
}

/// The per-user data directory (XDG / Application Support / roaming AppData), or [`DATA_DIR_ENV`].
pub fn data_dir() -> Option<PathBuf> {
    data_dir_override().or_else(|| project_dirs().map(|dirs| dirs.data_dir().to_path_buf()))
}

/// The per-user local data directory (the same as [`data_dir`] off Windows), or [`DATA_DIR_ENV`].
pub fn data_local_dir() -> Option<PathBuf> {
    data_dir_override().or_else(|| project_dirs().map(|dirs| dirs.data_local_dir().to_path_buf()))
}

/// The directory for secrets the client holds on the user's device (feature 1002, research R5): a
/// **sibling** of [`data_dir`], so no mount of the state directory (the legacy default container
/// daemon mounts all of `data_dir()`) reaches it.
pub fn secrets_dir() -> Option<PathBuf> {
    data_dir().and_then(|dir| secrets_dir_beside(&dir))
}

/// The name of [`secrets_dir`].
pub const SECRETS_DIR_NAME: &str = "micold-ai-ide-secrets";

/// [`secrets_dir`] for a given data directory.
pub fn secrets_dir_beside(data_dir: &std::path::Path) -> Option<PathBuf> {
    data_dir
        .parent()
        .map(|parent| parent.join(SECRETS_DIR_NAME))
}

/// `name` under [`data_dir`].
pub fn data_subdir(name: &str) -> Option<PathBuf> {
    data_dir().map(|dir| dir.join(name))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn the_data_dir_variable_names_the_directory_and_empty_means_unset() {
        assert_eq!(
            override_from(Some("/var/lib/micold-ai-ide".into())),
            Some(PathBuf::from("/var/lib/micold-ai-ide"))
        );
        assert_eq!(override_from(Some("".into())), None);
        assert_eq!(override_from(None), None);
    }

    #[test]
    fn the_secrets_dir_is_a_sibling_of_the_data_dir_on_every_layout() {
        for data in [
            "/home/u/.local/share/micold-ai-ide",
            "/Users/u/Library/Application Support/micold-ai-ide",
            // The Windows shape, `Roaming/micold-ai-ide/data`, in this platform's separators.
            "/Users/u/AppData/Roaming/micold-ai-ide/data",
        ] {
            let data = Path::new(data);
            let secrets = secrets_dir_beside(data).unwrap();
            assert_eq!(
                secrets.file_name().unwrap(),
                "micold-ai-ide-secrets",
                "{data:?}"
            );
            assert_eq!(secrets.parent(), data.parent(), "{data:?}");
            assert!(
                !secrets.starts_with(data) && !data.starts_with(&secrets),
                "{data:?}"
            );
        }
    }

    #[test]
    fn the_real_secrets_dir_is_outside_the_data_dir() {
        if let (Some(data), Some(secrets)) = (data_dir(), secrets_dir()) {
            assert!(!secrets.starts_with(&data) && !data.starts_with(&secrets));
        }
    }
}
