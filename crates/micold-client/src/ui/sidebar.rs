//! The left navigation sidebar: worktrees (top level) → sessions (sub-items), built from the
//! shared [`tree_view`] primitive (FR-002, FR-003, Constitution Principle VIII).

use crate::app::{Message, State};
use crate::features::session::Msg as SessionMsg;
use crate::features::session::{PressTarget, StartIntent};
use crate::features::sidebar::Msg as SidebarMsg;
use crate::features::sidebar::TagFilter;
use crate::features::sidebar::{hover_holds, row_selected, RowRef, RowSelection, SelectableRow};
use crate::features::worktree::Msg as WorktreeMsg;
use crate::features::worktree_form::Msg as FormMsg;
use crate::icons::Icon;
use crate::ui::cdk;
use crate::ui::material::{
    self, ActivityBadge, Button, ButtonVariant, CheckMark, Divider, HoverReveal, IconButton,
    PrMark, PullRequestIndicator, Scrollable, SplitAction, StateIcon, SurfaceKind, Text,
    ToggleChip, Tooltip, TreeItem, TreeView, TypeRole,
};
use iced::widget::{column, container, row};
use iced::{Alignment, Element, Length};
use micold_core::naming::Tag;
use micold_core::pull_request::{CheckStatus, PrState};
use micold_core::session::{Session, SessionLifecycle, SessionLocation};
use micold_core::tokens::{self, spacing, Rgb, Roles};
use micold_core::worktree::WorktreeStatus;

/// The sidebar list's scroll viewport, by name (feature 024, FR-008).
///
/// A `LazyLock` because `scrollable::Id` is not a `const` and the id has to be the *same* one on
/// every frame — a fresh id each render would name a viewport the previous frame's scroll operation
/// was addressed to, and the scroll would land nowhere.
pub static SIDEBAR_SCROLL_ID: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("sidebar-list"));

/// The deepest indent a nested session row takes; deeper rows keep their order but not their indent,
/// so the child marker at the end of the row stays on screen (feature 551).
const MAX_NESTING: usize = 4;

/// Width of the collapsed strip that hosts the "show sidebar" button.
pub(crate) const STRIP_WIDTH: f32 = 32.0;

/// Render the sidebar for the active project's worktrees and sessions, at the current
/// (adjustable) width.
pub fn view<'a>(state: &'a State, scheme: micold_core::theme::ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let width = state.sidebar_width_px() as f32;

    // Feature 1125 (S3, R11): the chip bar under the Overview row took the place of the header row
    // (the "Worktrees" title, the filter button and five actions) and of the filter accordion. Only
    // with a project open, as the Overview row (S1).
    let chip_bar = state.workspace.active.as_ref().map(|_| chip_bar(state, r));

    // The "Default" entry (feature 010) is always present once a project is open — see
    // `sidebar_entries()` — so, unlike before this feature, the sidebar is never truly "empty":
    // a zero-worktree or filtered-to-nothing project still shows the Default row and its
    // start-session action, with a muted hint appended below about worktrees specifically.
    //
    // The groups hold every entry once, the Default entry among them, so "one entry in all" is
    // "no worktree entries matched". Built once per render (feature 1125): they give both the hint
    // and the list.
    let groups = state.sidebar_groups();
    let no_worktree_entries = groups.iter().map(|g| g.entries.len()).sum::<usize>() <= 1;
    // A project with no discovered worktree takes the "none yet" branch — the "no match / clear
    // filters" branch would offer to clear a filter that was never applied.
    let hint: Option<Element<'_, Message>> = if state.worktree.worktrees.is_empty() {
        Some(
            Text::new(
                "No worktrees yet. Add one to get started.",
                TypeRole::Caption,
                r,
            )
            .muted()
            .into(),
        )
    } else if no_worktree_entries {
        // Active filter matched nothing (FR-027): a message + a one-tap clear.
        Some(
            column![
                Text::new("No worktrees match the filter.", TypeRole::Caption, r).muted(),
                Button::with_content(
                    Text::new("Clear filters", TypeRole::SidebarTag, r),
                    ButtonVariant::Text,
                    r
                )
                .padding(spacing::XS)
                .on_press(Message::Sidebar(SidebarMsg::FiltersCleared)),
            ]
            .spacing(spacing::XS)
            .into(),
        )
    } else {
        None
    };

    let tree: Element<'_, Message> = TreeView::new(list_items(state, groups, r), r)
        // The sidebar is the one list at `dense` (§7.2, FR-026a): 36dp rows rather than 48dp, so
        // the worktree count visible without scrolling does not drop. A named step on the shared
        // density scale, not a bespoke shrink (FR-026c).
        //
        // Read from `features::sidebar` rather than named here, because the row metrics that decide
        // where to scroll are computed from the same constant (feature 024). Two copies of it would
        // agree until one of them changed, and the disagreement would be a scroll landing slightly
        // off — with nothing on screen to say why.
        .density(crate::features::sidebar::SIDEBAR_DENSITY)
        .label_role(TypeRole::SidebarName)
        // The current session's row is marked by the selected fill *and* by a heavier name
        // (feature 024, FR-003a). Two channels rather than one, because the fill is a colour and
        // the row beside it can be hovered — a state layer is also a fill change, and the two must
        // stay tellable apart without relying on hue.
        .selected_label_role(TypeRole::SidebarSessionCurrent)
        // The row the user is on is filled with the primary container (feature 1125, FR-010).
        .selection(material::Selection::Primary)
        .into();
    let list: Element<'_, Message> = match hint {
        Some(hint) => column![tree, hint].spacing(spacing::SM).into(),
        None => tree,
    };
    // Scroll the list when it exceeds the sidebar height, with a thin themed scrollbar.
    // The list gets a little right padding so rows never sit under the scrollbar.
    let body: Element<'_, Message> = Scrollable::new(
        container(list).padding(iced::Padding {
            top: 0.0,
            right: spacing::SM,
            bottom: 0.0,
            left: 0.0,
        }),
        r,
    )
    .height(Length::Fill)
    // Scrolling the list is the third dismissal trigger (feature 017, FR-009): a menu opened from
    // a row is stale once the rows have moved. Reported unconditionally — whether anything closes
    // is the reducer's decision, taken through the shared rule.
    // Reports the offset rather than a bare notification: the app bar's elevation derives from it
    // (FR-025a), and the sidebar is the only scroll region beneath the bar. The reducer runs the
    // dismissal from this message too, so the third dismissal trigger (feature 017, FR-009) is
    // unchanged — a scrollable gets one subscription, not two.
    .on_scroll_offset(|offset| Message::Sidebar(SidebarMsg::Scrolled(offset)))
    // Feature 024: the reveal has to know whether its row is inside the viewport, and iced reports
    // no child position — so the geometry is computed, and this is the one input that is not
    // already in state. Reported from a sensor rather than from `on_scroll`, which fires only when
    // something scrolls; the frame that matters is the first one after a switch, where nothing has.
    .on_viewport_resize(|size| {
        Message::Sidebar(SidebarMsg::ViewportResized(crate::app::scroll_offset_px(
            size.height,
        )))
    })
    // Addressable so `operation::scroll_to` can reach it. On the scrollable itself, never on the
    // sensor wrapping it — a wrapper that does not forward `operate` swallows scroll operations for
    // its whole subtree (`ui/material/ripple.rs`).
    .id(SIDEBAR_SCROLL_ID.clone())
    .into();

    // Minimal left/right padding to maximize name/tag width (FR-009); a little vertical breathing
    // room is kept.
    // Feature 582 (FR-012): the start-up offer, between the header and the list. The shared banner,
    // not a snackbar: the offer stands until the user answers it.
    // Feature 1125 (S1, R9): the Overview row first, above the chip bar and outside the scroll
    // region, so it stays the first row however far the list is scrolled.
    let mut content = column![];
    if let Some(overview) = overview_row(state, r) {
        content = content.push(overview);
    }
    if let Some(chip_bar) = chip_bar {
        content = content.push(chip_bar);
    }
    if let Some(offer) = offer_banner(state, r) {
        content = content.push(offer);
    }
    let content = content
        .push(body)
        .spacing(spacing::SM)
        .padding(iced::Padding {
            top: spacing::SM,
            bottom: spacing::SM,
            left: spacing::XS,
            right: spacing::XS,
        })
        .width(Length::Fixed(width))
        .height(Length::Fill);

    material::Surface::new(content, SurfaceKind::Sidebar, r)
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .into()
}

/// What the rows' selected look is read from (feature 1125, FR-009).
fn row_selection(state: &State) -> RowSelection<'_> {
    RowSelection {
        overview_open: state.overview.open,
        viewed: state.session.active,
        revealed: state.sidebar.revealed.as_ref(),
    }
}

/// The sessions a location row may be selected for: none while the row has no main session (every
/// one archived), so viewing an archived session never selects the row, as before feature 1125.
fn pressable_sessions<T>(main: Option<T>, sessions: &[Session]) -> &[Session] {
    if main.is_some() {
        sessions
    } else {
        &[]
    }
}

/// The Overview row (feature 1125, S2, R9), or `None` with no project open: selected exactly while
/// the Overview is open, pressable only while Settings is closed, and carrying the top bar button's
/// badge and tooltip, read from the same cards.
fn overview_row(state: &State, r: Roles) -> Option<Element<'static, Message>> {
    use crate::features::overview::{self, Msg};
    state.workspace.active.as_ref()?;
    let settings_open = state.settings.settings_draft.is_some();
    Some(overview_row_view(
        overview::badge(&crate::ui::overview::cards(state)),
        state.overview.open,
        (!settings_open).then_some(Message::Overview(Msg::Opened { settings_open })),
        Message::NoOp,
        r,
    ))
}

/// The Overview row as drawn (feature 1125, R9; the showcase poses it too): "Overview" with its
/// icon in a one-item tree with the sidebar's density and primary selection, a Tab stop while it
/// has `on_press`, the badge's words in a solid chip of its tone's container pair, and the
/// Overview's tooltip. `idle` is what a change of the row's keyboard focus sends.
pub fn overview_row_view<'a, M: Clone + 'a>(
    badge: Option<crate::features::overview::Badge>,
    selected: bool,
    on_press: Option<M>,
    idle: M,
    r: Roles,
) -> Element<'a, M> {
    use crate::features::overview::{overview_tooltip, Tone};
    let mut item = TreeItem::new(0, "Overview", r.on_surface)
        .with_icon(Icon::Overview)
        .selected(selected)
        .row_tooltip(overview_tooltip(badge.as_ref()))
        .tab_stop()
        .keyboard_focus(false, move |_| idle.clone());
    if let Some(on_press) = on_press {
        item = item.on_press(on_press);
    }
    if let Some(badge) = badge {
        let (fill, on_fill) = match badge.tone {
            Tone::Error => (r.error_container, r.on_error_container),
            Tone::Warning => (r.warning_container, r.on_warning_container),
        };
        item = item.trailing_element(material::Tag::new(badge.label, fill).solid(on_fill));
    }
    TreeView::new(vec![item], r)
        .density(crate::features::sidebar::SIDEBAR_DENSITY)
        .label_role(TypeRole::SidebarName)
        .selected_label_role(TypeRole::SidebarSessionCurrent)
        .selection(material::Selection::Primary)
        .into()
}

/// The start-up offer's banner, when the active project is offered one (feature 582, FR-012;
/// 552 FR-009): it is shown when discovery found stored sessions the catalog lacks in worktrees
/// that have no session (BUG-1164). "Resume all" resumes the newest of each worktree; with none
/// resumable it opens the "Attach existing…" list.
fn offer_banner<'a>(
    state: &'a State,
    r: micold_core::tokens::Roles,
) -> Option<Element<'a, Message>> {
    use crate::features::attach::{offer_visible, Msg};
    if !offer_visible(state) {
        return None;
    }
    let offer = &state.attach.offer;
    let catalog = state.active_sessions();
    let resumable = offer.targets(catalog).len();
    let sessions = offer.session_count(catalog);
    let found = format!(
        "Found {sessions} stored {}",
        if sessions == 1 { "session" } else { "sessions" }
    );
    let banner = material::ConnectionBanner::new(found, "This app has no record of them yet.", r)
        .level(crate::features::notifications::NoticeLevel::Info);
    // Once "Resume all" is pressed the banner shows no buttons until the daemon answers, so the
    // same targets cannot be sent twice.
    if !offer.in_flight.is_empty() {
        return Some(banner.into());
    }
    let (label, press) = if resumable > 0 {
        ("Resume all", Msg::OfferAttachAll)
    } else {
        ("Review", Msg::Opened)
    };
    Some(
        banner
            .action(label, Message::Attach(press))
            .secondary_action("Dismiss", Message::Attach(Msg::OfferDismissed))
            .into(),
    )
}

/// The collapsed sidebar: a thin vertical strip hosting the "show sidebar" button (with a
/// tooltip), wide enough for the icon.
pub fn collapsed_strip(scheme: micold_core::theme::ColorScheme) -> Element<'static, Message> {
    let r = tokens::roles(scheme);
    let show = Tooltip::new(
        IconButton::new(Icon::ShowSidebar, r)
            .compact()
            .tint(r.on_surface_variant)
            .on_press(Message::Sidebar(SidebarMsg::Toggled)),
        "Show sidebar",
        r,
    );
    let content = material::Surface::new(
        column![show]
            .align_x(Alignment::Center)
            .padding(spacing::XS),
        SurfaceKind::Sidebar,
        r,
    )
    .width(Length::Fixed(STRIP_WIDTH - 1.0))
    .height(Length::Fill);

    // A subtle right border so the collapsed strip still reads as a bounded panel edge.
    let border: Element<'static, Message> = Divider::vertical(r).into();

    row![content, border].height(Length::Fill).into()
}

/// The human label for a tag filter chip.
fn filter_label(filter: TagFilter) -> String {
    match filter {
        TagFilter::Type(t) => t.as_str().to_string(),
        TagFilter::HasIssue => "issue".to_string(),
        TagFilter::Untyped => "untyped".to_string(),
    }
}

/// A toggle chip for one tag filter (feature 008, FR-024): filled in its tag color when active,
/// outlined when inactive. Pressing it toggles the filter.
fn filter_chip(filter: TagFilter, active: bool, r: Roles) -> Element<'static, Message> {
    // Feature 014: the pill styling that used to live here is now the shared `ToggleChip`
    // primitive, so this and the reveal chip below stay identical by construction rather than by
    // two copies drifting apart (Principle VIII Component-reuse gate).
    let (fill, on) = match filter {
        TagFilter::Type(t) => r.type_tag(t),
        TagFilter::HasIssue => r.issue_tag(),
        TagFilter::Untyped => material::chip_neutral_accent(r),
    };
    ToggleChip::new(
        filter_label(filter),
        Message::Sidebar(SidebarMsg::FilterToggled(filter)),
        r,
    )
    .active(active)
    .accent(fill, on)
    .into()
}

/// The chip bar (feature 1125, contract S3, research R11): **All** and the tag filter chips
/// (feature 008), wrapping onto further lines, left of add, hide and more (⋯), which stay on the
/// first line (US4.7, FR-016). Every control takes the keyboard, in display order (FR-017).
fn chip_bar(state: &State, r: Roles) -> Element<'static, Message> {
    use crate::features::sidebar::{chip_bar_chips, ChipBarChip};
    let chips: Vec<Element<'static, Message>> =
        chip_bar_chips(&state.available_tag_filters(), &state.sidebar.filters)
            .into_iter()
            .map(|(chip, active)| match chip {
                // Selected while no filter is; pressing it clears every filter (US4.2, US4.3).
                ChipBarChip::All => {
                    let clear = Message::Sidebar(SidebarMsg::FiltersCleared);
                    takes_the_keyboard(
                        ToggleChip::new("All", clear.clone(), r).active(active),
                        clear,
                        r,
                    )
                }
                ChipBarChip::Filter(filter) => takes_the_keyboard(
                    filter_chip(filter, active, r),
                    Message::Sidebar(SidebarMsg::FilterToggled(filter)),
                    r,
                ),
            })
            .collect();
    let chips = container(
        iced::widget::Row::with_children(chips)
            .spacing(spacing::XS)
            .wrap()
            .vertical_spacing(spacing::XS),
    )
    .width(Length::Fill);

    let add_message = Message::WorktreeForm(FormMsg::Opened);
    let add = Tooltip::new(
        takes_the_keyboard(
            IconButton::new(Icon::AddWorktree, r)
                .compact()
                .tint(r.primary)
                .on_press(add_message.clone()),
            add_message,
            r,
        ),
        "Add a worktree (new git branch)",
        r,
    );
    let hide_message = Message::Sidebar(SidebarMsg::Toggled);
    let hide = Tooltip::new(
        takes_the_keyboard(
            IconButton::new(Icon::HideSidebar, r)
                .compact()
                .tint(r.on_surface_variant)
                .on_press(hide_message.clone()),
            hide_message,
            r,
        ),
        "Hide sidebar",
        r,
    );
    // Refresh, attach existing and run in parallel (D4): the menu hangs from this trigger's
    // bottom-left corner, reported on the press ahead of the toggle (the switcher's BUG-1023).
    let more_message = Message::Sidebar(SidebarMsg::MoreMenuToggled);
    let more = cdk::context_area::ContextArea::new(Tooltip::new(
        takes_the_keyboard(
            // `MenuTrigger`'s button at the chip bar's compact size, beside add and hide: the
            // trigger is a plain icon button, and its 48dp target would make the first line taller
            // than the chips it shares.
            IconButton::new(Icon::MoreHorizontal, r)
                .compact()
                .tint(r.on_surface_variant)
                .on_press(more_message.clone()),
            more_message,
            r,
        ),
        "More actions",
        r,
    ))
    .on_primary_bounds(|corner| Message::Sidebar(SidebarMsg::MoreMenuAnchored(corner)));
    // As tall as a chip line and centred in it, so the actions sit on the first line's midline.
    let actions = container(
        row![add, hide, more]
            .spacing(spacing::XS)
            .align_y(Alignment::Center),
    )
    .height(Length::Fixed(micold_core::tokens::anatomy::chip::HEIGHT))
    .align_y(Alignment::Center);
    row![chips, actions]
        .spacing(spacing::XS)
        .align_y(Alignment::Start)
        .into()
}

/// `control` as a Tab stop answering Enter and Space with `message`, with §5's focus ring: the
/// chip bar's chips and icon buttons hold no keyboard focus of their own (`keyboard_focus.rs`).
fn takes_the_keyboard<'a>(
    control: impl Into<Element<'a, Message>>,
    message: Message,
    r: Roles,
) -> Element<'a, Message> {
    use crate::ui::material::keyboard_focus::{Indicator, TakesTheKeyboard};
    TakesTheKeyboard::new(control, true)
        .key(iced::keyboard::key::Named::Enter, message.clone())
        .key(iced::keyboard::key::Named::Space, message)
        .indicator(Indicator {
            outline: r.secondary,
            layer: r.on_surface_variant,
            radius: tokens::shape::FULL,
        })
        .into()
}

/// The more menu's items (feature 1125, contract S3): the header buttons' labels, enabled rules
/// and tooltips, each choosing [`SidebarMsg::MoreMenuChosen`].
pub fn more_menu_items(state: &State) -> Vec<material::MenuItem<Message>> {
    use crate::features::sidebar::MoreAction;
    crate::features::sidebar::more_menu_items(
        state.workspace.active.is_some(),
        state.worktree.refreshing,
    )
    .into_iter()
    .map(|item| {
        let icon = match item.action {
            MoreAction::Refresh => Icon::Refresh,
            MoreAction::AttachExisting => Icon::OpenProject,
            MoreAction::RunInParallel => Icon::AiCli,
        };
        material::MenuItem {
            message: item
                .enabled
                .then_some(Message::Sidebar(SidebarMsg::MoreMenuChosen(item.action))),
            // No `trailing_tooltip`: a menu item draws one only over a trailing icon, so the
            // label carries the header button's tooltip text instead (D20).
            ..material::MenuItem::new(icon, item.label, Message::NoOp)
        }
    })
    .collect()
}

/// What choosing `action` on the more menu does: the message today's header button sent (FR-015).
/// The shell sends it once the menu has closed, so the refresh still goes through its request path.
pub fn more_action_message(action: crate::features::sidebar::MoreAction) -> Message {
    use crate::features::sidebar::MoreAction;
    match action {
        MoreAction::Refresh => Message::Worktree(WorktreeMsg::RefreshRequested),
        MoreAction::AttachExisting => Message::Attach(crate::features::attach::Msg::Opened),
        MoreAction::RunInParallel => Message::Sidebar(SidebarMsg::RunInParallelRequested),
    }
}

/// Where the more menu hangs when no pointer press reported its trigger's corner (a keyboard
/// press has no point): the sidebar's trailing edge, level with the chip bar's first line's
/// bottom. The menu is clamped into the window from there like any other.
pub fn more_menu_fallback_anchor(state: &State) -> (u16, u16) {
    use micold_core::tokens::{anatomy, density};
    let overview_row = density::height(
        density::LIST_ROW_BASE,
        crate::features::sidebar::SIDEBAR_DENSITY,
    );
    let y = anatomy::app_bar::BOTTOM_EDGE
        + spacing::SM
        + overview_row
        + spacing::SM
        + anatomy::chip::HEIGHT;
    (state.sidebar_width_px(), y.round() as u16)
}

/// One action icon in a worktree row's hover cluster. When `active` (the hovered row) it is
/// pressable and carries a tooltip; otherwise it is inert. The button occupies its slot
/// regardless, so the row never reflows.
///
/// It does not fade itself: the whole cluster does, through [`HoverReveal`], which is one track
/// per row rather than one per icon and so cannot let a row's two icons drift apart.
fn action_icon(
    glyph: Icon,
    tint: Rgb,
    message: Message,
    tooltip: &'static str,
    active: bool,
    r: Roles,
) -> Element<'static, Message> {
    let button = IconButton::new(glyph, r)
        // Inside the sidebar's dense rows — see `IconButton::compact` for the contract conflict
        // this resolves at the call site rather than in the component.
        .compact()
        .size(TypeRole::SidebarName)
        .tint(tint)
        .on_press_maybe(active.then_some(message));
    if active {
        Tooltip::new(button, tooltip, r).into()
    } else {
        button.into()
    }
}

/// The stable widget id of a location's start control, so the Overview can move the keyboard to it
/// (feature 813, R12).
pub fn start_control_id(location: &SessionLocation) -> iced::advanced::widget::Id {
    iced::advanced::widget::Id::from(match location {
        SessionLocation::Default => "start-control-default".to_string(),
        SessionLocation::Worktree(dir) => format!("start-control-{dir}"),
    })
}

/// What the primary half of the start affordance publishes for `location` (feature 026, T085).
///
/// The branch is [`State::start_intent`]'s — this only names the message each answer arrives as, so
/// the view keeps dispatching and decides nothing (Principle I's GUI exception covers drawing and
/// does not cover branching).
///
/// The case this exists for is FR-004's fourth scenario: a stored default that is not installed.
/// Pressing start then offers the available CLIs and starts nothing, rather than sending the
/// missing binary to the daemon and letting FR-010's failure arrive on a row that was never going
/// to run. It publishes the same message the chevron does, which is also how the list it opens is
/// current: the row's answer is refreshed on `SessionStartMenuOpened`, one of the named events of
/// feature 033's contract C1.
///
/// The answer this decision reads is this row's directory's (feature 033, FR-001) — the home
/// directory's until the row's own arrives (FR-005) — as of the last event that asked for it, and
/// it can be out of date by a press. Both ways round are safe and neither is silent: a default that has since
/// been uninstalled reaches the daemon, whose launch-time check reports which CLI is missing
/// (FR-010, T076); a default that has since been installed opens a list that — refreshed by this
/// very press — contains it, one extra press away. What is ruled out is the third answer, which is
/// starting something other than what the user asked for.
fn start_press(state: &State, location: SessionLocation) -> Message {
    let dir = state.location_dir(&location).unwrap_or_default();
    match state.session.start_intent(PressTarget::Primary, &dir) {
        StartIntent::Start(provider) => Message::Session(SessionMsg::StartRequested {
            location,
            provider,
            kind: micold_core::session::SessionKind::Terminal,
            automatic: false,
        }),
        StartIntent::OfferChoice {
            unavailable_default,
            ..
        } => Message::Session(SessionMsg::StartMenuOpened {
            location,
            unavailable_default,
        }),
        // Nothing at all is installed, so there is no list: FR-006 forbids opening one that is
        // present-and-empty, and an inert `+` would leave the user with a control that answers
        // nothing. The stored default goes to the daemon, whose report names the CLI to install
        // (FR-010) — in this state that report is the only thing that can tell the user anything.
        StartIntent::NothingAvailable => Message::Session(SessionMsg::StartRequested {
            automatic: false,
            location,
            provider: state.session.provider_for_start(None),
            kind: micold_core::session::SessionKind::Terminal,
        }),
    }
}

/// The start-a-session affordance for one location (feature 026, T033).
///
/// A [`SplitAction`], not two icons in a row: the primary half publishes `press` in one press
/// exactly as the plain button did (SC-001), and the chevron opens the list of installed CLIs. The
/// chevron is **absent** — not disabled — when `offers_a_choice` is false, which is what keeps the
/// single-CLI user's sidebar exactly as it was.
///
/// It renders an intent and decides nothing: `press` and `offers_a_choice` are [`start_press`] and
/// `State::start_affordance_offers_a_choice`, and pressing either half emits a message the reducer
/// resolves.
fn start_session_action(
    location: SessionLocation,
    tooltip: &'static str,
    active: bool,
    press: Message,
    offers_a_choice: bool,
    focusable: bool,
    r: Roles,
) -> Element<'static, Message> {
    let mut action = SplitAction::new(Icon::AddSession, r);
    if focusable {
        action = action.focus_id(start_control_id(&location));
    }
    action
        // Inside the sidebar's dense rows — see `IconButton::compact` for the contract conflict
        // this resolves at the call site rather than in the component.
        .compact()
        .size(TypeRole::SidebarName)
        .tint(r.primary)
        .primary_tooltip(tooltip)
        .secondary_tooltip("Start a session on a different AI CLI")
        .on_press_maybe(active.then_some(press))
        .on_secondary_press_maybe((active && offers_a_choice).then(|| {
            Message::Session(SessionMsg::StartMenuOpened {
                location,
                // The chevron asked for the list. Nothing about it needs explaining, and a banner
                // here would fire on every override open for a user who knows their default is
                // gone (BUG-001).
                unavailable_default: None,
            })
        }))
        // Where the list hangs from: the press point of whichever half opened it, since a sidebar
        // row's position is not something the view holds (018 BUG-008, FR-029d). Both halves can
        // open it — the chevron always, the primary half when the default is not installed.
        .on_primary_anchor(|anchor| Message::Session(SessionMsg::StartMenuAnchored(anchor)))
        .on_secondary_anchor(|anchor| Message::Session(SessionMsg::StartMenuAnchored(anchor)))
        .into()
}

/// The hover-revealed row-action cluster for a worktree (feature 008): an add-session "+" (only
/// when a session can start) and a trash icon that requests deletion. The cluster is rendered on
/// EVERY row so its width is always reserved (no reflow when it appears); `active` (the hovered
/// row) both gates whether the icons are pressable and is the reveal's destination.
fn row_actions_cluster(
    dir: &str,
    can_start_session: bool,
    active: bool,
    press: Message,
    offers_a_choice: bool,
    revealed: bool,
    r: Roles,
) -> Element<'static, Message> {
    // The Overview's revealed row offers its start control to the keyboard whether or not hovered.
    let active = active || revealed;
    let mut cluster = row![].spacing(spacing::XS).align_y(Alignment::Center);
    if can_start_session {
        cluster = cluster.push(start_session_action(
            SessionLocation::Worktree(dir.to_string()),
            "Start a new session in this worktree",
            active,
            press,
            offers_a_choice,
            revealed,
            r,
        ));
    }
    cluster = cluster.push(action_icon(
        Icon::Delete,
        r.error,
        Message::Worktree(WorktreeMsg::DeleteRequested(dir.to_string())),
        "Delete this worktree",
        active,
        r,
    ));
    // The veil has to be the tone the cluster sits on, which is the sidebar panel — elevation 1,
    // `surface_container_low` — and *not* `r.surface`. Passing the latter painted a rectangle four
    // tones too dark around the icons for the length of every reveal, which is what a screen
    // recording of the row actions showed.
    HoverReveal::new(cluster, material::SurfaceKind::Sidebar.tone(r))
        .shown(active)
        .into()
}

/// The drawn list (feature 1125, contract S4): each state group's header, then the group's
/// entries when it is open, as [`build_items`] draws them.
fn list_items(
    state: &State,
    groups: Vec<crate::features::sidebar::SidebarGroup>,
    r: Roles,
) -> Vec<TreeItem<'static, Message>> {
    use crate::features::sidebar::ListRow;
    let mut items = Vec::new();
    for row in crate::features::sidebar::list_rows(groups) {
        match row {
            // A header holds no chord's keyboard row, so a change of its focus tells nobody.
            ListRow::Header(header) => items.push(
                group_header(header, r)
                    .keyboard_focus(false, |_| Message::NoOp)
                    .key(row_key(RowKey::Header(header.group))),
            ),
            ListRow::Entry(entry) => {
                let at = items.len();
                items.extend(build_items(state, vec![entry], 0, None, at, r))
            }
        }
    }
    items
}

/// A state group's header row (feature 1125, R8): its name and count, the twisty open or closed,
/// and a press anywhere on it (or Enter or Space) toggling the group.
fn group_header(
    header: crate::features::sidebar::GroupHeaderRow,
    r: Roles,
) -> TreeItem<'static, Message> {
    TreeItem::new(
        0,
        micold_core::sidebar_groups::header_text(header.group, header.count),
        r.on_surface_variant,
    )
    .heading()
    .expandable(
        header.open,
        Message::Sidebar(SidebarMsg::GroupToggled(header.group)),
    )
}

/// Flatten the sidebar's location list (feature 010: the "Default" entry, then worktrees) into
/// ordered [`TreeItem`]s, each followed by its sessions when expanded.
fn build_items(
    state: &State,
    entries: Vec<crate::features::sidebar::SidebarEntry>,
    depth: u16,
    run: Option<(u8, micold_core::session::AiCli)>,
    at: usize,
    r: Roles,
) -> Vec<TreeItem<'static, Message>> {
    let mut items = Vec::new();
    let in_view = crate::features::attention::in_view(&state.attention);
    let project_root = state.workspace.active.as_deref();
    let now = unix_now();

    for entry in entries {
        // Feature 040 (FR-001): the pull request of this row's branch, as the marks the shared
        // indicator draws. `None` for every row when nothing is held, and the row is then built
        // exactly as it was.
        let row_pr = crate::features::sidebar::row_pull_request(
            &entry,
            &state.pr_status.statuses,
            &state.pr_status.removable,
            state.pr_status.read_at,
            now,
        );
        let pull_request = row_pr.map(|row| (indicator_marks(&row.status.state), row.stale));
        // The location's attention indicator (feature 575, FR-001/002): its unread sessions, not
        // closed and not the one in view, counted whether the row is expanded or not (FR-003).
        let unread = entry.unread_count(in_view);
        let node = match entry {
            crate::features::sidebar::SidebarEntry::Default(node) => {
                items.extend(build_default_item(state, &node, unread, r));
                continue;
            }
            crate::features::sidebar::SidebarEntry::Worktree(node) => node,
            crate::features::sidebar::SidebarEntry::Group(group) => {
                items.extend(group_items(state, group, at + items.len(), r));
                continue;
            }
        };
        let wt = &node.worktree;
        // No leading git icon (FR-010); a non-Valid worktree is cued by an error-tinted name
        // plus a status tag (FR-011).
        let tint = match wt.status {
            WorktreeStatus::Valid => r.on_surface,
            WorktreeStatus::Missing | WorktreeStatus::Invalid => r.error,
        };

        // The label chips (feature 1125, FR-013): today's, in today's order, without the type and
        // issue tags the second line's short branch now carries. Why each is there is on
        // `features::sidebar::RowChip`; the status chip keeps its error accent (FR-011).
        let tags: Vec<(String, Rgb)> =
            crate::features::sidebar::row_chips(&node, row_pr.is_some_and(|row| row.removable))
                .into_iter()
                .map(|chip| {
                    let accent = match chip {
                        crate::features::sidebar::RowChip::Status(_) => r.error,
                        _ => r.secondary,
                    };
                    (chip.label(), accent)
                })
                .collect();
        let dir = wt.dir_name.clone();
        // The index this row is drawn at, which a hover records (feature 1125, R14).
        let index = at + items.len();
        // Feature 1101 (FR-003): the ticket's status, first after the second line's text (feature
        // 1125, R6), from the one board every surface reads (FR-010). A ticket without a status has
        // no chip and no gap.
        let status_chip = node.tags.iter().find_map(|tag| match tag {
            Tag::Issue(key) => state.jira_status.board.status_of(key),
            _ => None,
        });
        // The two lines (feature 1125, FR-011): the title, then the short branch and sessions.
        let title = crate::features::sidebar::row_title(
            &node,
            state.overview.known_title(&node.location()),
            state.workspace.worktree_name(&dir),
        );
        let second_line = crate::features::sidebar::row_second_line(&node, &title);

        let keyboard = state.sidebar.keyboard_row.as_ref();
        let mut item = TreeItem::new(depth, title.clone(), tint)
            .supporting_text(second_line, r.on_surface_variant)
            .tags(tags)
            .on_right_press({
                let dir = dir.clone();
                move |point| Message::Worktree(WorktreeMsg::MenuToggled(dir.clone(), point))
            })
            .hover(
                Message::Worktree(WorktreeMsg::Hovered(dir.clone(), index)),
                Message::Worktree(WorktreeMsg::Unhovered(dir.clone())),
            );
        if let Some(entry) = status_chip {
            item = item.leading_chip(
                crate::ui::material::StatusChip::new(&entry.status, r).stale(entry.stale),
            );
        }
        // FR-017: an expander only for several sessions. FR-018: a press selects the main
        // session, never a helper; with no session it selects nothing.
        if node.has_expander() {
            item = item.expandable(
                node.expanded,
                Message::Sidebar(SidebarMsg::WorktreeExpansionToggled(dir.clone())),
            );
        }
        if let Some(main) = node.main_session() {
            item = item.on_press(Message::Session(SessionMsg::Selected(main)));
        }
        // The row holding the viewed session when it draws no sub-row for it, or the row the
        // Overview sent the user to; none while the Overview is open (feature 1125, FR-009).
        let location = node.location();
        item = item.selected(row_selected(
            &row_selection(state),
            SelectableRow::Location {
                location: &location,
                sessions: pressable_sessions(node.main_session(), &node.sessions),
                sessions_shown: node.sessions_shown(),
            },
        ));
        // Row tooltip (feature 010 FR-010, extended by feature 029): the full name the row had to
        // shorten, plus the facts its prettified label leaves out. Attached unconditionally — an
        // unknown project root costs the tooltip its `Location` line, not its existence (029 R5),
        // and the builder is the only thing that decides which lines those are.
        //
        // `node.display_name` rather than a second derivation from `dir_name`: it is the exact
        // string this row renders, so the row and its tooltip cannot disagree about a name (FR-002).
        //
        // The indicator states its meaning in words on the tooltip's last line (feature 575,
        // FR-005); the row shows the mark and the number only.
        //
        // The state icon (feature 905, FR-001, FR-009): the worktree's state from all its
        // sessions, in its slot after the twisty, and in words before the unread line.
        let location_state = node.state();
        item = item
            .state_icon(StateIcon::new(location_state, r))
            .unread_count(unread)
            .row_tooltip(crate::features::sidebar::with_unread_line(
                crate::features::sidebar::with_state_line(
                    crate::features::sidebar::with_title_line(
                        &node,
                        &title,
                        crate::features::sidebar::worktree_tooltip(
                            project_root,
                            wt,
                            &node.display_name,
                            row_pr,
                        ),
                    ),
                    location_state,
                ),
                unread,
            ));

        // Always reserve the action cluster's width so hovering never reflows the row; each row
        // fades its icons in/out independently via its own animation track (feature 008). The
        // hovered row is the pressable one, while it is still where the pointer entered it (feature
        // 1125, R14: a row a state change moved out from under the pointer is not hovered).
        let active = state
            .worktree
            .hovered
            .as_ref()
            .is_some_and(|(hovered, entered)| {
                *hovered == dir && hover_holds(state, &dir, *entered)
            });
        let cluster = row_actions_cluster(
            &dir,
            // FR-004: no start action on a worktree that has a session.
            wt.can_start_session() && state.start_offered(&node.location()),
            active,
            // Threaded in rather than reached for: this function takes narrow arguments and cannot
            // see `State` (feature 026, T014a). Both answers come from the render-free layer —
            // nothing here decides what a press means or whether there is a choice to offer.
            start_press(state, SessionLocation::Worktree(dir.clone())),
            state.session.start_affordance_offers_a_choice(
                &state
                    .location_dir(&SessionLocation::Worktree(dir.clone()))
                    .unwrap_or_default(),
            ),
            state.sidebar.revealed.as_ref() == Some(&node.location()),
            r,
        );
        item = item.trailing_element(match pull_request {
            // The indicator leads the trailing element, before the actions (UI §2).
            Some(((mark, checks), stale)) => {
                let mut indicator = PullRequestIndicator::<Message>::new(mark, &r).stale(stale);
                if let Some(checks) = checks {
                    indicator = indicator.checks(checks);
                }
                row![Element::from(indicator), cluster]
                    .spacing(spacing::XS)
                    .align_y(Alignment::Center)
                    .into()
            }
            None => cluster,
        });
        // A run's row says which run and provider it is (G1).
        if let Some((number, provider)) = run {
            item = item.annotation(
                format!("#{number} {}", provider.provider().command()),
                r.on_surface_variant,
            );
        }
        let focused = keyboard.is_some_and(|row| {
            row.is_location_row(
                &node.location(),
                node.sessions_shown(),
                &node.listed_sessions(),
            )
        });
        items.push(with_keyboard_focus(
            item,
            focused,
            RowKey::Row(RowRef::Location(location)),
        ));

        if node.sessions_shown() {
            let main = node.main_session();
            for row in node.session_rows() {
                items.push(with_keyboard_focus(
                    with_fold(
                        session_tree_item(
                            depth + 1 + row.depth.min(MAX_NESTING) as u16,
                            row.session,
                            row_selected(
                                &row_selection(state),
                                SelectableRow::Session(row.session.id),
                            ),
                            in_view,
                            Some(row.session.id) != main && !row.place.is_child(),
                            row.place,
                            state.session_start_block_text(row.session.id),
                            r,
                        ),
                        &row,
                        in_view,
                    ),
                    keyboard.is_some_and(|k| k.is_session_row(row.session.id)),
                    RowKey::Row(RowRef::Session(row.session.id)),
                ));
            }
        }
    }

    items
}

/// What a sidebar row is (feature 1125, R13): a location's or a session's row by its [`RowRef`], a
/// state group's header, a run group's row, or a run that has no worktree.
#[derive(Hash)]
enum RowKey {
    Row(RowRef),
    Header(micold_core::sidebar_groups::StateGroup),
    RunGroup(micold_core::runs::GroupId),
    Run(micold_core::runs::GroupId, u8),
}

/// The [`TreeItem::key`] of a row: every sidebar row has one, so the keyboard's focus, the ripple
/// and the hover stay on a row that moves to another group (FR-017, US5.5).
fn row_key(row: RowKey) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    row.hash(&mut hasher);
    hasher.finish()
}

/// Give `item` the keyboard's focus wrapper (feature 803, FR-010, U4) and its key (feature 1125,
/// R13). Every row gets one, focused or not, so the tree keeps its shape across presses; losing
/// the keyboard tells the sidebar.
fn with_keyboard_focus(
    item: TreeItem<'static, Message>,
    focused: bool,
    key: RowKey,
) -> TreeItem<'static, Message> {
    item.key(row_key(key)).keyboard_focus(focused, |focused| {
        if focused {
            Message::NoOp
        } else {
            Message::Sidebar(SidebarMsg::KeyboardRowLeft)
        }
    })
}

/// A run group's row and, when it is expanded, its runs (feature 483, G1, G1a, G2, G3).
fn group_items(
    state: &State,
    group: crate::features::sidebar::GroupNode,
    at: usize,
    r: Roles,
) -> Vec<TreeItem<'static, Message>> {
    use crate::features::sidebar::{RunRow, SidebarEntry};
    let mut tags = vec![(
        match group.run_count {
            1 => "1 run".to_string(),
            n => format!("{n} runs"),
        },
        r.secondary,
    )];
    if group.failed_count > 0 {
        tags.push((format!("{} failed", group.failed_count), r.error));
    }
    let mut items = vec![with_keyboard_focus(
        TreeItem::new(0, group.name.clone(), r.on_surface)
            .tags(tags)
            .row_tooltip(group.name.clone())
            .on_right_press({
                let id = group.id;
                move |point| Message::Runs(crate::features::runs::Msg::MenuToggled(id, point))
            })
            .expandable(
                group.expanded,
                Message::Runs(crate::features::runs::Msg::GroupToggled(group.id)),
            ),
        false,
        RowKey::RunGroup(group.id),
    )];
    if !group.expanded {
        return items;
    }
    for run in group.runs {
        let (number, provider) = (run.number, run.provider);
        match run.row {
            RunRow::Worktree(node) => items.extend(build_items(
                state,
                vec![SidebarEntry::Worktree(node)],
                1,
                Some((number, provider)),
                at + items.len(),
                r,
            )),
            RunRow::NoWorktree => {
                let failed = run.status.is_failed();
                let mut item = TreeItem::new(
                    1,
                    format!("#{number}"),
                    if failed {
                        r.error
                    } else {
                        r.on_surface_variant
                    },
                )
                .annotation(provider.provider().command(), r.on_surface_variant);
                if failed {
                    item = item.tags(vec![("failed".to_string(), r.error)]);
                }
                if let Some(reason) = run.reason() {
                    item = item.row_tooltip(reason.to_string());
                }
                items.push(with_keyboard_focus(
                    item,
                    false,
                    RowKey::Run(group.id, number),
                ));
            }
        }
    }
    items
}

/// Unix seconds now, for the age of a pull request reading. The view glue owns the clock: the
/// render-free layer is handed the time, never asks for it.
pub(crate) fn unix_now() -> u64 {
    micold_core::unix_time::unix_secs()
}

/// The indicator's marks for a pull request state: the shared component does not know the pull
/// request module, so the mapping lives here (contract §1). Only an open or draft one has checks,
/// and `None` means no checks at all (FR-003).
pub(super) fn indicator_marks(state: &PrState) -> (PrMark, Option<CheckMark>) {
    let checks = |c: &CheckStatus| crate::features::changes::check_mark(*c);
    match state {
        PrState::Open { checks: c } => (PrMark::Open, checks(c)),
        PrState::Draft { checks: c } => (PrMark::Draft, checks(c)),
        PrState::Merged => (PrMark::Merged, None),
        PrState::Closed => (PrMark::Closed, None),
    }
}

/// The twisty of a session that started others, and the summary of the children it has folded away
/// (feature 551, FR-011): their count, and the mark when one of them needs the user.
fn with_fold(
    item: TreeItem<'static, Message>,
    row: &crate::features::sidebar::SessionRow<'_>,
    in_view: Option<micold_core::session::SessionId>,
) -> TreeItem<'static, Message> {
    let Some(fold) = &row.fold else {
        return item;
    };
    let item = item.expandable(
        !fold.folded,
        Message::Sidebar(SidebarMsg::ToggleOrigin(row.session.id)),
    );
    if fold.folded {
        item.folded_summary(fold.children, fold.needs_attention(in_view))
    } else {
        item
    }
}

/// One session sub-item, depth 1 — shared by worktree rows and the "Default" row (feature 010)
/// so the two locations render their sessions identically (FR-005 lifecycle parity).
#[allow(clippy::too_many_arguments)]
fn session_tree_item(
    depth: u16,
    session: &micold_core::session::Session,
    selected: bool,
    in_view: Option<micold_core::session::SessionId>,
    helper: bool,
    place: micold_core::origin::TreePlace,
    blocked: Option<String>,
    r: Roles,
) -> TreeItem<'static, Message> {
    use micold_core::origin::{Orphan, TreePlace};
    let tint = match session.lifecycle {
        SessionLifecycle::Failed { .. } => r.error,
        SessionLifecycle::Idle => r.on_surface_variant,
        // Interrupted-but-resumable reads as needing attention, distinct from a plain idle stop
        // (FR-006a: "visibly different from both running and a deliberately stopped session").
        SessionLifecycle::InterruptedResumable => r.primary,
        _ => r.on_surface,
    };
    // No leading icon: the activity dot below is the row's *sole* leading indicator (FR-016f).
    // A session row used to carry an unconditional `Icon::ActiveMarker` (`check_circle`) here — a
    // feature-005 leftover from that icon's real job, marking the *active known project*. It never
    // varied with session state, so it read as "done / OK" on a failed or interrupted session while
    // competing with the dot that does vary (BUG-005). Lifecycle still reaches the user: `tint`
    // above colours the label itself, not just a glyph.
    let mut item = TreeItem::new(depth, session.label.display().to_string(), tint)
        // The derived activity dot beside the name (feature 010 US2, FR-016d): Working/AwaitingInput
        // show a filled dot, Ended a hollow one, Unknown nothing (ambient — H2) in a slot that stays
        // the same width either way, so names stay aligned as signals change (FR-016f).
        .badge(ActivityBadge::<Message>::new(session.activity.clone(), r))
        // Which AI CLI this session runs, as short text (feature 026, FR-016).
        //
        // `command()` — `claude`, `copilot` — and not `display_name()`. A row label lives in a
        // width budget; "GitHub Copilot" is a menu entry, and the human-readable register stays
        // where sentences are (Clarifications 2026-08-18).
        //
        // Text, not a colour or a glyph alone: those need a legend, and a legend is not something
        // a sidebar has room for. Muted, because it identifies rather than states — the name is
        // what the row is *about*, and this is what it is *running*.
        //
        // An annotation, not a tag chip: tags open a second line, and `row_heights` hardcodes that
        // a session row is one line. The label changes the row's content, never its height.
        .annotation(
            // A helper says so (FR-017); the main session is the unmarked one.
            {
                let command =
                    &crate::features::session::command_caption(session.provider, session.kind);
                match place {
                    // An orphan says why it is not nested (FR-016, FR-018).
                    TreePlace::Orphan(Orphan::Pending) => format!("origin pending, {command}"),
                    TreePlace::Orphan(Orphan::Removed) => format!("origin removed, {command}"),
                    _ if helper => format!("helper, {command}"),
                    _ => command.to_string(),
                }
            },
            r.on_surface_variant,
        )
        // The child marker (feature 551, FR-007): on every session an agent started, nested or not.
        .child(place.is_child())
        // The unread mark (feature 039, FR-018, contract `unread-mark.md` U5): on the row of an
        // unread session this window does not have in view.
        .unread(crate::features::attention::row_unread(session, in_view))
        .selected(selected)
        .on_press(Message::Session(SessionMsg::Selected(session.id)))
        .on_right_press({
            // The id, copied out of the borrow: the closure outlives this `&Session`.
            let id = session.id;
            move |point| Message::Session(SessionMsg::MenuToggled(id, point))
        })
        .trailing(
            Icon::Close,
            Message::Session(SessionMsg::CloseRequested(session.id)),
            "Close this session",
        );
    // A start another live session of the worktree blocks says why on hover (feature 776, FR-013).
    if let Some(text) = blocked {
        item = item.row_tooltip(text);
    }
    item
}

/// The "Default" (project-root) row + its session sub-items when expanded (feature 010, US1).
/// Always present when a project is open (`sidebar_entries`'s first entry); its "start a
/// session" action reuses the same `IconButton`/`Icon::AddSession` affordance as a worktree row,
/// but — unlike worktree rows' hover-revealed cluster — is always shown, since it is the row's
/// sole action and there is only ever one Default row to keep tidy (no reflow/clutter concern
/// that motivated the fade-on-hover treatment for the potentially-many worktree rows).
fn build_default_item(
    state: &State,
    node: &crate::features::sidebar::DefaultNode,
    unread: usize,
    r: Roles,
) -> Vec<TreeItem<'static, Message>> {
    let mut items = Vec::new();

    // `active: true` gives the always-visible, always-pressable button this row needs (see the doc
    // comment above), reusing the same construction as a worktree row's hover-revealed action icons
    // instead of hand-rebuilding it. It is deliberately not wrapped in `HoverReveal`: this row's
    // action is always shown, so it has nothing to reveal.
    let start_session = start_session_action(
        SessionLocation::Default,
        "Start a new session in the project root",
        true,
        start_press(state, SessionLocation::Default),
        state.session.start_affordance_offers_a_choice(
            &state
                .location_dir(&SessionLocation::Default)
                .unwrap_or_default(),
        ),
        state.sidebar.revealed == Some(SessionLocation::Default),
        r,
    );

    let location_state = node.state();
    let mut item = TreeItem::new(0, node.display_name.to_string(), r.on_surface)
        // Distinct icon (FR-006): never the git/branch iconography used for worktree rows.
        .with_icon(Icon::ProjectRoot)
        // "project root · " and its sessions (feature 1125, FR-011).
        .supporting_text(
            crate::features::sidebar::default_second_line(node),
            r.on_surface_variant,
        )
        // Feature 482, V1: the Default row's menu, which is the worktree menu under the wire's
        // name for the project root (`""`).
        .on_right_press(|point| Message::Worktree(WorktreeMsg::MenuToggled(String::new(), point)))
        // The project root's state icon (feature 905, FR-002), as on a worktree row.
        .state_icon(StateIcon::new(location_state, r))
        // The attention indicator (feature 575, FR-001), as on a worktree row.
        .unread_count(unread)
        // Location tooltip (FR-010): fixed, since the Default entry is always the project root,
        // plus the state's words (feature 905, FR-009) and the indicator's when it shows (feature
        // 575, FR-005).
        .row_tooltip(crate::features::sidebar::with_unread_line(
            crate::features::sidebar::with_state_line(
                crate::features::sidebar::DEFAULT_LOCATION_LABEL.to_string(),
                location_state,
            ),
            unread,
        ));
    if state.start_offered(&SessionLocation::Default) {
        item = item.trailing_element(start_session);
    }
    if node.has_expander() {
        item = item.expandable(
            node.expanded,
            Message::Sidebar(SidebarMsg::DefaultExpansionToggled),
        );
    }
    if let Some(main) = node.main_session() {
        item = item.on_press(Message::Session(SessionMsg::Selected(main)));
    }
    item = item.selected(row_selected(
        &row_selection(state),
        SelectableRow::Location {
            location: &SessionLocation::Default,
            sessions: pressable_sessions(node.main_session(), &node.sessions),
            sessions_shown: node.sessions_shown(),
        },
    ));
    let keyboard = state.sidebar.keyboard_row.as_ref();
    let focused = keyboard.is_some_and(|row| {
        row.is_location_row(
            &SessionLocation::Default,
            node.sessions_shown(),
            &node.listed_sessions(),
        )
    });
    items.push(with_keyboard_focus(
        item,
        focused,
        RowKey::Row(RowRef::Location(SessionLocation::Default)),
    ));

    if node.sessions_shown() {
        let in_view = crate::features::attention::in_view(&state.attention);
        let main = node.main_session();
        for row in node.session_rows() {
            items.push(with_keyboard_focus(
                with_fold(
                    session_tree_item(
                        1 + row.depth.min(MAX_NESTING) as u16,
                        row.session,
                        row_selected(
                            &row_selection(state),
                            SelectableRow::Session(row.session.id),
                        ),
                        in_view,
                        Some(row.session.id) != main && !row.place.is_child(),
                        row.place,
                        state.session_start_block_text(row.session.id),
                        r,
                    ),
                    &row,
                    in_view,
                ),
                keyboard.is_some_and(|k| k.is_session_row(row.session.id)),
                RowKey::Row(RowRef::Session(row.session.id)),
            ));
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::protocol::messages::ActivitySignal;
    use micold_core::session::{AiCli, Session};
    use micold_core::theme::ColorScheme;

    /// A `Failed` in the shape supervision produces one. These cases are about the *variant* — the
    /// tint, the status word, whether restart is offered — not about the sentence, but the variant
    /// has carried one since `010` BUG-017 and something has to fill it.
    fn failed() -> SessionLifecycle {
        SessionLifecycle::Failed {
            reason: "Gave up after 3 restart attempts — last exit: exit status 1.".into(),
            attempts: 3,
        }
    }

    fn session(activity: ActivitySignal, lifecycle: SessionLifecycle) -> Session {
        let mut s = Session::start_new(
            SessionLocation::Worktree("feat-a".to_string()),
            AiCli::ClaudeCode,
        );
        s.activity = activity;
        s.lifecycle = lifecycle;
        s
    }

    /// FR-016f: the activity badge is a session row's *sole* leading indicator. A constant glyph
    /// beside it carries no state while competing with the one that does — this is BUG-005, where
    /// an unconditional `check_circle` read as "done/OK" on failed and interrupted sessions alike.
    #[test]
    fn a_session_row_has_no_leading_icon() {
        let r = tokens::roles(ColorScheme::Dark);
        for lifecycle in [
            SessionLifecycle::Idle,
            SessionLifecycle::Starting,
            SessionLifecycle::Running,
            SessionLifecycle::Restarting { attempts: 1 },
            failed(),
            SessionLifecycle::InterruptedResumable,
        ] {
            let s = session(ActivitySignal::Unknown, lifecycle.clone());
            let item: TreeItem<'_, Message> = session_tree_item(
                1,
                &s,
                false,
                None,
                false,
                micold_core::origin::TreePlace::Root,
                None,
                r,
            );
            assert!(
                item.icon.is_none(),
                "session row for {lifecycle:?} still carries a leading icon"
            );
            assert!(
                item.badge.is_some(),
                "the activity badge must still occupy the row's indicator slot"
            );
        }
    }

    /// The lifecycle distinction survives the icon's removal because the row tint is applied to the
    /// label too (`tree_view.rs`), so no state information rode on the glyph alone (FR-006a).
    #[test]
    fn lifecycle_still_reaches_the_row_through_the_tint() {
        let r = tokens::roles(ColorScheme::Dark);
        let tint = |l| {
            session_tree_item(
                1,
                &session(ActivitySignal::Unknown, l),
                false,
                None,
                false,
                micold_core::origin::TreePlace::Root,
                None,
                r,
            )
            .tint
        };

        assert_eq!(tint(failed()), r.error);
        assert_eq!(tint(SessionLifecycle::Idle), r.on_surface_variant);
        assert_eq!(tint(SessionLifecycle::InterruptedResumable), r.primary);
        assert_eq!(tint(SessionLifecycle::Running), r.on_surface);
    }

    /// FR-007: a child row's select and close messages are the ones a top-level row's are, for its
    /// own session; only the marker and the annotation tell them apart.
    #[test]
    fn a_child_row_acts_like_a_top_level_row() {
        use micold_core::origin::{Orphan, TreePlace};
        let r = tokens::roles(ColorScheme::Dark);
        let s = session(ActivitySignal::Unknown, SessionLifecycle::Running);
        let row = |place| session_tree_item(1, &s, false, None, false, place, None, r);
        let top = row(TreePlace::Root);
        for place in [TreePlace::Child, TreePlace::Orphan(Orphan::Pending)] {
            let child = row(place);
            assert!(child.child && !top.child);
            assert_eq!(
                format!("{:?}", child.on_press),
                format!("{:?}", top.on_press)
            );
            assert_eq!(
                format!("{:?}", child.trailing),
                format!("{:?}", top.trailing)
            );
        }
    }

    // --- Feature 1125 (T035, S2): the Overview row --------------------------------------------

    /// A state with one project open and nothing else.
    fn with_project() -> State {
        use micold_core::project::{Availability, Project};
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state
            .workspace
            .projects
            .push(Project::new(path.clone(), true, Availability::Available));
        state.workspace.active = Some(path);
        state
    }

    /// What Enter on the sidebar's first Tab stop publishes.
    fn enter_on_first_focusable(state: &State) -> Vec<String> {
        use crate::ui::material::test_support::{named_key, Mounted};
        let mut mounted = Mounted::new(view(state, ColorScheme::Dark).map(|m| format!("{m:?}")));
        mounted.focus_next();
        mounted.redraw();
        mounted.send(
            named_key(iced::keyboard::key::Named::Enter),
            iced::mouse::Cursor::Unavailable,
        )
    }

    /// The Overview row's message, as `Debug` prints it.
    fn opened() -> String {
        format!(
            "{:?}",
            Message::Overview(crate::features::overview::Msg::Opened {
                settings_open: false
            })
        )
    }

    #[test]
    fn the_first_focusable_is_the_overview_row_and_enter_opens_the_overview() {
        assert_eq!(enter_on_first_focusable(&with_project()), [opened()]);
    }

    #[test]
    fn with_settings_open_the_overview_row_publishes_nothing() {
        let mut state = with_project();
        state.settings.settings_draft = Some(Default::default());
        let published = enter_on_first_focusable(&state);
        assert!(
            !published.iter().any(|m| m.contains("Opened")),
            "{published:?}"
        );
    }

    /// What Enter publishes on each of the sidebar's first `stops` Tab stops, in Tab order.
    fn tab_order(state: &State, stops: usize) -> Vec<Vec<String>> {
        use crate::ui::material::test_support::{named_key, Mounted};
        (1..=stops)
            .map(|k| {
                let mut mounted =
                    Mounted::new(view(state, ColorScheme::Dark).map(|m| format!("{m:?}")));
                for _ in 0..k {
                    mounted.focus_next();
                }
                mounted.redraw();
                mounted.send(
                    named_key(iced::keyboard::key::Named::Enter),
                    iced::mouse::Cursor::Unavailable,
                )
            })
            .collect()
    }

    /// Feature 1125 (T045, US5.1, FR-017, SC-005): Tab visits the Overview row, **All**, each
    /// filter chip, add, hide and more, then the group headers, in display order.
    #[test]
    fn tab_visits_the_overview_row_the_chip_bar_then_the_groups_in_display_order() {
        use micold_core::naming::ConventionalType;
        use micold_core::sidebar_groups::StateGroup;
        let mut state = with_project();
        state.worktree.worktrees = ["feat-a", "fix-b"]
            .into_iter()
            .map(|dir| micold_core::worktree::Worktree {
                dir_name: dir.into(),
                path: std::path::Path::new("/repo/.claude/worktrees").join(dir),
                branch: Some(dir.replace('-', "/")),
                status: WorktreeStatus::Valid,
                included: false,
            })
            .collect();
        let m = |message: Message| vec![format!("{message:?}")];
        let filter = |t| {
            m(Message::Sidebar(SidebarMsg::FilterToggled(
                TagFilter::Type(t),
            )))
        };
        let stops = tab_order(&state, 8);
        let mut expected = vec![
            vec![opened()],
            m(Message::Sidebar(SidebarMsg::FiltersCleared)),
            filter(ConventionalType::Feat),
            filter(ConventionalType::Fix),
            m(Message::WorktreeForm(FormMsg::Opened)),
            m(Message::Sidebar(SidebarMsg::Toggled)),
            m(Message::Sidebar(SidebarMsg::MoreMenuToggled)),
        ];
        let groups: Vec<StateGroup> = state.sidebar_groups().iter().map(|g| g.group).collect();
        expected.extend(
            groups
                .iter()
                .take(1)
                .map(|&g| m(Message::Sidebar(SidebarMsg::GroupToggled(g)))),
        );
        assert_eq!(stops, expected);
    }

    #[test]
    fn with_no_project_open_there_is_no_overview_row() {
        use crate::ui::material::test_support::Mounted;
        let label = |state: &State| {
            Mounted::new(view(state, ColorScheme::Dark).map(|m| format!("{m:?}")))
                .painted_text()
                .iter()
                .any(|p| p.content == "Overview")
        };
        assert!(label(&with_project()), "drawn with a project open");
        assert!(!label(&State::default()), "not drawn with none");
    }

    /// Feature 1125 T056 (R13): every drawn row has a key, and no two share one, so the tree lays
    /// them out keyed and a row that moves to another group keeps the keyboard (US5.5).
    #[test]
    fn every_sidebar_row_has_a_distinct_key() {
        use micold_core::worktree::{Worktree, WorktreeStatus};
        let mut state = with_project();
        state.worktree.worktrees = ["feat-a", "feat-b"]
            .into_iter()
            .map(|dir| Worktree {
                dir_name: dir.to_string(),
                path: std::path::PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
                branch: Some(format!("feat/{dir}")),
                status: WorktreeStatus::Valid,
                included: false,
            })
            .collect();
        let mut sessions: Vec<Session> = (0..2)
            .map(|_| {
                Session::start_new(
                    SessionLocation::Worktree("feat-a".to_string()),
                    AiCli::ClaudeCode,
                )
            })
            .collect();
        sessions[0].activity = ActivitySignal::Working;
        state
            .workspace
            .sessions
            .insert(std::path::PathBuf::from("/repo"), sessions);
        state.sidebar.expanded.insert("feat-a".to_string());
        let items = list_items(
            &state,
            state.sidebar_groups(),
            tokens::roles(ColorScheme::Dark),
        );
        let keys: Vec<Option<u64>> = items.iter().map(|item| item.key).collect();
        assert!(
            keys.len() >= 6,
            "headers, locations and session rows: {keys:?}"
        );
        assert!(keys.iter().all(Option::is_some), "{keys:?}");
        let distinct: std::collections::BTreeSet<_> = keys.iter().collect();
        assert_eq!(distinct.len(), keys.len(), "{keys:?}");
    }
}
