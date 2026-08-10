//! The left navigation sidebar: worktrees (top level) → sessions (sub-items), built from the
//! shared [`tree_view`] primitive (FR-002, FR-003, Constitution Principle VIII).

use crate::app::{Message, State};
use crate::features::sidebar::TagFilter;
use crate::icons::Icon;
use crate::ui::material::{
    self, Accordion, ActivityBadge, Button, ButtonVariant, Divider, FilterTrigger, HoverReveal,
    IconButton, Scrollable, SurfaceKind, Text, ToggleChip, Tooltip, TreeItem, TreeView, TypeRole,
};
use iced::widget::{column, container, row};
use iced::{Alignment, Element, Length};
use micold_core::naming::Tag;
use micold_core::session::{SessionLifecycle, SessionLocation};
use micold_core::tokens::{self, spacing, Rgb, Roles};
use micold_core::worktree::WorktreeStatus;

/// The sidebar list's scroll viewport, by name (feature 024, FR-008).
///
/// A `LazyLock` because `scrollable::Id` is not a `const` and the id has to be the *same* one on
/// every frame — a fresh id each render would name a viewport the previous frame's scroll operation
/// was addressed to, and the scroll would land nowhere.
pub static SIDEBAR_SCROLL_ID: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("sidebar-list"));

/// Width of the collapsed strip that hosts the "show sidebar" button.
const STRIP_WIDTH: f32 = 32.0;

/// Render the sidebar for the active project's worktrees and sessions, at the current
/// (adjustable) width.
pub fn view<'a>(state: &'a State, scheme: micold_core::theme::ColorScheme) -> Element<'a, Message> {
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
            .compact()
            .tint(r.primary)
            .on_press(Message::AddWorktreeOpened),
        "Add a worktree (new git branch)",
        r,
    );
    let hide = Tooltip::new(
        IconButton::new(Icon::HideSidebar, r)
            .compact()
            .tint(r.on_surface_variant)
            .on_press(Message::SidebarToggled),
        "Hide sidebar",
        r,
    );
    let header = row![
        filter_toggle,
        Text::new("Worktrees", TypeRole::Section, r).width(Length::Fill),
        add_worktree,
        hide,
    ]
    .align_y(Alignment::Center)
    .spacing(spacing::XS);

    // The filter chips (feature 008) live in an accordion that expands/collapses below the
    // header (feature 009) — collapsed to zero height by default, pushing the worktree list
    // down rather than floating over it.
    //
    // Built on every render, including while collapsed. It used to be skipped in that case
    // (`filter_bar()` is an O(worktrees) scan plus a chip `Element` tree), but the test was "is
    // the reveal still in progress", which only the accordion itself can answer now that it owns
    // its own track — and asking it would mean it could no longer be built from a single pass over
    // the state. A collapsed accordion lays out and hit-tests nothing regardless.
    //
    // Feature 014 (FR-010c): the reveal chip is the accordion's FIRST element and is rendered
    // unconditionally — deliberately not inside `filter_bar()`, which returns early with "No tags
    // to filter yet." exactly when a project's only worktrees are agent-owned, i.e. when the
    // control matters most.
    let filter_accordion: Element<'_, Message> = Accordion::new(
        column![reveal_chip(state, r), filter_bar(state, r)].spacing(spacing::XS),
        r,
    )
    .open(state.sidebar_filter_open)
    .into();

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
                .on_press(Message::SidebarFiltersCleared),
            ]
            .spacing(spacing::XS)
            .into(),
        )
    } else {
        None
    };

    let tree: Element<'_, Message> = TreeView::new(build_items(state, entries, r), r)
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
    .on_scroll_offset(Message::SidebarScrolled)
    // Feature 024: the reveal has to know whether its row is inside the viewport, and iced reports
    // no child position — so the geometry is computed, and this is the one input that is not
    // already in state. Reported from a sensor rather than from `on_scroll`, which fires only when
    // something scrolls; the frame that matters is the first one after a switch, where nothing has.
    .on_viewport_resize(|size| {
        Message::SidebarViewportResized(crate::app::scroll_offset_px(size.height))
    })
    // Addressable so `operation::scroll_to` can reach it. On the scrollable itself, never on the
    // sensor wrapping it — a wrapper that does not forward `operate` swallows scroll operations for
    // its whole subtree (`ui/material/ripple.rs`).
    .id(SIDEBAR_SCROLL_ID.clone())
    .into();

    // Minimal left/right padding to maximize name/tag width (FR-009); a little vertical breathing
    // room is kept.
    let content = column![header, filter_accordion, body]
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

/// The collapsed sidebar: a thin vertical strip hosting the "show sidebar" button (with a
/// tooltip), wide enough for the icon.
pub fn collapsed_strip(scheme: micold_core::theme::ColorScheme) -> Element<'static, Message> {
    let r = tokens::roles(scheme);
    let show = Tooltip::new(
        IconButton::new(Icon::ShowSidebar, r)
            .compact()
            .tint(r.on_surface_variant)
            .on_press(Message::SidebarToggled),
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
        return Text::new("No tags to filter yet.", TypeRole::SidebarTag, r)
            .muted()
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
            Button::with_content(
                Text::new("Clear filters", TypeRole::SidebarTag, r),
                ButtonVariant::Text,
                r,
            )
            .padding(spacing::XS)
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
        Tag::Issue(key) => (key.clone(), r.issue_tag().0),
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

/// The hover-revealed row-action cluster for a worktree (feature 008): an add-session "+" (only
/// when a session can start) and a trash icon that requests deletion. The cluster is rendered on
/// EVERY row so its width is always reserved (no reflow when it appears); `active` (the hovered
/// row) both gates whether the icons are pressable and is the reveal's destination.
fn row_actions_cluster(
    dir: &str,
    can_start_session: bool,
    active: bool,
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
            r,
        ));
    }
    cluster = cluster.push(action_icon(
        Icon::Delete,
        r.error,
        Message::WorktreeDeleteRequested(dir.to_string()),
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

/// Flatten the sidebar's location list (feature 010: the "Default" entry, then worktrees) into
/// ordered [`TreeItem`]s, each followed by its sessions when expanded.
fn build_items(
    state: &State,
    entries: Vec<crate::features::sidebar::SidebarEntry>,
    r: Roles,
) -> Vec<TreeItem<'static, Message>> {
    let mut items = Vec::new();
    let hovered = state.hovered_worktree.as_deref();
    let project_root = state.workspace.active.as_deref();

    for entry in entries {
        let node = match entry {
            crate::features::sidebar::SidebarEntry::Default(node) => {
                items.extend(build_default_item(state, &node, r));
                continue;
            }
            crate::features::sidebar::SidebarEntry::Worktree(node) => node,
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
            item = item.row_tooltip(crate::features::sidebar::worktree_location_label(root, wt));
        }

        // Always reserve the action cluster's width so hovering never reflows the row; each row
        // fades its icons in/out independently via its own animation track (feature 008). The
        // hovered row is the pressable one.
        let active = hovered == Some(dir.as_str());
        item = item.trailing_element(row_actions_cluster(&dir, wt.can_start_session(), active, r));
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
    session: &micold_core::session::Session,
    active_session: Option<micold_core::session::SessionId>,
    r: Roles,
) -> TreeItem<'static, Message> {
    let tint = match session.lifecycle {
        SessionLifecycle::Failed => r.error,
        SessionLifecycle::Idle => r.on_surface_variant,
        // Interrupted-but-resumable reads as needing attention, distinct from a plain idle stop
        // (FR-006a: "visibly different from both running and a deliberately stopped session").
        SessionLifecycle::InterruptedResumable => r.primary,
        _ => r.on_surface,
    };
    let selected = active_session == Some(session.id);
    // No leading icon: the activity dot below is the row's *sole* leading indicator (FR-016f).
    // A session row used to carry an unconditional `Icon::ActiveMarker` (`check_circle`) here — a
    // feature-005 leftover from that icon's real job, marking the *active known project*. It never
    // varied with session state, so it read as "done / OK" on a failed or interrupted session while
    // competing with the dot that does vary (BUG-005). Lifecycle still reaches the user: `tint`
    // above colours the label itself, not just a glyph.
    TreeItem::new(1, session.label.display().to_string(), tint)
        // The derived activity dot beside the name (feature 010 US2, FR-016d): Working/AwaitingInput
        // show a filled dot, Ended a hollow one, Unknown nothing (ambient — H2) in a slot that stays
        // the same width either way, so names stay aligned as signals change (FR-016f).
        .badge(ActivityBadge::<Message>::new(session.activity.clone(), r))
        .selected(selected)
        .on_press(Message::SessionSelected(session.id))
        .on_right_press(Message::SessionMenuToggled(session.id))
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
    node: &crate::features::sidebar::DefaultNode,
    r: Roles,
) -> Vec<TreeItem<'static, Message>> {
    let mut items = Vec::new();

    // `active: true` gives the always-visible, always-pressable button this row needs (see the doc
    // comment above), reusing the same construction as a worktree row's hover-revealed action icons
    // instead of hand-rebuilding it. It is deliberately not wrapped in `HoverReveal`: this row's
    // action is always shown, so it has nothing to reveal.
    let start_session = action_icon(
        Icon::AddSession,
        r.primary,
        Message::SessionStartRequested {
            location: SessionLocation::Default,
        },
        "Start a new session in the project root",
        true,
        r,
    );

    let item = TreeItem::new(0, node.display_name.to_string(), r.on_surface)
        // Distinct icon (FR-006): never the git/branch iconography used for worktree rows.
        .with_icon(Icon::ProjectRoot)
        .expandable(node.expanded, Message::DefaultExpansionToggled)
        .trailing_element(start_session)
        // Location tooltip (FR-010): fixed, since the Default entry is always the project root.
        .row_tooltip(crate::features::sidebar::DEFAULT_LOCATION_LABEL);
    items.push(item);

    if node.expanded {
        for session in &node.sessions {
            items.push(session_tree_item(session, state.active_session, r));
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::protocol::messages::ActivitySignal;
    use micold_core::session::Session;
    use micold_core::theme::ColorScheme;

    fn session(activity: ActivitySignal, lifecycle: SessionLifecycle) -> Session {
        let mut s = Session::start_new(SessionLocation::Worktree("feat-a".to_string()));
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
            SessionLifecycle::Failed,
            SessionLifecycle::InterruptedResumable,
        ] {
            let s = session(ActivitySignal::Unknown, lifecycle);
            let item: TreeItem<'_, Message> = session_tree_item(&s, None, r);
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
        let tint = |l| session_tree_item(&session(ActivitySignal::Unknown, l), None, r).tint;

        assert_eq!(tint(SessionLifecycle::Failed), r.error);
        assert_eq!(tint(SessionLifecycle::Idle), r.on_surface_variant);
        assert_eq!(tint(SessionLifecycle::InterruptedResumable), r.primary);
        assert_eq!(tint(SessionLifecycle::Running), r.on_surface);
    }
}
