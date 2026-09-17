//! Windows Job Objects: kill a spawned process together with everything it started.
//!
//! Used by the bounded profile probe (`env_include`).

/// A Windows Job Object owning the spawned process and everything it starts (BUG-003).
///
/// The counterpart to `process_group(0)` + `kill(-pid)` on Unix, and it exists for the same reason:
/// `env_include`'s `read_to_end` returns when the pipe has no *writers* left, not when the child it
/// spawned has died. A profile that backgrounds a helper — a version manager, an agent daemon —
/// leaves that helper holding the pipe, and without a whole-tree kill the read waits for it. On
/// Windows that wait was unbounded: `Child::kill` terminates one process and none of its
/// descendants.
///
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` means closing the handle kills whatever is still in the
/// job, so the `Drop` below is the backstop even on a path that forgets to terminate explicitly.
///
/// **The assignment races the child.** `CreateProcessW` has already started the process by the time
/// `spawn()` returns, so a child that spawns a grandchild in its first instants could in principle
/// escape. Closing that window needs `CREATE_SUSPENDED` and a `ResumeThread`, which needs the main
/// thread handle — and `std`'s `Child` does not expose one. The window is microseconds against
/// interpreter startup measured in tens of milliseconds, and an escape degrades to exactly the
/// behaviour this replaces rather than to something worse.
pub(crate) struct JobHandle(windows_sys::Win32::Foundation::HANDLE);

impl JobHandle {
    /// Create a job and put `child` in it. `None` if either step fails — in which case the caller
    /// falls back to killing the top-level process alone, which is what it did before this existed.
    pub(crate) fn capture(child: &std::process::Child) -> Option<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::HANDLE;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        // Safety: a null name and null security attributes are the documented "anonymous job"
        // arguments; the call returns a null handle on failure rather than misbehaving.
        let job: HANDLE = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return None;
        }
        let handle = Self(job);

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // Safety: `info` is a correctly-sized, fully-initialised value of the type the
        // `JobObjectExtendedLimitInformation` class requires, and its length is passed alongside.
        let set = unsafe {
            SetInformationJobObject(
                handle.0,
                JobObjectExtendedLimitInformation,
                std::ptr::addr_of!(info).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if set == 0 {
            return None;
        }
        // Safety: the handle comes from a live `Child` this function borrows, so it is valid for
        // the duration of the call.
        let assigned =
            unsafe { AssignProcessToJobObject(handle.0, child.as_raw_handle() as HANDLE) };
        if assigned == 0 {
            return None;
        }
        Some(handle)
    }

    /// Kill every process still in the job.
    pub(crate) fn terminate(&self) {
        // Safety: `self.0` is a job handle owned by this value and closed only in `Drop`.
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
        }
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        // Closing the handle kills whatever remains, per the limit flag set above.
        // Safety: closing a handle this value owns, exactly once.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
