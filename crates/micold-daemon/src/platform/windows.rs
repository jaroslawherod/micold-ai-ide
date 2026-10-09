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

/// The class of the service's hidden end-of-session window. Tests find the window by it.
pub const STOP_WINDOW_CLASS: &str = "Micold.Daemon.StopWindow";

/// How long the end-of-session window holds Windows back while the unwind runs. Windows ends an
/// application that has not answered `WM_ENDSESSION` after about five seconds anyway.
const END_SESSION_WAIT: std::time::Duration = std::time::Duration::from_millis(4_500);

/// The one request of this process, raised by the stop event or by the end-of-session window.
static REQUEST: std::sync::OnceLock<tokio::sync::watch::Sender<bool>> = std::sync::OnceLock::new();

/// The request's sender, creating it, the event thread and the window thread on first use.
fn request() -> &'static tokio::sync::watch::Sender<bool> {
    REQUEST.get_or_init(|| {
        let (sender, _) = tokio::sync::watch::channel(false);
        match StopEvent::create() {
            Ok(event) => {
                let spawned =
                    std::thread::Builder::new()
                        .name("stop-event".into())
                        .spawn(move || {
                            event.wait();
                            raise();
                        });
                if let Err(err) = spawned {
                    tracing::warn!(%err, "could not wait for the stop event");
                }
            }
            Err(err) => tracing::warn!(
                %err,
                "could not create the stop event; a restart or an update will end this service \
                 without saving first"
            ),
        }
        if let Err(err) = std::thread::Builder::new()
            .name("stop-window".into())
            .spawn(run_end_session_window)
        {
            tracing::warn!(%err, "could not start the end-of-session window");
        }
        sender
    })
}

/// Raise the request. A second raise changes nothing.
fn raise() {
    if let Some(sender) = REQUEST.get() {
        sender.send_replace(true);
    }
}

/// The named, manual-reset event that asks this user's service to stop (stop-request contract §3).
struct StopEvent(HANDLE);

// SAFETY: an event handle is a kernel object reference with no thread affinity.
unsafe impl Send for StopEvent {}

impl StopEvent {
    /// Created with a protected DACL of one entry, full access for the current user, as the pipe's.
    /// An event of that name that already exists (a second service of this user starting) is opened
    /// instead of created, which is the same event.
    fn create() -> std::io::Result<Self> {
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Authorization::{
            ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
        };
        use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
        use windows_sys::Win32::System::Threading::CreateEventW;

        let name = micold_core::endpoint::stop_event_name()?;
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let sddl = format!("D:P(A;;GA;;;{})", micold_core::endpoint::user_sid()?);
        let sddl: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
        let mut descriptor: *mut core::ffi::c_void = std::ptr::null_mut();
        // SAFETY: `sddl` is NUL-terminated and outlives the call; `descriptor` is a local out
        // pointer, freed with `LocalFree` below as the call's contract requires.
        let converted = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                std::ptr::null_mut(),
            )
        };
        if converted == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        // SAFETY: `attributes` and `name` are valid for the call, which copies the descriptor.
        let handle = unsafe { CreateEventW(&attributes, 1, 0, name.as_ptr()) };
        let created = std::io::Error::last_os_error();
        // SAFETY: freeing the descriptor `ConvertString...` allocated, once, after its last use.
        unsafe { LocalFree(descriptor) };
        if handle.is_null() {
            return Err(created);
        }
        Ok(Self(handle))
    }

    /// Block until the event is set. Returns at once if waiting fails, which is not retried.
    fn wait(&self) {
        use windows_sys::Win32::System::Threading::{WaitForSingleObject, INFINITE};
        // SAFETY: `self.0` is a live event handle owned by this value.
        unsafe { WaitForSingleObject(self.0, INFINITE) };
    }
}

impl Drop for StopEvent {
    fn drop(&mut self) {
        // SAFETY: closing a handle this value owns, exactly once.
        unsafe { CloseHandle(self.0) };
    }
}

/// The window procedure of the hidden window: a logout, shutdown or restart of Windows sends it
/// `WM_QUERYENDSESSION`, then `WM_ENDSESSION`.
unsafe extern "system" fn end_session_proc(
    window: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, WM_ENDSESSION, WM_QUERYENDSESSION,
    };
    match message {
        // Never veto: the service has nothing the user could be asked about.
        WM_QUERYENDSESSION => 1,
        WM_ENDSESSION => {
            // `wparam` is true when the session really ends. Raise the request, then hold Windows
            // back so the unwind can save; the process exits when it has, ending this wait.
            if wparam != 0 {
                raise();
                std::thread::sleep(END_SESSION_WAIT);
            }
            0
        }
        // SAFETY: the arguments are the ones this procedure received.
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

/// Create the hidden top-level window and pump its messages for the life of the process. It must be
/// top-level, not message-only: Windows sends the end-of-session messages to top-level windows only.
fn run_end_session_window() {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DispatchMessageW, GetMessageW, RegisterClassW, TranslateMessage, MSG,
        WNDCLASSW,
    };

    let class: Vec<u16> = STOP_WINDOW_CLASS.encode_utf16().chain(Some(0)).collect();
    // SAFETY: plain FFI with valid, NUL-terminated strings that outlive the calls; a zero return
    // from either create call is the failure signal and is handled.
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let mut window_class: WNDCLASSW = std::mem::zeroed();
        window_class.lpfnWndProc = Some(end_session_proc);
        window_class.hInstance = instance;
        window_class.lpszClassName = class.as_ptr();
        if RegisterClassW(&window_class) == 0 {
            tracing::warn!(err = %std::io::Error::last_os_error(), "could not register the end-of-session window");
            return;
        }
        let window = CreateWindowExW(
            0,
            class.as_ptr(),
            class.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if window.is_null() {
            tracing::warn!(err = %std::io::Error::last_os_error(), "could not create the end-of-session window");
            return;
        }
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// Completes when the process is asked to stop: the event `Local\Micold.Daemon.Stop.<SID>` is set
/// (**Restart service**, the installer), or the hidden window receives `WM_ENDSESSION` (logout,
/// shutdown). Pending until then. A later request changes nothing (stop-request contract §1).
///
/// The event and the window exist from the call, not from the first poll, so a request that arrives
/// before the accept loop starts is held for it. Call it inside the runtime.
pub fn stop_requested() -> impl std::future::Future<Output = ()> + Send {
    let mut receiver = request().subscribe();
    async move {
        let _ = receiver.wait_for(|raised| *raised).await;
    }
}
