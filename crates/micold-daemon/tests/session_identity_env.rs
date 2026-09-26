//! A session's environment names micold, not the terminal the daemon was started from (feature
//! 031, FR-006, contract session-terminal-identity §2, U68–U71).
//!
//! The inherited environment belongs to a re-executed copy of this test binary, never to this
//! process through `std::env::set_var`: the child is started with the daemon-side variables a
//! scenario names, spawns a real session through `PtySession::spawn_shell` and
//! `PtySession::spawn_ai_cli`, and each session runs a stand-in that writes its environment to a
//! file. The stand-in is put first on `PATH` as the AI CLI's command, and set as `SHELL` (or
//! `COMSPEC` on Windows) for the shell, so no real CLI or shell configuration is involved.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use micold_core::env_include::merge_with_term;
use micold_core::session::{AiCli, SessionId};
use micold_core::terminal::{LaunchMode, LaunchSpec};
use micold_daemon::supervisor::PtySession;
use micold_daemon::terminal::TerminalColors;

/// Set on the re-executed child: the directory holding the stand-in and the environment dumps.
const CHILD_DIR: &str = "MICOLD_IDENTITY_CHILD_DIR";
/// Set on the re-executed child: the include-script values, as `KEY=VALUE` joined by `;`.
const CHILD_INCLUDE: &str = "MICOLD_IDENTITY_CHILD_INCLUDE";
/// Passed to each session through its include values: the file its stand-in writes.
const DUMP: &str = "MICOLD_IDENTITY_DUMP";

/// The variables a scenario may set on the daemon side; each run starts with all of them removed,
/// so the machine running the suite cannot leak its own terminal's identity in.
const SCENARIO_KEYS: [&str; 3] = ["TERM_PROGRAM", "FORCE_HYPERLINK", "COLORTERM"];

/// What the two sessions of one child saw.
struct Seen {
    shell: HashMap<String, String>,
    ai_cli: HashMap<String, String>,
}

impl Seen {
    fn both(&self) -> [(&'static str, &HashMap<String, String>); 2] {
        [("spawn_shell", &self.shell), ("spawn_ai_cli", &self.ai_cli)]
    }
}

/// The stand-in a session runs: it writes its own environment to `MICOLD_IDENTITY_DUMP`, whatever
/// arguments it is given, since `spawn_ai_cli` passes the provider's launch arguments.
///
/// On Unix it is a `sh` script. On Windows it is **compiled**, because a session's program is
/// started with `CreateProcessW`, which refuses a `.cmd` (`%1 is not a valid Win32 application`,
/// os error 193) — a batch file is not an executable image, and there is no room to wrap it in
/// `cmd.exe /C`: `spawn_shell` runs `COMSPEC` with no arguments of its own.
#[cfg(not(windows))]
const STAND_IN: &str = "claude";
#[cfg(windows)]
const STAND_IN: &str = "claude.exe";

#[cfg(not(windows))]
fn write_stand_in(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(STAND_IN);
    std::fs::write(
        &path,
        "#!/bin/sh\n\
         env > \"$MICOLD_IDENTITY_DUMP.tmp\" && mv \"$MICOLD_IDENTITY_DUMP.tmp\" \"$MICOLD_IDENTITY_DUMP\"\n",
    )
    .expect("write the stand-in");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("make the stand-in executable");
    path
}

#[cfg(windows)]
fn write_stand_in(dir: &Path) -> PathBuf {
    let path = dir.join(STAND_IN);
    std::fs::copy(compiled_stand_in(), &path).expect("copy the stand-in next to the dumps");
    path
}

/// Compile the stand-in once per test process and hand out its path.
#[cfg(windows)]
fn compiled_stand_in() -> &'static Path {
    use std::sync::OnceLock;

    /// Prints nothing and parses nothing: it writes its environment where it was told, then
    /// renames, so a reader never sees a half-written dump.
    const SOURCE: &str = r#"
fn main() {
    let dump = std::env::var("MICOLD_IDENTITY_DUMP").expect("MICOLD_IDENTITY_DUMP");
    let mut out = String::new();
    for (key, value) in std::env::vars_os() {
        out.push_str(&format!("{}={}\n", key.to_string_lossy(), value.to_string_lossy()));
    }
    let partial = format!("{dump}.tmp");
    std::fs::write(&partial, out).expect("write the dump");
    std::fs::rename(&partial, &dump).expect("publish the dump");
}
"#;

    static STAND_IN_EXE: OnceLock<PathBuf> = OnceLock::new();
    STAND_IN_EXE.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("micold-identity-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory for the stand-in");
        let source = dir.join("stand_in.rs");
        std::fs::write(&source, SOURCE).expect("write the stand-in's source");
        let exe = dir.join(STAND_IN);
        // `rustc` beside the `cargo` that is running this test, so a toolchain that is not on
        // `PATH` still resolves.
        let rustc = std::env::var_os("RUSTC")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("CARGO")
                    .map(PathBuf::from)
                    .and_then(|cargo| cargo.parent().map(|dir| dir.join("rustc.exe")))
                    .filter(|rustc| rustc.exists())
            })
            .unwrap_or_else(|| PathBuf::from("rustc"));
        let out = Command::new(&rustc)
            .arg(&source)
            .arg("-o")
            .arg(&exe)
            .output()
            .unwrap_or_else(|err| panic!("run {}: {err}", rustc.display()));
        assert!(
            out.status.success(),
            "compile the stand-in with {}: {}",
            rustc.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        exe
    })
}

/// Re-execute `test` with `inherited` in its environment and `include` as the sessions' include
/// values, and return what each session saw.
fn sessions_seeing(test: &str, inherited: &[(&str, &str)], include: &[(&str, &str)]) -> Seen {
    let inherited = inherited
        .iter()
        .map(|(k, v)| (OsString::from(k), OsString::from(v)))
        .collect();
    sessions_seeing_os(test, inherited, include)
}

/// [`sessions_seeing`], for an inherited environment that need not be UTF-8.
fn sessions_seeing_os(
    test: &str,
    inherited: Vec<(OsString, OsString)>,
    include: &[(&str, &str)],
) -> Seen {
    let dir = tempfile::tempdir().expect("a temp dir");
    let stand_in = write_stand_in(dir.path());

    let mut path = OsString::from(dir.path());
    if let Some(rest) = std::env::var_os("PATH") {
        path.push(if cfg!(windows) { ";" } else { ":" });
        path.push(rest);
    }
    let mut child = Command::new(std::env::current_exe().expect("the test binary"));
    child.args([test, "--exact", "--nocapture", "--test-threads=1"]);
    for key in SCENARIO_KEYS {
        child.env_remove(key);
    }
    child
        .envs(inherited)
        .env(CHILD_DIR, dir.path())
        .env(
            CHILD_INCLUDE,
            include
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(";"),
        )
        .env("PATH", path)
        .env(if cfg!(windows) { "COMSPEC" } else { "SHELL" }, &stand_in);
    let out = child.output().expect("run the child");
    assert!(
        out.status.success(),
        "the child spawning the sessions failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Seen {
        shell: parse_dump(&dir.path().join("shell.env")),
        ai_cli: parse_dump(&dir.path().join("ai_cli.env")),
    }
}

fn parse_dump(path: &Path) -> HashMap<String, String> {
    String::from_utf8_lossy(&std::fs::read(path).expect("the session's environment dump"))
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.to_string(), v.trim_end_matches('\r').to_string()))
        .collect()
}

/// The child's side: when this process is a re-executed child, spawn both sessions, wait for
/// their dumps, and return `true` so the calling test returns at once.
fn child_side() -> bool {
    let Some(dir) = std::env::var_os(CHILD_DIR).map(PathBuf::from) else {
        return false;
    };
    let include: Vec<(String, String)> = std::env::var(CHILD_INCLUDE)
        .unwrap_or_default()
        .split(';')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let env_for = |dump: &Path| {
        let mut env = merge_with_term(&include);
        env.push((DUMP.to_string(), dump.to_string_lossy().into_owned()));
        env
    };

    let shell_dump = dir.join("shell.env");
    let shell = PtySession::spawn_shell(
        SessionId::new(),
        &dir,
        &env_for(&shell_dump),
        1000,
        Some((80, 24)),
        &TerminalColors::default(),
    )
    .expect("spawn the shell session");
    wait_for(&shell_dump);
    let _ = shell.kill();

    let ai_dump = dir.join("ai_cli.env");
    let spec = LaunchSpec {
        cwd: dir.clone(),
        session_id: uuid::Uuid::new_v4(),
        provider: AiCli::ClaudeCode,
        mode: LaunchMode::Fresh,
        env: env_for(&ai_dump),
    };
    let ai = PtySession::spawn_ai_cli(
        SessionId::new(),
        &spec,
        1000,
        Some((80, 24)),
        &[],
        &TerminalColors::default(),
    )
    .expect("spawn the AI CLI session");
    wait_for(&ai_dump);
    let _ = ai.kill();
    true
}

fn wait_for(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "the session's stand-in never wrote {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn a_session_does_not_see_the_inherited_terminal_identity() {
    if child_side() {
        return;
    }
    let seen = sessions_seeing(
        "a_session_does_not_see_the_inherited_terminal_identity",
        &[("TERM_PROGRAM", "WezTerm"), ("FORCE_HYPERLINK", "1")],
        &[],
    );
    for (spawn, env) in seen.both() {
        for key in ["TERM_PROGRAM", "FORCE_HYPERLINK"] {
            assert_eq!(
                env.get(key),
                None,
                "a {spawn} session must not inherit {key} from the terminal the daemon was \
                 started in (FR-006)"
            );
        }
    }
}

#[test]
fn the_include_script_can_opt_a_session_into_hyperlinks() {
    if child_side() {
        return;
    }
    let test = "the_include_script_can_opt_a_session_into_hyperlinks";
    let opt_in = [("FORCE_HYPERLINK", "1")];
    for (case, inherited) in [
        ("set only by the include script", &[][..]),
        ("also inherited", &opt_in[..]),
    ] {
        let seen = sessions_seeing(test, inherited, &opt_in);
        for (spawn, env) in seen.both() {
            assert_eq!(
                env.get("FORCE_HYPERLINK").map(String::as_str),
                Some("1"),
                "FORCE_HYPERLINK=1 from the include script is the documented opt-in, so a {spawn} \
                 session sees it when it is {case} (contract §2)"
            );
        }
    }
}

#[test]
fn an_inherited_colour_depth_is_kept() {
    if child_side() {
        return;
    }
    let seen = sessions_seeing(
        "an_inherited_colour_depth_is_kept",
        &[("COLORTERM", "truecolor")],
        &[],
    );
    for (spawn, env) in seen.both() {
        assert_eq!(
            env.get("COLORTERM").map(String::as_str),
            Some("truecolor"),
            "COLORTERM=truecolor describes colour depth, not a terminal, so a {spawn} session \
             keeps it"
        );
    }
}

#[test]
fn term_stays_xterm_256color() {
    if child_side() {
        return;
    }
    let seen = sessions_seeing(
        "term_stays_xterm_256color",
        &[("TERM", "wezterm"), ("TERM_PROGRAM", "WezTerm")],
        &[],
    );
    for (spawn, env) in seen.both() {
        assert_eq!(
            env.get("TERM").map(String::as_str),
            Some("xterm-256color"),
            "a {spawn} session's TERM is pinned as before (contract §4)"
        );
    }
}

/// An identity variable is dropped even when its value is not UTF-8 (review A, M4). Unix only: a
/// Windows environment is UTF-16, where the same bytes cannot be set.
#[cfg(unix)]
#[test]
fn an_identity_variable_that_is_not_utf8_is_dropped_too() {
    use std::os::unix::ffi::OsStringExt;
    if child_side() {
        return;
    }
    let seen = sessions_seeing_os(
        "an_identity_variable_that_is_not_utf8_is_dropped_too",
        vec![(
            OsString::from("TERMINAL_EMULATOR"),
            OsString::from_vec(b"JetBrains-\xff".to_vec()),
        )],
        &[],
    );
    for (spawn, env) in seen.both() {
        assert_eq!(
            env.get("TERMINAL_EMULATOR"),
            None,
            "a {spawn} session must not inherit TERMINAL_EMULATOR, whatever its bytes (FR-006)"
        );
    }
}
