//! Per-OS endpoint location + protection (contracts/protocol.md §1, research R1.2/R1.3, FR-030).
//!
//! The socket/pipe is reachable only by the owning user. The path and the directory that guards it
//! are chosen per platform; any ownership/mode surprise **bails loudly** rather than binding into an
//! attacker-controlled location — the "loud, early failure over silent drift" rule.
//!
//! | OS | Endpoint | Guard |
//! |---|---|---|
//! | Linux | `$XDG_RUNTIME_DIR/micold/daemon.sock` | dir `0700`, sticky bit so cleanup won't reap it |
//! | Linux (`$XDG_RUNTIME_DIR` unset) | `/tmp/micold-<uid>/daemon.sock` | dir created `0700`, or **verified, never repaired** if it already exists |
//! | macOS | `$HOME/.micold/run/d.sock` | dir `0700`; `sun_path` length asserted at resolve time |
//! | Windows | `\\.\pipe\Micold.Daemon.<user-SID>` | explicit protected DACL (applied at bind, T083) |
//!
//! # The sandbox's endpoint (feature 027)
//!
//! A containerised daemon cannot be reached this way. A bind-mounted Unix socket does not survive
//! Docker Desktop's file sharing on macOS or Windows — the layer passes file *contents*, not socket
//! semantics — so socket-only would mean Linux-only, which Principle VI forbids. The sandbox
//! therefore listens on **loopback TCP**, published from the container to `127.0.0.1`.
//!
//! That transport carries none of the protection the table above describes: any local process can
//! connect to a loopback port. What replaces it is [`crate::protocol::auth`]'s shared secret, which
//! is mounted into the container as a `0600` file — so the guarantee moves from "you cannot reach
//! it" to "you cannot answer for it", and the filesystem permission is still what enforces it.

use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

/// A resolved endpoint: where to bind the socket/pipe and where the single-instance lock lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// The socket path (Unix) or pipe name (Windows) to bind/connect.
    pub socket_path: PathBuf,
    /// The single-instance lock file (Unix; held for the daemon's lifetime — protocol.md §2).
    pub lock_path: PathBuf,
}

/// Resolve the endpoint for the current user + platform, creating and verifying the guarding
/// directory. A wrong owner or wrong mode on a predictable directory is an error, not a warning.
pub fn resolve() -> io::Result<Endpoint> {
    imp::resolve()
}

/// The default loopback port the sandboxed daemon publishes its control channel on.
///
/// Fixed rather than ephemeral: the client has to know where to dial before the container exists,
/// and a port chosen by the runtime would have to be read back out of `inspect` on every start.
/// It is configurable for the case where something else already holds it.
pub const DEFAULT_SANDBOX_PORT: u16 = 7727;

/// Where a client dials to reach the daemon.
///
/// Two shapes because there are two transports, and the reason is Principle VI rather than taste:
/// see this module's header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialAddress {
    /// The Unix socket or Windows named pipe. Authenticated by filesystem permission.
    Local(Endpoint),
    /// Loopback TCP, for the sandbox. Authenticated by the shared secret, never by the transport.
    Loopback { port: u16 },
}

impl DialAddress {
    /// The loopback socket address for a TCP endpoint.
    ///
    /// `127.0.0.1`, never `0.0.0.0`: publishing the daemon on every interface would put session
    /// input and terminal output on the network, which no part of this feature asks for.
    pub fn socket_addr(&self) -> Option<SocketAddr> {
        match self {
            DialAddress::Local(_) => None,
            DialAddress::Loopback { port } => Some(SocketAddr::from((Ipv4Addr::LOCALHOST, *port))),
        }
    }

    /// A human-readable form for diagnostics.
    pub fn describe(&self) -> String {
        match self {
            DialAddress::Local(e) => e.socket_path.display().to_string(),
            DialAddress::Loopback { port } => format!("127.0.0.1:{port}"),
        }
    }
}

// macOS `sockaddr_un.sun_path` is 104 bytes, 103 usable (research R1.2). Overruns surface as an
// opaque EINVAL, so we assert the budget up front instead.
#[cfg(target_os = "macos")]
const SUN_PATH_MAX: usize = 103;

#[cfg(unix)]
mod imp {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    /// The current effective uid. Only the non-macOS paths (Linux/other-Unix `resolve` and
    /// `verify_owned_0700`) consult it; macOS keys the endpoint on `$HOME`, so gate it off there to
    /// avoid a dead-code warning.
    #[cfg(not(target_os = "macos"))]
    fn euid() -> u32 {
        // SAFETY: `geteuid` is always safe — it takes no arguments and cannot fail.
        unsafe { libc::geteuid() }
    }

    /// Create `dir` (and parents) if missing, then force its mode to exactly `mode`.
    ///
    /// Repair is unconditional, so this is only sound where the *parent* is already ours: the XDG
    /// runtime dir and `$HOME`. On a world-writable parent use [`guard_predictable_dir`], which
    /// verifies instead of repairing.
    fn ensure_dir_mode(dir: &std::path::Path, mode: u32) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        fs::set_permissions(dir, fs::Permissions::from_mode(mode))
    }

    /// Prepare a directory whose parent is world-writable and whose name is predictable: create it
    /// `0700` if it is not there, and **verify it before touching it** if it is.
    ///
    /// The order is the entire point, and getting it wrong is silent (BUG-019). `ensure_dir_mode`
    /// then verify — what this path did until 2026-08-27 — cannot fail: `create_dir_all` succeeds on
    /// an existing directory and `set_permissions` overwrites whatever mode it had, so by the time
    /// the verifier looks the mode is always `0o700` and its refusal is unreachable. A hostile
    /// directory was quietly *repaired* and bound into, which is the opposite of what
    /// protocol.md §1 asks for: anything an attacker left inside it survives the `chmod`.
    ///
    /// So the two cases are kept apart. `create_dir` (not `create_dir_all`) distinguishes them
    /// atomically — `AlreadyExists` is the pre-existing branch, and nothing else reaches it. The
    /// fresh directory is verified too, cheaply, because `mkdir`'s mode is masked by the umask and a
    /// `0700` umask would leave it `0000`; the point is that the verifier is now the only way past
    /// this function.
    #[cfg(not(target_os = "macos"))]
    fn guard_predictable_dir(dir: &std::path::Path) -> io::Result<()> {
        use std::os::unix::fs::DirBuilderExt;
        match fs::DirBuilder::new().mode(0o700).create(dir) {
            Ok(()) => {}
            // Someone else's directory, our own from last time, or a planted symlink — the verifier
            // is what tells them apart, and it is not allowed to be preceded by a repair.
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        verify_owned_0700(dir)
    }

    /// Verify a *predictable, world-writable-parent* directory is ours and locked down. Uses
    /// `symlink_metadata` (not `metadata`) to defeat a planted symlink; requires `uid == euid` and
    /// mode exactly `0o700`. Any failure bails loudly (protocol.md §1, FR-030).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn verify_owned_0700(dir: &std::path::Path) -> io::Result<()> {
        let meta = fs::symlink_metadata(dir)?;
        if !meta.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("endpoint dir {} is not a directory", dir.display()),
            ));
        }
        if meta.uid() != euid() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "endpoint dir {} is owned by uid {}, not {} — refusing to bind",
                    dir.display(),
                    meta.uid(),
                    euid()
                ),
            ));
        }
        let mode = meta.mode() & 0o777;
        if mode != 0o700 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "endpoint dir {} has mode {:#o}, expected 0o700 — refusing to bind",
                    dir.display(),
                    mode
                ),
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(super) fn resolve() -> io::Result<Endpoint> {
        if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()) {
            // XDG dir is already 0700 and user-owned; our subdir gets the sticky bit (0o1700) so a
            // 6-hourly cleanup of untouched files won't reap the socket (research R1.2).
            let dir = PathBuf::from(runtime).join("micold");
            ensure_dir_mode(&dir, 0o1700)?;
            return Ok(endpoint_in(&dir));
        }
        // Fallback: /tmp is world-writable and the path is predictable, so verify before we
        // touch it — see `guard_predictable_dir`.
        resolve_in_predictable_dir(&PathBuf::from(format!("/tmp/micold-{}", euid())))
    }

    /// The `/tmp`-style fallback, with the directory as a parameter.
    ///
    /// `resolve()` above chooses `/tmp/micold-<uid>` and nothing else; splitting the choice from the
    /// guarding is what lets a test drive this path against a directory it can make hostile. Driving
    /// the real `resolve()` would mean making the *actual* `/tmp/micold-<uid>` world-writable on the
    /// machine running the suite, next to a daemon that may be using it (BUG-019).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn resolve_in_predictable_dir(dir: &std::path::Path) -> io::Result<Endpoint> {
        guard_predictable_dir(dir)?;
        Ok(endpoint_in(dir))
    }

    #[cfg(target_os = "macos")]
    pub(super) fn resolve() -> io::Result<Endpoint> {
        let home = std::env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is unset"))?;
        let dir = PathBuf::from(home).join(".micold").join("run");
        ensure_dir_mode(&dir, 0o700)?;
        let ep = Endpoint {
            socket_path: dir.join("d.sock"),
            lock_path: dir.join("daemon.lock"),
        };
        // Assert the sun_path budget so an overrun is loud, not an opaque EINVAL at bind time.
        let len = ep.socket_path.as_os_str().len();
        if len > super::SUN_PATH_MAX {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "socket path is {len} bytes, exceeds macOS sun_path budget of {} — \
                     fall back to _CS_DARWIN_USER_CACHE_DIR",
                    super::SUN_PATH_MAX
                ),
            ));
        }
        Ok(ep)
    }

    // Other Unix (e.g. BSD): treat like Linux's XDG/tmp policy.
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub(super) fn resolve() -> io::Result<Endpoint> {
        resolve_in_predictable_dir(&PathBuf::from(format!("/tmp/micold-{}", euid())))
    }

    #[cfg(any(target_os = "linux", not(target_os = "macos")))]
    fn endpoint_in(dir: &std::path::Path) -> Endpoint {
        Endpoint {
            socket_path: dir.join("daemon.sock"),
            lock_path: dir.join("daemon.lock"),
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::*;

    /// The current user's SID as a string (e.g. `S-1-5-21-...`).
    fn user_sid() -> io::Result<String> {
        // Minimal, dependency-light: shell out to whoami is undesirable; use the USERNAME-derived
        // pipe name is insufficient for isolation. The protected DACL (protocol.md §1) is applied at
        // bind time and verified by the Windows CI gate (T083/W5). Until then we key the pipe on the
        // SID via the `windows-sys` LookupAccountName path added with that gate.
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows endpoint resolution lands with the Windows CI gate (T083/W5)",
        ))
    }

    pub(super) fn resolve() -> io::Result<Endpoint> {
        let sid = user_sid()?;
        Ok(Endpoint {
            socket_path: PathBuf::from(format!(r"\\.\pipe\Micold.Daemon.{sid}")),
            // Windows uses FILE_FLAG_FIRST_PIPE_INSTANCE (atomic create), not a lock file.
            lock_path: PathBuf::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn resolve_creates_a_usable_endpoint_pair() {
        // Windows endpoint resolution is a deliberate stub until the Windows CI gate (T083/W5): it
        // must fail loudly as `Unsupported` rather than bind a half-configured pipe. Asserting the
        // stub contract keeps CI honest without pretending Windows is done.
        let err = resolve().expect_err("windows resolve is a planned stub (T083/W5)");
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }

    #[cfg(unix)]
    #[test]
    fn resolve_creates_a_usable_endpoint_pair() {
        // Force the /tmp-style fallback into a temp XDG dir so the test is hermetic on Linux.
        // (macOS ignores XDG and resolves under $HOME/.micold/run; setting it is harmless there.)
        let tmp = tempfile::tempdir().unwrap();
        // SAFETY: single-threaded test; we set then resolve immediately.
        std::env::set_var("XDG_RUNTIME_DIR", tmp.path());
        let ep = resolve().expect("resolve endpoint");
        // The socket file name is shortened on macOS to fit the 103-byte `sun_path` budget
        // (`d.sock`); every other Unix uses `daemon.sock`. Assert the platform's own name.
        let expected = if cfg!(target_os = "macos") {
            "d.sock"
        } else {
            "daemon.sock"
        };
        assert!(
            ep.socket_path.ends_with(expected),
            "socket path {:?} should end with {expected}",
            ep.socket_path
        );
        assert_ne!(ep.socket_path, ep.lock_path);
        assert!(ep.socket_path.parent().unwrap().is_dir());
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_loose_mode_is_rejected_by_the_verifier() {
        use std::os::unix::fs::PermissionsExt;
        // A dir with mode 0o755 (not 0o700) must be rejected by the /tmp verifier. This is the
        // predicate on its own; `a_pre_existing_loose_directory_is_refused_not_repaired` below is
        // what says `resolve()` ever reaches it. (Named `wrong_owner_dir_bails_loudly` until
        // 2026-08-27, which promised an owner check and performed a mode one — BUG-019.)
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("loose");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let err = imp::verify_owned_0700(&dir).expect_err("loose mode must be rejected");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_directory_owned_by_someone_else_is_refused_by_owner_and_says_so() {
        // The owner branch needs a directory belonging to another uid, which a test cannot create.
        // `/` is owned by root on every machine this runs on, and reading it creates nothing — so it
        // is the one such directory always available. When the suite itself runs as root, `/` *is*
        // ours and the branch is unreachable; skip rather than assert the wrong thing.
        // SAFETY: `geteuid` takes no arguments and cannot fail.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let err = imp::verify_owned_0700(std::path::Path::new("/"))
            .expect_err("a root-owned directory must be refused");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(
            err.to_string().contains("owned by uid 0"),
            "the refusal must name the owner, not the mode: {err}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_missing_directory_is_created_locked_down() {
        use std::os::unix::fs::PermissionsExt;
        // The ordinary first run: nothing there, so make it, 0700, and bind in it.
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("micold-fresh");
        let ep = imp::resolve_in_predictable_dir(&dir).expect("a fresh dir is not hostile");
        assert_eq!(ep.socket_path, dir.join("daemon.sock"));
        let mode = std::fs::symlink_metadata(&dir)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "a directory we create must be ours alone");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_directory_that_is_already_ours_and_locked_down_is_accepted() {
        use std::os::unix::fs::PermissionsExt;
        // The ordinary second run. Refusing every pre-existing directory would be a safe rule and a
        // useless one — the daemon restarts into the same directory it made.
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("micold-again");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        imp::resolve_in_predictable_dir(&dir).expect("our own 0700 dir must be accepted");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_pre_existing_loose_directory_is_refused_not_repaired() {
        use std::os::unix::fs::PermissionsExt;
        // Quickstart S11, third block, as reported: a world-writable `/tmp/micold-<uid>` must bail
        // loudly. The second assertion is the one that distinguishes the fix from the defect —
        // repairing the mode and *then* verifying also ends 0700, and never fails (BUG-019).
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("micold-loose");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o777)).unwrap();

        let err = imp::resolve_in_predictable_dir(&dir).expect_err("a 0777 dir must be refused");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(
            err.to_string().contains("0o777"),
            "the refusal must name the mode it found: {err}"
        );
        let mode = std::fs::symlink_metadata(&dir)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, 0o777,
            "the directory must be left as found — a human decides what to do with it"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_symlink_in_place_of_the_directory_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        // The planted symlink `symlink_metadata` exists for: the target is ours and 0700, so every
        // check but the first one passes. `create_dir` on it returns AlreadyExists, which is exactly
        // why the pre-existing branch must verify rather than assume.
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("elsewhere");
        std::fs::create_dir(&target).unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700)).unwrap();
        let link = tmp.path().join("micold-link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let err = imp::resolve_in_predictable_dir(&link).expect_err("a symlink must be refused");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(
            err.to_string().contains("not a directory"),
            "the refusal must say what it found: {err}"
        );
    }
}
