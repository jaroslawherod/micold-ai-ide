//! The composer's large editor dialog (feature 1703, US1): a large multi-line text area over the
//! draft, with **Done** and **Cancel**. Ctrl/Cmd+Enter is **Done**, Escape is **Cancel**
//! (`features::large_editor::editor_key`). `ui::view` wraps the body in the shared modal
//! transition; Escape and a scrim click are the registry's cancellation.

use crate::app::{EditorAction, Message, State};
use crate::features::large_editor::{editor_key, Msg};
use crate::ui::composer::key_input;
use crate::ui::material::{self, Button, Surface, SurfaceKind, Text, TextArea, TypeRole};
use iced::widget::text_editor::{Binding, KeyPress, Status};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, Roles};

/// The dialog's width.
pub const WIDTH: f32 = 760.0;
/// The text area's rows at least, and at most before it scrolls.
pub const ROWS: (usize, usize) = (14, 24);

/// The text area's id, which the shell focuses as the editor opens.
pub fn field() -> iced::widget::Id {
    iced::widget::Id::from("large-editor-area")
}

/// The dialog while an editor is open.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let open = state.large_editor.open.as_ref()?;
    Some(body(open.content.content(), tokens::roles(scheme)))
}

/// The dialog body over `content`; also posed by the showcase.
pub fn body<'a>(content: &'a iced::widget::text_editor::Content, r: Roles) -> Element<'a, Message> {
    let area = TextArea::new(content, r)
        .rows(ROWS.0, ROWS.1)
        .id(field())
        .on_action(|action| Message::LargeEditor(Msg::Edited(EditorAction(action))))
        .key_binding(key_binding);
    let fields = material::dialog::fields(column![
        Text::new("Large editor", TypeRole::Headline, r),
        area
    ]);
    let actions = material::dialog::actions(row![
        Button::text("Cancel", r).on_press(Message::LargeEditor(Msg::Cancelled)),
        Button::filled("Done", r).on_press(Message::LargeEditor(Msg::Done)),
    ]);
    Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(WIDTH))
    .into()
}

/// Ctrl/Cmd+Enter and Escape are the dialog's; every other key is the text area's own.
fn key_binding(press: KeyPress) -> Option<Binding<Message>> {
    if matches!(press.status, Status::Focused { .. }) {
        if let Some(msg) = key_input(
            &press.key,
            &press.physical_key,
            press.modifiers,
            press.text.as_deref(),
        )
        .as_ref()
        .and_then(editor_key)
        {
            return Some(Binding::Custom(Message::LargeEditor(msg)));
        }
    }
    Binding::from_key_press(press)
}
