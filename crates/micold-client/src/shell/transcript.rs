//! The transcript view's shell half (feature 939, research R9; feature 940, contracts/
//! rich-renderers.md *Shell*).
//!
//! The root applies a `TranscriptMsg` and says what it needs; this performs it. Following is held
//! by iced's relative offset: `snap_to(END)` is resolved against the content height at each
//! layout, so growth keeps the end in view with no task per event, and a user scroll turns it into
//! an absolute offset that growth below does not move. The rich blocks' requests are thin glue:
//! highlight jobs on blocking tasks under a time budget, the checked clipboard write through
//! `shell/clipboard.rs`, the confirmation timer, and links through `shell/links.rs`.

use std::time::Duration;

use iced::widget::scrollable::AbsoluteOffset;
use iced::Task;

use micold_client::app::Message;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::structured_transcript::{Scroll, TranscriptMsg};
use micold_client::features::transcript_rich::HighlightJob;
use micold_client::features::Outcome;
use micold_client::ui::syntax;
use micold_client::ui::transcript::transcript_scroll_id;

use crate::App;

/// How long one highlight job may colour before the block stays plain (the Changes view's
/// `highlight_within` budget, #771).
const HIGHLIGHT_BUDGET: Duration = Duration::from_secs(5);

/// Apply `msg` and perform what it asks for.
pub fn update(app: &mut App, msg: TranscriptMsg) -> Task<Message> {
    let session = msg.session();
    let effect = app.core.update_transcript(msg);
    let scroll = match (session, effect.scroll) {
        (Some(session), Some(Scroll::End)) => {
            iced::widget::operation::snap_to_end(transcript_scroll_id(session))
        }
        (Some(session), Some(Scroll::To(y))) => iced::widget::operation::scroll_to(
            transcript_scroll_id(session),
            AbsoluteOffset {
                x: 0.0,
                y: y as f32,
            },
        ),
        _ => Task::none(),
    };
    let outcomes = effect
        .outcomes
        .into_iter()
        .map(|outcome| perform(app, outcome));
    Task::batch(
        std::iter::once(scroll)
            .chain(outcomes)
            .chain(std::iter::once(highlight(effect.jobs))),
    )
}

/// One effect request of the transcript's (contracts/rich-renderers.md *Shell*).
fn perform(app: &App, outcome: Outcome) -> Task<Message> {
    match outcome {
        Outcome::OpenLink(request) => crate::shell::links::perform(app, request),
        Outcome::CopyConfirmTimer { ticket, after } => {
            Task::perform(async move { tokio::time::sleep(after).await }, move |()| {
                transcript(TranscriptMsg::CopyExpired { ticket })
            })
        }
        other => crate::shell::clipboard::interpret(other),
    }
}

fn transcript(msg: TranscriptMsg) -> Message {
    Message::Session(SessionMsg::Transcript(msg))
}

/// Run `jobs` off the update thread, each in both schemes under [`HIGHLIGHT_BUDGET`], answering
/// with `TranscriptMsg::Highlighted`; past the budget the part stays plain (research R4).
pub fn highlight(jobs: Vec<HighlightJob>) -> Task<Message> {
    Task::batch(jobs.into_iter().map(|job| {
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let spans = syntax::highlight_job(&job.token, &job.lines, HIGHLIGHT_BUDGET);
                    if spans.is_none() {
                        crate::log_line(&format!(
                            "transcript: {} lines of {} not highlighted, over {}s",
                            job.lines.len(),
                            job.token,
                            HIGHLIGHT_BUDGET.as_secs()
                        ));
                    }
                    spans.map(|spans| TranscriptMsg::Highlighted {
                        session: job.session,
                        key: job.key,
                        part: job.part,
                        spans,
                    })
                })
                .await
                .ok()
                .flatten()
            },
            std::convert::identity,
        )
        .then(|done| done.map_or_else(Task::none, |msg| Task::done(transcript(msg))))
    }))
}
