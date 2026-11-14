//! Worktree ids and parent links (spec 1857): the pure rules over one project's two maps, so the
//! daemon's catalog only decides *when* to apply them and what is live.
//!
//! A link stores ids, never names. A worktree that is deleted is *retired*: its id and its own link
//! go, and every link that pointed at it is marked removed (a tombstone). A later worktree with the
//! same directory name gets a fresh id, so it adopts nothing (FR-007).

use crate::worktree::{ParentLink, ParentRef, ParentState, WorktreeId};
use std::collections::{BTreeMap, BTreeSet};

/// `dir_name` to id, one project.
pub type Ids = BTreeMap<String, WorktreeId>;
/// Child id to its link, one project.
pub type Links = BTreeMap<WorktreeId, ParentLink>;

/// The name `default` has as a parent, in ref form.
const DEFAULT_NAME: &str = "default";

/// A parent as the creator names it: the project root, or a worktree by directory name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParentChoice {
    Default,
    Worktree(String),
}

/// A link resolved for display: the parent's current name (its last one when removed) and state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentView {
    pub name: String,
    pub state: ParentState,
}

fn retire(ids: &mut Ids, links: &mut Links, dir: &str) {
    let Some(id) = ids.remove(dir) else { return };
    links.remove(&id);
    for link in links.values_mut() {
        if link.parent == ParentRef::Worktree(id) {
            link.removed = true;
        }
    }
}

/// Bring the maps in line with `live`, the directory names the caller holds to exist: mint an id
/// for each live worktree without one, retire the ids of worktrees no longer live, keep each live
/// parent's `last_name` current. Returns whether anything changed.
pub fn reconcile(ids: &mut Ids, links: &mut Links, live: &BTreeSet<String>) -> bool {
    let mut changed = false;
    for dir in live {
        if !ids.contains_key(dir) {
            ids.insert(dir.clone(), WorktreeId::mint());
            changed = true;
        }
    }
    let dead: Vec<String> = ids.keys().filter(|d| !live.contains(*d)).cloned().collect();
    changed |= !dead.is_empty();
    for dir in dead {
        retire(ids, links, &dir);
    }
    let names: BTreeMap<WorktreeId, &String> = ids.iter().map(|(d, id)| (*id, d)).collect();
    let before = links.len();
    links.retain(|child, _| names.contains_key(child));
    changed |= links.len() != before;
    for link in links.values_mut() {
        let ParentRef::Worktree(parent) = link.parent else {
            continue;
        };
        match names.get(&parent) {
            Some(name) if !link.removed && link.last_name != **name => {
                link.last_name = (*name).clone();
                changed = true;
            }
            None if !link.removed => {
                link.removed = true;
                changed = true;
            }
            _ => {}
        }
    }
    changed
}

/// Record a newly created worktree `dir`: always a fresh id (retiring a stale one of the same
/// name), and a link when `parent` is given. A parent without an id yet is minted one.
pub fn record(ids: &mut Ids, links: &mut Links, dir: &str, parent: Option<&ParentChoice>) {
    retire(ids, links, dir);
    let child = WorktreeId::mint();
    ids.insert(dir.to_string(), child);
    let link = match parent {
        None => return,
        Some(ParentChoice::Default) => ParentLink {
            parent: ParentRef::Default,
            removed: false,
            last_name: DEFAULT_NAME.into(),
        },
        Some(ParentChoice::Worktree(name)) if name == dir => return,
        Some(ParentChoice::Worktree(name)) => {
            let id = *ids.entry(name.clone()).or_insert_with(WorktreeId::mint);
            ParentLink {
                parent: ParentRef::Worktree(id),
                removed: false,
                last_name: name.clone(),
            }
        }
    };
    links.insert(child, link);
}

/// The worktree `dir` was deleted: its children become removed, its own entries go.
pub fn forget(ids: &mut Ids, links: &mut Links, dir: &str) {
    retire(ids, links, dir);
}

/// Each linked worktree's parent, by the child's directory name.
pub fn views(ids: &Ids, links: &Links) -> BTreeMap<String, ParentView> {
    let names: BTreeMap<WorktreeId, &String> = ids.iter().map(|(d, id)| (*id, d)).collect();
    ids.iter()
        .filter_map(|(dir, id)| {
            let link = links.get(id)?;
            let view = match link.parent {
                ParentRef::Default => ParentView {
                    name: DEFAULT_NAME.into(),
                    state: ParentState::Known,
                },
                ParentRef::Worktree(parent) => match names.get(&parent) {
                    Some(name) if !link.removed => ParentView {
                        name: (*name).clone(),
                        state: ParentState::Known,
                    },
                    _ => ParentView {
                        name: link.last_name.clone(),
                        state: ParentState::Removed,
                    },
                },
            };
            Some((dir.clone(), view))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live(dirs: &[&str]) -> BTreeSet<String> {
        dirs.iter().map(|d| d.to_string()).collect()
    }

    fn name_of(views: &BTreeMap<String, ParentView>, dir: &str) -> (String, ParentState) {
        let v = &views[dir];
        (v.name.clone(), v.state)
    }

    #[test]
    fn reconcile_mints_once_and_keeps_ids() {
        let (mut ids, mut links) = (Ids::new(), Links::new());
        assert!(reconcile(&mut ids, &mut links, &live(&["a", "b"])));
        let first = ids.clone();
        assert_eq!(first.len(), 2);
        assert!(!reconcile(&mut ids, &mut links, &live(&["a", "b"])));
        assert_eq!(ids, first);
    }

    #[test]
    fn a_vanished_parent_tombstones_its_children_and_a_same_name_worktree_adopts_none() {
        let (mut ids, mut links) = (Ids::new(), Links::new());
        reconcile(&mut ids, &mut links, &live(&["w"]));
        record(
            &mut ids,
            &mut links,
            "c",
            Some(&ParentChoice::Worktree("w".into())),
        );
        assert_eq!(name_of(&views(&ids, &links), "c").1, ParentState::Known);

        assert!(reconcile(&mut ids, &mut links, &live(&["c"])));
        assert_eq!(
            name_of(&views(&ids, &links), "c"),
            ("w".to_string(), ParentState::Removed)
        );
        reconcile(&mut ids, &mut links, &live(&["c", "w"]));
        assert_eq!(
            name_of(&views(&ids, &links), "c"),
            ("w".to_string(), ParentState::Removed),
            "a new worktree named like the old parent is not its parent"
        );
    }

    #[test]
    fn default_links_are_never_removed() {
        let (mut ids, mut links) = (Ids::new(), Links::new());
        record(&mut ids, &mut links, "c", Some(&ParentChoice::Default));
        reconcile(&mut ids, &mut links, &live(&["c", "x"]));
        reconcile(&mut ids, &mut links, &live(&["c"]));
        assert_eq!(
            name_of(&views(&ids, &links), "c"),
            ("default".to_string(), ParentState::Known)
        );
    }

    #[test]
    fn forgetting_a_child_drops_its_entries_and_a_parent_tombstones() {
        let (mut ids, mut links) = (Ids::new(), Links::new());
        reconcile(&mut ids, &mut links, &live(&["w"]));
        record(
            &mut ids,
            &mut links,
            "c",
            Some(&ParentChoice::Worktree("w".into())),
        );
        forget(&mut ids, &mut links, "c");
        assert!(!ids.contains_key("c") && links.is_empty());
        record(
            &mut ids,
            &mut links,
            "d",
            Some(&ParentChoice::Worktree("w".into())),
        );
        forget(&mut ids, &mut links, "w");
        assert_eq!(name_of(&views(&ids, &links), "d").1, ParentState::Removed);
        let child = ids["d"];
        assert!(
            links[&child].removed,
            "the persisted link is flagged, not only reported through the missing id"
        );
    }

    #[test]
    fn recording_over_a_stale_id_replaces_it_and_tombstones_its_children() {
        let (mut ids, mut links) = (Ids::new(), Links::new());
        reconcile(&mut ids, &mut links, &live(&["w"]));
        record(
            &mut ids,
            &mut links,
            "c",
            Some(&ParentChoice::Worktree("w".into())),
        );
        let stale = ids["w"];
        record(&mut ids, &mut links, "w", None);
        assert_ne!(ids["w"], stale);
        assert_eq!(name_of(&views(&ids, &links), "c").1, ParentState::Removed);
    }
}
