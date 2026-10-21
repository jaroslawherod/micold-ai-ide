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
use iced::advanced::widget::Operation;
use iced::widget::{column, container, row, Space};
use iced::{Element, Length};
use micold_core::issue_link::IssueLink;
use micold_core::location_state::location_state;
use micold_core::occupancy::main_session;
use micold_core::overview::{
    activation, card_state, location_type, order, session_counts, CardModel, CardSignals,
    CardState, CurrentPr, LocationType, PrDisplay, PrSignal, PrUnknown,
};
use micold_core::overview_expansion::{page_target, Event, Expansion};
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
    card_part_id("overview-card", location)
}

/// The expanded card's inner scrollable, by location (feature 1121, FR-008): Page Up and Page Down
/// scroll it while the card has the keyboard.
pub fn card_scroll_id(location: &SessionLocation) -> iced::advanced::widget::Id {
    card_part_id("overview-card-body", location)
}

/// A card's issue link, by location (feature 1122): the grid finds the link under the pointer by
/// it, and the arrow keys step over it.
pub fn card_link_id(location: &SessionLocation) -> iced::advanced::widget::Id {
    card_part_id("overview-card-link", location)
}

/// The id of one part of a card (review A F2, M3): the worktree's directory name follows a `/`,
/// which no directory name holds, and Default's id has none, so no worktree's name (`link-x`,
/// `default`) can make one part's id equal another card's or another part's.
fn card_part_id(part: &str, location: &SessionLocation) -> iced::advanced::widget::Id {
    iced::advanced::widget::Id::from(match location {
        SessionLocation::Default => part.to_string(),
        SessionLocation::Worktree(dir) => format!("{part}/{dir}"),
    })
}

/// Scroll the scrollable named `id` by one of its own visible heights, down or up (feature 1121,
/// FR-008, research R6): Page Down / Page Up on the expanded card's body. Its own operation rather
/// than `scroll_by`, because only the scrollable knows how tall its view is.
pub fn page_body<T>(id: iced::advanced::widget::Id, down: bool) -> impl Operation<T> {
    struct PageBody {
        id: iced::advanced::widget::Id,
        down: bool,
    }
    impl<T> Operation<T> for PageBody {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }

        fn scrollable(
            &mut self,
            id: Option<&iced::advanced::widget::Id>,
            bounds: iced::Rectangle,
            content_bounds: iced::Rectangle,
            _translation: iced::Vector,
            state: &mut dyn iced::advanced::widget::operation::Scrollable,
        ) {
            if id == Some(&self.id) {
                let page = if self.down {
                    bounds.height
                } else {
                    -bounds.height
                };
                state.scroll_by(
                    iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: page },
                    bounds,
                    content_bounds,
                );
            }
        }
    }
    PageBody { id, down }
}

/// The Overview: a grid with one card per location of the active project.
pub fn view<'a>(state: &State, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let menu_open = state.overview.card_menu.is_some();
    let models = cards(state);
    let expanded_at = state
        .overview
        .expansion
        .expanded()
        .and_then(|at| models.iter().position(|model| &model.location == at));
    // Page Up / Page Down go to the expanded card only while `page_target` gives them to it (K4).
    let paging = |down: bool| page_target(&state.overview.expansion, down).is_some();
    let paging = (paging(false), paging(true));
    let cards = models
        .iter()
        .enumerate()
        .map(|(index, model)| {
            let location = model.location.clone();
            let menu_at = location.clone();
            let expansion = |event: Event| Message::Overview(Msg::Expansion(event));
            let card = OverviewCard::new(model, r)
                .id(card_id(&location))
                .targeted(is_targeted(state, &location))
                .menu_open(menu_open)
                .expanded(expanded_at == Some(index))
                .on_rest(expansion(Event::PointerRested(location.clone())))
                .on_leave(expansion(Event::PointerLeft(location.clone())))
                // Feature 1122: the issue link opens its address; a rest on it expands nothing
                // (FR-007). It is the Tab stop after its card, and focus on it counts as focus on
                // the card (FR-008); its Menu key opens the card's menu from it (FR-009).
                .on_issue_link(Message::Overview(Msg::IssueLinkActivated(location.clone())))
                .link_id(card_link_id(&location))
                .on_link_rest(expansion(Event::LinkRested(location.clone())))
                .on_link_keyboard_focus({
                    let location = location.clone();
                    move |shown| {
                        expansion(if shown {
                            Event::LinkFocusGained(location.clone())
                        } else {
                            Event::LinkFocusLost(location.clone())
                        })
                    }
                })
                .on_link_menu_key(Message::Overview(Msg::LinkMenuRequested(location.clone())))
                .scroll_id(card_scroll_id(&location))
                .on_body_scroll(move |up, down| expansion(Event::BodyScrolled { up, down }))
                .on_keyboard_focus({
                    let location = location.clone();
                    move |shown| {
                        expansion(if shown {
                            Event::FocusGained(location.clone())
                        } else {
                            Event::FocusLost(location.clone())
                        })
                    }
                })
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
                });
            if expanded_at == Some(index) && (paging.0 || paging.1) {
                card.on_page(move |down| {
                    let claimed = if down { paging.1 } else { paging.0 };
                    claimed.then_some(Message::Overview(Msg::CardPaged { down }))
                })
            } else {
                card
            }
        })
        .collect();
    // The grid scrolls itself; an outer scrollable would give it unbounded height.
    let mut grid = CardGrid::new(cards, r);
    if let Some(index) = expanded_at {
        // The twin: the same card collapsed, with no id or messages, measured for its row (U2).
        grid = grid.expanded(index, OverviewCard::new(&models[index], r));
    }
    let grid = grid
        .on_scroll(Message::Overview(Msg::GridScrolled))
        // The window losing focus collapses the expanded card; regaining it expands none (FR-005).
        .on_window_blur(Message::Overview(Msg::Expansion(Event::WindowBlurred)))
        // The Default card is at the same location in every project: a switch rests anew.
        .rest_scope(&state.workspace.active)
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

/// The card the Overview's opening puts the keyboard on, its first (research R13); None with no
/// cards. Tells the expansion first, so that focus expands nothing (FR-013, review A F1, M4).
pub fn open_focus(state: &mut State) -> Option<SessionLocation> {
    let first = cards(state).into_iter().next()?.location;
    state.update(Message::Overview(Msg::Expansion(Event::OpenFocused(
        first.clone(),
    ))));
    Some(first)
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
        // Closing the Overview collapses the card, and reopening expands none (FR-005, FR-013).
        state.overview.expansion = Expansion::default();
        return;
    }
    let target = state
        .overview
        .card_menu
        .as_ref()
        .map(|m| m.location.clone());
    // `cards()` derives every card: only when a menu, an expanded card or a focused link (feature
    // 1122) may have lost its card.
    let expansion = &state.overview.expansion;
    let watched = expansion.expanded().is_some() || expansion.focused_link().is_some();
    if target.is_none() && !watched {
        return;
    }
    let listed: Vec<SessionLocation> = cards(state).into_iter().map(|m| m.location).collect();
    if watched {
        state
            .overview
            .expansion
            .apply(Event::Listed(listed.clone()));
    }
    if target.is_some_and(|target| !listed.contains(&target)) {
        state.overview.card_menu = None;
        state.overview.focus_return = None;
        state.overview.expansion.apply(Event::MenuClosed);
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

/// The issue link of the card at `location`, from what the window holds now (feature 1122, FR-001,
/// R10): `None` on *Default*, for a name with no ticket, and while its source is unknown or off.
pub fn issue_link_for(state: &State, location: &SessionLocation) -> Option<IssueLink> {
    let SessionLocation::Worktree(dir) = location else {
        return None;
    };
    let github = match &state.overview.repo {
        crate::features::overview::RepoLookup::Known(repo) => repo.as_ref(),
        _ => None,
    };
    micold_core::issue_link::issue_link(
        dir,
        &micold_core::issue_link::IssueSources {
            github,
            github_on: state.settings.github_issues_enabled,
            // The open project's own base URL (feature 1122, D4, FR-010).
            jira_base: state.workspace.active.as_deref().and_then(|project| {
                state
                    .settings
                    .jira_base_urls
                    .get(&micold_core::jira::connection_key(project))
            }),
            jira_on: state.settings.jira_enabled,
        },
    )
}

/// Move the keyboard to the next (`forward`) or previous card of `cards` (feature 1122, R9,
/// contract U4), counting from the focused card or from the card whose issue link is focused. It
/// never stops on a link. Past the first or the last card it does what iced's `focus_previous` /
/// `focus_next` do there; with nothing focused it takes the first or the last card.
pub fn step_card<T: Send + 'static>(forward: bool, cards: &[SessionLocation]) -> impl Operation<T> {
    use iced::advanced::widget::operation::{Focusable, Outcome};
    use iced::advanced::widget::Id;

    /// The first pass: every focusable in order, and which one holds the keyboard.
    struct StepCard {
        forward: bool,
        /// Each card's id and its link's id.
        cards: Vec<(Id, Id)>,
        seen: Vec<Option<Id>>,
        focused: Option<usize>,
    }

    impl StepCard {
        fn is_link(&self, at: usize) -> bool {
            self.seen[at]
                .as_ref()
                .is_some_and(|id| self.cards.iter().any(|(_, link)| link == id))
        }

        /// Where the step counts from: the focused card, or the card of the focused link.
        fn from(&self) -> Option<usize> {
            let at = self.focused?;
            let card = self.seen[at]
                .as_ref()
                .and_then(|id| self.cards.iter().find(|(_, link)| link == id))
                .and_then(|(card, _)| self.seen.iter().position(|s| s.as_ref() == Some(card)));
            Some(card.unwrap_or(at))
        }
    }

    impl<T: Send + 'static> Operation<T> for StepCard {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }

        fn focusable(
            &mut self,
            id: Option<&Id>,
            _bounds: iced::Rectangle,
            state: &mut dyn Focusable,
        ) {
            if state.is_focused() {
                self.focused = Some(self.seen.len());
            }
            self.seen.push(id.cloned());
        }

        fn finish(&self) -> Outcome<T> {
            let from = self.from();
            let target = if self.forward {
                let start = from.map_or(0, |at| at + 1);
                (start..self.seen.len()).find(|&at| !self.is_link(at))
            } else {
                let end = from.unwrap_or(self.seen.len());
                (0..end).rev().find(|&at| !self.is_link(at))
            };
            Outcome::Chain(Box::new(FocusAt { target, current: 0 }))
        }
    }

    /// The second pass: focus the `target`-th focusable, unfocus every other.
    struct FocusAt {
        target: Option<usize>,
        current: usize,
    }

    impl<T> Operation<T> for FocusAt {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }

        fn focusable(
            &mut self,
            _id: Option<&Id>,
            _bounds: iced::Rectangle,
            state: &mut dyn Focusable,
        ) {
            if self.target == Some(self.current) {
                state.focus();
            } else {
                state.unfocus();
            }
            self.current += 1;
        }
    }

    StepCard {
        forward,
        cards: cards
            .iter()
            .map(|location| (card_id(location), card_link_id(location)))
            .collect(),
        seen: Vec::new(),
        focused: None,
    }
}

/// What a card stands for.
enum Place<'a> {
    Default,
    Worktree(&'a WorktreeNode),
}

fn card(app: &State, place: &Place) -> CardModel {
    let (location, display_name, branch, kind, dir) = match place {
        Place::Default => (
            SessionLocation::Default,
            "Default".to_string(),
            None,
            LocationType::ProjectRoot,
            app.workspace.active.clone().unwrap_or_default(),
        ),
        Place::Worktree(node) => {
            let location = SessionLocation::Worktree(node.worktree.dir_name.clone());
            let kind = location_type(&location, node.worktree.included, node.worktree.status);
            (
                location,
                node.display_name.clone(),
                node.worktree.branch.clone(),
                kind,
                node.worktree.path.clone(),
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
    let issue_link = issue_link_for(app, &location);
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
        dir,
        issue_link,
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
    use iced::Size;

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
        let scrollables = mounted.scrollables();
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

    /// Feature 1121 T032, FR-005 (review A F4, B F1, M3): with a card expanded, the window losing
    /// focus makes the mounted Overview publish `Expansion(WindowBlurred)`, the event that
    /// collapses it; regaining focus publishes nothing (Edge Case *Window regains focus*).
    #[test]
    fn the_window_losing_focus_publishes_the_expansion_blur() {
        use micold_core::overview_expansion::Event;
        use micold_core::project::{Availability, Project};
        use micold_core::session::SessionLocation;
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state
            .workspace
            .projects
            .push(Project::new(path.clone(), true, Availability::Available));
        state.workspace.active = Some(path);
        state.overview.open = true;
        state
            .overview
            .expansion
            .apply(Event::FocusGained(SessionLocation::Default));
        let mut mounted = Mounted::new(view(&state, ColorScheme::Dark).map(|m| format!("{m:?}")));
        let away = iced::mouse::Cursor::Unavailable;
        let blur = iced::Event::Window(iced::window::Event::Unfocused);
        assert_eq!(
            mounted.send(blur, away),
            [format!(
                "{:?}",
                Message::Overview(crate::features::overview::Msg::Expansion(
                    Event::WindowBlurred
                ))
            )]
        );
        let focus = iced::Event::Window(iced::window::Event::Focused);
        assert!(mounted.send(focus, away).is_empty());
    }

    /// An open Overview of the root and `n` worktrees.
    fn overview_of(n: usize) -> State {
        use micold_core::project::{Availability, Project};
        use micold_core::worktree::{Worktree, WorktreeStatus};
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state
            .workspace
            .projects
            .push(Project::new(path.clone(), true, Availability::Available));
        state.workspace.active = Some(path);
        state.worktree.worktrees = (0..n)
            .map(|i| Worktree {
                dir_name: format!("feat-{i}"),
                path: format!("/repo/.claude/worktrees/feat-{i}").into(),
                branch: Some(format!("feat/{i}")),
                status: WorktreeStatus::Valid,
                included: false,
            })
            .collect();
        state.overview.open = true;
        state
    }

    fn mount(state: &State) -> Mounted<'static> {
        Mounted::new(view(state, ColorScheme::Dark).map(|m| format!("{m:?}")))
    }

    fn expansion_message(event: Event) -> String {
        format!(
            "{:?}",
            Message::Overview(crate::features::overview::Msg::Expansion(event))
        )
    }

    /// The focus messages of `moves` Tab moves through the mounted Overview, in order.
    fn focus_moves(mounted: &mut Mounted<'_>, moves: usize) -> Vec<String> {
        (0..moves)
            .flat_map(|_| {
                mounted.focus_next();
                let mut reported = mounted.redraw();
                // Within one frame, the card that lost focus first.
                reported.sort_by_key(|m| !m.contains("FocusLost"));
                reported
            })
            .filter(|m| m.contains("Focus"))
            .collect()
    }

    /// Feature 1121 T038, Story 3 sc.1/sc.3, FR-002: a Tab move onto a card reports its focus to
    /// the expansion, and moving on reports the loss and the next card's gain.
    #[test]
    fn tab_moves_report_card_focus_to_the_expansion() {
        let state = overview_of(1);
        let locations: Vec<_> = cards(&state).into_iter().map(|c| c.location).collect();
        let mut mounted = mount(&state);
        let moves = focus_moves(&mut mounted, 8);
        let first = expansion_message(Event::FocusGained(locations[0].clone()));
        let at = moves
            .iter()
            .position(|m| *m == first)
            .unwrap_or_else(|| panic!("no card took focus: {moves:?}"));
        assert_eq!(
            moves[at + 1..at + 3],
            [
                expansion_message(Event::FocusLost(locations[0].clone())),
                expansion_message(Event::FocusGained(locations[1].clone())),
            ]
        );
    }

    /// T038, FR-002: a press on a card takes the keyboard but reports no focus: nothing expands.
    #[test]
    fn a_press_on_a_card_reports_no_focus() {
        let mut mounted = mount(&overview_of(1));
        let at = iced::Point::new(100.0, 120.0);
        let mut published = mounted.press(at);
        published.extend(mounted.redraw());
        assert!(
            !published.iter().any(|m| m.contains("Focus")),
            "{published:?}"
        );
    }

    /// T038, Edge Case (feature 972 FR-011): the focus hand-back to a card, by its id, reports
    /// that card's focus, so it expands.
    #[test]
    fn a_focus_hand_back_reports_the_cards_focus() {
        let state = overview_of(2);
        let target = SessionLocation::Worktree("feat-1".into());
        let mut mounted = mount(&state);
        mounted.drive(iced::advanced::widget::operation::focusable::focus::<()>(
            card_id(&target),
        ));
        assert_eq!(
            mounted.redraw(),
            [expansion_message(Event::FocusGained(target))]
        );
    }

    /// Feature 1122 FR-008, FR-009 (D3): on the mounted Overview, Tab from a card with an issue
    /// link reports the card's loss and the link's gain to the expansion; the link's Menu key asks
    /// for the card's menu from the link; Tab on reports the link's loss.
    #[test]
    fn a_cards_link_reports_its_focus_and_menu_key() {
        use crate::ui::material::test_support::key_with;
        use iced::keyboard::{key::Named, Modifiers};
        let mut state = overview_of(2);
        state.worktree.worktrees[1].dir_name = "feat-1122_x".into();
        state.settings.github_issues_enabled = true;
        state.overview.repo = crate::features::overview::RepoLookup::Known(Some(
            micold_core::github::GithubRepo::from_remote_url("git@github.com:o/r.git")
                .expect("a GitHub remote"),
        ));
        let linked = cards(&state)
            .into_iter()
            .find(|card| card.issue_link.is_some())
            .expect("a worktree card has its issue link")
            .location;
        let mut mounted = mount(&state);
        mounted.drive(iced::advanced::widget::operation::focusable::focus::<()>(
            card_id(&linked),
        ));
        mounted.redraw();
        let tabbed = focus_moves(&mut mounted, 1);
        assert_eq!(
            tabbed,
            [
                expansion_message(Event::FocusLost(linked.clone())),
                expansion_message(Event::LinkFocusGained(linked.clone())),
            ]
        );
        let none = iced::mouse::Cursor::Unavailable;
        let request = format!(
            "{:?}",
            Message::Overview(Msg::LinkMenuRequested(linked.clone()))
        );
        for key in [
            key_with(Named::ContextMenu, Modifiers::empty()),
            key_with(Named::F10, Modifiers::SHIFT),
        ] {
            assert_eq!(mounted.send(key, none), std::slice::from_ref(&request));
        }
        let on = focus_moves(&mut mounted, 1);
        assert_eq!(
            on.first(),
            Some(&expansion_message(Event::LinkFocusLost(linked)))
        );
    }

    /// Review A F1 (M4), FR-013, R13: the opening's focus on the first card, run on the mounted
    /// Overview, reports that card's focus, and the expansion that takes it in expands nothing.
    #[test]
    fn the_opening_focus_expands_no_card() {
        let mut state = overview_of(2);
        let first = open_focus(&mut state).expect("an Overview with cards has a first");
        let mut mounted = mount(&state);
        mounted.drive(iced::advanced::widget::operation::focusable::focus::<()>(
            card_id(&first),
        ));
        let gained = Event::FocusGained(first);
        assert_eq!(mounted.redraw(), [expansion_message(gained.clone())]);
        state.update(Message::Overview(
            crate::features::overview::Msg::Expansion(gained),
        ));
        assert_eq!(state.overview.expansion.expanded(), None);
    }

    /// T038, US4 sc.4: Tab visits the cards in the same order with a card expanded as without.
    #[test]
    fn tab_order_is_the_same_with_a_card_expanded() {
        let gained = |state: &State| -> Vec<String> {
            focus_moves(&mut mount(state), 12)
                .into_iter()
                .filter(|m| m.contains("FocusGained"))
                .collect()
        };
        let plain = overview_of(3);
        let mut expanded = plain.clone();
        expanded
            .overview
            .expansion
            .apply(Event::PointerRested(SessionLocation::Worktree(
                "feat-1".into(),
            )));
        let order = gained(&plain);
        assert!(order.len() >= 4, "every card takes focus: {order:?}");
        assert_eq!(gained(&expanded), order);
    }

    /// T038, FR-008: a focused card expanded by focus, whose body can scroll down, sends
    /// `CardPaged { down: true }` on Page Down; at its top, Page Up is not claimed.
    #[test]
    fn page_down_on_a_focus_expanded_card_pages_it() {
        use iced::keyboard::key::Named;
        let mut state = overview_of(0);
        state
            .overview
            .expansion
            .apply(Event::FocusGained(SessionLocation::Default));
        state.overview.expansion.apply(Event::BodyScrolled {
            up: false,
            down: true,
        });
        let mut mounted = mount(&state);
        mounted.drive(iced::advanced::widget::operation::focusable::focus::<()>(
            card_id(&SessionLocation::Default),
        ));
        mounted.redraw();
        let cursor = iced::mouse::Cursor::Unavailable;
        use crate::ui::material::test_support::named_key;
        assert_eq!(
            mounted.send_captured(named_key(Named::PageDown), cursor),
            (
                vec![format!(
                    "{:?}",
                    Message::Overview(crate::features::overview::Msg::CardPaged { down: true })
                )],
                true
            )
        );
        assert!(!mounted.send_captured(named_key(Named::PageUp), cursor).1);
    }

    /// T043, R6: the page operation scrolls the named scrollable by one of its own visible
    /// heights, down or up, and leaves any other scrollable alone.
    #[test]
    fn a_page_scrolls_the_named_body_by_its_visible_height() {
        use crate::ui::material::Scrollable;
        use iced::widget::text;
        let id = iced::advanced::widget::Id::new("paged");
        let tall = column((0..60).map(|i| text(format!("line {i}")).into()));
        let body = Scrollable::new(tall, tokens::roles(ColorScheme::Dark))
            .id(id.clone())
            .on_scroll_metrics(|offset, view, _| format!("{offset} {view}"))
            .height(Length::Fixed(200.0));
        let mut mounted = Mounted::new(body);
        let metrics = |mounted: &mut Mounted<'_>| mounted.redraw().pop();
        let before = metrics(&mut mounted).expect("an overflowing body reports");
        assert_eq!(before, "0 200");
        mounted.drive(page_body::<()>(id.clone(), true));
        assert_eq!(metrics(&mut mounted).as_deref(), Some("200 200"));
        mounted.drive(page_body::<()>(id.clone(), false));
        assert_eq!(metrics(&mut mounted).as_deref(), Some("0 200"));
        mounted.drive(page_body::<()>(
            iced::advanced::widget::Id::new("other"),
            true,
        ));
        assert_eq!(metrics(&mut mounted), None, "another id moved it");
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
        state.workspace.sessions.insert(
            path,
            vec![
                session(Some(r#"<agent-message from="x"></agent-message>"#)),
                session(Some("Helper prompt")),
            ],
        );
        let all = cards(&state);
        let card = all
            .iter()
            .find(|c| c.location == location)
            .expect("the worktree's card");
        assert_eq!(
            card.activity.as_ref().map(|a| a.line.as_str()),
            Some(r#"<agent-message from="x"></agent-message>"#),
            "the main session's activity, not the helper's"
        );
        assert_eq!(
            micold_core::overview_text::activity_line(card.activity.as_ref()),
            None
        );
    }

    // --- Feature 1122, contract U4: the arrows step over issue links -------------------------

    /// The Overview's three cards (Default and two worktrees), the middle one with an issue link
    /// that takes Tab focus (as it does from M4), mounted as a grid.
    fn stepping_grid() -> (Vec<SessionLocation>, Mounted<'static>) {
        let mut models = cards(&overview_of(2));
        models[1].issue_link = Some(IssueLink {
            label: "Issue #1".into(),
            address: "https://github.com/o/r/issues/1".into(),
        });
        let locations: Vec<SessionLocation> = models.iter().map(|m| m.location.clone()).collect();
        let r = tokens::roles(ColorScheme::Dark);
        let grid: Vec<OverviewCard<'static, String>> = models
            .iter()
            .map(|model| {
                OverviewCard::new(model, r)
                    .id(card_id(&model.location))
                    .on_press("open".to_string())
                    .on_issue_link("link".to_string())
                    .link_id(card_link_id(&model.location))
                    .on_link_keyboard_focus(|_| String::new())
            })
            .collect();
        (locations, Mounted::new(CardGrid::new(grid, r)))
    }

    fn focus_on(mounted: &mut Mounted<'_>, id: iced::advanced::widget::Id) {
        mounted.drive(iced::advanced::widget::operation::focusable::focus::<()>(
            id,
        ));
    }

    fn focused(mounted: &mut Mounted<'_>) -> Option<iced::advanced::widget::Id> {
        mounted.drive(iced::advanced::widget::operation::focusable::find_focused())
    }

    /// What `step_card` focuses from `from`.
    fn step_from(
        from: iced::advanced::widget::Id,
        forward: bool,
    ) -> Option<iced::advanced::widget::Id> {
        let (locations, mut mounted) = stepping_grid();
        focus_on(&mut mounted, from);
        mounted.drive(step_card::<()>(forward, &locations));
        focused(&mut mounted)
    }

    /// U4, FR-008: from a card or from its link the arrows go to the next or previous card, never
    /// onto a link.
    #[test]
    fn the_arrows_step_between_cards_and_never_onto_a_link() {
        let (l, _) = stepping_grid();
        assert_eq!(step_from(card_id(&l[0]), true), Some(card_id(&l[1])));
        assert_eq!(step_from(card_id(&l[1]), true), Some(card_id(&l[2])));
        assert_eq!(step_from(card_link_id(&l[1]), true), Some(card_id(&l[2])));
        assert_eq!(step_from(card_link_id(&l[1]), false), Some(card_id(&l[0])));
        assert_eq!(step_from(card_id(&l[2]), false), Some(card_id(&l[1])));
        assert_eq!(step_from(card_id(&l[1]), false), Some(card_id(&l[0])));
    }

    /// Review A F2 (M3): no worktree's name makes a card's id equal another card's link or body,
    /// or Default's.
    #[test]
    fn no_worktree_name_makes_one_cards_id_another_cards_part() {
        let worktree = |dir: &str| SessionLocation::Worktree(dir.into());
        let ids = |location: &SessionLocation| {
            [
                card_id(location),
                card_scroll_id(location),
                card_link_id(location),
            ]
        };
        let locations = [
            SessionLocation::Default,
            worktree("default"),
            worktree("x"),
            worktree("link-x"),
            worktree("body-x"),
            worktree("link-default"),
        ];
        let all: Vec<_> = locations.iter().flat_map(ids).collect();
        for (at, id) in all.iter().enumerate() {
            assert!(!all[at + 1..].contains(id), "{id:?} names two parts");
        }
    }

    /// U4: past the first card backward and the last forward, it does what iced's
    /// `focus_previous` / `focus_next` did there.
    #[test]
    fn past_the_ends_the_arrows_do_what_the_traversal_did() {
        use iced::advanced::widget::operation::focusable::{focus_next, focus_previous};
        let (l, _) = stepping_grid();
        let traversed = |from: iced::advanced::widget::Id, forward: bool| {
            let (_, mut mounted) = stepping_grid();
            focus_on(&mut mounted, from);
            if forward {
                mounted.drive(focus_next::<()>());
            } else {
                mounted.drive(focus_previous::<()>());
            }
            focused(&mut mounted)
        };
        assert_eq!(
            step_from(card_id(&l[0]), false),
            traversed(card_id(&l[0]), false)
        );
        assert_eq!(
            step_from(card_id(&l[2]), true),
            traversed(card_id(&l[2]), true)
        );
        // With nothing focused: the first card forward, the last backward.
        let (locations, mut mounted) = stepping_grid();
        mounted.drive(step_card::<()>(true, &locations));
        assert_eq!(focused(&mut mounted), Some(card_id(&l[0])));
        let (locations, mut mounted) = stepping_grid();
        mounted.drive(step_card::<()>(false, &locations));
        assert_eq!(focused(&mut mounted), Some(card_id(&l[2])));
    }
}
