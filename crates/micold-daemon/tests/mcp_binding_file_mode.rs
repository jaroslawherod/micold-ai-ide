//! Feature 034 (FR-007, SC-009; U30–U33): a session's binding file carries its bearer credential, so
//! only the user who runs the service may read it. `platform::write_owner_only` creates the directory
//! and the file owner-only: modes `0700` / `0600` on Unix, a protected DACL whose one ACE is the
//! current user's on Windows.

use micold_daemon::platform::write_owner_only;

const CONTENT: &[u8] = br#"{"mcpServers":{}}"#;

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &std::path::Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn the_created_directory_is_owner_only() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(mode(&dir), 0o700, "the binding directory must be 0700");
    }

    #[test]
    fn the_written_file_is_owner_only() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        let path = write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(path, dir.join("a.json"));
        assert_eq!(mode(&path), 0o600, "the binding file must be 0600");
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
    }

    #[test]
    fn rewriting_replaces_the_bytes_and_stays_owner_only() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        write_owner_only(&dir, "a.json", b"a much longer first version of the file").unwrap();
        let path = write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
        assert_eq!(mode(&path), 0o600);
    }

    #[test]
    fn an_existing_wider_file_is_narrowed_on_rewrite() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.json");
        std::fs::write(&path, b"old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(mode(&path), 0o600);
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;

    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, ERROR_SUCCESS, HANDLE};
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        EqualSid, GetAce, GetSecurityDescriptorControl, GetTokenInformation, TokenUser,
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
        SE_DACL_PROTECTED, TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    /// `ACCESS_ALLOWED_ACE_TYPE` from winnt.h.
    const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;

    struct Dacl {
        protected: bool,
        ace_count: u16,
        first_ace_allows_current_user: bool,
    }

    fn read_dacl(path: &std::path::Path) -> Dacl {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: `wide` is NUL-terminated and outlives the call; the descriptor is freed below.
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
            assert!(!dacl.is_null(), "{path:?} has a NULL DACL");
            let mut control = 0u16;
            let mut revision = 0u32;
            assert_ne!(GetSecurityDescriptorControl(sd, &mut control, &mut revision), 0);
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

    fn with_current_user_sid<T>(f: impl FnOnce(*mut core::ffi::c_void) -> T) -> T {
        // SAFETY: the token handle is closed before returning; the SID lives in `buffer`.
        unsafe {
            let mut token: HANDLE = null_mut();
            assert_ne!(OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token), 0);
            let mut needed = 0u32;
            GetTokenInformation(token, TokenUser, null_mut(), 0, &mut needed);
            let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
            let ok = GetTokenInformation(
                token,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                needed,
                &mut needed,
            );
            CloseHandle(token);
            assert_ne!(ok, 0, "GetTokenInformation");
            let user = &*(buffer.as_ptr() as *const TOKEN_USER);
            f(user.User.Sid)
        }
    }

    #[test]
    fn the_written_file_has_a_protected_dacl_for_the_current_user_only() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        let path = write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
        let dacl = read_dacl(&path);
        assert!(dacl.protected, "the binding file's DACL must be protected");
        assert_eq!(dacl.ace_count, 1, "exactly one ACE");
        assert!(
            dacl.first_ace_allows_current_user,
            "the one ACE must allow the current user"
        );
    }

    #[test]
    fn rewriting_keeps_the_owner_only_dacl() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("mcp");
        write_owner_only(&dir, "a.json", b"first").unwrap();
        let path = write_owner_only(&dir, "a.json", CONTENT).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), CONTENT);
        let dacl = read_dacl(&path);
        assert!(dacl.protected);
        assert_eq!(dacl.ace_count, 1);
        assert!(dacl.first_ace_allows_current_user);
    }
}
