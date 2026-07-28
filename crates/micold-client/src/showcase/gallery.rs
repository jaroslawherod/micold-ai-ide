//! The page: the catalogue, traversed (feature 020, T011).
//!
//! There is no decision logic here. Every branch is either an iteration over the catalogue or an
//! `Option` unwrap; nothing tests a [`Showcase`] field to decide what to build. That is not a style
//! preference — it is the precondition of Principle I's GUI-wiring exception, which is why
//! `tests/showcase_glue.rs` asserts it rather than trusting this comment.
//!
//! # Layout
//!
//! One vertical scroll over a column of sections. Within a section, posed instances are chunked into
//! rows at a fixed count, and an entry may claim a full-width row of its own. The chunk is fixed
//! rather than measured because measuring would make the page depend on the window, and FR-022 says
//! the page is the same on every launch. When the window narrows the page reflows by scrolling
//! vertically; it never scrolls horizontally, because an instance clipped out of view reads as a
//! missing one (spec, Edge Cases).
//!
//! # Composition only
//!
//! Every visible thing here comes from `ui::material` — the headings, the captions, the controls, the
//! scroll. `tests/material_boundary.rs` scans this directory at the same zero budgets it holds the
//! application's feature modules to, so a hand-styled heading is a build failure. Where the gallery
//! needs something the library lacks, FR-021's answer is to add it to the library.

use iced::widget::{column, container, row};
use iced::{Element, Length};
use micold_core::tokens::{self, spacing, Roles};

use super::catalogue::{Entry, Layout, MotionEntry, Section, COMPONENTS, EXEMPTIONS, MOTION};
use super::state::{Message, Showcase};
use crate::ui::cdk;
use crate::ui::material;

/// How many inline instances share a row before the next one wraps to a new row.
///
/// Fixed rather than measured: a count derived from the window would make the page's layout depend on
/// the window's size, and two launches at different sizes would show different rows (FR-022).
const PER_ROW: usize = 3;

/// The showcase's window title.
pub const TITLE: &str = "Micold — Component Showcase";

/// The whole page.
pub fn view(showcase: &Showcase) -> Element<'_, Message> {
    let roles = tokens::roles(showcase.scheme);

    let page = column![
        header(showcase, roles),
        entries_of(Section::Components, showcase, roles),
        heading("Motion", roles),
        motion(showcase, roles),
        exemptions(roles),
    ]
    .spacing(spacing::LG)
    .padding(spacing::LG)
    .width(Length::Fill);

    let base: Element<'_, Message> = material::Surface::new(
        material::Scrollable::new(page, roles).height(Length::Fill),
        material::SurfaceKind::Window,
        roles,
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into();

    // Every floating surface the gallery can open goes onto the same overlay host the application
    // uses (FR-021): it already positions, stacks and dismisses correctly, and it is already tested.
    // `showcase.open` holds one surface, so the page can never be trapped behind two.
    let mut overlay = cdk::overlay::Overlay::new(base);
    for surface in super::sections::floating::surfaces(showcase, roles) {
        overlay = overlay.push(surface);
    }
    overlay.into()
}

/// The title, the scheme control, and what the page is for.
fn header<'a>(showcase: &'a Showcase, roles: Roles) -> Element<'a, Message> {
    // What the control says it will do is a decision about state, so the reducer owns it. See
    // `tests/showcase_glue.rs`, which is the reason it is not decided here.
    let scheme_label = showcase.scheme_control_label();
    column![
        material::Text::new(TITLE, material::TypeRole::Headline, roles),
        material::Text::new(
            "Every component the shared library provides, in every state that can be posed. Hover, \
             press and focus are live — point at an instance to see them.",
            material::TypeRole::Body,
            roles,
        ),
        material::Button::with_content(
            material::Text::new(scheme_label, material::TypeRole::Label, roles),
            material::ButtonVariant::Outlined,
            roles,
        )
        .on_press(Message::SchemeToggled),
    ]
    .spacing(spacing::SM)
    .into()
}

/// Every entry belonging to `section`, in catalogue order.
fn entries_of<'a>(section: Section, showcase: &'a Showcase, roles: Roles) -> Element<'a, Message> {
    let mut stack = column![].spacing(spacing::LG).width(Length::Fill);
    for (index, entry) in COMPONENTS.iter().enumerate() {
        if entry.section == section {
            stack = stack.push(entry_view(entry, index, showcase, roles));
        }
    }
    stack.into()
}

/// One entry: its heading, the caption naming what is live, and its instances.
fn entry_view<'a>(
    entry: &'a Entry,
    index: usize,
    showcase: &'a Showcase,
    roles: Roles,
) -> Element<'a, Message> {
    let instances = (entry.render)(showcase, roles, index);
    column![
        material::Text::new(entry.component, material::TypeRole::Title, roles),
        material::Text::new(caption(entry), material::TypeRole::Label, roles),
        instances,
    ]
    .spacing(spacing::XS)
    .width(Length::Fill)
    .into()
}

/// What the entry says about itself: which states are posed here, and which have to be produced with
/// a pointer or a keyboard (FR-005).
///
/// A pure function of the entry — no state, no branching on the showcase.
fn caption(entry: &Entry) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !entry.variants.is_empty() {
        parts.push(format!("variants: {}", entry.variants.join(", ")));
    }
    if !entry.posed.is_empty() {
        parts.push(format!("posed: {}", entry.posed.join(", ")));
    }
    if !entry.density.is_empty() {
        parts.push(format!("density: {}", entry.density.join(", ")));
    }
    if !entry.live.is_empty() {
        parts.push(format!("live (exercise it): {}", entry.live.join(", ")));
    }
    if parts.is_empty() {
        format!("{} — nothing to pose or exercise", entry.module)
    } else {
        format!("{} · {}", entry.module, parts.join(" · "))
    }
}

/// The motion section: every animation the library provides, each replayable on demand.
fn motion<'a>(showcase: &'a Showcase, roles: Roles) -> Element<'a, Message> {
    let mut stack = column![].spacing(spacing::LG).width(Length::Fill);
    for (offset, entry) in MOTION.iter().enumerate() {
        stack = stack.push(motion_view(entry, motion_index(offset), showcase, roles));
    }
    // The components whose appearance *is* an animation are catalogue entries in the motion section,
    // so they render here too rather than being posed as stills among the static components.
    stack = stack.push(entries_of(Section::Motion, showcase, roles));
    stack.into()
}

/// Motion entries index into the same per-entry state as components, past the components' range, so
/// no two entries share a replay counter.
fn motion_index(offset: usize) -> usize {
    COMPONENTS.len() + offset
}

/// One animation: its name and its replay control.
fn motion_view<'a>(
    entry: &'a MotionEntry,
    index: usize,
    showcase: &'a Showcase,
    roles: Roles,
) -> Element<'a, Message> {
    column![
        material::Text::new(entry.label, material::TypeRole::Title, roles),
        material::Text::new(
            format!("{} · replayable on demand", entry.animation),
            material::TypeRole::Label,
            roles,
        ),
        (entry.render)(showcase, roles, index),
    ]
    .spacing(spacing::XS)
    .width(Length::Fill)
    .into()
}

/// The recorded exemptions, on the page rather than only in the source.
///
/// A developer checking the gallery for a component and not finding it should learn *here* that its
/// absence was a decision with a reason, not an oversight — otherwise the exemption list protects the
/// build without informing the person the build was protecting.
fn exemptions<'a>(roles: Roles) -> Element<'a, Message> {
    let mut stack = column![].spacing(spacing::SM).width(Length::Fill);
    for entry in EXEMPTIONS {
        stack = stack.push(
            column![
                material::Text::new(
                    format!("{}::{}", entry.module, entry.component),
                    material::TypeRole::Label,
                    roles,
                ),
                material::Text::new(entry.reason, material::TypeRole::Body, roles),
            ]
            .spacing(spacing::XS),
        );
    }
    column![
        heading("Not shown, and why", roles),
        material::Text::new(
            "These are components with no appearance of their own. Each is exercised by the page \
             without being posed on it.",
            material::TypeRole::Body,
            roles,
        ),
        stack,
    ]
    .spacing(spacing::SM)
    .into()
}

/// A section heading, with a divider under it.
fn heading<'a>(text: &'a str, roles: Roles) -> Element<'a, Message> {
    column![
        material::Text::new(text, material::TypeRole::Headline, roles),
        material::Divider::horizontal(roles),
    ]
    .spacing(spacing::XS)
    .into()
}

/// Lay `instances` out as rows of [`PER_ROW`], or one full-width row.
///
/// Shared by the section render functions so every section wraps the same way. Takes the already-built
/// instances, so it decides nothing about what they are.
pub fn arrange<'a>(instances: Vec<Element<'a, Message>>, layout: Layout) -> Element<'a, Message> {
    let per_row = match layout {
        Layout::FullWidth => 1,
        Layout::Inline => PER_ROW,
    };
    let mut stack = column![].spacing(spacing::MD).width(Length::Fill);
    let mut line = row![].spacing(spacing::MD).align_y(iced::Alignment::Center);
    let mut filled = 0usize;
    // Consumed rather than borrowed: an `Element` is not `Clone`, so a row is assembled by moving
    // instances into it.
    for instance in instances {
        line = line.push(instance);
        filled += 1;
        if filled == per_row {
            stack = stack.push(line);
            line = row![].spacing(spacing::MD).align_y(iced::Alignment::Center);
            filled = 0;
        }
    }
    if filled > 0 {
        stack = stack.push(line);
    }
    stack.into()
}

/// A labelled instance: the component, with the state it is posed in named under it.
///
/// Every section uses this, so "each variant is labelled" (FR-003, US2 acceptance 1) is a property of
/// the gallery rather than of each section remembering.
pub fn posed<'a>(
    label: &'a str,
    instance: impl Into<Element<'a, Message>>,
    roles: Roles,
) -> Element<'a, Message> {
    container(
        column![
            instance.into(),
            material::Text::new(label, material::TypeRole::Label, roles),
        ]
        .spacing(spacing::XS),
    )
    .padding(spacing::XS)
    .into()
}
