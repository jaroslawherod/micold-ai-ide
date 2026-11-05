//! #1531: `micold-daemon gh-token --cwd <dir>` prints the token chosen for the cwd's `origin`
//! through the Issue 9 chooser (T055), and the `gh` shim it backs sets `GH_TOKEN`/`GH_HOST` for
//! the real `gh` only when a token was chosen, leaving either alone if the session's own `gh`
//! already set it (T056, T095).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Setup {
    dir: tempfile::TempDir,
}

impl Setup {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn token(&self, name: &str, content: &str) {
        std::fs::write(self.path(name), content).unwrap();
    }

    /// A host daemon's configuration file with `tokens` as `(name, host, repos filter)`.
    fn config(&self, tokens: &[(&str, &str, &[&str])]) -> PathBuf {
        let mut text = format!(
            "schema = 1\n\n[daemon]\nname = \"t\"\nkind = \"host\"\ndata_dir = \"{}\"\n",
            self.path("data").display()
        );
        for (name, host, repos) in tokens {
            text.push_str(&format!(
                "\n[[github.tokens]]\nname = \"{name}\"\ntoken_file = \"{}\"\nhost = \"{host}\"\n",
                self.path(&format!("{name}.tok")).display(),
            ));
            if !repos.is_empty() {
                let list: Vec<String> = repos.iter().map(|r| format!("\"{r}\"")).collect();
                text.push_str(&format!("repos = [{}]\n", list.join(", ")));
            }
        }
        let path = self.path("daemon.toml");
        std::fs::create_dir_all(self.path("data")).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    /// A git repository at `name` whose `origin` is `url`; no network, no clone.
    fn repo(&self, name: &str, url: &str) -> PathBuf {
        let dir = self.path(name);
        std::fs::create_dir_all(&dir).unwrap();
        for args in [vec!["init", "-q"], vec!["remote", "add", "origin", url]] {
            assert!(Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(&args)
                .status()
                .unwrap()
                .success());
        }
        dir
    }

    /// `gh-token --cwd <dir>`, with `config` as `MICOLD_DAEMON_CONFIG` (`None`: unset, as outside
    /// a daemon's session).
    fn gh_token(&self, config: Option<&Path>, dir: &Path) -> (i32, String) {
        let mut command = Command::new(env!("CARGO_BIN_EXE_micold-daemon"));
        command.args(["gh-token", "--cwd"]).arg(dir);
        match config {
            Some(c) => command.env("MICOLD_DAEMON_CONFIG", c),
            None => command.env_remove("MICOLD_DAEMON_CONFIG"),
        };
        let out = command.output().unwrap();
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8(out.stdout).unwrap(),
        )
    }
}

#[test]
fn gh_token_prints_the_token_the_chooser_picks_for_the_cwds_origin() {
    let s = Setup::new();
    s.token("a.tok", "ghp_abc\n");
    let config = s.config(&[("a", "github.com", &["acme/app"])]);
    let repo = s.repo("app", "https://github.com/acme/app.git");
    let (code, out) = s.gh_token(Some(&config), &repo);
    assert_eq!(code, 0);
    let mut lines = out.lines();
    assert_eq!(lines.next(), Some("ghp_abc"));
    assert_eq!(lines.next(), Some("github.com"));
}

#[test]
fn gh_token_exits_1_silently_with_no_token_chosen() {
    let s = Setup::new();
    s.token("a.tok", "ghp_abc\n");
    // Filtered to a repository this one is not.
    let config = s.config(&[("a", "github.com", &["acme/app"])]);
    let repo = s.repo("other", "https://github.com/other/repo.git");
    let (code, out) = s.gh_token(Some(&config), &repo);
    assert_eq!((code, out.as_str()), (1, ""));
}

#[test]
fn gh_token_exits_1_silently_with_no_configuration() {
    let s = Setup::new();
    let repo = s.repo("app", "https://github.com/acme/app.git");
    let (code, out) = s.gh_token(None, &repo);
    assert_eq!((code, out.as_str()), (1, ""));
}

#[test]
fn gh_token_matches_a_non_default_host_for_a_github_enterprise_token() {
    let s = Setup::new();
    s.token("a.tok", "ghe_tok\n");
    let config = s.config(&[("a", "ghe.example.com", &["acme/app"])]);
    let repo = s.repo("app", "ssh://git@ghe.example.com:2222/acme/app.git");
    let (code, out) = s.gh_token(Some(&config), &repo);
    assert_eq!(code, 0);
    let mut lines = out.lines();
    assert_eq!(lines.next(), Some("ghe_tok"));
    assert_eq!(lines.next(), Some("ghe.example.com"));
}

/// A fake `gh` that prints `GH_TOKEN=<...>` and `GH_HOST=<...>` (each `unset` if absent) and exits
/// with a code no real `gh` or the shim's own fallback (`1`) would produce, so the test can tell
/// the shim's `exec` really replaced the process image rather than, say, silently swallowing it.
fn fake_gh(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let file = dir.join("gh");
    std::fs::write(
        &file,
        "#!/bin/sh\n\
         printf 'GH_TOKEN=%s\\n' \"${GH_TOKEN:-unset}\"\n\
         printf 'GH_HOST=%s\\n' \"${GH_HOST:-unset}\"\n\
         exit 42\n",
    )
    .unwrap();
    std::fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    dir.to_path_buf()
}

/// Runs the shim `write_shim` wrote in `bin_dir`, with `fake_gh_dir` next on `PATH` (standing in
/// for the real `gh`), `cwd` and `extra_env` (e.g. a pre-set `GH_TOKEN`, T056's "leaves an
/// existing one alone"). Returns the fake `gh`'s exit code and what it printed.
fn run_shim(
    bin_dir: &Path,
    fake_gh_dir: &Path,
    cwd: &Path,
    config: Option<&Path>,
    extra_env: &[(&str, &str)],
) -> (i32, String) {
    let path = format!(
        "{}:{}:{}",
        bin_dir.display(),
        fake_gh_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut command = Command::new(bin_dir.join("gh"));
    command
        .current_dir(cwd)
        .env("PATH", path)
        .env_remove("GH_TOKEN")
        .env_remove("GH_HOST")
        .stdin(Stdio::null());
    match config {
        Some(c) => command.env("MICOLD_DAEMON_CONFIG", c),
        None => command.env_remove("MICOLD_DAEMON_CONFIG"),
    };
    for (k, v) in extra_env {
        command.env(k, v);
    }
    let out = command.output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
    )
}

#[test]
fn the_shim_execs_the_real_gh_with_gh_token_and_gh_host_set_from_the_chosen_token() {
    let s = Setup::new();
    let bin_dir = micold_daemon::gh_shim::write_shim(
        &s.path("bin"),
        Path::new(env!("CARGO_BIN_EXE_micold-daemon")),
    )
    .unwrap();
    let fake_gh_dir = fake_gh(&s.path("fake-gh"));
    s.token("a.tok", "ghp_abc\n");
    let config = s.config(&[("a", "github.com", &["acme/app"])]);
    let repo = s.repo("app", "https://github.com/acme/app.git");

    let (code, out) = run_shim(&bin_dir, &fake_gh_dir, &repo, Some(&config), &[]);
    assert_eq!(code, 42, "the shim exec'd the real gh, not a copy of itself");
    assert!(out.contains("GH_TOKEN=ghp_abc"), "{out}");
    assert!(out.contains("GH_HOST=github.com"), "{out}");
}

#[test]
fn the_shim_leaves_an_existing_gh_token_and_gh_host_alone() {
    let s = Setup::new();
    let bin_dir = micold_daemon::gh_shim::write_shim(
        &s.path("bin"),
        Path::new(env!("CARGO_BIN_EXE_micold-daemon")),
    )
    .unwrap();
    let fake_gh_dir = fake_gh(&s.path("fake-gh"));
    s.token("a.tok", "ghp_abc\n");
    let config = s.config(&[("a", "github.com", &["acme/app"])]);
    let repo = s.repo("app", "https://github.com/acme/app.git");

    let (code, out) = run_shim(
        &bin_dir,
        &fake_gh_dir,
        &repo,
        Some(&config),
        &[("GH_TOKEN", "preset"), ("GH_HOST", "preset.example.com")],
    );
    assert_eq!(code, 42);
    assert!(out.contains("GH_TOKEN=preset"), "{out}");
    assert!(out.contains("GH_HOST=preset.example.com"), "{out}");
}

#[test]
fn the_shim_leaves_gh_token_and_gh_host_unset_with_no_token_chosen() {
    let s = Setup::new();
    let bin_dir = micold_daemon::gh_shim::write_shim(
        &s.path("bin"),
        Path::new(env!("CARGO_BIN_EXE_micold-daemon")),
    )
    .unwrap();
    let fake_gh_dir = fake_gh(&s.path("fake-gh"));
    let repo = s.repo("app", "https://github.com/acme/app.git");

    let (code, out) = run_shim(&bin_dir, &fake_gh_dir, &repo, None, &[]);
    assert_eq!(code, 42);
    assert!(out.contains("GH_TOKEN=unset"), "{out}");
    assert!(out.contains("GH_HOST=unset"), "{out}");
}
