//! The left navigation sidebar: worktrees (top level) → sessions (sub-items), built from the
//! shared [`tree_view`] primitive (FR-002, FR-003, Constitution Principle VIII).

use crate::ui::material::{IconButton, Tooltip, TreeItem, TreeView};
use crate::ui::style;
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Length};
use micold_ai_ide::app::{Message, State, TagFilter};
use micold_ai_ide::icons::Icon;
use micold_ai_ide::naming::Tag;
use micold_ai_ide::session::SessionLifecycle;
use micold_ai_ide::tokens::{self, shape, sidebar, spacing, type_scale, Rgb, Roles};
use micold_ai_ide::worktree::WorktreeStatus;

/// Width of the draggable resize handle between the sidebar and the main area.
const HANDLE_WIDTH: f32 = 6.0;
/// Width of the collapsed strip that hosts the "show sidebar" button.
const STRIP_WIDTH: f32 = 32.0;

/// A low-contrast color for separator/border lines (the sidebar edge and the resize handle):
/// the outline role softened with reduced alpha so it reads as a subtle divider, not a hard rule.
fn separator_color(r: Roles) -> iced::Color {
    iced::Color {
        a: 0.4,
        ..style::color(r.outline)
    }
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
pub fn view<'a>(
    state: &'a State,
    scheme: micold_ai_ide::theme::ColorScheme,
    row_fx: &micold_ai_ide::motion::Animator<u64>,
) -> Element<'a, Message> {
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
        let filtered = state.filtered_worktree_tree();
        let list: Element<'_, Message> = if filtered.is_empty() {
            // Active filter matched nothing (FR-027): a message + a one-tap clear.
            column![
                text("No worktrees match the filter.")
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
                button(text("Clear filters").size(sidebar::TAG))
                    .padding(spacing::XS)
                    .style(style::text_button(r))
                    .on_press(Message::SidebarFiltersCleared),
            ]
            .spacing(spacing::XS)
            .into()
        } else {
            TreeView::new(build_items(state, filtered, r, row_fx), r)
                .label_size(sidebar::NAME)
                .into()
        };
        // Scroll the list when it exceeds the sidebar height, with a thin themed scrollbar.
        // The list gets a little right padding so rows never sit under the scrollbar, and the
        // filter bar stays fixed above it.
        let scrolled = scrollable(container(list).padding(iced::Padding {
            top: 0.0,
            right: spacing::SM as f32,
            bottom: 0.0,
            left: 0.0,
        }))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4.0)
                .scroller_width(4.0)
                .margin(1.0),
        ))
        .height(Length::Fill)
        .style(style::scrollbar(r));
        column![filter_bar(state, r), scrolled]
            .spacing(spacing::SM)
            .height(Length::Fill)
            .into()
    };

    // Minimal left/right padding to maximize name/tag width (FR-009); a little vertical breathing
    // room is kept.
    let content = column![header, body]
        .spacing(spacing::SM)
        .padding(iced::Padding {
            top: spacing::SM as f32,
            bottom: spacing::SM as f32,
            left: spacing::XS as f32,
            right: spacing::XS as f32,
        })
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
pub fn handle(scheme: micold_ai_ide::theme::ColorScheme, hover: f32) -> Element<'static, Message> {
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
    let (fill, on) = match filter {
        TagFilter::Type(t) => r.type_tag(t),
        TagFilter::HasIssue => r.issue_tag(),
        TagFilter::Untyped => (r.surface_variant, r.on_surface_variant),
    };
    let (fill, on) = (style::color(fill), style::color(on));
    let muted = style::color(r.on_surface_variant);
    let outline = style::color(r.outline);
    button(text(filter_label(filter)).size(sidebar::TAG))
        .padding(iced::Padding {
            top: 1.0,
            bottom: 1.0,
            left: spacing::SM as f32,
            right: spacing::SM as f32,
        })
        .on_press(Message::SidebarFilterToggled(filter))
        .style(
            move |_theme: &iced::Theme, _status| iced::widget::button::Style {
                background: Some(Background::Color(if active {
                    fill
                } else {
                    Color::TRANSPARENT
                })),
                text_color: if active { on } else { muted },
                border: Border {
                    color: if active { fill } else { outline },
                    width: if active { 0.0 } else { 1.0 },
                    radius: (shape::FULL as f32).into(),
                },
                ..Default::default()
            },
        )
        .into()
}

/// The filter chip bar shown above the worktree list (feature 008, FR-024/FR-026). One chip per
/// available filter (chunked into rows so they never overflow the sidebar), plus a "Clear"
/// control when any filter is active.
fn filter_bar(state: &State, r: Roles) -> Element<'static, Message> {
    let available = state.available_tag_filters();
    if available.is_empty() {
        return Space::with_height(Length::Fixed(0.0)).into();
    }
    let mut col = column![].spacing(spacing::XS);
    for chunk in available.chunks(3) {
        let mut rw = row![].spacing(spacing::XS).align_y(Alignment::Center);
        for &filter in chunk {
            rw = rw.push(filter_chip(
                filter,
                state.sidebar_filters.contains(&filter),
                r,
            ));
        }
        col = col.push(rw);
    }
    if !state.sidebar_filters.is_empty() {
        col = col.push(
            button(text("Clear filters").size(sidebar::TAG))
                .padding(spacing::XS)
                .style(style::text_button(r))
                .on_press(Message::SidebarFiltersCleared),
        );
    }
    col.into()
}

/// Resolve a [`Tag`] into a chip's `(label, accent)` for the active scheme (FR-005, FR-011).
/// Rendered as a dimmed tonal chip (see [`style::chip`]); status tags use the `error` accent.
fn tag_chip(tag: &Tag, r: Roles) -> (String, Rgb) {
    match tag {
        Tag::Type(t) => (t.as_str().to_string(), r.tag_fill(*t)),
        Tag::Issue(key) => (key.clone(), r.tag_issue),
        Tag::Status(status) => {
            let label = match status {
                WorktreeStatus::Missing => "missing",
                WorktreeStatus::Invalid => "invalid",
                WorktreeStatus::Valid => "",
            };
            (label.to_string(), r.error)
        }
    }
}

/// Blend `from`→`to` by `t` (0..=1), for fading an icon's tint.
fn lerp_rgb(from: Rgb, to: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Rgb {
        r: mix(from.r, to.r),
        g: mix(from.g, to.g),
        b: mix(from.b, to.b),
    }
}

/// One action icon in a worktree row's hover cluster. Its tint fades from the sidebar surface
/// (invisible at `progress` 0) to `base_tint` (fully shown at 1), so it appears/disappears on
/// hover without a scrim. When `active` (the hovered row) it is pressable and carries a tooltip;
/// otherwise it is inert. The button occupies its slot regardless, so the row never reflows.
fn action_icon(
    glyph: Icon,
    base_tint: Rgb,
    message: Message,
    tooltip: &'static str,
    active: bool,
    progress: f32,
    r: Roles,
) -> Element<'static, Message> {
    let tint = lerp_rgb(r.surface, base_tint, progress);
    let button = IconButton::new(glyph, r)
        .size(sidebar::NAME)
        .tint(tint)
        .on_press_maybe(active.then_some(message));
    if active {
        Tooltip::new(button, tooltip, r).into()
    } else {
        button.into()
    }
}

/// The hover-revealed row-action cluster for a worktree (feature 008): an add-session "+" (only
/// when a session can start) and a trash icon that requests deletion. The cluster is rendered on
/// EVERY row so its width is always reserved (no reflow when it appears); `progress` fades the
/// icons in/out, and `active` (the hovered row) gates whether they are pressable.
fn row_actions_cluster(
    dir: &str,
    can_start_session: bool,
    active: bool,
    progress: f32,
    r: Roles,
) -> Element<'static, Message> {
    let mut cluster = row![].spacing(spacing::XS).align_y(Alignment::Center);
    if can_start_session {
        cluster = cluster.push(action_icon(
            Icon::AddSession,
            r.primary,
            Message::SessionStartRequested {
                worktree_dir: dir.to_string(),
            },
            "Start a new session in this worktree",
            active,
            progress,
            r,
        ));
    }
    cluster = cluster.push(action_icon(
        Icon::Delete,
        r.error,
        Message::WorktreeDeleteRequested(dir.to_string()),
        "Delete this worktree",
        active,
        progress,
        r,
    ));
    cluster.into()
}

/// Flatten the worktree tree into ordered [`TreeItem`]s (worktrees, then their sessions when
/// expanded). `actions_row` is the worktree currently showing its hover actions (faded by
/// `row_actions_progress`).
fn build_items(
    state: &State,
    nodes: Vec<micold_ai_ide::app::WorktreeNode>,
    r: Roles,
    row_fx: &micold_ai_ide::motion::Animator<u64>,
) -> Vec<TreeItem<'static, Message>> {
    let mut items = Vec::new();
    let hovered = state.hovered_worktree.as_deref();

    for node in nodes {
        let wt = &node.worktree;
        // No leading git icon (FR-010); a non-Valid worktree is cued by an error-tinted name
        // plus a status tag (FR-011).
        let tint = match wt.status {
            WorktreeStatus::Valid => r.on_surface,
            WorktreeStatus::Missing | WorktreeStatus::Invalid => r.error,
        };

        let tags: Vec<(String, Rgb)> = node.tags.iter().map(|tag| tag_chip(tag, r)).collect();
        let dir = wt.dir_name.clone();

        let mut item = TreeItem::new(0, node.display_name.clone(), tint)
            .tags(tags)
            .on_right_press(Message::WorktreeMenuToggled(dir.clone()))
            .hover(
                Message::WorktreeHovered(dir.clone()),
                Message::WorktreeUnhovered(dir.clone()),
            )
            .expandable(
                node.expanded,
                Message::WorktreeExpansionToggled(dir.clone()),
            );

        // Always reserve the action cluster's width so hovering never reflows the row; each row
        // fades its icons in/out independently via its own animation track (feature 008). The
        // hovered row is the pressable one.
        let active = hovered == Some(dir.as_str());
        let progress = row_fx.get(crate::ui::worktree_fx_key(&dir));
        item = item.trailing_element(row_actions_cluster(
            &dir,
            wt.can_start_session(),
            active,
            progress,
            r,
        ));
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
