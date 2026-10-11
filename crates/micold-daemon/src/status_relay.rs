//! The status-line relay: `micold-daemon status-line <relay-file>` (feature 490, contract S2,
//! research R3, R4).
//!
//! Claude Code runs this as the `statusLine` command of a session the app started. It does two
//! things at once with the JSON Claude Code writes to its stdin:
//!
//! - posts the input's `rate_limits` object, and nothing else from it (FR-014), to the daemon's
//!   `/status/<session>` route, giving up after [`POST_TIMEOUT`];
//! - runs the user's own status-line command, if any, with the same input, and copies its output
//!   to Claude Code as it arrives (FR-020), never waiting on the post (FR-017).
//!
//! It is silent in every failure: nothing of its own reaches stdout or stderr, and the exit status
//! is the user command's, else 0 (FR-015). No tokio runtime, logging or singleton is started.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use micold_core::plan_usage::status_line;
use micold_core::plan_usage::user_status_line::RelayConfig;

/// Set in the user command's environment. A relay that starts with it set runs nothing, so a user
/// command that happens to call the relay cannot recurse (S2.6).
pub const RELAY_ENV: &str = "MICOLD_STATUS_RELAY";

/// Input over this size is still passed to the user command, but not read for a reading (S2.1).
pub const MAX_INPUT: usize = 1024 * 1024;

/// The whole exchange with the daemon, connect to answer (S2.3).
pub const POST_TIMEOUT: Duration = Duration::from_millis(500);

/// The binary's entry point for `status-line <relay-file>`; returns the process exit status.
pub fn main(relay_file: &Path) -> i32 {
    let nested = std::env::var_os(RELAY_ENV).is_some();
    relay(
        relay_file,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        nested,
        user_shell,
    )
}

/// Relay one status-line run. `shell` builds the command that runs the user's status line;
/// `nested` is whether [`RELAY_ENV`] was already set.
pub fn relay(
    relay_file: &Path,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    nested: bool,
    shell: impl Fn(&str) -> Command,
) -> i32 {
    let mut input = Vec::new();
    if stdin.read_to_end(&mut input).is_err() {
        return 0;
    }
    if nested {
        return 0;
    }
    let Some(config) = std::fs::read(relay_file)
        .ok()
        .and_then(|text| serde_json::from_slice::<RelayConfig>(&text).ok())
    else {
        return 0;
    };

    let post = (input.len() <= MAX_INPUT)
        .then(|| status_line::extract(&input))
        .flatten()
        .map(|limits| {
            let body = serde_json::to_vec(&limits).unwrap_or_default();
            let (url, token) = (config.url.clone(), config.token.clone());
            std::thread::spawn(move || {
                let _ = post(&url, &token, &body);
            })
        });

    let status = match &config.user_command {
        Some(command) => run_user_command(shell(command), input, stdout),
        None => 0,
    };
    if let Some(post) = post {
        // Bounded by POST_TIMEOUT; the user's line has already been written.
        let _ = post.join();
    }
    status
}

/// The shell Claude Code itself runs a status-line command with (research R4).
pub fn user_shell(command: &str) -> Command {
    #[cfg(windows)]
    {
        if let Some(bash) = git_bash() {
            let mut c = Command::new(bash);
            c.arg("-c").arg(command);
            return c;
        }
        let mut c = Command::new("powershell");
        c.args(["-NoProfile", "-Command", command]);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command);
        c
    }
}

/// `CLAUDE_CODE_GIT_BASH_PATH`, else the `bash.exe` of the Git for Windows that `git.exe` on
/// `PATH` belongs to.
#[cfg(windows)]
fn git_bash() -> Option<std::path::PathBuf> {
    if let Some(path) = std::env::var_os("CLAUDE_CODE_GIT_BASH_PATH") {
        return Some(path.into());
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        if !dir.join("git.exe").is_file() {
            return None;
        }
        // `Git\cmd\git.exe` sits beside `Git\bin\bash.exe`; `Git\bin\git.exe` beside `bash.exe`.
        [dir.join("bash.exe"), dir.parent()?.join("bin").join("bash.exe")]
            .into_iter()
            .find(|bash| bash.is_file())
    })
}

/// Run the user's command with `input` on its stdin and copy its stdout to `stdout` as it arrives.
/// Its exit status, or 0 when it could not be started or was ended by a signal.
fn run_user_command(mut command: Command, input: Vec<u8>, stdout: &mut dyn Write) -> i32 {
    command
        .env(RELAY_ENV, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let Ok(mut child) = command.spawn() else {
        return 0;
    };
    let feeder = child.stdin.take().map(|mut pipe| {
        std::thread::spawn(move || {
            // A command that stops reading early closes the pipe; that is not our failure.
            let _ = pipe.write_all(&input);
        })
    });
    if let Some(mut out) = child.stdout.take() {
        let mut chunk = [0u8; 8192];
        loop {
            match out.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if stdout.write_all(&chunk[..n]).is_err() || stdout.flush().is_err() {
                        break;
                    }
                }
            }
        }
    }
    if let Some(feeder) = feeder {
        let _ = feeder.join();
    }
    child.wait().ok().and_then(|s| s.code()).unwrap_or(0)
}

/// POST `body` to a loopback `http://127.0.0.1:<port>/<path>` URL, within [`POST_TIMEOUT`].
fn post(url: &str, token: &str, body: &[u8]) -> io::Result<()> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "not a loopback http URL");
    let rest = url.strip_prefix("http://").ok_or_else(invalid)?;
    let (authority, path) = rest.split_at(rest.find('/').ok_or_else(invalid)?);
    let addr: SocketAddr = authority.parse().map_err(|_| invalid())?;
    if !addr.ip().is_loopback() {
        return Err(invalid());
    }

    let deadline = Instant::now() + POST_TIMEOUT;
    let left = || {
        deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| io::Error::from(io::ErrorKind::TimedOut))
    };
    let mut stream = TcpStream::connect_timeout(&addr, left()?)?;
    stream.set_write_timeout(Some(left()?))?;
    let head = format!(
        "POST {path} HTTP/1.1\r\nhost: {authority}\r\nauthorization: Bearer {token}\r\n\
         content-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    // Wait for the answer's first bytes, so the daemon has the reading before the process ends.
    stream.set_read_timeout(Some(left()?))?;
    let mut first = [0u8; 16];
    let _ = stream.read(&mut first)?;
    Ok(())
}
