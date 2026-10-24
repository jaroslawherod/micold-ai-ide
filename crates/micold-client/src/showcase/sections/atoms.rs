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
use micold_core::tokens::{Rgb, Roles};

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

/// `RequiredLegend` — the `* required` line a form shows once above its first field (feature 937,
/// FR-011); the marker itself is posed on the `FormField` entry.
pub fn required_legend<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![posed(
            "legend",
            material::RequiredLegend::<Message>::new(roles),
            roles,
        )],
        Layout::Inline,
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
            // Feature 1126: the session area's header chips, led by a glyph and cut under a
            // ceiling.
            posed(
                "led by a glyph",
                material::Tag::<Message>::new("Working", roles.primary_container)
                    .solid(roles.on_primary_container)
                    .role(TypeRole::Label)
                    .leading(
                        material::Glyph::<Message>::new(Icon::StateRunning, TypeRole::Label, roles)
                            .tint(roles.on_primary_container),
                    ),
                roles,
            ),
            posed(
                "cut under a ceiling",
                material::Tag::<Message>::new(
                    "feat/1126_worktree-context-header-and-session-tabs",
                    roles.on_surface_variant,
                )
                .role(TypeRole::Label)
                .max_width(120.0),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `StatusChip` — one chip per status category and the neutral one, a stale one and one cut short.
pub fn status_chip<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    use micold_core::jira_status::{StatusCategory, TicketStatus};
    let status = |name: &str, category| TicketStatus {
        name: name.to_string(),
        category,
    };
    let chip = |label: &'static str, status: TicketStatus, stale: bool| {
        posed(
            label,
            Element::<Message>::from(material::StatusChip::new(&status, roles).stale(stale)),
            roles,
        )
    };
    arrange(
        vec![
            chip("To Do", status("To Do", Some(StatusCategory::ToDo)), false),
            chip(
                "In Progress",
                status("In Review", Some(StatusCategory::InProgress)),
                false,
            ),
            chip("Done", status("Done", Some(StatusCategory::Done)), false),
            chip("no category", status("Triage", None), false),
            chip(
                "stale",
                status("In Review", Some(StatusCategory::InProgress)),
                true,
            ),
            chip(
                "long name",
                status(
                    "Waiting for customer confirmation",
                    Some(StatusCategory::InProgress),
                ),
                false,
            ),
        ],
        Layout::Inline,
    )
}

/// `IconLabel` — a glyph and a label as one unit, posed at the three roles it is used at.
///
/// The gallery is where the distinction this type exists for is visible: the glyph tracks the
/// label's role, so a `Label`-role instance carries a smaller picture than a `Body`-role one. A
/// button's leading icon does *not* behave this way — §7.3 fixes it at 18dp whatever the label
/// says — and the button section poses that separately.
pub fn icon_label<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "Label role",
                material::IconLabel::<Message>::new(Icon::Git, "git", TypeRole::Label, roles)
                    .tint(roles.primary)
                    .muted(),
                roles,
            ),
            posed(
                "Body role",
                material::IconLabel::<Message>::new(Icon::Git, "git", TypeRole::Body, roles)
                    .tint(roles.primary),
                roles,
            ),
            posed(
                "untinted",
                material::IconLabel::<Message>::new(
                    Icon::Settings,
                    "settings",
                    TypeRole::Body,
                    roles,
                ),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `ActivityBadge` — one instance per signal, including the ones that deliberately render nothing, so
/// "this signal is invisible on purpose" is on the page rather than inferred from an absence.
///
/// The last instance is not a signal. `Stopped` is feature 026's tab mark, which reaches this
/// component through `for_emphasis` because a process lifecycle is not daemon activity — so it is
/// posed through the same door, rather than through a signal that does not map to it. Posing it
/// beside the other three is also the check the variant exists for: `Attention` and `Stopped` are
/// both in the error role and must not read as the same dot.
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
    let mut instances: Vec<Element<'a, Message>> = signals
        .into_iter()
        .map(|(label, signal)| {
            posed(
                label,
                material::ActivityBadge::<Message>::new(signal, roles),
                roles,
            )
        })
        .collect();
    instances.push(posed(
        "Stopped",
        material::ActivityBadge::<Message>::for_emphasis(
            Some(material::BadgeEmphasis::Stopped),
            roles,
        ),
        roles,
    ));
    arrange(instances, Layout::Inline)
}

/// `UnreadMark` — on a session row, beside the same row without it, and on the switcher's button
/// (feature 039, FR-030). The switcher's rows with their unread counts are `MenuOverlay`'s second
/// opener, because a menu's rows exist only inside its panel.
///
/// With the count, on a sidebar location row collapsed and expanded, beside one without (feature
/// 575, FR-012): what has to be checked by eye is that the count sits where a session row's mark
/// does, and that the location row's name keeps its weight.
///
/// Posed in its host and not alone: what has to be checked by eye is that the mark is told from the
/// activity badge, which both rows carry, by its place at the trailing edge and by the weight of
/// the name (FR-018), and that the two rows are the same height (FR-032).
pub fn unread_mark<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let session_row = |unread: bool| -> Element<'a, Message> {
        let row = material::TreeItem::new(1, samples::LABEL, roles.on_surface)
            .badge(material::ActivityBadge::<Message>::new(
                ActivitySignal::AwaitingInput,
                roles,
            ))
            .annotation("claude", roles.on_surface_variant)
            .unread(unread)
            .on_press(Message::NoOp);
        material::TreeView::new(vec![row], roles)
            .density(micold_core::tokens::density::DENSE)
            .label_role(TypeRole::SidebarName)
            .selected_label_role(TypeRole::SidebarSessionCurrent)
            .into()
    };
    // A worktree row as the sidebar builds it: the count whether collapsed or expanded, and the
    // expanded row followed by its two unread session rows.
    let location_row = |unread: usize, expanded: bool| -> Element<'a, Message> {
        let name = samples::WORKTREES[1].0;
        let mut rows = vec![material::TreeItem::new(0, name, roles.on_surface)
            .expandable(expanded, Message::NoOp)
            .unread_count(unread)
            .on_press(Message::NoOp)];
        if expanded {
            for label in [samples::LABEL, samples::OTHER_LABEL] {
                rows.push(
                    material::TreeItem::new(1, label, roles.on_surface)
                        .badge(material::ActivityBadge::<Message>::new(
                            ActivitySignal::AwaitingInput,
                            roles,
                        ))
                        .annotation("claude", roles.on_surface_variant)
                        .unread(true)
                        .on_press(Message::NoOp),
                );
            }
        }
        material::TreeView::new(rows, roles)
            .density(micold_core::tokens::density::DENSE)
            .label_role(TypeRole::SidebarName)
            .selected_label_role(TypeRole::SidebarSessionCurrent)
            .into()
    };
    // A session that started agents, folded: their count, and the mark when one needs the user
    // (feature 551, FR-011).
    let folded_row = |attention: bool| -> Element<'a, Message> {
        let row = material::TreeItem::new(1, samples::LABEL, roles.on_surface)
            .annotation("claude", roles.on_surface_variant)
            .expandable(false, Message::NoOp)
            .folded_summary(3, attention)
            .on_press(Message::NoOp);
        material::TreeView::new(vec![row], roles)
            .density(micold_core::tokens::density::DENSE)
            .label_role(TypeRole::SidebarName)
            .selected_label_role(TypeRole::SidebarSessionCurrent)
            .into()
    };
    // The switcher's button as the app bar builds it. A count of zero is the button as it was.
    let switcher_button = |unread: usize| -> Element<'a, Message> {
        material::Button::text(samples::LABEL, roles)
            .leading(Icon::OpenProject)
            .trailing_mark(unread, material::unread_total_tooltip(unread))
            .on_press(Message::NoOp)
            .into()
    };
    // The Overview button as the app bar builds it: toggled, and with a tinted badge that carries
    // the state's icon and the count (feature 813, FR-012).
    let overview_button = |open: bool, glyph: Icon, tint: Rgb| -> Element<'a, Message> {
        material::Button::text("Overview", roles)
            .leading(Icon::Overview)
            .selected(open)
            .mark(
                material::UnreadMark::new(roles)
                    .icon(glyph)
                    .count(2)
                    .role(TypeRole::Action)
                    .tint(tint),
            )
            .on_press(Message::NoOp)
            .into()
    };
    arrange(
        vec![
            posed("an unread session row", session_row(true), roles),
            posed("a read session row", session_row(false), roles),
            posed(
                "a collapsed worktree row with an unread count",
                location_row(2, false),
                roles,
            ),
            posed(
                "the same worktree row expanded",
                location_row(2, true),
                roles,
            ),
            posed("a worktree row with none", location_row(0, false), roles),
            posed(
                "a folded origin with three agents",
                folded_row(false),
                roles,
            ),
            posed(
                "a folded origin with a child needing attention",
                folded_row(true),
                roles,
            ),
            posed(
                "the switcher's button with an unread count",
                switcher_button(3),
                roles,
            ),
            posed("the switcher's button with none", switcher_button(0), roles),
            posed(
                "the Overview button toggled on, with a needs-action badge",
                overview_button(true, Icon::CardNeedsAction, roles.error),
                roles,
            ),
            posed(
                "the Overview button with a needs-attention badge",
                overview_button(false, Icon::CardNeedsAttention, roles.warning),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// `StateIcon` — the three states and the empty slot of an unknown one, each on a worktree row as
/// the sidebar builds it, and one row with tags (feature 905, FR-012). What has to be checked by
/// eye is that the triangle, octagon and two bars read as three shapes in both themes, and that
/// the name starts in the same column whatever the state.
pub fn state_icon<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    use micold_core::location_state::{BlockedReason, LocationState};
    let row = |state: LocationState, tagged: bool| -> Element<'a, Message> {
        let mut item = material::TreeItem::new(0, samples::WORKTREES[1].0, roles.on_surface)
            .state_icon(material::StateIcon::new(state, roles))
            .on_press(Message::NoOp);
        if tagged {
            item = item.tags(vec![
                ("feat".to_string(), roles.primary),
                ("local".to_string(), roles.secondary),
            ]);
        }
        material::TreeView::new(vec![item], roles)
            .density(micold_core::tokens::density::DENSE)
            .label_role(TypeRole::SidebarName)
            .selected_label_role(TypeRole::SidebarSessionCurrent)
            .into()
    };
    arrange(
        vec![
            posed("Running", row(LocationState::Running, false), roles),
            posed(
                "Blocked",
                row(
                    LocationState::Blocked(BlockedReason::NeedsPermission),
                    false,
                ),
                roles,
            ),
            posed("Idle", row(LocationState::Idle, false), roles),
            posed("Unknown", row(LocationState::Unknown, false), roles),
            posed(
                "a worktree row with tags carrying it",
                row(LocationState::Running, true),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// `PullRequestIndicator` — the four states alone, open and draft with each check status, and the
/// stale form of open-passing and of merged (feature 040, FR-033). What has to be checked by eye is
/// that the seven glyphs read as seven shapes in both themes, and that the stale form is quieter.
pub fn pull_request_indicator<'a>(
    _s: &'a Showcase,
    roles: Roles,
    _i: usize,
) -> Element<'a, Message> {
    use material::{CheckMark, PrMark, PullRequestIndicator};
    let pose = |label: &'static str,
                state: PrMark,
                checks: Option<CheckMark>,
                stale: bool|
     -> Element<'a, Message> {
        let mut indicator = PullRequestIndicator::<Message>::new(state, &roles).stale(stale);
        if let Some(checks) = checks {
            indicator = indicator.checks(checks);
        }
        posed(label, Element::from(indicator), roles)
    };
    let (pending, passing, failing) = (CheckMark::Pending, CheckMark::Passing, CheckMark::Failing);
    arrange(
        vec![
            pose("open", PrMark::Open, None, false),
            pose("draft", PrMark::Draft, None, false),
            pose("merged", PrMark::Merged, None, false),
            pose("closed", PrMark::Closed, None, false),
            pose("open, checks passing", PrMark::Open, Some(passing), false),
            pose("open, checks pending", PrMark::Open, Some(pending), false),
            pose("open, checks failing", PrMark::Open, Some(failing), false),
            pose("draft, checks passing", PrMark::Draft, Some(passing), false),
            pose("draft, checks pending", PrMark::Draft, Some(pending), false),
            pose("draft, checks failing", PrMark::Draft, Some(failing), false),
            pose(
                "open, checks passing, stale",
                PrMark::Open,
                Some(passing),
                true,
            ),
            pose("merged, stale", PrMark::Merged, None, true),
        ],
        Layout::Inline,
    )
}

/// `UsageIndicator` — the plan's usage in the app bar (feature 490, contract U4): a label with its
/// reset today, with the details as its tooltip, the warning look, and the widest label, a full
/// plan resetting on another day. What has to be checked by eye is the glyph–label gap, the
/// warning glyph's shape, and the 40 px height beside the bar's other actions.
pub fn usage_indicator<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    use material::UsageIndicator;
    let details = || {
        samples::USAGE_DETAILS
            .iter()
            .map(|l| l.to_string())
            .collect()
    };
    arrange(
        vec![
            posed(
                "normal",
                UsageIndicator::<Message>::new(samples::USAGE_LABEL, &roles).details(details()),
                roles,
            ),
            posed(
                "warning",
                UsageIndicator::<Message>::new(samples::USAGE_WARNING_LABEL, &roles).warning(true),
                roles,
            ),
            posed(
                "long label",
                UsageIndicator::<Message>::new(samples::USAGE_LONG_LABEL, &roles),
                roles,
            ),
        ],
        Layout::Inline,
    )
}
