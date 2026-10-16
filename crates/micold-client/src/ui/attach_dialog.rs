//! The attach-existing dialog (feature 582, reduced by 552): the project's stored provider sessions
//! the catalog lacks, each with a Resume button, and "Resume all". Resuming touches nothing on
//! disk but the session's own entry; worktrees are never attached (FR-009).

use crate::app::{Message, State};
use crate::features::attach::{can_resume, status_text, Confirm, Dialog, Listing, Msg};
use crate::features::window::FieldId;
use crate::ui::material::{self, Button, Scrollable, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::attach::ResumableSession;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The dialog body; `ui::view` wraps it in the shared [`Modal`](crate::ui::material::Modal)
/// transition.
pub fn modal<'a>(
    dialog: &'a Dialog,
    scheme: ColorScheme,
    _focused: Option<FieldId>,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    if let Some(confirm) = &dialog.confirm {
        return confirm_modal(confirm, r);
    }

    let mut fields = material::dialog::fields(column![
        Text::new("Resume stored sessions", TypeRole::Headline, r),
        Text::new(
            "Shows provider sessions stored on this computer that this app has no record of. \
             Resuming one adds it to the sidebar; nothing else is changed.",
            TypeRole::Body,
            r
        )
        .muted(),
    ]);

    let offered = dialog.resumable().len();
    match &dialog.listing {
        Listing::Loading => {
            fields =
                fields.push(Text::new("Looking for stored sessions…", TypeRole::Body, r).muted());
        }
        Listing::Failed(reason) => {
            fields = fields.push(
                Text::new(
                    format!("Could not read stored sessions: {reason}"),
                    TypeRole::Caption,
                    r,
                )
                .tint(r.error),
            );
        }
        Listing::Listed if dialog.sessions.is_empty() => {
            fields = fields.push(Text::new("No stored sessions found.", TypeRole::Body, r).muted());
        }
        Listing::Listed => {}
    }

    if !dialog.sessions.is_empty() {
        let mut list = column![].spacing(spacing::XS);
        for session in &dialog.sessions {
            list = list.push(session_row(dialog, session, r));
        }
        fields = fields.push(Scrollable::new(list, r).height(Length::Fixed(240.0)));
    }
    for note in dialog.footer_notes() {
        fields = fields.push(Text::new(note, TypeRole::Caption, r).muted());
    }

    if let Some(reason) = &dialog.error {
        fields = fields.push(
            Text::new(format!("Could not resume: {reason}"), TypeRole::Caption, r).tint(r.error),
        );
    }

    let busy = dialog.applying();
    let all = Button::filled("Resume all", r)
        .on_press_maybe((!busy && offered > 0).then_some(Message::Attach(Msg::ResumeAll)));
    let cancel = Button::text("Cancel", r).on_press(Message::Attach(Msg::Cancelled));
    let actions = material::dialog::actions(row![all, cancel]);

    material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(520.0))
    .into()
}

/// The question a refused resume leads to: replace the worktree's session, keep both, or cancel
/// (feature 776, FR-008b, FR-008c).
fn confirm_modal<'a>(confirm: &'a Confirm, r: tokens::Roles) -> Element<'a, Message> {
    let fields = material::dialog::fields(column![
        Text::new("Replace the session?", TypeRole::Headline, r),
        Text::new(confirm.text(), TypeRole::Body, r).muted(),
    ]);
    let actions = material::dialog::actions(row![
        Button::filled("Replace", r).on_press(Message::Attach(Msg::ConfirmReplace)),
        Button::outlined("Keep both", r).on_press(Message::Attach(Msg::ConfirmKeepBoth)),
        Button::text("Cancel", r).on_press(Message::Attach(Msg::Cancelled)),
    ]);
    material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(520.0))
    .into()
}

/// One stored session: its title, provider and what resuming it does, with a Resume button.
fn session_row<'a>(
    dialog: &'a Dialog,
    session: &'a ResumableSession,
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
