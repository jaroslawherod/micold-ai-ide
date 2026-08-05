//! The small inline components (feature 020, T018): text, a glyph, a rule, a chip, a badge.
//!
//! Each function builds every posed instance for one catalogue entry and hands them to
//! [`arrange`], which lays them out; [`posed`] labels each one, so "every variant is labelled"
//! (US2 acceptance 1) is a property of the gallery rather than of each section remembering.
//!
//! Nothing here styles anything — the components decide their own appearance, and
//! `tests/material_boundary.rs` holds this file to that at the same zero budgets as the
//! application's feature modules.

use iced::Element;
use micold_core::naming::ConventionalType;
use micold_core::protocol::messages::ActivitySignal;
use micold_core::tokens::Roles;

use crate::icons::Icon;
use crate::showcase::catalogue::Layout;
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::samples;
use crate::showcase::state::{Message, Showcase};
use crate::ui::material::{self, TypeRole};

/// How tall a swatch is when a component has no natural height of its own — a vertical rule, a drag
/// handle. A layout dimension, deliberately not derived from the type scale: how big a swatch is has
/// nothing to do with how big text is, and borrowing a text size to answer a layout question is how
/// the two end up coupled.
const SWATCH_HEIGHT: f32 = 48.0;

/// Every type role the scale offers, so a change to one is visible against the others.
///
/// Read from `TypeRole::ALL` rather than restated here: a hand-written list is one more place to
/// forget, and the roles that differ only in *weight* — `Caption` against `Label`, `Body` against
/// `Action` — are exactly the ones a missing gallery entry would hide.
fn roles_in_the_scale() -> Vec<(&'static str, TypeRole)> {
    TypeRole::ALL.iter().map(|r| (r.name(), *r)).collect()
}

/// `Text` — one instance per type role, plus the muted emphasis.
pub fn text<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let mut instances: Vec<Element<'a, Message>> = roles_in_the_scale()
        .into_iter()
        .map(|(label, role)| {
            posed(
                label,
                material::Text::new(samples::LABEL, role, roles),
                roles,
            )
        })
        .collect();
    instances.push(posed(
        "Body, muted",
        material::Text::new(samples::LABEL, TypeRole::Body, roles).muted(),
        roles,
    ));
    arrange(instances, Layout::Inline)
}

/// `Ellipsized` — the component whose whole job is a label that does not fit, so it is posed with one
/// that does and one that does not.
pub fn ellipsized<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "fits",
                material::Ellipsized::<Message>::at_role(
                    samples::LABEL,
                    TypeRole::Body,
                    roles.on_surface,
                ),
                roles,
            ),
            posed(
                "truncated",
                material::Ellipsized::<Message>::at_role(
                    samples::LONG_LABEL,
                    TypeRole::Body,
                    roles.on_surface,
                ),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `Glyph` — an icon at two roles, tinted, and disabled.
pub fn glyph<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "Title",
                material::Glyph::<Message>::new(Icon::Settings, TypeRole::Title, roles),
                roles,
            ),
            posed(
                "Body",
                material::Glyph::<Message>::new(Icon::Settings, TypeRole::Body, roles),
                roles,
            ),
            posed(
                "tinted",
                material::Glyph::<Message>::new(Icon::Git, TypeRole::Title, roles)
                    .tint(roles.primary),
                roles,
            ),
            posed(
                "disabled",
                material::Glyph::<Message>::new(Icon::Delete, TypeRole::Title, roles)
                    .disabled(true),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `Divider` — both orientations. The vertical one needs a height to be visible at all, which the
/// container around it supplies; that is layout, not appearance.
pub fn divider<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "horizontal",
                material::Divider::<Message>::horizontal(roles),
                roles,
            ),
            posed(
                "vertical",
                iced::widget::container(material::Divider::<Message>::vertical(roles))
                    .height(iced::Length::Fixed(SWATCH_HEIGHT)),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// `Tag` — a chip in two accents, so the tint reads as a tint rather than as the component's own
/// colour, and once at the label size.
pub fn tag<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "feat accent",
                material::Tag::<Message>::new(samples::TAG, roles.tag_fill(ConventionalType::Feat)),
                roles,
            ),
            posed(
                "fix accent",
                material::Tag::<Message>::new("fix", roles.tag_fill(ConventionalType::Fix)),
                roles,
            ),
            posed(
                "at the label role",
                material::Tag::<Message>::new(samples::TAG, roles.tag_fill(ConventionalType::Feat))
                    .role(TypeRole::Label),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `ActivityBadge` — one instance per signal, including the ones that deliberately render nothing, so
/// "this signal is invisible on purpose" is on the page rather than inferred from an absence.
pub fn activity_badge<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let signals: Vec<(&str, ActivitySignal)> = vec![
        ("Unknown", ActivitySignal::Unknown),
        ("Working", ActivitySignal::Working),
        ("AwaitingInput", ActivitySignal::AwaitingInput),
        (
            "Ended",
            ActivitySignal::Ended {
                reason: "exited".to_string(),
            },
        ),
    ];
    arrange(
        signals
            .into_iter()
            .map(|(label, signal)| {
                posed(
                    label,
                    material::ActivityBadge::<Message>::new(signal, roles),
                    roles,
                )
            })
            .collect(),
        Layout::Inline,
    )
}
