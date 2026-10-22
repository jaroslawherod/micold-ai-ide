//! `PromptCard` — a bordered block that asks the user something: a title, a body, and a row of
//! actions (feature 942, T059, research R11).
//!
//! All three of a structured session's permission views are one: a tool request, a plan approval
//! and Claude's questions. Each draws its own body (the input, the plan, the questions) and its
//! own buttons; the card gives them one shape — outlined, apart from the raised tool card above it
//! — and one action row that wraps onto further lines rather than running past the card's edge in
//! a narrow pane. A muted note and an error line go under the actions.
//!
//! Builder form: `PromptCard::new("Bash", roles).leading(Icon::NeedsPermission).body(input)
//!     .action(allow).action(deny).note(rule).into()`.

use iced::widget::text::Wrapping;
use iced::widget::{column, row, Column};
use iced::{Alignment, Element, Length};
use micold_core::tokens::{spacing, Roles};

use super::{style, Glyph, Text, TypeRole};
use crate::icons::Icon;

/// A card that asks.
pub struct PromptCard<'a, M> {
    title: String,
    roles: Roles,
    leading: Option<Icon>,
    body: Vec<Element<'a, M>>,
    actions: Vec<Element<'a, M>>,
    note: Option<String>,
    error: Option<String>,
}

impl<'a, M: 'a> PromptCard<'a, M> {
    /// A card headed `title`, themed by `roles`, with no body and no actions yet.
    pub fn new(title: impl Into<String>, roles: Roles) -> Self {
        Self {
            title: title.into(),
            roles,
            leading: None,
            body: Vec::new(),
            actions: Vec::new(),
            note: None,
            error: None,
        }
    }

    /// A glyph before the title.
    pub fn leading(mut self, icon: Icon) -> Self {
        self.leading = Some(icon);
        self
    }

    /// One more part of the body, under the title and the parts before it.
    pub fn body(mut self, part: impl Into<Element<'a, M>>) -> Self {
        self.body.push(part.into());
        self
    }

    /// One more action at the end of the action row.
    pub fn action(mut self, action: impl Into<Element<'a, M>>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// A muted line under the actions: what one of them does beyond its label.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// A line in the error role under the actions: why the last answer was refused.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

impl<'a, M: 'a> From<PromptCard<'a, M>> for Element<'a, M> {
    fn from(c: PromptCard<'a, M>) -> Self {
        let r = c.roles;
        let mut header = row![].spacing(spacing::SM).align_y(Alignment::Center);
        if let Some(icon) = c.leading {
            header = header.push(Glyph::new(icon, TypeRole::Section, r).tint(r.on_surface_variant));
        }
        header = header.push(
            Text::new(c.title, TypeRole::Section, r)
                .wrapping(Wrapping::WordOrGlyph)
                .width(Length::Fill),
        );
        let mut parts: Column<'a, M> = column![header].spacing(spacing::SM).width(Length::Fill);
        for part in c.body {
            parts = parts.push(part);
        }
        if !c.actions.is_empty() {
            parts = parts.push(
                iced::widget::Row::with_children(c.actions)
                    .spacing(spacing::SM)
                    .align_y(Alignment::Center)
                    .wrap()
                    .vertical_spacing(spacing::SM),
            );
        }
        let line = |text: String| {
            Text::new(text, TypeRole::Caption, r)
                .wrapping(Wrapping::WordOrGlyph)
                .width(Length::Fill)
        };
        if let Some(note) = c.note {
            parts = parts.push(line(note).muted());
        }
        if let Some(error) = c.error {
            parts = parts.push(line(error).tint(r.error));
        }
        iced::widget::container(parts)
            .padding(spacing::MD)
            .width(Length::Fill)
            .style(style::prompt_card(r))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, roles, Mounted};
    use super::super::Button;
    use super::*;

    fn card(r: Roles) -> PromptCard<'static, String> {
        PromptCard::new("Bash", r)
            .leading(Icon::NeedsPermission)
            .body(Text::new("echo hi", TypeRole::Body, r))
            .action(Button::filled("Allow", r).on_press("allow".to_string()))
            .action(Button::outlined("Deny", r).on_press("deny".to_string()))
            .note("Always allow adds a rule")
            .error("Refused")
    }

    /// The title, the body, every action, the note and the error are drawn, in both schemes.
    #[test]
    fn every_part_is_drawn() {
        for r in [roles(), dark_roles()] {
            let painted = Mounted::new(card(r)).painted_text();
            for text in [
                "Bash",
                "echo hi",
                "Allow",
                "Deny",
                "Always allow adds a rule",
                "Refused",
            ] {
                assert!(
                    painted.iter().any(|p| p.content == text),
                    "`{text}`: {painted:?}"
                );
            }
        }
    }

    /// On the card's plate, in both schemes, its words read at 4.5:1 — the title and body, the
    /// muted note and the error line — and a chosen mark at 3:1.
    #[test]
    fn its_words_are_legible_on_its_plate() {
        use micold_core::theme::ColorScheme;
        use micold_core::tokens::{contrast, roles as roles_of, AA_NON_TEXT, AA_TEXT};
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = roles_of(scheme);
            let plate = r.surface_container_low;
            for (what, ink, floor) in [
                ("text", r.on_surface, AA_TEXT),
                ("note", r.on_surface_variant, AA_TEXT),
                ("error", r.error, AA_TEXT),
                ("chosen mark", r.primary, AA_NON_TEXT),
                ("unchosen mark", r.on_surface_variant, AA_NON_TEXT),
            ] {
                let ratio = contrast(ink, plate);
                assert!(ratio >= floor, "{scheme:?} {what}: {ratio:.2}:1");
            }
        }
    }

    /// The card's edge is its outline, not a shadow (contract §1.5).
    #[test]
    fn the_card_is_outlined_and_casts_no_shadow() {
        let s = style::prompt_card(roles())(&style::theme(micold_core::theme::ColorScheme::Light));
        assert!(s.border.width > 0.0);
        assert_eq!(s.shadow, iced::Shadow::default());
    }
}
