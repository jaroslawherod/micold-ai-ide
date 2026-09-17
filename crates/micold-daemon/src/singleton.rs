//! Single-instance startup (contracts/protocol.md §2, research R1.4).
//!
//! Two clients starting simultaneously **must** converge on exactly one daemon. On Unix, socket
//! existence proves nothing (the kernel never removes a bound socket), so `connect()` is the
//! liveness discriminator and a `flock`-held lock file arbitrates the recovery race:
//!
//! ```text
//! 1. connect()  -> Ok            => a daemon is live; act as client. Fast path, no lock.
//! 2. try_lock exclusive          => WouldBlock: another starter is mid-recovery; back off, goto 1.
//! 3. RE-CHECK connect()          => the MANDATORY step: without it the lock loser unlinks the
//!                                   winner's live socket. Ok => drop lock, act as client.
//! 4. unlink(sock) if S_ISSOCK; bind; listen
//! 5. HOLD THE LOCK FOR PROCESS LIFETIME  (released only when the fd closes — incl. SIGKILL/OOM,
//!                                         which makes it an unforgeable liveness beacon)
//! ```

use std::io;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Duration;

use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::tokio::{Listener, Stream};
use interprocess::local_socket::{GenericFilePath, ListenerOptions, Name, ToFsName};

use micold_core::endpoint::Endpoint;

/// The result of a startup attempt.
pub enum Acquisition {
    /// We became the daemon; hold on to this for the process lifetime.
    Bound(BoundListener),
    /// A live daemon already owns the endpoint; act as a client.
    AlreadyRunning,
}

/// A bound listener plus the single-instance lock, held together for the daemon's whole life.
pub struct BoundListener {
    /// The accept side of the endpoint.
    pub listener: Listener,
    /// Held for lifetime — dropping it releases the `flock` and the endpoint (step 5). `None` on
    /// Windows, where the first pipe instance is itself the guard.
    _lock: Option<std::fs::File>,
    socket_path: PathBuf,
}

impl BoundListener {
    /// The path this listener is bound to.
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

impl Drop for BoundListener {
    fn drop(&mut self) {
        // Best-effort: leave no stale socket behind on a clean shutdown. (A crash leaves it, and the
        // next start reclaims it via the connect-test + S_ISSOCK unlink — that is by design.)
        let _ = unlink_if_socket(&self.socket_path);
    }
}

fn fs_name(path: &Path) -> io::Result<Name<'_>> {
    path.as_os_str().to_fs_name::<GenericFilePath>()
}

/// `true` iff something is accepting connections at `path` right now.
async fn is_live(path: &Path) -> bool {
    match fs_name(path) {
        Ok(name) => Stream::connect(name).await.is_ok(),
        Err(_) => false,
    }
}

/// Remove `path` only if it is a socket (guard against clobbering a real file); ignore `ENOENT`.
#[cfg(unix)]
fn unlink_if_socket(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::FileTypeExt;
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_socket() => std::fs::remove_file(path),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(unix))]
fn unlink_if_socket(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Run the single-instance startup sequence for `endpoint`.
#[cfg(unix)]
pub async fn acquire(endpoint: &Endpoint) -> io::Result<Acquisition> {
    use std::fs::{OpenOptions, TryLockError};

    // 1. Fast path: a live daemon already owns the endpoint.
    if is_live(&endpoint.socket_path).await {
        return Ok(Acquisition::AlreadyRunning);
    }

    loop {
        // 2. Take the recovery lock. Keep the lockfile off NFS (endpoint policy ensures a local dir).
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&endpoint.lock_path)?;

        match lock.try_lock() {
            Ok(()) => {
                // 3. RE-CHECK — the non-optional step. Another starter may have bound between our
                //    step 1 and acquiring the lock.
                if is_live(&endpoint.socket_path).await {
                    drop(lock);
                    return Ok(Acquisition::AlreadyRunning);
                }
                // 4. Reclaim a stale socket, then bind.
                unlink_if_socket(&endpoint.socket_path)?;
                let name = fs_name(&endpoint.socket_path)?;
                let listener = ListenerOptions::new().name(name).create_tokio()?;
                // 5. Return the listener AND the still-locked file — held for life.
                return Ok(Acquisition::Bound(BoundListener {
                    listener,
                    _lock: Some(lock),
                    socket_path: endpoint.socket_path.clone(),
                }));
            }
            Err(TryLockError::WouldBlock) => {
                // Another starter is mid-recovery. Touch nothing; back off and re-probe.
                drop(lock);
                tokio::time::sleep(Duration::from_millis(25)).await;
                if is_live(&endpoint.socket_path).await {
                    return Ok(Acquisition::AlreadyRunning);
                }
                continue;
            }
            Err(TryLockError::Error(e)) => return Err(e),
        }
    }
}

/// Windows needs no lock file: `interprocess` creates the first pipe instance with
/// `FILE_FLAG_FIRST_PIPE_INSTANCE` itself (`create_instance.rs:86`), an atomic create-or-fail with no
/// TOCTOU gap, so a second starter's bind fails and it acts as a client.
///
/// The pipe gets an explicit protected DACL granting only the current user (research R2, FR-021):
/// the default descriptor lets Everyone and anonymous logons read it.
#[cfg(windows)]
pub async fn acquire(endpoint: &Endpoint) -> io::Result<Acquisition> {
    #[allow(unused_imports)] // MUTANT
    use interprocess::os::windows::local_socket::ListenerOptionsExt;

    if is_live(&endpoint.socket_path).await {
        return Ok(Acquisition::AlreadyRunning);
    }
    let name = fs_name(&endpoint.socket_path)?;
    // MUTANT (U4, U5): the pipe is bound with the default security descriptor.
    let _ = owner_only_descriptor;
    let options = ListenerOptions::new().name(name);
    match options.create_tokio() {
        Ok(listener) => Ok(Acquisition::Bound(BoundListener {
            listener,
            _lock: None,
            socket_path: endpoint.socket_path.clone(),
        })),
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => Ok(Acquisition::AlreadyRunning),
        // Losing the first-instance race to this user's own daemon reads as access denied, and so
        // does a pipe another account created first. Only the first can be connected to.
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            match Stream::connect(fs_name(&endpoint.socket_path)?).await {
                Err(refused) if refused.kind() == io::ErrorKind::PermissionDenied => {
                    Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!(
                            "{} exists but refuses this user, so another account may hold it: {refused}",
                            endpoint.socket_path.display()
                        ),
                    ))
                }
                _ => Ok(Acquisition::AlreadyRunning),
            }
        }
        Err(e) => Err(e),
    }
}

/// `D:P(A;;GA;;;<sid>)`: a protected DACL (nothing inherited) with one entry, full access for the
/// user running this process.
#[cfg(windows)]
fn owner_only_descriptor(
) -> io::Result<interprocess::os::windows::security_descriptor::SecurityDescriptor> {
    use interprocess::os::windows::security_descriptor::{
        AsSecurityDescriptorExt, BorrowedSecurityDescriptor,
    };
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };

    let sddl = format!("D:P(A;;GA;;;{})", micold_core::endpoint::user_sid()?);
    let wide: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
    let mut relative: *mut core::ffi::c_void = std::ptr::null_mut();
    // SAFETY: `wide` is NUL-terminated and outlives the call; `relative` is a local out pointer that
    // is freed with `LocalFree` below, as the call's contract requires. (This is what
    // `SecurityDescriptor::deserialize` does, without naming its `widestring` argument type.)
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide.as_ptr(),
            SDDL_REVISION_1,
            &mut relative,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a successful conversion returned a valid self-relative descriptor, borrowed only until
    // the copy below and freed exactly once after it.
    unsafe {
        let owned = BorrowedSecurityDescriptor::from_ptr(relative).to_owned_sd();
        LocalFree(relative);
        owned
    }
}
