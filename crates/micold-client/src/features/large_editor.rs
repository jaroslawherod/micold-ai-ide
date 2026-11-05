//! The composer's large editor (feature 1703, US1): a modal text area holding a copy of one
//! session's draft. **Done** writes the text back to that session's draft; **Cancel** drops it.
//! Text only: attachments are never read or written (FR-003).

use crate::app::{Editor, EditorAction, Message, State as App};
use crate::floating_surface;
use crate::keymap::{Key, KeyInput, NamedKey};
use crate::overlay::registry::Registered;
use crate::overlay::SurfaceId;
use micold_core::session::SessionId;

/// The open editor: the session whose composer opened it, and the text it edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LargeEditor {
    pub session: SessionId,
    pub content: Editor,
}

/// What the large editor remembers: the open editor, if any, and the session whose composer text
/// area the pointer is over (the open-editor button shows on hover, FR-004).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct State {
    pub open: Option<LargeEditor>,
    pub hovered: Option<SessionId>,
}

/// Everything the user can say to the large editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The chord or the button asked for the editor of `session`'s draft. Ignored while one is open.
    Requested(SessionId),
    /// An action in the editor's text area.
    Edited(EditorAction),
    /// **Done** or Ctrl/Cmd+Enter: the text becomes the draft.
    Done,
    /// **Cancel** or Escape: the draft stays as it was.
    Cancelled,
    /// The pointer entered or left `session`'s composer text area.
    Hovered(SessionId, bool),
}

/// What a key press means in the open editor: Ctrl+Enter or Cmd+Enter is **Done**, Escape is
/// **Cancel** (FR-002); every other key is the text area's own.
pub fn editor_key(press: &KeyInput) -> Option<Msg> {
    let m = press.mods;
    match press.key {
        Key::Named(NamedKey::Enter) if (m.ctrl || m.logo) && !m.alt && !m.shift => Some(Msg::Done),
        Key::Named(NamedKey::Escape) if m.is_empty() => Some(Msg::Cancelled),
        _ => None,
    }
}

/// The caret position at the end of `text`: last line, byte column.
fn end_of(text: &str) -> (usize, usize) {
    let line = text.matches('\n').count();
    (line, text.rsplit('\n').next().map_or(0, str::len))
}

/// Apply one message.
pub fn update(app: &mut App, msg: Msg) {
    match msg {
        Msg::Requested(session) => {
            app.large_editor.hovered = None;
            if app.large_editor.open.is_none() {
                let text = app
                    .composers
                    .draft(session)
                    .map(|draft| draft.text.clone())
                    .unwrap_or_default();
                app.large_editor.open = Some(LargeEditor {
                    session,
                    content: Editor::at(&text, end_of(&text)),
                });
            }
        }
        Msg::Edited(action) => {
            if let Some(open) = app.large_editor.open.as_mut() {
                open.content.act(&action);
            }
        }
        Msg::Done => {
            if let Some(open) = app.large_editor.open.take() {
                let text = open.content.text();
                let caret = end_of(&text);
                app.composers.replace_text(open.session, text, caret);
            }
        }
        Msg::Cancelled => app.large_editor.open = None,
        Msg::Hovered(session, over) => {
            if over {
                app.large_editor.hovered = Some(session);
            } else if app.large_editor.hovered == Some(session) {
                app.large_editor.hovered = None;
            }
        }
    }
}

/// The dialog, as a floating surface: open while an editor is. Escape and a scrim click cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LargeEditorDialog;

floating_surface!(LargeEditorDialog, id: SurfaceId::new("large_editor"), layer: Dialog, cancelled_by: Message::LargeEditor(Msg::Cancelled));

impl Registered for LargeEditorDialog {
    fn open_in(state: &App) -> Option<Self> {
        state.large_editor.open.as_ref().map(|_| LargeEditorDialog)
    }
}
