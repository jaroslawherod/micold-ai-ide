//! `micold-daemon git-credential <get|store|erase>`: the credential helper git inside a sandbox
//! runs (#1002, FR-006, FR-007, contracts/git-in-container.md §2).
//!
//! It answers `get` from the in-memory file of the one repository the container holds a credential
//! for, and nothing for any other repository or host. `store` and `erase` read and discard their
//! input: a token is never kept anywhere git could write it. Every path exits 0, so git falls
//! through to "no credential" rather than to an error.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use micold_core::github::GithubRepo;
use micold_core::protocol::messages::CredentialTarget;
use micold_core::scoped_credential::helper::{
    answer, parse_request, render_credential_file, GIT_CREDENTIAL_ROOT,
};
use micold_core::scoped_credential::{SecretToken, UnixTime, CLOCK_SKEW_SECS, GITHUB_BASE, MAX_LIFETIME_SECS};

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
/// override [`ROOT_ENV`], else the default root in a container or when the runtime mounted an
/// authentication token. A host-process daemon holds no credential and refuses every grant.
pub fn credential_root() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os(ROOT_ENV).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(root));
    }
    let sandboxed = !crate::state::image_reference().is_empty()
        || std::env::var_os(micold_core::protocol::auth::TOKEN_PATH_ENV).is_some();
    sandboxed.then(|| PathBuf::from(GIT_CREDENTIAL_ROOT))
}

/// The file of `repo` under `root`: lower case, as the helper reads it.
fn file_of(root: &Path, repo: &GithubRepo) -> PathBuf {
    root.join(repo.owner().to_ascii_lowercase())
        .join(repo.name().to_ascii_lowercase())
}

/// Write `repo`'s credential atomically (a temporary file in the same directory, mode `0600`, then
/// a rename), replacing any earlier one.
pub fn write_grant(
    root: &Path,
    repo: &GithubRepo,
    expires_at: UnixTime,
    token: &SecretToken,
) -> std::io::Result<()> {
    let file = file_of(root, repo);
    let dir = file.parent().expect("a credential file has a directory");
    std::fs::create_dir_all(dir)?;
    // Unique per write: grants for two repositories of one owner may be written at once.
    static WRITES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temporary = dir.join(format!(
        ".{}.{}.tmp",
        std::process::id(),
        WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let written = options.open(&temporary).and_then(|mut f| {
        f.write_all(render_credential_file(expires_at, token.expose()).as_bytes())?;
        f.sync_all()
    });
    match written.and_then(|()| std::fs::rename(&temporary, &file)) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&temporary);
            Err(e)
        }
    }
}

/// Remove `repo`'s credential file; none there is fine.
pub fn remove_grant(root: &Path, repo: &GithubRepo) -> std::io::Result<()> {
    match std::fs::remove_file(file_of(root, repo)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// The version in `git --version` when it is older than 2.31 (no `GIT_CONFIG_COUNT`, so the
/// helper reset would not apply); `None` when it is new enough or cannot be told.
pub fn git_too_old() -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("--version")
        .output()
        .ok()?;
    too_old(&String::from_utf8_lossy(&out.stdout))
}

fn too_old(text: &str) -> Option<String> {
    let version = text.split_whitespace().nth(2)?;
    let mut parts = version.split('.').map(|p| p.parse::<u32>().ok());
    let (major, minor) = (parts.next()??, parts.next()??);
    ((major, minor) < (2, 31)).then(|| version.to_string())
}

/// What each of `projects` pushes to: `git remote get-url --push origin`, verbatim, `None` when
/// there is no `origin` or the folder is no repository. Blocking.
pub fn collect_targets(projects: &[PathBuf]) -> Vec<CredentialTarget> {
    projects
        .iter()
        .map(|project| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(project)
                .args(["remote", "get-url", "--push", "origin"])
                .output();
            let push_url = out
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|u| !u.is_empty());
            CredentialTarget {
                project: project.clone(),
                push_url,
            }
        })
        .collect()
}

/// Why a grant is not written.
#[derive(Debug)]
pub enum GrantRefused {
    /// A host-process daemon holds no credential.
    NotSandboxed,
    /// The container's git is older than 2.31 (the version).
    GitTooOld(String),
    /// None of the daemon's projects pushes to the repository.
    NotAProjectRepo,
    /// The token or its expiry cannot be a credential.
    Invalid,
    /// The file could not be written.
    Write(std::io::Error),
}

/// Write a grant when it is allowed: sandboxed, git new enough, and `repo` one of the repositories
/// `projects` push to (a client bug cannot widen the daemon's reach). Blocking.
pub fn apply_grant(
    projects: &[PathBuf],
    repo: &GithubRepo,
    token: &SecretToken,
    expires_at: UnixTime,
) -> Result<(), GrantRefused> {
    let root = credential_root().ok_or(GrantRefused::NotSandboxed)?;
    if let Some(version) = git_too_old() {
        return Err(GrantRefused::GitTooOld(version));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let plain = !token.expose().is_empty() && token.expose().bytes().all(|b| b.is_ascii_graphic());
    if !plain || expires_at <= now || expires_at > now + MAX_LIFETIME_SECS + CLOCK_SKEW_SECS {
        return Err(GrantRefused::Invalid);
    }
    let want = repo.lowercased();
    let pushes_there = collect_targets(projects)
        .iter()
        .any(|t| t.repo().is_ok_and(|r| r.lowercased() == want));
    if !pushes_there {
        return Err(GrantRefused::NotAProjectRepo);
    }
    write_grant(&root, repo, expires_at, token).map_err(GrantRefused::Write)
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
    fn only_a_git_older_than_2_31_is_too_old() {
        assert_eq!(too_old("git version 2.30.0\n").as_deref(), Some("2.30.0"));
        assert_eq!(too_old("git version 1.9.5").as_deref(), Some("1.9.5"));
        assert_eq!(too_old("git version 2.31.0"), None);
        assert_eq!(too_old("git version 2.43.0.windows.1"), None);
        assert_eq!(too_old("git version 3.0.0"), None);
        assert_eq!(too_old("nonsense"), None);
    }

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
