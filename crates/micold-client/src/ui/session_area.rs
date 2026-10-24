//! The session area: what the main area draws while a session is shown (feature 1126, contract
//! A1, research R12). Top to bottom: the context header naming the shown location, then the pane
//! area with its bar. The header's words come from `micold_core::context_header`; this module only
//! lays them out (contract H9).

use crate::app::{Message, State};
use crate::ui::material::{
    card_palette, card_state_icon, Ellipsized, Glyph, PullRequestIndicator, Tag, Tooltip, TypeRole,
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

/// The session area for the shown session: the header above the pane area (A1). The header's slot
/// is there even when the shown location does not resolve, empty, so the pane area's place in the
/// tree never moves (feature 023 FR-008a).
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
    column![
        header,
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
