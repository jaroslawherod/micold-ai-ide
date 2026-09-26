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
        OpenRequest::Path { path, address } => Task::perform(
            async move {
                // One blocking task for the whole of O4: the file's facts are read and acted on back
                // to back, so the window between the check and the open is two syscalls wide.
                let result = tokio::task::spawn_blocking(move || open_path(&*opener, &path))
                    .await
                    .unwrap_or_else(|e| Err(OpenFailure::LaunchFailed(e.to_string())));
                (address, result)
            },
            |(address, result)| Message::Session(SessionMsg::LinkOpenFinished { address, result }),
        ),
    }
}

/// Open `path`, or reveal it when this machine would run it (contract link-opening §3, O4).
///
/// `std::fs::metadata` follows symbolic links, so a link is judged by the file it points to
/// (FR-013). Everything platform-specific is read here and handed to
/// `micold_core::link::runnable::action_for`, which decides.
fn open_path(
    opener: &dyn crate::shell::link_opener::LinkOpener,
    path: &str,
) -> Result<(), OpenFailure> {
    use micold_core::link::runnable::{action_for, FileAction};

    let file = std::path::Path::new(path);
    let meta = match std::fs::metadata(file) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(OpenFailure::NotFound),
        Err(e) => return Err(OpenFailure::LaunchFailed(e.to_string())),
    };
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match action_for(HOST_PLATFORM, &name, facts_for(&meta, file), &pathext()) {
        FileAction::Open => opener.open(path),
        FileAction::Reveal => opener.reveal(file),
    }
}

/// Which platform's runnable rules apply here (research R9: a parameter, never a `cfg` in core).
#[cfg(target_os = "linux")]
const HOST_PLATFORM: micold_core::link::runnable::HostPlatform =
    micold_core::link::runnable::HostPlatform::Linux;
#[cfg(target_os = "macos")]
const HOST_PLATFORM: micold_core::link::runnable::HostPlatform =
    micold_core::link::runnable::HostPlatform::MacOs;
#[cfg(windows)]
const HOST_PLATFORM: micold_core::link::runnable::HostPlatform =
    micold_core::link::runnable::HostPlatform::Windows;

/// What this machine can say about the file, without opening it.
fn facts_for(
    meta: &std::fs::Metadata,
    path: &std::path::Path,
) -> micold_core::link::runnable::FileFacts {
    use micold_core::link::runnable::{FileFacts, Kind};

    let kind = if meta.is_dir() { Kind::Dir } else { Kind::File };
    #[cfg(unix)]
    let any_exec_bit = {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    };
    // Windows has no execute bit; there the extension decides (FR-013).
    #[cfg(windows)]
    let any_exec_bit = false;
    // A macOS bundle the Finder knows only by what it holds; the extension list is core's.
    #[cfg(target_os = "macos")]
    let is_bundle = kind == Kind::Dir && path.join("Contents/Info.plist").exists();
    #[cfg(not(target_os = "macos"))]
    let is_bundle = {
        let _ = path;
        false
    };
    FileFacts {
        kind,
        any_exec_bit,
        is_bundle,
    }
}

/// The `%PATHEXT%` entries this machine runs, empty off Windows (FR-013).
fn pathext() -> Vec<String> {
    #[cfg(windows)]
    {
        std::env::var("PATHEXT")
            .unwrap_or_default()
            .split(';')
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.to_string())
            .collect()
    }
    #[cfg(not(windows))]
    Vec::new()
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

    /// Records each `open` with the moment it was called, and each `reveal`.
    #[derive(Default)]
    pub(super) struct RecordingOpener {
        pub(super) opened: Mutex<Vec<(String, Instant)>>,
        pub(super) revealed: Mutex<Vec<std::path::PathBuf>>,
    }

    impl LinkOpener for RecordingOpener {
        fn open(&self, target: &str) -> Result<(), OpenFailure> {
            self.opened
                .lock()
                .unwrap()
                .push((target.to_string(), Instant::now()));
            Ok(())
        }

        fn reveal(&self, path: &std::path::Path) -> Result<(), OpenFailure> {
            self.revealed.lock().unwrap().push(path.to_path_buf());
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

    /// A real file in `dir` this machine would run rather than read: the execute bit on Unix, a
    /// `%PATHEXT%` extension on Windows (FR-013).
    pub(super) fn runnable_file(dir: &std::path::Path, stem: &str) -> std::path::PathBuf {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.join(stem);
            std::fs::write(&path, b"#!/bin/sh\n").expect("write the runnable file");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("set the execute bit");
            path
        }
        #[cfg(windows)]
        {
            let path = dir.join(format!("{stem}.bat"));
            std::fs::write(&path, b"@echo off\n").expect("write the runnable file");
            path
        }
    }

    /// Every kind of file this OS runs, as real files inside `dir` (SC-007, FR-013).
    ///
    /// Each entry is what the spec's FR-013 list names for this platform, created for real so the
    /// fact gatherer reads a real `metadata` rather than a fixture.
    fn every_runnable_kind(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut kinds = vec![runnable_file(dir, "build")];
        #[cfg(unix)]
        {
            let link = dir.join("link-to-build");
            std::os::unix::fs::symlink(&kinds[0], &link).expect("symlink to an executable file");
            kinds.push(link);
        }
        #[cfg(target_os = "linux")]
        for name in ["app.desktop", "Tool.AppImage"] {
            let path = dir.join(name);
            std::fs::write(&path, b"x").expect("write a launcher");
            kinds.push(path);
        }
        #[cfg(target_os = "macos")]
        {
            let bundle = dir.join("Thing.app");
            std::fs::create_dir_all(bundle.join("Contents")).expect("create a bundle");
            kinds.push(bundle);
            // U146: a bundle the Finder knows only by what it holds.
            let plain = dir.join("Plain");
            std::fs::create_dir_all(plain.join("Contents")).expect("create a directory");
            std::fs::write(plain.join("Contents/Info.plist"), b"<plist/>").expect("write a plist");
            kinds.push(plain);
            let command = dir.join("run.command");
            std::fs::write(&command, b"echo hi\n").expect("write a .command file");
            kinds.push(command);
        }
        #[cfg(windows)]
        for name in ["a.exe", "b.ps1", "c.cmd", "d.vbs"] {
            let path = dir.join(name);
            std::fs::write(&path, b"x").expect("write a runnable file");
            kinds.push(path);
        }
        kinds
    }

    /// Files every OS reads rather than runs (SC-007).
    fn every_document_kind(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        ["notes.txt", "pic.png", "report.pdf", "page.html"]
            .iter()
            .map(|name| {
                let path = dir.join(name);
                std::fs::write(&path, b"x").expect("write a document");
                path
            })
            .collect()
    }

    /// U97, U146 (SC-007): on this OS, every runnable kind is revealed and every document opened,
    /// including inside a directory whose name has a space.
    #[test]
    fn every_runnable_kind_is_revealed_and_every_document_opened() {
        let root = tempfile::tempdir().expect("a temp dir");
        for holder in ["plain", "with space"] {
            let dir = root.path().join(holder);
            std::fs::create_dir(&dir).expect("create the holding directory");
            for path in every_runnable_kind(&dir) {
                let (opener, _) = activate(path_link("file:///x", &path));
                assert_eq!(
                    *opener.revealed.lock().unwrap(),
                    vec![path.clone()],
                    "{} is a kind this machine runs, so it is revealed (SC-007, FR-013)",
                    path.display()
                );
                assert!(
                    opened(&opener).is_empty(),
                    "{} must never be handed to the system opener",
                    path.display()
                );
            }
            for path in every_document_kind(&dir) {
                let (opener, _) = activate(path_link("file:///x", &path));
                assert_eq!(
                    opened(&opener),
                    vec![path.to_string_lossy().to_string()],
                    "{} is a document, so it opens in its application (SC-007)",
                    path.display()
                );
                assert!(
                    opener.revealed.lock().unwrap().is_empty(),
                    "{} is not revealed",
                    path.display()
                );
            }
        }
    }

    fn path_link(address: &str, path: &std::path::Path) -> ResolvedLink {
        ResolvedLink {
            target: Target::HostPath(path.to_string_lossy().into_owned()),
            display: path.to_string_lossy().into_owned(),
            ..url_link(address)
        }
    }

    /// Activate `link` through `update_inner` with a recording opener, and return what the opener
    /// was asked for and what came back.
    fn activate(link: ResolvedLink) -> (Arc<RecordingOpener>, Vec<Message>) {
        let opener = Arc::new(RecordingOpener::default());
        let mut app = crate::tests::base_app();
        app.caps = app.caps.clone().with_link_opener(opener.clone());
        let messages = run(crate::update_inner(
            &mut app,
            Message::Session(SessionMsg::LinkActivated(link)),
        ));
        (opener, messages)
    }

    fn opened(opener: &RecordingOpener) -> Vec<String> {
        opener
            .opened
            .lock()
            .unwrap()
            .iter()
            .map(|(t, _)| t.clone())
            .collect()
    }

    /// U95 (T11): a file the program named but this machine does not have.
    #[test]
    fn opening_a_path_that_is_not_there_finishes_as_not_found_and_calls_no_opener() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let missing = dir.path().join("gone.txt");
        let address = "file:///gone.txt";
        let (opener, messages) = activate(path_link(address, &missing));
        assert_eq!(
            messages,
            vec![Message::Session(SessionMsg::LinkOpenFinished {
                address: address.to_string(),
                result: Err(OpenFailure::NotFound),
            })],
            "the answer is NotFound for the address the program printed (FR-015)"
        );
        assert!(
            opened(&opener).is_empty() && opener.revealed.lock().unwrap().is_empty(),
            "nothing is handed to the operating system for a file that is not there"
        );
    }

    /// U96 (T11): a document is opened and a runnable file is revealed, through the real facts.
    #[test]
    fn a_document_is_opened_and_a_runnable_file_is_revealed() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let doc = dir.path().join("notes.txt");
        std::fs::write(&doc, b"x").expect("write the document");
        let (opener, messages) = activate(path_link("file:///notes.txt", &doc));
        assert_eq!(
            opened(&opener),
            vec![doc.to_string_lossy().to_string()],
            "a document goes to the system opener (FR-010)"
        );
        assert_eq!(
            messages,
            vec![Message::Session(SessionMsg::LinkOpenFinished {
                address: "file:///notes.txt".to_string(),
                result: Ok(()),
            })],
            "and the answer comes back for the same address"
        );

        let runnable = runnable_file(dir.path(), "run");
        let (opener, _) = activate(path_link("file:///run", &runnable));
        assert_eq!(
            *opener.revealed.lock().unwrap(),
            vec![runnable],
            "a file this machine runs is revealed, never opened (FR-013)"
        );
        assert!(
            opened(&opener).is_empty(),
            "and never handed to the system opener, which would run it"
        );
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

/// The outer loop for the pane (feature 031, A1–A18): the real session pane from
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
    use micold_core::link::{CellSpan, ResolvedLink};
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
        declared: Vec<(std::ops::Range<u16>, String)>,
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
        fn declare(mut self, cols: std::ops::Range<u16>, uri: &str) -> Self {
            self.declared.push((cols, uri.to_string()));
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
                        let index =
                            hyperlinks
                                .iter()
                                .position(|u| *u == uri)
                                .unwrap_or_else(|| {
                                    hyperlinks.push(uri.clone());
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

        /// The names this machine answers to, as boot fills them (U136).
        fn named(mut self, host_names: &[&str]) -> Self {
            self.app.core.session.host_names = host_names.iter().map(|n| n.to_string()).collect();
            self
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
                // The context the application builds, so these tests cover the glue that fills it
                // (U134, U136).
                let context =
                    micold_client::ui::terminal::link_context(&self.app.core, &self.app.sandbox);
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
            // Every message, and every message its task produces in turn, exactly as iced's runtime
            // feeds them back: an open's `LinkOpenFinished` is a second round, and the notification
            // it raises is only visible once that round has run.
            let mut pending: std::collections::VecDeque<Message> =
                published.iter().cloned().collect();
            while let Some(message) = pending.pop_front() {
                pending.extend(run(crate::update_inner(&mut self.app, message)));
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

        fn revealed(&self) -> Vec<std::path::PathBuf> {
            self.opener.revealed.lock().unwrap().clone()
        }

        /// Every notification raised so far, in arrival order.
        fn notifications(&mut self) -> Vec<String> {
            let mut seen = Vec::new();
            while let Some(n) = self.app.core.notifications.queue.visible().cloned() {
                seen.push(n.message);
                self.app.core.notifications.queue.dismiss();
            }
            seen
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

    // -----------------------------------------------------------------------------------
    // File links on this machine (US3 scenarios 2–6, US2 scenario 6)
    // -----------------------------------------------------------------------------------

    /// A `file://` address for `path`, as a program prints it: the host, then the path with each
    /// space percent-encoded and each separator a `/`.
    fn file_url(host: &str, path: &std::path::Path) -> String {
        let path = path.to_string_lossy().replace('\\', "/");
        let path = if path.starts_with('/') {
            path
        } else {
            format!("/{path}")
        };
        format!("file://{host}{}", path.replace(' ', "%20"))
    }

    /// What the opener is handed for `path` on this platform.
    fn host_path(path: &std::path::Path) -> String {
        path.to_string_lossy().into_owned()
    }

    /// A12: `ls --hyperlink=always` declares the host's name and percent-encodes the space.
    #[test]
    fn a_declared_file_link_naming_this_host_opens_the_decoded_path() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let file = dir.path().join("readme a.txt");
        std::fs::write(&file, b"hello").expect("write the document");
        let address = file_url("devbox", &file);
        let mut session = Session::on_screen(vec![line("readme a.txt").declare(0..12, &address)])
            .named(&["devbox.example.com", "devbox"]);
        session.activate(3, 0);
        assert_eq!(
            session.opened(),
            vec![host_path(&file)],
            "a declared file link naming this machine opens the decoded path (US2.6, FR-012)"
        );
    }

    /// A14.
    #[test]
    fn activating_a_file_link_to_a_document_opens_its_host_path() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, b"hello").expect("write the document");
        let address = file_url("", &file);
        let mut session = Session::on_screen(vec![line(&address)]);
        session.activate(10, 0);
        assert_eq!(
            session.opened(),
            vec![host_path(&file)],
            "a document opens in its application (US3.2, FR-010)"
        );
        assert!(
            session.revealed().is_empty(),
            "a document is opened, not revealed"
        );
    }

    /// A15.
    #[test]
    fn activating_a_file_link_to_a_folder_opens_the_folder() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let folder = dir.path().join("notes");
        std::fs::create_dir(&folder).expect("create the folder");
        let address = file_url("", &folder);
        let mut session = Session::on_screen(vec![line(&address)]);
        session.activate(10, 0);
        assert_eq!(
            session.opened(),
            vec![host_path(&folder)],
            "a folder opens in the file manager (US3.3, FR-010)"
        );
    }

    /// A16.
    #[test]
    fn activating_a_file_link_to_a_runnable_file_reveals_it_and_never_opens_it() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let file = super::tests::runnable_file(dir.path(), "build");
        let address = file_url("", &file);
        let mut session = Session::on_screen(vec![line(&address)]);
        session.activate(10, 0);
        assert_eq!(
            session.revealed(),
            vec![file.clone()],
            "a runnable file is shown in the file manager (US3.4, FR-013)"
        );
        assert_eq!(
            session.opened(),
            Vec::<String>::new(),
            "and is never handed to the system opener, which would run it"
        );
    }

    /// A17.
    #[test]
    fn activating_a_file_link_to_a_missing_file_opens_nothing_and_says_so() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let missing = dir.path().join("gone.txt");
        let address = file_url("", &missing);
        let mut session = Session::on_screen(vec![line(&address)]);
        session.activate(10, 0);
        assert_eq!(
            session.opened(),
            Vec::<String>::new(),
            "nothing opens for a file that is not there (US3.5)"
        );
        assert_eq!(
            session.notifications(),
            vec![format!(
                "Couldn't open {address}: the file doesn't exist on this machine"
            )],
            "one notification naming the address the program printed (FR-015)"
        );
    }

    /// A18: a guard, green on arrival; its red is the mutant in tdd/test-list.md.
    #[test]
    fn a_file_link_to_another_machine_and_an_application_scheme_are_no_links() {
        for address in ["file://otherhost/x", "javascript:alert(1)", "vscode://x"] {
            let mut session = Session::on_screen(vec![line("open this").declare(0..9, address)])
                .named(&["devbox"]);
            session.hover(3, 0);
            session.hold(keyboard::Modifiers::COMMAND);
            assert_eq!(
                session.pointer(),
                Some(mouse::Interaction::Text),
                "{address} is not offered as a link (US3.6, FR-011, FR-012)"
            );
            session.hold(keyboard::Modifiers::empty());
            assert_eq!(
                session.opened(),
                Vec::<String>::new(),
                "{address} opens nothing"
            );
            assert!(session.revealed().is_empty(), "{address} reveals nothing");
        }
    }
}
