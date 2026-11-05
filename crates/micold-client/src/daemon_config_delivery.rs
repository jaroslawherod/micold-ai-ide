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
///
/// A start script goes into `delivery.config` and its `env_file` into `delivery.secrets`, and the
/// file names them at `/run/micold/config/startup.sh` and `/run/micold/secrets/startup.env`
/// (#1533, FR-011). Without a script, a copy left by an earlier one is removed.
pub fn deliver_container(
    name: &str,
    profile: &micold_core::sandbox::SandboxProfile,
    home: &Path,
    delivery: &micold_core::sandbox::DeliveryMount,
    startup: Option<&StartupTable>,
) -> io::Result<bool> {
    use micold_core::daemon_config::{IdleStopSetting, Listen, SocketSpec};
    use micold_core::sandbox::argv::CONTAINER_SSH_DIR;
    use micold_core::sandbox::{CONTAINER_CONFIG_DIR, CONTAINER_SECRETS_DIR};
    use micold_core::ssh::argv::CONTAINER_DATA_DIR;
    let settings = DaemonSettings {
        data_dir: Some(CONTAINER_DATA_DIR.into()),
        home: Some(home.to_path_buf()),
        image: Some(profile.image.reference().to_string()),
        idle_stop: profile.survive_logout.then_some(IdleStopSetting::Off),
        ssh_dir: Some(CONTAINER_SSH_DIR.into()),
        startup: startup.cloned(),
        ..DaemonSettings::new(name, Listen::Socket(SocketSpec::Auto))
    };
    let files = read_startup_files(&settings)?;
    // POSIX paths inside the container, whatever this client is.
    let settings = match &files {
        Some(_) => relocated_startup(
            &settings,
            format!("{CONTAINER_CONFIG_DIR}/{SCRIPT_NAME}").into(),
            format!("{CONTAINER_SECRETS_DIR}/{ENV_NAME}").into(),
        ),
        None => settings,
    };
    let text = daemon_config_render::prepare(&settings, Kind::Container)
        .map_err(|errors| io::Error::other(errors[0].to_string()))?;
    // Created here, as the user, so the runtime does not create them as root (as for the sandbox
    // home), and the secrets directory exists for the files delivered into it.
    micold_core::owner_only::ensure_dir(&delivery.config)?;
    micold_core::owner_only::ensure_dir(&delivery.secrets)?;
    let script = delivery.config.join(SCRIPT_NAME);
    let env = delivery.secrets.join(ENV_NAME);
    let mut wrote = false;
    match &files {
        Some(files) => {
            wrote |= write_if_changed(&script, &files.script)?;
            match &files.env {
                Some(bytes) => wrote |= write_if_changed(&env, bytes)?,
                None => wrote |= remove_if_present(&env)?,
            }
        }
        None => {
            wrote |= remove_if_present(&script)?;
            wrote |= remove_if_present(&env)?;
        }
    }
    let config = write_if_changed(
        &delivery.config.join(daemon_config_render::ORIGIN),
        text.as_bytes(),
    )?;
    Ok(wrote | config)
}

/// What a pod's start script delivers: the settings to render, naming the files where the pod
/// mounts them, and the texts for the config map (script) and the startup secret (`env_file`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodStartup {
    pub settings: DaemonSettings,
    pub script: Option<String>,
    pub env: Option<String>,
}

/// A config map or secret holds 1 MiB in all; half of it for one file leaves room for the rest.
const POD_FILE_LIMIT: usize = 512 * 1024;

/// Reads `settings`' start script and `env_file` for a pod (#1533, FR-011): the script goes in the
/// config map beside `daemon.toml` and the `env_file` in the startup secret, and the rendered file
/// names them at their mounts. A file that cannot be read, or is not text, stops the start naming
/// it, so the pod does not start without the script the user chose.
pub fn pod_startup(settings: &DaemonSettings) -> io::Result<PodStartup> {
    use micold_core::kube::manifest::{
        CONFIG_MOUNT, STARTUP_ENV_FILE, STARTUP_ENV_MOUNT, STARTUP_SCRIPT_FILE,
    };
    let Some(files) = read_startup_files(settings)? else {
        return Ok(PodStartup {
            settings: settings.clone(),
            script: None,
            env: None,
        });
    };
    let table = settings.startup.as_ref().expect("files were read");
    let text = |what: &str, path: &Path, bytes: Vec<u8>| {
        if bytes.len() > POD_FILE_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "the start script's {what} {} is {} bytes, over the {POD_FILE_LIMIT} a pod can hold",
                    path.display(),
                    bytes.len()
                ),
            ));
        }
        String::from_utf8(bytes).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "the start script's {what} is not text, so it cannot be delivered to a pod"
                ),
            )
        })
    };
    Ok(PodStartup {
        settings: relocated_startup(
            settings,
            format!("{CONFIG_MOUNT}/{STARTUP_SCRIPT_FILE}").into(),
            format!("{STARTUP_ENV_MOUNT}/{STARTUP_ENV_FILE}").into(),
        ),
        script: Some(text("script", &table.script_file, files.script)?),
        env: files
            .env
            .map(|bytes| {
                let path = table.settings.env_file.as_deref().unwrap_or(Path::new(""));
                text("env_file", path, bytes)
            })
            .transpose()?,
    })
}

/// Removes `path` when it exists; whether it did.
fn remove_if_present(path: &Path) -> io::Result<bool> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
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

#[cfg(unix)]
fn set_owner_only(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(windows)]
fn set_owner_only(_path: &Path) -> io::Result<()> {
    Ok(())
}
