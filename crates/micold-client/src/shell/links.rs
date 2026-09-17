//! Opening an activated link (feature 031, contract link-opening §3).
//!
//! The session reducer decides what an activation asks for; this performs it. An open is I/O that
//! can wait on a launcher (the launch window in `shell/link_opener.rs`), so it runs on a blocking
//! task, and its answer comes back as `LinkOpenFinished` for the reducer to report (FR-015).

use iced::Task;

use micold_client::app::Message;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::{OpenFailure, OpenRequest, Outcome};

use crate::App;

/// Run the session reducer on a link message and perform what it asks for.
///
/// The root applies the message and returns the effect requests: an open goes to [`perform`], a
/// clipboard write to `shell::clipboard::interpret`.
pub fn on_link_message(app: &mut App, msg: SessionMsg) -> Task<Message> {
    let effects = app.core.update_session_for_effects(msg);
    Task::batch(effects.into_iter().map(|effect| match effect {
        Outcome::OpenLink(request) => perform(app, request),
        other => crate::shell::clipboard::interpret(other),
    }))
}

/// Hand `request` to the opener on a blocking task, with nothing in between (SC-003).
fn perform(app: &App, request: OpenRequest) -> Task<Message> {
    let opener = app.caps.link_opener();
    match request {
        OpenRequest::Url(address) => Task::perform(
            async move {
                let target = address.clone();
                let result = tokio::task::spawn_blocking(move || opener.open(&target))
                    .await
                    .unwrap_or_else(|e| Err(OpenFailure::LaunchFailed(e.to_string())));
                (address, result)
            },
            |(address, result)| Message::Session(SessionMsg::LinkOpenFinished { address, result }),
        ),
        // File links arrive with M5 (contract §3, O4); nothing emits this request before then.
        OpenRequest::Path { .. } => Task::none(),
    }
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
    pub(super) struct RecordingOpener {
        pub(super) opened: Mutex<Vec<(String, Instant)>>,
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
    pub(super) fn run(task: iced::Task<Message>) -> Vec<Message> {
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

/// The outer loop for the pane (feature 031, A1–A11 and A13): the real session pane from
/// `ui::terminal::pane`, laid out headless, pointer and modifier events in, and every message it
/// publishes run through `update_inner` on a `base_app()` whose opener records.
#[cfg(test)]
mod acceptance {
    use std::sync::Arc;

    use iced::advanced::renderer::Headless;
    use iced::advanced::widget::Tree;
    use iced::advanced::{clipboard, layout::Limits, Layout, Shell};
    use iced::{keyboard, mouse, Event, Point, Rectangle, Size};
    use micold_client::app::Message;
    use micold_client::features::session::Msg as SessionMsg;
    use micold_client::grid::GridCache;
    use micold_client::ui::terminal::{CellMetrics, TERM_FONT_SIZE};
    use micold_core::link::{CellSpan, LinkContext, ResolvedLink};
    use micold_core::protocol::grid::{
        CellExtras, GridFrame, LineId, StyleRun, WireColor, WireCursor, WireCursorShape, WireLine,
        WireStyle,
    };
    use micold_core::session::SessionId;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens::state::FOCUS_RING_WIDTH;

    use super::tests::{run, RecordingOpener};
    use crate::App;

    const COLS: u16 = 60;
    const ROWS: u16 = 6;
    const WINDOW: Size = Size::new(900.0, 400.0);
    const SENTENCE: &str = "See https://example.com/docs/page.html for details.";
    const ADDRESS: &str = "https://example.com/docs/page.html";
    const MANUAL: &str = "https://example.com/manual";

    /// A printed line: its text and the declared runs on it.
    struct Line {
        text: String,
        wrapped: bool,
        declared: Vec<(std::ops::Range<u16>, &'static str)>,
    }

    fn line(text: &str) -> Line {
        Line {
            text: text.to_string(),
            wrapped: false,
            declared: Vec::new(),
        }
    }

    impl Line {
        fn wrapped(mut self) -> Self {
            self.wrapped = true;
            self
        }
        fn declare(mut self, cols: std::ops::Range<u16>, uri: &'static str) -> Self {
            self.declared.push((cols, uri));
            self
        }
    }

    /// A session showing `lines` at the given line ids, with its screen at `viewport_top`.
    struct Session {
        app: App,
        id: SessionId,
        opener: Arc<RecordingOpener>,
        tree: Option<Tree>,
        renderer: iced::Renderer,
        /// Where the pointer is, and what it looks like after the last event.
        cursor: Point,
        pointer: Option<mouse::Interaction>,
    }

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        let mut f = Box::pin(f);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
            std::hint::spin_loop();
        }
    }

    impl Session {
        fn showing(lines: Vec<(i64, Line)>, viewport_top: i64) -> Self {
            const STYLE: WireStyle = WireStyle {
                fg: WireColor::Named(256),
                bg: WireColor::Named(257),
                flags: 0,
                underline_color: None,
            };
            let mut hyperlinks: Vec<String> = Vec::new();
            let wire = lines
                .into_iter()
                .map(|(id, line)| {
                    let text = format!("{:<width$}", line.text, width = COLS as usize);
                    let mut extras = Vec::new();
                    for (cols, uri) in line.declared {
                        let index = hyperlinks.iter().position(|u| u == uri).unwrap_or_else(|| {
                            hyperlinks.push(uri.to_string());
                            hyperlinks.len() - 1
                        });
                        extras.extend(cols.map(|col| CellExtras {
                            col,
                            zerowidth: Vec::new(),
                            hyperlink: Some(index as u16),
                        }));
                    }
                    WireLine {
                        id: LineId(id),
                        runs: vec![StyleRun {
                            len: text.chars().count() as u16,
                            style: 0,
                        }],
                        text,
                        extras,
                        wrapped: line.wrapped,
                    }
                })
                .collect();
            let id = SessionId::new();
            let mut grid = GridCache::new();
            grid.apply(&GridFrame {
                session: id,
                seq: 1,
                generation: 1,
                full: true,
                viewport_top: LineId(viewport_top),
                oldest_available: LineId(0),
                cols: COLS,
                rows: ROWS,
                cursor: WireCursor {
                    line: LineId(viewport_top),
                    col: 0,
                    shape: WireCursorShape::Block,
                    visible: false,
                    blinking: false,
                },
                styles: vec![STYLE],
                hyperlinks,
                lines: wire,
                mode: 0,
                input_serial: None,
            });
            let opener = Arc::new(RecordingOpener::default());
            let mut app = crate::tests::base_app();
            app.caps = app.caps.clone().with_link_opener(opener.clone());
            app.core.session.active = Some(id);
            app.grids.insert(id, grid);
            let renderer = block_on(<iced::Renderer as Headless>::new(
                iced::Font::DEFAULT,
                iced::Pixels(16.0),
                Some("tiny-skia"),
            ))
            .expect("the tiny-skia headless renderer");
            Self {
                app,
                id,
                opener,
                tree: None,
                renderer,
                cursor: Point::ORIGIN,
                pointer: None,
            }
        }

        fn on_screen(lines: Vec<Line>) -> Self {
            Self::showing(
                lines
                    .into_iter()
                    .enumerate()
                    .map(|(row, line)| (row as i64, line))
                    .collect(),
                0,
            )
        }

        /// Deliver `event` to the pane with the pointer at `cursor`, and run what it publishes
        /// through `update_inner`. Returns what the pane published.
        fn send(&mut self, cursor: Point, event: Event) -> Vec<Message> {
            self.cursor = cursor;
            let published = {
                let context = LinkContext {
                    host_names: Vec::new(),
                    windows_host: cfg!(windows),
                    sandbox: None,
                };
                let mut element = micold_client::ui::terminal::pane(
                    &self.app.core,
                    self.app.grids.get(&self.id),
                    self.app.selection.as_ref(),
                    self.app.display_offset,
                    ColorScheme::Dark,
                    context,
                );
                if self.tree.is_none() {
                    self.tree = Some(Tree::new(&element));
                }
                let tree = self.tree.as_mut().expect("just made");
                tree.diff(&element);
                let node = element.as_widget_mut().layout(
                    tree,
                    &self.renderer,
                    &Limits::new(Size::ZERO, WINDOW),
                );
                let mut messages = Vec::new();
                let mut shell = Shell::new(&mut messages);
                element.as_widget_mut().update(
                    tree,
                    &event,
                    Layout::new(&node),
                    mouse::Cursor::Available(cursor),
                    &self.renderer,
                    &mut clipboard::Null,
                    &mut shell,
                    &Rectangle::with_size(WINDOW),
                );
                let pointer = element.as_widget().mouse_interaction(
                    tree,
                    Layout::new(&node),
                    mouse::Cursor::Available(cursor),
                    &Rectangle::with_size(WINDOW),
                    &self.renderer,
                );
                (messages, pointer)
            };
            let (published, pointer) = published;
            self.pointer = Some(pointer);
            for message in published.clone() {
                run(crate::update_inner(&mut self.app, message));
            }
            published
        }

        /// The pointer the pane shows after the last event.
        fn pointer(&self) -> Option<mouse::Interaction> {
            self.pointer
        }

        fn hover(&mut self, col: u16, row: u16) -> Vec<Message> {
            self.send(at(col, row), moved(at(col, row)))
        }

        fn hold(&mut self, modifiers: keyboard::Modifiers) {
            let cursor = self.cursor;
            self.send(
                cursor,
                Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)),
            );
        }

        /// Hover the cell, hold Ctrl (Cmd on macOS), press and release there, and let go of it.
        fn activate(&mut self, col: u16, row: u16) -> Vec<Message> {
            let mut published = self.hover(col, row);
            self.hold(keyboard::Modifiers::COMMAND);
            published.extend(self.send(at(col, row), press()));
            published.extend(self.send(at(col, row), release()));
            self.hold(keyboard::Modifiers::empty());
            published
        }

        fn opened(&self) -> Vec<String> {
            self.opener
                .opened
                .lock()
                .unwrap()
                .iter()
                .map(|(target, _)| target.clone())
                .collect()
        }
    }

    /// The centre of a cell: the pane is the column's first child, at the window's origin.
    fn at(col: u16, row: u16) -> Point {
        let m = CellMetrics::new(TERM_FONT_SIZE);
        Point::new(
            FOCUS_RING_WIDTH + (col as f32 + 0.5) * m.width,
            FOCUS_RING_WIDTH + (row as f32 + 0.5) * m.height,
        )
    }

    fn moved(position: Point) -> Event {
        Event::Mouse(mouse::Event::CursorMoved { position })
    }

    fn press() -> Event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
    }

    fn release() -> Event {
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
    }

    fn activations(published: &[Message]) -> Vec<ResolvedLink> {
        published
            .iter()
            .filter_map(|m| match m {
                Message::Session(SessionMsg::LinkActivated(link)) => Some(link.clone()),
                _ => None,
            })
            .collect()
    }

    /// A1.
    #[test]
    fn hovering_an_address_marks_exactly_its_cells_and_the_pointer_is_a_hand() {
        let mut session = Session::on_screen(vec![line(SENTENCE)]);
        for col in 0..SENTENCE.len() as u16 {
            session.hover(col, 0);
            session.hold(keyboard::Modifiers::COMMAND);
            let expected = if (4..38).contains(&col) {
                mouse::Interaction::Pointer
            } else {
                mouse::Interaction::Text
            };
            assert_eq!(
                session.pointer(),
                Some(expected),
                "column {col} of {SENTENCE:?} (FR-005, FR-007)"
            );
            session.hold(keyboard::Modifiers::empty());
        }
        let activated = activations(&session.activate(20, 0));
        assert_eq!(
            activated
                .iter()
                .map(|l| l.link.cells.clone())
                .collect::<Vec<_>>(),
            vec![vec![CellSpan {
                row: 0,
                cols: 4..38
            }]],
            "the marked link covers the address, not `See`, the space or the full stop"
        );
    }

    /// A2.
    #[test]
    fn a_command_click_on_an_address_opens_it_once_and_writes_nothing_to_the_program() {
        let mut session = Session::on_screen(vec![line(SENTENCE)]);
        let published = session.activate(20, 0);
        assert_eq!(
            session.opened(),
            vec![ADDRESS.to_string()],
            "SC-001, FR-010"
        );
        assert!(
            !published
                .iter()
                .any(|m| matches!(m, Message::Session(SessionMsg::TerminalBytes(_)))),
            "the program gets no input (FR-014): {published:?}"
        );
    }

    /// A3.
    #[test]
    fn a_soft_wrapped_address_opens_complete_from_either_row() {
        let address = format!("https://example.com/{}end.html", "segment/".repeat(8));
        let prefix = "Read more at ";
        let split = COLS as usize - prefix.len();
        let rows = || {
            vec![
                line(&format!("{prefix}{}", &address[..split])).wrapped(),
                line(&format!("{} and more.", &address[split..])),
            ]
        };
        let mut first = Session::on_screen(rows());
        first.activate(20, 0);
        let mut second = Session::on_screen(rows());
        second.activate(3, 1);
        assert_eq!(
            first.opened(),
            vec![address.clone()],
            "from the first row (FR-003)"
        );
        assert_eq!(second.opened(), vec![address], "from the second row");
    }

    /// A4.
    #[test]
    fn surrounding_punctuation_is_left_out_of_what_opens() {
        let mut session = Session::on_screen(vec![
            line("(https://example.com/a_(b))"),
            line("\"https://example.com\""),
        ]);
        session.activate(5, 0);
        session.activate(5, 1);
        assert_eq!(
            session.opened(),
            vec![
                "https://example.com/a_(b)".to_string(),
                "https://example.com".to_string()
            ],
            "FR-005"
        );
    }

    /// A5: a guard, green before the gesture exists; its red is the mutant in tdd/test-list.md.
    #[test]
    fn selecting_on_a_link_selects_and_opens_nothing() {
        let mut session = Session::on_screen(vec![line(SENTENCE)]);
        session.hover(6, 0);
        session.send(at(6, 0), press());
        session.send(at(20, 0), moved(at(20, 0)));
        session.send(at(20, 0), release());
        assert!(session.app.selection.is_some(), "the drag selected");
        for clicks in [2, 3] {
            session.hover(30, 0);
            for _ in 0..clicks {
                session.send(at(30, 0), press());
                session.send(at(30, 0), release());
            }
        }
        assert!(session.app.selection.is_some());
        assert_eq!(session.opened(), Vec::<String>::new(), "FR-004");
    }

    /// A6.
    #[test]
    fn an_address_in_scrollback_opens_the_same_address() {
        let mut lines: Vec<(i64, Line)> = vec![(90, line(SENTENCE))];
        lines.extend((100..106).map(|id| (id, line("live output"))));
        let mut session = Session::showing(lines, 100);
        session.app.display_offset = 10;
        session.activate(20, 0);
        assert_eq!(
            session.opened(),
            vec![ADDRESS.to_string()],
            "US1 scenario 6"
        );
    }

    /// A13.
    #[test]
    fn a_mail_address_reaches_the_opener_verbatim() {
        let mut session = Session::on_screen(vec![line("mailto:team@example.com")]);
        session.activate(3, 0);
        assert_eq!(
            session.opened(),
            vec!["mailto:team@example.com".to_string()]
        );
    }

    /// A7.
    #[test]
    fn hovering_a_declared_run_marks_it_and_shows_its_declared_address() {
        let mut session =
            Session::on_screen(vec![line("Read the docs today").declare(9..13, MANUAL)]);
        session.hover(10, 0);
        session.hold(keyboard::Modifiers::COMMAND);
        assert_eq!(session.pointer(), Some(mouse::Interaction::Pointer));
        session.hold(keyboard::Modifiers::empty());
        let activated = activations(&session.activate(10, 0));
        assert_eq!(
            activated
                .iter()
                .map(|l| (l.display.as_str(), l.link.cells.clone()))
                .collect::<Vec<_>>(),
            vec![(
                MANUAL,
                vec![CellSpan {
                    row: 0,
                    cols: 9..13
                }]
            )],
            "the run is marked and the hint's text is the declared address (FR-002, FR-008)"
        );
    }

    /// A8.
    #[test]
    fn activating_a_declared_run_opens_its_declared_address() {
        let mut session =
            Session::on_screen(vec![line("Read the docs today").declare(9..13, MANUAL)]);
        session.activate(12, 0);
        assert_eq!(session.opened(), vec![MANUAL.to_string()]);
    }

    /// A9.
    #[test]
    fn adjacent_declared_runs_open_their_own_addresses() {
        let other = "https://example.com/other";
        let mut session = Session::on_screen(vec![line("docsother")
            .declare(0..4, MANUAL)
            .declare(4..9, other)]);
        session.activate(3, 0);
        session.activate(4, 0);
        assert_eq!(
            session.opened(),
            vec![MANUAL.to_string(), other.to_string()]
        );
    }

    /// A10.
    #[test]
    fn same_address_runs_apart_are_marked_one_at_a_time() {
        let mut session = Session::on_screen(vec![line("docs and docs")
            .declare(0..4, MANUAL)
            .declare(9..13, MANUAL)]);
        let activated = activations(&session.activate(10, 0));
        assert_eq!(
            activated
                .iter()
                .map(|l| l.link.cells.clone())
                .collect::<Vec<_>>(),
            vec![vec![CellSpan {
                row: 0,
                cols: 9..13
            }]],
            "US2 scenario 4"
        );
    }

    /// A11.
    #[test]
    fn declared_text_that_reads_as_another_address_shows_and_opens_the_declared_one() {
        let mut session = Session::on_screen(vec![
            line("https://a.example").declare(0..17, "https://b.example")
        ]);
        let activated = activations(&session.activate(5, 0));
        assert_eq!(
            activated
                .iter()
                .map(|l| l.display.as_str())
                .collect::<Vec<_>>(),
            vec!["https://b.example"]
        );
        assert_eq!(
            session.opened(),
            vec!["https://b.example".to_string()],
            "FR-008"
        );
    }
}
