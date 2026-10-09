//! Terminal-area panes (feature 484): keeping each project's [`PaneLayout`] in step with what the
//! session reducers select, and telling the daemon which terminals to stream.
//!
//! The decisions live in `micold_core::pane_layout`; this module is the glue that applies them to
//! `App`. A project that was never split has one pane showing the displayed terminal, which is
//! exactly today's behaviour (FR-017): no `SetViewedTerminals` goes out for it.

use std::path::PathBuf;

use micold_core::pane_layout::{Axis, PaneId, PaneLayout, Refusal};
use micold_core::protocol::messages::{ClientMsg, SessionProcess, TerminalRef};

use crate::App;
use micold_client::app::Message;
use micold_client::features::session::Msg as SessionMsg;

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
}

/// A terminal of the active project no pane shows yet: what a new pane opens on (FR-004).
#[allow(dead_code)] // Reached from the pane header and focus handling in T013 / T016.
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
#[allow(dead_code)] // Reached from the pane header and focus handling in T013 / T016.
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
#[allow(dead_code)] // Reached from the pane header and focus handling in T013 / T016.
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
#[allow(dead_code)] // Reached from the pane header and focus handling in T013 / T016.
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
