//! The Changes view (feature 482, contracts/changes-view.md V2, L1–L4): glue only.
//!
//! What the view says — the base line, the empty states, whether the Committed toggle can be used —
//! is decided by the render-free `features::changes`; this module lays it out from library
//! components. It stands in the terminal pane's place while open (V1).

use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length};
use micold_core::review::changes::{ChangeKind, ChangedFile, Content};
use micold_core::session::SessionLocation;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Rgb, Roles};

use crate::app::{Message, State};
use crate::features::changes::{
    base_line, committed_available, list_body, ListBody, Msg, OpenView, DEFAULT_ENTRY_NOTE,
};
use crate::icons::Icon;
use crate::ui::material::{Button, ButtonVariant, Tag, Text, ToggleChip, TypeRole, VirtualRows};

/// One file row's height: fixed, which is what lets `VirtualRows` build only the visible rows.
pub const ROW_HEIGHT: f32 = 36.0;

/// The Changes view of `view`'s entry.
pub fn view<'a>(state: &'a State, view: &'a OpenView, scheme: ColorScheme) -> Element<'a, Message> {
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
    let header = row![
        container(heading).width(Length::Fill),
        Button::text("Close", r)
            .leading(Icon::Close)
            .on_press(Message::Changes(Msg::Closed)),
    ]
    .spacing(spacing::MD)
    .align_y(Alignment::Center);

    let body = row![
        container(list_pane(view, r)).width(Length::FillPortion(2)),
        container(diff_pane(view, r)).width(Length::FillPortion(3)),
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
fn list_pane<'a>(view: &'a OpenView, r: Roles) -> Element<'a, Message> {
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
            VirtualRows::new(
                files.len(),
                ROW_HEIGHT,
                move |index| {
                    let file = &files[index];
                    file_row(file, selected == Some(&file.path), r)
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

/// One file (L2): path, kind tag, content tag when not text, and `+a −r`.
fn file_row(file: &ChangedFile, selected: bool, r: Roles) -> Element<'static, Message> {
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

/// The right pane: the diff arrives with M2; until a file is picked it says what to do.
fn diff_pane<'a>(view: &'a OpenView, r: Roles) -> Element<'a, Message> {
    let content: Element<'a, Message> = match &view.selected {
        None => Text::new("Select a file", TypeRole::Body, r).muted().into(),
        Some(path) => Text::new(path.to_string(), TypeRole::Title, r).into(),
    };
    container(column![content, Space::new().height(Length::Fill)])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .into()
}
