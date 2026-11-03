//! Host config delivery (#1526, M1a, FR-005, FR-014): write a daemon's rendered `daemon.toml`
//! to its data directory before the host process is spawned, only when it changed, mode `0600`.
//!
//! The file holds no secret value — `daemon_config_render` already keeps those out — so there is
//! nothing more to protect than the file's own contents from another local user; `0600` is enough.

use std::io;
use std::path::{Path, PathBuf};

use crate::daemon_config_render::{self, DaemonSettings, StartupTable};
use micold_core::daemon_config::Kind;

/// The script's file name beside the delivered configuration (host data dir; SSH `secrets/`).
pub const SCRIPT_NAME: &str = "startup.sh";
/// The `env_file`'s name beside it.
pub const ENV_NAME: &str = "startup.env";

/// The bytes of a daemon's start script and `env_file`, read on this machine for delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupFiles {
    pub script: Vec<u8>,
    pub env: Option<Vec<u8>>,
}

/// Reads the start script and `env_file` the settings name. `Ok(None)` when the daemon has no
/// start script. An unreadable file is an error naming it, so the delivery stops instead of the
/// daemon starting without the script the user chose (FR-011).
pub fn read_startup_files(settings: &DaemonSettings) -> io::Result<Option<StartupFiles>> {
    let Some(table) = &settings.startup else {
        return Ok(None);
    };
    let read = |what: &str, path: &Path| {
        std::fs::read(path).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "the start script's {what} {} could not be read: {e}",
                    path.display()
                ),
            )
        })
    };
    Ok(Some(StartupFiles {
        script: read("script", &table.script_file)?,
        env: table
            .settings
            .env_file
            .as_deref()
            .map(|path| read("env_file", path))
            .transpose()?,
    }))
}

/// `settings` with the start script (and `env_file`, when it has one) pointing at where delivery
/// put them: `script` and `env`.
pub fn relocated_startup(
    settings: &DaemonSettings,
    script: PathBuf,
    env: PathBuf,
) -> DaemonSettings {
    let mut moved = settings.clone();
    if let Some(StartupTable {
        script_file,
        settings: table,
    }) = &mut moved.startup
    {
        *script_file = script;
        if table.env_file.is_some() {
            table.env_file = Some(env);
        }
    }
    moved
}

/// Renders `settings` for [`Kind::Host`] and writes it to `<data_dir>/daemon.toml`, skipping the
/// write (but not the mode fix) when the file already holds the same bytes. Returns whether it wrote.
///
/// A start script and `env_file` are written beside it (`startup.sh`, `startup.env`, mode `0600`),
/// and the file names those copies, so the script that runs is the one delivered (FR-011). Neither
/// the script text nor a secret value is in `daemon.toml` (US5.4).
pub fn deliver_host(settings: &DaemonSettings, data_dir: &Path) -> io::Result<bool> {
    let files = read_startup_files(settings)?;
    let settings = match &files {
        Some(_) => std::borrow::Cow::Owned(relocated_startup(
            settings,
            data_dir.join(SCRIPT_NAME),
            data_dir.join(ENV_NAME),
        )),
        None => std::borrow::Cow::Borrowed(settings),
    };
    let text = daemon_config_render::prepare(&settings, Kind::Host)
        .map_err(|errors| io::Error::other(errors[0].to_string()))?;
    let mut wrote = false;
    if let Some(files) = &files {
        wrote |= write_if_changed(&data_dir.join(SCRIPT_NAME), &files.script)?;
        if let Some(env) = &files.env {
            wrote |= write_if_changed(&data_dir.join(ENV_NAME), env)?;
        }
    }
    let config = write_if_changed(
        &data_dir.join(daemon_config_render::ORIGIN),
        text.as_bytes(),
    )?;
    Ok(wrote | config)
}

/// The `daemon.toml` of a container daemon, and the directories it is delivered in (#1527, FR-005).
///
/// The same file for a container the boot brings up and one the SSH bridge starts the daemon in:
/// the container mounts `delivery.config` read-only at `/run/micold/config`, where the daemon
/// started by the entrypoint and the one started over SSH both read it. The client reaches the
/// container through its SSH server, so the daemon listens on its socket, as it did when `-e` values
/// told it so. Returns whether it wrote.
pub fn deliver_container(
    name: &str,
    profile: &micold_core::sandbox::SandboxProfile,
    home: &Path,
    delivery: &micold_core::sandbox::DeliveryMount,
) -> io::Result<bool> {
    use micold_core::daemon_config::{IdleStopSetting, Listen, SocketSpec};
    use micold_core::sandbox::argv::CONTAINER_SSH_DIR;
    use micold_core::ssh::argv::CONTAINER_DATA_DIR;
    let settings = DaemonSettings {
        data_dir: Some(CONTAINER_DATA_DIR.into()),
        home: Some(home.to_path_buf()),
        image: Some(profile.image.reference().to_string()),
        idle_stop: profile.survive_logout.then_some(IdleStopSetting::Off),
        ssh_dir: Some(CONTAINER_SSH_DIR.into()),
        ..DaemonSettings::new(name, Listen::Socket(SocketSpec::Auto))
    };
    let text = daemon_config_render::prepare(&settings, Kind::Container)
        .map_err(|errors| io::Error::other(errors[0].to_string()))?;
    // Created here, as the user, so the runtime does not create them as root (as for the sandbox
    // home), and the secrets directory exists for the files delivered into it.
    micold_core::owner_only::ensure_dir(&delivery.config)?;
    micold_core::owner_only::ensure_dir(&delivery.secrets)?;
    write_if_changed(
        &delivery.config.join(daemon_config_render::ORIGIN),
        text.as_bytes(),
    )
}

/// Writes `bytes` to `path` only when the file does not already hold them, by writing a temporary
/// file and renaming it (contract R9: a daemon that reads the file as it is replaced sees the old
/// one or the new one, never half of it), mode `0600`. The file holds no secret, but there is no
/// reason to leave it world- or group-readable either.
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
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    // Created owner-only, so a secret (`startup.env`) is never readable in between.
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    io::Write::write_all(&mut options.open(path)?, bytes)?;
    set_owner_only(path)?;
    Ok(true)
}
