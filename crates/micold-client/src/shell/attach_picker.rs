//! The structured composer's **Add attachment** picker (#1464, research R1): the system's own
//! multi-file dialog, opened off the UI thread so the interface keeps answering while it is up
//! (FR-007, SC-004).

use iced::Task;
use micold_client::app::Message;
use micold_client::features::composer::Msg;
use micold_core::session::SessionId;

/// Open the picker for `session`. Its answer is `AttachPicked` with the chosen files in the order
/// the dialog returned them, or `AttachPickerClosed` when none were chosen (FR-009). Parentless:
/// the dialog is not tied to the main window, which is what the portal and Windows accept
/// (research R2).
pub fn open(session: SessionId) -> Task<Message> {
    Task::future(async move {
        let picked = rfd::AsyncFileDialog::new()
            .set_title("Add attachment")
            .pick_files()
            .await;
        Message::Composer(answer(
            session,
            picked.map(|files| files.into_iter().map(|f| f.path().to_path_buf()).collect()),
        ))
    })
}

/// The message for what the dialog returned: no files, whether cancelled or empty, is a close.
fn answer(session: SessionId, picked: Option<Vec<std::path::PathBuf>>) -> Msg {
    match picked {
        Some(paths) if !paths.is_empty() => Msg::AttachPicked { session, paths },
        _ => Msg::AttachPickerClosed(session),
    }
}

#[cfg(test)]
mod tests {
    use super::answer;
    use micold_client::features::composer::Msg;
    use micold_core::session::SessionId;
    use std::path::PathBuf;

    #[test]
    fn a_choice_keeps_its_order_and_a_cancel_or_an_empty_choice_is_a_close() {
        let s = SessionId::new();
        let paths = vec![PathBuf::from("/d/b.png"), PathBuf::from("/d/a.txt")];
        assert_eq!(
            answer(s, Some(paths.clone())),
            Msg::AttachPicked { session: s, paths }
        );
        assert_eq!(answer(s, None), Msg::AttachPickerClosed(s));
        assert_eq!(answer(s, Some(Vec::new())), Msg::AttachPickerClosed(s));
    }
}
