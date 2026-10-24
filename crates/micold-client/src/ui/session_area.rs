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
use micold_core::context_header::{Chip, HeaderText, PrChipInput};
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
    Some((HeaderText::of(&model, pr.as_ref()), pr))
}

/// The context header (H9 without the Changes button): the title line, then the chips row.
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
    container(
        column![
            title,
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

    /// T010, US1 scenario 5 (FR-014): a change of the location's state and pull request, applied
    /// before one `State::update`, shows in the sidebar row and in the header text of that same
    /// state; the two never disagree.
    #[test]
    fn the_header_shows_a_change_on_the_refresh_the_sidebar_shows_it() {
        let (mut state, location) = shown_worktree();
        state.update(Message::NoOp);
        let (before, _) = header_text(&state, &location, NOW).expect("the worktree resolves");
        assert_eq!(before.state, micold_core::overview::CardState::Idle);
        assert_eq!(before.pull_request, None);
        assert_eq!(sidebar_pr(&state, NOW), None);

        // The session starts running and a reading finds the branch's pull request.
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
        let (after, _) = header_text(&state, &location, NOW).expect("the worktree resolves");
        let card = crate::ui::overview::location_card(&state, &location).unwrap();
        assert_eq!(after.state, card.state, "the header's state is the card's");
        assert_ne!(after.state, before.state, "the change shows");
        assert_eq!(sidebar_pr(&state, NOW), Some(1118));
        assert!(
            after
                .pull_request
                .as_ref()
                .is_some_and(|chip| chip.text.starts_with("PR #1118")),
            "the header names the sidebar's pull request: {:?}",
            after.pull_request
        );
        assert_eq!(
            state.changes.header.entry,
            Some((std::path::PathBuf::from("/repo"), location)),
            "the follow step points the header at the shown location"
        );
    }
}
