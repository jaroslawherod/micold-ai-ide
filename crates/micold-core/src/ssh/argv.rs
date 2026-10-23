//! The command lines of every `ssh` the client runs (contracts/ssh-transport.md). Pure builders.

use std::path::PathBuf;

use super::target::SshTarget;

/// The client's platform, as data (like `pathmap::map_for` takes `windows_host`), so one CI
/// platform exercises all three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientPlatform {
    /// A Linux client.
    Linux,
    /// A macOS client.
    MacOs,
    /// A Windows client.
    Windows,
}

impl ClientPlatform {
    /// The platform this build runs on.
    pub fn current() -> Self {
        if cfg!(windows) {
            ClientPlatform::Windows
        } else if cfg!(target_os = "macos") {
            ClientPlatform::MacOs
        } else {
            ClientPlatform::Linux
        }
    }

    /// The `os` string a remote probe prints for this platform's own OS.
    pub fn os(self) -> &'static str {
        match self {
            ClientPlatform::Linux => "linux",
            ClientPlatform::MacOs => "macos",
            ClientPlatform::Windows => "windows",
        }
    }
}

/// One process to run: program, arguments and environment overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshCommand {
    /// The `ssh` executable name (`ssh.exe` on Windows); a runner may resolve a full path.
    pub program: String,
    /// The arguments, in order.
    pub args: Vec<String>,
    /// Environment variables set for the child.
    pub env: Vec<(String, String)>,
}

/// How a command reaches its SSH server (contracts/provisioning.md, *What a runtime supplies*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
    /// A host the user owns, reached through their own SSH configuration (feature 489).
    UserConfig(SshTarget),
    /// An app-owned target (a container, a pod) reached on loopback with the app's own key and
    /// `known_hosts`, never the user's configuration, keys or agent.
    Pinned(PinnedReach),
}

/// Everything a pinned reach needs; nothing is read from the user's SSH setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedReach {
    /// The host, always loopback (`127.0.0.1`).
    pub host: String,
    /// The port the target's SSH server is published on.
    pub port: u16,
    /// The user name to log in as.
    pub user: String,
    /// The app's private key for this target.
    pub key: PathBuf,
    /// The app's own `known_hosts` for this target.
    pub known_hosts: PathBuf,
}

/// The loopback port an install command's relay is forwarded to inside the target.
pub const RELAY_REMOTE_PORT: u16 = 17728;

/// The seconds a connect may take when the user's SSH configuration sets no `ConnectTimeout`
/// (FR-009).
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u32 = 15;

/// `<PACKAGE_VERSION>-<first 12 hex of the build fingerprint>`: the per-build directory name on the
/// remote host. Built from this build's own constants, never from user input.
pub fn build_id(package_version: &str, fingerprint: &str) -> String {
    let short: String = fingerprint.chars().take(12).collect();
    format!("{package_version}-{short}")
}

/// This build's [`build_id`].
pub fn current_build_id() -> String {
    build_id(
        crate::protocol::version::PACKAGE_VERSION,
        crate::protocol::version::BUILD_FINGERPRINT,
    )
}

/// The directory of a build on the remote host, as the remote shell expands it.
pub fn remote_dir(build: &str) -> String {
    format!("$HOME/.micold/remote/{build}")
}

fn program(platform: ClientPlatform) -> String {
    match platform {
        ClientPlatform::Windows => "ssh.exe".to_string(),
        _ => "ssh".to_string(),
    }
}

/// The common prefix, in order, then `--`, the host and the remote command. `relay` is the local
/// port of the install relay, forwarded to the target's loopback; only [`install`] passes one, and
/// only an install command lacks `ClearAllForwardings`.
fn command(
    reach: &Reach,
    platform: ClientPlatform,
    connect_timeout_secs: u32,
    remote: &str,
    forward: Option<Forward>,
) -> SshCommand {
    let mut args: Vec<String> = Vec::new();
    if matches!(reach, Reach::Pinned(_)) {
        args.extend(["-F".into(), "none".into()]);
    }
    args.extend(
        [
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    // `UpdateHostKeys=no`: never write to known_hosts on the user's behalf (FR-010). The server
    // alive options bound a network that dies mid-copy or mid-session (FR-009).
    let mut options = vec![
        "UpdateHostKeys=no",
        "ServerAliveInterval=15",
        "ServerAliveCountMax=3",
    ];
    if forward.is_none() {
        options.insert(0, "ClearAllForwardings=yes");
    }
    for option in options {
        args.extend(["-o".into(), option.into()]);
    }
    args.extend([
        "-o".into(),
        format!("ConnectTimeout={connect_timeout_secs}"),
    ]);
    let host = match reach {
        Reach::UserConfig(target) => {
            if let Some(port) = target.port {
                args.extend(["-p".into(), port.to_string()]);
            }
            if let Some(user) = &target.user {
                args.extend(["-l".into(), user.clone()]);
            }
            if let Some(key) = &target.key_file {
                args.extend(["-i".into(), key.display().to_string()]);
            }
            target.host.clone()
        }
        Reach::Pinned(pinned) => {
            for option in [
                "IdentitiesOnly=yes".to_string(),
                "IdentityAgent=none".to_string(),
                format!("UserKnownHostsFile={}", pinned.known_hosts.display()),
                "GlobalKnownHostsFile=none".to_string(),
            ] {
                args.extend(["-o".into(), option]);
            }
            args.extend(["-i".into(), pinned.key.display().to_string()]);
            args.extend(["-l".into(), pinned.user.clone()]);
            args.extend(["-p".into(), pinned.port.to_string()]);
            pinned.host.clone()
        }
    };
    if let Some(Forward { relay_port }) = forward {
        // A forward that cannot bind must end the command: the proxy address is then not ours.
        args.extend(["-o".into(), "ExitOnForwardFailure=yes".into()]);
        args.extend([
            "-R".into(),
            format!("127.0.0.1:{RELAY_REMOTE_PORT}:127.0.0.1:{relay_port}"),
        ]);
    }
    args.push("--".into());
    args.push(host);
    args.push(remote.to_string());
    SshCommand {
        program: program(platform),
        args,
        env: vec![("LC_ALL".into(), "C".into())],
    }
}

/// The one remote forward a command may carry: the install relay.
#[derive(Clone, Copy)]
struct Forward {
    relay_port: u16,
}

/// `ssh -G`: the effective settings the user's configuration gives the target, no connection made.
/// Entered fields go first so they override the file, as they will when connecting.
pub fn resolve(target: &SshTarget, platform: ClientPlatform) -> SshCommand {
    let mut args = vec!["-G".to_string()];
    if let Some(port) = target.port {
        args.extend(["-p".into(), port.to_string()]);
    }
    if let Some(user) = &target.user {
        args.extend(["-l".into(), user.clone()]);
    }
    if let Some(key) = &target.key_file {
        args.extend(["-i".into(), key.display().to_string()]);
    }
    args.extend(["--".into(), target.host.clone()]);
    SshCommand {
        program: program(platform),
        args,
        env: vec![("LC_ALL".into(), "C".into())],
    }
}

/// The probe: the fixed script goes on stdin of `sh -s`, so the user's login shell never parses it.
pub fn probe(reach: &Reach, platform: ClientPlatform, timeout: u32) -> SshCommand {
    command(reach, platform, timeout, "sh -s", None)
}

/// The reach check: runs `true`, so a pinned target's SSH server is proven up before the probe.
pub fn reach_check(reach: &Reach, platform: ClientPlatform, timeout: u32) -> SshCommand {
    command(reach, platform, timeout, "true", None)
}

/// The install: the fixed install script goes on stdin of `sh -s`. The only command that may carry
/// a forward: `-R` to the install relay, when `relay_port` is given.
pub fn install(
    reach: &Reach,
    platform: ClientPlatform,
    timeout: u32,
    relay_port: Option<u16>,
) -> SshCommand {
    command(
        reach,
        platform,
        timeout,
        "sh -s",
        relay_port.map(|relay_port| Forward { relay_port }),
    )
}

/// Records which build the target should start unattended (`$HOME/.micold/remote/last`), for the
/// image's entrypoint. App-owned targets only. The build id is this build's own constant.
pub fn record_last(
    reach: &Reach,
    platform: ClientPlatform,
    timeout: u32,
    build: &str,
) -> SshCommand {
    let remote = format!(
        "sh -c 'mkdir -p \"$HOME/.micold/remote\" && printf %s \"{build}\" > \"$HOME/.micold/remote/last\"'"
    );
    command(reach, platform, timeout, &remote, None)
}

/// The probe script itself, a fixed string with only the build directory name substituted.
pub fn probe_script(build: &str) -> String {
    let dir = remote_dir(build);
    format!(
        "os=$(uname -s 2>/dev/null); arch=$(uname -m 2>/dev/null)\n\
         mkdir -p \"$HOME/.micold/remote\" 2>/dev/null && test -w \"$HOME/.micold/remote\" && w=1 || w=0\n\
         test -x \"{dir}/micold-daemon\" && h=1 || h=0\n\
         echo \"MICOLD_PROBE os=$os arch=$arch home_writable=$w have=$h dir={dir}\"\n"
    )
}

/// The copy: the artefact bytes arrive on stdin and land atomically in the build's directory.
/// The partial file is the copying shell's own (`.part.$$`): two clients copying the same build
/// at once (686 FR-023) each rename a whole file, and neither moves the other's away.
pub fn copy(reach: &Reach, platform: ClientPlatform, timeout: u32, build: &str) -> SshCommand {
    let dir = remote_dir(build);
    let remote = format!(
        "sh -c 'umask 077; p=\"{dir}/.part.$$\"; mkdir -p \"{dir}\" && cat > \"$p\" && chmod 700 \"$p\" && mv \"$p\" \"{dir}/micold-daemon\" || {{ rm -f \"$p\"; exit 1; }}'"
    );
    command(reach, platform, timeout, &remote, None)
}

/// The state directory a container's daemon keeps its data in: the image's
/// `ENV MICOLD_DATA_DIR`, which an SSH session does not inherit (sshd builds the session's
/// environment afresh), so it rides the bridge command. Never `XDG_DATA_HOME` (1286): that would
/// reach every session the daemon starts.
pub const CONTAINER_DATA_DIR: &str = "/var/lib/micold-ai-ide";

/// The variable that carries [`CONTAINER_DATA_DIR`] to the daemon.
pub const CONTAINER_DATA_ENV: &str = "MICOLD_DATA_DIR";

/// The helper's `status`: prints the remote daemon's `RemoteStatus` as one JSON line.
pub fn status(reach: &Reach, platform: ClientPlatform, timeout: u32, build: &str) -> SshCommand {
    helper(reach, platform, timeout, build, "status")
}

/// The helper's `stop`: ends the remote daemon only when its status proves it has no sessions.
pub fn stop(reach: &Reach, platform: ClientPlatform, timeout: u32, build: &str) -> SshCommand {
    helper(reach, platform, timeout, build, "stop")
}

fn helper(
    reach: &Reach,
    platform: ClientPlatform,
    timeout: u32,
    build: &str,
    sub: &str,
) -> SshCommand {
    let remote = format!("\"{}/micold-daemon\" remote {sub}", remote_dir(build));
    command(reach, platform, timeout, &remote, None)
}

/// The bridge: the remote daemon binary's relay, whose stdin/stdout carry the protocol.
pub fn bridge(reach: &Reach, platform: ClientPlatform, timeout: u32, build: &str) -> SshCommand {
    bridge_with_env(reach, platform, timeout, build, &[])
}

/// [`bridge`] with the daemon's environment: sshd clears the login environment, so what the
/// daemon needs (686 FR-010: `MICOLD_IDLE_STOP=off`) rides the remote command as `env K='v'`.
pub fn bridge_with_env(
    reach: &Reach,
    platform: ClientPlatform,
    timeout: u32,
    build: &str,
    env: &[(String, String)],
) -> SshCommand {
    let mut remote = String::new();
    if !env.is_empty() {
        remote.push_str("env");
        for (key, value) in env {
            remote.push_str(&format!(" {key}='{}'", value.replace('\'', "'\\''")));
        }
        remote.push(' ');
    }
    remote.push_str(&format!(
        "\"{}/micold-daemon\" remote bridge",
        remote_dir(build)
    ));
    command(reach, platform, timeout, &remote, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn target() -> Reach {
        Reach::UserConfig(SshTarget {
            host: "build".into(),
            user: Some("me".into()),
            port: Some(2222),
            key_file: Some(PathBuf::from("/home/me/.ssh/id")),
        })
    }

    fn pinned() -> Reach {
        Reach::Pinned(PinnedReach {
            host: "127.0.0.1".into(),
            port: 40222,
            user: "dev".into(),
            key: PathBuf::from("/state/ssh/client_ed25519"),
            known_hosts: PathBuf::from("/state/ssh/known_hosts"),
        })
    }

    fn all_commands(platform: ClientPlatform) -> Vec<SshCommand> {
        let t = target();
        vec![
            probe(&t, platform, 15),
            copy(&t, platform, 15, "1.0-abc"),
            bridge(&t, platform, 15, "1.0-abc"),
            reach_check(&t, platform, 15),
            record_last(&t, platform, 15, "1.0-abc"),
        ]
    }

    #[test]
    fn every_command_forces_batch_strict_host_keys_and_no_forwarding() {
        for platform in [
            ClientPlatform::Linux,
            ClientPlatform::MacOs,
            ClientPlatform::Windows,
        ] {
            for cmd in all_commands(platform) {
                let joined = cmd.args.join(" ");
                for needle in [
                    "BatchMode=yes",
                    "StrictHostKeyChecking=yes",
                    "ClearAllForwardings=yes",
                    "UpdateHostKeys=no",
                    "ServerAliveInterval=15",
                    "ConnectTimeout=15",
                ] {
                    assert!(joined.contains(needle), "{needle} missing: {joined}");
                }
                assert!(cmd.args.contains(&"-T".to_string()), "no tty: {joined}");
            }
        }
    }

    #[test]
    fn the_host_follows_a_double_dash_so_it_can_never_be_an_option() {
        for cmd in all_commands(ClientPlatform::Linux) {
            let at = cmd.args.iter().position(|a| a == "--").expect("a `--`");
            assert_eq!(cmd.args[at + 1], "build", "host directly after --");
        }
    }

    #[test]
    fn the_locale_is_pinned_so_stderr_patterns_are_stable() {
        for cmd in all_commands(ClientPlatform::Linux) {
            assert!(cmd.env.contains(&("LC_ALL".into(), "C".into())));
        }
    }

    #[test]
    fn no_command_forwards_a_port_reads_a_config_or_loosens_host_key_checking() {
        for cmd in all_commands(ClientPlatform::Linux) {
            for forbidden in ["-L", "-R", "-D", "-W", "-N", "-f", "-F", "-v"] {
                assert!(
                    !cmd.args.iter().any(|a| a == forbidden),
                    "{forbidden} must not appear: {:?}",
                    cmd.args
                );
            }
            let joined = cmd.args.join(" ");
            for forbidden in [
                "accept-new",
                "StrictHostKeyChecking=no",
                "UserKnownHostsFile",
                "PasswordAuthentication",
            ] {
                assert!(!joined.contains(forbidden), "{forbidden} in {joined}");
            }
        }
    }

    #[test]
    fn the_daemons_environment_rides_the_remote_command_quoted() {
        let env = [
            ("MICOLD_IDLE_STOP".to_string(), "off".to_string()),
            ("A".to_string(), "it's".to_string()),
        ];
        let cmd = bridge_with_env(&pinned(), ClientPlatform::Linux, 15, "b", &env);
        let remote = cmd.args.last().unwrap();
        assert_eq!(
            remote,
            "env MICOLD_IDLE_STOP='off' A='it'\\''s' \"$HOME/.micold/remote/b/micold-daemon\" remote bridge"
        );
        assert_eq!(
            bridge_with_env(&pinned(), ClientPlatform::Linux, 15, "b", &[]),
            bridge(&pinned(), ClientPlatform::Linux, 15, "b")
        );
    }

    #[test]
    fn a_key_path_may_appear_but_no_secret_does() {
        let joined = bridge(&target(), ClientPlatform::Linux, 15, "b")
            .args
            .join(" ");
        assert!(joined.contains("-i /home/me/.ssh/id"));
        assert!(!joined.to_lowercase().contains("password"));
    }

    #[test]
    fn user_and_port_are_passed_only_when_entered() {
        let bare = Reach::UserConfig(SshTarget {
            host: "h".into(),
            user: None,
            port: None,
            key_file: None,
        });
        let joined = probe(&bare, ClientPlatform::Linux, 15).args.join(" ");
        for flag in ["-p ", "-l ", "-i "] {
            assert!(!joined.contains(flag), "{flag} in {joined}");
        }
        let joined = probe(&target(), ClientPlatform::Linux, 15).args.join(" ");
        assert!(joined.contains("-p 2222") && joined.contains("-l me"));
    }

    #[test]
    fn the_program_is_ssh_exe_on_windows_and_ssh_elsewhere() {
        assert_eq!(
            probe(&target(), ClientPlatform::Windows, 15).program,
            "ssh.exe"
        );
        assert_eq!(probe(&target(), ClientPlatform::Linux, 15).program, "ssh");
        assert_eq!(probe(&target(), ClientPlatform::MacOs, 15).program, "ssh");
    }

    #[test]
    fn the_build_directory_name_is_version_and_twelve_hex_of_the_fingerprint() {
        assert_eq!(
            build_id("1.2.3", "0123456789abcdef0123"),
            "1.2.3-0123456789ab"
        );
    }

    #[test]
    fn the_bridge_runs_the_helper_from_the_build_directory() {
        let cmd = bridge(&target(), ClientPlatform::Linux, 15, "1.0-abc");
        let last = cmd.args.last().unwrap();
        assert!(
            last.ends_with("micold-daemon\" remote bridge")
                && last.contains(".micold/remote/1.0-abc/"),
            "{last}"
        );
    }

    #[test]
    fn the_copy_has_no_text_the_user_controls_beyond_the_build_directory() {
        let cmd = copy(&target(), ClientPlatform::Linux, 15, "1.0-abc");
        let last = cmd.args.last().unwrap();
        assert!(last.contains("umask 077") && last.contains(".part") && last.contains("mv "));
        assert!(
            !last.contains("build"),
            "host name must not reach the script: {last}"
        );
    }

    #[test]
    fn a_user_config_reach_is_byte_for_byte_todays_argv() {
        let cmd = probe(&target(), ClientPlatform::Linux, 15);
        let want: Vec<&str> = vec![
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ClearAllForwardings=yes",
            "-o",
            "UpdateHostKeys=no",
            "-o",
            "ServerAliveInterval=15",
            "-o",
            "ServerAliveCountMax=3",
            "-o",
            "ConnectTimeout=15",
            "-p",
            "2222",
            "-l",
            "me",
            "-i",
            "/home/me/.ssh/id",
            "--",
            "build",
            "sh -s",
        ];
        assert_eq!(cmd.args, want);
    }

    #[test]
    fn a_pinned_reach_reads_no_user_configuration_key_or_agent() {
        for cmd in [
            probe(&pinned(), ClientPlatform::Linux, 15),
            reach_check(&pinned(), ClientPlatform::Linux, 15),
            copy(&pinned(), ClientPlatform::Linux, 15, "b"),
            bridge(&pinned(), ClientPlatform::Linux, 15, "b"),
            record_last(&pinned(), ClientPlatform::Linux, 15, "b"),
            install(&pinned(), ClientPlatform::Linux, 15, None),
        ] {
            let a = &cmd.args;
            let pair =
                |flag: &str, value: &str| a.windows(2).any(|w| w[0] == flag && w[1] == value);
            assert_eq!(&a[..2], ["-F", "none"], "{a:?}");
            for option in [
                "IdentitiesOnly=yes",
                "IdentityAgent=none",
                "UserKnownHostsFile=/state/ssh/known_hosts",
                "GlobalKnownHostsFile=none",
                "StrictHostKeyChecking=yes",
            ] {
                assert!(pair("-o", option), "{option} missing: {a:?}");
            }
            assert!(pair("-i", "/state/ssh/client_ed25519"), "{a:?}");
            assert!(pair("-l", "dev") && pair("-p", "40222"), "{a:?}");
            let at = a.iter().position(|x| x == "--").unwrap();
            assert_eq!(a[at + 1], "127.0.0.1");
        }
    }

    #[test]
    fn the_reach_check_runs_true() {
        for reach in [target(), pinned()] {
            let cmd = reach_check(&reach, ClientPlatform::Linux, 15);
            assert_eq!(cmd.args.last().map(String::as_str), Some("true"));
        }
    }

    #[test]
    fn install_is_the_only_command_without_clear_all_forwardings() {
        let with = install(&pinned(), ClientPlatform::Linux, 15, Some(5555));
        let joined = with.args.join(" ");
        assert!(!joined.contains("ClearAllForwardings"), "{joined}");
        assert!(
            joined.contains("-R 127.0.0.1:17728:127.0.0.1:5555"),
            "{joined}"
        );
        assert_eq!(with.args.last().map(String::as_str), Some("sh -s"));
        let without = install(&pinned(), ClientPlatform::Linux, 15, None);
        assert!(!without.args.iter().any(|a| a == "-R"));
        for platform in [ClientPlatform::Linux, ClientPlatform::Windows] {
            for reach in [target(), pinned()] {
                for cmd in [
                    probe(&reach, platform, 15),
                    copy(&reach, platform, 15, "b"),
                    bridge(&reach, platform, 15, "b"),
                    reach_check(&reach, platform, 15),
                    record_last(&reach, platform, 15, "b"),
                ] {
                    assert!(cmd.args.contains(&"ClearAllForwardings=yes".to_string()));
                }
            }
        }
    }

    #[test]
    fn a_pinned_install_is_platform_parametrised() {
        assert_eq!(
            install(&pinned(), ClientPlatform::Windows, 15, None).program,
            "ssh.exe"
        );
        assert_eq!(
            install(&pinned(), ClientPlatform::MacOs, 15, None).program,
            "ssh"
        );
    }

    #[test]
    fn resolve_asks_ssh_for_its_effective_config_with_the_host_after_the_separator() {
        let t = SshTarget::new("box", Some("me"), Some(2200), None).unwrap();
        let cmd = resolve(&t, ClientPlatform::Linux);
        assert_eq!(cmd.args.first().map(String::as_str), Some("-G"));
        let sep = cmd.args.iter().position(|a| a == "--").unwrap();
        assert_eq!(
            cmd.args[sep + 1..],
            ["box"],
            "the host is the last argument"
        );
        assert!(
            cmd.args.contains(&"-p".to_string()) && cmd.args.contains(&"me".to_string()),
            "entered fields override the file: {:?}",
            cmd.args
        );
    }
}
