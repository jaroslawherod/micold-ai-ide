//! The embedded terminal: the real `portable-pty`-backed [`TerminalBackend`] impl, VT/ANSI
//! interpretation via `alacritty_terminal`, and the terminal pane rendering (gui-only).
//! Research R1/R2/R4/R6.
//!
//! A session's `claude` process runs in a PTY; a reader thread streams its raw bytes into a
//! shared buffer, which is fed into an `alacritty_terminal::Term` (a VT emulator) so escape
//! sequences are interpreted into a character grid rather than shown literally (FR-012). The
//! pane renders that grid as monospace text and sends input lines to the PTY writer (FR-014).
//! The [`crate::terminal::TerminalBackend`]/`SessionRouter` seam keeps this local.

use crate::ui::style;
use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;
use iced::widget::{column, container, row, scrollable, text, text_input};
use iced::{Element, Font, Length};
use micold_ai_ide::app::{Message, State};
use micold_ai_ide::session::{SessionId, SessionLifecycle};
use micold_ai_ide::terminal::{claude_args, LaunchSpec, TerminalBackend, TerminalHandle};
use micold_ai_ide::tokens::{self, spacing, type_scale};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// The PTY (and terminal grid) dimensions. Fixed for now; a resize pass is future work (T058).
const ROWS: u16 = 30;
const COLS: u16 = 100;

/// Fixed terminal dimensions for the `alacritty_terminal` grid.
struct TermDimensions {
    rows: usize,
    cols: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

/// A live PTY session: writer + child, a shared raw-output buffer the reader thread appends
/// to, and the VT emulator that interprets it into a renderable grid.
pub struct RuntimeTerminal {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Raw bytes streamed from the child, awaiting interpretation (drained by [`Self::pump`]).
    output: Arc<Mutex<Vec<u8>>>,
    /// The VT emulator turning the byte stream into a character grid.
    term: Term<VoidListener>,
    parser: Processor,
    rows: usize,
    cols: usize,
}

impl RuntimeTerminal {
    /// Feed any newly-streamed bytes into the VT emulator (call on the UI tick).
    pub fn pump(&mut self) {
        let bytes = std::mem::take(&mut *self.output.lock().unwrap());
        if !bytes.is_empty() {
            self.parser.advance(&mut self.term, &bytes);
        }
    }

    /// The current visible screen as plain text (escape sequences already interpreted).
    pub fn screen_text(&self) -> String {
        let mut grid = vec![vec![' '; self.cols]; self.rows];
        let content = self.term.renderable_content();
        for cell in content.display_iter {
            let line = cell.point.line.0;
            let col = cell.point.column.0;
            if line >= 0 && (line as usize) < self.rows && col < self.cols {
                let c = cell.c;
                grid[line as usize][col] = if c == '\0' { ' ' } else { c };
            }
        }
        grid.into_iter()
            .map(|r| r.into_iter().collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Write raw bytes (e.g. a submitted line + newline) to the child (FR-014).
    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    /// Resize the PTY to `rows`×`cols`.
    pub fn resize(&mut self, rows: u16, cols: u16) -> std::io::Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(std::io::Error::other)
    }

    /// Terminate the child process (FR-015a).
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }

    /// Whether the child has exited (drives crash/exit handling, FR-022).
    pub fn has_exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }
}

impl TerminalHandle for RuntimeTerminal {
    fn write_input(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.write(bytes)
    }
    fn resize(&mut self, rows: u16, cols: u16) -> std::io::Result<()> {
        RuntimeTerminal::resize(self, rows, cols)
    }
    fn kill(&mut self) -> std::io::Result<()> {
        RuntimeTerminal::kill(self)
    }
}

/// Spawn `claude` for `spec` in a PTY and start streaming its output (research R1/R4/R6).
pub fn spawn_pty(spec: &LaunchSpec) -> std::io::Result<RuntimeTerminal> {
    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize {
            rows: ROWS,
            cols: COLS,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(std::io::Error::other)?;

    let mut cmd = CommandBuilder::new("claude");
    cmd.cwd(&spec.cwd);
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    for arg in claude_args(spec) {
        cmd.arg(arg);
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(std::io::Error::other)?;
    // Drop the slave so EOF propagates once the child exits.
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(std::io::Error::other)?;
    let writer = pair.master.take_writer().map_err(std::io::Error::other)?;

    let output = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&output);
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => sink.lock().unwrap().extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
    });

    let dims = TermDimensions {
        rows: ROWS as usize,
        cols: COLS as usize,
    };
    let term = Term::new(Config::default(), &dims, VoidListener);
    let parser = Processor::new();

    Ok(RuntimeTerminal {
        master: pair.master,
        writer,
        child,
        output,
        term,
        parser,
        rows: ROWS as usize,
        cols: COLS as usize,
    })
}

/// The production [`TerminalBackend`] (constitution seam per contracts/terminal-backend-trait.md).
/// The binary uses [`spawn_pty`] directly so it can also read the interpreted grid for
/// rendering; this impl keeps the abstract seam satisfiable (and swappable for `iced_term`).
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct PtyTerminalBackend;

impl TerminalBackend for PtyTerminalBackend {
    fn spawn(&self, spec: LaunchSpec) -> std::io::Result<Box<dyn TerminalHandle>> {
        Ok(Box::new(spawn_pty(&spec)?))
    }
}

/// Render the terminal pane for the active session (FR-012). `output` is the interpreted
/// screen text of the active session (supplied by the binary); `None` renders an empty state.
pub fn pane<'a>(
    state: &'a State,
    output: Option<&str>,
    scheme: micold_ai_ide::theme::ColorScheme,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let Some(active) = state.active_session else {
        return container(
            text("Select or start a session to open its terminal.")
                .size(type_scale::BODY)
                .style(style::muted(r)),
        )
        .padding(spacing::LG)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into();
    };

    let status = session_status(state, active);
    let header = row![
        text(session_title(state, active)).size(type_scale::TITLE),
        container(text("").width(Length::Fill)).width(Length::Fill),
        text(status).size(type_scale::LABEL).style(style::muted(r)),
    ]
    .spacing(spacing::SM);

    let body = scrollable(
        text(output.unwrap_or("").to_string())
            .font(Font::MONOSPACE)
            .size(type_scale::LABEL),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    let input = text_input("Type a message and press Enter…", &state.terminal_input)
        .on_input(Message::TerminalInputChanged)
        .on_submit(Message::TerminalLineSubmitted)
        .font(Font::MONOSPACE)
        .padding(spacing::SM)
        .style(style::input(r));

    container(
        column![
            header,
            container(body).height(Length::Fill).width(Length::Fill),
            input
        ]
        .spacing(spacing::SM),
    )
    .padding(spacing::MD)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn session_title(state: &State, id: SessionId) -> String {
    state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.label.display().to_string())
        .unwrap_or_else(|| "Session".to_string())
}

fn session_status(state: &State, id: SessionId) -> &'static str {
    match state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.lifecycle)
    {
        Some(SessionLifecycle::Running) => "running",
        Some(SessionLifecycle::Starting) => "starting…",
        Some(SessionLifecycle::Restarting { .. }) => "restarting…",
        Some(SessionLifecycle::Failed) => "failed",
        Some(SessionLifecycle::Idle) => "idle",
        None => "",
    }
}
