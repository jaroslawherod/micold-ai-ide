//! Opening an activated link (feature 031, contract link-opening §3).

use iced::Task;

use micold_client::app::Message;
use micold_client::features::session::Msg as SessionMsg;

use crate::App;

/// Run the session reducer on a link message and perform what it asks for.
pub fn on_link_message(_app: &mut App, _msg: SessionMsg) -> Task<Message> {
    Task::none()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use iced::futures::StreamExt;
    use micold_client::app::Message;
    use micold_client::features::session::Msg as SessionMsg;
    use micold_client::features::OpenFailure;
    use micold_core::link::{CellSpan, Link, LinkOrigin, ResolvedLink, Target};

    use crate::shell::link_opener::LinkOpener;

    /// Records each `open` with the moment it was called.
    #[derive(Default)]
    struct RecordingOpener {
        opened: Mutex<Vec<(String, Instant)>>,
    }

    impl LinkOpener for RecordingOpener {
        fn open(&self, target: &str) -> Result<(), OpenFailure> {
            self.opened
                .lock()
                .unwrap()
                .push((target.to_string(), Instant::now()));
            Ok(())
        }

        fn reveal(&self, _path: &std::path::Path) -> Result<(), OpenFailure> {
            Ok(())
        }
    }

    fn url_link(address: &str) -> ResolvedLink {
        ResolvedLink {
            link: Link {
                address: address.to_string(),
                origin: LinkOrigin::Detected,
                cells: vec![CellSpan {
                    row: 0,
                    cols: 0..address.len() as u16,
                }],
            },
            display: address.to_string(),
            target: Target::Url(address.to_string()),
            needs_confirmation: false,
        }
    }

    /// Every message the task produces, run to its end on a tokio runtime, as iced's executor would.
    fn run(task: iced::Task<Message>) -> Vec<Message> {
        let Some(stream) = iced_runtime::task::into_stream(task) else {
            return Vec::new();
        };
        let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
        runtime.block_on(async {
            stream
                .filter_map(|action| async move {
                    match action {
                        iced_runtime::Action::Output(message) => Some(message),
                        _ => None,
                    }
                })
                .collect()
                .await
        })
    }

    /// U93 and U94: through `update_inner`, the opener receives the address as written, at once.
    #[test]
    fn activating_a_url_through_update_inner_opens_it_verbatim_and_without_delay() {
        let address = "https://example.com/a(b)?q=%20x#frag";
        let opener = Arc::new(RecordingOpener::default());
        let mut app = crate::tests::base_app();
        app.caps = app.caps.clone().with_link_opener(opener.clone());

        let activated = Instant::now();
        let task = crate::update_inner(
            &mut app,
            Message::Session(SessionMsg::LinkActivated(url_link(address))),
        );
        let messages = run(task);

        let opened = opener.opened.lock().unwrap().clone();
        assert_eq!(
            opened.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(),
            vec![address],
            "one activation is one open of exactly the address (FR-010)"
        );
        assert!(
            opened[0].1.duration_since(activated) < Duration::from_millis(250),
            "nothing waits between the activation and the opener (SC-003), took {:?}",
            opened[0].1.duration_since(activated)
        );
        assert_eq!(
            messages,
            vec![Message::Session(SessionMsg::LinkOpenFinished {
                address: address.to_string(),
                result: Ok(()),
            })],
            "the opener's answer comes back as LinkOpenFinished for the same address"
        );
    }
}
