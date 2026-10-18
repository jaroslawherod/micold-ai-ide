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

/// The Run in parallel dialog (feature 483), posed with three runs on mixed providers and the names
/// they will get, and again with a validation error. Held in statics: the element borrows it.
fn parallel_dialog_pose(error: bool) -> &'static crate::features::runs::ParallelDialog {
    use micold_core::session::AiCli;
    use std::sync::OnceLock;
    static VALID: OnceLock<crate::features::runs::ParallelDialog> = OnceLock::new();
    static INVALID: OnceLock<crate::features::runs::ParallelDialog> = OnceLock::new();
    let make = |error: bool| crate::features::runs::ParallelDialog {
        required: Default::default(),
        project: std::path::PathBuf::from("/p"),
        naming: micold_core::naming::WorktreeNaming {
            type_: Some(ConventionalType::Feat),
            ticket: None,
            name: if error {
                String::new()
            } else {
                "login page".to_string()
            },
        },
        prompt: if error {
            String::new()
        } else {
            "Add a login page".to_string()
        },
        base_branch: "main".to_string(),
        branches: vec!["main".to_string(), "develop".to_string()],
        runs: vec![AiCli::ClaudeCode, AiCli::Copilot, AiCli::Pi],
        offered: AiCli::ALL.to_vec(),
        default_cli: AiCli::ClaudeCode,
        error: error.then(|| "Enter a prompt".to_string()),
    };
    if error {
        INVALID.get_or_init(|| make(true))
    } else {
        VALID.get_or_init(|| make(false))
    }
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

/// `Scrollable` — content larger than its viewport on each axis, so the themed scrollbar is
/// actually visible and both directions can be compared.
///
/// The horizontal instance is posed beside the vertical one rather than instead of it: the axis is
/// a variant now (feature 026 FR-002a), and the thing worth seeing is that it is the *same* 4px
/// themed bar turned through ninety degrees — not a second scrollbar that arrived with the second
/// axis.
pub fn scrollable<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let mut tall = iced::widget::column![].spacing(spacing::XS);
    for (name, _) in samples::WORKTREES.iter().cycle().take(12) {
        tall = tall.push(material::Text::new(*name, TypeRole::Body, roles));
    }
    let mut wide = iced::widget::row![].spacing(spacing::MD);
    for (name, _) in samples::WORKTREES.iter().cycle().take(12) {
        wide = wide.push(material::Text::new(*name, TypeRole::Body, roles));
    }
    arrange(
        vec![
            posed(
                "Vertical — scroll it",
                iced::widget::container(material::Scrollable::new(tall, roles))
                    .height(Length::Fixed(120.0)),
                roles,
            ),
            posed(
                "Horizontal — scroll it",
                iced::widget::container(
                    material::Scrollable::new(wide, roles)
                        .direction(material::ScrollDirection::Horizontal),
                )
                .width(Length::Fixed(220.0)),
                roles,
            ),
        ],
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

/// `AccordionSection` — a titled section, closed and open, and one that toggles live.
pub fn accordion_section<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    let body = |roles: Roles| material::Text::new(samples::BODY, TypeRole::Body, roles);
    let section =
        |open: bool| material::AccordionSection::new("Network", body(roles), roles).open(open);
    arrange(
        vec![
            posed("closed", section(false), roles),
            posed("open", section(true), roles),
            posed(
                "press the header, or focus it and press Space or Enter",
                section(showcase.shown(index)).on_toggle(Message::Reversed(index)),
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

/// `ConnectionBanner` — both notice levels, and one carrying an action **on each of them**.
///
/// The two action specimens sit side by side because that pair is the rule made visible. On `Error`
/// the button takes `on_error`, the fill's own paired foreground; on `Info` it keeps §7.3's
/// `primary`, because `surface_variant` is inside §1.3's neutral enumeration. Showing only the red
/// one would read as "the banner recolours its button" — showing both says the button asks what it
/// is standing on, and gets a different answer (FR-004a, BUG-009 T156).
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
            posed(
                "with an action, on Info",
                material::ConnectionBanner::new(
                    "Reconnecting to the session service",
                    "You can keep working; unsent changes are queued.",
                    roles,
                )
                .level(NoticeLevel::Info)
                .action("Retry now", Message::NoOp),
                roles,
            ),
            posed(
                "with two actions, on Info",
                material::ConnectionBanner::new(
                    "2 worktrees found",
                    "Attach them to this project, or dismiss this for now.",
                    roles,
                )
                .level(NoticeLevel::Info)
                .action("Attach all", Message::NoOp)
                .secondary_action("Dismiss", Message::NoOp),
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

/// `ProgressBar` at none, most and all of a task list.
pub fn progress_bar<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    use micold_core::task_progress::Ratio;
    let bar = |done| {
        iced::widget::container(material::ProgressBar::new(Ratio { done, total: 10 }, roles))
            .width(Length::Fixed(320.0))
    };
    arrange(
        vec![
            posed("0/10", bar(0), roles),
            posed("6/10", bar(6), roles),
            posed("10/10", bar(10), roles),
        ],
        Layout::FullWidth,
    )
}

/// A sample card for the Overview poses.
fn card_model(
    name: &str,
    kind: micold_core::overview::LocationType,
    state: micold_core::overview::CardState,
    reason: micold_core::overview::CardReason,
) -> micold_core::overview::CardModel {
    micold_core::overview::CardModel {
        location: micold_core::session::SessionLocation::Worktree(name.to_string()),
        display_name: name.to_string(),
        branch: Some(format!("feat/{name}")),
        kind,
        sessions: (3, 2),
        state,
        reason,
        summary: "Adds the worktree overview".to_string(),
        prs: micold_core::overview::PrDisplay::Counts(micold_core::overview::PrCounts {
            open: 1,
            merged: 2,
            open_capped: false,
            merged_capped: false,
        }),
        progress: Default::default(),
        activity: None,
        activation: micold_core::overview::Activation::RevealOnly(
            micold_core::session::SessionLocation::Default,
        ),
    }
}

/// `OverviewCard` in each of the five states, the Default (project root) card and an orphan.
pub fn overview_card<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    use micold_core::overview::{CardReason as R, CardState as S, LocationType as T};
    let card = |m: micold_core::overview::CardModel| {
        iced::widget::container(Element::from(material::OverviewCard::<Message>::new(
            &m, roles,
        )))
        .width(Length::Fixed(320.0))
    };
    let mut default = card_model("Default", T::ProjectRoot, S::Idle, R::Idle);
    default.branch = Some("main".to_string());
    let mut orphan = card_model(
        "gone-worktree",
        T::OrphanInvalid,
        S::NeedsAttention,
        R::MergeConflict,
    );
    orphan.branch = None;
    orphan.prs = micold_core::overview::PrDisplay::Hidden;
    let mut unknown = card_model("unknown-counts", T::Feature, S::Idle, R::Idle);
    unknown.prs =
        micold_core::overview::PrDisplay::Unknown(micold_core::overview::PrUnknown::Unreachable);
    let mut with_progress = card_model("with-progress", T::Feature, S::Working, R::Working);
    with_progress.progress = micold_core::task_progress::Progress {
        phase: Some(micold_core::task_progress::Phase::Implement),
        ratio: Some(micold_core::task_progress::Ratio { done: 6, total: 10 }),
        source: micold_core::task_progress::Source::TaskList,
    };
    let mut with_activity = card_model("with-activity", T::Feature, S::Working, R::Working);
    with_activity.activity = Some(micold_core::hook_activity::CurrentActivity {
        line: "Writing the activity line".to_string(),
        ratio: Some(micold_core::task_progress::Ratio { done: 2, total: 5 }),
    });
    with_activity.progress = with_progress.progress;
    arrange(
        vec![
            posed("with progress", card(with_progress), roles),
            posed("with activity", card(with_activity), roles),
            posed(
                "without progress",
                card(card_model("without-progress", T::Feature, S::Idle, R::Idle)),
                roles,
            ),
            posed(
                "Needs action",
                card(card_model(
                    "needs-action",
                    T::Feature,
                    S::NeedsAction,
                    R::NeedsPermission,
                )),
                roles,
            ),
            posed(
                "Needs attention",
                card(card_model(
                    "needs-attention",
                    T::Bugfix,
                    S::NeedsAttention,
                    R::ChecksFailing,
                )),
                roles,
            ),
            posed(
                "Working",
                card(card_model("working", T::Chore, S::Working, R::Working)),
                roles,
            ),
            posed(
                "Done",
                card(card_model("done", T::Other, S::Done, R::Merged)),
                roles,
            ),
            posed(
                "Idle",
                card(card_model("idle", T::Included, S::Idle, R::Idle)),
                roles,
            ),
            posed("Default", card(default), roles),
            posed("orphan", card(orphan), roles),
            posed("unknown counts", card(unknown), roles),
        ],
        Layout::FullWidth,
    )
}

/// `CardGrid` with 1, 3 and 30 cards, and in a narrow room.
pub fn card_grid<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    use micold_core::overview::{CardReason as R, CardState as S, LocationType as T};
    let grid = |n: usize, width: Length| {
        let cards = (0..n)
            .map(|i| {
                let state = S::ALL[i % S::ALL.len()];
                material::OverviewCard::<Message>::new(
                    &card_model(&format!("worktree-{i}"), T::Feature, state, R::Working),
                    roles,
                )
            })
            .collect();
        iced::widget::container(Element::from(material::CardGrid::new(cards, roles)))
            .width(width)
            .height(Length::Fixed(320.0))
    };
    arrange(
        vec![
            posed("1 card", grid(1, Length::Fill), roles),
            posed("3 cards", grid(3, Length::Fill), roles),
            posed("30 cards", grid(30, Length::Fill), roles),
            posed("narrow", grid(3, Length::Fixed(279.0)), roles),
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
    // The Run in parallel group row (feature 483, parallel-surfaces G1–G3): expanded with a failed
    // run that has no worktree, and collapsed with its counts.
    let group = |expanded: bool| {
        let mut items = vec![material::TreeItem::new(0, samples::GROUP, roles.on_surface)
            .with_icon(Icon::Git)
            .tags(vec![
                ("3 runs".to_string(), roles.secondary),
                ("1 failed".to_string(), roles.error),
            ])
            .expandable(expanded, Message::NoOp)];
        if expanded {
            items.push(
                material::TreeItem::new(1, "#1", roles.on_surface)
                    .with_icon(Icon::Git)
                    .annotation("claude", roles.on_surface_variant)
                    .on_press(Message::NoOp),
            );
            items.push(
                material::TreeItem::new(1, "#2", roles.on_surface)
                    .with_icon(Icon::Git)
                    .annotation("copilot", roles.on_surface_variant)
                    .on_press(Message::NoOp),
            );
            items.push(
                material::TreeItem::new(1, "#3", roles.error)
                    .annotation("pi", roles.on_surface_variant)
                    .tags(vec![("failed".to_string(), roles.error)])
                    .row_tooltip(samples::RUN_FAILED_REASON),
            );
        }
        material::TreeView::new(items, roles)
    };
    // Sessions an agent started (feature 551): nested under their origin, and an orphan whose
    // origin has not appeared, each carrying the child marker.
    let spawned = {
        let session = |depth: u16, label: &str, child: bool, note: &str| {
            material::TreeItem::new(depth, label.to_string(), roles.on_surface)
                .annotation(note.to_string(), roles.on_surface_variant)
                .child(child)
                .on_press(Message::NoOp)
        };
        material::TreeView::new(
            vec![
                session(1, samples::LABEL, false, "claude"),
                session(2, "background agent", true, "claude"),
                session(3, "its helper", true, "claude"),
                session(1, "orphaned agent", true, "origin pending, claude"),
            ],
            roles,
        )
    };
    // A row a navigation chord gave the sidebar's keyboard (feature 803, FR-010): the focus ring on
    // the selected pill, shown as keyboard focus.
    let keyboard_focused = material::TreeView::new(
        vec![
            material::TreeItem::new(0, samples::WORKTREES[0].0, roles.on_surface)
                .with_icon(Icon::Git)
                .on_press(Message::NoOp)
                .keyboard_focus(false, |_| Message::NoOp),
            material::TreeItem::new(0, samples::WORKTREES[1].0, roles.on_surface)
                .with_icon(Icon::Git)
                .selected(true)
                .on_press(Message::NoOp)
                .keyboard_focus(true, |_| Message::NoOp),
        ],
        roles,
    )
    .selected_label_role(material::TypeRole::Label);
    arrange(
        vec![
            posed("keyboard-focused worktree row", keyboard_focused, roles),
            posed("sessions started by an agent", spawned, roles),
            posed("run group, expanded", group(true), roles),
            posed("run group, collapsed", group(false), roles),
            posed(
                "Run in parallel dialog",
                crate::ui::parallel_dialog::modal(parallel_dialog_pose(false), roles, None)
                    .map(|_| Message::NoOp),
                roles,
            ),
            posed(
                "Run in parallel dialog, invalid",
                crate::ui::parallel_dialog::modal(parallel_dialog_pose(true), roles, None)
                    .map(|_| Message::NoOp),
                roles,
            ),
            posed(
                "Dismiss group dialog",
                crate::ui::confirm_dismiss_group::modal("login page", roles).map(|_| Message::NoOp),
                roles,
            ),
            posed(
                "Cleanup offer",
                crate::ui::compare::cleanup_modal(&cleanup_pose(None), None, roles)
                    .map(|_| Message::NoOp),
                roles,
            ),
            posed(
                "Cleanup offer, second confirmation",
                crate::ui::compare::second_confirmation_modal(&cleanup_pose(Some(vec![3])), roles)
                    .map(|_| Message::NoOp),
                roles,
            ),
            posed(
                "a worktree tree",
                material::TreeView::new(items, roles)
                    // The selected row's heavier label (feature 024, FR-003a). Posed here because it is
                    // the half of a selected row that a colour-blind reader, or a greyscale screenshot,
                    // has to rely on — and the gallery is where that is checked.
                    .selected_label_role(material::TypeRole::Label),
                roles,
            ),
        ],
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

/// `SectionList` — the navigation rail a full-surface view is divided by (feature 027, FR-026a).
///
/// One live instance rather than several posed ones. The component's whole subject is *which row is
/// current*, and static rails each claiming a different row would show the same thing several times
/// without ever showing the transition between them. The badge rides on the last row, which is
/// where the settings surface puts it.
///
/// The collapse control is live for the same reason (FR-026d). Collapsed, a row is its icon and
/// nothing else, and the claim that costs nothing — every destination still pressable, the badged
/// row still marked — is a claim about a rail you can collapse and then press, not about a second
/// picture of one.
pub fn section_list<'a>(showcase: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let mut sections = vec![
        material::Section::new("Appearance", Message::SectionShown(0)),
        material::Section::new("Terminal", Message::SectionShown(1)),
        material::Section::new("Environment", Message::SectionShown(2)),
        material::Section::new("Daemons", Message::SectionShown(3)),
    ];
    // The settings surface's own four icons, so the page shows the glyphs a reader will actually
    // meet rather than four stand-ins that happen to be distinguishable.
    for (row, icon) in sections.iter_mut().zip([
        Icon::LightMode,
        Icon::RegularTerminal,
        Icon::AiCli,
        Icon::Daemons,
    ]) {
        row.icon = Some(icon);
    }
    sections[3].badge = Some("Sharing".to_string());

    arrange(
        vec![posed(
            "press a row to make it current; collapse it to its icons",
            iced::widget::container(
                material::SectionList::new(sections, roles)
                    .selected(showcase.section_shown())
                    .collapsed(showcase.section_rail_collapsed())
                    .toggle(Message::SectionRailToggled)
                    .badge_accent(roles.error, roles.on_error),
            )
            .height(Length::Fixed(240.0)),
            roles,
        )],
        Layout::Inline,
    )
}

/// The cleanup offer after run 2 was merged: run 1 is clean and selected, run 3 holds uncommitted
/// changes and starts unselected (feature 483, K1-K3).
fn cleanup_pose(confirming: Option<Vec<u8>>) -> crate::features::runs::CleanupOffer {
    use crate::features::runs::{CleanupLoser, CleanupOffer, Uncommitted};
    let loser = |number: u8, sessions: usize, selected: bool, uncommitted| CleanupLoser {
        number,
        dir_name: format!("feat-login-page-{number}"),
        branch: format!("feat/login-page-{number}"),
        sessions,
        selected,
        uncommitted,
        confirmed: false,
        delete_branch: true,
        reading: None,
        touched: false,
    };
    CleanupOffer {
        group: micold_core::runs::GroupId(uuid::Uuid::nil()),
        project: std::path::PathBuf::from("/p"),
        base_branch: "main".into(),
        heading: "Run 2 was merged into main".into(),
        losers: vec![
            loser(1, 2, true, Uncommitted::Clean),
            loser(3, 1, false, Uncommitted::Uncommitted),
        ],
        rechecking: false,
        confirming,
    }
}
