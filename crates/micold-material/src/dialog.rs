//! §7.4's dialog anatomy, owned in one place instead of nine (FR-028).
//!
//! A dialog in this application is not a component — it is a [`Surface`](super::Surface) of
//! [`Kind::Dialog`](super::SurfaceKind::Dialog) that nine call sites each fill with a column of
//! content and a row of actions. That is a reasonable shape, and it left §7.4's four spatial
//! figures with nowhere to live: every call site spelled them as
//! [`spacing`](micold_core::tokens::spacing) steps, and `anatomy::dialog::PADDING`,
//! `TITLE_TO_BODY`, `BODY_TO_ACTIONS` and `ACTION_GAP` were referenced by nothing.
//!
//! Three of the four steps happened to equal the figure. The fourth did not: the action row was
//! pushed into the body's own column and took its 16dp where §7.4 states 24, so the separation the
//! contract asks for — "the actions read as a separate region rather than as more body" — was not
//! there.
//!
//! So the figures move here, and a call site names a *part* rather than a number. This is the same
//! argument `Surface` already makes for §7.4's width bounds: applied by the kind rather than by
//! each dialog, because "the seven that build dialogs were each free to forget it".

use super::{Button, Surface, SurfaceKind};
use iced::widget::{column, row, Column, Row};
use iced::{Element, Length};
use micold_core::tokens::{anatomy, Roles};

/// The dialog's content column — title, body, and whatever fields sit between them — spaced by
/// §7.4's title-to-body gap.
///
/// Takes the column rather than returning an empty one so a call site keeps `column![..]` and its
/// conditional `push`es, and changes by one word. The figure is named here and nowhere else.
pub fn fields<'a, M: 'a>(fields: Column<'a, M>) -> Column<'a, M> {
    fields.spacing(anatomy::dialog::TITLE_TO_BODY)
}

/// The dialog's action row, spaced by §7.4's gap between adjacent actions.
pub fn actions<'a, M: 'a>(actions: Row<'a, M>) -> Row<'a, M> {
    actions.spacing(anatomy::dialog::ACTION_GAP)
}

/// The dialog's content: [`fields`] above [`actions`], separated by §7.4's body-to-actions gap.
///
/// A column of its own rather than pushing the row into the fields, which is what every call site
/// did and is why the gap was the title's 16dp. The two gaps differ on purpose and cannot both come
/// from one column's spacing.
pub fn body<'a, M: 'a>(
    fields: impl Into<Element<'a, M>>,
    actions: impl Into<Element<'a, M>>,
) -> Column<'a, M> {
    column![fields.into(), actions.into()].spacing(anatomy::dialog::BODY_TO_ACTIONS)
}

/// A confirm dialog: `fields` above a filled `confirm` button and an outlined `cancel` one, each a
/// label and the message it sends, on a [`SurfaceKind::Dialog`] surface `width` dp wide.
///
/// The tail every `confirm_*` dialog repeated. Takes the fields column un-spaced, so a call site
/// keeps `column![..]` and its conditional `push`es.
pub fn confirm<'a, M: Clone + 'a>(
    r: Roles,
    fields: Column<'a, M>,
    confirm: (&str, M),
    cancel: (&str, M),
    width: f32,
) -> Element<'a, M> {
    let actions = actions(row![
        Button::filled(confirm.0, r).on_press(confirm.1),
        Button::outlined(cancel.0, r).on_press(cancel.1),
    ]);
    Surface::new(body(self::fields(fields), actions), SurfaceKind::Dialog, r)
        .width(Length::Fixed(width))
        .into()
}
