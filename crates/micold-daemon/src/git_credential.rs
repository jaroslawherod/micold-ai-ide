//! `micold-daemon git-credential <get|store|erase>`: the credential helper git inside a sandbox
//! runs (#1002, FR-006, FR-007, contracts/git-in-container.md §2).
//!
//! It answers `get` from the in-memory file of the one repository the container holds a credential
//! for, and nothing for any other repository or host. `store` and `erase` read and discard their
//! input: a token is never kept anywhere git could write it. Every path exits 0, so git falls
//! through to "no credential" rather than to an error.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use micold_core::scoped_credential::helper::{answer, parse_request, GIT_CREDENTIAL_ROOT};
use micold_core::scoped_credential::GITHUB_BASE;

/// Test override of the credential root, as `repo_clone::ROOT_ENV` is for the clone root.
pub const ROOT_ENV: &str = "MICOLD_GIT_CREDENTIAL_ROOT";
/// Test override of the forge base the helper answers for (a loopback server in the tests).
pub const BASE_ENV: &str = "MICOLD_GIT_CREDENTIAL_BASE";

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Run the helper with `args` (what follows `git-credential`), reading `input` and writing
/// `output`. Returns the exit status: always 0.
pub fn run(args: &[OsString], input: &mut impl Read, output: &mut impl Write) -> i32 {
    let mut text = String::new();
    // Git sends text; whatever cannot be read is an empty request, which gets no answer.
    let _ = input.read_to_string(&mut text);
    if args.first().map(OsString::as_os_str) == Some(std::ffi::OsStr::new("get")) {
        let root = PathBuf::from(env_or(ROOT_ENV, GIT_CREDENTIAL_ROOT));
        let base = env_or(BASE_ENV, GITHUB_BASE);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let _ = output.write_all(answer(&root, &base, &parse_request(&text), now).as_bytes());
    }
    0
}

/// The process entry: stdin to stdout.
pub fn main(args: &[OsString]) -> i32 {
    run(args, &mut std::io::stdin().lock(), &mut std::io::stdout())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_erase_print_nothing_and_exit_zero() {
        for verb in ["store", "erase", "bogus"] {
            let mut out = Vec::new();
            let code = run(
                &[verb.into()],
                &mut &b"protocol=https\nhost=github.com\npassword=x\n\n"[..],
                &mut out,
            );
            assert_eq!((code, out.as_slice()), (0, &b""[..]), "{verb}");
        }
    }

    #[test]
    fn get_without_a_file_prints_nothing_and_exits_zero() {
        let mut out = Vec::new();
        let code = run(
            &["get".into()],
            &mut &b"protocol=https\nhost=github.com\npath=o/r\n\n"[..],
            &mut out,
        );
        assert_eq!((code, out.as_slice()), (0, &b""[..]));
    }
}
