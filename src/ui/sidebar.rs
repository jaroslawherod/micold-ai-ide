//! The left navigation sidebar: worktrees (top level) → sessions (sub-items), built from the
//! shared [`tree_view`] primitive (FR-002, FR-003, Constitution Principle VIII).

use crate::ui::material::{
    expand, menu_panel, FilterTrigger, IconButton, ToggleChip, Tooltip, TreeItem, TreeView,
};
use crate::ui::style;
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};
use micold_ai_ide::app::{Message, State, TagFilter};
use micold_ai_ide::icons::Icon;
use micold_ai_ide::naming::Tag;
use micold_ai_ide::session::{SessionLifecycle, SessionLocation};
use micold_ai_ide::tokens::{self, sidebar, spacing, type_scale, Rgb, Roles};
use micold_ai_ide::worktree::WorktreeStatus;

/// Width of the draggable resize handle between the sidebar and the main area.
const HANDLE_WIDTH: f32 = 6.0;
/// Width of the collapsed strip that hosts the "show sidebar" button.
const STRIP_WIDTH: f32 = 32.0;

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
    filter_progress: f32,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let width = state.sidebar_width_px() as f32;

    // Header: filter (left) + title (fill) + add-worktree + hide.
    // Toggles the filter accordion below (feature 009); tinted to show whether any filter is
    // currently active even while the accordion is collapsed (FR-005, US2).
    let filter_toggle: Element<'_, Message> =
        FilterTrigger::new(Message::SidebarFilterMenuToggled, r)
            .active(!state.sidebar_filters.is_empty())
            .into();
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
        filter_toggle,
        text("Worktrees")
            .size(type_scale::TITLE)
            .width(Length::Fill),
        add_worktree,
        hide,
    ]
    .align_y(Alignment::Center)
    .spacing(spacing::XS);

    // The filter chips (feature 008) live in an accordion that expands/collapses below the
    // header (feature 009) — collapsed to zero height by default, pushing the worktree list
    // down rather than floating over it. Skip building `filter_bar()` (an O(worktrees) scan
    // plus a chip `Element` tree) while fully collapsed — the common steady state — since it
    // would only be thrown away as invisible; `expand()` itself already renders a bare
    // zero-height node with no content to lay out or hit-test in that case.
    //
    // Feature 014 (FR-010c): the reveal chip is the accordion's FIRST element and is rendered
    // unconditionally — deliberately not inside `filter_bar()`, which returns early with "No tags
    // to filter yet." exactly when a project's only worktrees are agent-owned, i.e. when the
    // control matters most.
    let filter_accordion: Element<'_, Message> = if filter_progress > 0.001 {
        let body = column![reveal_chip(state, r), filter_bar(state, r)].spacing(spacing::XS);
        expand(menu_panel(body, Length::Shrink, r, false), filter_progress)
    } else {
        Space::with_height(Length::Fixed(0.0)).into()
    };

    // The "Default" entry (feature 010) is always present once a project is open — see
    // `sidebar_entries()` — so, unlike before this feature, the sidebar is never truly "empty":
    // a zero-worktree or filtered-to-nothing project still shows the Default row and its
    // start-session action, with a muted hint appended below about worktrees specifically.
    //
    // Computed once and reused for both the hint and the tree below (rather than calling
    // `filtered_worktree_tree()`/`sidebar_entries()` twice per render) — `sidebar_entries()`
    // always puts the Default entry first, so "only that one entry survived" is equivalent to
    // "no worktree entries matched" without a second filter pass.
    let entries = state.sidebar_entries();
    let no_worktree_entries = entries.len() <= 1;
    // Feature 014 (FR-003): asks for VISIBLE worktrees, not all discovered ones. A project whose
    // only worktrees are agent-owned has none visible, so it takes the "none yet" branch — the
    // "no match / clear filters" branch would offer to clear a filter that was never applied.
    let hint: Option<Element<'_, Message>> = if !state.has_visible_worktrees() {
        Some(
            text("No worktrees yet. Add one to get started.")
                .size(type_scale::LABEL)
                .style(style::muted(r))
                .into(),
        )
    } else if no_worktree_entries {
        // Active filter matched nothing (FR-027): a message + a one-tap clear.
        Some(
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
            .into(),
        )
    } else {
        None
    };

    let tree: Element<'_, Message> = TreeView::new(build_items(state, entries, r, row_fx), r)
        .label_size(sidebar::NAME)
        .into();
    let list: Element<'_, Message> = match hint {
        Some(hint) => column![tree, hint].spacing(spacing::SM).into(),
        None => tree,
    };
    // Scroll the list when it exceeds the sidebar height, with a thin themed scrollbar.
    // The list gets a little right padding so rows never sit under the scrollbar.
    let body: Element<'_, Message> = scrollable(container(list).padding(iced::Padding {
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
    .style(style::scrollbar(r))
    .into();

    // Minimal left/right padding to maximize name/tag width (FR-009); a little vertical breathing
    // room is kept.
    let content = column![header, filter_accordion, body]
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
    let line_color = lerp_color(style::separator(r), style::color(r.primary), hover);
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
            background: Some(iced::Background::Color(style::separator(r))),
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
    // Feature 014: the pill styling that used to live here is now the shared `ToggleChip`
    // primitive, so this and the reveal chip below stay identical by construction rather than by
    // two copies drifting apart (Principle VIII Component-reuse gate).
    let (fill, on) = match filter {
        TagFilter::Type(t) => r.type_tag(t),
        TagFilter::HasIssue => r.issue_tag(),
        TagFilter::Untyped => (r.surface_variant, r.on_surface_variant),
    };
    ToggleChip::new(
        filter_label(filter),
        Message::SidebarFilterToggled(filter),
        r,
    )
    .active(active)
    .accent(fill, on)
    .into()
}

/// The "Show agent worktrees" reveal chip (feature 014, FR-010).
///
/// Rendered by [`view`] ABOVE `filter_bar()` and outside its early return, so it stays reachable
/// in a project whose only worktrees are agent-owned — the case where a user most needs it, and
/// exactly the case where `filter_bar()` bails out with "No tags to filter yet." (FR-010c).
///
/// Uses the neutral default accent: this is not a tag, so it borrows no tag color.
fn reveal_chip(state: &State, r: Roles) -> Element<'static, Message> {
    ToggleChip::new(
        "Show agent worktrees",
        Message::ShowAgentWorktreesToggled,
        r,
    )
    .active(state.show_agent_worktrees)
    .into()
}

/// The filter chip bar, shown inside the sidebar's filter accordion (feature 009) rather than
/// always visible above the worktree list. One chip per available filter (chunked into rows so
/// they never overflow the panel), plus a "Clear" control when any filter is active (feature
/// 008, FR-024/FR-026). When no tags exist anywhere yet, shows a short message instead of an
/// empty panel (FR-009).
fn filter_bar(state: &State, r: Roles) -> Element<'static, Message> {
    let available = state.available_tag_filters();
    if available.is_empty() {
        return text("No tags to filter yet.")
            .size(sidebar::TAG)
            .style(style::muted(r))
            .into();
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
        // Feature 014 (FR-010b): a neutral accent, not `error` — an agent worktree is
        // informational, not a fault condition.
        Tag::Agent => ("agent".to_string(), r.on_surface_variant),
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
                location: SessionLocation::Worktree(dir.to_string()),
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

/// Flatten the sidebar's location list (feature 010: the "Default" entry, then worktrees) into
/// ordered [`TreeItem`]s, each followed by its sessions when expanded.
fn build_items(
    state: &State,
    entries: Vec<micold_ai_ide::app::SidebarEntry>,
    r: Roles,
    row_fx: &micold_ai_ide::motion::Animator<u64>,
) -> Vec<TreeItem<'static, Message>> {
    let mut items = Vec::new();
    let hovered = state.hovered_worktree.as_deref();
    let project_root = state.workspace.active.as_deref();

    for entry in entries {
        let node = match entry {
            micold_ai_ide::app::SidebarEntry::Default(node) => {
                items.extend(build_default_item(state, &node, r));
                continue;
            }
            micold_ai_ide::app::SidebarEntry::Worktree(node) => node,
        };
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
        // Location tooltip (feature 010, FR-010): the worktree's path relative to the project.
        if let Some(root) = project_root {
            item = item.row_tooltip(micold_ai_ide::app::worktree_location_label(root, wt));
        }

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
                items.push(session_tree_item(session, state.active_session, r));
            }
        }
    }

    items
}

/// One session sub-item, depth 1 — shared by worktree rows and the "Default" row (feature 010)
/// so the two locations render their sessions identically (FR-005 lifecycle parity).
fn session_tree_item(
    session: &micold_ai_ide::session::Session,
    active_session: Option<micold_ai_ide::session::SessionId>,
    r: Roles,
) -> TreeItem<'static, Message> {
    let tint = match session.lifecycle {
        SessionLifecycle::Failed => r.error,
        SessionLifecycle::Idle => r.on_surface_variant,
        _ => r.on_surface,
    };
    let selected = active_session == Some(session.id);
    TreeItem::new(1, session.label.display().to_string(), tint)
        .with_icon(Icon::ActiveMarker)
        .selected(selected)
        .on_press(Message::SessionSelected(session.id))
        .trailing(
            Icon::Close,
            Message::SessionCloseRequested(session.id),
            "Close this session",
        )
}

/// The "Default" (project-root) row + its session sub-items when expanded (feature 010, US1).
/// Always present when a project is open (`sidebar_entries`'s first entry); its "start a
/// session" action reuses the same `IconButton`/`Icon::AddSession` affordance as a worktree row,
/// but — unlike worktree rows' hover-revealed cluster — is always shown, since it is the row's
/// sole action and there is only ever one Default row to keep tidy (no reflow/clutter concern
/// that motivated the fade-on-hover treatment for the potentially-many worktree rows).
fn build_default_item(
    state: &State,
    node: &micold_ai_ide::app::DefaultNode,
    r: Roles,
) -> Vec<TreeItem<'static, Message>> {
    let mut items = Vec::new();

    // `active: true, progress: 1.0` gives the always-visible, always-pressable button this row
    // needs (see the doc comment above), reusing the same construction as a worktree row's
    // hover-revealed action icons instead of hand-rebuilding it.
    let start_session = action_icon(
        Icon::AddSession,
        r.primary,
        Message::SessionStartRequested {
            location: SessionLocation::Default,
        },
        "Start a new session in the project root",
        true,
        1.0,
        r,
    );

    let item = TreeItem::new(0, node.display_name.to_string(), r.on_surface)
        // Distinct icon (FR-006): never the git/branch iconography used for worktree rows.
        .with_icon(Icon::ProjectRoot)
        .expandable(node.expanded, Message::DefaultExpansionToggled)
        .trailing_element(start_session)
        // Location tooltip (FR-010): fixed, since the Default entry is always the project root.
        .row_tooltip(micold_ai_ide::app::DEFAULT_LOCATION_LABEL);
    items.push(item);

    if node.expanded {
        for session in &node.sessions {
            items.push(session_tree_item(session, state.active_session, r));
        }
    }

    items
}
