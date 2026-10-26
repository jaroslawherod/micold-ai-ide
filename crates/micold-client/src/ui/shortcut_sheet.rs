//! The shortcut sheet (#1274 FR-012, plan D7): the Review tab's keys in a dialog, each in the
//! running platform's form beside what it does, in `REVIEW_KEYS` order; with the single-key switch
//! off, only the chords and the note saying so (FR-022). Built from `ui/material` parts; the
//! showcase poses it with [`sheet`].

use iced::widget::{column, container, row, Column};
use iced::{Alignment, Background, Element, Length};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, shape, spacing, Roles};

use crate::app::{Message, State};
use crate::features::changes::{shortcut_sheet, Msg, SheetModel};
use crate::ui::material::{dialog, style, Button, Surface, SurfaceKind, Text, TypeRole};

/// The sheet's title, as the overflow menu's item names it (US5 s4).
pub const TITLE: &str = "Keyboard shortcuts";

/// The width of the key column: the longest label, *Ctrl+Shift+Enter*, with its keycap padding.
const KEY_WIDTH: f32 = 156.0;

/// The sheet while it is open over the shown Review tab; `None` otherwise.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a micold_core::env_include::EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let model = shortcut_sheet(state)?;
    Some(sheet(
        model,
        Message::Changes(Msg::ShortcutsClosed),
        tokens::roles(scheme),
    ))
}

/// The sheet's surface over `model`; **Close** sends `close`.
pub fn sheet<'a, M: Clone + 'a>(model: SheetModel, close: M, r: Roles) -> Element<'a, M> {
    let rows = model.rows.into_iter().map(|(key, text)| {
        row![
            container(keycap(key, r)).width(Length::Fixed(KEY_WIDTH)),
            Text::new(text, TypeRole::Body, r),
        ]
        .spacing(spacing::MD)
        .align_y(Alignment::Center)
        .into()
    });
    let mut fields = dialog::fields(column![
        Text::new(TITLE, TypeRole::Headline, r),
        Column::with_children(rows).spacing(spacing::SM),
    ]);
    if let Some(note) = model.note {
        fields = fields.push(Text::new(note, TypeRole::Body, r).muted());
    }
    let actions = dialog::actions(row![Button::text("Close", r).on_press(close)]);
    Surface::new(dialog::body(fields, actions), SurfaceKind::Dialog, r)
        .width(Length::Fixed(520.0))
        .into()
}

/// A key's label on a keycap: the label role on the highest container fill.
fn keycap<'a, M: 'a>(label: String, r: Roles) -> Element<'a, M> {
    container(Text::new(label, TypeRole::Label, r))
        .padding([spacing::XS / 2.0, spacing::SM])
        .style(move |_| container::Style {
            background: Some(Background::Color(style::color(r.surface_container_highest))),
            border: iced::border::rounded(shape::EXTRA_SMALL),
            ..container::Style::default()
        })
        .into()
}
