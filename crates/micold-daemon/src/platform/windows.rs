//! Windows process-tree teardown via a per-session job object (plan W5, FR-036, research R3.2).
//!
//! `portable-pty`'s Windows `kill()` is a `TerminateProcess` on the direct child and nothing else:
//! Windows has no process groups a signal could reach, and a process's children are not ended with
//! it. So the tree is collected into a **job object** as the session starts, and teardown terminates
//! the job.
//!
//! Three constraints from R3.2 shape it, and each is a way to break something that still compiles:
//!
//! - **Only the shell child is assigned.** ConPTY's `conhost.exe` is a separate process the
//!   pseudoconsole owns; putting it in the job would kill the console out from under the reader
//!   rather than letting `ClosePseudoConsole` shut it down in order.
//! - **No `JOB_OBJECT_LIMIT_BREAKAWAY_OK` / `SILENT_BREAKAWAY_OK`.** With either, a descendant can
//!   leave the job and survive the session — precisely the orphan this exists to prevent.
//! - **No UI restrictions.** `JOB_OBJECT_UILIMIT_*` would stop an interactive CLI from using the
//!   clipboard or opening a browser for sign-in.

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

/// The job object holding a session's PTY child and everything it starts.
///
/// `None` when the job could not be set up. Teardown then falls back to the direct child alone —
/// what it did before this existed — and the failure is logged once, at spawn, where it happened.
///
/// **The assignment races the child.** `portable-pty` has already started the process when
/// `spawn_command` returns, and it does not expose the `CREATE_SUSPENDED` + `ResumeThread` sequence
/// that would close the window. A grandchild started in the microseconds before the assignment is
/// outside the job; one started after it — every helper an interactive CLI launches in response to
/// anything — is inside. An escape degrades to the pre-job behaviour, not to something worse.
pub struct ProcessTree {
    job: Option<Job>,
}

impl ProcessTree {
    /// Create a job with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and put the process `pid` in it.
    ///
    /// Opening by pid is safe from reuse here: the caller holds `portable-pty`'s own handle to the
    /// child, and a process object — with its pid — cannot be recycled while any handle to it is
    /// open.
    pub fn adopt(pid: u32) -> Self {
        let job = Job::create_for(pid);
        if let Err(err) = &job {
            tracing::warn!(
                pid,
                %err,
                "could not put the session in a job object; closing it will end its own process \
                 but not processes it started"
            );
        }
        Self { job: job.ok() }
    }

    /// The child's exit status if it has exited. Nothing has to be held back for teardown: the job
    /// reaches the child's descendants whether or not the child is still around.
    pub fn exit_status(
        &self,
        child: &mut dyn portable_pty::Child,
    ) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        child.try_wait()
    }

    /// Nothing to forget: the job is a handle, not a pid, so reaping the child cannot make it name
    /// anything else — and it goes on holding the child's descendants, which teardown should still
    /// end.
    pub fn leader_reaped(&mut self) {}

    /// Terminate every process still in the job. Idempotent: terminating an empty job succeeds.
    pub fn terminate(&self) {
        if let Some(job) = &self.job {
            job.terminate();
        }
    }
}

/// An owned job-object handle. Closing it kills whatever is still inside, so even a teardown path
/// that never calls [`ProcessTree::terminate`] cannot leave the tree running once the session is
/// dropped.
struct Job(HANDLE);

// SAFETY: a job-object handle is a kernel object reference with no thread affinity; every
// operation on it is a thread-safe system call.
unsafe impl Send for Job {}
// SAFETY: as above — `&Job` permits only `TerminateJobObject`, which the kernel synchronises.
unsafe impl Sync for Job {}

impl Job {
    fn create_for(pid: u32) -> std::io::Result<Self> {
        // SAFETY: a null name and null security attributes are the documented "anonymous job"
        // arguments; the call returns a null handle on failure rather than misbehaving.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        // Owned from here on, so every early return below closes it.
        let job = Self(handle);

        // SAFETY: an all-zero `JOBOBJECT_EXTENDED_LIMIT_INFORMATION` is a valid value (no limits).
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: `info` is a fully-initialised value of the type the
        // `JobObjectExtendedLimitInformation` class requires, and its size is passed alongside.
        let set = unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                std::ptr::addr_of!(info).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if set == 0 {
            return Err(std::io::Error::last_os_error());
        }

        // `PROCESS_SET_QUOTA | PROCESS_TERMINATE` is exactly the access `AssignProcessToJobObject`
        // documents; the handle is needed only for the call.
        // SAFETY: plain FFI; failure returns a null handle, checked below.
        let process = unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid) };
        if process.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: both handles are live and owned here.
        let assigned = unsafe { AssignProcessToJobObject(job.0, process) };
        let assign_err = std::io::Error::last_os_error();
        // SAFETY: closing the process handle opened above, exactly once.
        unsafe { CloseHandle(process) };
        if assigned == 0 {
            return Err(assign_err);
        }
        Ok(job)
    }

    fn terminate(&self) {
        // SAFETY: `self.0` is a live job handle owned by this value and closed only in `Drop`.
        unsafe { TerminateJobObject(self.0, 1) };
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: closing a handle this value owns, exactly once.
        unsafe { CloseHandle(self.0) };
    }
}

/// The Windows half of [`super::write_owner_only`]: the directory and the file each get a
/// protected DACL whose only ACE grants the current user full access (`D:P(A;;GA;;;<sid>)`, as the
/// daemon's pipe has). The directory's ACE is inheritable, so the temporary file is owner-only from
/// its creation; its own protected DACL is set before it is renamed into place.
pub(super) fn write_owner_only(
    dir: &std::path::Path,
    file: &str,
    bytes: &[u8],
) -> std::io::Result<std::path::PathBuf> {
    let sid = micold_core::endpoint::user_sid()?;
    std::fs::create_dir_all(dir)?;
    set_protected_dacl(dir, &format!("D:P(A;OICI;GA;;;{sid})"))?;
    let path = dir.join(file);
    let tmp = dir.join(format!(".{file}.tmp"));
    let _ = std::fs::remove_file(&tmp);
    std::fs::write(&tmp, bytes)?;
    set_protected_dacl(&tmp, &format!("D:P(A;;GA;;;{sid})"))?;
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

/// Replace `path`'s DACL with the one `sddl` describes, protected from inheritance.
fn set_protected_dacl(path: &std::path::Path, sddl: &str) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{LocalFree, ERROR_SUCCESS};
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SetNamedSecurityInfoW,
        SDDL_REVISION_1, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::{
        GetSecurityDescriptorDacl, ACL, DACL_SECURITY_INFORMATION,
        PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    };

    let wide_sddl: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
    let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: both strings are NUL-terminated and outlive the calls; the descriptor is a local out
    // pointer freed exactly once with `LocalFree`, and the DACL pointer borrowed from it is used only
    // before that.
    unsafe {
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide_sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let mut present = 0;
        let mut defaulted = 0;
        let mut dacl: *mut ACL = std::ptr::null_mut();
        if GetSecurityDescriptorDacl(sd, &mut present, &mut dacl, &mut defaulted) == 0 {
            let err = std::io::Error::last_os_error();
            LocalFree(sd);
            return Err(err);
        }
        let status = SetNamedSecurityInfoW(
            wide_path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            dacl,
            std::ptr::null(),
        );
        LocalFree(sd);
        if status != ERROR_SUCCESS {
            return Err(std::io::Error::from_raw_os_error(status as i32));
        }
    }
    Ok(())
}
