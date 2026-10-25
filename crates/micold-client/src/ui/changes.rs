//! The Changes view (feature 482, contracts/changes-view.md V2, L1–L4): glue only.
//!
//! What the view says — the base line, the empty states, whether the Committed toggle can be used —
//! is decided by the render-free `features::changes`; this module lays it out from library
//! components. It stands in the terminal pane's place while open (V1).

use micold_core::review::comment::Scope;
use std::collections::BTreeMap;

use iced::widget::{column, container, row, stack, text_editor, Row, Space};
use iced::{Alignment, Element, Length, Padding};
use micold_core::review::changes::{ChangeKind, ChangedFile, Content};
use micold_core::review::comment::{ReviewComment, Status};
use micold_core::review::Side;
use micold_core::session::SessionLocation;
use micold_core::settings::DiffLayout;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, anatomy, spacing, Rgb, Roles};
use micold_core::typeahead::{match_one, Query};

use crate::app::{EditorAction, Message, State};
use crate::features::changes::{
    self as changes, adjacent, base_line, browse_body, can_go_back, card_model,
    changed_since_you_looked, composer_in_drawer, composer_placed, diff_body, discard_prompt,
    drawer_counts, drawer_groups, drawer_rows, file_comments, file_diff_available, file_reviews,
    list_body, mark_control, pending_counts, picker_matches, pr_header, progress_label, review_bar,
    review_of, reviews_progress, selected_file, source_control_tooltip, source_menu_input,
    stale_notice, stop_message, tree_pane, tree_rows, whole_file, BrowseBody, ComposerTarget,
    DiffBody, DrawerRow, ListBody, Load, Msg, OpenView, ReviewBarModel, Source, Step, TreeKind,
    TreePane, WholeFile, NO_LONGER_CHANGED, WHOLE_FILE_UNAVAILABLE,
};
use crate::icons::{icon_role, Icon, IconSurface};
use crate::ui::cdk::overlay::Surface;
use crate::ui::material::shaped_text::width_of;
use crate::ui::material::{
    menu_panel_size, overflow_trigger_right, tab_content_colour, BrowseBar, Button, ButtonVariant,
    CardState, ChangeBar, DiffView, Ellipsized, Glyph, IconButton, MenuItem, MenuOverlay,
    ProgressBar, PullRequestIndicator, ReviewBar, ReviewCommentCard, ReviewPanelRow, Scrollable,
    SideSheet, StageProgress, Tab, TabStrip, Tag, Text, TextArea, ToggleChip, Tooltip,
    TooltipPosition, TreeItem, TreeView, TypeRole, Typeahead, TypeaheadRow, UnreadMark,
    VirtualRows, CHANGE_BAR_WIDTH, REVIEW_BAR_COMMENTS as COMMENTS,
};
use micold_core::review::drawer::{DrawerEvent, DrawerTab, START_HINT};
use micold_core::review::marks::{FileReview, ReviewState};
use micold_core::review::tree::{self, BarParts, RowKind};
use micold_core::review::RelPath;
use micold_core::task_progress::Ratio;

/// The heading of the comments whose lines the diff on screen does not show (C3).
pub const NOT_IN_DIFF: &str = "Not in the current diff";
/// Above a new comment whose lines a refresh removed (C4).
/// A pull request GitHub will not diff (above 300 files or 20 000 changed lines): not an error, so
/// not a notice (FR-008).
pub const PR_TOO_LARGE: &str = "This pull request is too large to show here";
/// A pull request whose diff could not be read; pressing its row again retries.
pub const PR_UNREAD: &str = "The pull request's diff could not be read; select it again to retry";
pub const UNPLACED: &str = "The lines this comment was on are gone. Pick lines to place it.";

/// One tree row's height: fixed, which is what lets `VirtualRows` build only the visible rows. Two
/// lines: a file's name over its tags, bar and counts; a folder's label over its totals (feature
/// 1273, SC-006).
pub const ROW_HEIGHT: f32 = 48.0;

/// The header's *n of m reviewed* slot at its widest: laid out the same width shown or empty, so
/// the header's other controls never move with it (feature 1271, FR-009). In a narrow header it
/// takes half the room the title and the source control leave (feature 1272), and its bar, then
/// its words, give way first.
pub const PROGRESS_SLOT_WIDTH: f32 = 260.0;
/// The progress bar's width in that slot at its widest (research R9).
pub const PROGRESS_BAR_WIDTH: f32 = 120.0;

/// The file column's width, whatever the window's (feature 1272, FR-008, R8): the diff takes the
/// rest.
pub const FILE_COLUMN_WIDTH: f32 = 260.0;
/// The header's source control's width, which its open list takes too: room for the Working tree
/// option's words beside its two chips on one line, and for `Files · #12345 · ` and some of a
/// title when closed. Kept at every window width while the title column gives way (Edge "Narrow
/// window").
pub const SOURCE_CONTROL_WIDTH: f32 = 360.0;

/// The diff's scroll viewport, by name: a panel jump scrolls it (feature 946, FR-017). A
/// `LazyLock` for the reason [`crate::ui::SIDEBAR_SCROLL_ID`] is one.
pub static DIFF_SCROLL_ID: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("changes-diff"));

/// The comments drawer's list viewport, by name: shown again, it is scrolled back to its tab's
/// offset (feature 1270, FR-019).
pub static DRAWER_SCROLL_ID: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("changes-drawer"));

/// The Changes view of `view`'s entry. `composer` is the review composer's editor, which only the
/// binary holds; without it an open composer shows its text read-only. `connected`: the session
/// service that keeps the marks is reachable (feature 1271, FR-013).
///
/// Top to bottom (feature 1270): the header, the file list beside the diff area — over whose right
/// side the comments drawer is stacked while open — and the review bar, the tab's last row, at full
/// width directly above the status bar (FR-001, FR-010).
pub fn view<'a>(
    state: &'a State,
    view: &'a OpenView,
    scheme: ColorScheme,
    composer: Option<&'a text_editor::Content>,
    connected: bool,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let title = match &view.entry {
        SessionLocation::Default => crate::features::sidebar::DEFAULT_LOCATION_LABEL.to_string(),
        SessionLocation::Worktree(dir) => state.worktree_display_name(dir),
    };

    // V2: the entry and the base (or why there is none). FR-009: **Review (n)** and **Send
    // review** moved to the bar; feature 1269 (FR-007, U13): the Review tab's × closes the review.
    // Feature 1272 (Edge "Narrow window"): the base line gives way with an ellipsis before the
    // source control does; the control's tooltip holds it whole (FR-009).
    let mut heading = column![Text::new(title, TypeRole::Title, r)].spacing(spacing::XS);
    if let Some(line) = base_line(view) {
        heading = heading.push(Ellipsized::<Message>::at_role_spaced(
            line,
            TypeRole::Body,
            r.on_surface_variant,
        ));
    }
    // Feature 1271: the rows' review states, judged once for the list and the header, whose
    // *n of m reviewed* slot sits beside the heading (FR-008, FR-009).
    let reviews = file_reviews(&state.changes, view, connected);
    let mut controls = row![container(heading).width(Length::FillPortion(1))]
        .spacing(spacing::MD)
        .align_y(Alignment::Center);
    // Feature 1275 (FR-007): what changed while the list and the diff were held, and the way to
    // show it.
    if let Some(notice) = stale_notice(view) {
        controls =
            controls.push(Button::text(notice, r).on_press(Message::Changes(Msg::StaleApplied)));
    }
    let header = container(
        controls
            .push(source_control(state, view, r))
            .push(progress_slot(reviews_progress(&reviews, connected), r)),
    )
    .width(Length::Fill)
    .padding(spacing::LG);

    // FR-010, SC-002: the drawer covers the diff area's right side without changing the diff's
    // layout. Always a stack, open or closed, so the diff keeps its widget state across a toggle.
    let mut diff_area = stack![
        container(diff_pane(state, view, scheme, composer, connected)).padding(Padding {
            right: spacing::LG,
            bottom: spacing::LG,
            ..Padding::ZERO
        })
    ]
    .width(Length::Fill)
    .height(Length::Fill);
    if view.drawer.state.open {
        diff_area = diff_area.push(comments_drawer(state, view, composer, r));
    }
    // Feature 1272 (FR-008, R8): two columns, the file column at its fixed width and the diff area
    // taking the rest, so a wider or narrower window changes only the diff's width.
    let body = row![
        container(list_pane(state, view, reviews, r))
            .width(Length::Fixed(FILE_COLUMN_WIDTH))
            .padding(Padding {
                left: FILE_COLUMN_INSET,
                bottom: spacing::LG,
                ..Padding::ZERO
            }),
        diff_area,
    ]
    .spacing(spacing::LG)
    .height(Length::Fill);

    // No gap above the bar: the drawer runs down to it (D2).
    let mut tab = column![header, body]
        .width(Length::Fill)
        .height(Length::Fill);
    if let Some(bar) = review_bar(&state.changes) {
        tab = tab.push(bar_of(bar, r));
    }
    tab.into()
}

/// The review's source control (feature 1272, FR-001, R8): the shown source as its label, the
/// working tree with its kind chips, the pull requests and Files in its list, the base line in its
/// tooltip. A press on a row or chip sends what the stop stands for; the keyboard walks the stops.
fn source_control<'a>(state: &'a State, view: &'a OpenView, r: Roles) -> Element<'a, Message> {
    let enabled = state.changes.pr_enabled;
    let now = super::sidebar::unix_now();
    let send = |msg: Msg| Message::Changes(msg);
    let select = crate::ui::source_menu::control(
        &source_menu_input(view, enabled, now),
        view.source_menu.open,
        view.source_menu.highlight,
        crate::ui::source_menu::Control {
            on_toggle: send(Msg::SourceMenuToggled),
            on_dismiss: send(Msg::SourceMenuDismissed),
            on_pick: send(Msg::SourceMenuPicked),
        },
        move |stop| send(stop_message(stop)),
        move |direction| send(Msg::SourceMenuMoved(direction)),
        source_control_tooltip(view, enabled, now),
        r,
    );
    container(select)
        .width(Length::Fixed(SOURCE_CONTROL_WIDTH))
        .into()
}

/// *n of m reviewed* and its bar (feature 1271, FR-008), in a slot whose width follows the header's
/// only, laid out empty when there is no progress to show (FR-009).
fn progress_slot<'a>(ratio: Option<Ratio>, r: Roles) -> Element<'a, Message> {
    let content: Element<'a, Message> = match ratio {
        Some(ratio) => row![
            Ellipsized::<Message>::at_role_spaced(
                progress_label(ratio),
                TypeRole::Body,
                r.on_surface_variant
            )
            .shrink(),
            // Fixed, not Fill: this row is Shrink, where a Fill bar is laid out at zero width
            // (1271 visual re-pass V2). Laid out after the words, a fixed length still gives way
            // to them: it is clamped to what they leave.
            container(ProgressBar::new(ratio, r).width(Length::Fill))
                .width(Length::Fixed(PROGRESS_BAR_WIDTH)),
        ]
        .width(Length::Shrink)
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .into(),
        None => Space::new().into(),
    };
    let slot = container(content)
        .width(Length::Fill)
        .max_width(PROGRESS_SLOT_WIDTH)
        .align_x(iced::alignment::Horizontal::Right);
    container(slot)
        .width(Length::FillPortion(1))
        .align_x(iced::alignment::Horizontal::Right)
        .into()
}

/// The review bar (feature 1270, B1–B8): the shown diff's counts, the send error with its ×, and
/// **Comments**, ⋮ and **Send review (n)** at the trailing edge.
fn bar_of<'a>(bar: ReviewBarModel, r: Roles) -> Element<'a, Message> {
    let mut element = ReviewBar::new(bar.counts, bar.send, r)
        .drawer_open(bar.drawer_open)
        .on_send(Message::Changes(Msg::SendPressed))
        .on_overflow(Message::Changes(Msg::OverflowToggled))
        .on_toggle_drawer(Message::Changes(Msg::Drawer(DrawerEvent::Toggle)))
        .on_dismiss_error(Message::Changes(Msg::SendErrorDismissed));
    if let Some(error) = bar.error {
        element = element.error(error);
    }
    element.into()
}

/// The bar's overflow menu (feature 1270, T022, B7): **Clear sent** and **Discard drafts…**, each
/// pressable only when `Overflow` allows it, hung from the ⋮ and rising above the bar. `None` while
/// no Review tab is shown; closed (and fading out) while the menu is not open.
pub fn overflow_menu<'a>(state: &'a State, r: Roles) -> Option<Surface<'a, Message>> {
    let view = state.changes.tabs.shown_view()?;
    let bar = review_bar(&state.changes)?;
    let items: Vec<MenuItem<Message>> = OVERFLOW_ITEMS
        .into_iter()
        .zip([
            bar.overflow.clear_sent.then_some(Msg::ClearSentPressed),
            bar.overflow
                .discard_drafts
                .then_some(Msg::DiscardPendingPressed),
        ])
        .map(|(label, msg)| MenuItem {
            message: msg.map(Message::Changes),
            ..MenuItem::labeled(label, Message::Changes(Msg::OverflowDismissed))
        })
        .collect();
    let panel = menu_panel_size(items.len());
    let (width, height) = state.window.window_size;
    // The bar's right edge is the window's and its bottom the status bar's top (FR-001), so the
    // panel's bottom-right corner sits on the ⋮'s top-right without a press reporting it.
    let hung = (width > 0 && height > 0).then(|| {
        let right = overflow_trigger_right(f32::from(width));
        let bar_top = f32::from(height) - anatomy::status_bar::HEIGHT - anatomy::review_bar::HEIGHT;
        iced::Point::new(
            (right - f32::from(panel.0)).max(0.0),
            (bar_top - f32::from(panel.1)).max(0.0),
        )
    });
    Some(
        MenuOverlay::new(items, Message::Changes(Msg::OverflowDismissed), r)
            .open(view.overflow_open)
            .hung_from(hung)
            .into(),
    )
}

/// The overflow menu's items, in order (B7).
pub const OVERFLOW_ITEMS: [&str; 2] = ["Clear sent", "Discard drafts\u{2026}"];

/// The comments drawer (feature 1270, D2–D9, T042, T043): a `SideSheet` over the diff area's
/// right, from under the header to the bar; its Drafts and Replies tabs with their counts, a
/// refused jump's reason, and the selected tab's rows or its empty text.
fn comments_drawer<'a>(
    state: &'a State,
    view: &'a OpenView,
    composer: Option<&'a text_editor::Content>,
    r: Roles,
) -> Element<'a, Message> {
    let tab = view.drawer.state.tab;
    let (drafts, replies) = drawer_counts(&state.changes);
    let tab_of = |of: DrawerTab, n: usize| {
        let active = of == tab;
        Tab::new(
            Text::new(of.label(n), TypeRole::Label, r).tint(tab_content_colour(active, r)),
            r,
        )
        .content_sized()
        .active(active)
        .on_press(Message::Changes(Msg::Drawer(DrawerEvent::SelectTab(of))))
    };
    let tabs = TabStrip::new(
        vec![
            tab_of(DrawerTab::Drafts, drafts),
            tab_of(DrawerTab::Replies, replies),
        ],
        r,
    );
    let mut pane = column![tabs].spacing(spacing::SM).height(Length::Fill);
    if let Some(why) = view.drawer.hidden {
        pane = pane.push(Text::new(why.message(), TypeRole::Body, r));
    }
    let rows = drawer_rows(&state.changes, tab);
    if rows.is_empty() {
        // D9: a tab with no rows says so; with no comment at all, today's hint follows.
        pane = pane.push(Text::new(tab.empty_text(), TypeRole::Body, r).muted());
        if drafts + replies == 0 {
            pane = pane.push(Text::new(START_HINT, TypeRole::Body, r).muted());
        }
    } else {
        // A pull request's heading leads each of its rows' labels.
        let headings: BTreeMap<Scope, String> = drawer_groups(&state.changes)
            .into_iter()
            .filter_map(|group| Some((group.scope, group.heading()?)))
            .collect();
        // Clear of the scrollbar, which is drawn over the content's right edge.
        let mut list = column![].spacing(spacing::SM).padding(Padding {
            right: spacing::MD,
            ..Padding::ZERO
        });
        for item in rows {
            let heading = headings.get(&item.scope).map(String::as_str);
            list = list.push(drawer_row(view, item, heading, composer, r));
        }
        // Rows differ in height (a whole reply, a composer), so the list is a plain scrolling
        // column rather than `VirtualRows`; it reports its offset per tab (FR-019).
        pane = pane.push(
            Scrollable::new(list, r)
                .id(DRAWER_SCROLL_ID.clone())
                .height(Length::Fill)
                .on_scroll_metrics(move |offset, viewport, _| {
                    Message::Changes(Msg::DrawerScrolled {
                        tab,
                        offset,
                        viewport,
                    })
                }),
        );
    }
    SideSheet::new(
        container(pane).padding(Padding {
            left: spacing::MD,
            right: spacing::MD,
            bottom: spacing::MD,
            top: 0.0,
        }),
        r,
    )
    .width(anatomy::comments_drawer::WIDTH)
    .title(COMMENTS)
    .on_close(Message::Changes(Msg::Drawer(DrawerEvent::Close)))
    .into()
}

/// One row of the drawer (D6, D7): a Drafts row with **Edit** and **Delete** while no send holds
/// it; a Replies row with its status, the whole reply (Show all past 4,000 characters) and, on a
/// Sent row, **Mark applied** and **Answer…**. A composer the drawer opened (`ComposerPlace::Drawer`)
/// is drawn under its row.
fn drawer_row<'a>(
    view: &'a OpenView,
    item: DrawerRow,
    heading: Option<&str>,
    composer: Option<&'a text_editor::Content>,
    r: Roles,
) -> Element<'a, Message> {
    let place = format!("{} · {}", item.path.as_str(), item.lines);
    let label = match heading {
        Some(heading) => format!("{heading} · {place}"),
        None => place,
    };
    let id = item.id;
    let mut row_ = ReviewPanelRow::new(label, item.text, item.status, r)
        .selected(item.selected)
        .not_in_diff(item.not_in_diff)
        .on_press(Message::Changes(Msg::DrawerRowSelected(id)));
    if item.status == Status::Draft {
        if let Some(reply) = item.reply {
            row_ = row_.reply(reply);
        }
    } else {
        row_ = row_.whole_reply(item.reply.unwrap_or_default(), item.has_full_reply);
        if item.has_full_reply {
            row_ = row_.on_show_reply(Message::Changes(Msg::ShowReply(id)));
        }
    }
    if item.editable {
        row_ = row_
            .on_edit(Message::Changes(Msg::DrawerEdit(id)))
            .on_delete(Message::Changes(Msg::DeleteComment(id)));
    }
    if item.markable {
        row_ = row_
            .on_mark_applied(Message::Changes(Msg::MarkApplied(id)))
            .on_answer(Message::Changes(Msg::DrawerAnswer(id)));
    }
    let row_: Element<'a, Message> = row_.into();
    let Some(open) = view.composer.as_ref().filter(|_| composer_in_drawer(view)) else {
        return row_;
    };
    let placeholder = match open.target {
        ComposerTarget::Edit(target) if target == id => "Comment",
        ComposerTarget::Answer(target) if target == id => "Why not, or what you did",
        _ => return row_,
    };
    column![
        row_,
        composer_box(&open.text, composer, true, placeholder, r)
    ]
    .into()
}

/// The left pane (feature 1272, FR-003): the file list or its empty state (L2, L3), or, with
/// Files on, the repository's tree. The source and the kinds are picked in the header's control.
fn list_pane<'a>(
    state: &'a State,
    view: &'a OpenView,
    reviews: BTreeMap<RelPath, FileReview>,
    r: Roles,
) -> Element<'a, Message> {
    let pane = column![].width(Length::Fill).height(Length::Fill);
    if view.files.shown {
        return pane.push(files_pane(view, r)).into();
    }
    let list: Element<'a, Message> = match list_body(view) {
        ListBody::Loading => Text::new("Reading changes…", TypeRole::Body, r)
            .muted()
            .into(),
        ListBody::Failed(message) => Text::new(message, TypeRole::Body, r).tint(r.error).into(),
        ListBody::Empty(sentence) => Text::new(sentence, TypeRole::Body, r).muted().into(),
        ListBody::PullRequestUnread { too_large } => {
            let mut lines = column![Text::new(
                if too_large { PR_TOO_LARGE } else { PR_UNREAD },
                TypeRole::Body,
                r
            )
            .muted()]
            .spacing(spacing::SM);
            if let (true, Source::PullRequest(pr)) = (too_large, &view.source) {
                lines = lines.push(
                    Button::outlined("Open on GitHub", r)
                        .on_press(Message::Changes(Msg::PullRequestOpenRequested(pr.number))),
                );
            }
            lines.into()
        }
        ListBody::Files(files) => match tree_pane(view) {
            // M5 draws the no-match line; nothing of the tree shows unless it has rows (FR-016).
            TreePane::Hidden | TreePane::NoMatch => Space::new().into(),
            TreePane::Rows => change_tree(state, view, files, reviews, r),
        },
    };
    pane.push(list).into()
}

/// The repository's files in place of the changed ones (FR-011): a path picker over the file tree.
fn files_pane<'a>(view: &'a OpenView, r: Roles) -> Element<'a, Message> {
    let tree: Element<'a, Message> = match &view.files.tree {
        Load::Failed(message) => Text::new(message.clone(), TypeRole::Body, r)
            .tint(r.error)
            .into(),
        Load::Idle | Load::Loading { last: None, .. } => {
            Text::new("Reading files…", TypeRole::Body, r)
                .muted()
                .into()
        }
        _ => {
            let items: Vec<TreeItem<'a, Message>> = tree_rows(view)
                .into_iter()
                .map(|row| match row.kind {
                    TreeKind::Folder { path, open } => {
                        TreeItem::new(row.depth, row.name, r.on_surface)
                            .expandable(open, Message::Changes(Msg::FolderToggled(path)))
                    }
                    TreeKind::File(path) => TreeItem::new(row.depth, row.name, r.on_surface)
                        .selected(view.browse.as_ref().is_some_and(|b| b.path == path))
                        .on_press(Message::Changes(Msg::BrowseOpen(path))),
                })
                .collect();
            crate::ui::material::Scrollable::new(TreeView::new(items, r), r)
                .height(Length::Fill)
                .into()
        }
    };
    let matches = picker_matches(view);
    let query = Query::new(view.files.picker.trim());
    let rows: Vec<TypeaheadRow> = matches
        .iter()
        .map(|path| {
            let spans = match_one(path.as_str(), &query)
                .map(|m| m.spans)
                .unwrap_or_default();
            TypeaheadRow::new(path.as_str(), spans)
        })
        .collect();
    let picker = Typeahead::new(
        &view.files.picker,
        rows,
        |text| Message::Changes(Msg::PathPickerChanged(text)),
        r,
    )
    .placeholder("Open file by path…")
    .in_flow(true)
    .open(!view.files.picker.trim().is_empty())
    .on_pick(move |index| {
        matches.get(index).cloned().map_or(
            Message::Changes(Msg::PathPickerChanged(String::new())),
            |p| Message::Changes(Msg::BrowseOpen(p)),
        )
    });
    let mut pane = column![picker];
    if view.files.truncated {
        pane = pane.push(
            Text::new(
                "GitHub listed only part of this tree; type a path to open any file.",
                TypeRole::Caption,
                r,
            )
            .muted(),
        );
    }
    pane.push(tree)
        .spacing(spacing::SM)
        .height(Length::Fill)
        .into()
}

/// A browsed file in the diff pane: its bar over the file's lines, or why they are not shown.
fn browsed_pane<'a>(
    view: &'a OpenView,
    body: BrowseBody<'a>,
    layout: DiffLayout,
    scheme: ColorScheme,
    r: Roles,
) -> Element<'a, Message> {
    let path = view.browse.as_ref().map_or("", |b| b.path.as_str());
    let mut bar = BrowseBar::new(path, r);
    if can_go_back(view) {
        bar = bar.on_back(Message::Changes(Msg::BrowseBack));
    }
    if file_diff_available(view) {
        bar = bar.on_open_diff(Message::Changes(Msg::OpenFileDiff));
    }
    let content: Element<'a, Message> = match body {
        BrowseBody::Reading => StageProgress::new("Loading file…", r).into(),
        BrowseBody::Failed { message, large } => {
            let mut lines =
                column![Text::new(message, TypeRole::Body, r).tint(r.error)].spacing(spacing::SM);
            if large {
                lines = lines.push(
                    Button::outlined("Show anyway", r).on_press(Message::Changes(Msg::ShowLarge)),
                );
            }
            lines.into()
        }
        BrowseBody::File(loaded) => {
            let spans = match scheme {
                ColorScheme::Light => &loaded.spans.light,
                ColorScheme::Dark => &loaded.spans.dark,
            };
            DiffView::new(&loaded.diff, layout, r)
                .scroll_id(DIFF_SCROLL_ID.clone())
                .spans(spans)
                .offset(view.diff_offset)
                .viewport(view.diff_viewport)
                .on_scroll(|offset, viewport| {
                    Message::Changes(Msg::DiffScrolled { offset, viewport })
                })
                .references(|text| changes::references_in(view, text))
                .on_reference(|path, line| Message::Changes(Msg::ReferenceActivated(path, line)))
                .into()
        }
    };
    let mut pane = column![Element::from(bar)].spacing(spacing::SM);
    if let Some(notice) = view.notice {
        pane = pane.push(Text::new(notice, TypeRole::Caption, r).tint(r.error));
    }
    pane.push(content)
        .spacing(spacing::SM)
        .height(Length::Fill)
        .into()
}

/// The changed files as a tree (feature 1273, contract §2.3): [`OpenView::change_rows`] on
/// `VirtualRows`, so only the rows on screen are built (FR-015).
fn change_tree<'a>(
    state: &'a State,
    view: &'a OpenView,
    files: &'a [ChangedFile],
    reviews: BTreeMap<RelPath, FileReview>,
    r: Roles,
) -> Element<'a, Message> {
    let selected = selected_file(view);
    let counts = pending_counts(&state.changes);
    let rows = &view.change_rows;
    let scale = view.change_scale;
    let cap = *DEFAULT_INDENT_CAP;
    VirtualRows::new(
        rows.len(),
        ROW_HEIGHT,
        move |index| {
            let row = &rows[index];
            let levels = tree::indent_levels(row.depth, cap);
            match &row.kind {
                RowKind::Folder(folder) => folder_row(row, folder, levels, r),
                RowKind::File { leaf } => {
                    // Rows are laid out from these files by `update`; a row that no longer names
                    // one draws nothing rather than panicking mid-draw.
                    let Some(file) = files.get(*leaf) else {
                        return Space::new().into();
                    };
                    let bar = counted(file)
                        .then(|| tree::bar(file.added, file.removed, scale, CHANGE_BAR_WIDTH))
                        .flatten();
                    let review = reviews.get(&file.path).copied();
                    let facts = FileFacts {
                        selected: selected == Some(&file.path),
                        pending: counts.get(&file.path).copied().unwrap_or(0),
                        review,
                        dot: RowDot::of(review, changed_since_you_looked(view, &file.path)),
                        bar,
                    };
                    file_row(file, &row.label, levels, facts, r)
                }
            }
        },
        r,
    )
    .offset(view.list_offset)
    .viewport(view.list_viewport)
    .on_scroll(|offset, viewport| Message::Changes(Msg::ListScrolled { offset, viewport }))
    .into()
}

/// Whether a file's `+a −r` are line counts the bar draws: a text change other than a Mode one
/// (FR-006), as the tree's leaves count them.
fn counted(file: &ChangedFile) -> bool {
    file.content == Content::Text && !matches!(file.kind, ChangeKind::ModeOnly)
}

/// The file column's leading inset: the list's rows are [`FILE_COLUMN_WIDTH`] less it wide.
const FILE_COLUMN_INSET: f32 = spacing::LG;
/// How wide the list's rows are in the fixed file column (feature 1273, SC-006).
pub const FILE_LIST_WIDTH: f32 = FILE_COLUMN_WIDTH - FILE_COLUMN_INSET;
/// The changed-file tree's indentation step: narrower than the `TreeView`'s 16 px so a tree three
/// levels deep still indents in the 236 px list (ledger decision 11).
pub const TREE_INDENT_STEP: f32 = spacing::SM + spacing::XS;
/// A tree row's leading padding: small, as the indentation is the row's leading space; the
/// trailing padding stays the button's, where the changed dot is drawn.
const TREE_ROW_LEAD: f32 = spacing::XS;
/// A 24-character file name: names up to that length show whole at the default list width
/// (SC-006). Measured for the room a name keeps before the indentation stops growing (edge case
/// "Very deep paths").
const NAME_SAMPLE: &str = "review_file_tree_test.rs";
/// A file row's name: emphasised, the row's most prominent text (FR-005).
const NAME_ROLE: TypeRole = TypeRole::Action;

/// The room a row's name has at depth 0 in a list `list_width` wide: the list less the row's
/// padding and the review state's slot.
pub fn name_room(list_width: f32) -> f32 {
    list_width
        - TREE_ROW_LEAD
        - anatomy::button::PADDING_TEXT
        - (Glyph::<Message>::extent(TypeRole::Body) + spacing::SM)
}

/// How many indentation steps a row may take in a list `list_width` wide, so a
/// 24-character name keeps its room (SC-006).
pub fn tree_indent_cap(list_width: f32) -> u16 {
    tree::max_indent_levels(
        name_room(list_width),
        TREE_INDENT_STEP,
        width_of(NAME_SAMPLE, NAME_ROLE),
    )
}

/// [`tree_indent_cap`] of the fixed file column, shaped once rather than on every draw.
static DEFAULT_INDENT_CAP: std::sync::LazyLock<u16> =
    std::sync::LazyLock::new(|| tree_indent_cap(FILE_LIST_WIDTH));

/// The indentation of a row `levels` steps in.
fn indent(levels: u16) -> Space {
    Space::new().width(Length::Fixed(f32::from(levels) * TREE_INDENT_STEP))
}

/// `+a −r`, as a row's caption.
fn counts_caption(added: u64, removed: u64, r: Roles) -> Text<'static, Message> {
    Text::new(format!("+{added} \u{2212}{removed}"), TypeRole::Caption, r)
        .muted()
        .wrapping(iced::widget::text::Wrapping::None)
}

/// A folder row (FR-001–FR-003, FR-007): its disclosure chevron and label over the summed
/// `+a −r` of the files listed below it; a press folds or unfolds it.
fn folder_row(
    row: &tree::Row,
    folder: &tree::FolderRow,
    levels: u16,
    r: Roles,
) -> Element<'static, Message> {
    let chevron = if folder.open {
        Icon::SelectChevron
    } else {
        Icon::ChevronRight
    };
    let text = column![
        Ellipsized::<Message>::at_role(row.label.clone(), TypeRole::Body, r.on_surface),
        counts_caption(folder.added, folder.removed, r),
    ];
    let line = row![
        indent(levels),
        Glyph::new(chevron, TypeRole::Body, r).tint(r.on_surface_variant),
        text,
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
    container(
        Button::with_content(
            container(line).width(Length::Fill).clip(true),
            ButtonVariant::Text,
            r,
        )
        .padding(Padding {
            left: TREE_ROW_LEAD,
            right: tokens::anatomy::button::PADDING_TEXT,
            ..Padding::ZERO
        })
        .width(Length::Fill)
        .on_press(Message::Changes(Msg::TreeFolderToggled(
            folder.paths.clone(),
        ))),
    )
    .height(Length::Fixed(ROW_HEIGHT))
    .width(Length::Fill)
    .align_y(Alignment::Center)
    .into()
}

/// What a file row shows besides the file itself.
struct FileFacts {
    selected: bool,
    pending: usize,
    review: Option<FileReview>,
    dot: Option<RowDot>,
    bar: Option<BarParts>,
}

/// One file (L2, FR-005): its review state (feature 1271, FR-001) and its name first, emphasised;
/// under them its kind tag, content tag when not text, change bar with `+a −r` when it has
/// counted lines, and its pending comments. The full path is on hover.
fn file_row(
    file: &ChangedFile,
    name: &str,
    levels: u16,
    facts: FileFacts,
    r: Roles,
) -> Element<'static, Message> {
    let variant = if facts.selected {
        ButtonVariant::Outlined
    } else {
        ButtonVariant::Text
    };
    let mut title = row![].spacing(spacing::SM).align_y(Alignment::Center);
    if let Some(review) = facts.review {
        title = title.push(review_glyph(review.state, r));
    }
    // One line, ellipsised at its end: the name keeps the row's room before the indentation
    // grows (SC-006), and a longer one is cut where the tags below cannot reach it.
    let title = title.push(Ellipsized::<Message>::at_role(
        name.to_owned(),
        NAME_ROLE,
        variant.content(r, None),
    ));
    let mut details = row![].spacing(spacing::SM).align_y(Alignment::Center);
    if let Some((kind, accent)) = kind_tag(&file.kind, r) {
        details = details.push(Tag::new(kind, accent).one_line());
    }
    match file.content {
        Content::Text => {}
        Content::Binary => details = details.push(Tag::new("Binary", r.secondary).one_line()),
        Content::NotUtf8 => details = details.push(Tag::new("Not text", r.secondary).one_line()),
    }
    if let Some(parts) = facts.bar {
        details = details
            .push(ChangeBar::new(parts, r).max_width(CHANGE_BAR_WIDTH))
            .push(counts_caption(
                u64::from(file.added),
                u64::from(file.removed),
                r,
            ));
    }
    if facts.pending > 0 {
        details = details.push(Tag::new(pending_label(facts.pending), r.tertiary).one_line());
    }
    let line = row![
        indent(levels),
        column![title, details].spacing(spacing::XS / 2.0)
    ]
    .align_y(Alignment::Center);
    let renamed_from = match &file.kind {
        ChangeKind::Renamed { from } => Some(from.as_str()),
        _ => None,
    };
    let button = container(Tooltip::new(
        // The row's inset is the list's, not the variant's: selecting a row outlines it without
        // taking 24 px from its name (1271 visual re-pass V1).
        // The content is clipped to the button's content box: in a list too narrow for the name
        // and the tags, the overflow is cut there and never reaches the padding the changed dot is
        // drawn over (1271 round 3, V4).
        Button::with_content(container(line).width(Length::Fill).clip(true), variant, r)
            .padding(Padding {
                left: TREE_ROW_LEAD,
                right: tokens::anatomy::button::PADDING_TEXT,
                ..Padding::ZERO
            })
            .width(Length::Fill)
            .on_press(Message::Changes(Msg::FileSelected(file.path.clone()))),
        tree::hover_text(file.path.as_str(), renamed_from),
        r,
    ))
    .height(Length::Fixed(ROW_HEIGHT))
    .width(Length::Fill)
    .align_y(Alignment::Center);
    let Some(dot) = facts.dot else {
        return button.into();
    };
    // FR-003: the changed dot sits in the button's trailing padding, over it rather than in the
    // row, so it takes no room from the name and the tags (1271 round 1, V2) and its tooltip is
    // not a child of the button (V1).
    let dot = container(
        Tooltip::new(UnreadMark::new(r), dot.tooltip(), r).position(TooltipPosition::Left),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::End)
    .align_y(Alignment::Center)
    .padding([0.0, CHANGED_DOT_INSET]);
    container(stack![button, dot])
        .height(Length::Fixed(ROW_HEIGHT))
        .width(Length::Fill)
        .into()
}

/// The dot's distance from the row's trailing edge, inside the button's 12 px padding.
const CHANGED_DOT_INSET: f32 = 4.0;
/// The changed dot's tooltip (feature 1271, FR-003).
pub const CHANGED_SINCE_TOOLTIP: &str = "Changed since you marked it reviewed";
/// The dot's tooltip on a row changed while the list was held (feature 1275, FR-008).
pub const CHANGED_SINCE_LOOKED_TOOLTIP: &str = "Changed since you looked";
/// The dot's tooltip on a row that is both.
pub const CHANGED_SINCE_BOTH_TOOLTIP: &str =
    "Changed since you looked, and since you marked it reviewed";

/// Why a row carries the changed dot: one dot, whichever says so (features 1271 and 1275).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowDot {
    /// Changed since the user marked it reviewed (1271, FR-003).
    Reviewed,
    /// Changed since the user looked: the list is held (1275, FR-008).
    Looked,
    /// Both.
    Both,
}

impl RowDot {
    /// The dot a row carries, if any.
    pub fn of(review: Option<FileReview>, looked: bool) -> Option<Self> {
        match (review.is_some_and(|review| review.changed_since), looked) {
            (true, true) => Some(Self::Both),
            (true, false) => Some(Self::Reviewed),
            (false, true) => Some(Self::Looked),
            (false, false) => None,
        }
    }

    /// The dot's tooltip.
    pub fn tooltip(self) -> &'static str {
        match self {
            Self::Reviewed => CHANGED_SINCE_TOOLTIP,
            Self::Looked => CHANGED_SINCE_LOOKED_TOOLTIP,
            Self::Both => CHANGED_SINCE_BOTH_TOOLTIP,
        }
    }
}

/// A row's review state: ○, ● or ✓ (feature 1271, research R9). Reviewed reads in the label's
/// own role, the other two quieter, as badges.
fn review_glyph(state: ReviewState, r: Roles) -> Element<'static, Message> {
    let (icon, surface) = match state {
        ReviewState::Unseen => (Icon::ReviewUnseen, IconSurface::Badge),
        ReviewState::HasDrafts => (Icon::ReviewDrafts, IconSurface::Badge),
        ReviewState::Reviewed => (Icon::ReviewDone, IconSurface::AppBarAction),
    };
    Glyph::new(icon, TypeRole::Body, r)
        .tint(icon_role(surface, r))
        .into()
}

/// The kind's tag and accent (L2, FR-005): none for Modified. An untracked file is new to the
/// reviewer: Added. A rename names its old path on hover, not in the tag.
fn kind_tag(kind: &ChangeKind, r: Roles) -> Option<(&'static str, Rgb)> {
    match kind {
        ChangeKind::Added | ChangeKind::Untracked => Some(("Added", r.primary)),
        ChangeKind::Modified => None,
        ChangeKind::Deleted => Some(("Deleted", r.error)),
        ChangeKind::Renamed { .. } => Some(("Renamed", r.tertiary)),
        ChangeKind::ModeOnly => Some(("Mode", r.secondary)),
    }
}

/// The diff area: while a pull request is shown, its header above (feature 1003, FR-007).
fn diff_pane<'a>(
    state: &'a State,
    view: &'a OpenView,
    scheme: ColorScheme,
    composer: Option<&'a text_editor::Content>,
    connected: bool,
) -> Element<'a, Message> {
    let body = diff_pane_body(state, view, scheme, composer, connected);
    let Some(header) = pull_request_header(view, tokens::roles(scheme)) else {
        return body;
    };
    column![header, body]
        .spacing(spacing::MD)
        .height(Length::Fill)
        .into()
}

/// The header of the shown pull request: its state and checks, `#number title`, the base line, and
/// previous/next, each disabled at an end of the list (contracts/changes-view.md § Header).
fn pull_request_header<'a>(view: &'a OpenView, r: Roles) -> Option<Element<'a, Message>> {
    let header = pr_header(view)?;
    let (prs, _) = view.pull_requests.shown()?;
    let step = |icon: Icon, step: Step, tip: &'static str| {
        Tooltip::new(
            IconButton::new(icon, r).compact().on_press_maybe(
                adjacent(prs, header.number, step)
                    .map(|_| Message::Changes(Msg::PullRequestStepped(step))),
            ),
            tip,
            r,
        )
    };
    let mut indicator = PullRequestIndicator::<Message>::new(header.mark, &r);
    if let Some(checks) = header.check {
        indicator = indicator.checks(checks);
    }
    let naming = column![
        Text::new(
            format!("#{} {}", header.number, header.title),
            TypeRole::Label,
            r
        ),
        Text::new(header.base, TypeRole::Caption, r).muted(),
    ]
    .spacing(spacing::XS);
    Some(
        row![
            Element::from(indicator),
            container(naming).width(Length::Fill),
            step(Icon::ChevronLeft, Step::Previous, "Previous pull request"),
            step(Icon::ChevronRight, Step::Next, "Next pull request"),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .into(),
    )
}

/// The right pane (D1–D4): what to do before a file is picked, the progress line while its first
/// read runs, git's message when it fails, else the diff — whose D3 messages and D4 large gate
/// `DiffView` draws.
fn diff_pane_body<'a>(
    state: &'a State,
    view: &'a OpenView,
    scheme: ColorScheme,
    composer: Option<&'a text_editor::Content>,
    connected: bool,
) -> Element<'a, Message> {
    let layout = state.changes.layout;
    let r = tokens::roles(scheme);
    if let Some(body) = browse_body(view) {
        return browsed_pane(view, body, layout, scheme, r);
    }
    // D1, FR-006: the layout is a setting, so its choice shows whatever the pane holds.
    let mut layouts: Vec<Element<'a, Message>> = vec![
        ToggleChip::new(
            "Unified",
            Message::Changes(Msg::LayoutChosen(DiffLayout::Unified)),
            r,
        )
        .active(layout == DiffLayout::Unified)
        .into(),
        ToggleChip::new(
            "Side by side",
            Message::Changes(Msg::LayoutChosen(DiffLayout::SideBySide)),
            r,
        )
        .active(layout == DiffLayout::SideBySide)
        .into(),
    ];
    // FR-004: the file's whole-file toggle sits with the layout choice.
    match whole_file(view) {
        WholeFile::Hidden => {}
        WholeFile::Unavailable => layouts.push(
            Tooltip::new(
                ToggleChip::new("Whole file", Message::Changes(Msg::WholeFileToggled), r)
                    .disabled(true),
                WHOLE_FILE_UNAVAILABLE,
                r,
            )
            .into(),
        ),
        WholeFile::Available { on } => layouts.push(
            ToggleChip::new("Whole file", Message::Changes(Msg::WholeFileToggled), r)
                .active(on)
                .into(),
        ),
    }
    // Feature 1271 (FR-002): the selected file's mark control, beside **Whole file**.
    if let Some(control) = mark_control(&state.changes, view, connected) {
        layouts.push(
            Tooltip::new(
                // Cut, not wrapped, where the pane is narrower than the label (1271 re-pass V3).
                ToggleChip::new(control.label, Message::Changes(Msg::MarkToggled), r)
                    .one_line()
                    .active(control.pressed)
                    .disabled(control.block.is_some()),
                control.tooltip(),
                r,
            )
            .into(),
        );
    }
    // A narrow pane takes the chips onto a second line rather than squeezing one (1271 V2).
    let layouts: Element<'a, Message> = Row::with_children(layouts)
        .spacing(spacing::SM)
        .wrap()
        .vertical_spacing(spacing::SM)
        .into();
    let content: Element<'a, Message> = match diff_body(view) {
        DiffBody::NoSelection => Text::new("Select a file", TypeRole::Body, r).muted().into(),
        DiffBody::Loading => StageProgress::new("Loading diff…", r).into(),
        DiffBody::Failed(message) => Text::new(message, TypeRole::Body, r).tint(r.error).into(),
        DiffBody::NoLongerChanged => Text::new(NO_LONGER_CHANGED, TypeRole::Body, r)
            .muted()
            .into(),
        DiffBody::Diff(loaded) => {
            let spans = match scheme {
                ColorScheme::Light => &loaded.spans.light,
                ColorScheme::Dark => &loaded.spans.dark,
            };
            let heights: BTreeMap<(Side, u32), f32> = view
                .slot_heights
                .iter()
                .map(|(anchor, height)| (*anchor, *height as f32))
                .collect();
            let mut diff = DiffView::new(&loaded.diff, layout, r)
                .scroll_id(DIFF_SCROLL_ID.clone())
                .spans(spans)
                .offset(view.diff_offset)
                .viewport(view.diff_viewport)
                .on_scroll(|offset, viewport| {
                    Message::Changes(Msg::DiffScrolled { offset, viewport })
                })
                .on_show_large(Message::Changes(Msg::ShowLarge))
                .slot_heights(&heights)
                .on_slot_measured(|side, line, height| {
                    Message::Changes(Msg::SlotMeasured {
                        side,
                        line,
                        height: height.ceil() as u32,
                    })
                });
            diff = diff
                .shaped(&view.shaped)
                .on_gap(|key| Message::Changes(Msg::GapToggled(key)))
                .references(|text| changes::references_in(view, text))
                .on_reference(|path, line| Message::Changes(Msg::ReferenceActivated(path, line)));
            if changes::pick_allowed(view) {
                diff = diff.on_gutter(|old, new, extend| {
                    Message::Changes(Msg::GutterPressed { old, new, extend })
                });
            }
            if let Some(pick) = view.pick {
                diff = diff.pick(pick.side, pick.range());
            }
            // C3: each comment under the row of its last line; an edit's composer takes its card's
            // place.
            let comments = file_comments(&state.changes);
            let sending = review_of(&state.changes, view).is_some_and(|review| review.sending);
            for (&(side, line), placed) in &comments.placed {
                for comment in placed {
                    diff = diff.slot(
                        side,
                        line,
                        comment_or_editor(view, comment, sending, composer, r),
                    );
                }
            }
            // C2: the composer, or the way to open it, under the last picked row. C4: a new
            // comment's composer whose lines a refresh removed waits above the diff for a pick.
            let mut unplaced = None;
            match (view.pick, view.composer.as_ref()) {
                (Some(pick), None) => {
                    diff = diff.slot(
                        pick.side,
                        pick.head,
                        row![
                            Space::new().width(Length::Fill),
                            Button::text("Add comment", r)
                                .on_press(Message::Changes(Msg::AddComment)),
                        ]
                        .padding(spacing::XS),
                    );
                }
                (_, Some(open)) => {
                    if let ComposerTarget::New(_) = open.target {
                        match composer_placed(view) {
                            Some(pick) => {
                                diff = diff.slot(
                                    pick.side,
                                    pick.head,
                                    composer_box(&open.text, composer, true, "Comment", r),
                                );
                            }
                            None => {
                                unplaced = Some(
                                    column![
                                        Text::new(UNPLACED, TypeRole::Caption, r).muted(),
                                        composer_box(&open.text, composer, false, "Comment", r),
                                    ]
                                    .spacing(spacing::XS),
                                );
                            }
                        }
                    }
                }
                (None, None) => {}
            }
            let mut pane = column![layouts].spacing(spacing::MD).height(Length::Fill);
            // FR-009: the head version's state, with the diff usable either way.
            if let Some(note) = changes::head_note(view) {
                let text = Text::new(note.clone(), TypeRole::Caption, r);
                pane = pane.push(if note == changes::READING_FILE {
                    text.muted()
                } else {
                    text.tint(r.error)
                });
            }
            if let Some(notice) = view.notice {
                pane = pane.push(Text::new(notice, TypeRole::Caption, r).tint(r.error));
            }
            if let Some(unplaced) = unplaced {
                pane = pane.push(unplaced);
            }
            if !comments.not_in_diff.is_empty() {
                let mut group = column![Text::new(NOT_IN_DIFF, TypeRole::Label, r).muted()]
                    .spacing(spacing::SM);
                for comment in &comments.not_in_diff {
                    group = group.push(
                        column![
                            Text::new(range_label(comment), TypeRole::Caption, r).muted(),
                            comment_or_editor(view, comment, sending, composer, r),
                        ]
                        .spacing(spacing::XS),
                    );
                }
                pane = pane.push(group);
            }
            return pane.push(diff).into();
        }
    };
    let message = container(column![content, Space::new().height(Length::Fill)])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill);
    column![layouts, message]
        .spacing(spacing::MD)
        .height(Length::Fill)
        .into()
}

/// A file row's pending-comment count (L2).
pub fn pending_label(pending: usize) -> String {
    match pending {
        1 => "1 comment".into(),
        n => format!("{n} comments"),
    }
}

/// Which lines a comment is on, for the "Not in the current diff" group.
pub fn range_label(comment: &ReviewComment) -> String {
    let side = match comment.side {
        Side::New => "",
        Side::Old => "old ",
    };
    let (start, end) = (comment.range.start(), comment.range.end());
    if start == end {
        format!("On {side}line {start}")
    } else {
        format!("On {side}lines {start}\u{2013}{end}")
    }
}

/// A comment's card, or the composer editing it (C2, C3): its status and reply (946 FR-007), and
/// Edit and Delete only on a pending comment no send has taken, Mark applied and Answer… on a
/// Sent one (946 FR-006), with the answer's composer under the card while it is open.
fn comment_or_editor<'a>(
    view: &'a OpenView,
    comment: &'a ReviewComment,
    sending: bool,
    composer: Option<&'a text_editor::Content>,
    r: Roles,
) -> Element<'a, Message> {
    // A composer the drawer opened is drawn in its row while that row is shown (feature 1270, D6).
    let on_card = view.composer.as_ref().filter(|_| !composer_in_drawer(view));
    if let Some(open) = on_card {
        if open.target == ComposerTarget::Edit(comment.id) {
            return composer_box(&open.text, composer, true, "Comment", r);
        }
    }
    let model = card_model(comment, sending);
    let state = match model.status {
        Status::Draft if model.in_send => CardState::InSend,
        Status::Draft => CardState::Draft,
        Status::Sent => CardState::Sent,
        Status::Applied => CardState::Applied,
        Status::Answered => CardState::Answered,
    };
    let mut card = ReviewCommentCard::new(&comment.text, state, r)
        .outdated(crate::features::changes::is_outdated(view, comment))
        .highlighted(view.drawer.focus == Some(comment.id));
    if let Some((reply, shortened)) = model.reply {
        card = card.reply(reply, shortened);
        if shortened {
            card = card.on_show_reply(Message::Changes(Msg::ShowReply(comment.id)));
        }
    }
    if model.editable {
        card = card
            .on_edit(Message::Changes(Msg::EditComment(comment.id)))
            .on_delete(Message::Changes(Msg::DeleteComment(comment.id)));
    }
    if model.markable {
        card = card
            .on_mark_applied(Message::Changes(Msg::MarkApplied(comment.id)))
            .on_answer(Message::Changes(Msg::AnswerComment(comment.id)));
    }
    let card: Element<'a, Message> = card.into();
    match on_card {
        Some(open) if open.target == ComposerTarget::Answer(comment.id) => column![
            card,
            composer_box(&open.text, composer, true, "Why not, or what you did", r)
        ]
        .into(),
        _ => card,
    }
}

/// The composer (C2): the text area, Cancel and Save. Save (and Ctrl/Cmd+Enter) only with some
/// text, the service refuses an empty comment (W1), and only while `placed` on lines (C4).
fn composer_box<'a>(
    text: &'a str,
    editor: Option<&'a text_editor::Content>,
    placed: bool,
    placeholder: &'a str,
    r: Roles,
) -> Element<'a, Message> {
    let has_text = placed && !text.trim().is_empty();
    let save = Message::Changes(Msg::ComposerSaved);
    let area: Element<'a, Message> = match editor {
        Some(content) => {
            let mut area = TextArea::new(content, r)
                .placeholder(placeholder)
                .on_action(|action| Message::Changes(Msg::ComposerAction(EditorAction(action))));
            if has_text {
                area = area.on_submit(save.clone());
            }
            area.into()
        }
        None => Text::new(text, TypeRole::Body, r).into(),
    };
    let actions = row![
        Space::new().width(Length::Fill),
        Button::text("Cancel", r).on_press(Message::Changes(Msg::ComposerCancelled)),
        Button::filled("Save", r).on_press_maybe(has_text.then_some(save)),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
    column![area, actions]
        .spacing(spacing::SM)
        .padding(spacing::SM)
        .into()
}

/// The **Discard drafts…** confirmation (S3), built as `ui/confirm_delete.rs` builds its dialog;
/// `None` when it is not open. `ui::view` wraps it in the shared `Modal` transition.
pub fn discard_dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a micold_core::env_include::EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let prompt = discard_prompt(&state.changes)?;
    let r = tokens::roles(scheme);
    let fields = crate::ui::material::dialog::fields(column![
        Text::new("Discard draft comments", TypeRole::Headline, r),
        Text::new(prompt, TypeRole::Body, r).muted(),
    ]);
    let actions = crate::ui::material::dialog::actions(row![
        Button::filled("Discard", r).on_press(Message::Changes(Msg::DiscardConfirmed)),
        Button::outlined("Cancel", r).on_press(Message::Changes(Msg::DiscardCancelled)),
    ]);
    Some(
        crate::ui::material::Surface::new(
            crate::ui::material::dialog::body(fields, actions),
            crate::ui::material::SurfaceKind::Dialog,
            r,
        )
        .width(Length::Fixed(460.0))
        .into(),
    )
}

/// The full-reply dialog (946 FR-006), built as [`discard_dialog`] is; `None` when it is not open.
pub fn reply_dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a micold_core::env_include::EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let reply = changes::reply_dialog(&state.changes)?;
    Some(reply_dialog_surface(
        reply,
        Message::Changes(Msg::ReplyClosed),
        tokens::roles(scheme),
    ))
}

/// The full-reply dialog's surface around `reply`; the showcase poses it too.
pub fn reply_dialog_surface<'a, M: Clone + 'a>(
    reply: String,
    close: M,
    r: Roles,
) -> Element<'a, M> {
    let fields = crate::ui::material::dialog::fields(column![
        Text::new("Reply", TypeRole::Headline, r),
        crate::ui::material::Scrollable::new(Text::new(reply, TypeRole::Body, r).muted(), r)
            .height(Length::Fixed(280.0)),
    ]);
    let actions =
        crate::ui::material::dialog::actions(row![Button::filled("Close", r).on_press(close),]);
    crate::ui::material::Surface::new(
        crate::ui::material::dialog::body(fields, actions),
        crate::ui::material::SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(560.0))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SC-006, edge case "Very deep paths": at the default list width the tree indents, and at any
    /// depth the indentation it takes leaves a 24-character name its whole width.
    #[test]
    fn a_24_character_name_keeps_its_room_at_every_depth() {
        assert_eq!(NAME_SAMPLE.chars().count(), 24);
        let name = width_of(NAME_SAMPLE, NAME_ROLE);
        let cap = tree_indent_cap(FILE_LIST_WIDTH);
        assert!(cap >= 2, "the tree indents at the default width: cap {cap}");
        for depth in 0..20 {
            let levels = tree::indent_levels(depth, cap);
            let room = name_room(FILE_LIST_WIDTH) - f32::from(levels) * TREE_INDENT_STEP;
            assert!(
                room >= name,
                "depth {depth}: {room} px of room for a {name} px name"
            );
        }
    }

    fn changed(kind: ChangeKind, content: Content) -> ChangedFile {
        ChangedFile {
            path: RelPath::from_native("src/a.rs").expect("a relative path"),
            kind,
            content,
            added: 4,
            removed: 1,
            large: false,
            origin: micold_core::review::changes::Origin::Uncommitted,
            identity: micold_core::review::changes::ChangeIdentity::Unknown,
        }
    }

    /// FR-005: a kind tag for Added, Deleted, Renamed and Mode, none for Modified; an untracked
    /// file reads as Added, and a rename's old path is left to the hover text.
    #[test]
    fn modified_files_carry_no_kind_tag() {
        let r = micold_core::tokens::LIGHT;
        let label = |kind: ChangeKind| kind_tag(&kind, r).map(|(label, _)| label);
        assert_eq!(label(ChangeKind::Modified), None);
        assert_eq!(label(ChangeKind::Added), Some("Added"));
        assert_eq!(label(ChangeKind::Untracked), Some("Added"));
        assert_eq!(label(ChangeKind::Deleted), Some("Deleted"));
        assert_eq!(label(ChangeKind::ModeOnly), Some("Mode"));
        let from = RelPath::from_native("src/old.rs").expect("a relative path");
        assert_eq!(label(ChangeKind::Renamed { from }), Some("Renamed"));
    }

    /// FR-006, scenarios 2.3 and 2.4: only a text change other than a Mode one has a bar.
    #[test]
    fn binary_not_text_and_mode_rows_have_no_bar() {
        assert!(counted(&changed(ChangeKind::Modified, Content::Text)));
        assert!(!counted(&changed(ChangeKind::Added, Content::Binary)));
        assert!(!counted(&changed(ChangeKind::Modified, Content::NotUtf8)));
        assert!(!counted(&changed(ChangeKind::ModeOnly, Content::Text)));
    }
}
