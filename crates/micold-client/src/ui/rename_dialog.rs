//! The rename dialog shared by the project and worktree renames: the two differ only in their
//! texts, field, messages and draft type, which [`RenameSpec`] carries.

use crate::app::Message;
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{self, Button, RequiredLegend, SurfaceKind, Text, TextField, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::required::FieldRequirement;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// What one rename dialog shows and sends.
pub(crate) struct RenameSpec<'a> {
    pub title: &'static str,
    pub caption: String,
    pub label: &'static str,
    pub field: FieldId,
    pub text: &'a str,
    /// How the name field is marked (feature 937): required, and missing once left or confirmed.
    pub required: FieldRequirement,
    pub on_input: fn(String) -> Message,
    pub on_confirm: Message,
    pub on_cancel: Message,
}

/// The rename dialog body; `ui::view` wraps it in the shared modal transition.
pub(crate) fn view<'a>(
    spec: RenameSpec<'a>,
    scheme: ColorScheme,
    focused: Option<FieldId>,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let input = TextField::new("", spec.text, r)
        .label(spec.label)
        .required(spec.required)
        .track_focus(spec.field, focused)
        .on_input(spec.on_input)
        .on_submit(spec.on_confirm.clone());

    let mut fields = material::dialog::fields(column![
        Text::new(spec.title, TypeRole::Headline, r),
        Text::new(spec.caption, TypeRole::Caption, r).muted(),
    ]);
    // The marker's explanation, once, above the first field (feature 937, FR-004).
    if spec.required != FieldRequirement::Optional {
        fields = fields.push(RequiredLegend::new(r));
    }
    fields = fields.push(input);

    let actions = material::dialog::actions(row![
        Button::filled("Rename", r).on_press(spec.on_confirm),
        Button::outlined("Cancel", r).on_press(spec.on_cancel),
    ]);

    material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(420.0))
    .into()
}
