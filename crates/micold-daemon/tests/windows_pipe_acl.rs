//! Feature 030, E2.1 (FR-021, SC-009): the daemon's pipe is reachable only by the user who started it.
//!
//! A named pipe created with the default security descriptor grants read access to Everyone and to
//! anonymous logons, so another user on the machine could open it. The daemon sets an explicit DACL
//! instead: protected (no inherited entries) and holding exactly one ACE, for the current token's user
//! SID. These cases bind through the real `singleton::acquire` and read the DACL back off the pipe.

#![cfg(windows)]

use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::{null, null_mut};

use micold_core::endpoint::Endpoint;
use micold_daemon::singleton::{self, Acquisition};
use windows_sys::Win32::Foundation::{
    CloseHandle, LocalFree, ERROR_SUCCESS, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
    SE_KERNEL_OBJECT,
};
use windows_sys::Win32::Security::{
    EqualSid, GetAce, GetSecurityDescriptorControl, GetTokenInformation, TokenUser,
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    SECURITY_ATTRIBUTES, SE_DACL_PROTECTED, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    PIPE_ACCESS_DUPLEX, READ_CONTROL,
};
use windows_sys::Win32::System::Pipes::{
    CreateNamedPipeW, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// `ACCESS_ALLOWED_ACE_TYPE` from winnt.h. windows-sys files it under `System_SystemServices`, a
/// whole feature for one constant.
const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;

/// A pipe name no other test or running daemon uses: the pipe namespace is machine-wide.
fn test_endpoint(dir: &Path, case: &str) -> Endpoint {
    Endpoint {
        socket_path: format!(r"\\.\pipe\Micold.Test.Acl.{case}.{}", std::process::id()).into(),
        lock_path: dir.join("micold-daemon.pid"),
    }
}

/// A pipe instance someone else holds at `pipe`, created with the DACL in `sddl`. Dropping it closes the
/// only instance, so the name goes away.
struct SquattedPipe(HANDLE);

impl SquattedPipe {
    fn create(pipe: &Path, sddl: &str) -> Self {
        let wide: Vec<u16> = pipe.as_os_str().encode_wide().chain(Some(0)).collect();
        let sddl: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
        // SAFETY: both strings are NUL-terminated and outlive the calls; the descriptor is freed once
        // the pipe holds its own copy, and the handle is closed in `Drop`.
        unsafe {
            let mut sd: PSECURITY_DESCRIPTOR = null_mut();
            assert_ne!(
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    SDDL_REVISION_1,
                    &mut sd,
                    null_mut(),
                ),
                0,
                "ConvertStringSecurityDescriptorToSecurityDescriptorW: {}",
                std::io::Error::last_os_error()
            );
            let attributes = SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: sd,
                bInheritHandle: 0,
            };
            let handle = CreateNamedPipeW(
                wide.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE,
                PIPE_UNLIMITED_INSTANCES,
                4096,
                4096,
                0,
                &attributes,
            );
            let created = std::io::Error::last_os_error();
            LocalFree(sd);
            assert_ne!(handle, INVALID_HANDLE_VALUE, "create {pipe:?}: {created}");
            SquattedPipe(handle)
        }
    }
}

impl Drop for SquattedPipe {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful `CreateNamedPipeW` and is closed only here.
        unsafe { CloseHandle(self.0) };
    }
}

/// What the bound pipe's security descriptor says, read while the listener is still held.
struct PipeDacl {
    protected: bool,
    ace_count: u16,
    /// Whether ACE 0 is `ACCESS_ALLOWED` for the current token's user SID.
    first_ace_allows_current_user: bool,
}

async fn bound_pipe_dacl(case: &str) -> PipeDacl {
    let dir = tempfile::tempdir().unwrap();
    let ep = test_endpoint(dir.path(), case);
    let Acquisition::Bound(listener) = singleton::acquire(&ep).await.expect("acquire") else {
        panic!("a fresh test pipe must bind, not find a running daemon");
    };
    let dacl = read_dacl(&ep.socket_path);
    drop(listener);
    dacl
}

fn read_dacl(pipe: &Path) -> PipeDacl {
    let wide: Vec<u16> = pipe.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: `wide` is NUL-terminated and outlives the call; every out pointer is either null or a
    // local, and the handle, the token and the descriptor are each released below.
    unsafe {
        let handle = CreateFileW(
            wide.as_ptr(),
            READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            0,
            null_mut(),
        );
        assert_ne!(
            handle,
            INVALID_HANDLE_VALUE,
            "open {pipe:?} for READ_CONTROL: {}",
            std::io::Error::last_os_error()
        );

        let mut dacl: *mut ACL = null_mut();
        let mut sd: PSECURITY_DESCRIPTOR = null_mut();
        let status = GetSecurityInfo(
            handle,
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut sd,
        );
        CloseHandle(handle);
        assert_eq!(status, ERROR_SUCCESS, "GetSecurityInfo on {pipe:?}");
        assert!(
            !dacl.is_null(),
            "{pipe:?} has a NULL DACL, which grants everyone full access"
        );

        let mut control = 0u16;
        let mut revision = 0u32;
        assert_ne!(
            GetSecurityDescriptorControl(sd, &mut control, &mut revision),
            0,
            "GetSecurityDescriptorControl: {}",
            std::io::Error::last_os_error()
        );

        let ace_count = (*dacl).AceCount;
        let mut first_ace_allows_current_user = false;
        let mut ace: *mut core::ffi::c_void = null_mut();
        if ace_count > 0 && GetAce(dacl, 0, &mut ace) != 0 {
            let header = &*(ace as *const ACE_HEADER);
            if header.AceType == ACCESS_ALLOWED_ACE_TYPE {
                let allowed = ace as *const ACCESS_ALLOWED_ACE;
                let sid = std::ptr::addr_of!((*allowed).SidStart) as *mut core::ffi::c_void;
                first_ace_allows_current_user =
                    with_current_user_sid(|user| EqualSid(sid, user) != 0);
            }
        }

        LocalFree(sd);
        PipeDacl {
            protected: control & SE_DACL_PROTECTED != 0,
            ace_count,
            first_ace_allows_current_user,
        }
    }
}

/// Run `f` with the current process token's user SID.
fn with_current_user_sid<T>(f: impl FnOnce(*mut core::ffi::c_void) -> T) -> T {
    // SAFETY: the token handle is closed before returning, and the SID pointer lives in `buffer`,
    // which outlives `f`.
    unsafe {
        let mut token: HANDLE = null_mut();
        assert_ne!(
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token),
            0,
            "OpenProcessToken: {}",
            std::io::Error::last_os_error()
        );
        let mut needed = 0u32;
        GetTokenInformation(token, TokenUser, null_mut(), 0, &mut needed);
        // u64 words keep the buffer aligned for TOKEN_USER's pointer field.
        let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        );
        CloseHandle(token);
        assert_ne!(
            ok,
            0,
            "GetTokenInformation: {}",
            std::io::Error::last_os_error()
        );
        let user = &*(buffer.as_ptr() as *const TOKEN_USER);
        f(user.User.Sid)
    }
}

#[tokio::test]
async fn the_bound_pipe_dacl_is_protected() {
    // U4: no entry inherited from a parent object can widen who may open the pipe.
    let dacl = bound_pipe_dacl("Protected").await;
    assert!(
        dacl.protected,
        "the pipe's DACL must carry SE_DACL_PROTECTED"
    );
}

#[tokio::test]
async fn the_bound_pipe_dacl_allows_only_the_current_user() {
    // U5: exactly one ACE, and it names the user who started the daemon.
    let dacl = bound_pipe_dacl("OnlyUser").await;
    assert_eq!(
        dacl.ace_count, 1,
        "the pipe's DACL must hold exactly one ACE"
    );
    assert!(
        dacl.first_ace_allows_current_user,
        "the pipe's one ACE must be ACCESS_ALLOWED for the current token's user SID"
    );
}

#[tokio::test]
async fn binding_over_a_pipe_that_refuses_this_user_is_an_error_naming_it() {
    // U73 (security review D1): another account got to this user's pipe name first. Its pipe lets
    // only SYSTEM in, so this user can neither connect nor add an instance. That is not a running
    // daemon of this user, and quietly exiting as if it were would leave the squatter serving.
    let dir = tempfile::tempdir().unwrap();
    let ep = test_endpoint(dir.path(), "Squatted");
    let _squatter = SquattedPipe::create(&ep.socket_path, "D:P(A;;GA;;;SY)");

    match singleton::acquire(&ep).await {
        Err(e) => assert!(
            e.to_string().contains(&*ep.socket_path.to_string_lossy()),
            "the error must name the pipe {:?}, got: {e}",
            ep.socket_path
        ),
        Ok(Acquisition::AlreadyRunning) => {
            panic!("a pipe this user cannot open was taken for this user's running daemon")
        }
        Ok(Acquisition::Bound(_)) => panic!("bound over a pipe another account holds"),
    }
}
