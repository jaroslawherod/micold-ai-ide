//! `OverviewCard`: one location of the project Overview (feature 813, FR-005, FR-006, FR-021).
//!
//! The type icon and the display name (ellipsized, the full name in a tooltip), the state icon and
//! word (the reason is the word's tooltip), the branch when it has one, the summary line and the
//! sessions footer. Every colour comes from the theme's roles through [`palette`]: Needs action is
//! the `error` family, Needs attention `warning`, Working `primary`, Done `success`, and Idle sits
//! on the de-emphasised `surface_container_low`. The card is informational until M4 adds a press.

use crate::icons::Icon;
use crate::ui::material::{style, Ellipsized, Glyph, Text, Tooltip, TypeRole};
use iced::widget::{column, container, row};
use iced::{Alignment, Element, Length};
use micold_core::overview::{CardModel, CardState, LocationType};
use micold_core::tokens::{spacing, Rgb, Roles};
use std::marker::PhantomData;

/// The card's one height: the same for every state and every content, so a grid row never ragged.
pub const HEIGHT: f32 = 136.0;

/// The colours one card state is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The card's fill.
    pub fill: Rgb,
    /// The words on the fill.
    pub text: Rgb,
    /// The icons on the fill.
    pub icon: Rgb,
}

/// The fill, text and icon roles of `state` (research R11).
pub fn palette(state: CardState, r: &Roles) -> Palette {
    let (fill, on) = match state {
        CardState::NeedsAction => (r.error_container, r.on_error_container),
        CardState::NeedsAttention => (r.warning_container, r.on_warning_container),
        CardState::Working => (r.primary_container, r.on_primary_container),
        CardState::Done => (r.success_container, r.on_success_container),
        CardState::Idle => (r.surface_container_low, r.on_surface_variant),
    };
    Palette {
        fill,
        text: on,
        icon: on,
    }
}

/// The icon of a card state.
pub fn state_icon(state: CardState) -> Icon {
    match state {
        CardState::NeedsAction => Icon::CardNeedsAction,
        CardState::NeedsAttention => Icon::CardNeedsAttention,
        CardState::Working => Icon::StateRunning,
        CardState::Done => Icon::CardDone,
        CardState::Idle => Icon::StateIdle,
    }
}

/// The icon of a location type.
pub fn type_icon(kind: LocationType) -> Icon {
    match kind {
        LocationType::Feature => Icon::TypeFeature,
        LocationType::Bugfix => Icon::TypeBugfix,
        LocationType::Chore => Icon::TypeChore,
        LocationType::Other => Icon::TypeOther,
        LocationType::Included => Icon::TypeIncluded,
        LocationType::OrphanInvalid => Icon::TypeOrphan,
        LocationType::ProjectRoot => Icon::ProjectRoot,
    }
}

/// What the card says in words, apart from the layout that places it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Words {
    /// The type icon.
    pub type_icon: Icon,
    /// The type's word: the footer's first words and the type icon's tooltip.
    pub type_label: &'static str,
    /// The name's tooltip: the whole display name, however much of it shows.
    pub name_tooltip: String,
    /// The state icon.
    pub state_icon: Icon,
    /// The state's word.
    pub state_label: &'static str,
    /// The reason, the state word's tooltip.
    pub reason_tooltip: &'static str,
    /// The footer line.
    pub footer: String,
}

/// The words of `model`.
pub fn words(model: &CardModel) -> Words {
    Words {
        type_icon: type_icon(model.kind),
        type_label: model.kind.label(),
        name_tooltip: model.display_name.clone(),
        state_icon: state_icon(model.state),
        state_label: model.state.label(),
        reason_tooltip: model.reason.label(),
        footer: format!(
            "{} · {} sessions, {} running",
            model.kind.label(),
            model.sessions.0,
            model.sessions.1
        ),
    }
}

/// The card. See the module documentation.
#[derive(Clone)]
pub struct OverviewCard<'a, M> {
    model: CardModel,
    roles: Roles,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> OverviewCard<'a, M> {
    /// The card of `model`, themed by `roles`.
    pub fn new(model: &CardModel, roles: Roles) -> Self {
        Self {
            model: model.clone(),
            roles,
            _marker: PhantomData,
        }
    }
}

impl<'a, M: 'a> From<OverviewCard<'a, M>> for Element<'a, M> {
    fn from(card: OverviewCard<'a, M>) -> Self {
        let roles = card.roles;
        let p = palette(card.model.state, &roles);
        let w = words(&card.model);
        let glyph =
            |icon: Icon| Element::from(Glyph::<M>::new(icon, TypeRole::Title, roles).tint(p.icon));
        let header = row![
            Tooltip::new(glyph(w.type_icon), w.type_label, roles),
            Tooltip::new(
                Ellipsized::<M>::at_role(card.model.display_name.clone(), TypeRole::Title, p.text),
                w.name_tooltip,
                roles
            ),
            Tooltip::new(
                row![
                    glyph(w.state_icon),
                    Text::new(w.state_label, TypeRole::Label, roles).tint(p.text)
                ]
                .spacing(spacing::XS)
                .align_y(Alignment::Center),
                w.reason_tooltip,
                roles
            ),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
        let mut body = column![header].spacing(spacing::XS);
        if let Some(branch) = &card.model.branch {
            body = body.push(Ellipsized::<M>::at_role(
                branch.clone(),
                TypeRole::Caption,
                p.text,
            ));
        }
        body = body
            .push(Ellipsized::<M>::at_role(
                card.model.summary.clone(),
                TypeRole::Body,
                p.text,
            ))
            .push(Text::new(w.footer, TypeRole::Caption, roles).tint(p.text));
        container(body)
            .width(Length::Fill)
            .height(Length::Fixed(HEIGHT))
            .padding(spacing::MD)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(style::color(p.fill))),
                border: iced::Border {
                    radius: 12.0.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{leaves, overlap};
    use super::*;
    use iced::{Rectangle, Size};
    use micold_core::overview::CardReason;
    use micold_core::session::SessionLocation;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens;

    fn model(state: CardState, name: &str) -> CardModel {
        CardModel {
            location: SessionLocation::Worktree("w".into()),
            display_name: name.into(),
            branch: Some("feat/813-overview".into()),
            kind: LocationType::Feature,
            sessions: (3, 2),
            state,
            reason: CardReason::Working,
            summary: "Adds the worktree overview".into(),
        }
    }

    fn card(model: &CardModel) -> Element<'static, ()> {
        OverviewCard::<()>::new(model, tokens::roles(ColorScheme::Light)).into()
    }

    /// U4: the card is one height whatever its state.
    #[test]
    fn the_card_has_one_height_in_all_five_states() {
        let heights: Vec<f32> = CardState::ALL
            .into_iter()
            .map(|s| {
                super::super::test_support::size_of(
                    card(&model(s, "feat-a")),
                    Size::new(400.0, 400.0),
                )
                .height
            })
            .collect();
        assert!(heights.iter().all(|h| *h == HEIGHT), "{heights:?}");
    }

    /// U4: at 280 px no two pieces of the card overlap and none leaves it, even with a long name.
    #[test]
    fn no_text_overlaps_at_280_and_a_long_name_ellipsizes() {
        let long = "a-display-name-that-is-far-too-long-to-fit-in-the-card ".repeat(4);
        for state in CardState::ALL {
            for name in ["feat-a", long.as_str()] {
                let node = super::super::test_support::laid_out(
                    card(&model(state, name)),
                    Size::new(280.0, 400.0),
                );
                let parts = leaves(&node);
                let whole = Rectangle::new(iced::Point::ORIGIN, node.size());
                for (i, a) in parts.iter().enumerate() {
                    assert!(
                        a.x >= -0.01
                            && a.x + a.width <= whole.width + 0.01
                            && a.y + a.height <= whole.height + 0.01,
                        "{state:?}: {a:?} leaves the card {whole:?}"
                    );
                    for b in &parts[i + 1..] {
                        assert!(!overlap(a, b), "{state:?}: {a:?} overlaps {b:?}");
                    }
                }
                assert_eq!(node.size().width, 280.0);
            }
        }
    }

    /// U4: the full name is the tooltip of the ellipsized name.
    #[test]
    fn the_full_display_name_is_in_the_tooltip() {
        let long = "x".repeat(200);
        assert_eq!(words(&model(CardState::Idle, &long)).name_tooltip, long);
    }

    /// SC-005: each card carries its type icon with the type's word, and its state icon with the
    /// state's word and the reason; the five states and the types are told apart by icon.
    #[test]
    fn a_card_shows_its_type_and_state_with_their_words() {
        let mut state_icons = Vec::new();
        for state in CardState::ALL {
            let w = words(&model(state, "n"));
            assert_eq!(w.state_label, state.label());
            assert_eq!(w.reason_tooltip, CardReason::Working.label());
            assert_eq!(w.type_label, LocationType::Feature.label());
            assert_eq!(w.type_icon, Icon::TypeFeature);
            state_icons.push(w.state_icon);
        }
        state_icons.dedup();
        assert_eq!(state_icons.len(), 5, "{state_icons:?}");
        let types = [
            LocationType::Feature,
            LocationType::Bugfix,
            LocationType::Chore,
            LocationType::Other,
            LocationType::Included,
            LocationType::OrphanInvalid,
            LocationType::ProjectRoot,
        ];
        let icons: Vec<Icon> = types.iter().map(|t| type_icon(*t)).collect();
        for (i, a) in icons.iter().enumerate() {
            assert!(!icons[i + 1..].contains(a), "{a:?} stands for two types");
        }
        assert_eq!(
            words(&model(CardState::Idle, "n")).footer,
            "Feature · 3 sessions, 2 running",
            "FR-002: the type's word is visible text on the card, not only a tooltip"
        );
    }
}
