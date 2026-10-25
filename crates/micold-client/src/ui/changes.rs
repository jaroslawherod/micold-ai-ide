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
    committed_available, composer_placed, diff_body, discard_prompt, drawer_counts, drawer_groups,
    drawer_rows, file_comments, file_diff_available, file_reviews, list_body, mark_control,
    pending_counts, picker_matches, pr_header, progress_label, review_bar, review_of,
    reviews_progress, section, selected_file, tree_rows, whole_file, BrowseBody, ComposerPlace,
    ComposerTarget, DiffBody, DrawerRow, ListBody, Load, Msg, OpenView, PrSection, ReviewBarModel,
    Source, Step, TreeKind, WholeFile, DEFAULT_ENTRY_NOTE, WHOLE_FILE_UNAVAILABLE,
};
use crate::icons::{icon_role, Icon, IconSurface};
use crate::ui::cdk::overlay::Surface;
use crate::ui::material::{
    menu_panel_size, overflow_trigger_right, tab_content_colour, BrowseBar, Button, ButtonVariant,
    CardState, DiffView, Ellipsized, Glyph, IconButton, MenuItem, MenuOverlay, ProgressBar,
    PullRequestIndicator, PullRequestRow, ReviewBar, ReviewCommentCard, ReviewPanelRow, Scrollable,
    SideSheet, StageProgress, Tab, TabStrip, Tag, Text, TextArea, ToggleChip, Tooltip, TreeItem,
    TreeView, TypeRole, Typeahead, TypeaheadRow, VirtualRows, REVIEW_BAR_COMMENTS as COMMENTS,
};
use micold_core::review::drawer::{DrawerEvent, DrawerTab, START_HINT};
use micold_core::review::marks::{FileReview, ReviewState};
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
/// The section's wording, decided in core with the source control's (feature 1272, R2).
pub use micold_core::review::source_menu::{
    POSSIBLY_STALE, PRS_EMPTY, PRS_LOADING, PRS_UNAVAILABLE, WORKING_TREE,
};
/// The tallest the list of pull requests grows before it scrolls.
const PRS_MAX_HEIGHT: f32 = 220.0;
pub const UNPLACED: &str = "The lines this comment was on are gone. Pick lines to place it.";

/// One file row's height: fixed, which is what lets `VirtualRows` build only the visible rows.
pub const ROW_HEIGHT: f32 = 36.0;

/// The header's *n of m reviewed* slot at its widest: laid out the same width shown or empty, so
/// the header's other controls never move with it (feature 1271, FR-009). In a narrow header it
/// takes a third of the room the title leaves, and its bar, then its words, give way first.
pub const PROGRESS_SLOT_WIDTH: f32 = 260.0;
/// The progress bar's width in that slot at its widest (research R9).
pub const PROGRESS_BAR_WIDTH: f32 = 120.0;

/// The diff's scroll viewport, by name: a panel jump scrolls it (feature 946, FR-017). A
/// `LazyLock` for the reason [`crate::ui::SIDEBAR_SCROLL_ID`] is one.
pub static DIFF_SCROLL_ID: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("changes-diff"));

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
    let mut heading = column![Text::new(title, TypeRole::Title, r)].spacing(spacing::XS);
    if let Some(line) = base_line(view) {
        heading = heading.push(Text::new(line, TypeRole::Body, r).muted());
    }
    // Feature 1271: the rows' review states, judged once for the list and the header, whose
    // *n of m reviewed* slot sits beside the heading (FR-008, FR-009).
    let reviews = file_reviews(&state.changes, view, connected);
    let header = container(
        row![
            container(heading).width(Length::FillPortion(2)),
            progress_slot(reviews_progress(&reviews, connected), r),
        ]
        .spacing(spacing::MD)
        .align_y(Alignment::Center),
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
    .width(Length::FillPortion(5))
    .height(Length::Fill);
    if view.drawer.state.open {
        diff_area = diff_area.push(comments_drawer(state, view, composer, r));
    }
    // SC-004: the file list keeps its share of the old three columns (2 of 2 + 3 + 2), and the
    // diff area takes the rest — the old diff and review column together.
    let body = row![
        container(list_pane(state, view, reviews, r))
            .width(Length::FillPortion(2))
            .padding(Padding {
                left: spacing::LG,
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
    let items = vec![
        MenuItem::labeled(OVERFLOW_ITEMS[0], Message::Changes(Msg::ClearSentPressed)),
        MenuItem::labeled(
            OVERFLOW_ITEMS[1],
            Message::Changes(Msg::DiscardPendingPressed),
        ),
    ];
    let enabled = [bar.overflow.clear_sent, bar.overflow.discard_drafts];
    let items: Vec<MenuItem<Message>> = items
        .into_iter()
        .zip(enabled)
        .map(|(item, on)| MenuItem {
            message: item.message.filter(|_| on),
            ..item
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
    let Some(open) = view
        .composer
        .as_ref()
        .filter(|open| open.place == ComposerPlace::Drawer)
    else {
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

/// The left pane: the two toggles (L1) over the file list or its empty state (L2, L3).
fn list_pane<'a>(
    state: &'a State,
    view: &'a OpenView,
    reviews: BTreeMap<RelPath, FileReview>,
    r: Roles,
) -> Element<'a, Message> {
    let available = committed_available(view);
    let toggles = row![
        ToggleChip::new("Committed", Message::Changes(Msg::CommittedToggled), r)
            .active(available && view.toggles.committed)
            .disabled(!available),
        ToggleChip::new("Uncommitted", Message::Changes(Msg::UncommittedToggled), r)
            .active(view.toggles.uncommitted),
        ToggleChip::new("Files", Message::Changes(Msg::FilesPaneToggled), r)
            .active(view.files.shown),
    ]
    .spacing(spacing::SM)
    .wrap()
    .vertical_spacing(spacing::SM);
    // The toggles shape the entry's own list, which a shown pull request replaces.
    let mut pane = column![]
        .spacing(spacing::MD)
        .width(Length::Fill)
        .height(Length::Fill);
    if view.source == Source::Changes {
        pane = pane.push(toggles);
    } else {
        // A shown pull request replaces the entry's list, but its files are browsed all the same.
        pane = pane.push(row![ToggleChip::new(
            "Files",
            Message::Changes(Msg::FilesPaneToggled),
            r
        )
        .active(view.files.shown)]);
    }
    if let Some(section) = pull_request_section(view, state.changes.pr_enabled, r) {
        pane = pane.push(section);
    }
    if !available && view.source == Source::Changes {
        pane = pane.push(Text::new(DEFAULT_ENTRY_NOTE, TypeRole::Caption, r).muted());
    }

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
        ListBody::Files(files) => {
            let selected = selected_file(view);
            let counts = pending_counts(&state.changes);
            VirtualRows::new(
                files.len(),
                ROW_HEIGHT,
                move |index| {
                    let file = &files[index];
                    let pending = counts.get(&file.path).copied().unwrap_or(0);
                    let review = reviews.get(&file.path).map(|review| review.state);
                    file_row(file, selected == Some(&file.path), pending, review, r)
                },
                r,
            )
            .offset(view.list_offset)
            .viewport(view.list_viewport)
            .on_scroll(|offset, viewport| Message::Changes(Msg::ListScrolled { offset, viewport }))
            .into()
        }
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

/// The pull requests of the worktree's branches, above the file list (features 775, 1003): the
/// loading, unavailable and empty lines, or the list; absent per `section(..)`.
fn pull_request_section<'a>(
    view: &'a OpenView,
    enabled: bool,
    r: Roles,
) -> Option<Element<'a, Message>> {
    let line = |text: &'static str, muted: bool| {
        let text = Text::new(text, TypeRole::Body, r);
        let text = if muted { text.muted() } else { text };
        column![Text::new("Pull requests", TypeRole::Section, r), text].spacing(spacing::XS)
    };
    let (prs, stale) = match section(view, enabled, super::sidebar::unix_now()) {
        PrSection::Absent => return None,
        PrSection::Loading => return Some(line(PRS_LOADING, true).into()),
        PrSection::Unavailable => return Some(line(PRS_UNAVAILABLE, true).into()),
        PrSection::Empty => return Some(line(PRS_EMPTY, false).into()),
        PrSection::List { prs, stale } => (prs, stale),
    };
    let shown = match &view.source {
        Source::Changes => None,
        Source::PullRequest(pr) => Some(pr.number),
    };
    let choice = |content: Element<'a, Message>, chosen: bool, msg: Msg| {
        Button::with_content(
            content,
            if chosen {
                ButtonVariant::Outlined
            } else {
                ButtonVariant::Text
            },
            r,
        )
        .width(Length::Fill)
        .on_press(Message::Changes(msg))
    };
    let mut section = column![
        Text::new("Pull requests", TypeRole::Section, r),
        choice(
            Text::new(WORKING_TREE, TypeRole::Body, r).into(),
            shown.is_none(),
            Msg::SourceChosen(None),
        ),
    ]
    .spacing(spacing::XS);
    if stale {
        section = section.push(Text::new(POSSIBLY_STALE, TypeRole::Caption, r).muted());
    }
    let mut rows = column![].spacing(spacing::XS);
    for pr in prs {
        let mut row = PullRequestRow::<Message>::new(
            pr.number,
            pr.title.as_str(),
            crate::features::changes::pr_mark(pr.state),
            &r,
        )
        .selected(false);
        if let Some(mark) = crate::features::changes::check_mark(pr.checks) {
            row = row.checks(mark);
        }
        rows = rows.push(choice(
            Element::from(row),
            shown == Some(pr.number),
            Msg::SourceChosen(Some(pr.number)),
        ));
    }
    section = section.push(
        container(crate::ui::material::Scrollable::new(rows, r).height(Length::Shrink))
            .max_height(PRS_MAX_HEIGHT),
    );
    Some(section.into())
}

/// One file (L2): its review state (feature 1271, FR-001), path, kind tag, content tag when not
/// text, `+a −r`, and its pending comments.
fn file_row(
    file: &ChangedFile,
    selected: bool,
    pending: usize,
    review: Option<ReviewState>,
    r: Roles,
) -> Element<'static, Message> {
    let (kind, accent) = kind_tag(&file.kind, r);
    let variant = if selected {
        ButtonVariant::Outlined
    } else {
        ButtonVariant::Text
    };
    let mut line = row![].spacing(spacing::SM).align_y(Alignment::Center);
    if let Some(state) = review {
        line = line.push(review_glyph(state, r));
    }
    let mut line = line
        // One line, cut from the directory side where the row's tags leave too little room, so
        // the file name stays while it fits (1271 V1).
        .push(
            Ellipsized::<Message>::at_role_spaced(
                file.path.to_string(),
                TypeRole::Body,
                variant.content(r, None),
            )
            .path(),
        )
        .push(Tag::new(kind, accent));
    match file.content {
        Content::Text => {}
        Content::Binary => line = line.push(Tag::new("Binary", r.secondary)),
        Content::NotUtf8 => line = line.push(Tag::new("Not text", r.secondary)),
    }
    line = line.push(
        Text::new(
            format!("+{} \u{2212}{}", file.added, file.removed),
            TypeRole::Caption,
            r,
        )
        .muted(),
    );
    if pending > 0 {
        line = line.push(Tag::new(pending_label(pending), r.tertiary));
    }
    container(
        // The row's inset is the list's, not the variant's: selecting a row outlines it without
        // taking 24 px from its path (1271 visual re-pass V1).
        Button::with_content(line, variant, r)
            .padding([0.0, tokens::anatomy::button::PADDING_TEXT])
            .width(Length::Fill)
            .on_press(Message::Changes(Msg::FileSelected(file.path.clone()))),
    )
    .height(Length::Fixed(ROW_HEIGHT))
    .width(Length::Fill)
    .into()
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

/// The kind's label and accent (L2). An untracked file is new to the reviewer: Added.
fn kind_tag(kind: &ChangeKind, r: Roles) -> (String, Rgb) {
    match kind {
        ChangeKind::Added | ChangeKind::Untracked => ("Added".into(), r.primary),
        ChangeKind::Modified => ("Modified".into(), r.secondary),
        ChangeKind::Deleted => ("Deleted".into(), r.error),
        ChangeKind::Renamed { from } => (format!("Renamed from {from}"), r.tertiary),
        ChangeKind::ModeOnly => ("Mode".into(), r.secondary),
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
    // A composer the drawer opened is drawn in its row instead (feature 1270, D6).
    let on_card = view
        .composer
        .as_ref()
        .filter(|open| open.place == ComposerPlace::Card);
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
