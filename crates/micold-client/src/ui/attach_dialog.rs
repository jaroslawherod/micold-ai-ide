//! The attach-existing-worktrees dialog (feature 582, FR-001, FR-002, FR-004): the project's
//! unattached provider worktrees as a checkbox list, with "Attach selected" and "Attach all".
//! Attaching records the worktrees in the app and touches nothing on disk; the dialog says so.

use crate::app::{Message, State};
use crate::features::attach::{can_resume, status_text, Dialog, Listing, Msg};
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{self, Button, Checkbox, Scrollable, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::attach::{AttachableWorktree, Availability, ResumableSession, Unavailable};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The dialog body; `ui::view` wraps it in the shared [`Modal`](crate::ui::material::Modal)
/// transition.
pub fn modal<'a>(
    dialog: &'a Dialog,
    scheme: ColorScheme,
    focused: Option<FieldId>,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let mut fields = material::dialog::fields(column![
        Text::new("Attach existing worktrees", TypeRole::Headline, r),
        Text::new(
            "Shows provider worktrees this app has no record of. Attaching only adds them to the \
             sidebar — their branch, files and uncommitted changes are left untouched.",
            TypeRole::Body,
            r
        )
        .muted(),
    ]);

    let offered = dialog.attachable().len();
    match &dialog.listing {
        Listing::Loading => {
            fields = fields.push(Text::new("Looking for worktrees…", TypeRole::Body, r).muted());
        }
        Listing::Failed(reason) => {
            fields = fields.push(
                Text::new(
                    format!("Could not read worktrees: {reason}"),
                    TypeRole::Caption,
                    r,
                )
                .tint(r.error),
            );
        }
        Listing::Listed(rows) if rows.is_empty() => {
            fields =
                fields.push(Text::new("No unattached worktrees found.", TypeRole::Body, r).muted());
        }
        Listing::Listed(rows) => {
            let mut list = column![].spacing(spacing::XS);
            for (index, w) in rows.iter().enumerate() {
                list = list.push(row_view(dialog, w, index, focused, r));
            }
            fields = fields.push(Scrollable::new(list, r).height(Length::Fixed(240.0)));
        }
    }

    if !dialog.sessions.is_empty() {
        let mut list = column![].spacing(spacing::XS);
        for (index, session) in dialog.sessions.iter().enumerate() {
            list = list.push(session_row(dialog, session, index, r));
        }
        fields = fields
            .push(Text::new("Stored sessions", TypeRole::Title, r))
            .push(Scrollable::new(list, r).height(Length::Fixed(180.0)));
    }
    for note in dialog.footer_notes() {
        fields = fields.push(Text::new(note, TypeRole::Caption, r).muted());
    }

    if let Some(reason) = &dialog.error {
        fields = fields.push(
            Text::new(format!("Could not attach: {reason}"), TypeRole::Caption, r).tint(r.error),
        );
    }

    let busy = dialog.applying();
    let select = Button::filled("Attach selected", r).on_press_maybe(
        (!busy && !dialog.selected.is_empty()).then_some(Message::Attach(Msg::AttachSelected)),
    );
    let all = Button::outlined("Attach all", r)
        .on_press_maybe((!busy && offered > 0).then_some(Message::Attach(Msg::AttachAll)));
    let cancel = Button::text("Cancel", r).on_press(Message::Attach(Msg::Cancelled));
    let actions = material::dialog::actions(row![select, all, cancel]);

    material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(520.0))
    .into()
}

fn row_view<'a>(
    dialog: &'a Dialog,
    w: &'a AttachableWorktree,
    index: usize,
    focused: Option<FieldId>,
    r: tokens::Roles,
) -> Element<'a, Message> {
    let branch = w.branch.as_deref().unwrap_or("detached HEAD");
    let note = match w.availability {
        Availability::Attachable => String::new(),
        Availability::Unavailable(Unavailable::Missing) => " — folder is missing".into(),
        Availability::Unavailable(Unavailable::Invalid) => " — not a usable worktree".into(),
    };
    let label = format!("{} ({branch}){note}", w.dir_name);
    let name = w.dir_name.clone();
    let mut checkbox = Checkbox::new(label, dialog.selected.contains(&w.dir_name), r)
        .track_focus(FieldId::AttachWorktreeRow(index), focused);
    if w.availability == Availability::Attachable && !dialog.applying() {
        checkbox = checkbox.on_toggle(move |checked| {
            Message::Attach(Msg::Toggled {
                dir_name: name.clone(),
                checked,
            })
        });
    }
    checkbox.into()
}

/// One stored session: its title, provider and what resuming it does, with a Resume button.
fn session_row<'a>(
    dialog: &'a Dialog,
    session: &'a ResumableSession,
    _index: usize,
    r: tokens::Roles,
) -> Element<'a, Message> {
    let title = session.title.as_deref().unwrap_or("Untitled session");
    let provider = session.provider.provider().display_name();
    let id = session.id;
    let resume = Button::outlined("Resume", r).on_press_maybe(
        (can_resume(session) && !dialog.applying()).then_some(Message::Attach(Msg::Resume { id })),
    );
    row![
        column![
            Text::new(format!("{title} ({provider})"), TypeRole::Body, r),
            Text::new(status_text(session), TypeRole::Caption, r).muted(),
        ]
        .width(Length::Fill),
        resume,
    ]
    .spacing(spacing::SM)
    .into()
}

/// This dialog's body, built from the state that opened it.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state
        .attach
        .dialog
        .as_ref()
        .map(|d| modal(d, scheme, state.window.focused_field))
}
