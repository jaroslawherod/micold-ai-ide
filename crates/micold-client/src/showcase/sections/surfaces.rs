//! Surfaces, containers and the list components (feature 020, T020).
//!
//! The larger components: the surface every panel sits on, a scroll viewport, an accordion, an app
//! bar, a banner, a progress indicator, a tree, and the navigation drawer. Several are naturally
//! full-width, which is why the catalogue gives them their own rows — a banner beside a chip would
//! push the chip off screen (spec, Edge Cases).

use iced::{Element, Length};
use micold_core::naming::ConventionalType;
use micold_core::tokens::{spacing, Roles};

use crate::features::notifications::NoticeLevel;
use crate::icons::Icon;
use crate::showcase::catalogue::Layout;
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::samples;
use crate::showcase::state::{Message, Showcase};
use crate::ui::material::{self, SurfaceKind, TypeRole};

/// Every surface kind, including the two that carry a payload — one representative payload each,
/// which is what "a variant has an instance" means for a variant that takes one.
fn kinds(roles: Roles) -> Vec<(&'static str, SurfaceKind)> {
    vec![
        ("Window", SurfaceKind::Window),
        ("Plain", SurfaceKind::Plain),
        ("Dialog", SurfaceKind::Dialog),
        ("Sidebar", SurfaceKind::Sidebar),
        ("Toolbar", SurfaceKind::Toolbar),
        ("Menu", SurfaceKind::Menu),
        ("ListItem", SurfaceKind::ListItem),
        (
            "Notification(Info)",
            SurfaceKind::Notification(NoticeLevel::Info),
        ),
        (
            "Notification(Error)",
            SurfaceKind::Notification(NoticeLevel::Error),
        ),
        (
            "Chip",
            SurfaceKind::Chip(roles.tag_fill(ConventionalType::Feat)),
        ),
    ]
}

/// `Surface` — one instance per kind, each holding the same label so the difference is the surface.
pub fn surface<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        kinds(roles)
            .into_iter()
            .map(|(label, kind)| {
                posed(
                    label,
                    material::Surface::new(
                        material::Text::new(samples::LABEL, TypeRole::Body, roles),
                        kind,
                        roles,
                    )
                    .padding(spacing::MD),
                    roles,
                )
            })
            .collect(),
        Layout::Inline,
    )
}

/// `Scrollable` — content taller than its viewport, so the themed scrollbar is actually visible.
pub fn scrollable<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let mut long = iced::widget::column![].spacing(spacing::XS);
    for (name, _) in samples::WORKTREES.iter().cycle().take(12) {
        long = long.push(material::Text::new(*name, TypeRole::Body, roles));
    }
    arrange(
        vec![posed(
            "scroll it",
            iced::widget::container(material::Scrollable::new(long, roles))
                .height(Length::Fixed(120.0)),
            roles,
        )],
        Layout::Inline,
    )
}

/// `Accordion` — closed, open, and paired with the trigger that drives it.
///
/// The third instance is the important one. `Accordion` is only the **panel half**: it has no header,
/// no twisty and nothing to press, because the thing that opens it is a separate component
/// (`FilterTrigger`) that the call site pairs it with — which is what the sidebar's tag filter does.
/// Posed alone it reads as floating text rather than as an accordion, which is a fair complaint about
/// the component's name and an unfair thing for the gallery to hide.
pub fn accordion<'a>(showcase: &'a Showcase, roles: Roles, index: usize) -> Element<'a, Message> {
    let body = |roles: Roles| material::Text::new(samples::BODY, TypeRole::Body, roles);
    let live_open = showcase.shown(index);
    arrange(
        vec![
            posed(
                "closed (zero height by design)",
                material::Accordion::new(body(roles), roles).open(false),
                roles,
            ),
            posed(
                "open",
                material::Accordion::new(body(roles), roles).open(true),
                roles,
            ),
            posed(
                "with its trigger — press it. `Accordion` is the panel only; the trigger is `FilterTrigger`, paired by the call site (this is what the sidebar's tag filter does). Unoutlined by design: in the sidebar it sits inline and the sidebar's own edge separates it.",
                iced::widget::column![
                    material::FilterTrigger::new(Message::Reversed(index), roles).active(live_open),
                    material::Accordion::new(body(roles), roles).open(live_open),
                ]
                .spacing(spacing::XS),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// `Toolbar` — a title alone, and a title with actions.
pub fn toolbar<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "title only",
                material::Toolbar::<Message>::new("Micold AI IDE", roles),
                roles,
            ),
            posed(
                "with actions",
                material::Toolbar::new("Micold AI IDE", roles)
                    .action(
                        material::IconButton::new(Icon::Settings, roles).on_press(Message::NoOp),
                    )
                    .action(material::IconButton::new(Icon::Menu, roles).on_press(Message::NoOp)),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// `ConnectionBanner` — both notice levels, and one carrying an action.
pub fn connection_banner<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "Info",
                material::ConnectionBanner::<Message>::new(
                    "Not connected to the session service",
                    "The displayed content may be stale. Reconnecting…",
                    roles,
                )
                .level(NoticeLevel::Info),
                roles,
            ),
            posed(
                "Error",
                material::ConnectionBanner::<Message>::new(
                    "The session service is a different version",
                    "Restart the service to match — your sessions are preserved.",
                    roles,
                )
                .level(NoticeLevel::Error),
                roles,
            ),
            posed(
                "with an action",
                material::ConnectionBanner::new(
                    "Another window took over this project",
                    "This window is read-only until you take it back.",
                    roles,
                )
                .action("Take over", Message::NoOp),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// `StageProgress` — the bar plus the step in flight.
///
/// **Indeterminate** since feature 018 (T050, FR-031f): the segment travels the track on `long_2`
/// and claims nothing about how far along the work is. It used to sit at a fixed 40% fill, which
/// asserts a completion fraction nothing can back up — the application does not learn whether the
/// submodule stage runs at all until the branch and worktree already exist, so a user watching that
/// bar had every reason to read "40% done" while it meant nothing.
///
/// # This page does not idle, and that is the accepted trade
///
/// An indeterminate bar animates for as long as it is mounted, and both poses below are mounted for
/// as long as the Components section is on screen — so this page asks for a frame every frame,
/// where the rest of the gallery comes to rest (T085).
///
/// It has **no run control**, and should not have one. The motion section gives each entry a replay
/// button because those animations are *transient*: they play once, and you press replay to see the
/// thing again. This one is continuous by definition. There is nothing to replay, and a paused
/// indeterminate bar is a static bar — which is the exact misreading T050 removed, reintroduced in
/// the one place whose job is to show what the component is.
///
/// So the cost is accepted rather than designed around. It is bounded to a development-only binary
/// (`packaging_excludes_showcase.rs` keeps the gallery out of the shipped package), and the
/// application's own quiescence is unaffected and separately gated —
/// `tests/indeterminate_stops_with_its_operation.rs` holds the real field: there the indicator is
/// mounted only while a create is in flight, so it stops within a frame of the operation ending
/// (FR-039d, SC-017).
pub fn stage_progress<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "in progress",
                material::StageProgress::new(samples::STAGE, roles),
                roles,
            ),
            // The second pose is what a stage looks like once it has been running long enough to
            // report where it has got to (BUG-009, T123) — the state a submodule fetch spends
            // minutes in, and the one that used to show nothing but the label above.
            posed(
                "with a live line",
                material::StageProgress::new(samples::STAGE, roles)
                    .detail(Some(samples::STAGE_DETAIL.to_string())),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// `TreeView` — the sidebar's list, with a selected row, tags, a badge and an expandable parent.
pub fn tree_view<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let items: Vec<material::TreeItem<'a, Message>> = samples::WORKTREES
        .iter()
        .enumerate()
        .map(|(row, (name, depth))| {
            let mut item = material::TreeItem::new(*depth, *name, roles.on_surface)
                .with_icon(Icon::Git)
                .on_press(Message::NoOp);
            if row == 1 {
                item = item.selected(true).tags(vec![(
                    samples::TAG.to_string(),
                    roles.tag_fill(ConventionalType::Feat),
                )]);
            }
            if row == 0 {
                item = item.expandable(true, Message::NoOp);
            }
            item
        })
        .collect();
    arrange(
        vec![posed(
            "a worktree tree",
            material::TreeView::new(items, roles),
            roles,
        )],
        Layout::FullWidth,
    )
}

/// `NavigationDrawer` — open (showing its panel) and closed (showing its rail).
pub fn navigation_drawer<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let panel = |roles: Roles| {
        material::Surface::new(
            material::Text::new("The panel", TypeRole::Body, roles),
            SurfaceKind::Sidebar,
            roles,
        )
        .padding(spacing::MD)
        .height(Length::Fixed(96.0))
    };
    let rail = |roles: Roles| {
        material::Surface::new(
            material::Glyph::<Message>::new(Icon::ShowSidebar, TypeRole::Title, roles),
            SurfaceKind::Sidebar,
            roles,
        )
        .padding(spacing::SM)
        .height(Length::Fixed(96.0))
    };
    arrange(
        vec![
            posed(
                "open",
                material::NavigationDrawer::new(panel(roles), rail(roles)).open(true),
                roles,
            ),
            posed(
                "closed (the rail)",
                material::NavigationDrawer::new(panel(roles), rail(roles)).open(false),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}
