//! `IconButton` — a reusable icon-only button primitive (Constitution Principle VIII).
//!
//! Theme-aware (tinted via a design-system color role) with a disabled state. Reused across
//! the sidebar (add worktree, close session) and any future icon actions rather than each
//! feature building its own.

use crate::ui::{icon, style};
use iced::widget::button;
use iced::Element;
use micold_ai_ide::icons::Icon;
use micold_ai_ide::tokens::{spacing, Rgb, Roles};

/// A compact icon-only button. `on_press` of `None` renders it disabled (greyed, inert).
///
/// Generic over the message type so any feature can reuse it (Principle VIII). Styled as a
/// low-emphasis text button so it sits unobtrusively in dense rows like the sidebar tree.
pub fn icon_button<'a, M: Clone + 'a>(
    glyph: Icon,
    size: u16,
    tint: Rgb,
    roles: Roles,
    on_press: Option<M>,
) -> Element<'a, M> {
    let content = icon(glyph, size, tint);
    let mut btn = button(content)
        .padding(spacing::XS)
        .style(style::text_button(roles));
    if let Some(message) = on_press {
        btn = btn.on_press(message);
    }
    btn.into()
}
