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
