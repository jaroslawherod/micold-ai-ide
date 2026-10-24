//! Feature 1269 — the Review tab rules: one Review tab per location, kept while hidden.
//!
//! A window holds at most one Review tab per location (a project and one of its locations,
//! [`EntryKey`]). Switching to a session hides the shown Review tab and keeps its view; only ×
//! ([`ReviewTabs::close`]) or the location's removal ([`ReviewTabs::drop_gone_worktrees`],
//! [`ReviewTabs::drop_project`]) drops it. The module also decides when a kept view must be read
//! again ([`Freshness`]), which session a location shows ([`ReviewTabs::session_for`]), and what
//! the tab and the Changes button say ([`ReviewTabFace`], [`ChangesButton`]).
//!
//! Render-free and iced-free (FR-016), generic over the kept view `V`: the client keeps its
//! `features::changes::OpenView` here, the tests a plain number. Rules:
//! `specs/1269-review-tab/contracts/review-tabs.md` T1–T8.

use crate::session::{SessionId, SessionLocation};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

/// A project and one of its locations (Default or a worktree's `dir_name`).
pub type EntryKey = (PathBuf, SessionLocation);

/// Whether a kept view's last read is current as far as the window knows (T5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Freshness {
    /// The last read is current.
    #[default]
    Fresh,
    /// The location's files changed while it was shown and the tab hidden: the tab carries the
    /// changed mark while hidden.
    Changed,
    /// The tab was not watched for a while (location left, covered, comments changed): read
    /// again on the next show, with no mark.
    Unwatched,
}

/// One location's Review tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewTab<V> {
    /// The kept view.
    pub view: V,
    freshness: Freshness,
    return_to: Option<SessionId>,
}

impl<V> ReviewTab<V> {
    /// Whether the kept view is current (T5).
    pub fn freshness(&self) -> Freshness {
        self.freshness
    }

    /// The session that was current when the tab was last shown; × shows it if still live (T3).
    pub fn return_to(&self) -> Option<SessionId> {
        self.return_to
    }

    /// The tab stops being watched: a fresh tab must be read again on its next show; a changed
    /// one keeps its mark.
    fn unwatch(&mut self) {
        if self.freshness == Freshness::Fresh {
            self.freshness = Freshness::Unwatched;
        }
    }

    /// Showing the tab: answers whether its view must be read again, and makes it fresh. At most
    /// once per change (T5).
    fn take_reread(&mut self) -> bool {
        std::mem::take(&mut self.freshness) != Freshness::Fresh
    }
}

/// A window's Review tabs: which exist, which is shown, and each location's last-shown session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewTabs<V> {
    /// At most one tab per key (FR-002).
    tabs: BTreeMap<EntryKey, ReviewTab<V>>,
    /// Names a key of `tabs`, or `None`.
    shown: Option<EntryKey>,
    /// Entries of removed keys are removed with them.
    last_session: BTreeMap<EntryKey, SessionId>,
}

impl<V> Default for ReviewTabs<V> {
    fn default() -> Self {
        Self {
            tabs: BTreeMap::new(),
            shown: None,
            last_session: BTreeMap::new(),
        }
    }
}

/// What [`ReviewTabs::open_or_show`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    /// No tab existed: one was made and shown.
    Created,
    /// A hidden tab was shown; `reread` says its view must be read again first.
    Shown { reread: bool },
    /// The tab was already the shown tab: nothing changed.
    AlreadyShown,
}

/// What [`ReviewTabs::close`] removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Closed<V> {
    /// The view the tab kept.
    pub view: V,
    /// Whether the closed tab was the shown tab.
    pub was_shown: bool,
    /// The session to show instead: the tab's `return_to` if live, else the main session.
    pub show: Option<SessionId>,
}

/// What [`ReviewTabs::files_changed`] asks of the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The tab is shown: read its changes again now.
    Reread,
    /// The tab is hidden: it now carries the changed mark.
    Marked,
    /// No tab for the location.
    Nothing,
}

impl<V> ReviewTabs<V> {
    /// T1: show `key`'s tab, making it with `make` when there is none.
    pub fn open_or_show(
        &mut self,
        key: EntryKey,
        current: Option<SessionId>,
        make: impl FnOnce() -> V,
    ) -> Opened {
        if self.shown.as_ref() == Some(&key) && self.tabs.contains_key(&key) {
            return Opened::AlreadyShown;
        }
        let opened = match self.tabs.get_mut(&key) {
            Some(tab) => {
                tab.return_to = current;
                Opened::Shown {
                    reread: tab.take_reread(),
                }
            }
            None => {
                self.tabs.insert(
                    key.clone(),
                    ReviewTab {
                        view: make(),
                        freshness: Freshness::Fresh,
                        return_to: current,
                    },
                );
                Opened::Created
            }
        };
        self.shown = Some(key);
        opened
    }

    /// T2: hide the shown tab, if any; answers whether one was shown.
    pub fn hide(&mut self) -> bool {
        self.shown.take().is_some()
    }

    /// T3: remove `key`'s tab (×) and say which session to show instead.
    pub fn close(
        &mut self,
        key: &EntryKey,
        live: &[SessionId],
        main: Option<SessionId>,
    ) -> Option<Closed<V>> {
        let tab = self.tabs.remove(key)?;
        let was_shown = self.shown.as_ref() == Some(key);
        if was_shown {
            self.shown = None;
        }
        let show = tab.return_to.filter(|s| live.contains(s)).or(main);
        Some(Closed {
            view: tab.view,
            was_shown,
            show,
        })
    }

    /// T7: show `key`'s tab when it exists; `Some(reread)` as in T1, `None` with no tab.
    pub fn show_if_present(&mut self, key: &EntryKey) -> Option<bool> {
        let reread = self.tabs.get_mut(key)?.take_reread();
        self.shown = Some(key.clone());
        Some(reread)
    }

    /// T4: remove every worktree tab of `project` whose `dir_name` is not in `names` (never its
    /// Default tab); answers the removed keys in key order.
    pub fn drop_gone_worktrees(
        &mut self,
        project: &Path,
        names: &BTreeSet<String>,
    ) -> Vec<EntryKey> {
        self.drop_where(|(p, location)| {
            p == project
                && matches!(location, SessionLocation::Worktree(dir) if !names.contains(dir))
        })
    }

    /// T4: remove every tab of `project`; answers the removed keys in key order.
    pub fn drop_project(&mut self, project: &Path) -> Vec<EntryKey> {
        self.drop_where(|(p, _)| p == project)
    }

    /// Removes the tabs, the shown mark and the last sessions of every key `gone` names, and
    /// answers the keys whose tab it removed. The one way a tab goes besides × (T4).
    fn drop_where(&mut self, gone: impl Fn(&EntryKey) -> bool) -> Vec<EntryKey> {
        let removed: Vec<EntryKey> = self.tabs.keys().filter(|k| gone(k)).cloned().collect();
        for key in &removed {
            self.tabs.remove(key);
        }
        if self.shown.as_ref().is_some_and(&gone) {
            self.shown = None;
        }
        self.last_session.retain(|key, _| !gone(key));
        removed
    }

    /// T5: the location's files changed.
    pub fn files_changed(&mut self, key: &EntryKey) -> Change {
        if self.shown.as_ref() == Some(key) {
            return Change::Reread;
        }
        match self.tabs.get_mut(key) {
            Some(tab) => {
                tab.freshness = Freshness::Changed;
                Change::Marked
            }
            None => Change::Nothing,
        }
    }

    /// T5: the location stopped being watched for its tab (left, or covered while shown).
    pub fn location_left(&mut self, key: &EntryKey) {
        if let Some(tab) = self.tabs.get_mut(key) {
            tab.unwatch();
        }
    }

    /// T5: the location's comments changed.
    pub fn comments_changed(&mut self, key: &EntryKey) {
        if self.shown.as_ref() == Some(key) {
            return;
        }
        if let Some(tab) = self.tabs.get_mut(key) {
            tab.unwatch();
        }
    }

    /// T5: the shown tab is no longer covered; answers whether it must be read again.
    pub fn uncovered(&mut self, key: &EntryKey) -> bool {
        self.tabs.get_mut(key).is_some_and(ReviewTab::take_reread)
    }

    /// T6: `session` is the location's last-shown session in this window.
    pub fn remember(&mut self, key: EntryKey, session: SessionId) {
        self.last_session.insert(key, session);
    }

    /// T6: the session the location shows: the remembered one if live, else `main`.
    pub fn session_for(
        &self,
        key: &EntryKey,
        live: &[SessionId],
        main: Option<SessionId>,
    ) -> Option<SessionId> {
        self.last_session
            .get(key)
            .copied()
            .filter(|s| live.contains(s))
            .or(main)
    }

    /// The shown tab's key.
    pub fn shown_key(&self) -> Option<&EntryKey> {
        self.shown.as_ref()
    }

    /// The shown tab.
    pub fn shown(&self) -> Option<&ReviewTab<V>> {
        self.tabs.get(self.shown.as_ref()?)
    }

    /// The shown tab, to update its view.
    pub fn shown_mut(&mut self) -> Option<&mut ReviewTab<V>> {
        self.tabs.get_mut(self.shown.as_ref()?)
    }

    /// `key`'s tab.
    pub fn get(&self, key: &EntryKey) -> Option<&ReviewTab<V>> {
        self.tabs.get(key)
    }

    /// `key`'s tab, to update its view.
    pub fn get_mut(&mut self, key: &EntryKey) -> Option<&mut ReviewTab<V>> {
        self.tabs.get_mut(key)
    }

    /// Whether `key` has a tab.
    pub fn has(&self, key: &EntryKey) -> bool {
        self.tabs.contains_key(key)
    }

    /// How many tabs exist.
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Whether no tab exists.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Every tab, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&EntryKey, &ReviewTab<V>)> {
        self.tabs.iter()
    }
}

/// The Review tab's label.
const REVIEW: &str = "Review";
/// The tooltip line of a tab with the changed mark (FR-010).
const CHANGED_SINCE_YOU_LOOKED: &str = "Changed since you looked";
/// The largest draft count the badge spells out; above it the badge reads `99+`.
const BADGE_CAP: usize = 99;

/// What a Review tab shows in the strip (T8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewTabFace {
    pub label: &'static str,
    /// The draft count; never zero.
    pub badge: Option<NonZeroUsize>,
    /// The changed mark; never on a shown tab.
    pub changed: bool,
    pub tooltip: String,
}

impl ReviewTabFace {
    /// The face of a tab with `drafts` draft comments (`None` while unknown).
    pub fn of(drafts: Option<usize>, freshness: Freshness, shown: bool) -> Self {
        let badge = drafts.and_then(NonZeroUsize::new);
        let changed = !shown && freshness == Freshness::Changed;
        let mut tooltip = String::from(REVIEW);
        if let Some(n) = badge {
            let plural = if n.get() == 1 { "" } else { "s" };
            tooltip.push_str(&format!(" · {n} draft{plural}"));
        }
        if changed {
            tooltip.push('\n');
            tooltip.push_str(CHANGED_SINCE_YOU_LOOKED);
        }
        Self {
            label: REVIEW,
            badge,
            changed,
            tooltip,
        }
    }

    /// The badge's text: `1`…`99`, then `99+`.
    pub fn badge_text(&self) -> Option<String> {
        self.badge.map(|n| {
            if n.get() > BADGE_CAP {
                format!("{BADGE_CAP}+")
            } else {
                n.to_string()
            }
        })
    }
}

/// What a press of the Changes button does (T8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// Make the location's Review tab.
    Create,
    /// Show the location's hidden Review tab.
    Show,
    /// The Review tab is shown: nothing changes.
    None,
}

/// The Changes button's face (T8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangesButton {
    pub press: Press,
    pub selected: bool,
    pub tooltip: &'static str,
}

impl ChangesButton {
    /// The button for a location that has a Review tab (`has_tab`), shown or not.
    pub fn of(has_tab: bool, shown: bool) -> Self {
        match (has_tab, shown) {
            (false, _) => Self {
                press: Press::Create,
                selected: false,
                tooltip: "Open the review",
            },
            (true, false) => Self {
                press: Press::Show,
                selected: false,
                tooltip: "Show the review",
            },
            (true, true) => Self {
                press: Press::None,
                selected: true,
                tooltip: "The review is shown",
            },
        }
    }
}

#[cfg(test)]
#[path = "../tests/support/rng.rs"]
mod test_rng;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use uuid::Uuid;

    fn project(name: &str) -> PathBuf {
        PathBuf::from(format!("/projects/{name}"))
    }

    fn default_of(name: &str) -> EntryKey {
        (project(name), SessionLocation::Default)
    }

    fn worktree(name: &str, dir: &str) -> EntryKey {
        (project(name), SessionLocation::Worktree(dir.to_owned()))
    }

    fn session(n: u128) -> SessionId {
        SessionId::from_uuid(Uuid::from_u128(n))
    }

    /// A map with one tab per key, each made with the key's index as its view, none shown.
    fn with_hidden(keys: &[EntryKey]) -> ReviewTabs<u32> {
        let mut tabs = ReviewTabs::default();
        for (n, key) in keys.iter().enumerate() {
            tabs.open_or_show(key.clone(), None, || n as u32);
        }
        tabs.hide();
        tabs
    }

    // ---- T1. Open or show -------------------------------------------------------------

    #[test]
    fn t1_opening_a_location_with_no_tab_makes_one_fresh_and_shows_it() {
        let mut tabs = ReviewTabs::default();
        let calls = Cell::new(0);
        let opened = tabs.open_or_show(worktree("p", "w"), Some(session(1)), || {
            calls.set(calls.get() + 1);
            7
        });
        assert_eq!(opened, Opened::Created);
        assert_eq!(calls.get(), 1);
        assert_eq!(tabs.shown_key(), Some(&worktree("p", "w")));
        let tab = tabs.shown().expect("shown tab");
        assert_eq!(tab.view, 7);
        assert_eq!(tab.freshness(), Freshness::Fresh);
        assert_eq!(tab.return_to(), Some(session(1)));
        assert_eq!(tabs.len(), 1);
        assert!(!tabs.is_empty());
    }

    #[test]
    fn t1_opening_a_hidden_fresh_tab_shows_it_without_making_or_reading() {
        let mut tabs = with_hidden(&[worktree("p", "w")]);
        let opened = tabs.open_or_show(worktree("p", "w"), Some(session(2)), || {
            panic!("make must not be called for an existing tab")
        });
        assert_eq!(opened, Opened::Shown { reread: false });
        assert_eq!(tabs.shown_key(), Some(&worktree("p", "w")));
        let tab = tabs.shown().expect("shown tab");
        assert_eq!(tab.view, 0);
        assert_eq!(tab.return_to(), Some(session(2)));
    }

    #[test]
    fn t1_opening_a_hidden_changed_or_unwatched_tab_rereads_it_and_makes_it_fresh() {
        let key = worktree("p", "w");
        let mut changed = with_hidden(&[key.clone()]);
        changed.files_changed(&key);
        assert_eq!(changed.get(&key).unwrap().freshness(), Freshness::Changed);
        assert_eq!(
            changed.open_or_show(key.clone(), None, || 9),
            Opened::Shown { reread: true }
        );
        assert_eq!(changed.get(&key).unwrap().freshness(), Freshness::Fresh);

        let mut unwatched = with_hidden(&[key.clone()]);
        unwatched.location_left(&key);
        assert_eq!(
            unwatched.get(&key).unwrap().freshness(),
            Freshness::Unwatched
        );
        assert_eq!(
            unwatched.open_or_show(key.clone(), None, || 9),
            Opened::Shown { reread: true }
        );
        assert_eq!(unwatched.get(&key).unwrap().freshness(), Freshness::Fresh);
        assert_eq!(unwatched.get(&key).unwrap().view, 0);
    }

    #[test]
    fn t1_opening_the_shown_tab_changes_nothing() {
        let key = worktree("p", "w");
        let mut tabs = ReviewTabs::default();
        tabs.open_or_show(key.clone(), Some(session(1)), || 3);
        tabs.location_left(&key);
        let before = tabs.clone();
        let opened = tabs.open_or_show(key.clone(), Some(session(2)), || {
            panic!("make must not be called for the shown tab")
        });
        assert_eq!(opened, Opened::AlreadyShown);
        assert_eq!(tabs, before, "not return_to, not freshness");
    }

    #[test]
    fn t1_repeated_opens_leave_one_tab_per_location() {
        let key = default_of("p");
        let mut tabs = ReviewTabs::default();
        for n in 0..5 {
            tabs.open_or_show(key.clone(), Some(session(n)), || n as u32);
            if n % 2 == 0 {
                tabs.hide();
            }
            assert_eq!(tabs.len(), 1);
        }
        assert_eq!(tabs.get(&key).unwrap().view, 0);
    }

    #[test]
    fn t1_opening_another_location_hides_the_shown_one_and_keeps_its_view() {
        let (w, x) = (worktree("p", "w"), worktree("p", "x"));
        let mut tabs = ReviewTabs::default();
        tabs.open_or_show(w.clone(), None, || 1);
        assert_eq!(tabs.open_or_show(x.clone(), None, || 2), Opened::Created);
        assert_eq!(tabs.shown_key(), Some(&x));
        assert_eq!(tabs.len(), 2);
        assert!(tabs.has(&w));
        assert_eq!(tabs.get(&w).unwrap().view, 1);
        assert_eq!(
            tabs.iter()
                .map(|(k, t)| (k.clone(), t.view))
                .collect::<Vec<_>>(),
            vec![(w, 1), (x, 2)]
        );
    }

    // ---- T2. Hide ---------------------------------------------------------------------

    #[test]
    fn t2_hide_answers_whether_a_tab_was_shown_and_changes_no_tab() {
        let (w, x) = (worktree("p", "w"), worktree("p", "x"));
        let mut tabs = ReviewTabs::default();
        tabs.open_or_show(w.clone(), Some(session(1)), || 1);
        tabs.open_or_show(x.clone(), Some(session(2)), || 2);
        tabs.files_changed(&w);
        let before: Vec<_> = tabs.iter().map(|(k, t)| (k.clone(), t.clone())).collect();
        assert!(tabs.hide());
        assert_eq!(tabs.shown_key(), None);
        assert!(tabs.shown().is_none());
        let after: Vec<_> = tabs.iter().map(|(k, t)| (k.clone(), t.clone())).collect();
        assert_eq!(after, before);

        let hidden = tabs.clone();
        assert!(!tabs.hide());
        assert_eq!(tabs, hidden);
    }

    // ---- T3. Close with × -------------------------------------------------------------

    #[test]
    fn t3_closing_a_location_with_no_tab_answers_none_and_changes_nothing() {
        let mut tabs = with_hidden(&[worktree("p", "w")]);
        let before = tabs.clone();
        assert_eq!(
            tabs.close(&worktree("p", "x"), &[session(1)], Some(session(1))),
            None
        );
        assert_eq!(tabs, before);
    }

    #[test]
    fn t3_close_removes_the_tab_and_shows_return_to_if_live_else_main_else_none() {
        let key = worktree("p", "w");
        let (s1, s2) = (session(1), session(2));
        let open = || {
            let mut tabs = ReviewTabs::default();
            tabs.open_or_show(key.clone(), Some(s2), || 5);
            tabs
        };

        let mut tabs = open();
        let closed = tabs.close(&key, &[s1, s2], Some(s1)).expect("closed");
        assert_eq!(
            closed,
            Closed {
                view: 5,
                was_shown: true,
                show: Some(s2)
            }
        );
        assert_eq!(tabs.shown_key(), None);
        assert!(!tabs.has(&key));
        assert!(tabs.is_empty());

        let mut tabs = open();
        assert_eq!(tabs.close(&key, &[s1], Some(s1)).unwrap().show, Some(s1));

        let mut tabs = open();
        assert_eq!(tabs.close(&key, &[], None).unwrap().show, None);

        let mut tabs = open();
        tabs.hide();
        assert!(!tabs.close(&key, &[s2], None).unwrap().was_shown);
    }

    #[test]
    fn t3_no_kept_view_survives_close_and_another_tab_is_untouched() {
        let (w, x) = (worktree("p", "w"), worktree("p", "x"));
        let mut tabs = ReviewTabs::default();
        tabs.open_or_show(w.clone(), None, || 1);
        tabs.open_or_show(x.clone(), None, || 2);
        let other = tabs.get(&w).cloned();
        tabs.close(&x, &[], None);
        assert_eq!(tabs.get(&w).cloned(), other);
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs.open_or_show(x.clone(), None, || 3), Opened::Created);
        assert_eq!(tabs.get(&x).unwrap().view, 3);

        // Closing a hidden tab leaves the shown one shown.
        tabs.open_or_show(w.clone(), None, || 0);
        tabs.close(&x, &[], None);
        assert_eq!(tabs.shown_key(), Some(&w));
    }

    // ---- T7. Last session of a location closed ------------------------------------------

    #[test]
    fn t7_show_if_present_shows_an_existing_tab_and_answers_reread_as_t1() {
        let (w, x) = (worktree("p", "w"), worktree("p", "x"));
        let mut tabs = with_hidden(&[w.clone()]);
        let before = tabs.clone();
        assert_eq!(tabs.show_if_present(&x), None);
        assert_eq!(tabs, before);

        assert_eq!(tabs.show_if_present(&w), Some(false));
        assert_eq!(tabs.shown_key(), Some(&w));

        tabs.hide();
        tabs.files_changed(&w);
        assert_eq!(tabs.show_if_present(&w), Some(true));
        assert_eq!(tabs.get(&w).unwrap().freshness(), Freshness::Fresh);
        assert_eq!(tabs.len(), 1);
    }

    // ---- T4. Drop rules ---------------------------------------------------------------

    fn names(dirs: &[&str]) -> BTreeSet<String> {
        dirs.iter().map(|d| (*d).to_owned()).collect()
    }

    #[test]
    fn t4_drop_gone_worktrees_removes_only_that_projects_missing_worktrees() {
        let (pd, pw, px, qx) = (
            default_of("p"),
            worktree("p", "w"),
            worktree("p", "x"),
            worktree("q", "x"),
        );
        let mut tabs = with_hidden(&[pd.clone(), pw.clone(), px.clone(), qx.clone()]);
        tabs.open_or_show(pw.clone(), None, || 0);
        for key in [&pd, &pw, &px, &qx] {
            tabs.remember(key.clone(), session(1));
        }
        tabs.remember(worktree("p", "y"), session(1));

        let removed = tabs.drop_gone_worktrees(&project("p"), &names(&["x"]));
        assert_eq!(removed, vec![pw.clone()]);
        assert_eq!(tabs.shown_key(), None, "the removed tab was shown");
        assert!(!tabs.has(&pw));
        assert!(tabs.has(&pd) && tabs.has(&px) && tabs.has(&qx));
        assert_eq!(tabs.session_for(&pw, &[session(1)], None), None);
        assert_eq!(
            tabs.session_for(&worktree("p", "y"), &[session(1)], None),
            None,
            "a gone worktree's last session goes with it, tab or not"
        );
        assert_eq!(tabs.session_for(&px, &[session(1)], None), Some(session(1)));
        assert_eq!(tabs.session_for(&qx, &[session(1)], None), Some(session(1)));

        // Every worktree gone: Default stays.
        assert_eq!(
            tabs.drop_gone_worktrees(&project("p"), &names(&[])),
            vec![px.clone()]
        );
        assert!(tabs.has(&pd) && tabs.has(&qx));
        assert_eq!(tabs.session_for(&pd, &[session(1)], None), Some(session(1)));
    }

    #[test]
    fn t4_drop_gone_worktrees_keeps_a_shown_tab_it_does_not_remove() {
        let (pw, px) = (worktree("p", "w"), worktree("p", "x"));
        let mut tabs = with_hidden(&[pw.clone()]);
        tabs.open_or_show(px.clone(), None, || 1);
        assert_eq!(
            tabs.drop_gone_worktrees(&project("p"), &names(&["x"])),
            vec![pw]
        );
        assert_eq!(tabs.shown_key(), Some(&px));
    }

    #[test]
    fn t4_drop_project_removes_every_key_of_that_project_only() {
        let (pd, pw, qx) = (default_of("p"), worktree("p", "w"), worktree("q", "x"));
        let mut tabs = with_hidden(&[pd.clone(), pw.clone(), qx.clone()]);
        tabs.open_or_show(pd.clone(), None, || 0);
        tabs.remember(pd.clone(), session(1));
        tabs.remember(qx.clone(), session(2));

        // In key order: a worktree sorts before Default.
        assert_eq!(
            tabs.drop_project(&project("p")),
            vec![pw.clone(), pd.clone()]
        );
        assert_eq!(tabs.shown_key(), None);
        assert_eq!(tabs.len(), 1);
        assert!(tabs.has(&qx));
        assert_eq!(tabs.session_for(&pd, &[session(1)], None), None);
        assert_eq!(tabs.session_for(&qx, &[session(2)], None), Some(session(2)));
        assert_eq!(tabs.drop_project(&project("p")), Vec::<EntryKey>::new());
    }

    #[test]
    fn t4_nothing_but_the_drop_rules_and_close_removes_a_tab() {
        let keys = [default_of("p"), worktree("p", "w"), worktree("q", "x")];
        let sessions = [session(1), session(2), session(3)];
        for seed in [1, 0x1269, 0xdead_beef, 42, 7_777_777] {
            let mut rng = test_rng::Rng::new(seed);
            let mut tabs = with_hidden(&keys);
            let start = tabs.len();
            for step in 0..200 {
                let key = keys[rng.below(keys.len())].clone();
                let current = [None, Some(sessions[rng.below(sessions.len())])][rng.below(2)];
                match rng.below(7) {
                    0 => {
                        tabs.open_or_show(key, current, || 99);
                    }
                    1 => {
                        tabs.hide();
                    }
                    2 => {
                        tabs.files_changed(&key);
                    }
                    3 => tabs.location_left(&key),
                    4 => tabs.comments_changed(&key),
                    5 => tabs.remember(key, sessions[rng.below(sessions.len())]),
                    _ => {
                        tabs.show_if_present(&key);
                    }
                }
                assert!(
                    tabs.len() >= start,
                    "seed {seed:#x}, step {step}: a tab was removed"
                );
                assert!(
                    tabs.shown_key().is_none_or(|k| tabs.has(k)),
                    "seed {seed:#x}, step {step}: shown names no tab"
                );
            }
        }
    }

    // ---- T5. Freshness and the changed mark ---------------------------------------------

    fn freshness(tabs: &ReviewTabs<u32>, key: &EntryKey) -> Freshness {
        tabs.get(key).expect("tab").freshness()
    }

    /// A map with a hidden tab for `key` in `state`, and a shown tab for another location.
    fn hidden_in(key: &EntryKey, state: Freshness) -> ReviewTabs<u32> {
        let mut tabs = with_hidden(&[key.clone()]);
        match state {
            Freshness::Fresh => {}
            Freshness::Changed => {
                tabs.files_changed(key);
            }
            Freshness::Unwatched => tabs.location_left(key),
        }
        tabs.open_or_show(worktree("other", "o"), None, || 0);
        assert_eq!(freshness(&tabs, key), state);
        tabs
    }

    #[test]
    fn t5_files_changed_rereads_a_shown_tab_marks_a_hidden_one_and_ignores_no_tab() {
        let key = worktree("p", "w");
        let mut tabs = ReviewTabs::default();
        tabs.open_or_show(key.clone(), None, || 1);
        assert_eq!(tabs.files_changed(&key), Change::Reread);
        assert_eq!(freshness(&tabs, &key), Freshness::Fresh);

        for state in [Freshness::Fresh, Freshness::Changed, Freshness::Unwatched] {
            let mut tabs = hidden_in(&key, state);
            assert_eq!(tabs.files_changed(&key), Change::Marked, "{state:?}");
            assert_eq!(freshness(&tabs, &key), Freshness::Changed, "{state:?}");
        }

        let mut tabs = with_hidden(&[key.clone()]);
        let before = tabs.clone();
        assert_eq!(tabs.files_changed(&worktree("p", "x")), Change::Nothing);
        assert_eq!(tabs, before);
    }

    #[test]
    fn t5_location_left_unwatches_a_fresh_tab_and_keeps_the_changed_mark() {
        let key = worktree("p", "w");
        let mut hidden = hidden_in(&key, Freshness::Fresh);
        hidden.location_left(&key);
        assert_eq!(freshness(&hidden, &key), Freshness::Unwatched);

        let mut shown = ReviewTabs::default();
        shown.open_or_show(key.clone(), None, || 1);
        shown.location_left(&key);
        assert_eq!(freshness(&shown, &key), Freshness::Unwatched);
        assert_eq!(
            shown.shown_key(),
            Some(&key),
            "covered: the review stays shown"
        );

        let mut changed = hidden_in(&key, Freshness::Changed);
        changed.location_left(&key);
        assert_eq!(freshness(&changed, &key), Freshness::Changed);

        let mut none = with_hidden(&[key.clone()]);
        let before = none.clone();
        none.location_left(&worktree("p", "x"));
        assert_eq!(none, before);
    }

    #[test]
    fn t5_comments_changed_unwatches_a_hidden_fresh_tab_only() {
        let key = worktree("p", "w");
        let mut hidden = hidden_in(&key, Freshness::Fresh);
        hidden.comments_changed(&key);
        assert_eq!(freshness(&hidden, &key), Freshness::Unwatched);

        let mut changed = hidden_in(&key, Freshness::Changed);
        changed.comments_changed(&key);
        assert_eq!(freshness(&changed, &key), Freshness::Changed);

        let mut shown = ReviewTabs::default();
        shown.open_or_show(key.clone(), None, || 1);
        let before = shown.clone();
        shown.comments_changed(&key);
        assert_eq!(shown, before);
    }

    #[test]
    fn t5_showing_rereads_at_most_once_per_change() {
        let key = worktree("p", "w");
        for state in [Freshness::Changed, Freshness::Unwatched] {
            let mut tabs = hidden_in(&key, state);
            assert_eq!(
                tabs.open_or_show(key.clone(), None, || 0),
                Opened::Shown { reread: true }
            );
            assert_eq!(freshness(&tabs, &key), Freshness::Fresh);
            tabs.hide();
            assert_eq!(
                tabs.open_or_show(key.clone(), None, || 0),
                Opened::Shown { reread: false },
                "{state:?}: no change between two shows"
            );

            let mut tabs = hidden_in(&key, state);
            assert_eq!(tabs.show_if_present(&key), Some(true));
            tabs.hide();
            assert_eq!(tabs.show_if_present(&key), Some(false));
        }
    }

    #[test]
    fn t5_uncovered_answers_true_exactly_once_after_location_left_on_the_shown_key() {
        let key = worktree("p", "w");
        let mut tabs = ReviewTabs::default();
        tabs.open_or_show(key.clone(), None, || 1);
        assert!(!tabs.uncovered(&key), "never covered");
        tabs.location_left(&key);
        assert!(tabs.uncovered(&key));
        assert_eq!(freshness(&tabs, &key), Freshness::Fresh);
        assert!(!tabs.uncovered(&key), "once");
        assert!(!tabs.uncovered(&worktree("p", "x")), "no tab");
    }

    // ---- T6. Last-shown session ---------------------------------------------------------

    #[test]
    fn t6_session_for_answers_the_remembered_live_session_else_main_else_none() {
        let key = worktree("p", "w");
        let (s1, s2) = (session(1), session(2));
        let mut tabs = ReviewTabs::<u32>::default();
        assert_eq!(
            tabs.session_for(&key, &[s1], Some(s1)),
            Some(s1),
            "none remembered"
        );
        assert_eq!(tabs.session_for(&key, &[], None), None, "sessionless");

        tabs.remember(key.clone(), s2);
        assert_eq!(tabs.session_for(&key, &[s1, s2], Some(s1)), Some(s2));
        assert_eq!(
            tabs.session_for(&key, &[s1], Some(s1)),
            Some(s1),
            "s2 closed"
        );
        assert_eq!(
            tabs.session_for(&worktree("p", "x"), &[s1, s2], Some(s1)),
            Some(s1),
            "per location"
        );
        assert!(tabs.is_empty(), "remembering makes no tab");
    }

    #[test]
    fn t6_remember_overwrites() {
        let key = default_of("p");
        let (s1, s2, s3) = (session(1), session(2), session(3));
        let mut tabs = ReviewTabs::<u32>::default();
        tabs.remember(key.clone(), s2);
        tabs.remember(key.clone(), s3);
        assert_eq!(tabs.session_for(&key, &[s1, s2, s3], Some(s1)), Some(s3));
    }

    // ---- T8. Faces ----------------------------------------------------------------------

    fn badge(n: usize) -> Option<NonZeroUsize> {
        NonZeroUsize::new(n)
    }

    #[test]
    fn t8_the_review_tab_face_follows_the_table() {
        use Freshness::*;
        let rows: [(
            Option<usize>,
            Freshness,
            bool,
            Option<NonZeroUsize>,
            bool,
            &str,
        ); 9] = [
            (None, Fresh, false, None, false, "Review"),
            (None, Fresh, true, None, false, "Review"),
            (Some(0), Fresh, false, None, false, "Review"),
            (Some(0), Fresh, true, None, false, "Review"),
            (Some(1), Fresh, false, badge(1), false, "Review · 1 draft"),
            (
                Some(3),
                Changed,
                false,
                badge(3),
                true,
                "Review · 3 drafts\nChanged since you looked",
            ),
            (Some(3), Changed, true, badge(3), false, "Review · 3 drafts"),
            (Some(0), Unwatched, false, None, false, "Review"),
            (
                Some(120),
                Fresh,
                false,
                badge(120),
                false,
                "Review · 120 drafts",
            ),
        ];
        for (drafts, freshness, shown, badge, changed, tooltip) in rows {
            let face = ReviewTabFace::of(drafts, freshness, shown);
            let row = format!("{drafts:?} {freshness:?} shown={shown}");
            assert_eq!(face.label, "Review", "{row}");
            assert_eq!(face.badge, badge, "{row}");
            assert_eq!(face.changed, changed, "{row}");
            assert_eq!(face.tooltip, tooltip, "{row}");
        }
    }

    #[test]
    fn t8_a_changed_tab_with_no_drafts_says_only_that_it_changed() {
        let face = ReviewTabFace::of(Some(0), Freshness::Changed, false);
        assert_eq!(face.badge, None);
        assert!(face.changed);
        assert_eq!(face.tooltip, "Review\nChanged since you looked");
    }

    #[test]
    fn t8_badge_text_caps_at_99_plus_and_keeps_the_count() {
        let text = |n| ReviewTabFace::of(Some(n), Freshness::Fresh, false).badge_text();
        assert_eq!(text(0), None);
        assert_eq!(text(1).as_deref(), Some("1"));
        assert_eq!(text(99).as_deref(), Some("99"));
        assert_eq!(text(100).as_deref(), Some("99+"));
        let face = ReviewTabFace::of(Some(120), Freshness::Fresh, false);
        assert_eq!(face.badge_text().as_deref(), Some("99+"));
        assert_eq!(face.badge, badge(120), "the count is kept");
    }

    #[test]
    fn t8_the_changes_button_follows_the_table() {
        let rows = [
            (false, false, Press::Create, false, "Open the review"),
            (false, true, Press::Create, false, "Open the review"),
            (true, false, Press::Show, false, "Show the review"),
            (true, true, Press::None, true, "The review is shown"),
        ];
        for (has_tab, shown, press, selected, tooltip) in rows {
            assert_eq!(
                ChangesButton::of(has_tab, shown),
                ChangesButton {
                    press,
                    selected,
                    tooltip
                },
                "has_tab={has_tab} shown={shown}"
            );
        }
    }
}
