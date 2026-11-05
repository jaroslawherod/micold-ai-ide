//! `micold-daemon ai-key <cli>`: Claude Code's `apiKeyHelper` command (#1532, contract row
//! `api_key_file`). Run by `claude` itself, as a short-lived child of the session, on each use —
//! never once at session start — so a replaced key file reaches a session already running
//! (FR-013, SC-008). Same reasons as `status-line` and `git-credential`: no runtime, log or
//! singleton, and stdout carries only the key.
//!
//! It has no daemon state of its own and never re-reads `daemon.toml`: the daemon that launched
//! this session already put the configured `api_key_file`'s **path** (never its content) in the
//! session's environment (`crates/micold-daemon/src/state.rs`'s `with_configured_env`), and this
//! subcommand only reads that variable and the file it names.

use std::ffi::OsString;
use std::io::Write;

use micold_core::daemon_config::ai_cli::CLAUDE_API_KEY_FILE_VAR;
use micold_core::session::AiCli;

const USAGE: &str = "usage: micold-daemon ai-key <claude_code>";

/// Runs `ai-key <args>` (the arguments after `ai-key`) and returns the exit status.
pub fn main(args: &[OsString]) -> i32 {
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    run(args, &|name| std::env::var(name).ok(), &mut out, &mut err)
}

/// [`main`] with the environment lookup and output streams injected.
pub fn run(
    args: &[OsString],
    env: &dyn Fn(&str) -> Option<String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let Some(cli) = args.first().and_then(|a| a.to_str()) else {
        let _ = writeln!(err, "{USAGE}");
        return 2;
    };
    if cli != cli_arg(AiCli::ClaudeCode) {
        let _ = writeln!(
            err,
            "micold-daemon: ai-key has no helper for \"{cli}\"\n{USAGE}"
        );
        return 2;
    }
    let Some(path) = env(CLAUDE_API_KEY_FILE_VAR) else {
        let _ = writeln!(err, "micold-daemon: {CLAUDE_API_KEY_FILE_VAR} is not set");
        return 1;
    };
    let mut text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            let _ = writeln!(err, "micold-daemon: cannot read {path}: {e}");
            return 1;
        }
    };
    while text.ends_with('\n') || text.ends_with('\r') {
        text.pop();
    }
    let _ = write!(out, "{text}");
    0
}

/// The name `ai-key` takes for `cli` (the `daemon.toml` `id`, contract CLI ids). Only Claude Code
/// uses `apiKeyHelper` today ([`micold_core::daemon_config::ai_cli::uses_api_key_helper`]).
pub fn cli_arg(cli: AiCli) -> &'static str {
    match cli {
        AiCli::ClaudeCode => "claude_code",
        AiCli::Copilot => "copilot",
        AiCli::Pi => "pi",
        AiCli::Codex => "codex",
        AiCli::OpenCode => "opencode",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_str(args: &[&str], env: &dyn Fn(&str) -> Option<String>) -> (i32, String, String) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        let code = run(&args, env, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn prints_the_key_files_content_stripped_of_its_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("claude.key");
        std::fs::write(&file, "sk-test\n").unwrap();
        let path = file.display().to_string();
        let env = move |name: &str| (name == CLAUDE_API_KEY_FILE_VAR).then(|| path.clone());
        let (code, stdout, _err) = run_str(&["claude_code"], &env);
        assert_eq!(code, 0);
        assert_eq!(stdout, "sk-test");
    }

    #[test]
    fn exits_1_with_the_variable_unset() {
        let (code, stdout, _err) = run_str(&["claude_code"], &|_| None);
        assert_eq!(code, 1);
        assert_eq!(stdout, "");
    }

    #[test]
    fn exits_1_when_the_named_file_cannot_be_read() {
        let env =
            |name: &str| (name == CLAUDE_API_KEY_FILE_VAR).then(|| "/does/not/exist".to_string());
        let (code, _stdout, _err) = run_str(&["claude_code"], &env);
        assert_eq!(code, 1);
    }

    #[test]
    fn refuses_a_cli_with_no_helper() {
        let (code, _stdout, err) = run_str(&["copilot"], &|_| None);
        assert_eq!(code, 2);
        assert!(err.contains("copilot"));
    }
}
