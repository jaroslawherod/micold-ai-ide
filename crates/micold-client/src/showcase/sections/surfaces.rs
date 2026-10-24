//! Surfaces, containers and the list components (feature 020, T020).
//!
//! The larger components: the surface every panel sits on, a scroll viewport, an accordion, an app
//! bar, a banner, a progress indicator, a tree, and the navigation drawer. Several are naturally
//! full-width, which is why the catalogue gives them their own rows — a banner beside a chip would
//! push the chip off screen (spec, Edge Cases).

use iced::widget::column;
use iced::{Element, Length};
use micold_core::naming::ConventionalType;
use micold_core::tokens::{spacing, Roles};

use crate::features::notifications::NoticeLevel;
use crate::icons::Icon;
use crate::showcase::catalogue::Layout;
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::samples;
use crate::showcase::state::{Message, Showcase, GROUP_DEFAULT};
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

/// The quick-switcher dialog (feature 968), posed with a query that emphasises its match, a
/// location row with tags, a session row below it and the current location's row; `nothing_matches` poses the empty list.
fn quick_switcher_pose(roles: Roles, nothing_matches: bool) -> Element<'static, Message> {
    use crate::ui::quick_switcher::{context_line, surface, View};
    use micold_core::quick_switch::EntryKey;
    use micold_core::session::SessionLocation;
    let (query, rows, keys) = if nothing_matches {
        ("zzz", Vec::new(), Vec::new())
    } else {
        (
            "log",
            vec![
                material::TypeaheadRow::new("Login page", std::iter::once(0..3).collect()).details(
                    context_line(
                        crate::features::quick_switch::Kind::Location,
                        "feat · PROJ-12",
                    ),
                    Vec::new(),
                ),
                material::TypeaheadRow::new("Login flow", std::iter::once(0..3).collect()).details(
                    context_line(crate::features::quick_switch::Kind::Session, "Login page"),
                    Vec::new(),
                ),
                material::TypeaheadRow::new("Default", vec![]).details(
                    context_line(crate::features::quick_switch::Kind::Location, ""),
                    Vec::new(),
                ),
                material::TypeaheadRow::new("Archive", vec![])
                    .details(
                        context_line(crate::features::quick_switch::Kind::Project, "unavailable"),
                        Vec::new(),
                    )
                    .disabled(),
            ],
            vec![
                EntryKey::Location(SessionLocation::Worktree("feat-login".into())),
                EntryKey::Session(micold_core::session::SessionId(uuid::Uuid::from_u128(1))),
                EntryKey::Location(SessionLocation::Default),
                EntryKey::Project("/archive".into()),
            ],
        )
    };
    surface(
        View {
            query,
            highlighted: (!rows.is_empty()).then_some(0),
            current: (!rows.is_empty()).then_some(2),
            rows,
            keys,
            empty_message: "No matches",
        },
        roles,
    )
    .map(|_| Message::NoOp)
}

/// The Run in parallel dialog (feature 483), posed with three runs on mixed providers and the names
/// they will get, and again with a validation error. Held in statics: the element borrows it.
fn parallel_dialog_pose(error: bool) -> &'static crate::features::runs::ParallelDialog {
    use micold_core::session::AiCli;
    use std::sync::OnceLock;
    static VALID: OnceLock<crate::features::runs::ParallelDialog> = OnceLock::new();
    static INVALID: OnceLock<crate::features::runs::ParallelDialog> = OnceLock::new();
    let make = |error: bool| crate::features::runs::ParallelDialog {
        // The error pose is the dialog after a confirm with the prompt and name empty: both read
        // `Missing` (feature 937).
        required: {
            let mut marks = micold_core::required::RequiredMarks::default();
            if error {
                marks.attempt();
            }
            marks
        },
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
        error: None,
        name_error: None,
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

/// `AccordionSection` — a titled section closed with its summary, open, closed with a required
/// field still empty, and one that toggles live (feature 890, FR-015).
pub fn accordion_section<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    let body = |roles: Roles| material::Text::new(samples::BODY, TypeRole::Body, roles);
    let section = |open: bool| {
        material::AccordionSection::new("Network", body(roles), roles)
            .open(open)
            .summary("Docker · Basic image")
    };
    arrange(
        vec![
            posed("closed, with a summary", section(false), roles),
            posed("open", section(true), roles),
            posed(
                "closed, a required field empty",
                section(false).required(true),
                roles,
            ),
            posed(
                "press the header, or focus it and press Space or Enter",
                section(showcase.shown(index)).on_toggle(Message::Reversed(index)),
                roles,
            ),
            posed(
                "a group: opening one closes the others",
                column(["Network", "Storage", "Tools"].into_iter().enumerate().map(
                    |(key, title)| {
                        material::AccordionSection::new(title, body(roles), roles)
                            .in_group(showcase.group(), key, GROUP_DEFAULT)
                            .summary("Docker · Basic image")
                            .on_toggle(Message::GroupSectionToggled(key))
                            .into()
                    },
                )),
                roles,
            ),
            posed(
                "independent sections",
                column([0, 1].map(|key| {
                    section(showcase.independent_open(key))
                        .on_toggle(Message::IndependentSectionToggled(key))
                        .into()
                })),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// What a posed `DisclosureCard`'s status says (feature 939).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPose {
    /// The result came back.
    Done,
    /// No result yet.
    Running,
    /// The result is an error: "Failed", `Icon::ToolFailed` and the error role (FR-012).
    Failed,
}

/// One posed tool-call card: a `DisclosureCard` with the sample tool's icon, summary, status and
/// monospace input. `pub` so `tests/gates/disclosure_card_header_fits.rs` lays out exactly what the
/// gallery shows.
pub fn tool_card<'a>(
    pose: ToolPose,
    open: bool,
    on_toggle: Message,
    roles: Roles,
) -> Element<'a, Message> {
    let (icon, label) = match pose {
        ToolPose::Done => (Icon::ToolDone, "Done"),
        ToolPose::Running => (Icon::ToolRunning, "Running"),
        ToolPose::Failed => (Icon::ToolFailed, "Failed"),
    };
    let status = material::IconLabel::new(icon, label, TypeRole::Label, roles);
    let status = match pose {
        ToolPose::Failed => status.tint(roles.error).label_tint(roles.error),
        ToolPose::Done | ToolPose::Running => status.muted(),
    };
    material::DisclosureCard::new(samples::TOOL_NAME, on_toggle, roles)
        .leading(Icon::ToolCall)
        .summary(samples::TOOL_SUMMARY)
        .trailing(status)
        .open(open)
        .body(material::Text::new(samples::TOOL_INPUT, TypeRole::Body, roles).monospace())
        .failed(pose == ToolPose::Failed)
        .into()
}

/// `DisclosureCard` — a tool-call card folded, open, running and failed, a thinking card, and one
/// that toggles live (feature 939, research R11).
pub fn disclosure_card<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    // The posed cards are pictures of a state: pressing one changes nothing. The last is live.
    let inert = || Message::NoOp;
    arrange(
        vec![
            posed(
                "folded",
                tool_card(ToolPose::Done, false, inert(), roles),
                roles,
            ),
            posed(
                "open",
                tool_card(ToolPose::Done, true, inert(), roles),
                roles,
            ),
            posed(
                "running",
                tool_card(ToolPose::Running, false, inert(), roles),
                roles,
            ),
            posed(
                "failed, with its label and icon",
                tool_card(ToolPose::Failed, false, inert(), roles),
                roles,
            ),
            posed(
                "a thinking card",
                material::DisclosureCard::new("Thinking", inert(), roles)
                    .leading(Icon::Thinking)
                    .summary(samples::BODY)
                    .body(material::Text::new(samples::BODY, TypeRole::Body, roles)),
                roles,
            ),
            posed(
                "an Edit card with its line counts",
                edit_card(false, inert(), roles),
                roles,
            ),
            posed(
                "press the header, or focus it and press Space or Enter",
                tool_card(
                    ToolPose::Done,
                    showcase.shown(index),
                    Message::Reversed(index),
                    roles,
                ),
                roles,
            ),
            posed(
                "a transcript of a recorded tool turn: press a card's header",
                transcript_sample(showcase, roles),
                roles,
            ),
            posed(
                "a recorded subagent turn with its session bar: press the thread's header",
                insights_sample(showcase, roles),
                roles,
            ),
            posed(
                "the session bar: context meter with its threshold, and the session totals",
                session_bar_sample(BarPose::Normal, roles),
                roles,
            ),
            posed(
                "the session bar near compaction: icon and words, not colour alone",
                session_bar_sample(BarPose::Warning, roles),
                roles,
            ),
            posed(
                "the session bar before the first reply",
                session_bar_sample(BarPose::NoData, roles),
                roles,
            ),
            posed(
                "the session bar with a background task running, its list unfolded",
                session_bar_sample(BarPose::Running, roles),
                roles,
            ),
            posed(
                "the session bar of a session in Bypass: its label, icon and words (#942)",
                session_bar_sample(BarPose::Bypass, roles),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// The transcript view over the recorded tool turn, through `BlockRenderers::PLAIN`, in a pane of
/// fixed height (feature 939, T037): its cards fold and unfold live.
fn transcript_sample<'a>(showcase: &'a Showcase, roles: Roles) -> Element<'a, Message> {
    let view = crate::ui::transcript::view(
        showcase.transcript(),
        samples::TRANSCRIPT_SESSION,
        &crate::ui::transcript::BlockRenderers::PLAIN,
        crate::ui::transcript::RenderCtx {
            roles,
            scheme: showcase.scheme,
            cache: showcase.block_cache(),
        },
    );
    iced::widget::container(view.map(Message::Transcript))
        .height(Length::Fixed(TRANSCRIPT_SAMPLE_HEIGHT))
        .into()
}

/// The transcript view with feature 944's insights over the recorded subagent turn, its CLI
/// running, in a pane of fixed height: the session bar on top, the `Agent` card's thread folded
/// under it and opening live (T042).
fn insights_sample<'a>(showcase: &'a Showcase, roles: Roles) -> Element<'a, Message> {
    let view = crate::ui::transcript::view_with_insights(
        showcase.transcript(),
        crate::ui::transcript::InsightsCtx {
            state: showcase.insights(),
            cli_running: true,
            mode: micold_core::session::PermissionMode::Default,
        },
        samples::INSIGHTS_SESSION,
        &crate::ui::transcript::BlockRenderers::PLAIN,
        crate::ui::transcript::RenderCtx {
            roles,
            scheme: showcase.scheme,
            cache: showcase.block_cache(),
        },
    );
    iced::widget::container(view.map(Message::Transcript))
        .height(Length::Fixed(TRANSCRIPT_SAMPLE_HEIGHT))
        .into()
}

/// Which session bar a pose shows (feature 944, T042).
#[derive(Clone, Copy, PartialEq, Eq)]
enum BarPose {
    Normal,
    Warning,
    NoData,
    Running,
    Bypass,
}

/// A structured session's bar (feature 944, X1) as `pose` says, its figures from the same view
/// model the transcript pane draws.
fn session_bar_sample(pose: BarPose, roles: Roles) -> Element<'static, Message> {
    use crate::ui::transcript::{
        meter_view, running_text, session_bar, task_status, SessionBarView, TaskRow,
    };
    use micold_core::structured::insights::{ContextState, Task, TaskKind, TaskState};

    let measured = |used: u64, warning| ContextState::Measured {
        used,
        window: 1_000_000,
        // Whole and rounded half up, as the core's reading is.
        percent: (used + 5_000) / 10_000,
        threshold: Some(800_000),
        warning,
    };
    let context = match pose {
        BarPose::Normal | BarPose::Running | BarPose::Bypass => measured(37_141, false),
        BarPose::Warning => measured(812_400, true),
        BarPose::NoData => ContextState::NoData,
    };
    let task = Task {
        task_id: "sample".into(),
        tool_use_id: None,
        description: "sleep 20; echo done".into(),
        kind: Some(TaskKind::Command),
        background: true,
        state: TaskState::Running,
        outcome: None,
        activity: None,
    };
    let running = pose == BarPose::Running;
    let view = SessionBarView {
        meter: meter_view(&context),
        totals: (pose != BarPose::NoData).then(|| "$0.1400 est. · 3 turns".to_owned()),
        totals_details: Vec::new(),
        running: running_text(usize::from(running)),
        tasks: if running {
            vec![TaskRow {
                kind: crate::ui::transcript::kind_icon(task.kind),
                description: task.description.clone(),
                status: task_status(&task, true),
            }]
        } else {
            Vec::new()
        },
    };
    let mode = if pose == BarPose::Bypass {
        micold_core::session::PermissionMode::Bypass
    } else {
        micold_core::session::PermissionMode::Default
    };
    session_bar::bar(&view, samples::TRANSCRIPT_SESSION, running, mode, roles)
        .map(Message::Transcript)
}

/// The transcript sample's pane height: a few entries in view, the rest a scroll away.
const TRANSCRIPT_SAMPLE_HEIGHT: f32 = 480.0;

/// `FieldGroup` — a labelled group of checkboxes with a note under its label, and without one
/// (feature 890, FR-008).
pub fn field_group<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let group = |note: Option<&str>| {
        let mut group = material::FieldGroup::new("AI tools in this daemon", roles)
            .push(material::Checkbox::new("Claude Code", true, roles).on_toggle(|_| Message::NoOp))
            .push(material::Checkbox::new("Codex", false, roles).on_toggle(|_| Message::NoOp));
        if let Some(note) = note {
            group = group.note(note);
        }
        group
    };
    arrange(
        vec![
            posed(
                "with a note",
                group(Some("Installed when the pod starts.")),
                roles,
            ),
            posed("without a note", group(None), roles),
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

/// `StatusBar` (feature 1124, B6): steady, bringing the sandbox up, with warnings, and too narrow
/// for its items — key hints dropped, the texts ending in "…".
pub fn status_bar<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    use material::{GiveWay, ItemTone, StatusBar, StatusItem};
    let connection = |text: &str, tone| {
        StatusItem::new(text)
            .dot(tone)
            .give_way(GiveWay::Shorten(2))
    };
    let hints = |bar: StatusBar<'a, Message>| {
        bar.hint(StatusItem::new("Ctrl+Shift+O Overview").give_way(GiveWay::Drop(0)))
            .hint(StatusItem::new("Ctrl+Shift+P Jump").give_way(GiveWay::Drop(0)))
    };
    let steady = || {
        hints(
            StatusBar::new(roles)
                .item(connection("Daemon connected · local", ItemTone::Positive))
                .item(StatusItem::new("Sandbox ready").give_way(GiveWay::Shorten(1)))
                .item(StatusItem::new("CLI: Claude Code").give_way(GiveWay::Shorten(0))),
        )
    };
    let busy = hints(
        StatusBar::new(roles)
            .item(connection("Daemon connected · local", ItemTone::Positive))
            .item(
                StatusItem::new("Getting image… 42%")
                    .busy(true)
                    .tooltip("Getting the sandbox image\nDownloading 42% — layer 3")
                    .give_way(GiveWay::Shorten(1)),
            ),
    );
    let warning = hints(
        StatusBar::new(roles)
            .item(connection(
                "Daemon disconnected · reconnecting…",
                ItemTone::Error,
            ))
            .item(
                StatusItem::new("Sandbox failed")
                    .tone(ItemTone::Warning)
                    .give_way(GiveWay::Shorten(1)),
            ),
    );
    arrange(
        vec![
            posed("steady", steady(), roles),
            posed("busy", busy, roles),
            posed("warning", warning, roles),
            posed(
                "too narrow",
                iced::widget::container(steady()).width(Length::Fixed(240.0)),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// `ProgressBar` at none, most and all of a task list; then with a marker at 8/10 (feature 944's
/// compaction threshold), below it and, as a warning bar, past it.
pub fn progress_bar<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    use micold_core::task_progress::Ratio;
    let ratio = |done| Ratio { done, total: 10 };
    let sized =
        |bar: material::ProgressBar| iced::widget::container(bar).width(Length::Fixed(320.0));
    let bar = |done| sized(material::ProgressBar::new(ratio(done), roles));
    arrange(
        vec![
            posed("0/10", bar(0), roles),
            posed("6/10", bar(6), roles),
            posed("10/10", bar(10), roles),
            posed(
                "6/10, marker 8/10",
                sized(material::ProgressBar::new(ratio(6), roles).marker(ratio(8))),
                roles,
            ),
            posed(
                "9/10, marker 8/10, warning",
                sized(
                    material::ProgressBar::new(ratio(9), roles)
                        .marker(ratio(8))
                        .warning(true),
                ),
                roles,
            ),
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
        branch: Some("feat/813_worktree-overview".to_string()),
        kind,
        sessions: (3, 2),
        state,
        reason,
        title: "Worktree overview: one card per location".to_string(),
        prs: micold_core::overview::PrDisplay::Counts(micold_core::overview::PrCounts {
            open: 1,
            merged: 2,
            open_capped: false,
            merged_capped: false,
        }),
        pull_requests: None,
        progress: Default::default(),
        activity: None,
        activation: micold_core::overview::Activation::RevealOnly(
            micold_core::session::SessionLocation::Default,
        ),
        dir: std::path::PathBuf::from("/home/dev/code/app"),
        issue_link: None,
    }
}

/// `OverviewCard` in each of the five states (in a `CardGrid`, so the even row height shows), the
/// Default (project root) card and an orphan; a grid whose first card expands and collapses with
/// its shadow easing (feature 1299).
pub fn overview_card<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    use micold_core::overview::{CardReason as R, CardState as S, LocationType as T};
    let card = |m: micold_core::overview::CardModel| {
        // An inert press, so the ripple shows without closing anything (feature 971, FR-009).
        iced::widget::container(Element::from(
            material::OverviewCard::<Message>::new(&m, roles).on_press(Message::NoOp),
        ))
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
    // The card whose context menu is open (feature 972, FR-006): outline and selected layer.
    let targeted = iced::widget::container(Element::from(
        material::OverviewCard::<Message>::new(
            &card_model("targeted", T::Feature, S::Working, R::Working),
            roles,
        )
        .on_press(Message::NoOp)
        .targeted(true),
    ))
    .width(Length::Fixed(320.0));
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
    // Agent markup as an orchestrator sends it: the card shows only "Continue M2" (feature 1120).
    with_activity.activity = Some(micold_core::hook_activity::CurrentActivity {
        line: r#"<agent-message from="orchestrator">Continue M2</agent-message>"#.to_string(),
        ratio: Some(micold_core::task_progress::Ratio { done: 2, total: 5 }),
    });
    with_activity.progress = with_progress.progress;
    // The five states in one grid (feature 1120, T035): Needs action carries an activity and
    // progress, so its row is taller than its sparse neighbours need and they stretch to it.
    let mut needs_action = card_model(
        "needs-action",
        T::Feature,
        S::NeedsAction,
        R::NeedsPermission,
    );
    needs_action.activity = with_activity.activity.clone();
    needs_action.progress = with_progress.progress;
    // Titles needing about 1, 2, 3 and 5 lines, the last cut on its third line, and a short and a
    // long activity, the long one cut on its second (feature 1121, US1), in a grid as the Overview
    // lays them out: a full-width one, where a row takes its tallest card's height (1120 FR-014),
    // and one column each at the card widths 280, 380 and 480 px (FR-007).
    let long_activity = "Rewriting the card grid so a row of cards keeps the height of its \
        tallest card while the title wraps, then running the layout snapshot";
    let wrapped_models: Vec<_> = [
        ("Fix tab bar", "Editing card_grid.rs"),
        (
            "Overview cards: full text without truncation",
            long_activity,
        ),
        (
            "Overview cards: full text without truncation, expanded in place on hover",
            "Editing card_grid.rs",
        ),
        (
            "Overview cards: the title wraps onto three lines and the activity onto two, \
             and hover or keyboard focus expands the card in place over its neighbours",
            long_activity,
        ),
    ]
    .into_iter()
    .map(|(title, activity)| {
        let mut m = card_model("wrapped", T::Feature, S::Working, R::Working);
        m.title = title.to_string();
        m.activity = Some(micold_core::hook_activity::CurrentActivity {
            line: activity.to_string(),
            ratio: None,
        });
        m
    })
    .collect();
    let wrapped_grid = |width: Length| {
        iced::widget::container(Element::from(
            material::CardGrid::new(
                wrapped_models
                    .iter()
                    .map(|m| {
                        material::OverviewCard::<Message>::new(m, roles).on_press(Message::NoOp)
                    })
                    .collect(),
                roles,
            )
            .unscrolled(),
        ))
        .width(width)
    };
    let wrapped = iced::widget::column![
        wrapped_grid(Length::Fill),
        iced::widget::row([280.0, 380.0, 480.0].map(|w| wrapped_grid(Length::Fixed(w)).into()))
            .spacing(spacing::MD)
            .wrap(),
    ]
    .spacing(spacing::MD);
    // The first card expanded over its neighbours, with every text longer than a card (feature
    // 1121, FR-015, quickstart B.2); its row keeps the height its collapsed twin gives it (FR-009).
    let mut long = card_model("expanded-display-name", T::Feature, S::Working, R::Working);
    long.title = "Overview cards: full text without truncation, the whole title shown once the \
        card expands in place over its neighbours"
        .to_string();
    long.branch = Some("feat/1121_overview-card-full-text-expanded-in-place-on-hover".to_string());
    long.dir = std::path::PathBuf::from(
        "/home/dev/code/micold-ai-ide/.worktrees/1121-overview-card-full-text-expanded",
    );
    long.activity = Some(micold_core::hook_activity::CurrentActivity {
        line: long_activity.to_string(),
        ratio: None,
    });
    long.progress = with_progress.progress;
    let expanded = iced::widget::container(Element::from(
        material::CardGrid::new(
            std::iter::once(
                material::OverviewCard::<Message>::new(&long, roles)
                    .on_press(Message::NoOp)
                    .expanded(true),
            )
            .chain(
                ["one", "two", "three", "four", "five"]
                    .map(|name| card_model(name, T::Chore, S::Idle, R::Idle))
                    .iter()
                    .map(|m| {
                        material::OverviewCard::<Message>::new(m, roles).on_press(Message::NoOp)
                    }),
            )
            .collect(),
            roles,
        )
        .expanded(0, material::OverviewCard::<Message>::new(&long, roles))
        .unscrolled(),
    ))
    .width(Length::Fill);
    // Feature 1122 (FR-003, FR-015, quickstart §B1): "Issue #1122" leading the meta line
    // (BUG-1298 FR-023), collapsed at the three card widths with a short branch, a 120-character
    // one and none (the title is the branch), and on an expanded card over its neighbours.
    let issue = |mut m: micold_core::overview::CardModel| {
        m.issue_link = Some(micold_core::issue_link::IssueLink {
            label: "Issue #1122".to_string(),
            address: "https://github.com/o/r/issues/1122".to_string(),
        });
        m
    };
    let linked = |m: &micold_core::overview::CardModel, id: String| {
        material::OverviewCard::<Message>::new(m, roles)
            .on_press(Message::NoOp)
            .on_issue_link(Message::NoOp)
            .link_id(iced::widget::Id::from(id))
    };
    let mut short = issue(card_model(
        "feat-1122_overview-issue-link",
        T::Feature,
        S::Working,
        R::Working,
    ));
    short.branch = Some("feat/1122_overview-issue-link".to_string());
    let mut long_branch = issue(card_model("long-branch", T::Other, S::Idle, R::Idle));
    long_branch.branch = Some(format!(
        "topic-overview-card-issue-link-{}",
        "with-a-branch-name-far-longer-than-any-card-".repeat(2)
    ));
    let mut by_branch = issue(card_model("feat-1122_x", T::Feature, S::Done, R::Merged));
    by_branch.branch = Some("feat/1122_x".to_string());
    by_branch.title = "feat/1122_x".to_string();
    let link_models = [short, long_branch, by_branch];
    let link_column = |width: f32| {
        iced::widget::container(
            iced::widget::column(
                link_models
                    .iter()
                    .enumerate()
                    .map(|(i, m)| Element::from(linked(m, format!("showcase-link-{width}-{i}")))),
            )
            .spacing(spacing::MD),
        )
        .width(Length::Fixed(width))
        .into()
    };
    let issue_links = iced::widget::row([280.0, 380.0, 480.0].map(link_column))
        .spacing(spacing::MD)
        .wrap();
    let mut long_linked = issue(long.clone());
    long_linked.branch = Some(format!(
        "feat/1122_{}",
        "overview-card-issue-link-".repeat(4)
    ));
    let issue_expanded = iced::widget::container(Element::from(
        material::CardGrid::new(
            std::iter::once(
                linked(&long_linked, "showcase-link-expanded".to_string()).expanded(true),
            )
            .chain(
                ["one", "two", "three"]
                    .map(|name| issue(card_model(name, T::Chore, S::Idle, R::Idle)))
                    .iter()
                    .enumerate()
                    .map(|(i, m)| linked(m, format!("showcase-link-beneath-{i}"))),
            )
            .collect(),
            roles,
        )
        .expanded(
            0,
            material::OverviewCard::<Message>::new(&long_linked, roles),
        )
        .unscrolled(),
    ))
    .width(Length::Fill);
    let pr_rows = pull_request_rows_pose(roles);
    let mut done = card_model("done", T::Other, S::Done, R::Merged);
    done.prs = micold_core::overview::PrDisplay::Hidden;
    // Unscrolled: the showcase page scrolls, and a grid with its own scrollable fills a height
    // the page does not have, so it drew nothing (feature 1120, M4 visual re-look).
    let five = iced::widget::container(
        material::CardGrid::new(
            [
                needs_action,
                card_model(
                    "needs-attention",
                    T::Bugfix,
                    S::NeedsAttention,
                    R::ChecksFailing,
                ),
                card_model("working", T::Chore, S::Working, R::Working),
                done,
                card_model("idle", T::Included, S::Idle, R::Idle),
            ]
            .iter()
            .map(|m| material::OverviewCard::<Message>::new(m, roles).on_press(Message::NoOp))
            .collect(),
            roles,
        )
        .unscrolled(),
    )
    .width(Length::Fill);
    // Feature 1299 (FR-013, quickstart §B): the first card expands and collapses at a press, its
    // shadow easing in and out over its neighbours; Tab onto a card fades its focus in. Eased:
    // the showcase grids keep the default.
    let lifted = showcase.shown(index);
    let lift = iced::widget::column![
        material::Button::text(if lifted { "Collapse" } else { "Expand" }, roles)
            .on_press(Message::Reversed(index)),
        iced::widget::container(Element::from({
            let models = ["lifted", "beside", "below"]
                .map(|name| card_model(name, T::Feature, S::Working, R::Working));
            let cards = models
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    material::OverviewCard::<Message>::new(m, roles)
                        .on_press(Message::NoOp)
                        .expanded(lifted && i == 0)
                })
                .collect();
            let grid = material::CardGrid::new(cards, roles).unscrolled();
            if lifted {
                grid.expanded(0, material::OverviewCard::<Message>::new(&models[0], roles))
            } else {
                grid
            }
        }))
        .width(Length::Fill),
    ]
    .spacing(spacing::MD);
    arrange(
        vec![
            posed("with progress", card(with_progress), roles),
            posed("with activity", card(with_activity), roles),
            posed(
                "without progress",
                card(card_model("without-progress", T::Feature, S::Idle, R::Idle)),
                roles,
            ),
            posed("five states", five, roles),
            posed("Default", card(default), roles),
            posed("orphan", card(orphan), roles),
            posed("unknown counts", card(unknown), roles),
            posed("targeted", targeted, roles),
            posed("wrapped titles", wrapped, roles),
            posed("expanded", expanded, roles),
            posed("issue link", issue_links, roles),
            posed("issue link, expanded", issue_expanded, roles),
            posed("pull request rows", pr_rows, roles),
            posed("expand and collapse — press it", lift, roles),
        ],
        Layout::FullWidth,
    )
}

/// Feature 1123 (FR-015, quickstart §B1, §B2): a card with four pull requests (open with *checks
/// failing*, draft with *checks pending*, open with *checks passing*, merged) collapsed with
/// "+2 more" and expanded, and one with a long number and *changes requested* at the card widths
/// 280, 380 and 480 px. Between them the rows pose each [`material::PrTone`] (BUG-1298 FR-026).
fn pull_request_rows_pose<'a>(roles: Roles) -> Element<'a, Message> {
    use micold_core::overview::{CardReason as R, CardState as S, LocationType as T};
    use micold_core::pr_list::{BranchPrList, ListedPr};
    use micold_core::pull_request::{BranchPrState as P, CheckStatus as C, ReviewState};
    let listed = |number: u64, state: P, checks: C, review: ReviewState, at: &str| ListedPr {
        number,
        url: format!("https://github.com/o/r/pull/{number}"),
        state,
        checks,
        review,
        conflict: false,
        created_at: at.to_string(),
    };
    let with = |name: &str, prs: Vec<ListedPr>| {
        let mut m = card_model(name, T::Feature, S::NeedsAttention, R::ChecksFailing);
        let list = BranchPrList::new(prs, false, false);
        m.prs = micold_core::overview::PrDisplay::Counts(list.counts());
        m.pull_requests = Some(list);
        m
    };
    let three = with(
        "feat-1123_x",
        vec![
            listed(
                12,
                P::Open,
                C::Failing,
                ReviewState::None,
                "2026-10-03T00:00:00Z",
            ),
            listed(
                11,
                P::Draft,
                C::Pending,
                ReviewState::None,
                "2026-10-02T00:00:00Z",
            ),
            listed(
                10,
                P::Open,
                C::Passing,
                ReviewState::None,
                "2026-10-01T12:00:00Z",
            ),
            listed(
                9,
                P::Merged,
                C::None,
                ReviewState::None,
                "2026-10-01T00:00:00Z",
            ),
        ],
    );
    let drawn = |m: &micold_core::overview::CardModel, expanded: bool| {
        let (rows, more) = crate::ui::overview::pr_rows(m, expanded);
        material::OverviewCard::<Message>::new(m, roles)
            .on_press(Message::NoOp)
            .pull_requests(rows, more)
            .expanded(expanded)
    };
    let fixed = |card: material::OverviewCard<'a, Message>, width: f32| -> Element<'a, Message> {
        iced::widget::container(Element::from(card))
            .width(Length::Fixed(width))
            .into()
    };
    let long = with(
        "feat-1123_long",
        vec![listed(
            1_123_456,
            P::Open,
            C::None,
            ReviewState::ChangesRequested,
            "2026-10-03T00:00:00Z",
        )],
    );
    iced::widget::column![
        iced::widget::row![
            fixed(drawn(&three, false), 320.0),
            fixed(drawn(&three, true), 320.0)
        ]
        .spacing(spacing::MD),
        iced::widget::row([280.0, 380.0, 480.0].map(|w| fixed(drawn(&long, false), w)))
            .spacing(spacing::MD)
            .wrap(),
    ]
    .spacing(spacing::MD)
    .into()
}

/// The width the zoom pose's drawer gives the grid when it retracts.
const ZOOM_DRAWER_WIDTH: f32 = 200.0;

/// `CardGrid` with 1, 3 and 30 cards, in a narrow room, and zooming with targeted cards.
pub fn card_grid<'a>(showcase: &'a Showcase, roles: Roles, index: usize) -> Element<'a, Message> {
    use micold_core::overview::{CardReason as R, CardState as S, LocationType as T};
    let zoomed = |n: usize, width: Length, zoom: material::Zoom| {
        let cards = (0..n)
            .map(|i| {
                let state = S::ALL[i % S::ALL.len()];
                // The zoom pose targets every third card, so a mid-flight frame shows a
                // targeted card split across two rows (1120: its mark stays inside each piece).
                material::OverviewCard::<Message>::new(
                    &card_model(&format!("worktree-{i}"), T::Feature, state, R::Working),
                    roles,
                )
                .targeted(zoom.freed > 0.0 && i % 3 == 1)
            })
            .collect();
        iced::widget::container(Element::from(
            material::CardGrid::new(cards, roles).zoom(zoom),
        ))
        .width(width)
        .height(Length::Fixed(320.0))
    };
    let grid = |n: usize, width: Length| zoomed(n, width, material::Zoom::default());
    arrange(
        vec![
            posed("1 card", grid(1, Length::Fill), roles),
            posed("3 cards", grid(3, Length::Fill), roles),
            posed("30 cards", grid(30, Length::Fill), roles),
            posed("narrow", grid(3, Length::Fixed(279.0)), roles),
            // Feature 970: the cards widen and slide with the drawer beside them.
            posed(
                "zoom — press it",
                iced::widget::column![
                    material::Button::text(
                        if showcase.shown(index) {
                            "Full screen"
                        } else {
                            "Exit full screen"
                        },
                        roles,
                    )
                    .on_press(Message::Reversed(index)),
                    iced::widget::row![
                        material::NavigationDrawer::new(
                            material::Surface::new(
                                material::Text::new("The sidebar", TypeRole::Body, roles),
                                SurfaceKind::Sidebar,
                                roles,
                            )
                            .padding(spacing::MD)
                            .width(Length::Fixed(ZOOM_DRAWER_WIDTH))
                            .height(Length::Fill),
                            material::Surface::new(
                                material::Glyph::<Message>::new(
                                    Icon::ShowSidebar,
                                    TypeRole::Title,
                                    roles,
                                ),
                                SurfaceKind::Sidebar,
                                roles,
                            )
                            .padding(spacing::SM)
                            .height(Length::Fill),
                        )
                        .open(true)
                        .retracted(!showcase.shown(index)),
                        zoomed(
                            12,
                            Length::Fill,
                            material::Zoom {
                                full_screen: !showcase.shown(index),
                                freed: ZOOM_DRAWER_WIDTH,
                            },
                        ),
                    ]
                    .height(Length::Fixed(320.0)),
                ]
                .spacing(spacing::XS),
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
    // A sidebar state group's header (feature 1125, FR-021, R8): open over its rows, and closed.
    let group_header = |open: bool| {
        let mut items = vec![material::TreeItem::new(
            0,
            micold_core::sidebar_groups::header_text(
                micold_core::sidebar_groups::StateGroup::Working,
                2,
            ),
            roles.on_surface_variant,
        )
        .heading()
        .expandable(open, Message::NoOp)];
        if open {
            for (name, _) in samples::WORKTREES.iter().take(2) {
                items.push(
                    material::TreeItem::new(0, *name, roles.on_surface).on_press(Message::NoOp),
                );
            }
        }
        material::TreeView::new(items, roles)
            .density(micold_core::tokens::density::DENSE)
            .label_role(TypeRole::SidebarName)
    };
    arrange(
        vec![
            posed("group header, open", group_header(true), roles),
            posed("group header, closed", group_header(false), roles),
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
                "Quick-switcher dialog",
                quick_switcher_pose(roles, false),
                roles,
            ),
            posed(
                "Quick-switcher dialog, nothing matches",
                quick_switcher_pose(roles, true),
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
pub fn navigation_drawer<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
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
            // Feature 970: retraction hides the whole drawer, rail included, and gives its width
            // to what stands beside it.
            posed(
                "retracted (zero width; the text beside it starts at the edge)",
                iced::widget::row![
                    material::NavigationDrawer::new(panel(roles), rail(roles))
                        .open(true)
                        .retracted(true),
                    material::Text::new("Beside the drawer", TypeRole::Body, roles),
                ]
                .spacing(spacing::SM),
                roles,
            ),
            posed(
                "retracting — press it",
                iced::widget::column![
                    material::Button::text(
                        if showcase.shown(index) {
                            "Retract the drawer"
                        } else {
                            "Show the drawer"
                        },
                        roles,
                    )
                    .on_press(Message::Reversed(index)),
                    iced::widget::row![
                        material::NavigationDrawer::new(panel(roles), rail(roles))
                            .open(true)
                            .retracted(!showcase.shown(index)),
                        material::Text::new("Beside the drawer", TypeRole::Body, roles),
                    ]
                    .spacing(spacing::SM),
                ]
                .spacing(spacing::XS),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::test_support::{laid_out, roles};
    use iced::advanced::Layout as Laid;
    use iced::{Rectangle, Size};

    /// The "five states" pose (feature 1120, T035) in the showcase's scrolling page: at a narrow
    /// and a wide window its grid is at least 100 px tall (a card is at least 172), and the pose
    /// after it starts below it.
    /// A grid that fills its room collapses to nothing in a page that scrolls.
    #[test]
    fn the_five_state_grid_is_drawn_and_the_next_pose_starts_below_it() {
        let showcase = Showcase::new(1);
        for width in [600.0, 1600.0] {
            let page = material::Scrollable::new(overview_card(&showcase, roles(), 0), roles());
            let node = laid_out(page.into(), Size::new(width, 800.0));
            let (five, grid, next) = five_state_pose(&node);
            assert!(
                grid.height >= 100.0,
                "at {width} px the five-state grid is {} px tall",
                grid.height
            );
            assert!(
                next.y >= five.y + five.height,
                "at {width} px the pose after the five-state grid starts at {} inside it {five:?}",
                next.y,
            );
        }
    }

    /// The "five states" pose in the laid-out page: the pose, its grid, and the pose after it.
    fn five_state_pose(node: &iced::advanced::layout::Node) -> (Rectangle, Rectangle, Rectangle) {
        let section = Laid::new(node)
            .children()
            .next()
            .expect("the page's content");
        let pose = |i: usize| {
            let row = section.children().nth(i).expect("a pose's row");
            row.children().next().expect("the pose")
        };
        let five = pose(3);
        let grid = five
            .children()
            .next()
            .and_then(|column| column.children().next())
            .expect("the pose's instance")
            .bounds();
        (five.bounds(), grid, pose(4).bounds())
    }

    /// T041 (1120): the "five states" grid paints each state's word exactly once: one card per
    /// state, none missing, none twice.
    #[test]
    fn the_five_state_grid_paints_each_state_once() {
        use crate::ui::material::test_support::Mounted;
        let showcase = Showcase::new(1);
        let page = material::Scrollable::new(overview_card(&showcase, roles(), 0), roles());
        let mut m = Mounted::new_in(
            Element::from(page).map(|_| String::new()),
            Size::new(1600.0, 4000.0),
        );
        let (_, grid, _) = five_state_pose(&m.node);
        let painted = m.painted_text();
        for state in micold_core::overview::CardState::ALL {
            let n = painted
                .iter()
                .filter(|p| p.content == state.label() && grid.contains(p.bounds.center()))
                .count();
            assert_eq!(
                n,
                1,
                "the five-state grid paints \"{}\" {n} times",
                state.label()
            );
        }
    }
}

// --- Feature 940: the transcript's rich components -------------------------------------------

/// The markdown subset fixture the core's markdown model is tested on: every element the view
/// draws, and the literal fallbacks (feature 940, research R13).
static MARKDOWN_SUBSET: std::sync::LazyLock<micold_core::markdown::Document> =
    std::sync::LazyLock::new(|| {
        micold_core::markdown::Document::parse(
            include_str!("../../../../micold-core/tests/fixtures/stream-json/markdown-subset.md"),
            false,
        )
    });

/// A short reply for the muted pose: what a thinking card holds.
static THINKING: std::sync::LazyLock<micold_core::markdown::Document> = std::sync::LazyLock::new(
    || {
        micold_core::markdown::Document::parse(
            "The user wants **two** things:\n\n1. a `Write` of the new file\n2. an edit to the old one",
            false,
        )
    },
);

/// The subset's code blocks highlighted, light and dark, as a highlight job would answer them.
static MARKDOWN_SPANS: std::sync::LazyLock<
    std::collections::HashMap<micold_core::markdown::CodeId, micold_core::review::diff::Spans>,
> = std::sync::LazyLock::new(|| {
    MARKDOWN_SUBSET
        .code_blocks()
        .filter_map(|(id, code, _)| {
            let token = code
                .token
                .as_deref()
                .filter(|t| crate::ui::syntax::knows(t))?;
            let lines: Vec<String> = code.code.lines().map(str::to_owned).collect();
            let spans =
                crate::ui::syntax::highlight_job(token, &lines, std::time::Duration::from_secs(5))?;
            Some((id, spans))
        })
        .collect()
});

/// `MarkdownView` — the markdown subset, and a muted reply as a thinking card draws it (feature
/// 940, research R9).
pub fn markdown_view<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let subset = material::MarkdownView::new(&MARKDOWN_SUBSET, roles)
        .on_link(|_| Message::NoOp)
        .on_copy(|_| Message::NoOp)
        .code_spans(|id| MARKDOWN_SPANS.get(&id))
        .on_show(|_| Message::NoOp);
    let muted = material::MarkdownView::new(&THINKING, roles).muted(true);
    arrange(
        vec![
            posed("the markdown subset", subset, roles),
            posed("muted, as a thinking card", muted, roles),
        ],
        Layout::FullWidth,
    )
}

/// How a posed `CodeBlock` stands (feature 940). `pub` so `tests/gates/code_block_fits.rs` and
/// `code_block_wheel.rs` lay out exactly what the gallery shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodePose {
    /// Nothing copied yet.
    Idle,
    /// The read-back held the code.
    Copied,
    /// The clipboard did not take it.
    Failed,
    /// Over the size limits, waiting for "Show in full".
    Large,
}

/// The sample code: Rust, with one line long enough to scroll sideways in a narrow transcript.
pub const CODE_SAMPLE: &str = "fn main() {\n    let greeting = \"a line long enough to run past a narrow transcript, so it scrolls sideways inside its block\";\n    println!(\"{greeting}\");\n}";

/// [`CODE_SAMPLE`]'s colours, light and dark.
static CODE_SPANS: std::sync::LazyLock<[micold_core::review::diff::SchemeSpans; 2]> =
    std::sync::LazyLock::new(|| {
        [
            micold_core::theme::ColorScheme::Light,
            micold_core::theme::ColorScheme::Dark,
        ]
        .map(|scheme| micold_core::review::diff::SchemeSpans {
            old: Default::default(),
            new: crate::ui::syntax::highlight_lines("rust", CODE_SAMPLE.lines(), scheme),
        })
    });

/// One posed code block of [`CODE_SAMPLE`], tagged `rust`, its copy control sending `on_copy`.
pub fn code_block_sample(
    pose: CodePose,
    on_copy: Message,
    roles: Roles,
) -> Element<'static, Message> {
    use crate::features::transcript_rich::{CopyState, Ticket};
    let spans = &CODE_SPANS[usize::from(roles.scheme() == micold_core::theme::ColorScheme::Dark)];
    let state = match pose {
        CodePose::Copied => Some(CopyState::Copied(Ticket::posed(1))),
        CodePose::Failed => Some(CopyState::Failed(Ticket::posed(1))),
        CodePose::Idle | CodePose::Large => None,
    };
    let block = material::CodeBlock::new(CODE_SAMPLE, roles)
        .language("rust")
        .spans(spans)
        .copy(state, on_copy);
    match pose {
        CodePose::Large => block.large(6_000, Message::NoOp).into(),
        _ => block.into(),
    }
}

/// `CodeBlock` — idle, copied, failed and large, the first with a line wider than the block
/// (feature 940, research R5, R9).
pub fn code_block<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let pose =
        |label: &'a str, pose| posed(label, code_block_sample(pose, Message::NoOp, roles), roles);
    arrange(
        vec![
            pose("idle, a line wider than the block", CodePose::Idle),
            pose("copied", CodePose::Copied),
            pose("copy failed", CodePose::Failed),
            pose("large", CodePose::Large),
        ],
        Layout::FullWidth,
    )
}

/// A paragraph with a web link, a mail link and a relative one that is shown but not opened.
static LINKS: std::sync::LazyLock<micold_core::markdown::Inlines> =
    std::sync::LazyLock::new(|| {
        let doc = micold_core::markdown::Document::parse(
        "Open [the docs](https://example.com/docs), write to [the team](mailto:team@example.com), \
         or read [the notes](docs/notes.md): a relative link, shown but not opened. Some \
         **bold**, *italic*, ~~struck~~ and `inline code`.",
        false,
    );
        match doc.blocks().first() {
            Some(micold_core::markdown::Block::Paragraph { text }) => text.clone(),
            _ => Vec::new(),
        }
    });

/// `LinkText` — web, mail and a relative link among formatted text (feature 940, FR-004).
pub fn link_text<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![posed(
            "web, mail and a relative link",
            material::LinkText::new(&LINKS, roles).on_link(|_| Message::NoOp),
            roles,
        )],
        Layout::FullWidth,
    )
}

/// An Edit card, folded or open: its header's `+12 −3` meta beside the summary and the status
/// (feature 940, FR-017). `pub` so `tests/gates/disclosure_card_header_fits.rs` lays it out.
pub fn edit_card<'a>(open: bool, on_toggle: Message, roles: Roles) -> Element<'a, Message> {
    material::DisclosureCard::new("Edit", on_toggle, roles)
        .leading(Icon::ToolCall)
        .summary("crates/micold-client/src/ui/material/disclosure_card.rs")
        .meta(EDIT_META)
        .trailing(material::IconLabel::new(Icon::ToolDone, "Done", TypeRole::Label, roles).muted())
        .open(open)
        .body(material::Text::new(samples::TOOL_INPUT, TypeRole::Body, roles).monospace())
        .into()
}

/// The posed Edit card's line counts.
pub const EDIT_META: &str = "+12 \u{2212}3";
