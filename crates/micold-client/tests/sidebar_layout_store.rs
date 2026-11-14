//! A project's sidebar layout is kept in `layouts/<project id>.json` beside `runs/`, survives a
//! restart, and a missing or corrupt file loads as no groups (feature 1786, contract C7, FR-012,
//! FR-018, SC-003). Modelled on the runs store: atomic write, unreadable file kept aside.

use std::path::Path;

use micold_core::sidebar_layout::{LayoutItem, SidebarLayout};
use micold_core::store::{JsonFileStore, ProjectStore};

const PROJECT: &str = "/projects/alpha";

fn store(dir: &Path) -> JsonFileStore {
    JsonFileStore::at(dir.join("state.json"))
}

fn layout() -> SidebarLayout {
    let mut layout = SidebarLayout::default();
    let id = layout.create("Work").expect("a fresh name");
    layout.groups[0].collapsed = true;
    layout.groups[0]
        .items
        .push(LayoutItem::Worktree("feat-a".into()));
    layout.ungrouped.push(LayoutItem::Worktree("fix-b".into()));
    assert_eq!(layout.groups[0].id, id);
    layout
}

#[test]
fn a_saved_layout_loads_back_after_a_restart() {
    let dir = tempfile::tempdir().expect("temp dir");
    store(dir.path())
        .save_layout(Path::new(PROJECT), &layout())
        .expect("the write succeeds");
    let reopened = store(dir.path());
    assert_eq!(
        ProjectStore::load_layout(&reopened, Path::new(PROJECT)),
        layout(),
        "groups, members, order and collapsed state come back (SC-003)"
    );
    let path = reopened.layouts_path(Path::new(PROJECT));
    assert_eq!(
        path.parent().and_then(|p| p.file_name()),
        Some("layouts".as_ref()),
        "the file sits in layouts/ beside runs/"
    );
}

#[test]
fn a_missing_file_loads_as_no_groups() {
    let dir = tempfile::tempdir().expect("temp dir");
    assert_eq!(
        store(dir.path()).load_layout(Path::new(PROJECT)),
        SidebarLayout::default(),
        "a project never grouped has no groups"
    );
}

#[test]
fn a_corrupt_file_is_kept_aside_and_loads_as_no_groups() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = store(dir.path());
    let path = store.layouts_path(Path::new(PROJECT));
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
    std::fs::write(&path, "{ not json").expect("write");
    assert_eq!(
        store.load_layout(Path::new(PROJECT)),
        SidebarLayout::default(),
        "an unreadable layout loads without error as no groups (FR-018)"
    );
    let mut aside = path.clone().into_os_string();
    aside.push(".corrupt");
    assert_eq!(
        std::fs::read_to_string(&aside).expect("the corrupt file is kept for inspection"),
        "{ not json",
        "its content is kept as it was"
    );
}

#[test]
fn the_write_replaces_the_file_whole_and_leaves_no_staging_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = store(dir.path());
    store
        .save_layout(Path::new(PROJECT), &SidebarLayout::default())
        .expect("first write");
    store
        .save_layout(Path::new(PROJECT), &layout())
        .expect("second write");
    let path = store.layouts_path(Path::new(PROJECT));
    let names: Vec<_> = std::fs::read_dir(path.parent().expect("a parent"))
        .expect("read dir")
        .map(|e| e.expect("entry").file_name())
        .collect();
    assert_eq!(names.len(), 1, "only the layout file is left: {names:?}");
    assert_eq!(
        SidebarLayout::try_from_json(&std::fs::read_to_string(&path).expect("read")),
        Ok(layout()),
        "the file holds the last write whole"
    );
}
