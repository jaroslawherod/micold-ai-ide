//! §7.4's spatial figures, read off a laid-out dialog (FR-028).
//!
//! Every figure in §7.4 except the width bounds is a *gap*: how far the surface pads its content,
//! how far the body sits below the title, how far the action row sits below the body, and how far
//! two actions sit apart. `anatomy_size` measures boxes and cannot see any of them, and the four
//! constants were referenced by nothing — nine call sites each spelled the number as a
//! [`spacing`](micold_core::tokens::spacing) step instead.
//!
//! Three of the four spacing steps happen to equal the figure they stood in for, which is the
//! dangerous half: `spacing::LG` is 24 and so is `dialog::PADDING`, so nothing looked wrong and
//! nothing would have, until §7.4 was re-valued and the dialogs did not move. That is
//! `type_scale::BODY` again, and `anatomy_call_sites.rs` is the check that names it.
//!
//! The fourth was simply wrong. The action row was pushed into the body's own column, so it took
//! that column's 16dp where §7.4 states 24 — and §7.4 states 24 *for a reason it gives*: the gap is
//! wider than the title's "so the actions read as a separate region rather than as more body".
//!
//! In-crate for the same reason as its neighbours: `material` is `pub(crate)`.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::Tree;
use iced::widget::{column, row};
use iced::{Element, Rectangle, Size};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{anatomy, Roles};

use super::{dialog, Button, Surface, SurfaceKind, Text, TypeRole};
use crate::showcase::state::Message;

/// Room enough that nothing here is under pressure from the limit, and narrower than §7.4's 560dp
/// cap so the surface is not also being clamped.
const ROOM: Size = Size::new(520.0, 600.0);

/// Layout arithmetic accumulates over a nested tree; far below anything a person could see, and far
/// below the one gap this module separates (16dp against 24).
const TOLERANCE: f32 = 0.5;

fn roles() -> Roles {
    micold_core::tokens::roles(ColorScheme::Light)
}

/// The absolute bounds of the node at `path` once `element` is laid out in [`ROOM`].
fn bounds_at(element: Element<'_, Message>, path: &[usize]) -> Rectangle {
    let mut element = element;
    let renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, ROOM),
    );

    let mut layout = Layout::new(&node);
    for (depth, &index) in path.iter().enumerate() {
        layout = layout.children().nth(index).unwrap_or_else(|| {
            panic!(
                "no child {index} at depth {depth} of {path:?} — the dialog's tree changed shape, \
                 so this test is measuring something other than what it names"
            )
        });
    }
    layout.bounds()
}

/// A dialog with a title, a body and two actions — the shape every confirm dialog in the
/// application builds, assembled the way a call site assembles it.
fn confirm() -> Element<'static, Message> {
    let r = roles();
    let fields = dialog::fields(column![
        Text::new("Delete this worktree?", TypeRole::Headline, r),
        Text::new("This cannot be undone.", TypeRole::Body, r),
    ]);
    let actions = dialog::actions(row![
        Button::filled("Delete", r).on_press(Message::NoOp),
        Button::outlined("Cancel", r).on_press(Message::NoOp),
    ]);
    Surface::new(dialog::body(fields, actions), SurfaceKind::Dialog, r).into()
}

/// §7.4: a dialog pads 24 on all sides.
///
/// Measured on the leading edge, which is the one a call site could get wrong independently — the
/// surface takes a single `Padding`, so one axis proves the others.
#[test]
fn a_dialog_pads_its_content_by_the_24dp_it_states() {
    let surface = bounds_at(confirm(), &[]);
    let content = bounds_at(confirm(), &[0]);

    let inset = content.x - surface.x;
    assert!(
        (inset - anatomy::dialog::PADDING).abs() < TOLERANCE,
        "a dialog inset its content by {inset}dp, but §7.4 states {}dp",
        anatomy::dialog::PADDING,
    );
}

/// §7.4: 16 between the title and the body.
#[test]
fn a_dialogs_body_sits_16dp_below_its_title() {
    let title = bounds_at(confirm(), &[0, 0, 0]);
    let body = bounds_at(confirm(), &[0, 0, 1]);

    let gap = body.y - (title.y + title.height);
    assert!(
        (gap - anatomy::dialog::TITLE_TO_BODY).abs() < TOLERANCE,
        "a dialog's body sat {gap}dp below its title, but §7.4 states {}dp",
        anatomy::dialog::TITLE_TO_BODY,
    );
}

/// §7.4: 24 between the body and the action row — wider than the title's gap, on purpose.
///
/// This is the figure that was not merely unbound but wrong. The action row was pushed into the
/// body's own column, so it inherited that column's 16dp, and the separation §7.4 asks for did not
/// exist: the actions read as one more line of body.
#[test]
fn a_dialogs_actions_sit_24dp_below_its_body() {
    let body = bounds_at(confirm(), &[0, 0]);
    let actions = bounds_at(confirm(), &[0, 1]);

    let gap = actions.y - (body.y + body.height);
    assert!(
        (gap - anatomy::dialog::BODY_TO_ACTIONS).abs() < TOLERANCE,
        "a dialog's action row sat {gap}dp below its body, but §7.4 states {}dp — and states it \
         wider than the title's {}dp so the actions read as a separate region rather than as more \
         body",
        anatomy::dialog::BODY_TO_ACTIONS,
        anatomy::dialog::TITLE_TO_BODY,
    );
}

/// §7.4: 8 between adjacent actions.
#[test]
fn a_dialogs_actions_sit_8dp_apart() {
    let first = bounds_at(confirm(), &[0, 1, 0]);
    let second = bounds_at(confirm(), &[0, 1, 1]);

    let gap = second.x - (first.x + first.width);
    assert!(
        (gap - anatomy::dialog::ACTION_GAP).abs() < TOLERANCE,
        "a dialog's actions sat {gap}dp apart, but §7.4 states {}dp",
        anatomy::dialog::ACTION_GAP,
    );
}
