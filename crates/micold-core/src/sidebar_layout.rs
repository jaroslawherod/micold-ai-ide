//! The sidebar's user groups (feature 1786, data-model.md): which groups a project has, in which
//! order, which worktrees and run groups each holds, and the order of the ungrouped rest.
//!
//! Render-free and pure: the client applies a [`SidebarLayout`] to its stable-order rows (feature
//! 1718) and calls the operations here; the store keeps one per project in
//! `layouts/<project id>.json` ([`crate::store::JsonFileStore::save_layout`]).
//!
//! Invariants (contract C1): an item is in exactly one list, either one group's `items` or
//! `ungrouped`; the project root (Default) is never an item, and [`LayoutItem`] has no variant for
//! it (FR-015); group names are trimmed, non-blank and unique case-insensitively (FR-004). No field
//! names a session or card state, so no state can move a row (FR-013).
//!
//! The run-group state of feature 483 is a different thing and lives in
//! [`crate::sidebar_groups`]; this module never reads it.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::runs::GroupId;

/// The only layout version this build writes and reads; any other loads as no groups (FR-018).
pub const LAYOUT_VERSION: u32 = 1;

/// A user group's identity: monotonic per layout and never reused, so a stale reference cannot
/// name a newer group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UserGroupId(pub u64);

/// One sidebar row the layout can place: a worktree by its directory name, or a run group, which
/// moves as one unit with its runs (FR-016). Default has no variant (FR-015).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayoutItem {
    /// A worktree row, by `dir_name` (research R2).
    Worktree(String),
    /// A run group's row (feature 483), by its id.
    Run(GroupId),
}

/// A group the user created (data-model.md § UserGroup).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserGroup {
    /// Never reused within a layout.
    pub id: UserGroupId,
    /// Trimmed, non-blank, unique case-insensitively (FR-004).
    pub name: String,
    /// Whether its rows are hidden under the header (FR-011).
    #[serde(default)]
    pub collapsed: bool,
    /// Its members, in display order.
    #[serde(default)]
    pub items: Vec<LayoutItem>,
}

/// Where a dragged item or group lands (contract C3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropTarget {
    /// Into a group's items at `index`.
    Group {
        /// The group.
        id: UserGroupId,
        /// Position among its items.
        index: usize,
    },
    /// Into the ungrouped list at `index`.
    Ungrouped {
        /// Position among the ungrouped items.
        index: usize,
    },
    /// Between group headers, for a dragged group: its new position among the groups.
    GroupSlot {
        /// Position among the groups.
        index: usize,
    },
}

/// Why an operation was refused; the layout is unchanged whenever one is returned (contract C2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutError {
    /// The name is empty or only whitespace.
    Blank,
    /// Another group already has this name, compared trimmed and case-insensitively.
    Duplicate,
    /// No group has this id.
    UnknownGroup,
    /// The item is in no list.
    UnknownItem,
    /// The project root was offered as an item.
    IsDefault,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LayoutError::Blank => "A group needs a name",
            LayoutError::Duplicate => "A group with this name already exists",
            LayoutError::UnknownGroup => "That group no longer exists",
            LayoutError::UnknownItem => "That row is no longer in the sidebar",
            LayoutError::IsDefault => "The project root cannot be put in a group",
        })
    }
}

impl std::error::Error for LayoutError {}

/// Why stored text is not a layout this build reads: the store keeps such a file aside as
/// `.corrupt` (FR-018), and [`SidebarLayout::from_json`] loads it as no groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutParseError(pub String);

/// One project's sidebar layout (data-model.md § SidebarLayout).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarLayout {
    /// [`LAYOUT_VERSION`] when written by this build.
    pub version: u32,
    /// The id the next created group gets; only ever grows, so ids are never reused.
    #[serde(default)]
    pub next_id: u64,
    /// The groups, in display order (FR-019).
    #[serde(default)]
    pub groups: Vec<UserGroup>,
    /// The ungrouped items, in their saved order.
    #[serde(default)]
    pub ungrouped: Vec<LayoutItem>,
}

impl Default for SidebarLayout {
    fn default() -> Self {
        Self {
            version: LAYOUT_VERSION,
            next_id: 0,
            groups: Vec::new(),
            ungrouped: Vec::new(),
        }
    }
}

impl SidebarLayout {
    /// The layout's JSON text.
    pub fn to_json(&self) -> String {
        let stored = SidebarLayout {
            version: LAYOUT_VERSION,
            ..self.clone()
        };
        serde_json::to_string_pretty(&stored).expect("a sidebar layout always serialises")
    }

    /// Parse stored text; an error for anything unparseable or of another version.
    pub fn try_from_json(text: &str) -> Result<Self, LayoutParseError> {
        let layout: SidebarLayout =
            serde_json::from_str(text).map_err(|err| LayoutParseError(err.to_string()))?;
        if layout.version != LAYOUT_VERSION {
            return Err(LayoutParseError(format!(
                "layout version {} is not {LAYOUT_VERSION}",
                layout.version
            )));
        }
        Ok(layout)
    }

    /// Parse stored text, never failing: anything [`Self::try_from_json`] refuses is no groups
    /// (FR-018).
    pub fn from_json(text: &str) -> Self {
        Self::try_from_json(text).unwrap_or_default()
    }

    /// Whether the layout holds no group and no saved order, so applying it changes nothing.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty() && self.ungrouped.is_empty()
    }

    /// Fit the layout to the rows the sidebar lists now, `live` in stable order (feature 1718):
    /// drop every item not in `live`, keep an item named twice only where it is first met (groups
    /// in order, then `ungrouped`), and append every live item the layout does not hold to
    /// `ungrouped`, in live order (contract C1, FR-005, FR-018).
    pub fn reconcile(&mut self, live: &[LayoutItem]) {
        let listed: HashSet<&LayoutItem> = live.iter().collect();
        let mut placed: HashSet<LayoutItem> = HashSet::new();
        let mut keep = |item: &LayoutItem| listed.contains(item) && placed.insert(item.clone());
        for group in &mut self.groups {
            group.items.retain(&mut keep);
        }
        self.ungrouped.retain(&mut keep);
        for item in live {
            if keep(item) {
                self.ungrouped.push(item.clone());
            }
        }
    }

    /// Whether some list holds `item`.
    pub fn holds(&self, item: &LayoutItem) -> bool {
        self.locate(item).is_some()
    }

    /// The list `item` is in (`None` for `ungrouped`), if any.
    pub fn list_of(&self, item: &LayoutItem) -> Option<Option<UserGroupId>> {
        self.locate(item).map(|(list, _)| list)
    }

    /// Put a live row the layout does not hold yet at the end of `ungrouped`, where [`reconcile`]
    /// would have put it, so [`Self::move_item`] can take it from there. Held items are left alone.
    ///
    /// [`reconcile`]: Self::reconcile
    pub fn adopt(&mut self, item: &LayoutItem) {
        if !self.holds(item) {
            self.ungrouped.push(item.clone());
        }
    }

    /// The group with `id`.
    pub fn group(&self, id: UserGroupId) -> Option<&UserGroup> {
        self.groups.iter().find(|group| group.id == id)
    }

    /// Create an empty group named `name`, trimmed, after the existing groups (FR-001, FR-004).
    pub fn create(&mut self, name: &str) -> Result<UserGroupId, LayoutError> {
        let name = self.valid_name(name, None)?;
        let next = self
            .groups
            .iter()
            .map(|group| group.id.0 + 1)
            .max()
            .unwrap_or(0)
            .max(self.next_id);
        let id = UserGroupId(next);
        self.next_id = next + 1;
        self.groups.push(UserGroup {
            id,
            name,
            collapsed: false,
            items: Vec::new(),
        });
        Ok(id)
    }

    /// Rename the group `id` to `name`, trimmed. Its own current name is not a duplicate (FR-002,
    /// FR-004).
    pub fn rename(&mut self, id: UserGroupId, name: &str) -> Result<(), LayoutError> {
        self.group(id).ok_or(LayoutError::UnknownGroup)?;
        let name = self.valid_name(name, Some(id))?;
        if let Some(group) = self.groups.iter_mut().find(|group| group.id == id) {
            group.name = name;
        }
        Ok(())
    }

    /// Delete the group `id`; its items return to the end of `ungrouped`, in their order, and none
    /// is dropped (FR-003, C1).
    pub fn delete(&mut self, id: UserGroupId) -> Result<(), LayoutError> {
        let at = self
            .groups
            .iter()
            .position(|group| group.id == id)
            .ok_or(LayoutError::UnknownGroup)?;
        let group = self.groups.remove(at);
        self.ungrouped.extend(group.items);
        Ok(())
    }

    /// Move `item` to `to` (contracts C1, C4): it leaves the list it is in before it enters the
    /// target, so it is in exactly one list afterwards (FR-005). `index` counts positions in the
    /// target list as it was before the move, so dropping an item below its own place in the same
    /// list lands it just after the row it was dropped under; an index past the end is clamped.
    /// A run group is one item and takes its runs along (FR-016).
    ///
    /// Refused, with the layout unchanged: the project root (`Worktree("")`, the wire's name for
    /// it) is [`LayoutError::IsDefault`]; an item in no list is [`LayoutError::UnknownItem`]; a
    /// group that is gone, or a [`DropTarget::GroupSlot`] (a place between headers, for a group),
    /// is [`LayoutError::UnknownGroup`].
    pub fn move_item(&mut self, item: &LayoutItem, to: DropTarget) -> Result<(), LayoutError> {
        if matches!(item, LayoutItem::Worktree(dir) if dir.is_empty()) {
            return Err(LayoutError::IsDefault);
        }
        let (target, index) = match to {
            DropTarget::Group { id, index } => {
                self.group(id).ok_or(LayoutError::UnknownGroup)?;
                (Some(id), index)
            }
            DropTarget::Ungrouped { index } => (None, index),
            DropTarget::GroupSlot { .. } => return Err(LayoutError::UnknownGroup),
        };
        let (from, at) = self.locate(item).ok_or(LayoutError::UnknownItem)?;
        self.list_mut(from).remove(at);
        let list = self.list_mut(target);
        let index = if from == target && at < index {
            index - 1
        } else {
            index
        };
        list.insert(index.min(list.len()), item.clone());
        Ok(())
    }

    /// The list `item` is in (`None` for `ungrouped`) and its position there.
    pub fn locate(&self, item: &LayoutItem) -> Option<(Option<UserGroupId>, usize)> {
        self.groups
            .iter()
            .find_map(|group| {
                let at = group.items.iter().position(|held| held == item)?;
                Some((Some(group.id), at))
            })
            .or_else(|| {
                let at = self.ungrouped.iter().position(|held| held == item)?;
                Some((None, at))
            })
    }

    /// The list of group `id`, or `ungrouped` for `None`; the group must exist.
    fn list_mut(&mut self, id: Option<UserGroupId>) -> &mut Vec<LayoutItem> {
        match id.and_then(|id| self.groups.iter_mut().find(|group| group.id == id)) {
            Some(group) => &mut group.items,
            None => &mut self.ungrouped,
        }
    }

    /// `name` trimmed, or why it cannot name a group: blank, or already the name of a group other
    /// than `except` (FR-004).
    fn valid_name(&self, name: &str, except: Option<UserGroupId>) -> Result<String, LayoutError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LayoutError::Blank);
        }
        let folded = name.to_lowercase();
        let taken = self
            .groups
            .iter()
            .any(|group| Some(group.id) != except && group.name.trim().to_lowercase() == folded);
        if taken {
            return Err(LayoutError::Duplicate);
        }
        Ok(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn worktree(name: &str) -> LayoutItem {
        LayoutItem::Worktree(name.to_string())
    }

    fn group(id: u64, name: &str, items: Vec<LayoutItem>) -> UserGroup {
        UserGroup {
            id: UserGroupId(id),
            name: name.to_string(),
            collapsed: false,
            items,
        }
    }

    fn sample() -> SidebarLayout {
        SidebarLayout {
            version: LAYOUT_VERSION,
            next_id: 3,
            groups: vec![
                UserGroup {
                    collapsed: true,
                    ..group(
                        1,
                        "Work",
                        vec![
                            worktree("feat-a"),
                            LayoutItem::Run(GroupId(Uuid::from_u128(7))),
                        ],
                    )
                },
                group(2, "Later", vec![]),
            ],
            ungrouped: vec![worktree("fix-b")],
        }
    }

    #[test]
    fn a_layout_survives_its_own_json() {
        let layout = sample();
        assert_eq!(
            SidebarLayout::try_from_json(&layout.to_json()),
            Ok(layout),
            "groups, order, members and collapsed state all come back from the file (FR-012)"
        );
    }

    #[test]
    fn missing_unparseable_older_or_newer_text_loads_as_no_groups() {
        let older = r#"{ "version": 0, "groups": [{ "id": 1, "name": "Old" }], "ungrouped": [] }"#;
        let newer = r#"{ "version": 2, "groups": [{ "id": 1, "name": "New" }], "ungrouped": [] }"#;
        for text in ["", "{ not json", r#"{ "groups": [] }"#, older, newer] {
            assert!(
                SidebarLayout::try_from_json(text).is_err(),
                "{text:?} is not a layout this build reads, so the store keeps it aside"
            );
            assert_eq!(
                SidebarLayout::from_json(text),
                SidebarLayout::default(),
                "{text:?} loads as no groups, without an error (FR-018)"
            );
        }
    }

    #[test]
    fn a_stored_item_of_no_known_kind_such_as_default_loads_as_no_groups() {
        let text = r#"{ "version": 1, "groups": [{ "id": 1, "name": "W", "items": [{ "Default": null }] }], "ungrouped": [] }"#;
        assert_eq!(
            SidebarLayout::from_json(text),
            SidebarLayout::default(),
            "the project root is never an item, so a file naming it is not one this build wrote (FR-015)"
        );
    }

    #[test]
    fn reconcile_drops_items_no_longer_listed() {
        let mut layout = sample();
        layout.reconcile(&[worktree("feat-a")]);
        assert_eq!(
            layout.groups[0].items,
            vec![worktree("feat-a")],
            "the vanished run group leaves its group"
        );
        assert!(
            layout.ungrouped.is_empty(),
            "the vanished worktree leaves the ungrouped list (FR-018)"
        );
        assert_eq!(layout.groups.len(), 2, "an emptied group stays");
    }

    #[test]
    fn reconcile_keeps_an_item_named_twice_in_the_first_group_only() {
        let mut layout = SidebarLayout {
            groups: vec![
                group(1, "A", vec![worktree("x")]),
                group(2, "B", vec![worktree("x"), worktree("y")]),
            ],
            ungrouped: vec![worktree("y")],
            next_id: 3,
            ..SidebarLayout::default()
        };
        layout.reconcile(&[worktree("x"), worktree("y")]);
        assert_eq!(
            layout.groups[0].items,
            vec![worktree("x")],
            "first group (group order) keeps it"
        );
        assert_eq!(
            layout.groups[1].items,
            vec![worktree("y")],
            "the later copy is dropped (C1)"
        );
        assert!(
            layout.ungrouped.is_empty(),
            "an item already in a group is not also ungrouped"
        );
    }

    #[test]
    fn reconcile_appends_unknown_live_items_to_ungrouped_in_live_order() {
        let mut layout = sample();
        let run = LayoutItem::Run(GroupId(Uuid::from_u128(7)));
        layout.reconcile(&[
            worktree("new-2"),
            worktree("feat-a"),
            run.clone(),
            worktree("fix-b"),
            worktree("new-1"),
        ]);
        assert_eq!(
            layout.ungrouped,
            vec![worktree("fix-b"), worktree("new-2"), worktree("new-1")],
            "saved order first, then the rows the layout did not know, in stable order (C1)"
        );
        assert_eq!(
            layout.groups[0].items,
            vec![worktree("feat-a"), run],
            "grouped rows stay put"
        );
    }

    #[test]
    fn create_trims_the_name_and_adds_an_empty_group_last() {
        let mut layout = sample();
        let id = layout
            .create("  Review  ")
            .expect("a fresh name is accepted");
        let created = layout.groups.last().expect("a group was added");
        assert_eq!(created.id, id);
        assert_eq!(
            created.name, "Review",
            "the stored name is trimmed (FR-004)"
        );
        assert!(
            created.items.is_empty() && !created.collapsed,
            "a new group holds nothing and is open"
        );
    }

    #[test]
    fn created_ids_grow_and_are_never_reused() {
        let mut layout = SidebarLayout::default();
        let first = layout.create("One").expect("accepted");
        let second = layout.create("Two").expect("accepted");
        assert!(second > first, "ids grow");
        layout.groups.retain(|g| g.id != second);
        let third = layout.create("Three").expect("accepted");
        assert!(
            third > second,
            "the id of a removed group is not handed out again"
        );
        let reloaded = SidebarLayout::from_json(&layout.to_json());
        let mut reloaded = reloaded;
        assert!(
            reloaded.create("Four").expect("accepted") > third,
            "the counter survives a restart"
        );
    }

    #[test]
    fn create_refuses_a_blank_name_and_changes_nothing() {
        for name in ["", "   ", "\t\n"] {
            let mut layout = sample();
            assert_eq!(
                layout.create(name),
                Err(LayoutError::Blank),
                "{name:?} is blank (FR-004)"
            );
            assert_eq!(
                layout,
                sample(),
                "a refused create leaves the layout as it was (C2)"
            );
        }
    }

    #[test]
    fn create_refuses_a_duplicate_name_case_insensitively_and_changes_nothing() {
        for name in ["Work", "work", "  WORK "] {
            let mut layout = sample();
            assert_eq!(
                layout.create(name),
                Err(LayoutError::Duplicate),
                "{name:?} names \"Work\" again"
            );
            assert_eq!(
                layout,
                sample(),
                "a refused create leaves the layout as it was (C2)"
            );
        }
    }

    #[test]
    fn rename_changes_the_name_and_keeps_the_members_and_position() {
        let mut layout = sample();
        assert_eq!(layout.rename(UserGroupId(1), "  Office "), Ok(()));
        assert_eq!(layout.groups[0].name, "Office", "the name is trimmed");
        assert_eq!(layout.groups[0].items.len(), 2, "its worktrees stay in it");
        assert!(layout.groups[0].collapsed, "nothing else about it changes");
        assert_eq!(layout.groups[1].name, "Later");
    }

    #[test]
    fn rename_to_its_own_current_name_is_not_a_duplicate() {
        let mut layout = sample();
        assert_eq!(layout.rename(UserGroupId(1), "work"), Ok(()));
        assert_eq!(layout.groups[0].name, "work", "only the case changed");
    }

    #[test]
    fn a_refused_rename_leaves_the_layout_as_it_was() {
        for (id, name, error) in [
            (1, "   ", LayoutError::Blank),
            (1, " later ", LayoutError::Duplicate),
            (9, "Fresh", LayoutError::UnknownGroup),
        ] {
            let mut layout = sample();
            assert_eq!(layout.rename(UserGroupId(id), name), Err(error));
            assert_eq!(layout, sample(), "a refused rename changes nothing (C2)");
        }
    }

    #[test]
    fn delete_appends_the_items_to_ungrouped_in_their_order() {
        let mut layout = sample();
        assert_eq!(layout.delete(UserGroupId(1)), Ok(()));
        assert_eq!(layout.groups.len(), 1);
        assert_eq!(layout.groups[0].id, UserGroupId(2));
        assert_eq!(
            layout.ungrouped,
            vec![
                worktree("fix-b"),
                worktree("feat-a"),
                LayoutItem::Run(GroupId(Uuid::from_u128(7)))
            ],
            "the group's rows return after the ones already ungrouped (FR-003)"
        );
        assert_eq!(layout.next_id, 3, "its id is never reused");
    }

    #[test]
    fn deleting_an_unknown_group_is_refused_and_changes_nothing() {
        let mut layout = sample();
        assert_eq!(
            layout.delete(UserGroupId(9)),
            Err(LayoutError::UnknownGroup)
        );
        assert_eq!(layout, sample());
    }

    fn run7() -> LayoutItem {
        LayoutItem::Run(GroupId(Uuid::from_u128(7)))
    }

    /// Every item the layout holds, with the list it is in.
    fn placements(layout: &SidebarLayout) -> Vec<(LayoutItem, Option<UserGroupId>)> {
        let mut all: Vec<_> = layout
            .groups
            .iter()
            .flat_map(|g| g.items.iter().map(move |i| (i.clone(), Some(g.id))))
            .collect();
        all.extend(layout.ungrouped.iter().map(|i| (i.clone(), None)));
        all
    }

    fn assert_each_item_in_one_list(layout: &SidebarLayout, expected: usize) {
        let all = placements(layout);
        let mut seen = HashSet::new();
        for (item, _) in &all {
            assert!(seen.insert(item.clone()), "{item:?} is in two lists");
        }
        assert_eq!(all.len(), expected, "no item is lost or invented");
    }

    #[test]
    fn adopt_places_an_unheld_item_in_ungrouped_and_leaves_a_held_one() {
        let mut layout = sample();
        layout.adopt(&worktree("new"));
        layout.adopt(&worktree("feat-a"));
        assert_eq!(layout.ungrouped, vec![worktree("fix-b"), worktree("new")]);
        assert!(layout.holds(&worktree("new")) && !layout.holds(&worktree("nope")));
        assert_eq!(
            layout.list_of(&worktree("feat-a")),
            Some(Some(UserGroupId(1)))
        );
        assert_eq!(layout.list_of(&worktree("fix-b")), Some(None));
        assert_eq!(layout.list_of(&worktree("nope")), None);
    }

    #[test]
    fn move_item_into_a_group_removes_it_from_where_it_was() {
        let mut layout = sample();
        let to = DropTarget::Group {
            id: UserGroupId(2),
            index: 0,
        };
        assert_eq!(layout.move_item(&worktree("fix-b"), to), Ok(()));
        assert_eq!(
            layout.group(UserGroupId(2)).unwrap().items,
            vec![worktree("fix-b")]
        );
        assert!(
            layout.ungrouped.is_empty(),
            "removed before inserted (FR-005)"
        );
        assert_each_item_in_one_list(&layout, 3);
    }

    #[test]
    fn move_item_to_ungrouped_appends_at_the_index() {
        let mut layout = sample();
        let to = DropTarget::Ungrouped { index: 0 };
        assert_eq!(layout.move_item(&worktree("feat-a"), to), Ok(()));
        assert_eq!(
            layout.ungrouped,
            vec![worktree("feat-a"), worktree("fix-b")]
        );
        assert_eq!(layout.group(UserGroupId(1)).unwrap().items, vec![run7()]);
    }

    #[test]
    fn move_item_clamps_an_index_past_the_end() {
        let mut layout = sample();
        let to = DropTarget::Group {
            id: UserGroupId(1),
            index: 99,
        };
        assert_eq!(layout.move_item(&worktree("fix-b"), to), Ok(()));
        assert_eq!(
            layout.group(UserGroupId(1)).unwrap().items,
            vec![worktree("feat-a"), run7(), worktree("fix-b")]
        );
    }

    #[test]
    fn move_item_within_its_list_counts_the_index_before_the_removal() {
        let mut layout = SidebarLayout {
            ungrouped: vec![worktree("a"), worktree("b"), worktree("c")],
            ..SidebarLayout::default()
        };
        let at = |index| DropTarget::Ungrouped { index };
        assert_eq!(layout.move_item(&worktree("a"), at(2)), Ok(()));
        assert_eq!(
            layout.ungrouped,
            vec![worktree("b"), worktree("a"), worktree("c")]
        );
        assert_eq!(layout.move_item(&worktree("a"), at(3)), Ok(()));
        assert_eq!(
            layout.ungrouped,
            vec![worktree("b"), worktree("c"), worktree("a")]
        );
        assert_eq!(layout.move_item(&worktree("a"), at(0)), Ok(()));
        assert_eq!(
            layout.ungrouped,
            vec![worktree("a"), worktree("b"), worktree("c")]
        );
        let before = layout.clone();
        assert_eq!(layout.move_item(&worktree("b"), at(1)), Ok(()));
        assert_eq!(
            layout, before,
            "a drop on its own place changes nothing (C4)"
        );
    }

    #[test]
    fn a_run_group_moves_as_one_item() {
        let mut layout = sample();
        let to = DropTarget::Group {
            id: UserGroupId(2),
            index: 0,
        };
        assert_eq!(layout.move_item(&run7(), to), Ok(()));
        assert_eq!(layout.group(UserGroupId(2)).unwrap().items, vec![run7()]);
        assert_eq!(
            layout.group(UserGroupId(1)).unwrap().items,
            vec![worktree("feat-a")]
        );
        assert_each_item_in_one_list(&layout, 3);
    }

    #[test]
    fn move_item_refuses_what_it_cannot_place_and_changes_nothing() {
        let mut layout = sample();
        let ungrouped = DropTarget::Ungrouped { index: 0 };
        assert_eq!(
            layout.move_item(&worktree("gone"), ungrouped),
            Err(LayoutError::UnknownItem)
        );
        assert_eq!(
            layout.move_item(&worktree(""), ungrouped),
            Err(LayoutError::IsDefault),
            "the project root is never an item (FR-015)"
        );
        let missing = DropTarget::Group {
            id: UserGroupId(9),
            index: 0,
        };
        assert_eq!(
            layout.move_item(&worktree("fix-b"), missing),
            Err(LayoutError::UnknownGroup)
        );
        assert_eq!(
            layout.move_item(&worktree("fix-b"), DropTarget::GroupSlot { index: 0 }),
            Err(LayoutError::UnknownGroup),
            "a slot between headers is for a group, not an item"
        );
        assert_eq!(layout, sample());
    }

    #[test]
    fn every_outcome_keeps_each_item_in_exactly_one_list() {
        let items = [worktree("feat-a"), run7(), worktree("fix-b")];
        let mut targets = vec![];
        for index in 0..4 {
            targets.push(DropTarget::Ungrouped { index });
            for id in [1, 2] {
                targets.push(DropTarget::Group {
                    id: UserGroupId(id),
                    index,
                });
            }
        }
        for item in &items {
            for to in &targets {
                for start in [sample(), {
                    let mut l = sample();
                    l.delete(UserGroupId(1)).unwrap();
                    l
                }] {
                    let mut layout = start.clone();
                    let _ = layout.move_item(item, *to);
                    assert_each_item_in_one_list(&layout, 3);
                }
            }
        }
    }
}
