//! The session area: what the main area draws while a session is shown (feature 1126, contract
//! A1, research R12). Top to bottom: the context header naming the shown location, the tab strip,
//! then the pane area; nothing under it. The header's words come from
//! `micold_core::context_header` and the strip's members from `features::session_strip`; this
//! module only lays them out (contracts H9, S7).

use crate::app::{Message, State};
use crate::features::session::Msg as SessionMsg;
use crate::features::session_strip::{strip_overflow, Member, StripModel};
use crate::icons::Icon;
use crate::ui::cdk::keyed::Keyed;
use crate::ui::material::{
    self, card_palette, card_state_icon, tab, tab_content_colour, ActivityBadge, BadgeEmphasis,
    Ellipsized, Glyph, IconButton, IndicatorEdge, PullRequestIndicator, SurfaceKind, Tab, Tag,
    Text, Tooltip, TooltipPosition, TypeRole,
};
use crate::ui::terminal;
use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length, Padding};
use micold_core::context_header::{Chip, CountsReading, HeaderText, PrChipInput};
use micold_core::review_tabs::ChangesButton;
use micold_core::session::SessionLocation;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Roles};

/// The widest a header chip grows before its text is cut with an ellipsis (FR-013); its full text
/// is in its tooltip.
pub const CHIP_MAX_WIDTH: f32 = 240.0;

/// The widest the strip's status words grow before they are cut with an ellipsis (S7); the full
/// words are in their tooltip.
pub const STATUS_MAX_WIDTH: f32 = 160.0;

/// The session area for the shown session: the header, the strip, then the pane area (A1, A2). The
/// header's and the strip's slots are there even when they have nothing to draw, empty, so the pane
/// area's place in the tree never moves (feature 023 FR-008a).
#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    state: &'a State,
    area: Option<&crate::ui::panes::PaneArea<'a>>,
    grid: Option<&'a crate::grid::GridCache>,
    selection: Option<&'a crate::selection::Selection>,
    display_offset: usize,
    scheme: ColorScheme,
    link_context: micold_core::link::LinkContext,
) -> Element<'a, Message> {
    let header: Element<'a, Message> = match state.current_session_location() {
        Some(location) => header(state, &location, scheme),
        None => container(Space::new()).into(),
    };
    let strip: Element<'a, Message> = match StripModel::of(state) {
        Some(model) => strip(state, &model, area, scheme),
        None => container(Space::new()).into(),
    };
    column![
        header,
        strip,
        terminal::pane(
            state,
            area,
            grid,
            selection,
            display_offset,
            scheme,
            link_context,
        )
    ]
    .into()
}

/// The shown location's current pull request, as the sidebar's indicator reads it (FR-005, R4):
/// `None` without a branch, while pull request status is off, or with no status for the branch.
pub(crate) fn pr_chip_input(state: &State, branch: Option<&str>, now: u64) -> Option<PrChipInput> {
    if !state.pr_status.enabled {
        return None;
    }
    let status = state.pr_status.statuses.get(branch?)?;
    let read_at = state.pr_status.read_at.unwrap_or(now);
    Some(PrChipInput {
        number: status.number,
        state: status.state,
        review: status.review,
        conflict: status.conflict,
        stale: micold_core::pull_request::is_stale(read_at, now),
    })
}

/// What the header says for `location`, from its Overview card's own model (R2); `None` when the
/// location does not resolve to a card.
pub(crate) fn header_text(
    state: &State,
    location: &SessionLocation,
    now: u64,
) -> Option<(HeaderText, Option<PrChipInput>)> {
    let model = crate::ui::overview::location_card(state, location)?;
    let pr = pr_chip_input(state, model.branch.as_deref(), now);
    Some((
        HeaderText::of(&model, pr.as_ref(), counts_of(state, location)),
        pr,
    ))
}

/// What is known of `location`'s change counts: the header's reading when the header follows it,
/// else nothing (US3 scenario 4: never another location's figure).
pub fn counts_of(state: &State, location: &SessionLocation) -> CountsReading {
    match &state.changes.header.entry {
        Some((project, entry))
            if entry == location && state.workspace.active.as_ref() == Some(project) =>
        {
            state.changes.header.reading()
        }
        _ => CountsReading::Unknown,
    }
}

/// What pressing the header's Changes button sends, and how it looks (H8; feature 1269, U12, R9):
/// always the sidebar's **Review changes** of `location`, which makes its Review tab or shows the
/// kept one; while that tab is shown the press changes nothing, and the button is drawn selected.
pub fn changes_press(state: &State, location: &SessionLocation) -> (Message, ChangesButton) {
    let key = state
        .workspace
        .active
        .clone()
        .map(|project| (project, location.clone()));
    let has_tab = key.as_ref().is_some_and(|key| state.changes.tabs.has(key));
    let shown = key.is_some() && state.review_shown() == key.as_ref();
    (
        Message::Sidebar(crate::features::sidebar::Msg::ReviewChangesRequested(
            location.clone(),
        )),
        ChangesButton::of(has_tab, shown),
    )
}

/// The shown review under its own location's header (R12, A1; feature 1269, U16): the header of
/// `shown_location()`, so a sessionless worktree's review has its own header and no review is
/// drawn under another location's. The strip is not drawn (its tabs and splits act on panes that
/// are not drawn). With no location shown, the view alone and no header (edge case *Empty*).
pub fn over_review<'a>(
    state: &'a State,
    changes: Element<'a, Message>,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    match state.shown_location() {
        Some(location) => column![header(state, &location, scheme), changes].into(),
        None => changes,
    }
}

/// The header's Changes button labelled `label` (H8, H9): outlined, the git glyph leading, shown
/// selected while `location`'s Review tab is shown, its tooltip saying what a press does. The label
/// is one line: where the title line cannot hold it (four-digit counts at the smallest window) it
/// is cut with an ellipsis and the tooltip holds it whole (FR-013, H10's bound).
pub fn changes_button<'a>(
    state: &State,
    location: &SessionLocation,
    label: String,
    r: Roles,
) -> Element<'a, Message> {
    let (press, button) = changes_press(state, location);
    let tooltip = format!("{}\n{label}", button.tooltip);
    let tint = material::ButtonVariant::Outlined.content(r, None);
    let text = Ellipsized::<Message>::at_role_spaced(label, TypeRole::Action, tint).shrink();
    Tooltip::new(
        material::Button::with_content(text, material::ButtonVariant::Outlined, r)
            .leading(Icon::Git)
            .selected(button.selected)
            .on_press(press),
        tooltip,
        r,
    )
    .into()
}

/// The context header (H9): the title line with the Changes button at its right, then the chips
/// row; child lists that never vary.
fn header<'a>(
    state: &State,
    location: &SessionLocation,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let Some((text, pr)) = header_text(state, location, crate::ui::sidebar::unix_now()) else {
        return container(Space::new()).into();
    };
    let mut chips: Vec<Element<'a, Message>> = vec![state_chip(&text, r)];
    let title = Tooltip::new(
        Ellipsized::<Message>::at_role_spaced(text.title, TypeRole::Title, r.on_surface),
        text.title_tooltip,
        r,
    );
    if let Some(branch) = text.branch {
        chips.push(neutral_chip(branch, None, r));
    }
    if let (Some(chip), Some(pr)) = (text.pull_request, pr) {
        let (mark, checks) = crate::ui::sidebar::indicator_marks(&pr.state);
        let mut indicator = PullRequestIndicator::<Message>::new(mark, &r).stale(pr.stale);
        if let Some(checks) = checks {
            indicator = indicator.checks(checks);
        }
        chips.push(neutral_chip(chip, Some(indicator.into()), r));
    }
    // Shrink beside the Fill title: laid out at the width it asks for (H10). On the title line, so
    // the chips keep the header's whole width at the smallest window.
    let changes = changes_button(state, location, text.changes, r);
    container(
        column![
            row![container(title).width(Length::Fill), changes]
                .spacing(spacing::MD)
                .align_y(Alignment::Center),
            row(chips).spacing(spacing::SM).align_y(Alignment::Center)
        ]
        .spacing(spacing::XS)
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(Padding {
        top: spacing::SM,
        bottom: spacing::SM,
        left: spacing::LG,
        right: spacing::LG,
    })
    .into()
}

/// The tab strip above the panes (S7): the scrolling members, then the status cluster, **New
/// session** and the split buttons, a child list that never varies (FR-011). The members sit from
/// the leading edge in a keyed row, so a press in flight follows its member when one is added
/// before it (R10); everything after the scrolling region is outside its viewport and keeps its
/// place and size at any member count (FR-010).
pub(crate) fn strip<'a>(
    state: &'a State,
    model: &StripModel,
    area: Option<&crate::ui::panes::PaneArea<'a>>,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let offset = state.session.tab_strip_scroll_offset as f32;
    let viewport = state.session.tab_strip_viewport_width as f32;
    let beyond = strip_overflow(offset, viewport, &model.members);
    // Feature 027 FR-002e: the fade takes the indicator's accent on the side the marked member lies
    // beyond.
    let marked_beyond = model.marked_index().and_then(|index| {
        crate::features::session_strip::scroll_into_view(index, offset, viewport, &model.members)
            .map(|target| target < offset)
    });
    let members = Keyed::row(model.members.iter().map(|m| (m.key(), member(m, r))))
        .spacing(spacing::SM)
        .align(Alignment::Center);
    let tabs = material::EdgeFade::new(
        material::Scrollable::new(members, r)
            .direction(material::ScrollDirection::Horizontal)
            .width(Length::Fill)
            .id(terminal::TAB_STRIP_SCROLL_ID.clone())
            // The content width it reports is dropped: `strip_overflow` derives it from the
            // members, because a measured one is stale until the first scroll (feature 027).
            .on_scroll_metrics(|offset, width, _content| {
                Message::Session(SessionMsg::TabStripScrolled { offset, width })
            })
            // The first frame matters most: a strip that already overflows must fade its edge
            // before anything has scrolled.
            .on_viewport_resize(|size| {
                Message::Session(SessionMsg::TabStripViewportResized {
                    width: crate::app::scroll_offset_px(size.width),
                })
            }),
        r,
    )
    .leading(beyond.leading)
    .trailing(beyond.trailing)
    .accent_on(marked_beyond)
    .width(Length::Fill);

    let new_session = Tooltip::new(
        IconButton::new(Icon::AddSession, r)
            .padding(spacing::SM)
            .on_press_maybe(
                model
                    .location
                    .clone()
                    .map(|at| Message::Session(SessionMsg::AdditionalRequested(at))),
            ),
        "Start another session…",
        r,
    )
    .position(TooltipPosition::Left);
    let splits: Element<'a, Message> = match area.and_then(|a| a.layout.pane(a.layout.focused())) {
        Some(pane) => crate::ui::panes::split_buttons(pane, r),
        None => container(Space::new()).into(),
    };
    let strip = row![tabs, status_cluster(model, r), new_session, splits]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
    material::Surface::new(strip, SurfaceKind::Toolbar, r)
        .width(Length::Fill)
        .padding(spacing::SM)
        .into()
}

/// The shown session's status words, cut under [`STATUS_MAX_WIDTH`] with the full words on hover,
/// and while its marked process is stopped the restart as the cluster's last child, so its coming
/// and going moves no other control (R9, US2 sc. 5 and 6).
fn status_cluster<'a>(model: &StripModel, r: tokens::Roles) -> Element<'a, Message> {
    let words = Tooltip::new(
        container(
            Ellipsized::<Message>::at_role(
                model.status.clone(),
                TypeRole::Label,
                r.on_surface_variant,
            )
            .shrink(),
        )
        .max_width(STATUS_MAX_WIDTH),
        model.status.clone(),
        r,
    )
    .position(TooltipPosition::Bottom);
    let mut cluster = row![words].spacing(spacing::SM).align_y(Alignment::Center);
    if let Some(restart) = model.restart.clone() {
        cluster = cluster.push(
            material::Button::with_content(
                Text::new("restart", TypeRole::Label, r).muted(),
                material::ButtonVariant::Text,
                r,
            )
            .padding(spacing::SM)
            .on_press(restart),
        );
    }
    cluster.into()
}

/// One strip member (S5, S7): a session tab at [`tab::SESSION_WIDTH`], a shell tab at
/// [`tab::WIDTH`] with its close control, or the "+". Indicators on the bottom edge: the strip sits
/// above the panes it selects (feature 012 FR-004b's rule, inverted with the strip's position).
fn member<'a>(m: &Member, r: tokens::Roles) -> Element<'a, Message> {
    let mark =
        |stopped: bool| ActivityBadge::for_emphasis(stopped.then_some(BadgeEmphasis::Stopped), r);
    let menu = m.menu_tab();
    let with_menu = |tab: Tab<'a, Message>| match menu {
        Some(target) => tab.on_secondary_press(move |(x, y)| {
            Message::Session(SessionMsg::StripTabMenuRequested(target, x, y))
        }),
        None => tab,
    };
    match m {
        Member::Session {
            label,
            tooltip,
            marked,
            stopped,
            ..
        } => {
            let tint = tab_content_colour(*marked, r);
            let tab = Tab::new(
                Ellipsized::<Message>::at_role(label.clone(), TypeRole::Label, tint).shrink(),
                r,
            )
            .fixed_width(tab::SESSION_WIDTH)
            .edge(IndicatorEdge::Bottom)
            .active(*marked)
            .leading(mark(*stopped))
            .on_press(m.press());
            Tooltip::new(with_menu(tab), tooltip.clone(), r)
                .position(TooltipPosition::Bottom)
                .into()
        }
        Member::Shell {
            label,
            marked,
            stopped,
            ..
        } => {
            // Every nested control takes the colour this tab draws its own label in (feature 012
            // BUG-001, FR-011a).
            let tint = tab_content_colour(*marked, r);
            let mut close = IconButton::new(Icon::Close, r)
                .size(TypeRole::Label)
                .padding(spacing::XS)
                .circular()
                .tint(tint);
            if let Some(message) = m.close() {
                close = close.on_press(message);
            }
            let tab = Tab::new(Text::new(label.clone(), TypeRole::Label, r).tint(tint), r)
                .edge(IndicatorEdge::Bottom)
                .active(*marked)
                .leading(mark(*stopped))
                .trailing(
                    Tooltip::new(close, "Close this terminal instance", r)
                        .position(TooltipPosition::Bottom),
                )
                .on_press(m.press());
            with_menu(tab).into()
        }
        Member::Plus => Tooltip::new(
            IconButton::new(Icon::AddTerminalInstance, r)
                .padding(spacing::SM)
                .on_press(m.press()),
            format!(
                "Open a new terminal instance ({})",
                crate::keymap::new_terminal_chord_label()
            ),
            r,
        )
        .position(TooltipPosition::Bottom)
        .into(),
    }
}

/// The state chip (FR-003): the card's state colour, its icon and its word, the reason on hover.
fn state_chip<'a>(text: &HeaderText, r: Roles) -> Element<'a, Message> {
    let p = card_palette(text.state, &r);
    let chip = Tag::<Message>::new(text.state.label(), p.header)
        .solid(p.on_header)
        .role(TypeRole::Label)
        .leading(
            Glyph::<Message>::new(card_state_icon(text.state), TypeRole::Label, r)
                .tint(p.on_header),
        )
        .max_width(CHIP_MAX_WIDTH);
    Tooltip::new(chip, text.state_tooltip.clone(), r).into()
}

/// A branch or pull request chip: neutral, led by `leading` when given, its full text on hover.
fn neutral_chip<'a>(
    chip: Chip,
    leading: Option<Element<'a, Message>>,
    r: Roles,
) -> Element<'a, Message> {
    let mut tag = Tag::<Message>::new(chip.text, r.on_surface_variant)
        .role(TypeRole::Label)
        .max_width(CHIP_MAX_WIDTH);
    if let Some(leading) = leading {
        tag = tag.leading(leading);
    }
    Tooltip::new(tag, chip.tooltip, r).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};

    const NOW: u64 = 1_000_000;

    fn status(number: u64, state: PrState) -> PullRequestStatus {
        PullRequestStatus {
            number,
            title: "A title".into(),
            url: format!("https://github.com/o/r/pull/{number}"),
            state,
            review: ReviewState::ChangesRequested,
            head: "abc".into(),
            conflict: true,
        }
    }

    fn with_status(enabled: bool, read_at: Option<u64>) -> State {
        let mut state = State::default();
        state.pr_status.enabled = enabled;
        state.pr_status.read_at = read_at;
        state.pr_status.statuses.insert(
            "fix/1117_ci".into(),
            status(
                1118,
                PrState::Open {
                    checks: CheckStatus::Failing,
                },
            ),
        );
        state
    }

    /// T009 (FR-005, data-model *PrChipInput*): no chip without a branch, while the status is off
    /// or with no status for the branch; otherwise the sidebar's reading, stale by its rule.
    #[test]
    fn the_pull_request_chip_reads_what_the_sidebar_reads() {
        let on = with_status(true, Some(NOW));
        assert_eq!(pr_chip_input(&on, None, NOW), None, "no branch");
        assert_eq!(
            pr_chip_input(&with_status(false, Some(NOW)), Some("fix/1117_ci"), NOW),
            None,
            "pull request status off"
        );
        assert_eq!(pr_chip_input(&on, Some("main"), NOW), None, "no status");
        assert_eq!(
            pr_chip_input(&on, Some("fix/1117_ci"), NOW),
            Some(PrChipInput {
                number: 1118,
                state: PrState::Open {
                    checks: CheckStatus::Failing,
                },
                review: ReviewState::ChangesRequested,
                conflict: true,
                stale: false,
            })
        );
        let old = with_status(true, Some(0));
        assert_eq!(
            pr_chip_input(&old, Some("fix/1117_ci"), NOW).map(|pr| pr.stale),
            Some(micold_core::pull_request::is_stale(0, NOW)),
            "stale under the sidebar's rule"
        );
        assert_eq!(
            pr_chip_input(&old, Some("fix/1117_ci"), NOW).map(|pr| pr.stale),
            Some(true)
        );
    }

    /// A project with the worktree `fix-1117_ci` on `fix/1117_ci` and one session in it, shown.
    fn shown_worktree() -> (State, SessionLocation) {
        use micold_core::project::{Availability, Project};
        use micold_core::session::{AiCli, Session};
        use micold_core::worktree::{Worktree, WorktreeStatus};
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state
            .workspace
            .projects
            .push(Project::new(path.clone(), true, Availability::Available));
        state.workspace.active = Some(path.clone());
        state.worktree.worktrees = vec![Worktree {
            dir_name: "fix-1117_ci".into(),
            path: "/repo/.claude/worktrees/fix-1117_ci".into(),
            branch: Some("fix/1117_ci".into()),
            status: WorktreeStatus::Valid,
            included: false,
        }];
        let location = SessionLocation::Worktree("fix-1117_ci".into());
        let mut session = Session::start_new(location.clone(), AiCli::ClaudeCode);
        session.lifecycle = micold_core::session::SessionLifecycle::Idle;
        let id = session.id;
        state.workspace.sessions.insert(path, vec![session]);
        state.session.active = Some(id);
        (state, location)
    }

    /// The pull request number the sidebar's row for `location` shows.
    fn sidebar_pr(state: &State, now: u64) -> Option<u64> {
        state.sidebar_entries().iter().find_map(|entry| {
            crate::features::sidebar::row_pull_request(
                entry,
                &state.pr_status.statuses,
                &state.pr_status.removable,
                state.pr_status.read_at,
                now,
            )
            .map(|row| row.status.number)
        })
    }

    /// The shown worktree's session starts running and a reading finds the branch's pull request,
    /// applied before one `State::update`.
    fn running_with_a_pull_request() -> (State, SessionLocation) {
        let (mut state, location) = shown_worktree();
        state.update(Message::NoOp);
        for session in state.workspace.sessions.values_mut().flatten() {
            session.lifecycle = micold_core::session::SessionLifecycle::Running;
        }
        state.pr_status.enabled = true;
        state.pr_status.read_at = Some(NOW);
        state.pr_status.statuses.insert(
            "fix/1117_ci".into(),
            status(
                1118,
                PrState::Open {
                    checks: CheckStatus::Failing,
                },
            ),
        );
        state.update(Message::NoOp);
        (state, location)
    }

    /// T010, US1 scenario 5 (FR-014): a change of the location's state and pull request, applied
    /// before one `State::update`, shows in the header text of that same state.
    #[test]
    fn the_header_shows_a_change_on_the_refresh() {
        let (mut state, location) = shown_worktree();
        state.update(Message::NoOp);
        let (before, _) = header_text(&state, &location, NOW).expect("the worktree resolves");
        assert_eq!(before.state, micold_core::overview::CardState::Idle);
        assert_eq!(before.pull_request, None);

        let (state, location) = running_with_a_pull_request();
        let (after, _) = header_text(&state, &location, NOW).expect("the worktree resolves");
        assert_eq!(
            after.state,
            micold_core::overview::CardState::NeedsAttention,
            "a running session whose pull request has failing checks and requested changes calls \
             for attention"
        );
        assert!(
            after
                .pull_request
                .as_ref()
                .is_some_and(|chip| chip.text.starts_with("PR #1118")),
            "the header names the pull request: {:?}",
            after.pull_request
        );
    }

    /// T010, US1 scenario 5 (FR-014): the header and the sidebar's row for the location agree on
    /// the pull request, before and after the change.
    #[test]
    fn the_header_and_the_sidebar_agree_on_the_pull_request() {
        let (mut state, location) = shown_worktree();
        state.update(Message::NoOp);
        assert_eq!(sidebar_pr(&state, NOW), None);
        let (before, _) = header_text(&state, &location, NOW).expect("the worktree resolves");
        assert_eq!(before.pull_request, None, "neither shows one yet");

        let (state, location) = running_with_a_pull_request();
        let (after, _) = header_text(&state, &location, NOW).expect("the worktree resolves");
        assert_eq!(sidebar_pr(&state, NOW), Some(1118));
        assert!(
            after
                .pull_request
                .is_some_and(|chip| chip.text.starts_with("PR #1118")),
            "the header names the sidebar's pull request"
        );
    }

    /// FR-014: the header's state is the card's own, which the Overview draws.
    #[test]
    fn the_header_state_is_the_cards_state() {
        let (state, location) = running_with_a_pull_request();
        let (after, _) = header_text(&state, &location, NOW).expect("the worktree resolves");
        let card = crate::ui::overview::location_card(&state, &location).expect("a card");
        assert_eq!(after.state, card.state, "the header's state is the card's");
    }

    /// R6: the follow step points the header at the shown location.
    #[test]
    fn the_follow_step_points_the_header_at_the_shown_location() {
        let (state, location) = running_with_a_pull_request();
        assert_eq!(
            state.changes.header.entry,
            Some((std::path::PathBuf::from("/repo"), location)),
            "the follow step points the header at the shown location"
        );
    }

    /// US3 scenario 4 (T062): `counts_of` shows the header's reading only for the location the
    /// header follows under the active project, never another location's figure.
    #[test]
    fn counts_of_reads_only_the_followed_location_under_the_active_project() {
        use crate::features::changes::Load;
        use micold_core::context_header::ChangeSummary;
        let (mut state, location) = shown_worktree();
        let summary = ChangeSummary {
            files: 3,
            added: 42,
            removed: 7,
        };
        state.changes.header.entry = Some((std::path::PathBuf::from("/repo"), location.clone()));
        state.changes.header.load = Load::Ready(summary);

        assert_eq!(
            counts_of(&state, &location),
            CountsReading::Known(summary),
            "the followed location under the active project, read"
        );
        assert_eq!(
            counts_of(&state, &SessionLocation::Default),
            CountsReading::Unknown,
            "another location"
        );
        state.workspace.active = Some(std::path::PathBuf::from("/other"));
        assert_eq!(
            counts_of(&state, &location),
            CountsReading::Unknown,
            "the same location under another active project"
        );
    }

    /// T046 (H8), feature 1269 T018 (U12, FR-012): the Changes button always asks for the
    /// location's review, never closes it; it is drawn selected exactly while that location's
    /// Review tab is shown, and its tooltip comes from `ChangesButton::of`.
    #[test]
    fn the_changes_press_opens_the_shown_location_or_shows_its_review() {
        use crate::features::sidebar::Msg as SidebarMsg;
        let (mut state, location) = shown_worktree();
        let open = Message::Sidebar(SidebarMsg::ReviewChangesRequested(location.clone()));
        assert_eq!(
            changes_press(&state, &location),
            (open.clone(), ChangesButton::of(false, false)),
            "no tab: opens one"
        );
        state.update(open.clone());
        assert_eq!(
            changes_press(&state, &location),
            (open.clone(), ChangesButton::of(true, true)),
            "shown: selected, a press changes nothing"
        );
        assert_eq!(
            changes_press(&state, &location).1.tooltip,
            "The review is shown"
        );
        state.update(Message::Sidebar(SidebarMsg::ReviewChangesRequested(
            SessionLocation::Default,
        )));
        assert_eq!(
            changes_press(&state, &location),
            (open.clone(), ChangesButton::of(true, false)),
            "another location's review shown: shows this one's kept tab, not selected"
        );
        let current = state.session.active.expect("a session is shown");
        state.update(Message::Session(SessionMsg::Selected(current)));
        assert_eq!(
            changes_press(&state, &location),
            (open, ChangesButton::of(true, false)),
            "a session shown: not selected"
        );
    }
}
