//! Keyboard navigation's entry point in the shell (feature 803, contracts/ui.md U3).
//!
//! The resolver ([`micold_client::features::navigation::update`]) reads the state and returns a
//! `Step`; `State::apply_navigation` applies it and returns the follow-ups, which run through the
//! root `update` so a `Selected` takes its full path (the Changes view, Compare and the Overview
//! close, the daemon is told).

use iced::Task;
use micold_client::app::Message;
use micold_client::features::navigation;
use micold_client::features::navigation::Msg;
use micold_client::features::navigation::Step;

use crate::App;

/// Resolve and apply a navigation press, then dispatch its follow-ups.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let Msg::Pressed(action) = msg;
    let terminal_had_focus = app.core.terminal_focused();
    let step = navigation::update(&app.core, action);
    apply_step(app, step, terminal_had_focus)
}

/// Apply a resolved step: `State::apply_navigation`, its follow-ups through the root `update`, then
/// `finish_navigation`. Shared by the chords and the quick switcher's choice (feature 968).
pub(crate) fn apply_step(app: &mut App, step: Step, terminal_had_focus: bool) -> Task<Message> {
    let follow_ups = app.core.apply_navigation(step.clone(), terminal_had_focus);
    let tasks: Vec<_> = follow_ups
        .into_iter()
        .map(|message| crate::update(app, message))
        .collect();
    app.core.finish_navigation(&step, terminal_had_focus);
    Task::batch(tasks)
}
