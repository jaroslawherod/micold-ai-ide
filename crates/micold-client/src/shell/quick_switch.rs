//! The quick switcher's entry point in the shell (feature 968, contracts/ui.md U3).
//!
//! The reducer ([`micold_client::features::quick_switch::update`]) resolves a choice to a
//! `Choice`; the shell only executes it: close the Overview first when asked, then apply the step
//! as a navigation press does.

use iced::Task;
use micold_client::app::Message;
use micold_client::features::overview;
use micold_client::features::quick_switch::Msg;

use crate::App;

/// Apply a switcher message, then the choice it resolved to, if any.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let Some(choice) = micold_client::features::quick_switch::update(&mut app.core, msg) else {
        return Task::none();
    };
    let close = choice
        .close_overview_first
        .then(|| crate::update(app, Message::Overview(overview::Msg::Closed)));
    let apply = super::navigation::apply_step(app, choice.step, choice.terminal_had_focus);
    Task::batch(close.into_iter().chain([apply]))
}
