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

use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::app::NoticeLevel;
use crate::tokens::{spacing, type_scale, Roles};
use crate::ui::style;

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
        let text_block = column![
            text(b.title).size(type_scale::BODY),
            text(b.detail).size(type_scale::LABEL),
        ]
        .spacing(spacing::XS)
        .width(Length::Fill);

        let mut line = row![text_block]
            .spacing(spacing::SM)
            .align_y(Alignment::Center);

        if let Some((label, on_press)) = b.action {
            line = line.push(
                button(text(label).size(type_scale::LABEL))
                    .on_press(on_press)
                    .style(style::outlined(b.roles)),
            );
        }

        container(line)
            .padding(spacing::MD)
            .width(Length::Fill)
            .style(style::notification(b.roles, b.level))
            .into()
    }
}
