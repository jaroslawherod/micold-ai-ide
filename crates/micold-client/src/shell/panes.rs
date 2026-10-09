//! Terminal-area panes (feature 484): keeping each project's [`PaneLayout`] in step with what the
//! session reducers select, and telling the daemon which terminals to stream.
//!
//! The decisions live in `micold_core::pane_layout`; this module is the glue that applies them to
//! `App`. A project that was never split has one pane showing the displayed terminal, which is
//! exactly today's behaviour (FR-017): no `SetViewedTerminals` goes out for it.

use std::path::PathBuf;

use iced::Task;
use micold_client::keymap::PaneAction;
use micold_core::pane_layout::{
    Axis, PaneId, PaneLayout, Refusal, SplitEvent, MIN_PANE_COLS, MIN_PANE_ROWS,
};
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
    let switched = app.pane_synced_project.as_ref() != Some(&project);
    app.pane_synced_project = Some(project.clone());
    let layout = app
        .pane_layouts
        .entry(project.clone())
        .or_insert_with(PaneLayout::single);
    layout.prune(|t| live.contains(&t));
    // Only a *change* of the displayed terminal moves a pane: focusing an empty pane leaves the
    // selection where it was and must not fill the pane with it.
    if let Some(t) = displayed {
        // A focus resting on an empty pane of a split project stays there across a project switch
        // (FR-016): the other project's selection is not a request to move it.
        let keeps_empty_focus = switched && layout.len() > 1 && layout.focused_terminal().is_none();
        if seen != displayed && layout.focused_terminal() != Some(t) && !keeps_empty_focus {
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
    save(app);
}

/// Adopt the layouts a catalog snapshot carries (feature 484, FR-013): once per project per run,
/// the first time a snapshot names it. A layout the snapshot lacks leaves the project on the single
/// default pane. The active project's selection then follows the restored focused pane.
pub fn restore(app: &mut App, catalog: &micold_core::protocol::messages::CatalogSnapshot) {
    let active = project(app);
    for p in &catalog.projects {
        if !app.pane_restored.insert(p.path.clone()) {
            continue;
        }
        let Some(layout) = p.pane_layout.as_deref().and_then(PaneLayout::from_json) else {
            continue;
        };
        app.pane_saved.insert(p.path.clone(), layout.to_json());
        app.pane_layouts.insert(p.path.clone(), layout);
        if active.as_ref() == Some(&p.path) {
            // Terminals that no longer exist become empty panes (FR-014); the pane stays.
            let live: Vec<TerminalRef> = app
                .pane_layouts
                .get(&p.path)
                .map(|l| l.terminals())
                .unwrap_or_default()
                .into_iter()
                .filter(|t| is_live(app, *t))
                .collect();
            if let Some(l) = app.pane_layouts.get_mut(&p.path) {
                l.prune(|t| live.contains(&t));
                // Emptying a pane is not a change to store: the file keeps the terminal until the
                // user changes the layout, so a session that is merely slow to appear is not lost.
                app.pane_saved.insert(p.path.clone(), l.to_json());
            }
            follow_focus(app);
        }
    }
}

/// Tell the daemon the active project's layout when it differs from the one last restored or sent.
/// A project that was never split and never had a stored layout sends nothing (FR-017).
fn save(app: &mut App) {
    let Some(project) = project(app) else {
        return;
    };
    let Some(layout) = app.pane_layouts.get(&project) else {
        return;
    };
    let json = layout.to_json();
    match app.pane_saved.get(&project) {
        Some(saved) if *saved == json => return,
        None if layout.len() < 2 => return,
        _ => {}
    }
    let Some(d) = &app.daemon else {
        return;
    };
    d.send(ClientMsg::SetPaneLayout {
        project: project.clone(),
        layout: Some(json.clone()),
    });
    app.pane_saved.insert(project, json);
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
    if layout.len() < 2 || app.divider_dragging {
        // A divider drag resizes every frame: the sizes go out once, on release.
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
    // A measurement is not a user action: showing the reason relayouts the panes, and the
    // `Resized` that follows must not clear what made room for it.
    if !matches!(msg, PaneMsg::Resized { .. }) {
        app.pane_refusal = None;
    }
    if matches!(
        msg,
        PaneMsg::Close(_) | PaneMsg::Split(..) | PaneMsg::Chord(_) | PaneMsg::Show(..)
    ) {
        // A drag whose release never came (the pointer left the window) must not hold the sizes back.
        app.divider_dragging = false;
    }
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
                PaneAction::Close => return on_pane_msg(app, PaneMsg::Close(focused)),
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
        PaneMsg::Close(pane) => close_pane(app, pane),
        PaneMsg::Gesture(event) => on_split_event(app, event),
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

/// Close `pane` (FR-006, FR-007): only the layout changes. No stop, restart or detach goes to the
/// daemon; the terminal stays in the tab strip and the sidebar, and merely stops being streamed
/// when no pane shows it (`sync`).
fn close_pane(app: &mut App, pane: PaneId) {
    let Some(project) = project(app) else {
        return;
    };
    let Some(layout) = app.pane_layouts.get_mut(&project) else {
        return;
    };
    let was_focused = layout.focused() == pane;
    match layout.close(pane) {
        Ok(()) => {
            app.pane_sizes.remove(&(project, pane));
            if was_focused {
                follow_focus(app);
            }
        }
        Err(refusal) => app.pane_refusal = Some(refusal.reason()),
    }
}

/// Apply a gesture of the split view: divider drag, double press, header drop.
fn on_split_event(app: &mut App, event: SplitEvent) {
    let Some(project) = project(app) else {
        return;
    };
    let Some(layout) = app.pane_layouts.get_mut(&project) else {
        return;
    };
    match event {
        SplitEvent::Drag {
            index,
            basis_points,
        } => {
            // The view already kept the share within both minimums.
            layout.set_ratio(
                index,
                f32::from(basis_points) / 10_000.0,
                (1.0, 1.0),
                (0.0, 0.0),
            );
            app.divider_dragging = true;
        }
        SplitEvent::Release => {
            app.divider_dragging = false;
            send_pane_sizes(app);
        }
        SplitEvent::Reset(index) => {
            layout.reset_equal(index);
        }
        SplitEvent::Swap(a, b) => {
            if layout.swap(a, b).is_ok() {
                follow_focus(app);
            }
        }
    }
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
