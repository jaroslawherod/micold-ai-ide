//! `micold-daemon gh-token --cwd <dir>` and the `gh` shim this writes for every session's `PATH`
//! (#1531, FR-011, FR-035, SC-003, D8, D15 row 7). `gh` has no credential-helper mode, so the shim
//! runs `gh-token` on every call (not once at spawn) and sets `GH_TOKEN`/`GH_HOST` for just that
//! call: a rotated token is used by the very next `gh` command, the way `git-credential get`
//! already is (`crate::credential_config::answer`, Issue 9's `TokenChoice` chooser). `gh-token`
//! resolves the repository through the same chooser and request shape `git-credential get` answers
//! from, built from `<dir>`'s `origin` instead of from what git sends: [`origin_repo`] parses the
//! remote generically (any forge, not only github.com, since a token's `host` may be a GitHub
//! Enterprise host).

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use micold_core::github::split_remote;
use micold_core::process::no_window;
use micold_core::scoped_credential::helper::CredentialRequest;
use micold_core::scoped_credential::UnixTime;

/// The host, owner and name of `dir`'s `origin` remote (its fetch URL), lower-cased; `None` with no
/// `origin`, no repository there, or a URL no form git accepts parses (`split_remote`).
fn origin_repo(dir: &Path) -> Option<(String, String, String)> {
    let out = no_window(&mut Command::new("git"))
        .arg("-C")
        .arg(dir)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let (host, path, _over_ssh) = split_remote(&url)?;
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, name) = path.split_once('/')?;
    if owner.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }
    Some((
        host.to_ascii_lowercase(),
        owner.to_string(),
        name.to_string(),
    ))
}

/// What `gh-token --cwd <dir>` resolves.
pub struct ChosenToken {
    pub token: String,
    /// The bare host to set `GH_HOST` to (D15 row 7): `origin_repo`'s host, which the chooser only
    /// returns a token for when a `[[github.tokens]]` entry's own `host` matches it.
    pub host: String,
}

/// [`ChosenToken`] for `dir`, through the configuration named by
/// [`micold_core::scoped_credential::git_env::CONFIG_ENV`] — the same file and the same chooser
/// `git-credential get` reads (`crate::credential_config::answer`), so a change to the token file
/// is used by the very next call, not cached from session spawn. `None` with no configuration, no
/// `origin`, or no token the chooser picks for it.
pub fn resolve(dir: &Path, now: UnixTime) -> Option<ChosenToken> {
    use micold_core::scoped_credential::git_env::CONFIG_ENV;
    let config_path = std::env::var_os(CONFIG_ENV).filter(|v| !v.is_empty())?;
    let (host, owner, name) = origin_repo(dir)?;
    let loaded = crate::config_load::load_file(Path::new(&config_path)).ok()?;
    let cache = loaded
        .config
        .daemon
        .data_dir
        .clone()
        .or_else(micold_core::paths::data_dir)
        .map(|dir| dir.join(crate::credential_config::REACH_FILE));
    let request = CredentialRequest {
        protocol: Some("https".to_string()),
        host: Some(host.clone()),
        path: Some(format!("{owner}/{name}")),
    };
    let reply = crate::credential_config::answer(
        &loaded.config,
        cache.as_deref(),
        &request,
        now,
        &crate::credential_config::UreqTransport::default(),
    )?;
    let token = reply
        .lines()
        .find_map(|l| l.strip_prefix("password="))?
        .to_string();
    Some(ChosenToken { token, host })
}

/// The unix shim's `sh` script: it asks `gh-token` for `$PWD`, sets `GH_TOKEN`/`GH_HOST` only when
/// a token comes back and only where the session's own git left them unset, then walks `PATH`
/// itself for the first `gh` outside `bin_dir` (not a prefix-strip of `PATH`: something later in a
/// session's own startup — a shell rc, `direnv`, a version manager — may have prepended to `PATH`
/// after spawn, so `bin_dir` is no longer guaranteed to still be a literal prefix; stripping only a
/// literal `"$bin_dir":` prefix would then be a no-op and `exec gh "$@"` would re-resolve to this
/// same shim, recursing) and replaces this process with it — the real `gh`'s exit code is then
/// simply this script's (unlike the Windows batch shim, which cannot replace the process image and
/// must forward it explicitly). No `gh` outside `bin_dir` on `PATH`: reports not found like a shell
/// would (exit 127), same as a session with no `gh` installed at all.
fn unix_shim(daemon: &Path, bin_dir: &Path) -> String {
    format!(
        "#!/bin/sh\n\
         # Written by micold-daemon; do not edit (#1531).\n\
         daemon={}\n\
         bin_dir={}\n\
         out=$(\"$daemon\" gh-token --cwd \"$PWD\" 2>/dev/null)\n\
         if [ -n \"$out\" ]; then\n\
         \ttoken=$(printf '%s\\n' \"$out\" | sed -n '1p')\n\
         \thost=$(printf '%s\\n' \"$out\" | sed -n '2p')\n\
         \t: \"${{GH_TOKEN:=$token}}\"\n\
         \t: \"${{GH_HOST:=$host}}\"\n\
         \texport GH_TOKEN GH_HOST\n\
         fi\n\
         real_gh=\n\
         old_ifs=$IFS\n\
         IFS=:\n\
         for dir in $PATH; do\n\
         \tif [ -z \"$dir\" ]; then\n\
         \t\tdir=.\n\
         \tfi\n\
         \tif [ \"$dir\" = \"$bin_dir\" ]; then\n\
         \t\tcontinue\n\
         \tfi\n\
         \tif [ -z \"$real_gh\" ] && [ -x \"$dir/gh\" ]; then\n\
         \t\treal_gh=\"$dir/gh\"\n\
         \tfi\n\
         done\n\
         IFS=$old_ifs\n\
         if [ -z \"$real_gh\" ]; then\n\
         \techo 'gh: command not found' >&2\n\
         \texit 127\n\
         fi\n\
         exec \"$real_gh\" \"$@\"\n",
        shell_quote(daemon),
        shell_quote(bin_dir),
    )
}

/// `path` quoted for a POSIX shell: single-quoted, with an embedded `'` escaped the way git's own
/// `write_grant`/`helper_command` do elsewhere in this crate.
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}

/// The shim file(s) for this platform under `bin_dir` (created if missing), naming `daemon` (this
/// process's own executable) inside them. `gh`, mode `0755`. Write-if-
/// changed, so a session spawn that finds the content already right touches nothing. Returns
/// `bin_dir`, for the caller to put first on the session's `PATH`.
pub fn write_shim(bin_dir: &Path, daemon: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(bin_dir)?;
    let name = "gh";
    let content = unix_shim(daemon, bin_dir);
    let file = bin_dir.join(name);
    let changed = std::fs::read_to_string(&file).ok().as_deref() != Some(content.as_str());
    if changed {
        let temporary = bin_dir.join(format!(".{name}.{}.tmp", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o755);
        options.open(&temporary)?.write_all(content.as_bytes())?;
        std::fs::rename(&temporary, &file)?;
    }
    Ok(bin_dir.to_path_buf())
}

/// Now, as whole seconds since the epoch (clock before 1970: `0`, same as `git_credential::main`).
fn now_unix() -> UnixTime {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `gh-token --cwd <dir>`: prints the chosen token, then the host for `GH_HOST`, each on its own
/// line; nothing and exit 1 with none (D8, T055). Any other argument shape is also "none".
pub fn main(args: &[OsString]) -> i32 {
    let mut cwd = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == "--cwd" {
            cwd = rest.next().map(PathBuf::from);
        }
    }
    let Some(dir) = cwd else {
        return 1;
    };
    match resolve(&dir, now_unix()) {
        Some(chosen) => {
            println!("{}", chosen.token);
            println!("{}", chosen.host);
            0
        }
        None => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_repo_parses_https_ssh_and_scp_for_any_host() {
        for (url, want) in [
            (
                "https://github.com/acme/app.git",
                ("github.com", "acme", "app"),
            ),
            (
                "ssh://git@ghe.example.com:2222/acme/app",
                ("ghe.example.com", "acme", "app"),
            ),
            (
                "git@GHE.example.com:acme/app.git",
                ("ghe.example.com", "acme", "app"),
            ),
        ] {
            let (host, path, over_ssh) = split_remote(url).unwrap();
            let _ = over_ssh;
            let path = path.trim_end_matches('/');
            let path = path.strip_suffix(".git").unwrap_or(path);
            let (owner, name) = path.split_once('/').unwrap();
            assert_eq!(
                (host.to_ascii_lowercase().as_str(), owner, name),
                want,
                "{url}"
            );
        }
    }

    #[test]
    fn with_no_cwd_argument_main_exits_1() {
        assert_eq!(main(&[]), 1);
        assert_eq!(main(&["--cwd".into()]), 1);
    }

    #[test]
    fn the_unix_shim_leaves_an_existing_gh_token_and_gh_host_alone() {
        let script = unix_shim(Path::new("/opt/micold-daemon"), Path::new("/data/bin"));
        assert!(script.contains(": \"${GH_TOKEN:=$token}\""));
        assert!(script.contains(": \"${GH_HOST:=$host}\""));
        assert!(script.contains("exec \"$real_gh\" \"$@\""));
    }

    /// T151 (#1531, Finding 3): `write_shim`'s doc comment promises write-if-changed — "a session
    /// spawn that finds the content already right touches nothing" — matching the write-if-
    /// changed rule `config_reload.rs`'s `a_reload_never_writes_the_config_or_a_secret` (Issue 5's
    /// U21) already tests for `daemon.toml` delivery. A second call with the same `bin_dir`/
    /// `daemon` args must leave the shim file's content, mtime and inode exactly as the first call
    /// left them.
    #[cfg(unix)]
    #[test]
    fn write_shim_touches_nothing_when_the_content_is_already_right() {
        use std::os::unix::fs::MetadataExt;

        let dir = tempfile::tempdir().unwrap();
        let bin_dir = dir.path().join("bin");
        let daemon = Path::new("/opt/micold-daemon");

        write_shim(&bin_dir, daemon).unwrap();
        let file = bin_dir.join("gh");
        let stamp = |p: &Path| {
            let m = std::fs::metadata(p).unwrap();
            (
                std::fs::read(p).unwrap(),
                m.mtime(),
                m.mtime_nsec(),
                m.ino(),
            )
        };
        let before = stamp(&file);

        write_shim(&bin_dir, daemon).unwrap();

        assert_eq!(
            before,
            stamp(&file),
            "a second write_shim call with the same content must not touch the file"
        );
    }
}
