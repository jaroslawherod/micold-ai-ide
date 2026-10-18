//! The Daemons feature's entry point in the shell (feature 491, M3).
//!
//! The reducer ([`micold_client::features::daemons`]) changes the registry, the links and the
//! bindings in place and reports what has to leave the process as outcomes. This performs them:
//! it writes the registry and a binding through [`App::apply_daemon_outcome`], stops the sessions
//! of an edited or removed daemon on that daemon's own outbox, and registers a newly bound
//! project. Starting and stopping need nothing more: the connection subscription reads the links
//! ([`micold_client::links::DaemonLinks::should_dial`]), so a started daemon is dialled and a
//! stopped one is neither dialled nor reconnected.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use iced::Task;
use micold_client::app::Message;
use micold_client::features::daemons::Msg;
use micold_client::features::Outcome;
use micold_core::daemons::DaemonId;
use micold_core::protocol::messages::ClientMsg;

use crate::App;

/// Apply a Daemons message and carry out what it asks for.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    // Taken before the reducer: stopping or removing a daemon drops its outbox, and its sessions
    // are stopped on it.
    let outboxes: BTreeMap<DaemonId, _> = app
        .core
        .settings
        .daemons
        .entries()
        .iter()
        .filter_map(|e| app.outbox_for(e.id).map(|o| (e.id, o)))
        .collect();
    // Ports the store holds that this window's registry copy may not: a picked port avoids them.
    let taken_elsewhere = if matches!(msg, Msg::Saved) {
        app.caps
            .settings()
            .map(|store| store.load().settings.registry().container_ports())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    // A forgotten Kubernetes daemon is gone from the registry when its key files are deleted: the
    // key they are filed under comes from its settings, read first.
    let kube_keys: BTreeMap<DaemonId, String> = app
        .core
        .settings
        .daemons
        .entries()
        .iter()
        .filter_map(|e| match &e.runtime {
            micold_core::daemons::DaemonRuntime::Kubernetes(k) => Some((
                e.id,
                daemon_key(&k.context.0, &k.namespace.0, e.name.as_str()),
            )),
            _ => None,
        })
        .collect();
    let outcomes = app.core.update_daemons(msg, taken_elsewhere);
    let mut rest = Vec::new();
    let mut bound = Vec::new();
    let mut gate = Vec::new();
    let mut tasks = Vec::new();
    for outcome in outcomes {
        match &outcome {
            Outcome::KubeContextsRequested => tasks.push(list_kube_contexts()),
            Outcome::KubeStopRequested {
                daemon,
                settings,
                name,
            } => tasks.push(stop_kube(app, *daemon, settings, name)),
            Outcome::KubeRemoveRequested {
                daemon,
                settings,
                name,
            } => tasks.push(remove_kube(app, *daemon, settings, name)),
            Outcome::KubeLogsRequested {
                daemon,
                settings,
                tail,
            } => tasks.push(kube_logs(app, *daemon, settings, *tail)),
            Outcome::KubeKeysDelete { daemon } => {
                if let Some(key) = kube_keys.get(daemon) {
                    delete_kube_keys(key);
                }
            }
            Outcome::DaemonRegistryChanged(_) => app.apply_daemon_outcome(&outcome),
            Outcome::DaemonBound { daemon, .. } => {
                app.apply_daemon_outcome(&outcome);
                bound.push(*daemon);
            }
            Outcome::DaemonSessionsEnded { daemon, sessions } => {
                if let Some(outbox) = outboxes.get(daemon) {
                    for session in sessions {
                        outbox.send(ClientMsg::SessionStop { session: *session });
                    }
                }
                rest.push(outcome);
            }
            Outcome::RemoteRepositoryRequested { path } => {
                // Judged by the daemon it names, then opened as a project bound to it.
                gate.push(path.clone());
            }
            _ => rest.push(outcome),
        }
    }
    // A daemon added since launch is tracked, a removed one forgotten with its catalog.
    app.sync_links();
    // A daemon stopped (or removed, or restarted by an edit) no longer has a link: its catalog,
    // pending requests and "connected" standing go the way a lost connection's do, so its
    // sessions are not shown as live (`Stopped` ignores the `Lost` this folds).
    let gone: Vec<DaemonId> = app
        .daemon_catalogs
        .keys()
        .copied()
        .filter(|id| {
            !matches!(
                app.core.settings.links.state(*id),
                Some(micold_core::daemons::DaemonState::Connected)
                    | Some(micold_core::daemons::DaemonState::Starting)
            )
        })
        .collect();
    for id in gone {
        let _ = crate::shell::daemon_sync::on_disconnected(app, id);
    }
    // A project first bound to a connected daemon is registered there before a session starts.
    for daemon in bound {
        crate::shell::daemon_sync::register_projects(app, daemon);
    }
    micold_client::app::drain(rest, |outcome| {
        micold_client::app::interpret(&mut app.core, outcome)
    });
    for path in gate {
        let _ = crate::shell::workspace::on_folder_chosen(app, path);
    }
    Task::batch(tasks)
}

/// Read the merged kubeconfig and list its contexts off the UI thread (feature 686, FR-002). Only
/// names and default namespaces leave the files: no credential is read into the form.
fn list_kube_contexts() -> Task<Message> {
    use micold_client::kube::live;
    use micold_core::ssh::argv::ClientPlatform;
    let read = || {
        let home = directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_default();
        let env = std::env::var_os("KUBECONFIG");
        let paths = micold_core::kube::kubeconfig::candidates(
            env.as_deref(),
            &home,
            ClientPlatform::current(),
        );
        live::read_kubeconfigs(&paths)
            .map(|config| live::list_contexts(&config))
            .map_err(|failure| failure.to_string())
    };
    Task::perform(
        async move {
            tokio::task::spawn_blocking(read)
                .await
                .unwrap_or_else(|joined| Err(joined.to_string()))
        },
        |listed| Message::Daemons(Msg::ContextsListed(listed)),
    )
}

/// The key a Kubernetes daemon's client-side key files are filed under: the same key its cluster
/// objects carry (`ResourceNames`).
fn daemon_key(context: &str, namespace: &str, name: &str) -> String {
    micold_core::kube::names::DaemonKey::of(context, namespace, name)
        .as_str()
        .to_string()
}

/// The directory holding every Kubernetes daemon's key directory.
pub fn kube_keys_root() -> PathBuf {
    micold_core::paths::data_subdir("kube-keys")
        .unwrap_or_else(|| std::env::temp_dir().join("micold-kube-keys"))
}

/// Delete this client's key files and pinned host key for the daemon filed under `key`.
fn delete_kube_keys(key: &str) {
    let _ = micold_client::kube::keys::KubeKeys::new(&kube_keys_root(), key).delete();
}

/// Stop the pod of a Kubernetes daemon off the UI thread (FR-014). A failure is shown in the
/// daemon's log lines.
fn stop_kube(
    app: &App,
    daemon: DaemonId,
    settings: &micold_core::kube::settings::KubeSettings,
    name: &str,
) -> Task<Message> {
    let connect = app.caps.kube_connector();
    let (context, namespace) = (settings.context.0.clone(), settings.namespace.0.clone());
    let names = micold_core::kube::names::ResourceNames::new(&context, &namespace, name);
    Task::perform(
        async move {
            let cluster = connect(context).await?;
            micold_core::kube::lifecycle::stop(&*cluster, &namespace, &names).await
        },
        move |result| match result {
            Ok(()) => Message::NoOp,
            Err(failure) => {
                Message::Daemons(Msg::LogLine(daemon, format!("Stop failed: {failure}")))
            }
        },
    )
}

/// Delete every cluster object of a Kubernetes daemon off the UI thread (FR-015).
fn remove_kube(
    app: &App,
    daemon: DaemonId,
    settings: &micold_core::kube::settings::KubeSettings,
    name: &str,
) -> Task<Message> {
    let connect = app.caps.kube_connector();
    let (context, namespace) = (settings.context.0.clone(), settings.namespace.0.clone());
    let allowed = settings.allowed.clone();
    let names = micold_core::kube::names::ResourceNames::new(&context, &namespace, name);
    Task::perform(
        async move {
            let cluster = connect(context).await?;
            micold_core::kube::lifecycle::remove(&*cluster, &namespace, &names, &allowed).await
        },
        move |result| {
            Message::Daemons(Msg::KubeRemoved(
                daemon,
                result.map_err(|failure| failure.to_string()),
            ))
        },
    )
}

/// Fetch the tail of a Kubernetes daemon's pod log off the UI thread (FR-018).
fn kube_logs(
    app: &App,
    daemon: DaemonId,
    settings: &micold_core::kube::settings::KubeSettings,
    tail: u32,
) -> Task<Message> {
    let connect = app.caps.kube_connector();
    let (context, namespace) = (settings.context.0.clone(), settings.namespace.0.clone());
    let name = app
        .core
        .settings
        .daemons
        .get(daemon)
        .map(|e| e.name.as_str().to_string())
        .unwrap_or_default();
    let pod = micold_core::kube::names::ResourceNames::new(&context, &namespace, &name).pod();
    Task::perform(
        async move {
            let cluster = connect(context).await.map_err(|f| f.to_string())?;
            cluster
                .logs(&namespace, &pod, tail)
                .await
                .map_err(|f| f.to_string())
        },
        move |result| Message::Daemons(Msg::LogsFetched(daemon, result)),
    )
}

/// The `ssh` executable on Windows: the one on `PATH`, then the OpenSSH that ships with Windows
/// (`%SystemRoot%\\System32\\OpenSSH\\ssh.exe`). `None` leaves the lookup to the runner, which
/// reports a missing `ssh` with the hint to install it. `is_file` is injected so the order is
/// testable on any platform.
pub fn locate_windows_ssh(
    path_var: &str,
    system_root: Option<&Path>,
    is_file: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let on_path = std::env::split_paths(path_var)
        .map(|dir| dir.join("ssh.exe"))
        .find(|p| is_file(p));
    on_path.or_else(|| {
        system_root
            .map(|root| root.join("System32").join("OpenSSH").join("ssh.exe"))
            .filter(|p| is_file(p))
    })
}

/// The one real `ssh` runner of this process (feature 489, FR-017): this build's own daemon is the
/// artefact copied to a host of the same platform, and the executable is looked up by name, except
/// on Windows where OpenSSH is not always on `PATH`.
pub fn system_ssh() -> micold_core::ssh::SystemSsh {
    let ssh_path = if cfg!(windows) {
        locate_windows_ssh(
            &std::env::var("PATH").unwrap_or_default(),
            std::env::var_os("SystemRoot").map(PathBuf::from).as_deref(),
            &|p| p.is_file(),
        )
    } else {
        None
    };
    micold_core::ssh::SystemSsh {
        own_daemon: PathBuf::from(micold_core::spawn::daemon_binary()),
        // The static Linux builds for remote hosts and containers ship with the package.
        bundled_dir: std::env::current_exe().ok().map(|exe| bundled_dir(&exe)),
        ssh_path,
        cancel: None,
    }
}

/// Where a package puts `micold-daemon-linux-<arch>` relative to the client executable (feature
/// 806, FR-009): `Contents/Resources/remote` in the macOS bundle (`Contents/MacOS` holds only the
/// two executables), `../lib/micold-ai-ide/remote` for the Linux packages that install into
/// `bin/`, and `remote` beside the executable on Windows and in a development tree.
fn bundled_dir(exe: &Path) -> PathBuf {
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    match dir.file_name().and_then(|n| n.to_str()) {
        Some("MacOS") if dir.parent().is_some_and(|c| c.ends_with("Contents")) => {
            dir.join("../Resources/remote")
        }
        Some("bin") => dir.join("../lib/micold-ai-ide/remote"),
        _ => dir.join("remote"),
    }
}

/// Makes key pairs with the machine's `ssh-keygen` (feature 686, the host key and this client's
/// key). `KeyGen` cannot fail, so a failed run returns empty lines, which the start refuses.
pub struct SshKeygen;

impl micold_core::kube::access::KeyGen for SshKeygen {
    fn generate(&self, comment: &str) -> micold_core::kube::access::KeyPair {
        make_key_pair(comment).unwrap_or(micold_core::kube::access::KeyPair {
            private_openssh: String::new(),
            public_line: String::new(),
        })
    }
}

fn make_key_pair(comment: &str) -> std::io::Result<micold_core::kube::access::KeyPair> {
    let dir = kube_keys_root().join(format!(".keygen-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let path = dir.join("key");
    let made = (|| {
        let argv = micold_core::provision::keys::keygen_argv(
            micold_core::ssh::ClientPlatform::current(),
            &path,
        );
        let mut command = std::process::Command::new(&argv.program);
        command
            .args(&argv.args)
            .args(["-C", comment])
            .envs(argv.env.iter().map(|(k, v)| (k, v)));
        let status = micold_core::process::no_window(&mut command).status()?;
        if !status.success() {
            return Err(std::io::Error::other("ssh-keygen failed"));
        }
        Ok(micold_core::kube::access::KeyPair {
            private_openssh: std::fs::read_to_string(&path)?,
            public_line: std::fs::read_to_string(path.with_extension("pub"))?
                .trim()
                .to_string(),
        })
    })();
    let _ = std::fs::remove_dir_all(&dir);
    made
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_builds_are_found_where_each_package_puts_them() {
        for (exe, want) in [
            (
                "/Applications/micold-ai-ide.app/Contents/MacOS/micold-ai-ide",
                "/Applications/micold-ai-ide.app/Contents/MacOS/../Resources/remote",
            ),
            (
                "/usr/bin/micold-ai-ide",
                "/usr/bin/../lib/micold-ai-ide/remote",
            ),
            ("/opt/app/micold-ai-ide.exe", "/opt/app/remote"),
        ] {
            assert_eq!(bundled_dir(Path::new(exe)), PathBuf::from(want), "{exe}");
        }
    }

    #[test]
    fn windows_ssh_is_taken_from_path_before_the_system_openssh() {
        let tools = PathBuf::from("tools");
        let root = PathBuf::from("win");
        let path_var = std::env::join_paths([&tools])
            .unwrap()
            .into_string()
            .unwrap();
        let on_path = tools.join("ssh.exe");
        let system = root.join("System32").join("OpenSSH").join("ssh.exe");

        let both = |p: &Path| p == on_path || p == system;
        assert_eq!(
            locate_windows_ssh(&path_var, Some(&root), &both),
            Some(on_path.clone())
        );
        let system_only = |p: &Path| p == system;
        assert_eq!(
            locate_windows_ssh(&path_var, Some(&root), &system_only),
            Some(system.clone())
        );
        assert_eq!(locate_windows_ssh(&path_var, None, &system_only), None);
    }

    /// T052 (FR-013, FR-020): launch only reads the registry. Tracking an auto-start remote and
    /// building its connection subscription are plain calls with no runtime to await on, so the
    /// window never waits for the remote's dial; the dial belongs to its actor.
    #[test]
    fn launch_tracks_and_subscribes_a_remote_without_awaiting_its_dial() {
        use micold_core::daemons::{DaemonRegistry, DaemonRuntime, DaemonState};
        use micold_core::ssh::{SshSettings, SshTarget};
        let mut registry = DaemonRegistry::default();
        let local = registry.add("Local", DaemonRuntime::Host, true).unwrap();
        let target = SshTarget::new("unreachable.invalid", None, None, None).unwrap();
        let remote = registry
            .add("Remote", DaemonRuntime::Ssh(SshSettings { target }), true)
            .unwrap();
        let links = micold_client::links::DaemonLinks::at_launch(&registry);
        assert_eq!(links.state(local), Some(&DaemonState::Starting));
        assert_eq!(links.state(remote), Some(&DaemonState::Starting));
        let _subscription = micold_client::daemon::connections(
            &registry,
            links.states(),
            Path::new("/nonexistent"),
            None,
            false,
        );
    }
}
