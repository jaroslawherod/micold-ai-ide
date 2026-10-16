//! This machine's hostname, read straight from the OS (`gethostname(2)` on Unix,
//! `GetComputerNameExW` on Windows). Empty when the OS gives none.

/// The hostname as the OS reports it; on Windows the physical DNS host name, which is what
/// `hostname` prints. Not valid UTF-8 is replaced lossily, as a hostname is only ever compared.
pub fn hostname() -> String {
    read().unwrap_or_default()
}

#[cfg(unix)]
fn read() -> Option<String> {
    // POSIX caps a hostname at HOST_NAME_MAX (255 on Linux, 255 on macOS's MAXHOSTNAMELEN - 1).
    let mut buf = [0u8; 256];
    // SAFETY: the pointer and length describe `buf`, which outlives the call.
    if unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) } != 0 {
        return None;
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    Some(String::from_utf8_lossy(&buf[..end]).into_owned())
}

#[cfg(windows)]
fn read() -> Option<String> {
    use windows_sys::Win32::System::SystemInformation::{
        ComputerNamePhysicalDnsHostname, GetComputerNameExW,
    };
    let mut len: u32 = 0;
    // SAFETY: a null buffer with a zero length only asks for the size, written to `len`.
    unsafe {
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            std::ptr::null_mut(),
            &mut len,
        )
    };
    if len == 0 {
        return None;
    }
    let mut buf = vec![0u16; len as usize];
    // SAFETY: `len` is `buf`'s length in UTF-16 units, as the call requires.
    if unsafe { GetComputerNameExW(ComputerNamePhysicalDnsHostname, buf.as_mut_ptr(), &mut len) }
        == 0
    {
        return None;
    }
    // On success `len` is the name's length without the trailing NUL.
    buf.truncate(len as usize);
    Some(String::from_utf16_lossy(&buf))
}

#[cfg(test)]
mod tests {
    use super::hostname;

    /// The name matches what the system's own tool prints (`uname -n`, `hostname` on Windows).
    #[test]
    fn hostname_matches_the_systems_own_tool() {
        let out = if cfg!(windows) {
            std::process::Command::new("hostname").output()
        } else {
            std::process::Command::new("uname").arg("-n").output()
        }
        .expect("the system's hostname tool runs");
        let expected = String::from_utf8_lossy(&out.stdout).trim_end().to_string();
        assert!(!expected.is_empty(), "the hostname tool printed a name");
        assert_eq!(hostname(), expected);
    }
}
