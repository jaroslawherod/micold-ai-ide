//! A rootless `sshd` for the `ssh_real_*` suite.
//!
//! The client's argv builders forbid `-F`, and OpenSSH reads `~/.ssh` from the account database,
//! not from `$HOME`, so the test cannot steer a plain `ssh` at its own configuration. Instead the
//! harness hands the runner a wrapper script (`SystemSsh.ssh_path`) that is `ssh -F <private
//! config>`; the config holds the `Host` alias, the port, the identity and the known-hosts file.
//!
//! The remote side sees a private `$HOME` (`SetEnv` in the server configuration), so the copied
//! build and the daemon it starts keep their files in the harness directory, not in the real home.

use std::fs;
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::proxy::Proxy;

/// The `Host` alias in the private `ssh` configuration.
pub const ALIAS: &str = "micold-test";

/// A running `sshd` and the files a client needs to reach it.
pub struct Sshd {
    child: Child,
    /// The harness directory, removed on drop.
    pub dir: PathBuf,
    /// The `$HOME` the remote sessions see.
    pub remote_home: PathBuf,
    /// The loopback port the client dials: the fault proxy in front of `sshd`.
    pub port: u16,
    /// Cut and restore the network between the client's `ssh` and `sshd`.
    pub proxy: Proxy,
    /// A script that is `ssh -F <private config> "$@"`.
    pub ssh_wrapper: PathBuf,
}

fn find(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .chain(["/usr/sbin", "/usr/local/sbin", "/usr/lib/openssh"].map(PathBuf::from))
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

fn run(cmd: &mut Command, what: &str) -> Result<(), String> {
    match cmd.output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format!(
            "{what} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("{what} could not run: {e}")),
    }
}

fn keygen(keygen: &Path, file: &Path) -> Result<(), String> {
    run(
        Command::new(keygen)
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(file),
        "ssh-keygen",
    )
}

impl Sshd {
    /// Start `sshd` on a free loopback port, or say why not.
    pub fn start() -> Result<Self, String> {
        let sshd = find("sshd").ok_or("`sshd` is not installed")?;
        let ssh = find("ssh").ok_or("`ssh` is not installed")?;
        let ssh_keygen = find("ssh-keygen").ok_or("`ssh-keygen` is not installed")?;

        let dir = std::env::temp_dir().join(format!("micold-sshd-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let remote_home = dir.join("home");
        fs::create_dir_all(&remote_home).map_err(|e| e.to_string())?;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;

        let host_key = dir.join("host_key");
        let client_key = dir.join("client_key");
        keygen(&ssh_keygen, &host_key)?;
        keygen(&ssh_keygen, &client_key)?;
        let authorized = dir.join("authorized_keys");
        fs::copy(dir.join("client_key.pub"), &authorized).map_err(|e| e.to_string())?;
        fs::set_permissions(&authorized, fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;

        let sshd_port = TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map_err(|e| e.to_string())?
            .port();
        let proxy = Proxy::start(sshd_port)?;
        let port = proxy.port;

        let config = dir.join("sshd_config");
        fs::write(
            &config,
            format!(
                "Port {sshd_port}\nListenAddress 127.0.0.1\nHostKey {}\nPidFile {}\n\
                 AuthorizedKeysFile {}\nStrictModes no\nUsePAM no\nPasswordAuthentication no\n\
                 KbdInteractiveAuthentication no\nPubkeyAuthentication yes\nLogLevel ERROR\n\
                 SetEnv HOME={} PATH={}/bin:/usr/local/bin:/usr/bin:/bin\n",
                host_key.display(),
                dir.join("sshd.pid").display(),
                authorized.display(),
                remote_home.display(),
                remote_home.display(),
            ),
        )
        .map_err(|e| e.to_string())?;

        let host_pub = fs::read_to_string(dir.join("host_key.pub")).map_err(|e| e.to_string())?;
        let mut parts = host_pub.split_whitespace();
        let (kind, key) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        let known_hosts = dir.join("known_hosts");
        fs::write(&known_hosts, format!("[127.0.0.1]:{port} {kind} {key}\n"))
            .map_err(|e| e.to_string())?;

        let ssh_config = dir.join("ssh_config");
        fs::write(
            &ssh_config,
            format!(
                "Host {ALIAS}\n  HostName 127.0.0.1\n  Port {port}\n  IdentityFile {}\n  \
                 IdentitiesOnly yes\n  IdentityAgent none\n  UserKnownHostsFile {}\n  \
                 GlobalKnownHostsFile /dev/null\n",
                client_key.display(),
                known_hosts.display(),
            ),
        )
        .map_err(|e| e.to_string())?;

        let ssh_wrapper = dir.join("ssh-wrapper");
        fs::write(
            &ssh_wrapper,
            format!(
                "#!/bin/sh\nexec \"{}\" -F \"{}\" \"$@\"\n",
                ssh.display(),
                ssh_config.display()
            ),
        )
        .map_err(|e| e.to_string())?;
        fs::set_permissions(&ssh_wrapper, fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;

        let log = fs::File::create(dir.join("sshd.log")).map_err(|e| e.to_string())?;
        let child = Command::new(&sshd)
            .args(["-D", "-e", "-f"])
            .arg(&config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .map_err(|e| format!("sshd could not start: {e}"))?;
        let mut host = Self {
            child,
            dir,
            remote_home,
            port,
            proxy,
            ssh_wrapper,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while TcpStream::connect(("127.0.0.1", sshd_port)).is_err() {
            if let Ok(Some(status)) = host.child.try_wait() {
                let log = fs::read_to_string(host.dir.join("sshd.log")).unwrap_or_default();
                return Err(format!("sshd exited ({status}): {}", log.trim()));
            }
            if Instant::now() > deadline {
                return Err("sshd did not listen within 10 s".to_string());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        // A rootless sshd can only log in as its own user; prove the whole path before a test
        // spends its time on it.
        run(
            Command::new(&host.ssh_wrapper).args([
                "-T",
                "-o",
                "BatchMode=yes",
                "-o",
                "StrictHostKeyChecking=yes",
                ALIAS,
                "true",
            ]),
            "a login to the stand-in host",
        )?;
        Ok(host)
    }
}

impl Drop for Sshd {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.dir);
    }
}
