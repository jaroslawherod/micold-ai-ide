//! `UsageIndicator` — the Claude plan's usage in the app bar (Constitution Principle VIII; feature
//! 490, contract `usage-indicator.md`).
//!
//! The `data_usage` glyph at 18 px, then the label at `label_large`, both `on_surface_variant`:
//! 40 px high like the bar's other actions, 8 px between glyph and label, 12 px ends, as wide as
//! the label. Not pressable; hovering shows the details in the shared [`Tooltip`].
//!
//! Takes strings rather than a reading, so `ui::material` does not depend on the feature module
//! (the `PullRequestIndicator` rule).

use crate::icons::Icon;
use crate::ui::material::glyph::icon;
use crate::ui::material::{Text, Tooltip, TypeRole};
use iced::widget::{container, row};
use iced::{Alignment, Element, Length, Padding};
use micold_core::tokens::Roles;
use std::marker::PhantomData;

/// The glyph's side (§7.3's leading icon).
const GLYPH: f32 = 18.0;
/// Between the glyph and the label.
const GAP: f32 = 8.0;
/// Before the glyph and after the label.
const ENDS: f32 = 12.0;
/// The app bar's action height.
const HEIGHT: f32 = 40.0;

/// The indicator. See the module documentation.
pub struct UsageIndicator<'a, M> {
    label: String,
    roles: Roles,
    details: Vec<String>,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> UsageIndicator<'a, M> {
    /// The indicator showing `label`, themed by `roles`.
    pub fn new(label: impl Into<String>, roles: &Roles) -> Self {
        Self {
            label: label.into(),
            roles: *roles,
            details: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// The tooltip's lines (FR-006). Empty, the default, shows no tooltip.
    pub fn details(mut self, lines: Vec<String>) -> Self {
        self.details = lines;
        self
    }
}

impl<'a, M: 'a> From<UsageIndicator<'a, M>> for Element<'a, M> {
    fn from(i: UsageIndicator<'a, M>) -> Self {
        let r = i.roles;
        let tint = r.on_surface_variant;
        let body = container(
            row![
                icon(Icon::PlanUsage, GLYPH, tint),
                Text::new(i.label, TypeRole::Action, r).tint(tint),
            ]
            .spacing(GAP)
            .align_y(Alignment::Center),
        )
        .padding(Padding::from([0.0, ENDS]))
        .height(Length::Fixed(HEIGHT))
        .align_y(Alignment::Center);
        if i.details.is_empty() {
            body.into()
        } else {
            Tooltip::new(body, i.details.join("\n"), r).into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::layout;
    use iced::advanced::widget::Tree;
    use iced::Size;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens;

    fn size_of(mut element: Element<'_, ()>) -> Size {
        let renderer = super::super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::new(800.0, 100.0));
        element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits)
            .bounds()
            .size()
    }

    /// U2.2: 40 px high whatever the label, as wide as the label, and the tooltip changes neither.
    #[test]
    fn the_indicator_is_40_high_and_follows_its_label() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            let short = size_of(UsageIndicator::new("42% · 15:30", &r).into());
            let long = size_of(UsageIndicator::new("100% · Mon 15:30", &r).into());
            let detailed = size_of(
                UsageIndicator::new("42% · 15:30", &r)
                    .details(vec!["5-hour: 42%, resets 15:30".into()])
                    .into(),
            );
            assert_eq!(short.height, HEIGHT);
            assert_eq!(long.height, HEIGHT);
            assert!(long.width > short.width, "{long:?} vs {short:?}");
            assert!(
                short.width > 2.0 * ENDS + GLYPH + GAP,
                "the label takes width: {short:?}"
            );
            assert_eq!(detailed, short, "the tooltip adds no size");
        }
    }

    /// U2: `data_usage` is its own glyph, and `on_surface_variant` is legible on the bar.
    #[test]
    fn the_glyph_is_data_usage_and_legible_on_the_bar() {
        assert_eq!(Icon::PlanUsage.glyph(), '\u{e1af}');
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            assert!(
                tokens::contrast(r.on_surface_variant, r.surface) >= tokens::AA_NON_TEXT,
                "{scheme:?}"
            );
        }
    }
}
