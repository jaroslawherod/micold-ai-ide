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

        let mut line = row![
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

        if let Some(message) = s.on_dismiss {
            // A text button in `inverse_primary`: the only accent that stays legible on the
            // inverted container, and the reason that role exists at all.
            //
            // Said once, to the button, instead of tinted onto the label. Tinting reached the
            // glyphs and nothing else — the hover and press layers and the ripple stayed `primary`
            // over the inverted fill, because the component decides those and the call site cannot
            // reach them. `.on_host` hands the whole variant the role, so all four move together
            // (FR-004a, FR-027b, BUG-009 T155).
            line = line.push(
                Button::with_content(
                    Text::new("Dismiss", TypeRole::Action, r),
                    ButtonVariant::Text,
                    r,
                )
                .on_host(style::snackbar_host(r))
                .on_press(message),
            );
        }

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
