//! Host config delivery (#1526, M1a, FR-005, FR-014): write a daemon's rendered `daemon.toml`
//! to its data directory before the host process is spawned, only when it changed, mode `0600`.
//!
//! The file holds no secret value — `daemon_config_render` already keeps those out — so there is
//! nothing more to protect than the file's own contents from another local user; `0600` is enough.

use std::io;
use std::path::{Path, PathBuf};

use crate::daemon_config_render::{self, DaemonSettings, GithubTokenSetting, StartupTable};
use micold_core::daemon_config::Kind;
use micold_core::daemons::SessionExtras;
use micold_core::github::GithubRepo;
use micold_core::scoped_credential::SecretToken;

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

/// The file names the extras' sources are delivered under, beside the configuration (#1536).
pub const SSH_KEY_NAME: &str = "ssh-key";
pub const KNOWN_HOSTS_NAME: &str = "known-hosts";
pub const SIGNING_KEY_NAME: &str = "signing-key";
pub const CA_NAME: &str = "ca.pem";
/// A session file's delivered name is this prefix and its position in `[[session.files]]`.
pub const SESSION_FILE_PREFIX: &str = "session-file-";

/// The extras with their sources read for delivery: `extras` names the delivered copies, and
/// `files` is each copy's name and bytes (#1536, FR-036..038).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedExtras {
    pub extras: SessionExtras,
    pub files: Vec<(String, Vec<u8>)>,
}

/// Reads the sources `extras` names and points them at `<dir>/<name>`, the POSIX directory where
/// the daemon finds its delivered files. `keys` says whether the SSH key, `known_hosts` file and
/// signing key are delivered (container and pod) or left out (an SSH host, contract: C and K
/// only); the CA file and the session files go everywhere. An unreadable source is an error naming
/// it, so the daemon does not start without a file the user chose.
pub fn stage_extras(extras: &SessionExtras, dir: &str, keys: bool) -> io::Result<StagedExtras> {
    let mut staged = StagedExtras {
        extras: SessionExtras::default(),
        files: Vec::new(),
    };
    let mut take = |source: &Option<PathBuf>, name: String| -> io::Result<Option<PathBuf>> {
        let Some(source) = source else {
            return Ok(None);
        };
        let bytes = std::fs::read(source).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("{} could not be read: {e}", source.display()),
            )
        })?;
        let at = PathBuf::from(format!("{dir}/{name}"));
        staged.files.push((name, bytes));
        Ok(Some(at))
    };
    if keys {
        staged.extras.ssh_key_file = take(&extras.ssh_key_file, SSH_KEY_NAME.into())?;
        staged.extras.known_hosts_file = take(&extras.known_hosts_file, KNOWN_HOSTS_NAME.into())?;
        staged.extras.signing_key_file = take(&extras.signing_key_file, SIGNING_KEY_NAME.into())?;
    }
    staged.extras.ca_file = take(&extras.ca_file, CA_NAME.into())?;
    for (n, file) in extras.files.iter().enumerate() {
        let at = take(
            &Some(file.file.clone()),
            format!("{SESSION_FILE_PREFIX}{n}"),
        )?;
        staged
            .extras
            .files
            .push(micold_core::daemons::SessionFileSetting {
                file: at.expect("a source was given"),
                ..file.clone()
            });
    }
    Ok(staged)
}

/// Makes `dir` hold exactly the staged files among the names delivery owns: each is written (only
/// when changed, owner-only), and a copy of an earlier start that the extras no longer name is
/// removed. Other files (`startup.env`, `gitconfig`) are left alone. Whether anything changed.
fn sync_extras(dir: &Path, files: &[(String, Vec<u8>)]) -> io::Result<bool> {
    let mut changed = false;
    for (name, bytes) in files {
        changed |= write_if_changed(&dir.join(name), bytes)?;
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let owned = [SSH_KEY_NAME, KNOWN_HOSTS_NAME, SIGNING_KEY_NAME, CA_NAME].contains(&&*name)
            || name.starts_with(SESSION_FILE_PREFIX);
        if owned && !files.iter().any(|(wanted, _)| *wanted == name) {
            changed |= remove_if_present(&entry.path())?;
        }
    }
    Ok(changed)
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
///
/// `extras` are the SSH key, `known_hosts`, signing key, CA file and session files (#1536): their
/// sources are copied into `delivery.secrets` (owner-only; a copy the extras no longer name is
/// removed) and the file names them at `/run/micold/secrets/<name>`.
///
/// `git_config` is the sanitized git configuration ([`micold_core::scoped_credential::shared_copy`]):
/// it goes into `delivery.secrets` and the file names it as `[git] config_file` at
/// `/run/micold/secrets/gitconfig` (#1530); `None` removes a copy left by an earlier start.
#[allow(clippy::too_many_arguments)]
pub fn deliver_container(
    name: &str,
    profile: &micold_core::sandbox::SandboxProfile,
    home: &Path,
    delivery: &micold_core::sandbox::DeliveryMount,
    git_config: Option<&str>,
    startup: Option<&StartupTable>,
    extras: &SessionExtras,
    wire_id: Option<&micold_core::protocol::routing::WireDaemonId>,
) -> io::Result<bool> {
    use micold_core::daemon_config::{IdleStopSetting, Listen, SocketSpec};
    use micold_core::sandbox::argv::CONTAINER_SSH_DIR;
    use micold_core::sandbox::{CONTAINER_CONFIG_DIR, CONTAINER_SECRETS_DIR};
    use micold_core::ssh::argv::CONTAINER_DATA_DIR;
    let staged = stage_extras(extras, CONTAINER_SECRETS_DIR, true)?;
    let settings = DaemonSettings {
        data_dir: Some(CONTAINER_DATA_DIR.into()),
        home: Some(home.to_path_buf()),
        image: Some(profile.image.reference().to_string()),
        idle_stop: profile.survive_logout.then_some(IdleStopSetting::Off),
        ssh_dir: Some(CONTAINER_SSH_DIR.into()),
        startup: startup.cloned(),
        extras: staged.extras,
        // The sanitized copy of the user's `.gitconfig`, delivered in the secrets folder (read-only)
        // whenever the host has one (#1530, D15 row 1).
        git_config_file: git_config.map(|_| {
            PathBuf::from(format!(
                "{CONTAINER_SECRETS_DIR}/{}",
                micold_core::sandbox::SANDBOX_GITCONFIG_FILE
            ))
        }),
        // The id it announces to the orchestrator (#1726, R6): without it the daemon is `local`.
        wire_id: wire_id.cloned(),
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
    let git_config_copy = delivery
        .secrets
        .join(micold_core::sandbox::SANDBOX_GITCONFIG_FILE);
    let mut wrote = match git_config {
        Some(text) => write_if_changed(&git_config_copy, text.as_bytes())?,
        None => remove_if_present(&git_config_copy)?,
    };
    wrote |= sync_extras(&delivery.secrets, &staged.files)?;
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
    /// The extras' delivered copies as secret keys and text (#1536), named at the startup secret's
    /// mount by `settings.extras`.
    pub files: Vec<(String, String)>,
}

/// A config map or secret holds 1 MiB in all; half of it for one file leaves room for the rest.
const POD_FILE_LIMIT: usize = 512 * 1024;

/// Reads `settings`' start script and `env_file` for a pod (#1533, FR-011): the script goes in the
/// config map beside `daemon.toml` and the `env_file` in the startup secret, and the rendered file
/// names them at their mounts. A file that cannot be read, or is not text, stops the start naming
/// it, so the pod does not start without the script the user chose.
pub fn pod_startup(settings: &DaemonSettings) -> io::Result<PodStartup> {
    use micold_core::kube::manifest::STARTUP_ENV_MOUNT;
    // The extras go into the startup secret too (#1536): a secret holds 1 MiB in all, so each
    // file is held to the same limit as the script, and all of them must be text.
    let staged = stage_extras(&settings.extras, STARTUP_ENV_MOUNT, true)?;
    let mut settings = settings.clone();
    settings.extras = staged.extras;
    let mut extras = Vec::new();
    for (name, bytes) in staged.files {
        extras.push((name.clone(), pod_text(&name, bytes)?));
    }
    let settings = &settings;
    let Some(files) = read_startup_files(settings)? else {
        return Ok(PodStartup {
            settings: settings.clone(),
            script: None,
            env: None,
            files: extras,
        });
    };
    use micold_core::kube::manifest::{CONFIG_MOUNT, STARTUP_ENV_FILE, STARTUP_SCRIPT_FILE};
    let table = settings.startup.as_ref().expect("files were read");
    let text = |what: &str, path: &Path, bytes: Vec<u8>| {
        pod_text(
            format_args!("the start script's {what} {}", path.display()),
            bytes,
        )
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
        files: extras,
    })
}

/// The stored tokens a pod daemon receives (#1760, FR-009): those whose filter names one of the
/// daemon's saved projects' repositories, and those with no filter (their reach is unknown here),
/// in Settings order. No repository, no token.
pub fn select_tokens(
    tokens: Vec<GithubTokenSetting>,
    repositories: &[GithubRepo],
) -> Vec<GithubTokenSetting> {
    use micold_core::scoped_credential::token_choice::{reaches, Filter};
    tokens
        .into_iter()
        .filter(|token| {
            let filter = Filter {
                name: &token.name,
                repos: &token.repos,
            };
            repositories.iter().any(|repo| reaches(&filter, repo))
        })
        .collect()
}

/// The selected tokens read for a pod (#1760, FR-001): `entries` render as `[[github.tokens]]`
/// naming each token at the startup Secret's mount, and `files` are the Secret's keys and values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodGithubTokens {
    pub entries: Vec<GithubTokenSetting>,
    pub files: Vec<(String, SecretToken)>,
}

/// A file's text for a pod's config map or startup secret: one over [`POD_FILE_LIMIT`], or not
/// text, is refused naming it as `what` (never quoting the content).
fn pod_text(what: impl std::fmt::Display, bytes: Vec<u8>) -> io::Result<String> {
    let invalid = |reason: String| io::Error::new(io::ErrorKind::InvalidData, reason);
    if bytes.len() > POD_FILE_LIMIT {
        return Err(invalid(format!(
            "{what} is {} bytes, over the {POD_FILE_LIMIT} a pod can hold",
            bytes.len()
        )));
    }
    String::from_utf8(bytes).map_err(|_| {
        invalid(format!(
            "{what} is not text, so it cannot be delivered to a pod"
        ))
    })
}

/// The data a Kubernetes Secret holds in all, which the API server enforces.
pub const SECRET_LIMIT: usize = 1024 * 1024;

/// Refuses a startup Secret whose `values` together pass [`SECRET_LIMIT`], so the start fails
/// before anything is created instead of the API server refusing the Secret partway through.
pub fn check_secret_size<'a>(values: impl IntoIterator<Item = &'a str>) -> io::Result<()> {
    let total: usize = values.into_iter().map(str::len).sum();
    if total > SECRET_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "the pod's startup Secret would hold {total} bytes, over the {SECRET_LIMIT} bytes \
                 Kubernetes allows; deliver fewer or smaller files and tokens"
            ),
        ));
    }
    Ok(())
}

/// A delivered token's key in the startup Secret is this prefix and the token's lower-case name
/// (the PAT store's file name), so a key always holds the same token, whatever the set's order.
pub const GITHUB_TOKEN_PREFIX: &str = "github-token-";

impl PodGithubTokens {
    /// Reads each selected token's file, verbatim (the daemon parses the stored shape), for the
    /// pod's startup Secret. A file that cannot be read, is empty, is not text, is over
    /// [`POD_FILE_LIMIT`] or, on Unix, can be read by another user stops the start naming the
    /// token, so the pod never starts with a token silently missing (#1533's stance).
    pub fn read(selected: Vec<GithubTokenSetting>) -> io::Result<Self> {
        use micold_core::kube::manifest::STARTUP_ENV_MOUNT;
        let mut read = Self {
            entries: Vec::with_capacity(selected.len()),
            files: Vec::with_capacity(selected.len()),
        };
        for entry in selected {
            let refused = |e: io::Error| {
                io::Error::new(
                    e.kind(),
                    format!(
                        "cannot deliver the GitHub token \"{}\" to the pod: {e}",
                        entry.name
                    ),
                )
            };
            if !micold_core::scoped_credential::pat_store::valid_name(&entry.name) {
                return Err(refused(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "not a valid token name",
                )));
            }
            let value = read_token_file(&entry.token_file).map_err(refused)?;
            let key = format!("{GITHUB_TOKEN_PREFIX}{}", entry.name.to_ascii_lowercase());
            read.entries.push(GithubTokenSetting {
                token_file: PathBuf::from(format!("{STARTUP_ENV_MOUNT}/{key}")),
                ..entry
            });
            read.files.push((key, value));
        }
        Ok(read)
    }
}

/// A token file's text, or why it cannot go to a pod (never quoting the content).
fn read_token_file(path: &Path) -> io::Result<SecretToken> {
    let bytes = std::fs::read(path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("{} could not be read: {e}", path.display()),
        )
    })?;
    let invalid = |reason: String| io::Error::new(io::ErrorKind::InvalidData, reason);
    if let Some(mode) = micold_core::daemon_config::readable_by_others(path) {
        return Err(invalid(format!(
            "{} can be read by other users (mode {mode:o}); make it owner-only (chmod 600)",
            path.display()
        )));
    }
    if bytes.is_empty() {
        return Err(invalid(format!("{} is empty", path.display())));
    }
    pod_text(path.display(), bytes).map(SecretToken::new)
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
