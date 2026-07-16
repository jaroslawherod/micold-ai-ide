//! The left navigation sidebar: worktrees (top level) → sessions (sub-items), built from the
//! shared [`tree_view`] primitive (FR-002, FR-003, Constitution Principle VIII).

use crate::ui::material::{icon_button, tree_view, with_tooltip, TreeItem};
use crate::ui::style;
use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Alignment, Element, Length};
use micold_ai_ide::app::{Message, State};
use micold_ai_ide::icons::Icon;
use micold_ai_ide::session::SessionLifecycle;
use micold_ai_ide::tokens::{self, spacing, type_scale, Roles};
use micold_ai_ide::worktree::WorktreeStatus;

/// Width of the draggable resize handle between the sidebar and the main area.
const HANDLE_WIDTH: f32 = 6.0;
/// Width of the collapsed strip that hosts the "show sidebar" button.
const STRIP_WIDTH: f32 = 44.0;

/// Render the sidebar for the active project's worktrees and sessions, at the current
/// (adjustable) width.
pub fn view(state: &State, scheme: micold_ai_ide::theme::ColorScheme) -> Element<'_, Message> {
    let r = tokens::roles(scheme);
    let width = state.sidebar_width_px() as f32;

    // Header: title (fill) + add-worktree + hide, the actions grouped on the right.
    let add_worktree = with_tooltip(
        icon_button(
            Icon::AddWorktree,
            type_scale::BODY,
            r.primary,
            r,
            Some(Message::AddWorktreeOpened),
        ),
        "Add a worktree (new git branch)",
        r,
    );
    let hide = with_tooltip(
        icon_button(
            Icon::HideSidebar,
            type_scale::BODY,
            r.on_surface_variant,
            r,
            Some(Message::SidebarToggled),
        ),
        "Hide sidebar",
        r,
    );
    let header = row![
        text("Worktrees")
            .size(type_scale::TITLE)
            .width(Length::Fill),
        add_worktree,
        hide,
    ]
    .align_y(Alignment::Center)
    .spacing(spacing::XS);

    let body: Element<'_, Message> = if state.worktrees.is_empty() {
        container(
            text("No worktrees yet. Add one to get started.")
                .size(type_scale::LABEL)
                .style(style::muted(r)),
        )
        .padding(spacing::MD)
        .into()
    } else {
        tree_view(build_items(state, r), r)
    };

    let content = column![header, body]
        .spacing(spacing::MD)
        .padding(spacing::MD)
        .width(Length::Fixed(width))
        .height(Length::Fill);

    container(content)
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .style(style::sidebar_surface(r))
        .into()
}

/// The resize handle between the sidebar and the main area. The drag zone itself is invisible
/// (a thin transparent hit target) with a single 1px boundary line that reads as the
/// sidebar's right border; hovering shows the horizontal-resize cursor. Pressing it starts a
/// resize drag (the binary captures the drag with a full-window overlay).
pub fn handle(scheme: micold_ai_ide::theme::ColorScheme) -> Element<'static, Message> {
    let r = tokens::roles(scheme);
    let line = container(Space::new(Length::Fixed(1.0), Length::Fill))
        .height(Length::Fill)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(style::color(r.outline))),
            ..Default::default()
        });
    mouse_area(
        row![line, Space::with_width(Length::Fixed(HANDLE_WIDTH - 1.0))].height(Length::Fill),
    )
    .on_press(Message::SidebarDragStarted)
    .interaction(iced::mouse::Interaction::ResizingHorizontally)
    .into()
}

/// The collapsed sidebar: a thin vertical strip hosting the "show sidebar" button (with a
/// tooltip), wide enough for the icon.
pub fn collapsed_strip(scheme: micold_ai_ide::theme::ColorScheme) -> Element<'static, Message> {
    let r = tokens::roles(scheme);
    let show = with_tooltip(
        icon_button(
            Icon::ShowSidebar,
            type_scale::BODY,
            r.on_surface_variant,
            r,
            Some(Message::SidebarToggled),
        ),
        "Show sidebar",
        r,
    );
    container(
        column![show]
            .align_x(Alignment::Center)
            .padding(spacing::XS),
    )
    .width(Length::Fixed(STRIP_WIDTH))
    .height(Length::Fill)
    .style(style::sidebar_surface(r))
    .into()
}

/// Flatten the worktree tree into ordered [`TreeItem`]s (worktrees, then their sessions when
/// expanded).
fn build_items(state: &State, r: Roles) -> Vec<TreeItem<'static, Message>> {
    let mut items = Vec::new();

    for node in state.worktree_tree() {
        let wt = &node.worktree;
        let (icon, tint) = match wt.status {
            WorktreeStatus::Valid => (Icon::Git, r.on_surface),
            WorktreeStatus::Missing | WorktreeStatus::Invalid => (Icon::Unavailable, r.error),
        };

        let mut item = TreeItem::new(0, wt.dir_name.clone(), tint)
            .with_icon(icon)
            .expandable(
                node.expanded,
                Message::WorktreeExpansionToggled(wt.dir_name.clone()),
            );

        // Only a valid worktree can host a new session (FR-018a): trailing "+" add-session.
        if wt.can_start_session() {
            item = item.trailing(
                Icon::AddSession,
                Message::SessionStartRequested {
                    worktree_dir: wt.dir_name.clone(),
                },
                "Start a new session in this worktree",
            );
        }
        items.push(item);

        if node.expanded {
            for session in &node.sessions {
                let tint = match session.lifecycle {
                    SessionLifecycle::Failed => r.error,
                    SessionLifecycle::Idle => r.on_surface_variant,
                    _ => r.on_surface,
                };
                let selected = state.active_session == Some(session.id);
                items.push(
                    TreeItem::new(1, session.label.display().to_string(), tint)
                        .with_icon(Icon::ActiveMarker)
                        .selected(selected)
                        .on_press(Message::SessionSelected(session.id))
                        .trailing(
                            Icon::Unavailable,
                            Message::SessionCloseRequested(session.id),
                            "Close this session",
                        ),
                );
            }
        }
    }

    items
}
