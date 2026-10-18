//! Shared by the structured-session tests (feature 938, research R12): a stand-in `claude` that
//! speaks stream-json over pipes, compiled and put first on `PATH` the way `support/history.rs`'s
//! `fake_cli` is, and helpers to script it and read what it recorded.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use micold_core::session::AiCli;

#[cfg(not(windows))]
const STAND_IN: &str = "claude";
#[cfg(windows)]
const STAND_IN: &str = "claude.exe";

/// The stand-in `claude`. At start it writes, in its cwd, its arguments to `argv-record` (one per
/// line), its cwd to `cwd-record` and `TERM`, `PATH` and every `MICOLD_*` variable to
/// `env-record` (`NAME=value`, sorted; `NAME` alone when unset), and from then on appends every
/// stdin line to `stdin-record`. Then it runs `.fake-stream`, one line at a time:
///
/// - `replay <file>` writes the file's bytes to stdout line by line, as they are, a trailing
///   partial line included;
/// - `pause <ms>` sleeps; `await-stdin` blocks until one more stdin line arrived;
/// - `stderr <text>` writes the text and a line end to stderr; `exit <code>` exits;
/// - any other line is written to stdout as it is, with a line end.
///
/// When the script ends it idles until killed.
const STAND_IN_SOURCE: &str = r##"
use std::io::{BufRead, Write};
use std::sync::mpsc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let _ = std::fs::write("argv-record", args.join("\n") + "\n");
    let cwd = std::env::current_dir().map(|d| d.display().to_string()).unwrap_or_default();
    let _ = std::fs::write("cwd-record", cwd + "\n");
    let mut env: Vec<String> = std::env::vars()
        .filter(|(k, _)| k.starts_with("MICOLD_"))
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    for name in ["TERM", "PATH"] {
        env.push(match std::env::var(name) {
            Ok(v) => format!("{name}={v}"),
            Err(_) => name.to_string(),
        });
    }
    env.sort();
    let _ = std::fs::write("env-record", env.join("\n") + "\n");

    let (lines_tx, lines_rx) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("stdin-record")
            {
                let _ = writeln!(f, "{line}");
            }
            let _ = lines_tx.send(());
        }
    });

    let script = std::fs::read_to_string(".fake-stream").unwrap_or_default();
    let mut out = std::io::stdout();
    for line in script.lines() {
        let (cmd, arg) = line.split_once(' ').unwrap_or((line, ""));
        match cmd {
            "replay" => {
                let bytes = std::fs::read(arg).expect("read the fixture");
                for piece in bytes.split_inclusive(|b| *b == b'\n') {
                    let _ = out.write_all(piece);
                    let _ = out.flush();
                }
            }
            "pause" => std::thread::sleep(std::time::Duration::from_millis(arg.parse().unwrap_or(0))),
            "await-stdin" => {
                let _ = lines_rx.recv();
            }
            "stderr" => {
                let _ = writeln!(std::io::stderr(), "{arg}");
            }
            "exit" => {
                let _ = out.flush();
                std::process::exit(arg.parse().unwrap_or(0));
            }
            _ => {
                let _ = writeln!(out, "{line}");
            }
        }
        let _ = out.flush();
    }
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
"##;

/// Compile the stand-in and put its directory first on `PATH`, once per test process. Every test
/// calls this first, so the one change of the process-wide variables happens before any test reads
/// them.
pub fn fake_stream_cli() {
    static READY: OnceLock<PathBuf> = OnceLock::new();
    READY.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("micold-938-stream-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory for the stand-in");
        let source = dir.join("stand_in.rs");
        std::fs::write(&source, STAND_IN_SOURCE).expect("write the stand-in's source");
        let exe = dir.join(STAND_IN);
        let rustc_name = if cfg!(windows) { "rustc.exe" } else { "rustc" };
        let rustc = std::env::var_os("RUSTC")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("CARGO")
                    .map(PathBuf::from)
                    .and_then(|cargo| cargo.parent().map(|dir| dir.join(rustc_name)))
                    .filter(|rustc| rustc.exists())
            })
            .unwrap_or_else(|| PathBuf::from("rustc"));
        let out = std::process::Command::new(&rustc)
            .args(["--edition", "2021"])
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
        let mut path = vec![dir.clone()];
        path.extend(
            std::env::var_os("PATH")
                .iter()
                .flat_map(std::env::split_paths),
        );
        std::env::set_var("PATH", std::env::join_paths(path).unwrap());
        #[cfg(unix)]
        std::env::set_var("SHELL", "/bin/sh");
        assert!(AiCli::ClaudeCode
            .provider()
            .is_available(&micold_core::provider::process_path()));
        dir
    });
}

/// A recorded fixture of `micold-core/tests/fixtures/stream-json`.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../micold-core/tests/fixtures/stream-json")
        .join(name)
}

/// The five recorded fixtures (SC-001).
pub const RECORDED: [&str; 5] = [
    "text-turn.jsonl",
    "tool-turn.jsonl",
    "partial-deltas.jsonl",
    "error-result.jsonl",
    "resume-init.jsonl",
];

/// What the stand-in does at its next start in `dir`.
pub fn stream_script(dir: &Path, directives: &str) {
    std::fs::write(dir.join(".fake-stream"), directives).expect("write .fake-stream");
}

/// A record the stand-in wrote in `dir`, once it exists and is complete, within 10 s.
pub fn record(dir: &Path, name: &str) -> String {
    let file = dir.join(name);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(text) = std::fs::read_to_string(&file) {
            if text.ends_with('\n') {
                return text;
            }
        }
        assert!(
            Instant::now() < deadline,
            "{} never appeared",
            file.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The fixture's lines as the parser sees them: every complete line, then a trailing partial one.
pub fn fixture_lines(name: &str) -> Vec<String> {
    let bytes = std::fs::read(fixture(name)).expect("read the fixture");
    let mut splitter = micold_core::structured::LineSplitter::default();
    let mut lines = splitter.push(&bytes);
    lines.extend(splitter.finish());
    lines
}
