//! The user-group layout's store side (feature 1786, research R9): the active project's layout is
//! read from the store the first time the project is shown, and a layout a group operation
//! changed is written back after the message that changed it.
//!
//! Both run in [`after_update`], which `update` calls after every message: the project can become
//! active by many routes (a switch, a reopen, the start-up restore), and a group operation reaches
//! the root as an outcome, so this is the one place that sees all of them.

use crate::App;

/// Load the active project's layout when none is held, then save every changed one. A failed save
/// keeps the layout in memory and says so in the usual status message (R9).
pub(crate) fn after_update(app: &mut App) {
    let Some(store) = app.caps.projects() else {
        return;
    };
    if let Some(project) = app.core.workspace.active.clone() {
        if !app.core.sidebar.layouts.contains_key(&project) {
            let layout = store.load_layout(&project);
            app.core.sidebar_layout_loaded(project, layout);
        }
    }
    let unsaved = std::mem::take(&mut app.core.sidebar.layouts_unsaved);
    for project in unsaved {
        let Some(layout) = app.core.sidebar.layouts.get(&project) else {
            continue;
        };
        if let Err(err) = store.save_layout(&project, layout) {
            app.core
                .notify_error(format!("Couldn't save the sidebar groups: {err}"));
        }
    }
}
