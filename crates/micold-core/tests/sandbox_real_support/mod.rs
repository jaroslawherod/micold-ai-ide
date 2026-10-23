//! Shared by the core crate's `sandbox_real_*` targets: the basic image, the daemon's SSH keys made
//! the way the client makes them, and a wait for the container's sshd.
//!
//! Each target is its own test binary and includes this by `mod`, so `dead_code` is allowed.
#![allow(dead_code)]

use std::io::Read;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use micold_core::provision::keys::DaemonKeys;
use micold_core::sandbox::basic_image::DEV_BASIC_IMAGE;
use micold_core::sandbox::dialect::Dialect;
use micold_core::sandbox::runtime::RuntimeKind;
use micold_core::sandbox::SecretMount;
use micold_core::ssh::{ClientPlatform, SystemSsh};

/// Which runtime to drive, from `MICOLD_TEST_RUNTIME`; Docker unless it says otherwise.
///
/// The questions these targets ask — does the runtime enforce what it claims to, does the posture
/// hold — have a different answer per runtime, and only one of them was ever measured until a
/// target asked the other. Selecting the kind here, and taking the program name from the dialect
/// rather than spelling it, is what lets T098 and T240 ask them of podman.
pub fn runtime_kind() -> RuntimeKind {
    match std::env::var("MICOLD_TEST_RUNTIME").as_deref() {
        Err(_) | Ok("") | Ok("docker") => RuntimeKind::Docker,
        Ok("podman") => RuntimeKind::Podman,
        // Loudly, rather than falling back to Docker: a typo that silently ran the Docker suite
        // again would report a podman pass that never happened.
        Ok(other) => panic!("MICOLD_TEST_RUNTIME={other:?}: expected `docker` or `podman`"),
    }
}

/// The selected runtime's program, from the application's own dialect table.
pub fn program() -> &'static str {
    Dialect::for_kind(runtime_kind()).program
}

/// The image under test: `MICOLD_BASIC_IMAGE_TAG`, else the `:dev` tag `mise run image` builds.
pub fn image() -> String {
    std::env::var("MICOLD_BASIC_IMAGE_TAG").unwrap_or_else(|_| DEV_BASIC_IMAGE.to_string())
}

/// Makes the daemon's keys in `<state>/ssh`, pinned for `port`, and returns them with the mount
/// that gives the container's sshd its half.
///
/// The files must exist before the container is created: the runtime would otherwise make each
/// missing single-file bind source as a root-owned directory.
pub fn make_keys(state: &Path, port: u16) -> (DaemonKeys, SecretMount) {
    let dir = state.join("ssh");
    let keys = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime")
                    .block_on(async {
                        let host = SystemSsh {
                            own_daemon: PathBuf::new(),
                            bundled_dir: None,
                            ssh_path: None,
                            cancel: None,
                        };
                        DaemonKeys::ensure(&dir, ClientPlatform::current(), &host)
                            .await
                            .unwrap_or_else(|f| panic!("could not make the SSH keys: {f:?}"))
                    })
            })
            .join()
            .expect("key helper thread")
    });
    keys.write_known_hosts(port).expect("pin the host key");
    let mount = SecretMount {
        host: dir,
        container: PathBuf::from(micold_core::sandbox::argv::CONTAINER_SSH_DIR),
    };
    (keys, mount)
}

/// Whether something on `127.0.0.1:port` sends an SSH banner within `within`.
pub fn sshd_answers(port: u16, within: Duration) -> bool {
    let deadline = Instant::now() + within;
    loop {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut banner = [0u8; 4];
            if stream.read_exact(&mut banner).is_ok() && &banner == b"SSH-" {
                return true;
            }
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// [`sshd_answers`], panicking with `what` when nothing answers.
pub fn wait_for_sshd(port: u16, within: Duration, what: &str) {
    assert!(
        sshd_answers(port, within),
        "{what}: no SSH banner on 127.0.0.1:{port} within {}s",
        within.as_secs()
    );
}
