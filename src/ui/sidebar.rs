//! The left navigation sidebar: worktrees (top level) → sessions (sub-items), built from the
//! shared [`tree_view`] primitive (FR-002, FR-003, Constitution Principle VIII).

use crate::ui::material::{IconButton, Tooltip, TreeItem, TreeView};
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
const STRIP_WIDTH: f32 = 32.0;

/// A low-contrast color for separator/border lines (the sidebar edge and the resize handle):
/// the outline role softened with reduced alpha so it reads as a subtle divider, not a hard rule.
fn separator_color(r: Roles) -> iced::Color {
    iced::Color { a: 0.4, ..style::color(r.outline) }
}

/// Linearly interpolate between two colors (`t` 0→1), for the handle's animated hover highlight.
fn lerp_color(from: iced::Color, to: iced::Color, t: f32) -> iced::Color {
    let t = t.clamp(0.0, 1.0);
    iced::Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

/// Render the sidebar for the active project's worktrees and sessions, at the current
/// (adjustable) width.
pub fn view(state: &State, scheme: micold_ai_ide::theme::ColorScheme) -> Element<'_, Message> {
    let r = tokens::roles(scheme);
    let width = state.sidebar_width_px() as f32;

    // Header: title (fill) + add-worktree + hide, the actions grouped on the right.
    let add_worktree = Tooltip::new(
        IconButton::new(Icon::AddWorktree, r)
            .tint(r.primary)
            .on_press(Message::AddWorktreeOpened),
        "Add a worktree (new git branch)",
        r,
    );
    let hide = Tooltip::new(
        IconButton::new(Icon::HideSidebar, r)
            .tint(r.on_surface_variant)
            .on_press(Message::SidebarToggled),
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
        TreeView::new(build_items(state, r), r).into()
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
pub fn handle(
    scheme: micold_ai_ide::theme::ColorScheme,
    hover: f32,
) -> Element<'static, Message> {
    let r = tokens::roles(scheme);
    // The invisible grab zone is blended with the sidebar surface and sits on the LEFT; the 1px
    // separator line sits on the RIGHT, flush against the main area — so no window-background gap
    // shows between the separator and the terminal.
    let grab = container(Space::new(Length::Fixed(HANDLE_WIDTH - 1.0), Length::Fill))
        .height(Length::Fill)
        .style(style::sidebar_surface(r));
    // The separator brightens toward the accent as the pointer hovers (animated via `hover`).
    let line_color = lerp_color(separator_color(r), style::color(r.primary), hover);
    let line = container(Space::new(Length::Fixed(1.0), Length::Fill))
        .height(Length::Fill)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(line_color)),
            ..Default::default()
        });
    mouse_area(row![grab, line].height(Length::Fill))
        .on_press(Message::SidebarDragStarted)
        .on_enter(Message::SidebarHandleHovered(true))
        .on_exit(Message::SidebarHandleHovered(false))
        .interaction(iced::mouse::Interaction::ResizingHorizontally)
        .into()
}

/// The collapsed sidebar: a thin vertical strip hosting the "show sidebar" button (with a
/// tooltip), wide enough for the icon.
pub fn collapsed_strip(scheme: micold_ai_ide::theme::ColorScheme) -> Element<'static, Message> {
    let r = tokens::roles(scheme);
    let show = Tooltip::new(
        IconButton::new(Icon::ShowSidebar, r)
            .tint(r.on_surface_variant)
            .on_press(Message::SidebarToggled),
        "Show sidebar",
        r,
    );
    let content = container(
        column![show]
            .align_x(Alignment::Center)
            .padding(spacing::XS),
    )
    .width(Length::Fixed(STRIP_WIDTH - 1.0))
    .height(Length::Fill)
    .style(style::sidebar_surface(r));

    // A subtle right border so the collapsed strip still reads as a bounded panel edge.
    let border = container(Space::new(Length::Fixed(1.0), Length::Fill))
        .height(Length::Fill)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(separator_color(r))),
            ..Default::default()
        });

    row![content, border].height(Length::Fill).into()
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
