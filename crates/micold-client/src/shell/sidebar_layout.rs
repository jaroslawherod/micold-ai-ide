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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::base_app;
    use micold_core::sidebar_layout::SidebarLayout;
    use micold_core::store::{FakeProjectStore, ProjectStore};
    use std::path::PathBuf;
    use std::sync::Arc;

    fn project() -> PathBuf {
        PathBuf::from("/repo/groups")
    }

    fn layout_with(name: &str) -> SidebarLayout {
        let mut layout = SidebarLayout::default();
        layout.create(name).expect("a valid name");
        layout
    }

    fn app_over(store: &Arc<FakeProjectStore>) -> crate::App {
        let mut app = base_app();
        app.caps = std::mem::replace(
            &mut app.caps,
            crate::Capabilities::real().without_projects(),
        )
        .with_projects(store.clone());
        app.core.workspace.active = Some(project());
        app
    }

    #[test]
    fn the_active_projects_stored_layout_is_loaded_once_and_not_marked_unsaved() {
        let store = Arc::new(FakeProjectStore::new().with_layout(project(), layout_with("Work")));
        let mut app = app_over(&store);
        after_update(&mut app);
        assert_eq!(
            app.core.sidebar.layouts.get(&project()),
            Some(&layout_with("Work"))
        );
        assert!(app.core.sidebar.layouts_unsaved.is_empty());
        // Held now: a store change is not read again.
        store
            .save_layout(&project(), &layout_with("Other"))
            .unwrap();
        after_update(&mut app);
        assert_eq!(
            app.core.sidebar.layouts.get(&project()),
            Some(&layout_with("Work"))
        );
    }

    #[test]
    fn a_changed_layout_is_written_and_the_unsaved_set_emptied() {
        let store = Arc::new(FakeProjectStore::new());
        let mut app = app_over(&store);
        app.core
            .sidebar_layout_changed(project(), layout_with("Later"));
        after_update(&mut app);
        assert_eq!(store.layout_of(&project()), Some(layout_with("Later")));
        assert!(app.core.sidebar.layouts_unsaved.is_empty());
    }

    #[test]
    fn a_failed_save_keeps_the_layout_in_memory_and_says_so() {
        let store = Arc::new(FakeProjectStore::new().failing_save(std::io::ErrorKind::Other));
        let mut app = app_over(&store);
        app.core
            .sidebar_layout_changed(project(), layout_with("Later"));
        after_update(&mut app);
        assert_eq!(
            app.core.sidebar.layouts.get(&project()),
            Some(&layout_with("Later"))
        );
        assert_eq!(store.layout_of(&project()), None);
        let said = app.core.notifications.queue.visible().cloned();
        assert!(
            said.is_some_and(|n| n.message.contains("Couldn't save the sidebar groups")),
            "the failure is raised"
        );
    }
}
