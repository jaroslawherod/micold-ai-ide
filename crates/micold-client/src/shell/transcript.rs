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
use micold_client::features::transcript_selection::SelectMsg;
use micold_client::features::Outcome;
use micold_client::ui::syntax;
use micold_client::ui::transcript::transcript_scroll_id;
use micold_core::protocol::messages::{SessionProcess, TerminalRef};

use crate::App;

/// How long one highlight job may colour before the block stays plain (the Changes view's
/// `highlight_within` budget, #771).
const HIGHLIGHT_BUDGET: Duration = Duration::from_secs(5);

/// Apply `msg` and perform what it asks for.
pub fn update(app: &mut App, msg: TranscriptMsg) -> Task<Message> {
    let session = msg.session();
    // #1550 M6 (FR-008, *Multi-session*): a press to select focuses its pane first, as a press on
    // the pane's header does. The selection area takes the press, so the pane's own `mouse_area`
    // never hears it, and the copy keys listen on the focused pane's transcript only.
    if let TranscriptMsg::Select(SelectMsg::Press { session, .. }) = msg {
        crate::shell::panes::focus_pane_showing(
            app,
            TerminalRef {
                session,
                process: SessionProcess::Primary,
            },
        );
    }
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
        (Some(session), Some(Scroll::By(dy))) => iced::widget::operation::scroll_by(
            transcript_scroll_id(session),
            AbsoluteOffset {
                x: 0.0,
                y: dy as f32,
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

/// Run `jobs` off the update thread, one after another on one blocking task (a replayed history
/// asks for many at once), each in both schemes under [`HIGHLIGHT_BUDGET`], answering with
/// `TranscriptMsg::Highlighted`; past the budget the part stays plain (research R4).
pub fn highlight(jobs: Vec<HighlightJob>) -> Task<Message> {
    if jobs.is_empty() {
        return Task::none();
    }
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || jobs.into_iter().filter_map(run).collect())
                .await
                .unwrap_or_default()
        },
        |done: Vec<TranscriptMsg>| done,
    )
    .then(|done| Task::batch(done.into_iter().map(|msg| Task::done(transcript(msg)))))
}

fn run(job: HighlightJob) -> Option<TranscriptMsg> {
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
}

#[cfg(test)]
mod tests {
    use micold_client::features::transcript_selection::{Click, EdgeDir};
    use micold_core::session::SessionId;

    use super::*;

    fn select(app: &mut App, msg: SelectMsg) -> Task<Message> {
        update(app, TranscriptMsg::Select(msg))
    }

    /// T039, A-3, FR-003: an edge during a drag asks for one scroll task (the `scroll_by` on that
    /// session's transcript); without a drag, or for another session, none.
    #[test]
    fn an_edge_during_a_drag_scrolls_and_otherwise_nothing() {
        let mut app = crate::tests::base_app();
        let (a, b) = (SessionId::new(), SessionId::new());
        let edge = |session| SelectMsg::Edge {
            session,
            dir: EdgeDir::Down,
        };
        assert_eq!(select(&mut app, edge(a)).units(), 0, "no drag");
        let _ = app.core.structured_events(
            a,
            0,
            &[micold_core::structured::SequencedEvent {
                seq: 0,
                event: micold_core::structured::StructuredEvent::UserSent {
                    text: "hello".into(),
                    attachments: vec![],
                },
            }],
            false,
            &micold_client::ui::syntax::GRAMMAR,
        );
        let _ = select(
            &mut app,
            SelectMsg::Press {
                session: a,
                clicks: Click::Single,
            },
        );
        let row = app
            .core
            .transcript_view(a)
            .and_then(|view| {
                micold_client::features::transcript_selection::rows(&view)
                    .into_iter()
                    .next()
            })
            .expect("a row");
        let pos = micold_core::structured::selection::TextPos {
            row,
            path: micold_core::structured::selection::TextPath::new(vec![0]),
            offset: 1,
        };
        let _ = select(
            &mut app,
            SelectMsg::Hit {
                session: a,
                pos,
                click: Click::Single,
                line: None,
            },
        );
        assert!(app.core.transcript_selection.dragging(a));
        assert_eq!(select(&mut app, edge(a)).units(), 1, "scrolls");
        assert_eq!(select(&mut app, edge(b)).units(), 0, "not b's drag");
    }
}
