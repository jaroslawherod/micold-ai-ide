//! `Snackbar` — the floating, elevated surface that replaces the inline notification strip
//! (feature 018, T052 — FR-032; contract §7.8, component-api §2.2).
//!
//! # The component owns presentation; the core owns the queue
//!
//! Which notification is visible, what order the rest follow in, when each expires, how dedup and
//! the cap interact — none of that has pixels in it, so all of it lives in
//! [`micold_core::notify`], tested with no renderer at all. This renders whatever is currently
//! visible and reports dismissal. It holds no timer, no queue and no memory of what it showed
//! last; hand it a different notification and it draws that one.
//!
//! That split is why the behaviour change FR-032a asks for — one at a time, queued, timed by
//! severity — could be specified and tested before a single pixel of this file existed.
//!
//! # Inverse roles, and why they are the point
//!
//! A snackbar is `inverse_surface` with `inverse_on_surface` text: light-on-dark in a light scheme
//! and dark-on-light in a dark one. It is the one surface in the application deliberately inverted,
//! which is what makes it read as an interruption rather than as another panel — and it is why the
//! action label takes `inverse_primary` rather than `primary`, a role that exists only to stay
//! legible against this container.

use iced::widget::{container, row, Space};
use iced::{Element, Length};
use micold_core::notify::Notification;
use micold_core::tokens::{anatomy, Roles};

use super::style;
use super::{Button, ButtonVariant, Text, TypeRole};
use crate::ui::cdk::reflow::Reflow;

/// The visible notification, drawn as Material's snackbar. Builder form (Principle VIII):
/// `Snackbar::new(notification, roles).on_dismiss(msg).into()`.
pub struct Snackbar<'a, M> {
    notification: &'a Notification,
    roles: Roles,
    on_dismiss: Option<M>,
}

impl<'a, M: Clone + 'a> Snackbar<'a, M> {
    /// Draw `notification`, themed by `roles`.
    pub fn new(notification: &'a Notification, roles: Roles) -> Self {
        Self {
            notification,
            roles,
            on_dismiss: None,
        }
    }

    /// The message emitted when the user dismisses it.
    ///
    /// Manual dismissal is *always* available (FR-032b) — the timeout is a convenience, not the
    /// only way out — so a call site that omits this is showing something the user cannot clear.
    pub fn on_dismiss(mut self, message: M) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}

impl<'a, M: Clone + 'a> From<Snackbar<'a, M>> for Element<'a, M> {
    fn from(s: Snackbar<'a, M>) -> Self {
        let r = s.roles;

        let message = row![
            // §7.8's 48dp floor. Width is left `Shrink` rather than set to 0, because iced drops
            // any child whose size hint `is_void()` — true the moment *either* dimension is
            // `Fixed(0)` — so a zero-width spacer is deleted outright and the floor it was
            // enforcing silently stops existing. A one-line `body_medium` message plus 14dp of
            // padding comes to 48dp exactly, so this changes nothing today; it is here because a
            // shorter role or tighter padding would drop below it and nothing would look wrong.
            Space::new().height(anatomy::snackbar::MIN_HEIGHT),
            Text::new(s.notification.message.clone(), TypeRole::Body, r)
                .tint(r.inverse_on_surface)
                .width(Length::Fill),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(anatomy::snackbar::PADDING_H);

        let line: Element<'a, M> = match s.on_dismiss {
            // A text button in `inverse_primary`: the only accent that stays legible on the
            // inverted container, and the reason that role exists at all.
            //
            // Said once, to the button, instead of tinted onto the label. Tinting reached the
            // glyphs and nothing else — the hover and press layers and the ripple stayed `primary`
            // over the inverted fill, because the component decides those and the call site cannot
            // reach them. `.on_host` hands the whole variant the role, so all four move together
            // (FR-004a, FR-027b, BUG-009 T155).
            //
            // Beside the message through `Reflow`, not as a third child of its row. The container
            // is sized by its content, which lays a row out child by child in order, each against
            // what the ones before it left: a message long enough to wrap took the whole line and
            // the action got the remainder, a sliver its label was drawn out of, past the
            // snackbar's edge (BUG-015). `Reflow` measures the action first, at its natural width,
            // and gives the message what is left.
            Some(on_dismiss) => Reflow::new(
                message,
                Button::with_content(
                    Text::new("Dismiss", TypeRole::Action, r),
                    ButtonVariant::Text,
                    r,
                )
                .on_host(style::snackbar_host(r))
                .on_press(on_dismiss),
            )
            .spacing(anatomy::snackbar::PADDING_H)
            .into(),
            None => message.into(),
        };

        container(line)
            .width(Length::Shrink)
            .max_width(anatomy::snackbar::MAX_WIDTH)
            .padding(iced::Padding {
                top: anatomy::snackbar::PADDING_V,
                bottom: anatomy::snackbar::PADDING_V,
                left: anatomy::snackbar::PADDING_H,
                right: anatomy::snackbar::PADDING_H,
            })
            .style(style::snackbar(r))
            .into()
    }
}

/// The action keeps its width and its place inside the container, whatever the message (BUG-015).
#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::layout;
    use iced::advanced::widget::Tree;
    use iced::{Point, Rectangle, Size};
    use micold_core::notify::Level;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens;

    /// Layout arithmetic accumulates over a nested tree; this is far below the defect, which
    /// squeezed a ~70dp button to under 2dp.
    const TOLERANCE: f32 = 0.5;

    /// The message BUG-015 was found on: a launch notification carrying a full folder path, long
    /// enough to reach §7.8's maximum width and wrap.
    const LONG: &str = "Couldn't reopen \"repo-gone\": its folder /home/someone/workspaces/\
                        clients/acme/monorepo/.claude/worktrees/feat-long-branch-name/nested/\
                        repo-gone is unavailable.";

    fn roles() -> Roles {
        tokens::roles(ColorScheme::Light)
    }

    fn layout_of(mut element: Element<'_, ()>, width: f32) -> layout::Node {
        let renderer = super::super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::new(width, 900.0));
        element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits)
    }

    /// The width the `Dismiss` button asks for when nothing competes with it.
    fn natural_action_width() -> f32 {
        let button: Element<'_, ()> = Button::with_content(
            Text::new("Dismiss", TypeRole::Action, roles()),
            ButtonVariant::Text,
            roles(),
        )
        .on_host(style::snackbar_host(roles()))
        .on_press(())
        .into();
        layout_of(button, f32::INFINITY).bounds().width
    }

    /// Lay a dismissible snackbar out at `window` width and return the container's box and the
    /// action's, the action in the container's coordinates.
    ///
    /// The action is the **last child** of the container's content — the trailing element of the
    /// line, whatever arranges it — so this reads the button without depending on how many
    /// siblings precede it.
    fn snackbar_at(message: &str, window: f32) -> (Rectangle, Rectangle) {
        let notification = Notification::new(Level::Error, message);
        let node = layout_of(
            Snackbar::new(&notification, roles()).on_dismiss(()).into(),
            window,
        );
        let content = &node.children()[0];
        let action = content
            .children()
            .last()
            .expect("a dismissible snackbar lays out an action");
        let offset = content.bounds().position() - Point::ORIGIN;
        (node.bounds(), action.bounds() + offset)
    }

    #[test]
    fn a_long_message_leaves_the_action_its_width_inside_the_container() {
        let natural = natural_action_width();
        for window in [1200.0, 400.0] {
            let (panel, action) = snackbar_at(LONG, window);
            assert!(
                action.width >= natural - TOLERANCE,
                "at a {window}dp window the Dismiss button is {}dp wide but its label needs \
                 {natural}dp — the message took the line first and left the action the remainder, \
                 so the label is drawn past its own box and past the snackbar's edge",
                action.width,
            );
            let inner_right = panel.width - anatomy::snackbar::PADDING_H;
            assert!(
                action.x + action.width <= inner_right + TOLERANCE,
                "at a {window}dp window the Dismiss button ends at {}dp, past the container's \
                 padded edge at {inner_right}dp",
                action.x + action.width,
            );
        }
    }

    #[test]
    fn a_short_message_still_sizes_the_snackbar_to_its_content() {
        let (panel, action) = snackbar_at("Created", 1200.0);
        assert!(
            panel.width < anatomy::snackbar::MAX_WIDTH - TOLERANCE,
            "a one-word snackbar is {}dp wide: it should be sized by what it holds, not stretched \
             to §7.8's {}dp cap",
            panel.width,
            anatomy::snackbar::MAX_WIDTH,
        );
        assert!(
            action.x + action.width <= panel.width - anatomy::snackbar::PADDING_H + TOLERANCE,
            "the action ends at {}dp, past the padded edge of a {}dp container",
            action.x + action.width,
            panel.width,
        );
    }
}
