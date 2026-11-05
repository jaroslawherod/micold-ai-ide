//! `micold-daemon git-credential <get|store|erase>`: the credential helper git inside a sandbox
//! runs (#1002, FR-006, FR-007, contracts/git-in-container.md §2).
//!
//! With `MICOLD_DAEMON_CONFIG` set (a session of a daemon that has a configuration file, #1530) it
//! answers `get` from the configuration: [`crate::credential_config`]. Without it, it answers from the
//! in-memory file of the one repository the container holds a credential for (the grant path, retired
//! with the grant messages), and nothing for any other repository or host. `store` and `erase` read and discard their
//! input: a token is never kept anywhere git could write it. Every path exits 0, so git falls
//! through to "no credential" rather than to an error.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use micold_core::scoped_credential::git_env::{CONFIG_ENV, FALLBACK_ENV};
use micold_core::scoped_credential::helper::{answer, parse_request, GIT_CREDENTIAL_ROOT};
use micold_core::scoped_credential::{UnixTime, GITHUB_BASE};

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

/// Where this daemon keeps credential files, or `None` when it is not sandboxed (#1002): the test
/// override [`ROOT_ENV`], else the default root in a container, in a pod (`POD_ENV`) or when the
/// runtime mounted an authentication token. A host-process daemon holds no credential.
pub fn credential_root() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os(ROOT_ENV).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(root));
    }
    credential_root_of(&crate::settings::current())
}

/// [`credential_root`] without the test override: a daemon with a configuration file is sandboxed
/// exactly when `daemon.kind` is `container` or `kubernetes` (#1524). One without a file still
/// infers it from the old variables, until they are retired.
pub fn credential_root_of(settings: &crate::settings::DaemonSettings) -> Option<PathBuf> {
    use micold_core::daemon_config::Kind;
    let sandboxed = match settings.kind {
        Some(kind) => matches!(kind, Kind::Container | Kind::Kubernetes),
        None => {
            !settings.image.is_empty()
                || settings.token_file.is_some()
                || std::env::var_os(micold_core::kube::credentials::POD_ENV).is_some()
        }
    };
    sandboxed.then(|| PathBuf::from(GIT_CREDENTIAL_ROOT))
}

/// Run the helper with `args` (what follows `git-credential`), reading `input` and writing
/// `output`. Returns the exit status: always 0.
pub fn run(args: &[OsString], input: &mut impl Read, output: &mut impl Write) -> i32 {
    let mut text = String::new();
    // Git sends text; whatever cannot be read is an empty request, which gets no answer.
    let _ = input.read_to_string(&mut text);
    if args.first().map(OsString::as_os_str) == Some(std::ffi::OsStr::new("get")) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let config = std::env::var_os(CONFIG_ENV).filter(|v| !v.is_empty());
        let reply = match config {
            Some(file) => answer_from_config(Path::new(&file), &text, now),
            None => {
                let root = PathBuf::from(env_or(ROOT_ENV, GIT_CREDENTIAL_ROOT));
                let base = env_or(BASE_ENV, GITHUB_BASE);
                answer(&root, &base, &parse_request(&text), now)
            }
        };
        let _ = output.write_all(reply.as_bytes());
    }
    0
}

/// The answer from the configuration file `file`: the token it chooses for the repository, else the
/// helpers the session's git named before our reset (host and SSH daemons, research D7), else
/// nothing. A file that cannot be read answers nothing.
fn answer_from_config(file: &Path, request_text: &str, now: UnixTime) -> String {
    let fallback = || {
        let helpers = std::env::var(FALLBACK_ENV).ok();
        crate::credential_config::run_fallback(
            &crate::credential_config::fallback_helpers(helpers.as_deref()),
            request_text,
        )
    };
    let Ok(loaded) = crate::config_load::load_file(file) else {
        return fallback();
    };
    let cache = loaded
        .config
        .daemon
        .data_dir
        .clone()
        .or_else(micold_core::paths::data_dir)
        .map(|dir| dir.join(crate::credential_config::REACH_FILE));
    let chosen = crate::credential_config::answer(
        &loaded.config,
        cache.as_deref(),
        &parse_request(request_text),
        now,
        &crate::credential_config::UreqTransport::default(),
    );
    chosen.unwrap_or_else(fallback)
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
