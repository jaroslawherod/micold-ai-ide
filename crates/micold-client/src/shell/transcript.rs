//! The transcript view's scroll operations (feature 939, research R9).
//!
//! The root applies a `TranscriptMsg` and says which scroll it needs; this performs it on the
//! session's scrollable. Following is held by iced's relative offset: `snap_to(END)` is resolved
//! against the content height at each layout, so growth keeps the end in view with no task per
//! event, and a user scroll turns it into an absolute offset that growth below does not move.

use iced::widget::scrollable::AbsoluteOffset;
use iced::Task;

use micold_client::app::Message;
use micold_client::features::structured_transcript::{Scroll, TranscriptMsg};
use micold_client::ui::transcript::transcript_scroll_id;

use crate::App;

/// Apply `msg` and issue the scroll it asks for, if any.
pub fn update(app: &mut App, msg: TranscriptMsg) -> Task<Message> {
    let session = match &msg {
        TranscriptMsg::Toggle { session, .. }
        | TranscriptMsg::Scrolled { session, .. }
        | TranscriptMsg::JumpToLatest { session }
        | TranscriptMsg::Shown { session } => *session,
    };
    let id = transcript_scroll_id(session);
    match app.core.update_transcript(msg) {
        None => Task::none(),
        Some(Scroll::End) => iced::widget::operation::snap_to_end(id),
        Some(Scroll::To(y)) => iced::widget::operation::scroll_to(
            id,
            AbsoluteOffset {
                x: 0.0,
                y: y as f32,
            },
        ),
    }
}
