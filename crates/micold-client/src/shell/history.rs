//! Back/Forward's entry point in the shell (feature 1465, contracts/ui.md U2).
//!
//! The root resolves the press (`State::press_history`) and names the messages that show its
//! target (`State::apply_history`); they run through the root `update`, so each takes its full
//! path, and `State::finish_history_move` turns observation back on once they have.

use iced::Task;
use micold_client::app::Message;
use micold_client::features::history::Msg;
use micold_client::features::history::Step;

use crate::App;

/// Resolve and apply a Back/Forward press, then dispatch its follow-ups.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let Msg::Pressed { source, .. } = msg;
    let step = app.core.press_history(msg, &|_| None);
    let moved = matches!(step, Step::Show(_));
    let follow_ups = app.core.apply_history(step);
    let tasks: Vec<_> = follow_ups
        .into_iter()
        .map(|message| crate::update(app, message))
        .collect();
    if moved {
        app.core.finish_history_move(source);
    }
    Task::batch(tasks)
}
