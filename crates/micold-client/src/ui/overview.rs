//! The Overview's main-pane view (feature 813, contract U1) and the cards it draws.
//!
//! The cards are derived from several features' state at once, which is composition, not a
//! feature's own business; so it lives here beside the view rather than in `features::overview`.

use crate::app::{Message, State};
use crate::features::overview::{MenuOrigin, Msg};
use crate::features::sidebar::{RunRow, SidebarEntry, WorktreeNode};
use crate::icons::Icon;
use crate::ui::cdk::context_area::ContextArea;
use crate::ui::material::{Button, CardGrid, MenuItem, OverviewCard, Tooltip, Zoom};
use iced::widget::{column, container, row, Space};
use iced::{Element, Length};
use micold_core::location_state::location_state;
use micold_core::occupancy::main_session;
use micold_core::overview::{
    activation, card_state, location_type, order, session_counts, CardModel, CardSignals,
    CardState, CurrentPr, LocationType, PrDisplay, PrSignal, PrUnknown,
};
use micold_core::session::{Session, SessionLocation};
use micold_core::task_progress::{self, Progress, Source};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The card grid's scroll viewport, by name: a zoom's reflow scrolls it to keep the first visible
/// card in view (feature 970, FR-013). A `LazyLock` for the reason
/// [`crate::ui::SIDEBAR_SCROLL_ID`] is one.
pub static GRID_SCROLL_ID: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("overview-grid"));

/// A card's widget id, stable per location: the keyboard goes to the first card's when the Overview
/// opens (research R13) and back to the target's when a card menu closes (feature 972, FR-011).
pub fn card_id(location: &SessionLocation) -> iced::advanced::widget::Id {
    iced::advanced::widget::Id::from(match location {
        SessionLocation::Default => "overview-card-default".to_string(),
        SessionLocation::Worktree(dir) => format!("overview-card-{dir}"),
    })
}

/// The Overview: a grid with one card per location of the active project.
pub fn view<'a>(state: &State, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let menu_open = state.overview.card_menu.is_some();
    let cards = cards(state)
        .iter()
        .map(|model| {
            let location = model.location.clone();
            let menu_at = location.clone();
            OverviewCard::new(model, r)
                .id(card_id(&location))
                .targeted(is_targeted(state, &location))
                .menu_open(menu_open)
                .on_press(Message::Overview(Msg::ActivationRequested(
                    model.activation.clone(),
                )))
                .on_menu_key(Message::Overview(Msg::CardMenuRequested(menu_at)))
                .on_secondary_press(move |anchor| {
                    Message::Overview(Msg::CardMenuOpened {
                        location: location.clone(),
                        anchor,
                        origin: MenuOrigin::Pointer,
                    })
                })
        })
        .collect();
    // The grid scrolls itself; an outer scrollable would give it unbounded height.
    let grid = CardGrid::new(cards, r)
        .on_scroll(Message::Overview(Msg::GridScrolled))
        .scroll_id(GRID_SCROLL_ID.clone())
        .on_reflow(|offset| Message::Overview(Msg::Reflowed(offset.y)))
        .zoom(Zoom {
            full_screen: state.overview.full_screen(),
            freed: crate::ui::freed_width(state),
        });
    let pane = container(column![header(state, r), grid].spacing(spacing::SM))
        .padding(spacing::LG)
        .width(Length::Fill)
        .height(Length::Fill);
    // A right-click on empty space closes an open card menu (FR-008, research R3): a press over a
    // card is captured by the card's own area first, so only the space between and around cards
    // reaches this one.
    ContextArea::new(pane)
        .on_secondary_press(|_| Message::Overview(Msg::CardMenuDismissed))
        .into()
}

/// Whether `location`'s card is the target of the open card menu (FR-006). Identity, not a grid
/// index, so the mark follows the location when the cards reorder.
pub fn is_targeted(state: &State, location: &SessionLocation) -> bool {
    state
        .overview
        .card_menu
        .as_ref()
        .is_some_and(|menu| &menu.location == location)
}

/// The message of the card menu's highlighted entry (feature 972, FR-010): already wrapped in
/// [`Msg::CardMenuEntryChosen`], as the entry's click publishes it. None with no menu, no
/// highlight, an index past the current list (the list changed under the highlight) or an entry
/// that is shown and not pressable.
pub fn card_menu_entry(state: &State) -> Option<Message> {
    let menu = state.overview.card_menu.as_ref()?;
    card_menu_items(state, &menu.location)
        .into_iter()
        .nth(menu.highlighted?)?
        .message
}

/// Where a keyboard-opened menu hangs: just inside `bounds`, the focused card's, by
/// [`spacing::MD`] from its top-left (FR-009). Window pixels, as the pointer's anchor is.
pub fn keyboard_anchor(bounds: iced::Rectangle) -> (u16, u16) {
    let px = |v: f32| v.round().clamp(0.0, f32::from(u16::MAX)) as u16;
    (px(bounds.x + spacing::MD), px(bounds.y + spacing::MD))
}

/// The card the keyboard goes back to after a keyboard-opened menu (FR-011): `target` while its card
/// is listed, else the first card; None with no cards.
pub fn focus_return_target(state: &State, target: &SessionLocation) -> Option<SessionLocation> {
    let cards = cards(state);
    cards
        .iter()
        .find(|model| &model.location == target)
        .or(cards.first())
        .map(|model| model.location.clone())
}

/// The full-screen toggle's label: its action (feature 970, contracts/ui.md *Overview header*).
pub fn full_screen_label(on: bool) -> &'static str {
    if on {
        "Exit full screen"
    } else {
        "Full screen"
    }
}

/// The full-screen toggle's tooltip: its action and the platform's chord (FR-012).
pub fn full_screen_tooltip(on: bool) -> String {
    format!(
        "{} ({})",
        full_screen_label(on),
        crate::keymap::overview_full_screen_chord_label()
    )
}

/// The full-screen toggle's leading glyph, showing which way it goes.
pub fn full_screen_icon(on: bool) -> Icon {
    if on {
        Icon::FullScreenExit
    } else {
        Icon::FullScreen
    }
}

/// The Overview's header row: the full-screen toggle at its end (feature 970, plan D5). Settings
/// open is filled as the top bar fills it for the Overview toggle; the update ignores it then.
fn header<'a>(state: &State, r: tokens::Roles) -> Element<'a, Message> {
    let on = state.overview.full_screen();
    let settings_open = state.settings.settings_draft.is_some();
    let toggle = Tooltip::new(
        Button::outlined(full_screen_label(on), r)
            .leading(full_screen_icon(on))
            .on_press(Message::Overview(Msg::FullScreenToggled { settings_open })),
        full_screen_tooltip(on),
        r,
    );
    row![Space::new().width(Length::Fill), toggle]
        .width(Length::Fill)
        .into()
}

/// The card menu of `location` (feature 972, FR-002): *Open* first, then exactly that location's
/// row-menu entries, each message wrapped in [`Msg::CardMenuEntryChosen`] so the root closes the
/// menu before running it. Built per render from held state, as the row menus are (SC-003).
pub fn card_menu_items(state: &State, location: &SessionLocation) -> Vec<MenuItem<Message>> {
    let chosen = |inner: Message| Message::Overview(Msg::CardMenuEntryChosen(Box::new(inner)));
    let open = cards(state)
        .into_iter()
        .find(|model| &model.location == location)
        .map(|model| chosen(Message::Overview(Msg::CardActivated(model.activation))));
    std::iter::once(with_message(MenuItem::new(Icon::Open, "Open", ()), open))
        .chain(
            crate::ui::location_menu_items(state, location)
                .into_iter()
                .map(|mut item| {
                    let message = item.message.take().map(chosen);
                    with_message(item, message)
                }),
        )
        .collect()
}

/// `item` with `message` in place of its own. Its right-press menu, if any, is dropped: a card-menu
/// entry has none.
fn with_message<A, B>(item: MenuItem<A>, message: Option<B>) -> MenuItem<B> {
    MenuItem {
        icon: item.icon,
        reserve_icon: item.reserve_icon,
        icon_tint: item.icon_tint,
        label: item.label,
        message,
        trailing_text: item.trailing_text,
        trailing_mark: item.trailing_mark,
        trailing_icon: item.trailing_icon,
        trailing_tooltip: item.trailing_tooltip,
        on_context: None,
    }
}

/// Close a card menu whose Overview closed or whose target is no longer listed (research R8,
/// FR-012), and drop a focus return with no Overview to return to. Called once per outermost
/// update, so every way a location disappears or the Overview closes is covered in one place.
pub fn settle_card_menu(state: &mut State) {
    if !state.overview.open {
        state.overview.card_menu = None;
        state.overview.focus_return = None;
        state.overview.focus_landed = None;
        return;
    }
    let Some(target) = state
        .overview
        .card_menu
        .as_ref()
        .map(|m| m.location.clone())
    else {
        return;
    };
    if !cards(state).iter().any(|model| model.location == target) {
        state.overview.card_menu = None;
        state.overview.focus_return = None;
    }
}

/// The cards for the active project: one per sidebar entry (a group contributes its runs'
/// worktrees), worst state first and in the sidebar's order within a state (FR-001, FR-007).
/// Derived from what the window holds on every call; nothing is cached (research R14).
pub fn cards(app: &State) -> Vec<CardModel> {
    let mut cards: Vec<CardModel> = app
        .sidebar_entries()
        .iter()
        .flat_map(|entry| match entry {
            SidebarEntry::Default(_) => vec![card(app, &Place::Default)],
            SidebarEntry::Worktree(node) => vec![card(app, &Place::Worktree(node))],
            SidebarEntry::Group(group) => group
                .runs
                .iter()
                .filter_map(|run| match &run.row {
                    RunRow::Worktree(node) => Some(card(app, &Place::Worktree(node))),
                    RunRow::NoWorktree => None,
                })
                .collect(),
        })
        .collect();
    order(&mut cards);
    cards
}

/// What a card stands for.
enum Place<'a> {
    Default,
    Worktree(&'a WorktreeNode),
}

fn card(app: &State, place: &Place) -> CardModel {
    let (location, display_name, branch, kind) = match place {
        Place::Default => (
            SessionLocation::Default,
            "Default".to_string(),
            None,
            LocationType::ProjectRoot,
        ),
        Place::Worktree(node) => {
            let location = SessionLocation::Worktree(node.worktree.dir_name.clone());
            let kind = location_type(&location, node.worktree.included, node.worktree.status);
            (
                location,
                node.display_name.clone(),
                node.worktree.branch.clone(),
                kind,
            )
        }
    };
    let sessions: Vec<&Session> = app
        .active_sessions()
        .iter()
        .filter(|s| s.location == location && !s.archived)
        .collect();
    let agent_request = app
        .agent_confirm
        .pending
        .iter()
        .any(|prompt| sessions.iter().any(|s| s.id == prompt.session));
    let signals = CardSignals {
        location: location_state(sessions.iter().copied()),
        agent_request,
        pr: pr_signal(&app.pr_status, branch.as_deref()),
    };
    let (state, reason) = card_state(&signals);
    let ticket = match &location {
        SessionLocation::Worktree(dir) => crate::features::overview::numeric_ticket(dir),
        SessionLocation::Default => None,
    };
    let issue_title = match ticket.and_then(|number| app.overview.titles.get(&number)) {
        Some(crate::features::overview::TitleLookup::Known(text)) => Some(text.as_str()),
        _ => None,
    };
    let title =
        micold_core::overview_text::card_title(issue_title, branch.as_deref(), &display_name);
    let pr_open = matches!(
        pr_signal(&app.pr_status, branch.as_deref()),
        PrSignal::Known { current: Some(current), .. } if current.open
    );
    let progress = progress(app, &location, state, pr_open);
    let branch_ref = branch.clone();
    let main = main_session(app.active_sessions(), &location);
    let activity = main
        .and_then(|id| app.active_sessions().iter().find(|s| s.id == id))
        .and_then(|s| s.hook_activity.as_ref())
        .and_then(|hooks| hooks.current());
    let activation = activation(&location, main, kind);
    CardModel {
        location,
        display_name,
        branch,
        kind,
        sessions: session_counts(sessions.iter().copied()),
        state,
        reason,
        title,
        progress,
        activity,
        prs: pr_display(&app.pr_status, branch_ref),
        activation,
    }
}

/// The card's feature progress: the phase from the ledger, the card state, an open pull request
/// or the files, and the task ratio; empty without a reading (research R9, FR-017). Only a
/// worktree has files.
fn progress(app: &State, location: &SessionLocation, state: CardState, pr_open: bool) -> Progress {
    let SessionLocation::Worktree(dir) = location else {
        return Progress::default();
    };
    let files = app.overview.progress.get(dir).cloned().unwrap_or_default();
    let source = if files.ledger.is_some() {
        Source::Ledger
    } else if files.tasks.is_some() || files.artifact.is_some() {
        Source::TaskList
    } else {
        Source::None
    };
    Progress {
        phase: task_progress::phase(files.ledger, state, pr_open, files.artifact),
        ratio: files.tasks,
        source,
    }
}

/// The pull request reading for a branch: unknown while the reading is off or holds nothing (never
/// read, or released), and for a location with no branch, such as Default (research R6, R14).
fn pr_signal(pr: &crate::features::pr_status::State, branch: Option<&str>) -> PrSignal {
    match branch {
        Some(branch) if pr.enabled && pr.read_at.is_some() => PrSignal::Known {
            current: pr.statuses.get(branch).map(CurrentPr::from_status),
            removable: pr.removable.contains(branch),
        },
        _ => PrSignal::Unknown,
    }
}

/// What a card's footer shows of its branch's pull requests: nothing without a branch, else the
/// counts of the last reading or the reason there are none (FR-004).
fn pr_display(pr: &crate::features::pr_status::State, branch: Option<String>) -> PrDisplay {
    let Some(branch) = branch else {
        return PrDisplay::Hidden;
    };
    if !pr.enabled {
        return PrDisplay::Unknown(PrUnknown::Off);
    }
    if let Some(counts) = pr.counts.get(&branch) {
        return PrDisplay::Counts(*counts);
    }
    if pr.unreachable {
        return PrDisplay::Unknown(PrUnknown::Unreachable);
    }
    match pr.pause_until {
        Some(until) if until > crate::ui::sidebar::unix_now() => {
            PrDisplay::Unknown(PrUnknown::RateLimited { until })
        }
        _ => PrDisplay::Unknown(PrUnknown::NotReadYet),
    }
}

/// Which way an arrow key moves the focus: `Some(true)` forward (Right, Down), `Some(false)` back
/// (Left, Up), `None` for any other key (research R13). Here rather than in the feature module,
/// which is render-free and so cannot name the key type.
pub fn arrow_focus(key: &iced::keyboard::Key) -> Option<bool> {
    use iced::keyboard::{key::Named, Key};
    match key {
        Key::Named(Named::ArrowRight | Named::ArrowDown) => Some(true),
        Key::Named(Named::ArrowLeft | Named::ArrowUp) => Some(false),
        _ => None,
    }
}

/// What follows an activation, once the Overview has closed (research R12): the existing message
/// the sidebar sends for the same intent. A reveal-only activation sends none; an offer to start
/// sends the start menu's, which the root anchors first (it needs the row's position). Named here
/// because it names the session feature's messages, which the Overview feature module may not.
pub fn follow_up(activation: &micold_core::overview::Activation) -> Option<Message> {
    use crate::features::session::Msg as SessionMsg;
    use micold_core::overview::Activation;
    match activation {
        Activation::OpenSession(id) => Some(Message::Session(SessionMsg::Selected(*id))),
        Activation::OfferStart(location) => Some(Message::Session(SessionMsg::StartMenuOpened {
            location: location.clone(),
            unavailable_default: None,
        })),
        Activation::RevealOnly(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::test_support::{Mounted, WINDOW};
    use iced::advanced::widget::operation::Outcome;
    use iced::advanced::widget::Operation;
    use iced::{Rectangle, Size};

    /// U1: the Overview outranks Changes and Compare in the crossfade key, and Settings outranks it.
    #[test]
    fn the_overview_key_sits_between_settings_and_changes() {
        let mut state = State::default();
        state.overview.open = true;
        assert_eq!(crate::ui::main_content_key(&state), u64::MAX - 3);
        state.settings.settings_draft = Some(Default::default());
        assert_eq!(
            crate::ui::main_content_key(&state),
            u64::MAX,
            "Settings first"
        );
        state.settings.settings_draft = None;
        let overview = crate::ui::main_content_key(&state);
        state.update(Message::Changes(crate::features::changes::Msg::Opened {
            project: "/p".into(),
            entry: SessionLocation::Default,
        }));
        let changes = {
            state.overview.open = false;
            crate::ui::main_content_key(&state)
        };
        assert_ne!(overview, changes, "Overview outranks Changes");
        state.changes.open = None;
        state.runs.compare = Some(crate::features::runs::CompareView {
            group: micold_core::runs::GroupId(uuid::Uuid::nil()),
            project: "/p".into(),
            reads: Default::default(),
        });
        let compare = crate::ui::main_content_key(&state);
        assert_ne!(overview, compare, "Overview outranks Compare");
        assert_ne!(changes, compare);
    }

    /// Every scrollable under the element, as (viewport, content) bounds.
    struct FindScrollables(Vec<(Rectangle, Rectangle)>);

    impl Operation<Vec<(Rectangle, Rectangle)>> for FindScrollables {
        fn traverse(
            &mut self,
            operate: &mut dyn FnMut(&mut dyn Operation<Vec<(Rectangle, Rectangle)>>),
        ) {
            operate(self);
        }

        fn scrollable(
            &mut self,
            _id: Option<&iced::widget::Id>,
            bounds: Rectangle,
            content_bounds: Rectangle,
            _translation: iced::Vector,
            _state: &mut dyn iced::advanced::widget::operation::Scrollable,
        ) {
            self.0.push((bounds, content_bounds));
        }

        fn finish(&self) -> Outcome<Vec<(Rectangle, Rectangle)>> {
            Outcome::Some(self.0.clone())
        }
    }

    /// F1: thirty cards in a short window scroll inside the grid, not the page. Built through the
    /// real `view` and asked, as the runtime would, which scrollables it holds: the containers
    /// and the responsive wrapper around the grid are `Fill` whatever their content does, so
    /// their size cannot show whether the grid scrolls itself; only the scrollable can.
    #[test]
    fn thirty_cards_lay_out_bounded_in_a_short_window() {
        use micold_core::project::{Availability, Project};
        use micold_core::worktree::{Worktree, WorktreeStatus};
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state
            .workspace
            .projects
            .push(Project::new(path.clone(), true, Availability::Available));
        state.workspace.active = Some(path);
        state.worktree.worktrees = (0..30)
            .map(|i| Worktree {
                dir_name: format!("feat-{i}"),
                path: format!("/repo/.claude/worktrees/feat-{i}").into(),
                branch: Some(format!("feat/{i}")),
                status: WorktreeStatus::Valid,
                included: false,
            })
            .collect();
        assert_eq!(
            cards(&state).len(),
            31,
            "the window holds 30 cards and the root"
        );
        let mut mounted = Mounted::new(view(&state, ColorScheme::Dark).map(|_| String::new()));
        let scrollables = mounted
            .drive(FindScrollables(Vec::new()))
            .expect("the operation finishes");
        let [(viewport, content)] = scrollables[..] else {
            panic!("the grid must hold exactly one scrollable of its own: {scrollables:?}");
        };
        assert!(
            viewport.height <= WINDOW.height + 0.01,
            "the grid grew to {viewport:?} in {WINDOW:?}"
        );
        assert!(
            content.height > viewport.height,
            "thirty cards fit without scrolling: {content:?} in {viewport:?}"
        );
    }

    /// Edge case: after a disconnect the window says the cards may be stale. The strip that says
    /// so is the connection banner, empty while connected and present once the link is lost. This
    /// pins the banner only; that the root view places it above the Overview is not asserted.
    #[test]
    fn the_connection_banner_has_height_only_when_disconnected() {
        use crate::features::connection::connection_status;
        let room = Size::new(900.0, 300.0);
        let roles = crate::ui::material::test_support::roles();
        let strip = |disconnected: bool| {
            let status = connection_status(None, None, None, disconnected);
            crate::ui::material::test_support::size_of(
                crate::ui::connection_banner(&status, roles),
                room,
            )
            .height
        };
        assert_eq!(strip(false), 0.0, "no strip while connected");
        assert!(
            strip(true) > 0.0,
            "a disconnect must show the stale-data strip"
        );
    }

    /// T008 (FR-008): a card's press asks for the activation; the delay and the close follow in
    /// the root.
    #[test]
    fn a_card_press_requests_its_activation() {
        use micold_core::project::{Availability, Project};
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state
            .workspace
            .projects
            .push(Project::new(path.clone(), true, Availability::Available));
        state.workspace.active = Some(path);
        state.overview.open = true;
        let expected = cards(&state)[0].activation.clone();
        let mut mounted = Mounted::new(view(&state, ColorScheme::Dark).map(|m| format!("{m:?}")));
        // Below the header row (feature 970), inside the first card.
        let at = iced::Point::new(100.0, 120.0);
        mounted.press(at);
        let published = mounted.send(
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
            iced::mouse::Cursor::Available(at),
        );
        assert_eq!(
            published,
            [format!(
                "{:?}",
                Message::Overview(crate::features::overview::Msg::ActivationRequested(
                    expected
                ))
            )]
        );
    }
    // Feature 970 (FR-012, contracts/ui.md *Overview header*): the full-screen toggle's words and
    // glyph name its action, and the tooltip carries the platform's chord.
    #[test]
    fn the_full_screen_toggle_names_its_action_and_chord() {
        assert_eq!(full_screen_label(false), "Full screen");
        assert_eq!(full_screen_label(true), "Exit full screen");
        for on in [false, true] {
            assert_eq!(
                full_screen_tooltip(on),
                format!(
                    "{} ({})",
                    full_screen_label(on),
                    crate::keymap::overview_full_screen_chord_label()
                )
            );
        }
        assert_eq!(full_screen_icon(false), Icon::FullScreen);
        assert_eq!(full_screen_icon(true), Icon::FullScreenExit);
    }

    /// Spec edge case "Several sessions" (feature 1120): the card shows its main session's
    /// activity only. With nothing readable there it shows no line, never a helper session's.
    #[test]
    fn the_activity_line_never_falls_back_to_another_sessions_prompt() {
        use micold_core::hook_activity::HookActivity;
        use micold_core::project::{Availability, Project};
        use micold_core::session::{AiCli, SessionLifecycle};
        use micold_core::worktree::{Worktree, WorktreeStatus};
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state
            .workspace
            .projects
            .push(Project::new(path.clone(), true, Availability::Available));
        state.workspace.active = Some(path.clone());
        state.worktree.worktrees = vec![Worktree {
            dir_name: "feat-a".into(),
            path: "/repo/.claude/worktrees/feat-a".into(),
            branch: Some("feat/a".into()),
            status: WorktreeStatus::Valid,
            included: false,
        }];
        let location = SessionLocation::Worktree("feat-a".into());
        let session = |prompt: Option<&str>| {
            let mut s = Session::start_new(location.clone(), AiCli::ClaudeCode);
            s.lifecycle = SessionLifecycle::Running;
            s.hook_activity = Some(HookActivity {
                todos: Vec::new(),
                prompt: prompt.map(str::to_string),
            });
            s
        };
        state
            .workspace
            .sessions
            .insert(path, vec![session(None), session(Some("Helper prompt"))]);
        let all = cards(&state);
        let card = all
            .iter()
            .find(|c| c.location == location)
            .expect("the worktree's card");
        assert_eq!(card.activity, None);
    }
}
