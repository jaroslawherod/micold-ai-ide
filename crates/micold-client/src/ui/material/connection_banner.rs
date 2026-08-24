//! `ConnectionBanner` — a reusable, prominent banner for the daemon-connection states
//! (Constitution Principle VIII).
//!
//! Distinct from the dismissible notification stack: this is a persistent, full-width strip that
//! says the displayed content may be stale and the service can't be reached — the disconnected and
//! taken-over states of US5 (FR-024/027). It carries a title, a one-line detail, and an optional
//! action (e.g. "Take over"). Theme-aware via [`NoticeLevel`] + the caller's [`Roles`]; exposed as a
//! chainable builder terminating in `.into()` (Principle VIII), so both states share one widget
//! rather than each forking a bespoke banner.

use std::marker::PhantomData;

use crate::ui::material::{Text, TypeRole};
use iced::widget::{column, container, row};
use iced::{Alignment, Element, Length};

use crate::features::notifications::NoticeLevel;
use crate::ui::material::style;
use micold_core::tokens::{spacing, Roles};

/// A persistent connection-status banner. Construct with a title + detail, optionally add an action.
pub struct ConnectionBanner<'a, M> {
    title: String,
    detail: String,
    level: NoticeLevel,
    action: Option<(String, M)>,
    roles: Roles,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: 'a + Clone> ConnectionBanner<'a, M> {
    /// A banner showing `title` over `detail`, styled from `roles`. Defaults to the error level (the
    /// disconnected/taken-over cases are both problems the user should notice).
    pub fn new(title: impl Into<String>, detail: impl Into<String>, roles: Roles) -> Self {
        Self {
            title: title.into(),
            detail: detail.into(),
            level: NoticeLevel::Error,
            action: None,
            roles,
            _marker: PhantomData,
        }
    }

    /// Override the severity level (color).
    pub fn level(mut self, level: NoticeLevel) -> Self {
        self.level = level;
        self
    }

    /// Add a trailing action button (e.g. "Take over" → re-attach with force).
    pub fn action(mut self, label: impl Into<String>, on_press: M) -> Self {
        self.action = Some((label.into(), on_press));
        self
    }
}

impl<'a, M: 'a + Clone> From<ConnectionBanner<'a, M>> for Element<'a, M> {
    fn from(b: ConnectionBanner<'a, M>) -> Self {
        // Title and detail are both prose — `body_medium` over `body_small`, which is how Material
        // sets a banner. The detail was `label_medium`: the same 12dp at the label weight, so a
        // sentence explaining a dropped connection was set in the voice reserved for UI labels.
        let text_block = column![
            Text::new(b.title, TypeRole::Body, b.roles),
            Text::new(b.detail, TypeRole::Caption, b.roles),
        ]
        .spacing(spacing::XS)
        .width(Length::Fill);

        let mut line = row![text_block]
            .spacing(spacing::SM)
            .align_y(Alignment::Center);

        if let Some((label, on_press)) = b.action {
            // The shared `Button`, not a locally styled one. Building the outlined look here would
            // put a second definition of "an outlined button" in the library — one that a change to
            // `Button` never reaches — and it would not ripple, because the ripple is composed
            // inside the component rather than by its callers (FR-021, FR-027).
            //
            // `.on_host(...)` reads the *same* `notification_host` the container below styles
            // itself from, so the action's colours are derived from the decision that produced the
            // fill rather than restated beside it — a level added to `NoticeLevel` cannot give the
            // banner one colour and its button another (FR-004a, FR-027b, BUG-009).
            line = line.push(
                super::Button::outlined(label, b.roles)
                    .on_host(style::notification_host(b.roles, b.level))
                    .on_press(on_press),
            );
        }

        container(line)
            .padding(spacing::MD)
            .width(Length::Fill)
            .style(style::notification(b.roles, b.level))
            .into()
    }
}
