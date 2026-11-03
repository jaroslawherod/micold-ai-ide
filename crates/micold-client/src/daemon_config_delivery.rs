//! Host config delivery (#1526, M1a, FR-005, FR-014): write a daemon's rendered `daemon.toml`
//! to its data directory before the host process is spawned, only when it changed, mode `0600`.
//!
//! The file holds no secret value — `daemon_config_render` already keeps those out — so there is
//! nothing more to protect than the file's own contents from another local user; `0600` is enough.

use std::io;
use std::path::Path;

use crate::daemon_config_render::{self, DaemonSettings};
use micold_core::daemon_config::Kind;

/// Renders `settings` for [`Kind::Host`] and writes it to `<data_dir>/daemon.toml`, skipping the
/// write (but not the mode fix) when the file already holds the same bytes. Returns whether it wrote.
pub fn deliver_host(settings: &DaemonSettings, data_dir: &Path) -> io::Result<bool> {
    let text = daemon_config_render::prepare(settings, Kind::Host)
        .map_err(|errors| io::Error::other(errors[0].to_string()))?;
    write_if_changed(
        &data_dir.join(daemon_config_render::ORIGIN),
        text.as_bytes(),
    )
}

/// Writes `bytes` to `path` only when the file does not already hold them, then (on unix) sets
/// its mode to `0600`. The file holds no secret, but there is no reason to leave it world- or
/// group-readable either.
fn write_if_changed(path: &Path, bytes: &[u8]) -> io::Result<bool> {
    if let Ok(existing) = std::fs::read(path) {
        if existing == bytes {
            // Same content: nothing to write, but a hand-widened mode is corrected, as the SSH
            // path does (I5-7).
            set_owner_only(path)?;
            return Ok(false);
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    set_owner_only(path)?;
    Ok(true)
}

#[cfg(unix)]
fn set_owner_only(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(windows)]
fn set_owner_only(_path: &Path) -> io::Result<()> {
    Ok(())
}
