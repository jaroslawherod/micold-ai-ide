//! `micold-daemon config check <file> [--kind <kind>] [--files]`: validates a `daemon.toml` and
//! prints what the daemon would run with (contracts/daemon-config.md §4).
//!
//! Exit 0 prints the resolved configuration, each secret as `path (readable, 0600)` or
//! `path (missing)`, never its content. Exit 2 prints `file:line:column: message` for each error.
//! Like `status-line` and `git-credential` it runs before any runtime, log or singleton.
//!
//! The files a configuration names are not required to exist unless `--files` is given: the
//! application runs the same check before it delivers them.

use std::ffi::OsString;
use std::io::Write;

use micold_core::daemon_config::{self, ConfigError, Severity};

const USAGE: &str =
    "usage: micold-daemon config check <file> [--kind host|ssh|container|kubernetes] [--files]";

/// Runs `config <args>` (the arguments after `config`) and returns the exit status.
pub fn main(args: &[OsString]) -> i32 {
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    run(args, &mut out, &mut err)
}

/// [`main`] with its output streams injected.
pub fn run(args: &[OsString], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let args: Vec<String> = args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    if args.first().map(String::as_str) != Some("check") {
        let _ = writeln!(err, "{USAGE}");
        return 2;
    }
    let mut file = None;
    let mut kind = None;
    let mut files = false;
    let mut rest = args[1..].iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--kind" => match rest.next().map(|k| k.parse::<daemon_config::Kind>()) {
                Some(Ok(k)) => kind = Some(k),
                Some(Err(message)) => {
                    let _ = writeln!(err, "micold-daemon: {message}");
                    return 2;
                }
                None => {
                    let _ = writeln!(err, "{USAGE}");
                    return 2;
                }
            },
            "--files" => files = true,
            other if other.starts_with("--") || file.is_some() => {
                let _ = writeln!(err, "micold-daemon: unexpected argument {other}\n{USAGE}");
                return 2;
            }
            other => file = Some(other.to_string()),
        }
    }
    let Some(file) = file else {
        let _ = writeln!(err, "{USAGE}");
        return 2;
    };
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(e) => {
            let _ = writeln!(err, "{file}:1:1: cannot read the file: {e}");
            return 2;
        }
    };
    let (config, mut findings) = daemon_config::diagnose(&text, &file);
    if let Some(config) = &config {
        if let Some(expected) = kind.filter(|k| *k != config.daemon.kind) {
            findings.push(ConfigError {
                origin: file.clone(),
                line: 1,
                column: 1,
                rule: "kind",
                key: "daemon.kind".to_string(),
                message: format!(
                    "`daemon.kind` is \"{}\" but the file is checked as \"{}\"",
                    config.daemon.kind.as_str(),
                    expected.as_str()
                ),
                severity: Severity::Error,
            });
        }
        if files {
            findings.extend(daemon_config::check_files(&text, &file, config));
        }
    }
    for finding in &findings {
        let _ = writeln!(err, "{finding}");
    }
    let failed = findings.iter().any(|f| f.severity == Severity::Error);
    match config {
        Some(config) if !failed => {
            let _ = write!(out, "{}", daemon_config::resolved_view(&config));
            0
        }
        _ => 2,
    }
}
