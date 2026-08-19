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
use micold_client::features::Outcome;
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
        | Outcome::WorktreeCreated(_) => Task::none(),
    }
}

/// What the displayed session's current selection asks to have copied, if anything.
///
/// The feature, not the shell, is the one that shrugs: a selection outlives the lines it points
/// at — it is anchored to absolute `LineId`s precisely so it can — so an unresolvable selection
/// and a whitespace-only one both come back as `None` from `selection::copy_request` rather than
/// being filtered here.
pub fn selection_copy_request(app: &App) -> Option<Outcome> {
    let grid = app.core.active_session.and_then(|id| app.grids.get(&id))?;
    selection::copy_request(app.selection.as_ref(), |id| {
        grid.line(id).map(|l| l.text.clone())
    })
}

/// Copy the current selection to the system clipboard (FR-013). Also closes the menu.
pub fn on_copy_requested(app: &mut App) -> Task<Message> {
    app.core.update(Message::TerminalContextMenuClosed);
    selection_copy_request(app).map_or_else(Task::none, interpret)
}

/// Paste the system clipboard into the displayed session's PTY (FR-013). The read is async;
/// its result flows back through `TerminalBytes`, which honours the Running write-gate.
pub fn on_paste_requested(app: &mut App) -> Task<Message> {
    app.core.update(Message::TerminalContextMenuClosed);
    iced::clipboard::read().map(|c| Message::TerminalBytes(c.unwrap_or_default().into_bytes()))
}

/// Copy arbitrary displayed text (e.g. a worktree name) to the system clipboard, so
/// labels the app itself doesn't make selectable are still reachable cross-application.
/// Also closes the worktree context menu, mirroring its other actions (idempotent if
/// the text wasn't copied from that menu).
/// The text is the request: the view named it (`ui::worktree_menu_items` asks the worktree
/// feature for the display name), so there is nothing left here to decide and no second
/// emitter to write. Translating it is all the shell does.
pub fn on_text_copy_requested(app: &mut App, text: String) -> Task<Message> {
    app.core.update(Message::WorktreeMenuDismissed);
    interpret(Outcome::ClipboardWrite(text))
}
