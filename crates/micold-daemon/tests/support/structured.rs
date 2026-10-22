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
/// `env-record` (`NAME=value`, sorted; `NAME` alone when unset), `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT`
/// to `forward-record` (the same form, feature 944), its pid to `pid-record`, appends
/// one line to `starts-record` (one per start), and from then on appends every stdin line to
/// `stdin-record`. Then it runs `.fake-stream`, one line at a time:
///
/// - `replay <file>` writes the file's bytes to stdout line by line, as they are, a trailing
///   partial line included;
/// - `pause <ms>` sleeps; `await-stdin` blocks until one more stdin line arrived;
/// - `stderr <text>` writes the text and a line end to stderr; `exit <code>` exits;
/// - `prompt <file>` POSTs the file (one JSON-RPC message) to the tool server its `--mcp-config`
///   file names, with that file's bearer, over HTTP/1.1 with `Connection: close`, and appends the
///   reply's `content[0].text` (or `<no reply>` when the connection closed without one) to
///   `prompt-record`, one line each (#942 R13); `prompt-async <file>` does the same on a thread,
///   so several calls are held at once while the script goes on;
/// - `control success`, `control error <code> <text>` and `control none` wait for the next
///   `control_request` line on stdin (skipping any other line) and answer it with a
///   `control_response` for its `request_id` (#942 contracts/claude-permission.md *Mode change*):
///   `success` with `response.mode` set to the requested mode, `error` with the `error` text and
///   `error_code`, `none` with nothing; an answer is noted in `order-record` (`control`) before it
///   is written, as each prompt-tool answer is (`answer`) once it came, so a test sees which was
///   first;
/// - `status <mode>` writes a `system`/`status` line reporting `permissionMode` `<mode>`;
/// - `await-file <name>` blocks until the test creates `<name>` in the cwd, so a test holds a turn
///   open (#941 T023);
/// - `on-message <file>` replays the file once for each later stdin `user` line, one turn per
///   message, skipping every other line (#941 T023); `on-message-gated <file>` does the same, but
///   before the n-th message's replay (counted from 1) it waits for `turn-<n>` in the cwd, so a
///   test decides when each turn's result is written. Neither returns while stdin is open;
/// - `on-interrupt <file>`: on each later stdin `control_request` `interrupt` line, write P-4's
///   `control_response` `success` for its `request_id`, then replay the file without its own
///   `control_response` lines, so each interrupted turn ends with the file's `result` (#941 T034);
///   every other line is skipped, so a user line opens a turn that only an interrupt ends. Does
///   not return while stdin is open;
/// - `ignore-interrupt <file>`: `on-message-gated <file>`, named for what the tests use it for: an
///   interrupt is recorded and nothing else is done with it, so only `turn-<n>` ends a turn;
/// - `initialize` waits for the daemon's `initialize` control request and answers it with
///   `command-initialize.jsonl`'s recorded answer for its `request_id` (#943 R12);
///   `initialize error <code> <text>` refuses it instead. Without the directive it is never answered
///   (silent), so a suite that does not ask for the list sees no note in its events;
/// - `reply <text>\t<file>` makes a later `on-message` replay `<file>` instead of its own file for a
///   user message whose text is exactly `<text>` (`/compact` → `command-compact.jsonl`, #943 R12);
///   `on-message` with no file replays only those;
/// - `models <name>…` answers every later `set_model` control request on stdin (#943 T035), from
///   the stdin reader, whatever the script is doing: a name in the list is accepted with
///   `model-change.jsonl`'s `success` shape naming it, any other refused with
///   `model-refused.jsonl`'s `error` (`Model '<name>' not found`, `not_found`); `models` with no
///   name refuses all. Each accepted model is reported in every later `init` line the stand-in
///   writes (`replay`, `on-message`), in place of the recorded one. A `set_model` request is
///   recorded like every stdin line but, once `models` ran, reaches no other directive;
/// - any other line is written to stdout as it is, with a line end.
///
/// An `initialize` request is recorded like every stdin line, in order, but never reaches
/// `await-stdin`, `control` or `on-message`: only the `initialize` directive reads it.
///
/// A stdin line containing `CLOSE-STDIN` is recorded and then the stand-in closes its stdin and
/// reads no more, so the daemon's next write to it fails while the process runs on (#941 T024).
///
/// When the script ends it idles until killed.
const STAND_IN_SOURCE: &str = r##"
use std::io::{BufRead, Write};
use std::sync::{mpsc, Mutex};

/// The `models` directive's accept list (`None` until it ran) and the last model accepted.
struct Models {
    accept: Mutex<Option<Vec<String>>>,
    accepted: Mutex<Option<String>>,
}

static MODELS: Models = Models {
    accept: Mutex::new(None),
    accepted: Mutex::new(None),
};

/// Answer a `set_model` request when the `models` directive ran; `false` leaves it to the script.
fn answer_set_model(line: &str) -> bool {
    if !line.contains("\"control_request\"") || !line.contains("\"subtype\":\"set_model\"") {
        return false;
    }
    let accept = MODELS.accept.lock().unwrap().clone();
    let Some(accept) = accept else { return false };
    let id = string_after(line, "request_id").unwrap_or_default();
    let model = string_after(line, "model").unwrap_or_default();
    let answer = if accept.contains(&model) {
        *MODELS.accepted.lock().unwrap() = Some(model.clone());
        format!(
            "{{\"type\":\"control_response\",\"response\":{{\"subtype\":\"success\",\"request_id\":\"{id}\",\"response\":{{\"model\":\"{model}\"}}}}}}"
        )
    } else {
        format!(
            "{{\"type\":\"control_response\",\"response\":{{\"subtype\":\"error\",\"request_id\":\"{id}\",\"error\":\"Model '{model}' not found\",\"error_code\":\"not_found\"}}}}"
        )
    };
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{answer}");
    let _ = out.flush();
    true
}

/// `piece`, a line the stand-in is about to write, with an `init` line's model replaced by the
/// last one accepted.
fn reported(piece: &[u8]) -> Vec<u8> {
    let accepted = MODELS.accepted.lock().unwrap().clone();
    let text = String::from_utf8_lossy(piece);
    match accepted {
        Some(model) if text.contains("\"subtype\":\"init\"") => {
            let Some(at) = text.find("\"model\":\"") else { return piece.to_vec() };
            let start = at + "\"model\":\"".len();
            let end = text[start..].find('"').map_or(start, |n| start + n);
            format!("{}{}{}", &text[..start], model, &text[end..]).into_bytes()
        }
        _ => piece.to_vec(),
    }
}

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
    let forward = match std::env::var("CLAUDE_CODE_FORWARD_SUBAGENT_TEXT") {
        Ok(v) => format!("CLAUDE_CODE_FORWARD_SUBAGENT_TEXT={v}"),
        Err(_) => "CLAUDE_CODE_FORWARD_SUBAGENT_TEXT".to_string(),
    };
    let _ = std::fs::write("forward-record", forward + "\n");
    let _ = std::fs::write("pid-record", format!("{}\n", std::process::id()));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("starts-record") {
        let _ = writeln!(f, "start {}", std::process::id());
    }

    let (lines_tx, lines_rx) = mpsc::channel::<String>();
    let (init_tx, init_rx) = mpsc::channel::<String>();
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
            let close = line.contains("CLOSE-STDIN");
            if line.contains("\"control_request\"") && line.contains("\"subtype\":\"initialize\"") {
                let _ = init_tx.send(line);
                continue;
            }
            if answer_set_model(&line) {
                continue;
            }
            let _ = lines_tx.send(line);
            if close {
                close_stdin();
                break;
            }
        }
    });

    let script = std::fs::read_to_string(".fake-stream").unwrap_or_default();
    let mut out = std::io::stdout();
    let mut replies: Vec<(String, String)> = Vec::new();
    for line in script.lines() {
        let (cmd, arg) = line.split_once(' ').unwrap_or((line, ""));
        match cmd {
            "replay" => {
                let bytes = std::fs::read(arg).expect("read the fixture");
                for piece in bytes.split_inclusive(|b| *b == b'\n') {
                    let _ = out.write_all(&reported(piece));
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
            "control" => control(&lines_rx, &mut out, arg),
            "models" => {
                *MODELS.accept.lock().unwrap() =
                    Some(arg.split_whitespace().map(str::to_string).collect());
            }
            "initialize" => initialize(&init_rx, &mut out, arg),
            "reply" => {
                if let Some((text, file)) = arg.split_once('\t') {
                    replies.push((text.to_string(), file.to_string()));
                }
            }
            "status" => {
                let _ = writeln!(
                    out,
                    "{{\"type\":\"system\",\"subtype\":\"status\",\"permissionMode\":\"{arg}\"}}"
                );
            }
            "await-file" => {
                while !std::path::Path::new(arg).exists() {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
            "on-message" => on_message(&lines_rx, &mut out, arg, &replies, false),
            "on-message-gated" | "ignore-interrupt" => {
                on_message(&lines_rx, &mut out, arg, &replies, true)
            }
            "on-interrupt" => on_interrupt(&lines_rx, &mut out, arg),
            "prompt" => prompt(&args, arg),
            "prompt-async" => {
                let (args, file) = (args.clone(), arg.to_string());
                std::thread::spawn(move || prompt(&args, &file));
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

/// Close this process's stdin, so a write to it fails.
#[cfg(unix)]
fn close_stdin() {
    extern "C" {
        fn close(fd: i32) -> i32;
    }
    // SAFETY: fd 0 is this process's stdin; nothing reads it after this.
    unsafe {
        close(0);
    }
}

#[cfg(windows)]
fn close_stdin() {
    extern "system" {
        fn GetStdHandle(which: u32) -> *mut std::ffi::c_void;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    }
    // SAFETY: the standard input handle; nothing reads it after this.
    unsafe {
        CloseHandle(GetStdHandle(-10i32 as u32));
    }
}

/// For each later stdin `user` line, replay `file`, or the `replies` file for its text: one turn
/// per message; a message with neither gets no turn. `gated`: the n-th waits for `turn-<n>` in the
/// cwd first.
fn on_message(
    lines: &mpsc::Receiver<String>,
    out: &mut impl Write,
    file: &str,
    replies: &[(String, String)],
    gated: bool,
) {
    let mut n = 0;
    while let Ok(line) = lines.recv() {
        if !line.contains("\"type\":\"user\"") {
            continue;
        }
        n += 1;
        if gated {
            let gate = format!("turn-{n}");
            while !std::path::Path::new(&gate).exists() {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        let text = line
            .find("\"text\":\"")
            .map(|at| json_string(&line[at + 8..]))
            .unwrap_or_default();
        let file = replies
            .iter()
            .find(|(said, _)| *said == text)
            .map_or(file, |(_, reply)| reply.as_str());
        if file.is_empty() {
            continue;
        }
        let bytes = std::fs::read(file).expect("read the fixture");
        for piece in bytes.split_inclusive(|b| *b == b'\n') {
            let _ = out.write_all(&reported(piece));
            let _ = out.flush();
        }
    }
}

/// For each later stdin `interrupt` control request: answer it, then replay `file` without its
/// `control_response` lines (P-4).
fn on_interrupt(lines: &mpsc::Receiver<String>, out: &mut impl Write, file: &str) {
    while let Ok(line) = lines.recv() {
        if !line.contains("\"control_request\"") || !line.contains("\"interrupt\"") {
            continue;
        }
        let id = string_after(&line, "request_id").unwrap_or_default();
        let _ = writeln!(
            out,
            "{{\"type\":\"control_response\",\"response\":{{\"subtype\":\"success\",\"request_id\":\"{id}\",\"response\":{{\"still_queued\":[]}}}}}}"
        );
        let text = std::fs::read_to_string(file).expect("read the fixture");
        for piece in text.split_inclusive('\n') {
            if !piece.contains("\"control_response\"") {
                let _ = out.write_all(piece.as_bytes());
                let _ = out.flush();
            }
        }
    }
}

/// Wait for the daemon's `initialize` request and answer it as `how` says: the recorded answer
/// (`""`) or `error <code> <text>`.
fn initialize(requests: &mpsc::Receiver<String>, out: &mut impl Write, how: &str) {
    let Ok(request) = requests.recv() else { return };
    let id = string_after(&request, "request_id").unwrap_or_default();
    let answer = match how.split_once(' ') {
        Some(("error", rest)) => {
            let (code, text) = rest.split_once(' ').unwrap_or((rest, ""));
            format!(
                "{{\"type\":\"control_response\",\"response\":{{\"subtype\":\"error\",\"request_id\":\"{id}\",\"error\":\"{text}\",\"error_code\":\"{code}\"}}}}"
            )
        }
        _ => {
            let recorded = std::fs::read_to_string(INITIALIZE_ANSWER).expect("read the recorded answer");
            recorded
                .lines()
                .find(|line| line.contains("\"control_response\"") && line.contains("\"commands\""))
                .expect("the recorded answer")
                .replace("\"request_id\":\"micold-1\"", &format!("\"request_id\":\"{id}\""))
        }
    };
    let _ = writeln!(out, "{answer}");
}

/// Wait for the next `control_request` line on stdin and answer it as `how` says: `success`,
/// `error <code> <text>` or `none`.
fn control(lines: &mpsc::Receiver<String>, out: &mut impl Write, how: &str) {
    let request = loop {
        match lines.recv() {
            Ok(line) if line.contains("\"control_request\"") => break line,
            Ok(_) => continue,
            Err(_) => return,
        }
    };
    let id = string_after(&request, "request_id").unwrap_or_default();
    let mode = string_after(&request, "mode").unwrap_or_default();
    let (kind, rest) = how.split_once(' ').unwrap_or((how, ""));
    let answer = match kind {
        "success" => format!(
            "{{\"type\":\"control_response\",\"response\":{{\"subtype\":\"success\",\"request_id\":\"{id}\",\"response\":{{\"mode\":\"{mode}\"}}}}}}"
        ),
        "error" => {
            let (code, text) = rest.split_once(' ').unwrap_or((rest, ""));
            format!(
                "{{\"type\":\"control_response\",\"response\":{{\"subtype\":\"error\",\"request_id\":\"{id}\",\"error\":\"{text}\",\"error_code\":\"{code}\"}}}}"
            )
        }
        _ => return,
    };
    note_order("control");
    let _ = writeln!(out, "{answer}");
}

/// Append `what` to `order-record`.
fn note_order(what: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("order-record") {
        let _ = writeln!(f, "{what}");
    }
}

/// The string value of the first `"key": "…"` in `text` (the binding file is pretty JSON).
fn string_after(text: &str, key: &str) -> Option<String> {
    let at = text.find(&format!("\"{key}\""))? + key.len() + 2;
    let rest = &text[at..];
    let open = rest.find('"')? + 1;
    let close = rest[open..].find('"')? + open;
    Some(rest[open..close].to_string())
}

/// The JSON string starting just after its opening quote in `text`, unescaped.
fn json_string(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('b') => out.push('\u{8}'),
                Some('f') => out.push('\u{c}'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    let code = u32::from_str_radix(&hex, 16).unwrap_or(0xfffd);
                    out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                }
                Some(other) => out.push(other),
                None => break,
            },
            c => out.push(c),
        }
    }
    out
}

/// Make the prompt-tool call in `file` and record its answer.
fn prompt(args: &[String], file: &str) {
    let answer = call_tool_server(args, file).unwrap_or_else(|| "<no reply>".to_string());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("prompt-record") {
        let _ = f.write_all(format!("{answer}\n").as_bytes());
    }
    note_order("answer");
}

fn call_tool_server(args: &[String], file: &str) -> Option<String> {
    use std::io::Read;
    let config_at = args.iter().position(|a| a == "--mcp-config")? + 1;
    let config = std::fs::read_to_string(args.get(config_at)?).ok()?;
    let url = string_after(&config, "url")?;
    let bearer = string_after(&config, "Authorization")?;
    let rest = url.strip_prefix("http://")?;
    let (host, path) = rest.split_once('/')?;
    let body = std::fs::read_to_string(file).ok()?;
    let mut stream = std::net::TcpStream::connect(host).ok()?;
    let request = format!(
        "POST /{path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nAuthorization: {bearer}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut reply = Vec::new();
    let _ = stream.read_to_end(&mut reply);
    let reply = String::from_utf8_lossy(&reply).into_owned();
    let (_head, json) = reply.split_once("\r\n\r\n")?;
    let key = "\"text\":\"";
    let at = json.find(key)? + key.len();
    Some(json_string(&json[at..]))
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
        let answer = fixture("command-initialize.jsonl");
        let source_text = format!(
            "const INITIALIZE_ANSWER: &str = {:?};\n{STAND_IN_SOURCE}",
            answer.display().to_string()
        );
        std::fs::write(&source, source_text).expect("write the stand-in's source");
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

/// Whether `line`, read from a stand-in's stdin, is the daemon's `initialize` request (#943 R1).
pub fn is_initialize(line: &str) -> bool {
    line.contains("\"control_request\"") && line.contains("\"subtype\":\"initialize\"")
}

/// The stdin lines the stand-in in `dir` recorded so far, but the `initialize` request every
/// structured spawn writes first (#943 R1); none when it recorded nothing.
pub fn stdin_lines(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("stdin-record"))
        .unwrap_or_default()
        .lines()
        .filter(|line| !is_initialize(line))
        .map(str::to_owned)
        .collect()
}

/// A `reply` directive: a later `on-message` replays the fixture `name` for a message `text`.
pub fn reply(text: &str, name: &str) -> String {
    format!("reply {text}\t{}\n", fixture(name).display())
}

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

/// How many times the stand-in has started in `dir`.
pub fn starts(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("starts-record"))
        .map(|text| text.lines().count())
        .unwrap_or(0)
}

/// Remove what an earlier start of the stand-in recorded in `dir`.
pub fn clear_records(dir: &Path) {
    for name in [
        "argv-record",
        "cwd-record",
        "env-record",
        "forward-record",
        "pid-record",
        "stdin-record",
        "prompt-record",
        "order-record",
    ] {
        let _ = std::fs::remove_file(dir.join(name));
    }
}

/// One event as a viewing client received it.
#[derive(Debug, Clone)]
pub struct Received {
    pub event: micold_core::structured::SequencedEvent,
    /// The `live` flag of the message that carried it.
    pub live: bool,
    /// The `omitted` count of that message.
    pub omitted: u64,
    pub at: Instant,
}

/// A client of `state` viewing `id`'s `Primary`, as a window does with `SetViewedTerminals`.
pub async fn viewer(
    state: &std::sync::Arc<micold_daemon::state::DaemonState>,
    project: &Path,
    id: micold_core::session::SessionId,
) -> crate::conn::Client {
    use futures_util::SinkExt;
    use micold_core::protocol::codec::Frame;
    use micold_core::protocol::messages::{ClientMsg, SessionProcess, TerminalRef};
    let mut client = crate::conn::connect_as(state, "test-client").await;
    client
        .send(Frame::Control(ClientMsg::SetViewedTerminals {
            project: project.to_path_buf(),
            terminals: vec![TerminalRef {
                session: id,
                process: SessionProcess::Primary,
            }],
        }))
        .await
        .expect("the view is sent");
    client
}

/// One `StructuredEvents` message as a viewing client received it.
#[derive(Debug, Clone)]
pub struct Message {
    pub omitted: u64,
    pub events: Vec<micold_core::structured::SequencedEvent>,
    pub live: bool,
    pub at: Instant,
}

/// The `StructuredEvents` messages of `id` that `client` receives until `done` holds for them,
/// within 15 s.
pub async fn messages_until(
    client: &mut crate::conn::Client,
    id: micold_core::session::SessionId,
    done: impl Fn(&[Message]) -> bool,
) -> Vec<Message> {
    use futures_util::StreamExt;
    use micold_core::protocol::codec::Frame;
    use micold_core::protocol::messages::DaemonMsg;
    let mut got = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while !done(&got) {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(!left.is_zero(), "the events never completed: {got:#?}");
        match tokio::time::timeout(left, client.next()).await {
            Ok(Some(Ok(Frame::Control(DaemonMsg::StructuredEvents {
                session,
                omitted,
                events,
                live,
            })))) => {
                assert_eq!(session, id, "only the viewed session's events");
                got.push(Message {
                    omitted,
                    events,
                    live,
                    at: Instant::now(),
                });
            }
            Ok(Some(Ok(_))) | Err(_) => {}
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => panic!("the connection closed"),
        }
    }
    got
}

/// Every event of `messages`, each with the flags of the message that carried it.
pub fn received(messages: &[Message]) -> Vec<Received> {
    messages
        .iter()
        .flat_map(|m| {
            m.events.iter().map(|event| Received {
                event: event.clone(),
                live: m.live,
                omitted: m.omitted,
                at: m.at,
            })
        })
        .collect()
}

/// The `StructuredEvents` of `id` that `client` receives until `done` holds for them, within 15 s.
pub async fn events_until(
    client: &mut crate::conn::Client,
    id: micold_core::session::SessionId,
    done: impl Fn(&[Received]) -> bool,
) -> Vec<Received> {
    received(&messages_until(client, id, |messages| done(&received(messages))).await)
}

/// The events the parser makes of the fixture `name`, in order.
pub fn fixture_events(name: &str) -> Vec<micold_core::structured::StructuredEvent> {
    fixture_lines(name)
        .iter()
        .map(|line| micold_core::structured::parse_line(line))
        .collect()
}

/// The events of `got`, without their seq.
pub fn events_of(got: &[Received]) -> Vec<micold_core::structured::StructuredEvent> {
    got.iter().map(|r| r.event.event.clone()).collect()
}

/// Whether the seqs of `got` strictly increase.
pub fn seqs_increase(got: &[Received]) -> bool {
    got.windows(2).all(|w| w[0].event.seq < w[1].event.seq)
}

/// A structured Claude Code session record at `location`.
pub fn structured_session(
    location: micold_core::session::SessionLocation,
) -> micold_core::session::Session {
    let mut session = micold_core::session::Session::start_new_of_kind(
        location,
        AiCli::ClaudeCode,
        micold_core::session::SessionKind::Structured,
    );
    session.set_mode(micold_core::session::TerminalMode::AiCli);
    session
}

/// Turn the permission prompts on for `state` (#942 R14): structured sessions it starts from now
/// on name the prompt tool when bound, and their tool-server callers are offered it.
pub fn prompts_on(state: &micold_daemon::state::DaemonState) {
    state.set_permission_prompts(true);
}

/// The answers the stand-in in `dir` recorded for its prompt-tool calls, once there are `n`,
/// within 15 s.
pub fn prompt_answers(dir: &Path, n: usize) -> Vec<serde_json::Value> {
    let file = dir.join("prompt-record");
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        if text.ends_with('\n') && lines.len() >= n {
            return lines
                .iter()
                .map(|line| {
                    serde_json::from_str(line)
                        .unwrap_or_else(|e| panic!("an answer is JSON ({e}): {line}"))
                })
                .collect();
        }
        assert!(
            Instant::now() < deadline,
            "{n} prompt answers never appeared: {text:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// How many answers the stand-in in `dir` has recorded so far.
pub fn prompt_answer_count(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("prompt-record"))
        .map(|text| text.lines().count())
        .unwrap_or(0)
}

/// Write the JSON-RPC `tools/call` of the prompt tool for `tool_name` with `input` to `dir/name`,
/// for a `prompt <file>` directive; returns the file's path.
pub fn prompt_call_file(
    dir: &Path,
    name: &str,
    tool_name: &str,
    input: serde_json::Value,
    tool_use_id: &str,
) -> PathBuf {
    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "permission_prompt",
            "arguments": {"tool_name": tool_name, "input": input, "tool_use_id": tool_use_id},
            "_meta": {"claudecode/toolUseId": tool_use_id},
        },
    });
    let path = dir.join(name);
    std::fs::write(&path, call.to_string()).expect("write the prompt call");
    path
}
