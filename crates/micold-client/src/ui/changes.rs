//! The Changes view (feature 482, contracts/changes-view.md V2, L1–L4): glue only.
//!
//! What the view says — the base line, the empty states, whether the Committed toggle can be used —
//! is decided by the render-free `features::changes`; this module lays it out from library
//! components. It stands in the terminal pane's place while open (V1).

use std::collections::BTreeMap;

use iced::widget::{column, container, row, text_editor, Space};
use iced::{Alignment, Element, Length};
use micold_core::review::changes::{ChangeKind, ChangedFile, Content};
use micold_core::review::comment::{ReviewComment, Status};
use micold_core::review::Side;
use micold_core::session::SessionLocation;
use micold_core::settings::DiffLayout;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Rgb, Roles};

use crate::app::{EditorAction, Message, State};
use crate::features::changes::{
    self as changes, adjacent, base_line, can_pick, card_model, committed_available,
    composer_placed, diff_body, discard_prompt, file_comments, list_body, panel_rows,
    pending_counts, pr_header, review_of, review_toolbar_label, section, selected_file,
    send_action, status_counts, tidy_actions, ComposerTarget, DiffBody, ListBody, Msg, OpenView,
    PrSection, Source, Step, DEFAULT_ENTRY_NOTE, PANEL_EMPTY_TEXT,
};
use crate::icons::Icon;
use crate::ui::material::{
    status_label, Button, ButtonVariant, CardState, DiffView, IconButton, PullRequestIndicator,
    PullRequestRow, ReviewCommentCard, ReviewPanelRow, StageProgress, Tag, Text, TextArea,
    ToggleChip, Tooltip, TypeRole, VirtualRows,
};

/// The heading of the comments whose lines the diff on screen does not show (C3).
pub const NOT_IN_DIFF: &str = "Not in the current diff";
/// Above a new comment whose lines a refresh removed (C4).
/// A pull request GitHub will not diff (above 300 files or 20 000 changed lines): not an error, so
/// not a notice (FR-008).
pub const PR_TOO_LARGE: &str = "This pull request is too large to show here";
/// A pull request whose diff could not be read; pressing its row again retries.
pub const PR_UNREAD: &str = "The pull request's diff could not be read; select it again to retry";
/// The row that returns the view to the entry's own changes (FR-005).
pub const WORKING_TREE: &str = "Working tree";
/// The caption under the pull request section's title once a failed refresh left a list older than
/// the PR status stale threshold (feature 775, FR-012).
pub const POSSIBLY_STALE: &str = "Possibly stale";
/// The section's line while the first read is pending (feature 1003, FR-010).
pub const PRS_LOADING: &str = "Reading pull requests…";
/// The section's muted line when no list can be read (FR-011).
pub const PRS_UNAVAILABLE: &str = "Pull requests can't be shown for this worktree";
/// The section's line for a worktree with no pull request (FR-010).
pub const PRS_EMPTY: &str = "No pull requests from this worktree yet";
/// The tallest the list of pull requests grows before it scrolls.
const PRS_MAX_HEIGHT: f32 = 220.0;
pub const UNPLACED: &str = "The lines this comment was on are gone. Pick lines to place it.";

/// One file row's height: fixed, which is what lets `VirtualRows` build only the visible rows.
pub const ROW_HEIGHT: f32 = 36.0;

/// One review panel row's height.
const PANEL_ROW_HEIGHT: f32 = crate::ui::material::PANEL_ROW_HEIGHT;

/// The diff's scroll viewport, by name: a panel jump scrolls it (feature 946, FR-017). A
/// `LazyLock` for the reason [`crate::ui::SIDEBAR_SCROLL_ID`] is one.
pub static DIFF_SCROLL_ID: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("changes-diff"));

/// The Changes view of `view`'s entry. `composer` is the review composer's editor, which only the
/// binary holds; without it an open composer shows its text read-only.
pub fn view<'a>(
    state: &'a State,
    view: &'a OpenView,
    scheme: ColorScheme,
    composer: Option<&'a text_editor::Content>,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let title = match &view.entry {
        SessionLocation::Default => crate::features::sidebar::DEFAULT_LOCATION_LABEL.to_string(),
        SessionLocation::Worktree(dir) => state.worktree_display_name(dir),
    };

    // V2: the entry, the base (or why there is none), and the way out.
    let mut heading = column![Text::new(title, TypeRole::Title, r)].spacing(spacing::XS);
    if let Some(line) = base_line(view) {
        heading = heading.push(Text::new(line, TypeRole::Body, r).muted());
    }
    // S1: Send review (n), "Sending…" while a send of this entry is open in any window.
    let mut header = row![container(heading).width(Length::Fill)];
    // FR-016: the toolbar toggle that closes and reopens the review panel.
    header = header.push(
        ToggleChip::new(
            review_toolbar_label(&state.changes),
            Message::Changes(Msg::PanelToggled),
            r,
        )
        .active(view.panel.open),
    );
    if !view.panel.open {
        header = header.push(review_buttons(state, r));
    }
    let header = header
        .push(
            Button::text("Close", r)
                .leading(Icon::Close)
                .on_press(Message::Changes(Msg::Closed)),
        )
        .spacing(spacing::MD)
        .align_y(Alignment::Center);

    let mut body = row![
        container(list_pane(state, view, r)).width(Length::FillPortion(2)),
        container(diff_pane(state, view, scheme, composer)).width(Length::FillPortion(3)),
    ]
    .spacing(spacing::LG)
    .height(Length::Fill);
    if view.panel.open {
        body = body.push(container(review_panel(state, view, r)).width(Length::FillPortion(2)));
    }

    container(
        column![header, body]
            .spacing(spacing::LG)
            .height(Length::Fill),
    )
    .padding(spacing::LG)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// **Clear sent**, **Discard drafts…** and **Send review (n)**, each with the availability 482
/// gave it (FR-019). In the panel's header while it is open, in the view's header otherwise.
fn review_buttons<'a>(state: &'a State, r: Roles) -> iced::widget::Row<'a, Message> {
    let mut buttons = row![].spacing(spacing::SM).align_y(Alignment::Center);
    if let Some(tidy) = tidy_actions(&state.changes) {
        buttons = buttons
            .push(
                Button::text("Clear sent", r).on_press_maybe(
                    tidy.clear_sent
                        .then_some(Message::Changes(Msg::ClearSentPressed)),
                ),
            )
            .push(
                Button::text("Discard drafts…", r).on_press_maybe(
                    tidy.discard_pending
                        .then_some(Message::Changes(Msg::DiscardPendingPressed)),
                ),
            );
    }
    if let Some(action) = send_action(&state.changes) {
        buttons = buttons.push(
            Button::filled(action.label, r)
                .on_press_maybe(action.enabled.then_some(Message::Changes(Msg::SendPressed))),
        );
    }
    buttons
}

/// The review panel (feature 946, FR-016 to FR-019): its buttons, the counts per status, and a row
/// per comment. `VirtualRows` has one row height, so the file path is in each row's label rather
/// than in a group header.
fn review_panel<'a>(state: &'a State, view: &'a OpenView, r: Roles) -> Element<'a, Message> {
    let buttons = review_buttons(state, r);
    let counts = status_counts(&state.changes)
        .iter()
        .filter(|(_, n)| *n > 0)
        .map(|(status, n)| format!("{n} {}", status_label(*status).to_lowercase()))
        .collect::<Vec<_>>()
        .join(" · ");
    let mut pane = column![
        Text::new("Review", TypeRole::Title, r),
        buttons.wrap(),
        Text::new(counts, TypeRole::Body, r).muted(),
    ]
    .spacing(spacing::SM)
    .height(Length::Fill);
    if let Some(why) = view.panel.hidden {
        pane = pane.push(Text::new(why.message(), TypeRole::Body, r));
    }
    let rows = panel_rows(&state.changes);
    if rows.is_empty() {
        return pane
            .push(Text::new(PANEL_EMPTY_TEXT, TypeRole::Body, r).muted())
            .into();
    }
    let list = VirtualRows::new(
        rows.len(),
        PANEL_ROW_HEIGHT,
        move |index| {
            let Some(item) = rows.get(index) else {
                return Space::new().into();
            };
            let mut card = ReviewPanelRow::new(
                format!("{} · {}", item.path.as_str(), item.lines),
                item.text.clone(),
                item.status,
                r,
            )
            .selected(item.selected)
            .not_in_diff(item.not_in_diff)
            .on_press(Message::Changes(Msg::PanelRowSelected(item.id)));
            if let Some(reply) = &item.reply {
                card = card.reply(reply.clone());
            }
            if item.editable {
                card = card
                    .on_edit(Message::Changes(Msg::EditComment(item.id)))
                    .on_delete(Message::Changes(Msg::DeleteComment(item.id)));
            }
            card.into()
        },
        r,
    )
    .offset(view.panel.offset)
    .viewport(view.panel.viewport)
    .on_scroll(|offset, viewport| Message::Changes(Msg::PanelScrolled { offset, viewport }));
    pane.push(list).into()
}

/// The left pane: the two toggles (L1) over the file list or its empty state (L2, L3).
fn list_pane<'a>(state: &'a State, view: &'a OpenView, r: Roles) -> Element<'a, Message> {
    let available = committed_available(view);
    let toggles = row![
        ToggleChip::new("Committed", Message::Changes(Msg::CommittedToggled), r)
            .active(available && view.toggles.committed)
            .disabled(!available),
        ToggleChip::new("Uncommitted", Message::Changes(Msg::UncommittedToggled), r)
            .active(view.toggles.uncommitted),
    ]
    .spacing(spacing::SM);
    // The toggles shape the entry's own list, which a shown pull request replaces.
    let mut pane = column![].spacing(spacing::MD).height(Length::Fill);
    if view.source == Source::Changes {
        pane = pane.push(toggles);
    }
    if let Some(section) = pull_request_section(view, state.changes.pr_enabled, r) {
        pane = pane.push(section);
    }
    if !available && view.source == Source::Changes {
        pane = pane.push(Text::new(DEFAULT_ENTRY_NOTE, TypeRole::Caption, r).muted());
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
                    file_row(file, selected == Some(&file.path), pending, r)
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

/// One file (L2): path, kind tag, content tag when not text, `+a −r`, and its pending comments.
fn file_row(
    file: &ChangedFile,
    selected: bool,
    pending: usize,
    r: Roles,
) -> Element<'static, Message> {
    let (kind, accent) = kind_tag(&file.kind, r);
    let mut line = row![
        container(Text::new(file.path.to_string(), TypeRole::Body, r)).width(Length::Fill),
        Tag::new(kind, accent),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
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
    let variant = if selected {
        ButtonVariant::Outlined
    } else {
        ButtonVariant::Text
    };
    container(
        Button::with_content(line, variant, r)
            .width(Length::Fill)
            .on_press(Message::Changes(Msg::FileSelected(file.path.clone()))),
    )
    .height(Length::Fixed(ROW_HEIGHT))
    .width(Length::Fill)
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
) -> Element<'a, Message> {
    let body = diff_pane_body(state, view, scheme, composer);
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
) -> Element<'a, Message> {
    let layout = state.changes.layout;
    let r = tokens::roles(scheme);
    // D1, FR-006: the layout is a setting, so its choice shows whatever the pane holds.
    let layouts = row![
        ToggleChip::new(
            "Unified",
            Message::Changes(Msg::LayoutChosen(DiffLayout::Unified)),
            r
        )
        .active(layout == DiffLayout::Unified),
        ToggleChip::new(
            "Side by side",
            Message::Changes(Msg::LayoutChosen(DiffLayout::SideBySide)),
            r
        )
        .active(layout == DiffLayout::SideBySide),
    ]
    .spacing(spacing::SM);
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
            if can_pick(view) {
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
    if let Some(open) = view.composer.as_ref() {
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
        .highlighted(view.panel.focus == Some(comment.id));
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
    match view.composer.as_ref() {
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
