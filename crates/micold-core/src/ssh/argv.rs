//! The command lines of every `ssh` the client runs (contracts/ssh-transport.md). Pure builders.

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

/// The common prefix, in order, then `--`, the host and the remote command.
fn command(
    target: &SshTarget,
    platform: ClientPlatform,
    connect_timeout_secs: u32,
    remote: &str,
) -> SshCommand {
    let mut args: Vec<String> = ["-T", "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=yes"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    // `UpdateHostKeys=no`: never write to known_hosts on the user's behalf (FR-010). The server
    // alive options bound a network that dies mid-copy or mid-session (FR-009).
    for option in [
        "ClearAllForwardings=yes",
        "UpdateHostKeys=no",
        "ServerAliveInterval=15",
        "ServerAliveCountMax=3",
    ] {
        args.extend(["-o".into(), option.into()]);
    }
    args.extend([
        "-o".into(),
        format!("ConnectTimeout={connect_timeout_secs}"),
    ]);
    if let Some(port) = target.port {
        args.extend(["-p".into(), port.to_string()]);
    }
    if let Some(user) = &target.user {
        args.extend(["-l".into(), user.clone()]);
    }
    if let Some(key) = &target.key_file {
        args.extend(["-i".into(), key.display().to_string()]);
    }
    args.push("--".into());
    args.push(target.host.clone());
    args.push(remote.to_string());
    SshCommand {
        program: program(platform),
        args,
        env: vec![("LC_ALL".into(), "C".into())],
    }
}

/// The probe: the fixed script goes on stdin of `sh -s`, so the user's login shell never parses it.
pub fn probe(target: &SshTarget, platform: ClientPlatform, timeout: u32) -> SshCommand {
    command(target, platform, timeout, "sh -s")
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
pub fn copy(target: &SshTarget, platform: ClientPlatform, timeout: u32, build: &str) -> SshCommand {
    let dir = remote_dir(build);
    let remote = format!(
        "sh -c 'umask 077; mkdir -p \"{dir}\" && cat > \"{dir}/.part\" && chmod 700 \"{dir}/.part\" && mv \"{dir}/.part\" \"{dir}/micold-daemon\"'"
    );
    command(target, platform, timeout, &remote)
}

/// The bridge: the remote daemon binary's relay, whose stdin/stdout carry the protocol.
pub fn bridge(target: &SshTarget, platform: ClientPlatform, timeout: u32, build: &str) -> SshCommand {
    let remote = format!("\"{}/micold-daemon\" remote bridge", remote_dir(build));
    command(target, platform, timeout, &remote)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn target() -> SshTarget {
        SshTarget {
            host: "build".into(),
            user: Some("me".into()),
            port: Some(2222),
            key_file: Some(PathBuf::from("/home/me/.ssh/id")),
        }
    }

    fn all_commands(platform: ClientPlatform) -> Vec<SshCommand> {
        let t = target();
        vec![
            probe(&t, platform, 15),
            copy(&t, platform, 15, "1.0-abc"),
            bridge(&t, platform, 15, "1.0-abc"),
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
            for forbidden in ["accept-new", "StrictHostKeyChecking=no", "UserKnownHostsFile", "PasswordAuthentication"] {
                assert!(!joined.contains(forbidden), "{forbidden} in {joined}");
            }
        }
    }

    #[test]
    fn a_key_path_may_appear_but_no_secret_does() {
        let joined = bridge(&target(), ClientPlatform::Linux, 15, "b").args.join(" ");
        assert!(joined.contains("-i /home/me/.ssh/id"));
        assert!(!joined.to_lowercase().contains("password"));
    }

    #[test]
    fn user_and_port_are_passed_only_when_entered() {
        let bare = SshTarget {
            host: "h".into(),
            user: None,
            port: None,
            key_file: None,
        };
        let joined = probe(&bare, ClientPlatform::Linux, 15).args.join(" ");
        for flag in ["-p ", "-l ", "-i "] {
            assert!(!joined.contains(flag), "{flag} in {joined}");
        }
        let joined = probe(&target(), ClientPlatform::Linux, 15).args.join(" ");
        assert!(joined.contains("-p 2222") && joined.contains("-l me"));
    }

    #[test]
    fn the_program_is_ssh_exe_on_windows_and_ssh_elsewhere() {
        assert_eq!(probe(&target(), ClientPlatform::Windows, 15).program, "ssh.exe");
        assert_eq!(probe(&target(), ClientPlatform::Linux, 15).program, "ssh");
        assert_eq!(probe(&target(), ClientPlatform::MacOs, 15).program, "ssh");
    }

    #[test]
    fn the_build_directory_name_is_version_and_twelve_hex_of_the_fingerprint() {
        assert_eq!(build_id("1.2.3", "0123456789abcdef0123"), "1.2.3-0123456789ab");
    }

    #[test]
    fn the_bridge_runs_the_helper_from_the_build_directory() {
        let cmd = bridge(&target(), ClientPlatform::Linux, 15, "1.0-abc");
        let last = cmd.args.last().unwrap();
        assert!(
            last.ends_with("micold-daemon\" remote bridge") && last.contains(".micold/remote/1.0-abc/"),
            "{last}"
        );
    }

    #[test]
    fn the_copy_has_no_text_the_user_controls_beyond_the_build_directory() {
        let cmd = copy(&target(), ClientPlatform::Linux, 15, "1.0-abc");
        let last = cmd.args.last().unwrap();
        assert!(last.contains("umask 077") && last.contains(".part") && last.contains("mv "));
        assert!(!last.contains("build"), "host name must not reach the script: {last}");
    }
}
