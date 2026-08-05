//! The motion section (feature 020, T023–T025 — FR-007a, FR-007b, FR-007c, FR-023a).
//!
//! An animation that can only be seen by catching it once is not reviewable, so every entry here has a
//! **Replay** control: pressing it bumps a generation counter, the wrapper sees a changed
//! `restart_on(key)`, and it plays from the start — as many times as asked. **Reverse** flips the
//! destination so the exit is watchable too, which matters because Material exits are quicker than
//! entrances and an entry that could only be entered would hide half the specification.
//!
//! There is no clock anywhere in this file. The wrappers own their own progress and ask the runtime for
//! frames only while they are moving (feature 017's `Progress`), which is what keeps the page inert at
//! rest (FR-023, SC-009).
//!
//! # The run control has no users yet
//!
//! FR-023a asks that a component whose appearance *runs continuously* be stopped at rest and started by
//! the developer. Nothing in the library does: `StageProgress`'s fill is a fixed value precisely because
//! it makes no claim about completion. So [`run_control`] exists, is wired to `Showcase::running`, and is
//! used by zero entries at delivery. Feature 018's indeterminate indicator is the first, and it plugs in
//! without this section changing shape.

use std::time::Duration;

use iced::{Element, Length};
use micold_core::tokens::{spacing, Roles};

use crate::showcase::catalogue::{Layout, MotionEntry};
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::samples;
use crate::showcase::state::{Message, Showcase};
use crate::ui::material::{self, SurfaceKind, TypeRole};

/// How long every demonstration here takes. Long enough to watch deliberately — a 90ms transition is
/// correct in the application and useless in a gallery.
const OVER: Duration = Duration::from_millis(600);

/// The content each wrapper animates: enough to see move, and the same in every entry so the
/// difference between two entries is the transition rather than the payload.
fn subject<'a>(roles: Roles) -> Element<'a, Message> {
    material::Surface::new(
        material::Text::new(samples::LABEL, TypeRole::Title, roles),
        SurfaceKind::Plain,
        roles,
    )
    .padding(spacing::MD)
    .width(Length::Fixed(240.0))
    .into()
}

/// The **Replay** and **Reverse** pair for entry `index`.
fn controls<'a>(showcase: &'a Showcase, roles: Roles, index: usize) -> Element<'a, Message> {
    let reverse_label = if showcase.shown(index) {
        "Reverse (play it out)"
    } else {
        "Reverse (play it in)"
    };
    iced::widget::row![
        material::Button::with_content(
            material::Text::new("Replay", TypeRole::Label, roles),
            material::ButtonVariant::Filled,
            roles,
        )
        .on_press(Message::Replayed(index)),
        material::Button::with_content(
            material::Text::new(reverse_label, TypeRole::Label, roles),
            material::ButtonVariant::Outlined,
            roles,
        )
        .on_press(Message::Reversed(index)),
    ]
    .spacing(spacing::SM)
    .into()
}

/// The **Run / Stop** control for a component whose appearance runs continuously (FR-023a).
///
/// Unused at delivery — nothing in the library runs continuously yet. Kept, and kept public, because
/// the mechanism is the point: at rest the component is stopped and asks for no frames, and the
/// developer's press is what stands in for the operation such an indication normally reports on. So it
/// is never displayed running with nothing running, and FR-023 needs no exemption for it.
pub fn run_control<'a>(showcase: &'a Showcase, roles: Roles, index: usize) -> Element<'a, Message> {
    let label = if showcase.running(index) {
        "Stop"
    } else {
        "Run"
    };
    material::Button::with_content(
        material::Text::new(label, TypeRole::Label, roles),
        material::ButtonVariant::Outlined,
        roles,
    )
    .on_press(Message::RunToggled(index))
    .into()
}

/// One demonstration: the animated subject above its controls.
fn demo<'a>(
    animated: Element<'a, Message>,
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    iced::widget::column![animated, controls(showcase, roles, index)]
        .spacing(spacing::SM)
        .into()
}

// ---------------------------------------------------------------------------------------------
// The animation helpers (FR-013a's second category)
// ---------------------------------------------------------------------------------------------

/// `fade` — compositing the surrounding surface over the content as it goes.
pub fn fade<'a>(showcase: &'a Showcase, roles: Roles, index: usize) -> Element<'a, Message> {
    // The subject is a rounded card, so the compositing veil is rounded to match. Left square it
    // paints past the corners — the same class of bug the ripple had, and the reason `fade` takes a
    // radius at all.
    let animated = material::fade(subject(roles), showcase.shown(index), OVER, roles.surface)
        .rounded(SurfaceKind::Plain.shape())
        .restart_on(showcase.replays(index))
        .into();
    demo(animated, showcase, roles, index)
}

/// `expand` — a top-anchored vertical reveal, as the sidebar's filter accordion uses.
pub fn expand<'a>(showcase: &'a Showcase, roles: Roles, index: usize) -> Element<'a, Message> {
    let animated = material::expand(subject(roles), showcase.shown(index), OVER)
        .restart_on(showcase.replays(index))
        .into();
    demo(animated, showcase, roles, index)
}

/// `scale` — the subtle dialog lift, about the content's centre.
pub fn scale<'a>(showcase: &'a Showcase, roles: Roles, index: usize) -> Element<'a, Message> {
    let animated = material::scale(subject(roles), showcase.shown(index), OVER)
        .restart_on(showcase.replays(index))
        .into();
    demo(animated, showcase, roles, index)
}

/// `scrim` — the dimming layer behind a modal surface.
///
/// A leaf with no child: what it dims is not inside it. It fills whatever it is given, so here it is
/// given a fixed box — otherwise it would dim the gallery.
pub fn scrim<'a>(showcase: &'a Showcase, roles: Roles, index: usize) -> Element<'a, Message> {
    let dimming: Element<'a, Message> =
        material::scrim(iced::Color::BLACK, showcase.shown(index), OVER)
            .restart_on(showcase.replays(index))
            .into();
    let animated = iced::widget::container(dimming)
        .width(Length::Fixed(240.0))
        .height(Length::Fixed(80.0))
        .into();
    demo(animated, showcase, roles, index)
}

/// Every animation the library provides, each replayable on demand (FR-007a).
pub const MOTION: &[MotionEntry] = &[
    MotionEntry {
        animation: "fade",
        label: "Fade — content dissolving behind its own surface colour",
        render: fade,
    },
    MotionEntry {
        animation: "expand",
        label: "Expand — a top-anchored vertical reveal",
        render: expand,
    },
    MotionEntry {
        animation: "scale",
        label: "Scale — the dialog lift, about the centre",
        render: scale,
    },
    MotionEntry {
        animation: "scrim",
        label: "Scrim — the dimming layer behind a modal surface",
        render: scrim,
    },
];

// ---------------------------------------------------------------------------------------------
// The components whose appearance *is* an animation
// ---------------------------------------------------------------------------------------------

/// `Fade` — the wrapper `fade` builds. Posed here rather than in the component grid, because a still
/// of a transition is a picture of it (FR-007a).
pub fn fade_component<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    arrange(
        vec![posed("replay it", fade(showcase, roles, index), roles)],
        Layout::FullWidth,
    )
}

/// `Expand` — the wrapper `expand` builds.
pub fn expand_component<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    arrange(
        vec![posed("replay it", expand(showcase, roles, index), roles)],
        Layout::FullWidth,
    )
}

/// `Scale` — the wrapper `scale` builds.
pub fn scale_component<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    arrange(
        vec![posed("replay it", scale(showcase, roles, index), roles)],
        Layout::FullWidth,
    )
}

/// `Scrim` — the leaf `scrim` builds.
pub fn scrim_component<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    arrange(
        vec![posed("replay it", scrim(showcase, roles, index), roles)],
        Layout::FullWidth,
    )
}

/// `ViewFade` — the main content area's entrance, which replays whenever what it shows changes. The
/// replay control changes its identity, which is exactly what a session switch does in the application.
pub fn view_fade<'a>(showcase: &'a Showcase, roles: Roles, index: usize) -> Element<'a, Message> {
    let animated: Element<'a, Message> = material::ViewFade::new(subject(roles), roles.background)
        .showing(showcase.replays(index))
        .into();
    arrange(
        vec![posed(
            "replay it",
            demo(animated, showcase, roles, index),
            roles,
        )],
        Layout::FullWidth,
    )
}

/// `HoverReveal` — row actions that fade in while the row is pointed at.
///
/// Its trigger is the pointer, not a control, so this entry poses both destinations and says so: the
/// reveal itself is exercised by hovering a row in the `TreeView` section.
pub fn hover_reveal<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    let revealed: Element<'a, Message> = material::HoverReveal::new(subject(roles), roles.surface)
        .shown(showcase.shown(index))
        .into();
    arrange(
        vec![posed(
            "toggle it with Reverse",
            demo(revealed, showcase, roles, index),
            roles,
        )],
        Layout::FullWidth,
    )
}
