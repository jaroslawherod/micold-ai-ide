//! The system clipboard (feature 021, T055 — FR-015a, FR-019a).
//!
//! The smallest external system in the shell and the one with the strictest rule about it: nothing
//! outside this directory may reach the clipboard at all (contract C1). A feature that called
//! `iced::clipboard::write` directly could not be tested without a windowing system, so a feature
//! that wants something copied emits [`Outcome::ClipboardWrite`] and [`interpret`] performs it.
//!
//! # `interpret` decides nothing, and that is checked
//!
//! One arm per variant, no branch. Whether an effect *should* happen belongs to the feature that
//! emits the request; translating it is all the shell does. `tests/clipboard_request.rs` reads
//! this function's body and fails on an `if`, an `else`, an `unwrap_or` or an `is_empty` — because
//! a body that grew one would still compile and still pass every behavioural test.
//!
//! # The read is not an outcome, and deliberately so
//!
//! `Outcome` carries requests *out* of a feature. A paste is a request for something to come
//! back, and it already has a way home: the clipboard read resolves into `TerminalBytes`, which is
//! the same message a keystroke arrives as and is therefore already subject to the write-gate and
//! the displacement check. Modelling it as an outcome would add a second inbound path to a
//! feature that has one (T056 recorded this decision; this module is where it is visible).

use iced::Task;

use micold_client::app::Message;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_client::features::Outcome;
use micold_client::keymap;
use micold_client::selection;

use crate::App;

/// The shell's whole translation of an effect request into an effect (FR-015a).
pub fn interpret(outcome: Outcome) -> Task<Message> {
    match outcome {
        Outcome::ClipboardWrite(text) => iced::clipboard::write(text),
        // The root's, not the shell's: cross-feature consequences are interpreted by
        // `app::interpret` (contract O3). The shell partitions the queue before draining, so these
        // are unreachable here — listed rather than caught by a `_` so a new *effect request*
        // variant is a compile error in this file, which is where it would need an arm.
        Outcome::SessionsClosed(_)
        | Outcome::OverlayDismissed(_)
        | Outcome::NotificationRaised(_)
        | Outcome::WorktreesReplaced(_)
        | Outcome::ChangesRequested(_)
        | Outcome::ChangesClosedForCompare
        | Outcome::RunInParallelRequested
        | Outcome::WorktreeCreated(_)
        | Outcome::LocationOpened(_)
        | Outcome::RevealScrollArmed
        | Outcome::ProjectEntered
        | Outcome::RevealSuppressed(_)
        | Outcome::FieldFocusCleared
        | Outcome::OpenLink(_)
        | Outcome::Insert { .. }
        | Outcome::SurfaceOpened(_) => Task::none(),
    }
}

/// What the displayed session's current selection asks to have copied, if anything.
///
/// The feature, not the shell, is the one that shrugs: a selection outlives the lines it points
/// at — it is anchored to absolute `LineId`s precisely so it can — so an unresolvable selection
/// and a whitespace-only one both come back as `None` from `selection::copy_request` rather than
/// being filtered here.
pub fn selection_copy_request(app: &App) -> Option<Outcome> {
    let grid = app.grids.get(&app.displayed_terminal()?)?;
    selection::copy_request(app.selection.as_ref(), |id| {
        grid.line(id).map(|l| l.text.clone())
    })
}

/// Copy the current selection to the system clipboard (FR-013). Also closes the menu.
pub fn on_copy_requested(app: &mut App) -> Task<Message> {
    app.core
        .update(Message::Session(SessionMsg::TerminalContextMenuClosed));
    selection_copy_request(app).map_or_else(Task::none, interpret)
}

/// Auto-copy the current selection when a selection gesture ends (FR-013).
///
/// Unlike [`on_copy_requested`] this leaves the context menu alone, and it reads `app.selection`
/// after the gesture's own `TerminalSelectStart`/`TerminalSelectUpdate` were applied — so a click,
/// which selects nothing, copies nothing (FR-013e, BUG-008).
pub fn on_selection_released(app: &mut App) -> Task<Message> {
    selection_copy_request(app).map_or_else(Task::none, interpret)
}

/// Where a paste takes its content from (feature 487, FR-006, FR-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteSource {
    /// The clipboard's text, as every paste has always done.
    Text,
    /// The clipboard's image, saved to a file whose path is typed instead.
    Image,
}

/// The image only when there is no text, an image, and an AI session to hand it to. Text always
/// wins: a copied spreadsheet range or web image carries both, and its text is what the user meant.
pub fn paste_source(text: &str, has_image: bool, is_ai_session: bool) -> PasteSource {
    if text.is_empty() && has_image && is_ai_session {
        PasteSource::Image
    } else {
        PasteSource::Text
    }
}

/// Paste the system clipboard into the displayed session's PTY (FR-013). The read is async;
/// its result flows back through `TerminalBytes`, which honours the Running write-gate.
///
/// Bracketed like the chord and middle-click (FR-013d, BUG-006). Whether to bracket is read now,
/// when the user chose Paste, not when the clipboard answers: that is the mode the user saw.
///
/// With no text on the clipboard, in an AI session, an image there is saved and its path is typed
/// instead (feature 487): see [`paste_source`].
pub fn on_paste_requested(app: &mut App) -> Task<Message> {
    app.core
        .update(Message::Session(SessionMsg::TerminalContextMenuClosed));
    let bracketed = app.attached_grid().is_some_and(|g| g.bracketed_paste());
    let image_target = image_target(app);
    iced::clipboard::read().then(move |c| {
        let text = c.unwrap_or_default();
        match (
            paste_source(&text, true, image_target.is_some()),
            &image_target,
        ) {
            (PasteSource::Image, Some(target)) => {
                let target = target.clone();
                Task::perform(
                    save_clipboard_image(target.clone()),
                    move |saved| match saved {
                        // No image after all: an empty paste, as before.
                        Ok(None) => text_message("", bracketed),
                        Ok(Some(path)) => Message::Session(SessionMsg::ImagePasted {
                            terminal: target.terminal,
                            shell: target.shell,
                            result: Ok(path),
                        }),
                        Err(reason) => Message::Session(SessionMsg::ImagePasted {
                            terminal: target.terminal,
                            shell: target.shell,
                            result: Err(reason),
                        }),
                    },
                )
            }
            _ => Task::done(text_message(&text, bracketed)),
        }
    })
}

/// `text` as the bytes of a paste.
fn text_message(text: &str, bracketed: bool) -> Message {
    Message::Session(SessionMsg::TerminalBytes(keymap::paste_bytes(
        text, bracketed,
    )))
}

/// What saving a pasted image needs, gathered when the user chose Paste.
#[derive(Debug, Clone)]
struct ImageTarget {
    terminal: micold_core::protocol::messages::TerminalRef,
    shell: micold_core::path_insert::ShellKind,
    worktree: Option<(std::path::PathBuf, micold_core::path_insert::PastedLayout)>,
    fallback: micold_core::path_insert::PastedLayout,
}

/// The displayed terminal as a target for an image, when it is an AI session's own (a regular
/// terminal pastes as before).
fn image_target(app: &App) -> Option<ImageTarget> {
    use micold_core::path_insert::PastedLayout;
    use micold_core::protocol::messages::SessionProcess;
    use micold_core::session::SessionLocation;
    let terminal = app.displayed_terminal()?;
    if terminal.process != SessionProcess::Primary {
        return None;
    }
    let (repo, session) = app.core.workspace.find_session(terminal.session)?;
    let data_dir = directories::ProjectDirs::from("", "", "micold-ai-ide")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(std::env::temp_dir);
    let worktree = match &session.location {
        SessionLocation::Worktree(_) => {
            let root = session.location.cwd(repo);
            let layout = PastedLayout::in_worktree(&root, terminal.session);
            Some((root, layout))
        }
        SessionLocation::Default => None,
    };
    Some(ImageTarget {
        terminal,
        shell: crate::shell::drops::login_shell_kind(),
        worktree,
        fallback: PastedLayout::in_data_dir(&data_dir, terminal.session),
    })
}

/// Distinguishes two pastes that land in the same nanosecond.
static PASTE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Read and save the clipboard's image off the UI thread. `Ok(None)`: it holds no image.
async fn save_clipboard_image(target: ImageTarget) -> Result<Option<std::path::PathBuf>, String> {
    let job = move || {
        let Some(image) = crate::shell::pasted_image::read_clipboard()? else {
            return Ok(None);
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let seq = PASTE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let worktree = target.worktree.as_ref().map(|(r, l)| (r.as_path(), l));
        crate::shell::pasted_image::save(&image, worktree, &target.fallback, now, seq).map(Some)
    };
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|e| format!("Nothing was inserted: the image could not be read ({e})."))?
}

/// An image paste finished: type the saved file's quoted path, or show why there is none.
pub fn on_image_pasted(
    app: &mut App,
    terminal: micold_core::protocol::messages::TerminalRef,
    shell: micold_core::path_insert::ShellKind,
    result: Result<std::path::PathBuf, String>,
) -> Task<Message> {
    let effects = app
        .core
        .update_session_for_effects(SessionMsg::ImagePasted {
            terminal,
            shell,
            result,
        });
    Task::batch(effects.into_iter().map(|effect| match effect {
        Outcome::Insert { terminal, text } => crate::shell::drops::insert(app, terminal, &text),
        other => interpret(other),
    }))
}

/// Copy arbitrary displayed text (e.g. a worktree name) to the system clipboard, so
/// labels the app itself doesn't make selectable are still reachable cross-application.
/// Also closes the worktree context menu, mirroring its other actions (idempotent if
/// the text wasn't copied from that menu).
/// The text is the request: the view named it (`ui::worktree_menu_items` asks the worktree
/// feature for the display name), so there is nothing left here to decide and no second
/// emitter to write. Translating it is all the shell does.
pub fn on_text_copy_requested(app: &mut App, text: String) -> Task<Message> {
    app.core
        .update(Message::Worktree(WorktreeMsg::MenuDismissed));
    interpret(Outcome::ClipboardWrite(text))
}

#[cfg(test)]
mod tests {
    use super::{paste_source, PasteSource};

    #[test]
    fn text_alone_or_with_an_image_pastes_as_text() {
        assert_eq!(paste_source("hi", false, true), PasteSource::Text);
        assert_eq!(paste_source("hi", true, true), PasteSource::Text);
    }

    #[test]
    fn an_image_alone_in_an_ai_session_pastes_as_an_image() {
        assert_eq!(paste_source("", true, true), PasteSource::Image);
    }

    #[test]
    fn an_image_alone_in_a_regular_terminal_pastes_as_before() {
        assert_eq!(paste_source("", true, false), PasteSource::Text);
    }

    #[test]
    fn nothing_on_the_clipboard_pastes_as_text() {
        assert_eq!(paste_source("", false, true), PasteSource::Text);
    }
}
