//! The Changes view (feature 482, contracts/changes-view.md V2, L1–L4): glue only.
//!
//! What the view says — the base line, the empty states, whether the Committed toggle can be used —
//! is decided by the render-free `features::changes`; this module lays it out from library
//! components. It stands in the terminal pane's place while open (V1).

use std::collections::BTreeMap;

use iced::widget::{column, container, row, text_editor, Space};
use iced::{Alignment, Element, Length};
use micold_core::review::changes::{ChangeKind, ChangedFile, Content};
use micold_core::review::comment::ReviewComment;
use micold_core::review::Side;
use micold_core::session::SessionLocation;
use micold_core::settings::DiffLayout;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Rgb, Roles};

use crate::app::{EditorAction, Message, State};
use crate::features::changes::{
    base_line, can_pick, committed_available, composer_placed, diff_body, file_comments,
    is_pending, list_body, pending_counts, review_of, send_action, ComposerTarget, DiffBody,
    ListBody, Msg, OpenView, DEFAULT_ENTRY_NOTE,
};
use crate::icons::Icon;
use crate::ui::material::{
    Button, ButtonVariant, CardState, DiffView, ReviewCommentCard, StageProgress, Tag, Text,
    TextArea, ToggleChip, TypeRole, VirtualRows,
};

/// The heading of the comments whose lines the diff on screen does not show (C3).
pub const NOT_IN_DIFF: &str = "Not in the current diff";
/// Above a new comment whose lines a refresh removed (C4).
pub const UNPLACED: &str = "The lines this comment was on are gone. Pick lines to place it.";

/// One file row's height: fixed, which is what lets `VirtualRows` build only the visible rows.
pub const ROW_HEIGHT: f32 = 36.0;

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
    // S1: Send to session (n), "Sending…" while a send of this entry is open in any window.
    let send = send_action(&state.changes).map(|action| {
        Button::filled(action.label, r)
            .on_press_maybe(action.enabled.then_some(Message::Changes(Msg::SendPressed)))
    });
    let mut header = row![container(heading).width(Length::Fill)];
    if let Some(send) = send {
        header = header.push(send);
    }
    let header = header
        .push(
            Button::text("Close", r)
                .leading(Icon::Close)
                .on_press(Message::Changes(Msg::Closed)),
        )
        .spacing(spacing::MD)
        .align_y(Alignment::Center);

    let body = row![
        container(list_pane(state, view, r)).width(Length::FillPortion(2)),
        container(diff_pane(state, view, scheme, composer)).width(Length::FillPortion(3)),
    ]
    .spacing(spacing::LG)
    .height(Length::Fill);

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
    let mut pane = column![toggles].spacing(spacing::MD).height(Length::Fill);
    if !available {
        pane = pane.push(Text::new(DEFAULT_ENTRY_NOTE, TypeRole::Caption, r).muted());
    }

    let list: Element<'a, Message> = match list_body(view) {
        ListBody::Loading => Text::new("Reading changes…", TypeRole::Body, r)
            .muted()
            .into(),
        ListBody::Failed(message) => Text::new(message, TypeRole::Body, r).tint(r.error).into(),
        ListBody::Empty(sentence) => Text::new(sentence, TypeRole::Body, r).muted().into(),
        ListBody::Files(files) => {
            let selected = view.selected.as_ref();
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

/// The right pane (D1–D4): what to do before a file is picked, the progress line while its first
/// read runs, git's message when it fails, else the diff — whose D3 messages and D4 large gate
/// `DiffView` draws.
fn diff_pane<'a>(
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
                                    composer_box(&open.text, composer, true, r),
                                );
                            }
                            None => {
                                unplaced = Some(
                                    column![
                                        Text::new(UNPLACED, TypeRole::Caption, r).muted(),
                                        composer_box(&open.text, composer, false, r),
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

/// A comment's card, or the composer editing it (C2, C3). Edit and Delete only on a pending
/// comment no send has taken.
fn comment_or_editor<'a>(
    view: &'a OpenView,
    comment: &'a ReviewComment,
    sending: bool,
    composer: Option<&'a text_editor::Content>,
    r: Roles,
) -> Element<'a, Message> {
    if let Some(open) = view.composer.as_ref() {
        if open.target == ComposerTarget::Edit(comment.id) {
            return composer_box(&open.text, composer, true, r);
        }
    }
    let state = match (is_pending(comment), sending) {
        (false, _) => CardState::Sent,
        (true, true) => CardState::InSend,
        (true, false) => CardState::Pending,
    };
    let mut card = ReviewCommentCard::new(&comment.text, state, r)
        .outdated(view.outdated.contains(&comment.id));
    if state == CardState::Pending {
        card = card
            .on_edit(Message::Changes(Msg::EditComment(comment.id)))
            .on_delete(Message::Changes(Msg::DeleteComment(comment.id)));
    }
    card.into()
}

/// The composer (C2): the text area, Cancel and Save. Save (and Ctrl/Cmd+Enter) only with some
/// text, the service refuses an empty comment (W1), and only while `placed` on lines (C4).
fn composer_box<'a>(
    text: &'a str,
    editor: Option<&'a text_editor::Content>,
    placed: bool,
    r: Roles,
) -> Element<'a, Message> {
    let has_text = placed && !text.trim().is_empty();
    let save = Message::Changes(Msg::ComposerSaved);
    let area: Element<'a, Message> = match editor {
        Some(content) => {
            let mut area = TextArea::new(content, r)
                .placeholder("Comment")
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
