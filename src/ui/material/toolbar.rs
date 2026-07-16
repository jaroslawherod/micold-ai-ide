//! `Toolbar` — a reusable Material toolbar primitive (Constitution Principle VIII).
//!
//! A flat `surface` bar with **no border** (Angular-Material style): a title on the leading
//! edge and a set of action elements pushed to the trailing edge. Reused by the app shell;
//! any future top bar should reuse it rather than fork a bespoke bar.

use crate::ui::style;
use iced::widget::{container, row, text, Space};
use iced::{Alignment, Element, Length};
use micold_ai_ide::tokens::{spacing, type_scale, Roles};

/// Render a toolbar with `title` on the left and `actions` (already-built elements) on the
/// right. Generic over the message type so any feature can reuse it (Principle VIII).
pub fn toolbar<'a, M: 'a>(
    title: impl Into<String>,
    actions: Vec<Element<'a, M>>,
    r: Roles,
) -> Element<'a, M> {
    let mut bar = row![
        text(title.into()).size(type_scale::TITLE),
        Space::with_width(Length::Fill),
    ]
    .spacing(spacing::MD)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    for action in actions {
        bar = bar.push(action);
    }

    container(bar)
        .width(Length::Fill)
        .padding(spacing::SM)
        .style(style::toolbar_surface(r))
        .into()
}
