//! Read back what protects a file: the mode bits on Unix, the DACL on Windows. Included with
//! `#[path]` by `owner_only.rs` (core), `mcp_binding_file_mode.rs` and `windows_pipe_acl.rs`
//! (daemon); each uses a different part, so unused items are expected.
#![allow(dead_code)]

#[cfg(unix)]
pub use unix::{mode, set_mode};

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    pub fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    pub fn set_mode(path: &Path, mode: u32) {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
}

#[cfg(windows)]
pub use windows::{read_dacl, summarize_dacl, with_current_user_sid, Dacl};

#[cfg(windows)]
mod windows {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr::null_mut;

    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, ERROR_SUCCESS, HANDLE};
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        EqualSid, GetAce, GetSecurityDescriptorControl, GetTokenInformation, TokenUser,
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
        SE_DACL_PROTECTED, TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    /// `ACCESS_ALLOWED_ACE_TYPE` from winnt.h. windows-sys files it under `System_SystemServices`,
    /// a whole feature for one constant.
    const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;

    /// What a security descriptor's DACL says.
    #[derive(Debug, PartialEq)]
    pub struct Dacl {
        pub protected: bool,
        pub ace_count: u16,
        /// Whether ACE 0 is `ACCESS_ALLOWED` for the current token's user SID.
        pub first_ace_allows_current_user: bool,
    }

    /// The DACL of the file or directory at `path`.
    pub fn read_dacl(path: &Path) -> Dacl {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: `wide` is NUL-terminated and outlives the call; `summarize_dacl` frees the
        // descriptor.
        unsafe {
            let mut dacl: *mut ACL = null_mut();
            let mut sd: PSECURITY_DESCRIPTOR = null_mut();
            let status = GetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut sd,
            );
            assert_eq!(status, ERROR_SUCCESS, "GetNamedSecurityInfoW on {path:?}");
            summarize_dacl(dacl, sd, path)
        }
    }

    /// Summarise `dacl`, which lives inside `sd`, then free `sd`. `what` names the object in a
    /// failure.
    ///
    /// # Safety
    /// `dacl` and `sd` come from one successful `Get*SecurityInfo` call; `sd` is not used again.
    pub unsafe fn summarize_dacl(dacl: *mut ACL, sd: PSECURITY_DESCRIPTOR, what: &Path) -> Dacl {
        // SAFETY: the caller's contract; every out pointer below is a local.
        unsafe {
            assert!(
                !dacl.is_null(),
                "{what:?} has a NULL DACL, which grants everyone full access"
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
            Dacl {
                protected: control & SE_DACL_PROTECTED != 0,
                ace_count,
                first_ace_allows_current_user,
            }
        }
    }

    /// Run `f` with the current process token's user SID.
    pub fn with_current_user_sid<T>(f: impl FnOnce(*mut core::ffi::c_void) -> T) -> T {
        // SAFETY: the token handle is closed before returning, and the SID pointer lives in
        // `buffer`, which outlives `f`.
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
}
