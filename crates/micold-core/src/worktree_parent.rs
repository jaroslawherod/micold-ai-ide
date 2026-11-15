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
const DEFAULT_NAME: &str = crate::mcp::tools::WorktreeRef::DEFAULT;

/// A parent as the creator names it: the project root, or a worktree by directory name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParentChoice {
    Default,
    Worktree(String),
}

/// A link resolved for display: the parent's current name (its last one when removed) and state.
/// `root` is a link to the project root, so a live worktree directory named `default` is told
/// apart from it by the link, never by the name (spec 1945).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentView {
    pub name: String,
    pub state: ParentState,
    pub root: bool,
}

impl ParentView {
    /// The label the sidebar shows (FR-010): `display` names a live or removed parent by its
    /// directory name, and answers the parent's display name (a rename override), or itself.
    pub fn label(&self, display: impl Fn(&str) -> String) -> crate::worktree::ParentLabel {
        crate::worktree::ParentLabel {
            name: if self.root {
                self.name.clone()
            } else {
                display(&self.name)
            },
            removed: self.state == ParentState::Removed,
        }
    }
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
                    root: true,
                },
                ParentRef::Worktree(parent) => match names.get(&parent) {
                    Some(name) if !link.removed => ParentView {
                        name: (*name).clone(),
                        state: ParentState::Known,
                        root: false,
                    },
                    _ => ParentView {
                        name: link.last_name.clone(),
                        state: ParentState::Removed,
                        root: false,
                    },
                },
            };
            Some((dir.clone(), view))
        })
        .collect()
}

/// A caller's parent, as it sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParentSide {
    /// A live parent; `recorded == false` is the implicit `default` of an unparented worktree.
    /// `root` is the project root, else a worktree named `name` (which may be `"default"`).
    Known {
        name: String,
        recorded: bool,
        root: bool,
    },
    /// The parent was removed: no sessions, no siblings.
    Removed { last_name: String },
}

/// A direct child; `implicit` only for an unparented child of `default`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Child {
    pub name: String,
    pub implicit: bool,
}

/// Where a worktree stands (spec 1945): its parent, siblings and direct children, by directory
/// name (`default` is `"default"`), sorted by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Neighbourhood {
    pub parent: Option<ParentSide>,
    pub siblings: Vec<String>,
    pub children: Vec<Child>,
}

/// The neighbourhood of `own` (`None` is `default`) among the `live` worktrees, whose recorded
/// links are `views`. A worktree with no recorded link is an implicit child of `default`, for
/// visibility only. An `own` that is not live has an empty neighbourhood.
pub fn neighbourhood(
    live: &BTreeSet<String>,
    views: &BTreeMap<String, ParentView>,
    own: Option<&str>,
) -> Neighbourhood {
    // The live parent of `dir` (`None` is the project root) and whether it was recorded: the
    // recorded one, else the root. `None` when it was removed.
    let parent_of = |dir: &str| match views.get(dir) {
        None => Some((None, false)),
        Some(v) if v.state == ParentState::Known => {
            Some(((!v.root).then_some(v.name.as_str()), true))
        }
        Some(_) => None,
    };
    // A live directory named `default` is not the project root: it has no parent and no siblings,
    // and is nobody's child; a worktree recorded under it is its child.
    let children_of = |parent: Option<&str>| {
        live.iter()
            .filter(|d| d.as_str() != DEFAULT_NAME)
            .filter(|d| parent_of(d).is_some_and(|(p, _)| p == parent))
            .map(|d| Child {
                name: d.clone(),
                implicit: parent.is_none() && parent_of(d).is_some_and(|(_, rec)| !rec),
            })
            .collect()
    };
    let Some(own) = own else {
        return Neighbourhood {
            parent: None,
            siblings: Vec::new(),
            children: children_of(None),
        };
    };
    if !live.contains(own) {
        return Neighbourhood::default();
    }
    if own == DEFAULT_NAME {
        return Neighbourhood {
            children: children_of(Some(own)),
            ..Neighbourhood::default()
        };
    }
    let (parent, siblings) = match parent_of(own) {
        Some((dir, recorded)) => (
            ParentSide::Known {
                name: dir.unwrap_or(DEFAULT_NAME).to_string(),
                recorded,
                root: dir.is_none(),
            },
            live.iter()
                .filter(|d| {
                    d.as_str() != own
                        && d.as_str() != DEFAULT_NAME
                        && parent_of(d).is_some_and(|(p, _)| p == dir)
                })
                .cloned()
                .collect(),
        ),
        None => (
            ParentSide::Removed {
                last_name: views[own].name.clone(),
            },
            Vec::new(),
        ),
    };
    Neighbourhood {
        parent: Some(parent),
        siblings,
        children: children_of(Some(own)),
    }
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

    fn view(name: &str, state: ParentState) -> ParentView {
        ParentView {
            name: name.into(),
            state,
            root: false,
        }
    }

    /// A link to the project root.
    fn root() -> ParentView {
        ParentView {
            root: true,
            ..known("default")
        }
    }

    fn known(name: &str) -> ParentView {
        view(name, ParentState::Known)
    }

    fn views_of(pairs: &[(&str, ParentView)]) -> BTreeMap<String, ParentView> {
        pairs
            .iter()
            .map(|(d, v)| (d.to_string(), v.clone()))
            .collect()
    }

    fn child(name: &str, implicit: bool) -> Child {
        Child {
            name: name.into(),
            implicit,
        }
    }

    #[test]
    fn an_unparented_worktree_has_the_implicit_default_parent_and_unparented_siblings() {
        let n = neighbourhood(
            &live(&["a", "b", "c"]),
            &views_of(&[("c", known("a"))]),
            Some("a"),
        );
        assert_eq!(
            n.parent,
            Some(ParentSide::Known {
                name: "default".into(),
                recorded: false,
                root: true
            })
        );
        assert_eq!(n.siblings, ["b"]);
        assert_eq!(n.children, [child("c", false)]);
    }

    #[test]
    fn siblings_are_mutual_between_a_recorded_and_an_implicit_default_parent() {
        let v = views_of(&[("d", root())]);
        let l = live(&["a", "b", "d"]);
        let from_d = neighbourhood(&l, &v, Some("d"));
        assert_eq!(
            from_d.parent,
            Some(ParentSide::Known {
                name: "default".into(),
                recorded: true,
                root: true
            })
        );
        assert_eq!(from_d.siblings, ["a", "b"]);
        assert!(from_d.children.is_empty());
        assert_eq!(neighbourhood(&l, &v, Some("a")).siblings, ["b", "d"]);
    }

    #[test]
    fn a_removed_parent_leaves_no_siblings() {
        let v = views_of(&[
            ("c", view("gone", ParentState::Removed)),
            ("e", view("gone", ParentState::Removed)),
        ]);
        let n = neighbourhood(&live(&["c", "e"]), &v, Some("c"));
        assert_eq!(
            n.parent,
            Some(ParentSide::Removed {
                last_name: "gone".into()
            })
        );
        assert!(n.siblings.is_empty() && n.children.is_empty());
    }

    #[test]
    fn default_has_recorded_and_implicit_children_but_not_grandchildren_or_removed_ones() {
        let v = views_of(&[
            ("d", root()),
            ("c", known("a")),
            ("r", view("gone", ParentState::Removed)),
        ]);
        let n = neighbourhood(&live(&["a", "c", "d", "r"]), &v, None);
        assert_eq!(n.parent, None);
        assert!(n.siblings.is_empty());
        assert_eq!(n.children, [child("a", true), child("d", false)]);
    }

    #[test]
    fn a_worktree_that_is_not_live_has_an_empty_neighbourhood() {
        let n = neighbourhood(&live(&["a"]), &BTreeMap::new(), Some("ghost"));
        assert_eq!(n, Neighbourhood::default());
    }

    #[test]
    fn a_live_directory_named_default_is_not_the_project_root() {
        let l = live(&["a", "default"]);
        let own = neighbourhood(&l, &BTreeMap::new(), Some("default"));
        assert_eq!(own, Neighbourhood::default());
        let a = neighbourhood(&l, &BTreeMap::new(), Some("a"));
        assert!(a.siblings.is_empty(), "{a:?}");
        let root = neighbourhood(&l, &BTreeMap::new(), None);
        assert_eq!(root.children, [child("a", true)]);
    }

    #[test]
    fn a_worktree_recorded_under_a_directory_named_default_is_its_child_not_the_roots() {
        let l = live(&["a", "default", "k"]);
        let v = views_of(&[("k", known("default"))]);
        let k = neighbourhood(&l, &v, Some("k"));
        assert_eq!(
            k.parent,
            Some(ParentSide::Known {
                name: "default".into(),
                recorded: true,
                root: false
            })
        );
        assert!(
            k.siblings.is_empty(),
            "the root's children are not siblings: {k:?}"
        );
        let dir = neighbourhood(&l, &v, Some("default"));
        assert_eq!(dir.parent, None);
        assert_eq!(dir.children, [child("k", false)]);
        let root = neighbourhood(&l, &v, None);
        assert_eq!(root.children, [child("a", true)]);
        assert!(neighbourhood(&l, &v, Some("a")).siblings.is_empty());
    }
}
