//! Files dropped on a terminal pane (feature 487).
//!
//! The window reports a drop as one event per file, with no pointer position. The pane widget
//! (`SplitView`) knows where the pointer is and says which pane each file landed on; this module
//! gathers the files of one drop, asks the session reducer what to type, and types it into the
//! terminal that pane shows. Nothing is submitted (FR-003): the text carries no newline of ours.

use std::time::Duration;

use iced::Task;
use micold_client::app::Message;
use micold_client::features::session::{DropTarget, Msg as SessionMsg, PaneMsg};
use micold_client::features::Outcome;
use micold_client::keymap;
use micold_core::pane_layout::PaneId;
use micold_core::protocol::messages::TerminalRef;
use std::path::PathBuf;

use crate::App;

/// How long after the first file of a drop the rest are waited for. The files of one drop arrive in
/// the same burst, so this is short; it is far inside SC-001's second.
const SETTLE: Duration = Duration::from_millis(40);

/// A file landed on `pane`: remember it, and start the wait for the rest of its drop.
pub fn on_file_dropped(app: &mut App, pane: PaneId, path: PathBuf) -> Task<Message> {
    let first = app.pending_drops.is_empty();
    app.pending_drops.push((pane, path));
    if !first {
        return Task::none();
    }
    Task::future(async {
        tokio::time::sleep(SETTLE).await;
        Message::Session(SessionMsg::Pane(PaneMsg::DropsSettled))
    })
}

/// The files of a drop are all in: insert each pane's files as one run of paths, in drop order.
pub fn on_drops_settled(app: &mut App) -> Task<Message> {
    let mut groups: Vec<(PaneId, Vec<PathBuf>)> = Vec::new();
    for (pane, path) in std::mem::take(&mut app.pending_drops) {
        match groups.last_mut() {
            Some((last, paths)) if *last == pane => paths.push(path),
            _ => groups.push((pane, vec![path])),
        }
    }
    // Known limit (M1): the user's login shell, not the one this terminal runs. A shell instance
    // started with another shell, or an AI CLI's own prompt, is quoted for the default shell.
    let shell = login_shell_kind();
    let sandbox = match crate::shell::sandbox::share(app) {
        Ok(sandbox) => sandbox,
        Err(reason) => {
            app.core
                .update(Message::Session(SessionMsg::InsertionFailed(reason)));
            return Task::none();
        }
    };
    Task::batch(groups.into_iter().map(|(pane, paths)| {
        let target = target_of(app, pane);
        let effects = app
            .core
            .update_session_for_effects(SessionMsg::FilesDropped {
                paths,
                target,
                shell,
                sandbox: sandbox.clone(),
            });
        Task::batch(effects.into_iter().map(|effect| match effect {
            Outcome::Insert { terminal, text } => insert(app, terminal, &text),
            other => crate::shell::clipboard::interpret(other),
        }))
    }))
}

/// The user's login shell, which is the shell inserted text is quoted for.
pub fn login_shell_kind() -> micold_core::path_insert::ShellKind {
    micold_core::path_insert::ShellKind::detect(&micold_core::terminal::default_shell_command(
        std::env::var("SHELL").ok().as_deref(),
        std::env::var("COMSPEC").ok().as_deref(),
    ))
}

/// What `pane` shows now. A pane closed between the drop and the settle is `Outside`.
fn target_of(app: &App, pane: PaneId) -> DropTarget {
    crate::shell::panes::layout(app)
        .and_then(|l| l.panes().into_iter().find(|p| p.id() == pane))
        .map_or(DropTarget::Outside, |p| match p.terminal() {
            Some(t) => DropTarget::Terminal(t),
            None => DropTarget::EmptyPane,
        })
}

/// Type `text` at `terminal`'s input. Bracketed when the program asked for it, as a paste is, so a
/// shell shows the text instead of acting on it (FR-003).
///
/// A terminal that did not ask for bracketing would take a control character inside a name as a
/// key (a line break is Enter, a tab completes, `^C` interrupts), so such a name is refused there
/// rather than typed (SC-004).
pub fn insert(app: &mut App, terminal: TerminalRef, text: &str) -> Task<Message> {
    let bracketed = app
        .grids
        .get(&terminal)
        .is_some_and(|g| g.bracketed_paste());
    if !bracketed && has_control_character(text) {
        app.core
            .update(Message::Session(SessionMsg::InsertionFailed(
            "Nothing was inserted: a file name contains a control character, and this terminal \
                 would act on it."
                .to_string(),
        )));
        return Task::none();
    }
    crate::shell::daemon_sync::send_to_terminal(
        app,
        terminal,
        keymap::paste_bytes(text, bracketed),
    );
    Task::none()
}

/// Whether typing `text` unbracketed would press a key: any control character, a line break first.
fn has_control_character(text: &str) -> bool {
    text.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::{has_control_character, insert};
    use micold_client::grid::GridCache;
    use micold_core::protocol::grid::{GridFrame, LineId, WireCursor, WireCursorShape};
    use micold_core::protocol::messages::{ClientMsg, SessionProcess, TerminalRef};
    use micold_core::session::SessionId;

    type Sent = iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>;

    /// An app with a live outbox and a terminal whose grid has (or has not) enabled DECSET 2004.
    fn app_with_terminal(bracketed: bool) -> (crate::App, TerminalRef, Sent) {
        let (tx, rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = crate::tests::base_app();
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        let t = TerminalRef {
            session: SessionId::new(),
            process: SessionProcess::Primary,
        };
        let mut grid = GridCache::new();
        grid.apply(&GridFrame {
            process: SessionProcess::Primary,
            session: t.session,
            seq: 1,
            generation: 1,
            full: true,
            viewport_top: LineId(0),
            oldest_available: LineId(0),
            cols: 80,
            rows: 2,
            cursor: WireCursor {
                line: LineId(0),
                col: 0,
                shape: WireCursorShape::Block,
                visible: true,
                blinking: false,
            },
            styles: Vec::new(),
            hyperlinks: Vec::new(),
            lines: Vec::new(),
            mode: if bracketed {
                alacritty_terminal::term::TermMode::BRACKETED_PASTE.bits()
            } else {
                0
            },
            input_serial: None,
        });
        assert_eq!(grid.bracketed_paste(), bracketed);
        app.grids.insert(t, grid);
        (app, t, rx)
    }

    fn sent_bytes(rx: &mut Sent) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        while let Ok(m) = rx.try_recv() {
            if let ClientMsg::SessionInput { bytes, .. } = m {
                out.push(bytes);
            }
        }
        out
    }

    /// SC-004: a name with a line break typed unbracketed would press Enter.
    #[test]
    fn a_name_with_a_line_break_is_refused_and_announced_when_the_terminal_did_not_ask_for_bracketing(
    ) {
        let (mut app, t, mut rx) = app_with_terminal(false);
        let _ = insert(&mut app, t, "'a\nb'");
        assert!(sent_bytes(&mut rx).is_empty(), "nothing may be typed");
        assert!(app.core.notifications.queue.visible().is_some());
    }

    #[test]
    fn a_name_with_a_line_break_is_sent_bracketed_when_the_terminal_asked_for_it() {
        let (mut app, t, mut rx) = app_with_terminal(true);
        let _ = insert(&mut app, t, "'a\nb'");
        let sent = sent_bytes(&mut rx);
        assert_eq!(sent.len(), 1, "{sent:?}");
        assert!(sent[0].starts_with(b"\x1b[200~"), "{:?}", sent[0]);
        assert!(!app.core.notifications.queue.is_active());
    }

    #[test]
    fn a_name_with_a_control_character_is_not_typed_unbracketed() {
        for name in [
            "a\nb", "a\rb", "a\tb", "a\x03b", "a\x1bb", "a\x7fb", "a\u{85}b",
        ] {
            assert!(has_control_character(&format!("'{name}'")), "{name:?}");
        }
        assert!(!has_control_character("'/d/a b.png' '/d/naïve 日本語'"));
    }
}
