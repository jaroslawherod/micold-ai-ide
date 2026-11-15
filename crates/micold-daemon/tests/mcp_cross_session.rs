//! Feature 034 (contracts/mcp-tools.md `read_session_output`, `send_session_input`; US4 scenarios
//! 1–5; FR-010, FR-012, FR-012a, FR-014, FR-015, FR-016; A19–A23, U198–U201): an agent reads a sibling
//! session's terminal and types into it, under the user's "Let agents read and type into other
//! sessions" option.
//!
//! S1, the caller, is a session in worktree `b`. S2 is created through `create_session` and runs a
//! stand-in `claude` that prints `<bin>/claude.lines` numbered lines (3 by default) and a coloured
//! prompt, then appends what is typed into its terminal to `<bin>/claude.input`. With
//! `<bin>/claude.raw` present it also turns bracketed paste on and puts its terminal in raw mode, as
//! a real CLI does, so the file holds the exact bytes the service wrote. A second project holds
//! session X and worktree `elsewhere`.
//!
//! At "Confirm each send" a fake window is shown the prompt and answers it, or leaves it until the
//! 60 s bound passes on a paused clock.

// unix-only: the stand-in CLI is a `#!/bin/sh` script (Windows port is a recorded follow-up).
#![cfg(unix)]

#[path = "support/mcp.rs"]
mod mcp_support;

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mcp_support::*;
use micold_core::mcp::policy::CrossSessionAccess;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ConfirmOperation, DaemonMsg, SessionProcess};
use micold_core::session::{AiCli, SessionId, ShellInstanceId, TerminalMode};
use micold_core::worktree_parent::ParentChoice;
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

/// The tests change process-wide variables (`PATH`, `HOME`, …); one at a time.
static ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A stand-in CLI starts, and a write reaches its file, well within this.
const BOUND: Duration = Duration::from_secs(20);

/// The caller, in worktree `b`.
const S1: u128 = 3;
/// A session in the project root that is never started.
const IDLE: u128 = 1;
/// A session of the other project.
const X: u128 = 9;
/// A session no project has.
const UNKNOWN: u128 = 999;
/// A plain shell in worktree `b`, ahead of any session created there (only in [`Sandbox::with`]).
const SHELL: u128 = 20;

/// What an agent sends; it must reach S2 and nothing else.
const TEXT: &str = "run the tests";

/// Every variable a test changes, restored on drop.
struct Env {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl Env {
    fn set(vars: &[(&'static str, Option<OsString>)]) -> Self {
        let saved = vars
            .iter()
            .map(|(name, value)| {
                let previous = std::env::var_os(name);
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
                (*name, previous)
            })
            .collect();
        Self { saved }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for (name, previous) in self.saved.drain(..) {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

struct Sandbox {
    _env: Env,
    bin: tempfile::TempDir,
    home: tempfile::TempDir,
    _project: tempfile::TempDir,
    _other: tempfile::TempDir,
    _store: tempfile::TempDir,
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
}

impl Sandbox {
    async fn new() -> Self {
        Self::with(Vec::new()).await
    }

    /// [`Self::new`] with `extra` sessions after S1 in the project's catalog.
    async fn with(extra: Vec<micold_core::session::Session>) -> Self {
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        install_cli(bin.path());
        init_repo(project.path());
        add_worktree(project.path(), "b");
        // The caller's own worktree: `b` is where S2 is created, and a location holds one session.
        add_worktree(project.path(), "caller");
        init_repo(other.path());
        add_worktree(other.path(), "elsewhere");
        trust_project(home.path(), project.path());
        let mut path = vec![bin.path().to_path_buf()];
        path.extend(["/usr/bin", "/bin"].map(PathBuf::from));
        let env = Env::set(&[
            ("PATH", Some(std::env::join_paths(path).unwrap())),
            ("HOME", Some(home.path().into())),
            (
                "XDG_DATA_HOME",
                Some(home.path().join(".local/share").into()),
            ),
            ("CLAUDE_CONFIG_DIR", None),
        ]);
        let state = state_over(
            vec![
                (
                    project.path().to_path_buf(),
                    true,
                    [
                        session(sid(IDLE), None, TerminalMode::AiCli, AiCli::ClaudeCode),
                        session(
                            sid(S1),
                            Some("caller"),
                            TerminalMode::AiCli,
                            AiCli::ClaudeCode,
                        ),
                    ]
                    .into_iter()
                    .chain(extra)
                    .collect(),
                ),
                (
                    other.path().to_path_buf(),
                    true,
                    vec![session(
                        sid(X),
                        None,
                        TerminalMode::AiCli,
                        AiCli::ClaudeCode,
                    )],
                ),
            ],
            store.path(),
        );
        let addr = serve_tool_server(&state, store.path().join("mcp")).await;
        Self {
            _env: env,
            bin,
            home,
            _project: project,
            _other: other,
            _store: store,
            state,
            addr,
        }
    }

    /// Call `tool` as S1; it must succeed.
    async fn ok(&self, tool: &str, args: Value) -> Value {
        call_ok(
            self.addr,
            &full_view_credential(&self.state, sid(S1)),
            tool,
            args,
        )
        .await
    }

    /// Call `tool` as S1; it must fail.
    async fn err(&self, tool: &str, args: Value) -> Value {
        call_err(
            self.addr,
            &full_view_credential(&self.state, sid(S1)),
            tool,
            args,
        )
        .await
    }

    /// How many numbered lines the stand-in prints before its prompt.
    fn set_lines(&self, lines: usize) {
        std::fs::write(self.bin.path().join("claude.lines"), lines.to_string()).unwrap();
    }

    /// Make the stand-in behave as a real CLI does: bracketed paste on, terminal in raw mode.
    fn set_raw(&self) {
        std::fs::write(self.bin.path().join("claude.raw"), "").unwrap();
    }

    /// Create S2 in worktree `b` and wait until its prompt is on its terminal.
    async fn start_s2(&self) -> SessionId {
        let out = self
            .ok(
                "create_session",
                json!({"worktree": "b", "ai_cli": "claude_code"}),
            )
            .await;
        let s2 = SessionId::from_uuid(out["session"].as_str().unwrap().parse().unwrap());
        self.ready(s2).await;
        s2
    }

    /// Wait until `session`'s prompt is on its terminal.
    async fn ready(&self, session: SessionId) {
        let deadline = Instant::now() + BOUND;
        while self.screen(session).last().map(String::as_str) != Some("ready>") {
            assert!(
                Instant::now() < deadline,
                "{session:?} never drew its prompt"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// Call `tool` as S1 with the scope its recorded links give it (spec 1857, FR-015), not the
    /// full view of [`Self::ok`]; it must succeed.
    async fn child_ok(&self, tool: &str, args: Value) -> Value {
        call_ok(self.addr, &credential(&self.state, sid(S1)), tool, args).await
    }

    /// [`Self::child_ok`]; it must fail.
    async fn child_err(&self, tool: &str, args: Value) -> Value {
        call_err(self.addr, &credential(&self.state, sid(S1)), tool, args).await
    }

    /// Record `parent` as the parent of S1's worktree `caller`.
    fn link_caller_to(&self, parent: &str) {
        self.state.record_worktree_parent(
            self._project.path(),
            "caller",
            Some(&ParentChoice::Worktree(parent.into())),
        );
    }

    /// A started session in `worktree`, created by S1 as a child; `additional` beside another one.
    async fn start_in(&self, worktree: &str, additional: bool) -> SessionId {
        let out = self
            .child_ok(
                "create_session",
                json!({"worktree": worktree, "ai_cli": "claude_code",
                       "allow_additional_session": additional}),
            )
            .await;
        let session = SessionId::from_uuid(out["session"].as_str().unwrap().parse().unwrap());
        self.ready(session).await;
        session
    }

    /// How many lines of `session`'s terminal show [`TEXT`]: the terminal echoes what is typed.
    fn shown(&self, session: SessionId) -> usize {
        self.screen(session)
            .iter()
            .filter(|line| line.contains(TEXT))
            .count()
    }

    /// S2's primary terminal as the service holds it, read without the tool under test.
    fn screen(&self, session: SessionId) -> Vec<String> {
        let (Some(pty), Some(framer)) = (
            self.state.primary_pty(session),
            self.state.primary_framer(session),
        ) else {
            return Vec::new();
        };
        let framer = framer.lock().unwrap();
        framer.plain_tail(pty.term(), 2_000).0
    }

    /// What was typed into the stand-in's terminal so far.
    fn typed(&self) -> String {
        String::from_utf8_lossy(
            &std::fs::read(self.bin.path().join("claude.input")).unwrap_or_default(),
        )
        .into_owned()
    }

    /// Wait until what was typed is `expected`.
    async fn typed_becomes(&self, expected: &str) {
        let deadline = Instant::now() + BOUND;
        while self.typed() != expected {
            assert!(
                Instant::now() < deadline,
                "typed {:?}, expected {expected:?}",
                self.typed()
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// Give a write that must not happen the time to happen, then check it did not.
    async fn nothing_typed(&self) {
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(self.typed(), "", "S2 must be untouched");
    }

    /// The label the sidebar shows for `session`.
    fn label(&self, session: SessionId) -> String {
        self.state
            .catalog_snapshot()
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .find(|s| s.id == session)
            .map(|s| s.title.display().to_string())
            .expect("the session is in the catalog")
    }

    /// Remove Claude Code's trust record, so it would ask about every folder.
    fn distrust(&self) {
        std::fs::remove_file(self.home.path().join(".claude.json")).unwrap();
    }

    /// Wait until the stand-in has put its terminal in raw mode.
    async fn raw_mode_is_on(&self) {
        let deadline = Instant::now() + BOUND;
        while !self.bin.path().join("claude.rawset").exists() {
            assert!(Instant::now() < deadline, "the stand-in never set raw mode");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    fn set_access(&self, access: CrossSessionAccess) {
        self.state.set_cross_session_access(access).unwrap();
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        for project in self.state.catalog_snapshot().projects {
            for session in project.sessions {
                for pty in self.state.session_ptys(session.id) {
                    let _ = pty.kill();
                }
            }
        }
    }
}

/// Claude Code's own record that it trusts `project` (and so every worktree below it).
fn trust_project(home: &Path, project: &Path) {
    let claude = json!({"projects": {project.to_str().unwrap(): {"hasTrustDialogAccepted": true}}});
    std::fs::write(home.join(".claude.json"), claude.to_string()).unwrap();
}

/// The stand-in `claude` described in the module docs.
fn install_cli(bin: &Path) {
    let dir = bin.display();
    let path = bin.join("claude");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\n\
             n=$(cat '{dir}/claude.lines' 2>/dev/null || echo 3)\n\
             i=1\n\
             while [ \"$i\" -le \"$n\" ]; do printf 'out%s\\n' \"$i\"; i=$((i + 1)); done\n\
             if [ -e '{dir}/claude.raw' ]; then printf '\\033[?2004h'; fi\n\
             printf '\\033[1;32mready>\\033[0m '\n\
             if [ -e '{dir}/claude.raw' ]; then stty raw -echo; : > '{dir}/claude.rawset'; fi\n\
             exec cat >> '{dir}/claude.input'\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A window that answers every prompt it is shown with `allow`; returns the prompts it was shown.
fn answering_window(state: &Arc<DaemonState>, allow: bool) -> Arc<Mutex<Vec<DaemonMsg>>> {
    let mut rx = fake_window(state);
    let shown = Arc::new(Mutex::new(Vec::new()));
    let (st, prompts) = (Arc::clone(state), Arc::clone(&shown));
    tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            if let Frame::Control(msg @ DaemonMsg::ConfirmationRequested { .. }) = frame {
                let DaemonMsg::ConfirmationRequested { id, .. } = &msg else {
                    unreachable!()
                };
                let id = *id;
                prompts.lock().unwrap().push(msg);
                st.answer_confirmation(id, allow);
            }
        }
    });
    shown
}

/// The one prompt a window was shown must ask to type into the session labelled `target` from S1,
/// and carry no part of the text (FR-018: the prompt names the operation and its target only).
fn assert_one_send_prompt(shown: &Mutex<Vec<DaemonMsg>>, target: &str) {
    let shown = shown.lock().unwrap();
    let [DaemonMsg::ConfirmationRequested {
        caller,
        operation,
        target_label,
        ..
    }] = shown.as_slice()
    else {
        panic!("exactly one prompt, got {shown:#?}");
    };
    assert_eq!(*caller, sid(S1));
    assert_eq!(*operation, ConfirmOperation::SendInput);
    assert_eq!(target_label, target, "the prompt names the target");
    assert!(
        !format!("{shown:?}").contains(TEXT),
        "the text must not reach the prompt: {shown:#?}"
    );
}

fn reference(session: SessionId) -> String {
    session.0.to_string()
}

/// A tool and the arguments that aim it at a ref.
type TargetedTool = (&'static str, fn(&str) -> Value);

fn lines_of(out: &Value) -> Vec<String> {
    out["lines"]
        .as_array()
        .unwrap_or_else(|| panic!("no `lines` in {out}"))
        .iter()
        .map(|l| l.as_str().unwrap().to_string())
        .collect()
}

// ---------------------------------------------------------------------------------------
// read_session_output
// ---------------------------------------------------------------------------------------

/// A19 (US4 s1): at Auto and at Confirm each send, S1 reads S2's last lines as plain text. No
/// window is attached, so a read that asked for a confirmation would fail here.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_auto_and_at_confirm_each_send_s1_reads_s2s_last_lines_as_plain_text() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;

    for access in [
        CrossSessionAccess::Auto,
        CrossSessionAccess::ConfirmEachSend,
    ] {
        s.set_access(access);
        let out = s
            .ok("read_session_output", json!({"session": reference(s2)}))
            .await;
        let lines = lines_of(&out);
        assert!(
            lines.ends_with(&["out1", "out2", "out3", "ready>"].map(String::from)),
            "{access:?}: {lines:?}"
        );
        assert!(lines.len() <= 200, "{access:?}: the default is 200 lines");
        assert!(
            lines.iter().all(|l| !l.chars().any(char::is_control)),
            "{access:?}: plain text, no escape sequences: {lines:?}"
        );
        assert_eq!(out["truncated"], json!(false), "{access:?}: {out}");

        let out = s
            .ok(
                "read_session_output",
                json!({"session": reference(s2), "lines": 2}),
            )
            .await;
        assert_eq!(lines_of(&out), ["out3", "ready>"], "{access:?}");
        assert_eq!(
            out["truncated"],
            json!(true),
            "{access:?}: older lines exist"
        );
    }

    s.nothing_typed().await;
}

/// U198: the primary terminal is read even while the user has a shell instance attached.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_primary_terminal_is_read_even_with_a_shell_instance_attached() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    let shell = ShellInstanceId(1);
    s.state.open_shell(s2, shell).expect("a shell opens");
    assert!(
        s.state
            .attach_process(s2, SessionProcess::Shell(shell))
            .is_some(),
        "the shell instance is the attached process"
    );

    let out = s
        .ok(
            "read_session_output",
            json!({"session": reference(s2), "lines": 2}),
        )
        .await;
    assert_eq!(
        lines_of(&out),
        ["out3", "ready>"],
        "the AI CLI's terminal, not the shell's"
    );
}

/// U199 (EC-16): a count above the maximum is clamped to 2,000.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_line_count_above_the_maximum_returns_at_most_2000_lines() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.set_lines(2_500);
    let s2 = s.start_s2().await;

    let out = s
        .ok(
            "read_session_output",
            json!({"session": reference(s2), "lines": 5000}),
        )
        .await;
    let lines = lines_of(&out);
    assert_eq!(lines.len(), 2_000);
    assert_eq!(lines.last().map(String::as_str), Some("ready>"));
    assert_eq!(lines[lines.len() - 2], "out2500");
    assert_eq!(out["truncated"], json!(true), "older lines exist");
}

/// FR-012: a count below 1 is invalid input, under every option value.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_line_count_below_one_is_invalid_input() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    for lines in [json!(0), json!(-3), json!("ten")] {
        let error = s
            .err(
                "read_session_output",
                json!({"session": reference(s2), "lines": lines.clone()}),
            )
            .await;
        assert_eq!(error["category"], "invalid_input", "{lines}: {error}");
    }
}

// ---------------------------------------------------------------------------------------
// send_session_input
// ---------------------------------------------------------------------------------------

/// A20 (US4 s2): at Auto the text reaches S2's primary process as typed, submitted once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_auto_the_text_is_typed_into_s2_and_submitted() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;

    let out = s
        .ok(
            "send_session_input",
            json!({"session": reference(s2), "text": TEXT}),
        )
        .await;
    assert_eq!(out, json!({}), "an acknowledgement and nothing else");
    // The terminal is in its line mode here, so the closing carriage return reads as a newline.
    s.typed_becomes(&format!("{TEXT}\n")).await;

    // And S1 can read what S2's terminal shows of it.
    let deadline = Instant::now() + BOUND;
    loop {
        let out = s
            .ok("read_session_output", json!({"session": reference(s2)}))
            .await;
        if lines_of(&out).iter().any(|l| l.contains(TEXT)) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the typed text never showed: {out}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// U200: two lines reach a CLI that has bracketed paste on as one bracketed submission ending in a
/// carriage return, so the line break inside does not submit the first line alone.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_two_line_text_is_one_bracketed_submission_ending_in_a_carriage_return() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.set_raw();
    let s2 = s.start_s2().await;
    // The stand-in sets raw mode after drawing its prompt; a write before that would be read in
    // line mode.
    s.raw_mode_is_on().await;

    s.ok(
        "send_session_input",
        json!({"session": reference(s2), "text": "line one\nline two\n"}),
    )
    .await;
    s.typed_becomes("\x1b[200~line one\nline two\x1b[201~\r")
        .await;
}

/// FR-012a: empty text is invalid input and types nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_text_is_invalid_input() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    for args in [
        json!({"session": reference(s2), "text": ""}),
        json!({"session": reference(s2)}),
        json!({"session": reference(s2), "text": 7}),
    ] {
        let error = s.err("send_session_input", args.clone()).await;
        assert_eq!(error["category"], "invalid_input", "{args}: {error}");
    }
    s.nothing_typed().await;
}

/// Text of line breaks only, or with a control character in it, is invalid input and types
/// nothing: not a bare Enter, and not a Ctrl-C that skips `interrupt_session`'s confirmation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn line_breaks_alone_and_control_characters_are_invalid_input() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    for text in ["\n", "\r\n", "\u{3}", "stop\u{3}", "up\u{1b}[A"] {
        let error = s
            .err(
                "send_session_input",
                json!({"session": reference(s2), "text": text}),
            )
            .await;
        assert_eq!(error["category"], "invalid_input", "{text:?}: {error}");
    }
    s.nothing_typed().await;
}

/// A CLI with no record that it trusts the folder may be showing its trust question, which the
/// submission's Enter would answer: nothing is typed and nobody is asked, as for the first prompt
/// of `create_session` (research R12).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cli_that_would_ask_to_trust_the_folder_gets_nothing_typed() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    s.distrust();
    s.set_access(CrossSessionAccess::ConfirmEachSend);
    let shown = answering_window(&s.state, true);

    let error = s
        .err(
            "send_session_input",
            json!({"session": reference(s2), "text": TEXT}),
        )
        .await;
    assert_eq!(error["category"], "conflict", "{error}");
    let message = error["message"].as_str().unwrap();
    assert!(
        message.contains("trust") && message.contains("Claude Code"),
        "names the CLI and the trust question: {message}"
    );
    assert!(shown.lock().unwrap().is_empty(), "nobody is asked");
    s.nothing_typed().await;

    // Reading types nothing, so it still works.
    s.ok("read_session_output", json!({"session": reference(s2)}))
        .await;
}

/// A session that has no running process has no terminal to read and nothing to type into: a
/// conflict that names the way out, not a silent success.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_session_that_is_not_running_is_a_conflict_naming_start_session() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    for (tool, args) in [
        (
            "read_session_output",
            json!({"session": reference(sid(IDLE))}),
        ),
        (
            "send_session_input",
            json!({"session": reference(sid(IDLE)), "text": TEXT}),
        ),
    ] {
        let error = s.err(tool, args).await;
        assert_eq!(error["category"], "conflict", "{tool}: {error}");
        assert!(
            error["message"].as_str().unwrap().contains("start_session"),
            "{tool}: {error}"
        );
    }
}

// ---------------------------------------------------------------------------------------
// The option (FR-016), the caller's own session (FR-015), the project boundary (FR-010)
// ---------------------------------------------------------------------------------------

/// A22 (US4 s4): at Off both tools are refused by policy and S2 is untouched.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_off_both_tools_are_refused_by_policy_and_s2_is_untouched() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    s.set_access(CrossSessionAccess::Off);

    for (tool, args) in [
        ("read_session_output", json!({"session": reference(s2)})),
        (
            "send_session_input",
            json!({"session": reference(s2), "text": TEXT}),
        ),
    ] {
        let error = s.err(tool, args).await;
        assert_eq!(error["category"], "refused_by_policy", "{tool}: {error}");
        assert!(
            error["message"].as_str().unwrap().contains("Settings"),
            "{tool}: the refusal says where the option is: {error}"
        );
    }
    s.nothing_typed().await;
}

/// A19 (US4 s1), with a window attached: a read at Confirm each send asks nobody.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_confirm_each_send_a_read_raises_no_confirmation() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    s.set_access(CrossSessionAccess::ConfirmEachSend);
    let shown = answering_window(&s.state, false);

    let out = s
        .ok("read_session_output", json!({"session": reference(s2)}))
        .await;
    assert!(
        lines_of(&out).ends_with(&["ready>".to_string()]),
        "the read went ahead: {out}"
    );
    assert!(
        shown.lock().unwrap().is_empty(),
        "a read never asks the user: {:#?}",
        shown.lock().unwrap()
    );
}

/// A23 (US4 s5), allowed: the send waits for the window's answer, and Allow delivers the text as
/// at Auto. The prompt names the caller, the operation and the target, never the text.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_confirm_each_send_an_allowed_send_is_delivered() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    s.set_access(CrossSessionAccess::ConfirmEachSend);
    let shown = answering_window(&s.state, true);

    let out = s
        .ok(
            "send_session_input",
            json!({"session": reference(s2), "text": TEXT}),
        )
        .await;
    assert_eq!(out, json!({}), "an acknowledgement and nothing else");
    s.typed_becomes(&format!("{TEXT}\n")).await;
    assert_one_send_prompt(&shown, &s.label(s2));
}

/// A23 (US4 s5), declined: the send is refused by policy and S2 is untouched.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_confirm_each_send_a_declined_send_is_refused_by_policy_and_s2_is_untouched() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    s.set_access(CrossSessionAccess::ConfirmEachSend);
    let shown = answering_window(&s.state, false);

    let error = s
        .err(
            "send_session_input",
            json!({"session": reference(s2), "text": TEXT}),
        )
        .await;
    assert_eq!(error["category"], "refused_by_policy", "{error}");
    assert_one_send_prompt(&shown, &s.label(s2));
    s.nothing_typed().await;
}

/// A23 (US4 s5), timed out: a window is shown the prompt and nobody answers; after the 60 s bound
/// the send fails as needing confirmation, the prompt is withdrawn and S2 is untouched. The clock
/// is paused only to step over the bound, once the prompt is in the window.
#[tokio::test(flavor = "current_thread")]
async fn at_confirm_each_send_an_unanswered_send_needs_confirmation_and_s2_is_untouched() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    s.set_access(CrossSessionAccess::ConfirmEachSend);
    let mut window = fake_window(&s.state);

    let (addr, bearer) = (s.addr, full_view_credential(&s.state, sid(S1)));
    let args = json!({"session": reference(s2), "text": TEXT});
    let call =
        tokio::spawn(async move { call_err(addr, &bearer, "send_session_input", args).await });

    let prompt = tokio::time::timeout(BOUND, async {
        loop {
            match window.recv().await.expect("the window stays connected") {
                Frame::Control(DaemonMsg::ConfirmationRequested { id, operation, .. }) => {
                    assert_eq!(operation, ConfirmOperation::SendInput);
                    return id;
                }
                _ => continue,
            }
        }
    })
    .await
    .expect("the window is shown the prompt");

    tokio::time::pause();
    tokio::time::advance(micold_daemon::mcp::confirm::CONFIRM_TIMEOUT).await;
    tokio::time::resume();

    let error = tokio::time::timeout(BOUND, call)
        .await
        .expect("the call returns once the bound passed")
        .unwrap();
    assert_eq!(error["category"], "needs_confirmation", "{error}");
    let withdrawn = tokio::time::timeout(BOUND, async {
        loop {
            if let Frame::Control(DaemonMsg::ConfirmationWithdrawn { id }) =
                window.recv().await.expect("the window stays connected")
            {
                return id;
            }
        }
    })
    .await
    .expect("the prompt is withdrawn");
    assert_eq!(withdrawn, prompt);
    s.nothing_typed().await;
}

/// A23, the part that needs no window: at Confirm each send with no window attached, a send fails
/// as needing confirmation and S2 is untouched (FR-014's no-window refusal).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_confirm_each_send_with_no_window_a_send_needs_confirmation_and_s2_is_untouched() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    s.set_access(CrossSessionAccess::ConfirmEachSend);

    let error = s
        .err(
            "send_session_input",
            json!({"session": reference(s2), "text": TEXT}),
        )
        .await;
    assert_eq!(error["category"], "needs_confirmation", "{error}");
    s.nothing_typed().await;
}

/// U201: a change of the option applies to the very next request of a session that is already
/// running, in both directions. (That `SettingsSet` reaches the service's copy is
/// `daemon_lifecycle.rs`; this is the service's copy reaching the next request.)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_change_of_the_option_applies_to_the_very_next_request() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let s2 = s.start_s2().await;
    let read = json!({"session": reference(s2), "lines": 1});
    let send = json!({"session": reference(s2), "text": TEXT});

    s.ok("read_session_output", read.clone()).await;

    s.set_access(CrossSessionAccess::Off);
    let error = s.err("read_session_output", read.clone()).await;
    assert_eq!(error["category"], "refused_by_policy", "{error}");
    let error = s.err("send_session_input", send.clone()).await;
    assert_eq!(error["category"], "refused_by_policy", "{error}");

    s.set_access(CrossSessionAccess::ConfirmEachSend);
    s.ok("read_session_output", read.clone()).await;
    s.nothing_typed().await;

    s.set_access(CrossSessionAccess::Auto);
    s.ok("send_session_input", send).await;
    s.typed_becomes(&format!("{TEXT}\n")).await;
}

/// FR-015: the caller's own session is invalid input for both tools, under every option value —
/// an agent reads its own terminal by other means and must not type into itself.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_callers_own_session_is_invalid_input_under_every_option_value() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    for access in CrossSessionAccess::ALL {
        s.set_access(access);
        for (tool, args) in [
            (
                "read_session_output",
                json!({"session": reference(sid(S1))}),
            ),
            (
                "send_session_input",
                json!({"session": reference(sid(S1)), "text": TEXT}),
            ),
        ] {
            let error = s.err(tool, args).await;
            assert_eq!(
                error["category"], "invalid_input",
                "{access:?} {tool}: {error}"
            );
        }
    }
}

/// A21 (US4 s3, EC-2): a session or worktree of another project fails exactly as one that does
/// not exist — same category, same words — under every option value, so nothing tells an agent
/// that the other project is there.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn another_projects_session_or_worktree_is_not_found_like_an_unknown_ref() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let session_tools: [TargetedTool; 7] = [
        ("get_session", |r| json!({"session": r})),
        ("start_session", |r| json!({"session": r})),
        ("read_session_output", |r| json!({"session": r})),
        (
            "send_session_input",
            |r| json!({"session": r, "text": "hello"}),
        ),
        ("stop_session", |r| json!({"session": r})),
        ("interrupt_session", |r| json!({"session": r})),
        ("delete_session", |r| json!({"session": r})),
    ];
    let worktree_tools: [TargetedTool; 5] = [
        ("list_sessions", |r| json!({"worktree": r})),
        (
            "rename_worktree",
            |r| json!({"worktree": r, "display_name": "Renamed"}),
        ),
        ("create_session", |r| json!({"worktree": r})),
        ("delete_worktree", |r| json!({"worktree": r})),
        (
            "delete_worktree",
            |r| json!({"worktree": r, "stop_sessions": true}),
        ),
    ];

    for access in CrossSessionAccess::ALL {
        s.set_access(access);
        let cases =
            session_tools
                .iter()
                .map(|(tool, args)| (*tool, *args, reference(sid(X)), reference(sid(UNKNOWN))))
                .chain(worktree_tools.iter().map(|(tool, args)| {
                    (*tool, *args, "elsewhere".to_string(), "nope".to_string())
                }));
        for (tool, args, theirs, unknown) in cases {
            let other = s.err(tool, args(&theirs)).await;
            let missing = s.err(tool, args(&unknown)).await;
            assert_eq!(other["category"], "not_found", "{access:?} {tool}: {other}");
            assert_eq!(
                missing["category"], "not_found",
                "{access:?} {tool}: {missing}"
            );
            assert_eq!(
                other["message"].as_str().unwrap().replace(&theirs, "<ref>"),
                missing["message"]
                    .as_str()
                    .unwrap()
                    .replace(&unknown, "<ref>"),
                "{access:?} {tool}: the two failures must read the same"
            );
        }
    }
    assert!(
        s.state.primary_pty(sid(X)).is_none(),
        "the other project's session was not started"
    );
    let theirs = s
        .state
        .catalog_snapshot()
        .projects
        .into_iter()
        .find(|p| p.sessions.iter().any(|s| s.id == sid(X)))
        .expect("the other project's session is still in the catalog");
    assert!(
        theirs.path.join(".claude/worktrees/elsewhere").is_dir(),
        "the other project's worktree was not deleted"
    );
}

// ---------------------------------------------------------------------------------------
// send_parent_input (spec 1945, contracts/mcp-neighbourhood.md; US3; FR-006..FR-008, SC-003)
// ---------------------------------------------------------------------------------------

fn to_parent() -> Value {
    json!({"text": TEXT})
}

/// The reply of a send that reached `session` in `parent`: `send_session_input`'s, plus both names.
fn reached(session: SessionId, parent: &str) -> Value {
    json!({"session": reference(session), "parent_worktree": parent})
}

/// US3 s1: the one session of the parent worktree gets the text, and the reply names it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn send_parent_input_types_into_the_one_session_of_the_parent_worktree() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.link_caller_to("b");
    let s2 = s.start_in("b", false).await;

    let out = s.child_ok("send_parent_input", to_parent()).await;
    assert_eq!(out, reached(s2, "b"));
    s.typed_becomes(&format!("{TEXT}\n")).await;
}

/// US3 s2: with two sessions in the parent worktree, both sends reach the first in catalog order.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_sends_to_a_parent_with_two_sessions_reach_the_same_first_session() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.link_caller_to("b");
    let first = s.start_in("b", false).await;
    let second = s.start_in("b", true).await;

    for _ in 0..2 {
        let out = s.child_ok("send_parent_input", to_parent()).await;
        assert_eq!(out, reached(first, "b"));
    }
    s.typed_becomes(&format!("{TEXT}\n{TEXT}\n")).await;
    assert!(
        wait_until(BOUND, || s.shown(first) == 2).await,
        "{:?}",
        s.screen(first)
    );
    assert_eq!(s.shown(second), 0, "{:?}", s.screen(second));
}

/// US3 s4: a worktree with no recorded parent reaches `default`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unparented_caller_reaches_its_implicit_parent_default() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.child_ok("start_session", json!({"session": reference(sid(IDLE))}))
        .await;
    s.ready(sid(IDLE)).await;

    let out = s.child_ok("send_parent_input", to_parent()).await;
    assert_eq!(out, reached(sid(IDLE), "default"));
    s.typed_becomes(&format!("{TEXT}\n")).await;
}

/// FR-008: at Off the send is refused by policy, as `send_session_input` is, and nothing is typed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_off_send_parent_input_is_refused_by_policy_and_nothing_is_typed() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.link_caller_to("b");
    s.start_in("b", false).await;
    s.set_access(CrossSessionAccess::Off);

    let error = s.child_err("send_parent_input", to_parent()).await;
    assert_eq!(error["category"], "refused_by_policy", "{error}");
    s.nothing_typed().await;
}

/// FR-008: at Confirm each send the user is asked about the parent session, by its label, and an
/// allowed send is delivered.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_confirm_each_send_the_parent_send_waits_for_the_user_and_names_the_target() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.link_caller_to("b");
    let s2 = s.start_in("b", false).await;
    s.set_access(CrossSessionAccess::ConfirmEachSend);
    let shown = answering_window(&s.state, true);

    let out = s.child_ok("send_parent_input", to_parent()).await;
    assert_eq!(out, reached(s2, "b"));
    s.typed_becomes(&format!("{TEXT}\n")).await;
    assert_one_send_prompt(&shown, &s.label(s2));
}

/// FR-008, SC-005: a running session outside the caller's scope is never the target, even when it
/// is the only running one; the refusal names the parent.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_running_session_outside_the_scope_is_never_reached() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.child_ok("start_session", json!({"session": reference(sid(IDLE))}))
        .await;
    s.ready(sid(IDLE)).await;
    // Now `default` is S1's grandparent: out of its scope.
    s.link_caller_to("b");

    let error = s.child_err("send_parent_input", to_parent()).await;
    assert_eq!(error["category"], "conflict", "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("\"b\""),
        "{error}"
    );
    s.nothing_typed().await;
}

/// Research R3: a plain shell listed first in the parent worktree is never the target; the text
/// reaches the parent's AI CLI session, and nothing is typed into the shell (where Enter would run it).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_shell_ahead_of_the_parents_ai_cli_session_is_never_reached() {
    let _guard = ENV.lock().await;
    let s = Sandbox::with(vec![session(
        sid(SHELL),
        Some("b"),
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    )])
    .await;
    s.link_caller_to("b");
    s.child_ok("start_session", json!({"session": reference(sid(SHELL))}))
        .await;
    let shell_runs = || {
        s.state
            .primary_pty(sid(SHELL))
            .is_some_and(|pty| pty.is_alive())
    };
    assert!(
        wait_until(BOUND, shell_runs).await,
        "the shell never started"
    );
    let ai = s.start_in("b", true).await;

    let out = s.child_ok("send_parent_input", to_parent()).await;
    assert_eq!(out, reached(ai, "b"));
    s.typed_becomes(&format!("{TEXT}\n")).await;
    assert_eq!(s.shown(sid(SHELL)), 0, "{:?}", s.screen(sid(SHELL)));
}

/// A live worktree directory literally named `default` that is the recorded parent is that
/// worktree, never the project root: with only a root session running, nothing is typed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_parent_directory_named_default_is_not_the_project_root() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    add_worktree(s._project.path(), "default");
    s.state.refresh_worktrees(s._project.path());
    s.link_caller_to("default");
    s.child_ok("start_session", json!({"session": reference(sid(IDLE))}))
        .await;
    s.ready(sid(IDLE)).await;

    let error = s.child_err("send_parent_input", to_parent()).await;
    assert_eq!(error["category"], "conflict", "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("\"default\""),
        "{error}"
    );
    s.nothing_typed().await;
}
