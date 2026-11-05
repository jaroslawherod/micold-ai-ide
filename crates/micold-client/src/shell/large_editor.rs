//! The large editor's entry point in the shell (feature 1703): the reducer
//! ([`micold_client::features::large_editor::update`]) changes the state, the shell moves the
//! keyboard, which belongs to the widget tree: into the editor as it opens, back to the composer
//! that opened it as it closes.

use iced::Task;
use micold_client::app::Message;
use micold_client::features::large_editor::Msg;
use micold_client::ui::composer::composer_field;
use micold_client::ui::large_editor::field;

use crate::App;

/// Apply a large editor message, then the focus change it caused, if any.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let before = app.core.large_editor.open.as_ref().map(|open| open.session);
    app.core.update(Message::LargeEditor(msg));
    let after = app.core.large_editor.open.as_ref().map(|open| open.session);
    match (before, after) {
        (None, Some(_)) => iced::widget::operation::focus(field()),
        (Some(session), None) => iced::widget::operation::focus(composer_field(session)),
        _ => Task::none(),
    }
}
