//! The structured session's transcript view (feature 939). M3 holds only the scrollable's id;
//! the view arrives with the renderer seam.

use micold_core::session::SessionId;

/// The transcript scrollable's id for `session` (`transcript-<session uuid>`, research R9): the
/// shell's `snap_to` and `scroll_to` tasks name it, so each session's view scrolls on its own.
pub fn transcript_scroll_id(session: SessionId) -> iced::advanced::widget::Id {
    iced::advanced::widget::Id::from(format!("transcript-{session}"))
}
