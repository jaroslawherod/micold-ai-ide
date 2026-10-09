//! Terminal-area panes (feature 484): keeping each project's [`PaneLayout`] in step with what the
//! session reducers select, and telling the daemon which terminals to stream.
//!
//! The decisions live in `micold_core::pane_layout`; this module is the glue that applies them to
//! `App`. A project that was never split has one pane showing the displayed terminal, which is
//! exactly today's behaviour (FR-017): no `SetViewedTerminals` goes out for it.

use std::path::PathBuf;

use iced::Task;
use micold_client::keymap::PaneAction;
use micold_core::pane_layout::{Axis, PaneId, PaneLayout, Refusal, MIN_PANE_COLS, MIN_PANE_ROWS};
use micold_core::protocol::messages::{ClientMsg, SessionProcess, TerminalRef};

use crate::App;
use micold_client::app::Message;
use micold_client::features::session::{Msg as SessionMsg, PaneMsg};

/// The project whose panes are displayed.
fn project(app: &App) -> Option<PathBuf> {
    app.core.workspace.active.clone()
}

/// Whether `t` still exists: its session is known and, for a shell, the instance is open.
fn is_live(app: &App, t: TerminalRef) -> bool {
    match app.core.workspace.find_session(t.session) {
        None => false,
        Some((_, s)) => match t.process {
            SessionProcess::Primary => true,
            SessionProcess::Shell(id) => s.shells.iter().any(|i| i.id == id),
        },
    }
}

/// The active project's layout, if it has been split or has shown anything.
pub fn layout(app: &App) -> Option<&PaneLayout> {
    app.pane_layouts.get(&project(app)?)
}

/// Bring the layout in step with the selection, then tell the daemon when the visible set changed.
/// Called after every message (`update`), so no reducer needs to know panes exist.
pub fn sync(app: &mut App) {
    let Some(project) = project(app) else {
        return;
    };
    let displayed = app.displayed_terminal();
    let live: Vec<TerminalRef> = app
        .pane_layouts
        .get(&project)
        .map(|l| l.terminals())
        .unwrap_or_default()
        .into_iter()
        .filter(|t| is_live(app, *t))
        .collect();
    let seen = app.pane_synced_displayed;
    app.pane_synced_displayed = displayed;
    let layout = app
        .pane_layouts
        .entry(project.clone())
        .or_insert_with(PaneLayout::single);
    layout.prune(|t| live.contains(&t));
    // Only a *change* of the displayed terminal moves a pane: focusing an empty pane leaves the
    // selection where it was and must not fill the pane with it.
    if let Some(t) = displayed {
        if seen != displayed && layout.focused_terminal() != Some(t) {
            layout.show_or_focus(t);
        }
    }
    let now = layout.terminals();
    let multi = now.len() > 1;
    let previous = match &app.viewed_terminals_sent {
        Some((p, set)) if *p == project => set.clone(),
        _ => Vec::new(),
    };
    let dirty = std::mem::take(&mut app.viewed_dirty);
    let changed = now != previous && (multi || previous.len() > 1);
    if changed || (dirty && multi) {
        if let Some(d) = &app.daemon {
            d.send(ClientMsg::SetViewedTerminals {
                project: project.clone(),
                terminals: now.clone(),
            });
        }
        app.viewed_terminals_sent = Some((project, now));
    }
    send_pane_sizes(app);
}

/// The smallest pane, in characters: what `PaneLayout::split` measures a pane against.
const MIN: (f32, f32) = (MIN_PANE_COLS as f32, MIN_PANE_ROWS as f32);

/// Send each pane's terminal the size of the pane it sits in, when that differs from the last size
/// sent (FR-014). One pane: nothing is sent here, the single-pane path owns its size. A terminal no
/// pane shows is not touched, so it keeps its last size until it is shown again.
fn send_pane_sizes(app: &mut App) {
    let Some(layout) = layout(app) else {
        return;
    };
    if layout.len() < 2 {
        return;
    }
    let due: Vec<(TerminalRef, (u16, u16))> = layout
        .panes()
        .into_iter()
        .filter_map(|p| {
            let size = *app.pane_sizes.get(&(project(app)?, p.id()))?;
            Some((p.terminal()?, size))
        })
        .filter(|(t, size)| app.pane_sent.get(t) != Some(size))
        .collect();
    for (t, (cols, rows)) in due {
        if let Some(d) = &app.daemon {
            d.send(ClientMsg::SessionResize {
                session: t.session,
                process: Some(t.process),
                cols,
                rows,
            });
        }
        app.pane_sent.insert(t, (cols, rows));
    }
}

/// Apply a pane message (FR-001, FR-004, FR-010, FR-014).
pub fn on_pane_msg(app: &mut App, msg: PaneMsg) -> Task<Message> {
    app.pane_refusal = None;
    match msg {
        PaneMsg::Split(pane, axis) => {
            // A pane never measured on its own (the lone pane of an unsplit project) is as big as
            // the last area measured, and unknown only before any frame.
            let size = project(app)
                .and_then(|p| app.pane_sizes.get(&(p, pane)).copied())
                .or(app.last_grid)
                .map_or((f32::MAX, f32::MAX), |(c, r)| (f32::from(c), f32::from(r)));
            if let Err(refusal) = split_pane(app, pane, axis, size, MIN) {
                app.pane_refusal = Some(refusal.reason());
            }
        }
        PaneMsg::Chord(action) => {
            let Some(focused) = layout(app).map(PaneLayout::focused) else {
                return Task::none();
            };
            match action {
                PaneAction::SplitVertical => {
                    return on_pane_msg(app, PaneMsg::Split(focused, Axis::Vertical))
                }
                PaneAction::SplitHorizontal => {
                    return on_pane_msg(app, PaneMsg::Split(focused, Axis::Horizontal))
                }
                PaneAction::Focus(dir) => {
                    if project(app)
                        .and_then(|p| app.pane_layouts.get_mut(&p))
                        .is_some_and(|l| l.focus_dir(dir))
                    {
                        follow_focus(app);
                    }
                }
            }
        }
        PaneMsg::Show(pane, terminal) => {
            let shown = project(app)
                .and_then(|p| app.pane_layouts.get_mut(&p).map(|l| l.show(pane, terminal)));
            match shown {
                Some(Ok(())) => follow_focus(app),
                Some(Err(refusal)) => app.pane_refusal = Some(refusal.reason()),
                None => {}
            }
        }
        PaneMsg::FocusPane(id) => focus_pane(app, id),
        PaneMsg::Resized { pane, cols, rows } => {
            if let Some(p) = project(app) {
                app.pane_sizes.insert((p, pane), (cols, rows));
            }
            if layout(app).is_some_and(|l| l.focused() == pane) {
                // The next session starts at the focused pane's size.
                app.last_grid = Some((cols, rows));
            }
            send_pane_sizes(app);
        }
    }
    Task::none()
}

/// A terminal of the active project no pane shows yet: what a new pane opens on (FR-004).
fn unshown_terminal(app: &App) -> Option<TerminalRef> {
    let project = project(app)?;
    let shown = app.pane_layouts.get(&project)?.terminals();
    let sessions = app.core.workspace.sessions.get(&project)?;
    sessions
        .iter()
        .flat_map(|s| {
            std::iter::once(TerminalRef {
                session: s.id,
                process: SessionProcess::Primary,
            })
            .chain(s.shells.iter().map(|i| TerminalRef {
                session: s.id,
                process: SessionProcess::Shell(i.id),
            }))
        })
        .find(|t| !shown.contains(t))
}

/// Split `pane` along `axis` (FR-001, FR-004): the new pane takes focus and opens on a terminal
/// no pane shows, else stays empty. `pane_size` and `min` are in the same units (pixels).
pub fn split_pane(
    app: &mut App,
    pane: PaneId,
    axis: Axis,
    pane_size: (f32, f32),
    min: (f32, f32),
) -> Result<(), Refusal> {
    let Some(project) = project(app) else {
        return Err(Refusal::UnknownPane);
    };
    let candidate = unshown_terminal(app);
    let layout = app
        .pane_layouts
        .entry(project)
        .or_insert_with(PaneLayout::single);
    layout.split(pane, axis, pane_size, min, candidate)?;
    follow_focus(app);
    Ok(())
}

/// Focus pane `id`: the keyboard, the selection and `core.session.active` follow its terminal
/// (FR-010). An empty pane takes the focus and leaves the selection where it was.
pub fn focus_pane(app: &mut App, id: PaneId) {
    let Some(project) = project(app) else {
        return;
    };
    let Some(layout) = app.pane_layouts.get_mut(&project) else {
        return;
    };
    if layout.focused() == id || !layout.focus(id) {
        return;
    }
    follow_focus(app);
}

/// Make the selection follow the focused pane's terminal.
fn follow_focus(app: &mut App) {
    let target = layout(app).and_then(PaneLayout::focused_terminal);
    app.selection = None;
    app.display_offset = 0;
    if let Some(t) = target {
        app.core
            .update(Message::Session(SessionMsg::Selected(t.session)));
        let current = app
            .core
            .workspace
            .find_session(t.session)
            .map(|(_, s)| crate::shell::daemon_sync::session_process(s));
        if current != Some(t.process) {
            let msg = match t.process {
                SessionProcess::Shell(s) => SessionMsg::ShellInstanceSelected(t.session, s),
                SessionProcess::Primary => SessionMsg::TerminalAiCliSelected(t.session),
            };
            app.core.update(Message::Session(msg));
        }
    }
    app.pane_synced_displayed = app.displayed_terminal();
}
