//! The window's Back/Forward history (feature 1465, data-model.md): the places the user has been,
//! a position in them, and the rules for recording and walking them. Render-free and generic over
//! the place, so the list rules are tested here with plain values (research R4).

use std::path::PathBuf;

use crate::runs::GroupId;
use crate::session::{SessionId, SessionLocation};

/// The most entries a history holds (FR-005, SC-004).
pub const CAP: usize = 50;

/// The direction of a Back/Forward press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// Towards earlier entries.
    Back,
    /// Towards later entries.
    Forward,
}

/// What the main area shows in a project (spec *Key Entities*, research R3). The Overview and
/// Compare are project-wide views, so they carry no location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    /// A session (structured or terminal) of a location.
    Session {
        location: SessionLocation,
        id: SessionId,
    },
    /// The Review tab of a location.
    Review(SessionLocation),
    /// The worktree Overview.
    Overview,
    /// The Compare view of a run group.
    Compare(GroupId),
}

/// A place the user has been: a view of a project. Two places are equal when what is shown is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub project: PathBuf,
    pub view: View,
}

/// The entries and the position in them.
///
/// Invariants: `entries.len() <= CAP`; no two neighbouring entries are equal; `pos <
/// entries.len()` when non-empty, `0` when empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History<P> {
    entries: Vec<P>,
    pos: usize,
}

impl<P> Default for History<P> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            pos: 0,
        }
    }
}

impl<P: Clone + PartialEq> History<P> {
    /// The entry at the position, if any.
    pub fn current(&self) -> Option<&P> {
        self.entries.get(self.pos)
    }

    /// Every entry, oldest first.
    pub fn entries(&self) -> &[P] {
        &self.entries
    }

    /// The index of the current entry (`0` when empty).
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Records a move to `place` (FR-002–FR-005): nothing when it equals the current entry; else
    /// every entry after the position is dropped, `place` is pushed and becomes current, and the
    /// oldest entry is dropped past [`CAP`].
    pub fn record(&mut self, place: P) {
        if self.current() == Some(&place) {
            return;
        }
        if !self.entries.is_empty() {
            self.entries.truncate(self.pos + 1);
        }
        self.entries.push(place);
        if self.entries.len() > CAP {
            self.entries.remove(0);
        }
        self.pos = self.entries.len() - 1;
    }

    /// Walks from the position towards `dir` (FR-006, FR-007, US4): an entry `exists` rejects is
    /// removed; an entry equal to `shown` is passed over and kept. The first other entry becomes
    /// current and is returned. None found: the position stays on its entry and `None` is
    /// returned. Removals merge neighbouring equal entries. Never records.
    pub fn step(
        &mut self,
        dir: Dir,
        shown: Option<&P>,
        mut exists: impl FnMut(&P) -> bool,
    ) -> Option<P> {
        if self.entries.is_empty() {
            return None;
        }
        let mut at = self.pos;
        let mut removed = false;
        let mut found = None;
        loop {
            let next = match dir {
                Dir::Back if at == 0 => break,
                Dir::Back => at - 1,
                Dir::Forward if at + 1 >= self.entries.len() => break,
                Dir::Forward => at + 1,
            };
            if !exists(&self.entries[next]) {
                self.entries.remove(next);
                removed = true;
                if next < self.pos {
                    self.pos -= 1;
                }
                if dir == Dir::Back {
                    at -= 1;
                }
                continue;
            }
            if shown == Some(&self.entries[next]) {
                at = next;
                continue;
            }
            found = Some(next);
            break;
        }
        if let Some(index) = found {
            self.pos = index;
        }
        if removed {
            self.merge_neighbours();
        }
        found.map(|_| self.entries[self.pos].clone())
    }

    /// Merges each run of equal neighbours into one, keeping the position on the merged entry.
    fn merge_neighbours(&mut self) {
        let mut k = 1;
        while k < self.entries.len() {
            if self.entries[k] == self.entries[k - 1] {
                self.entries.remove(k);
                if k <= self.pos {
                    self.pos -= 1;
                }
            } else {
                k += 1;
            }
        }
        if self.entries.is_empty() {
            self.pos = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled(items: &[u32]) -> History<u32> {
        let mut h = History::default();
        for &i in items {
            h.record(i);
        }
        h
    }

    fn all(_: &u32) -> bool {
        true
    }

    fn assert_invariants(h: &History<u32>) {
        assert!(h.entries().len() <= CAP, "never over the cap");
        assert!(
            h.entries().windows(2).all(|w| w[0] != w[1]),
            "no two neighbouring entries equal: {:?}",
            h.entries()
        );
        if h.entries().is_empty() {
            assert_eq!(h.position(), 0, "an empty history sits at 0");
        } else {
            assert!(h.position() < h.entries().len(), "the position is an entry");
        }
    }

    #[test]
    fn recording_the_current_entry_adds_nothing() {
        let mut h = filled(&[1, 2]);
        h.record(2);
        assert_eq!(h.entries(), &[1, 2], "FR-003: equal to current adds nothing");
        assert_eq!(h.position(), 1);
    }

    #[test]
    fn recording_after_back_drops_the_forward_entries() {
        let mut h = filled(&[1, 2, 3]);
        assert_eq!(h.step(Dir::Back, Some(&3), all), Some(2));
        assert_eq!(h.step(Dir::Back, Some(&2), all), Some(1));
        h.record(4);
        assert_eq!(h.entries(), &[1, 4], "US1 AS3, FR-004: B and C dropped");
        assert_eq!(h.step(Dir::Forward, Some(&4), all), None, "forward is empty");
        assert_eq!(h.step(Dir::Back, Some(&4), all), Some(1), "back returns to A");
    }

    #[test]
    fn sixty_records_keep_the_newest_fifty() {
        let mut h = History::default();
        for i in 0..60u32 {
            h.record(i);
        }
        assert_eq!(h.entries().len(), CAP, "FR-005, SC-004");
        assert_eq!(h.entries()[0], 10, "the oldest ten dropped");
        assert_eq!(h.current(), Some(&59));
        let mut shown = 59;
        while let Some(p) = h.step(Dir::Back, Some(&shown), all) {
            shown = p;
        }
        assert_eq!(shown, 10, "Back stops at the oldest kept entry");
        assert_eq!(h.position(), 0);
        assert_invariants(&h);
    }

    #[test]
    fn stepping_past_either_end_changes_nothing() {
        let mut h = filled(&[1, 2, 3]);
        assert_eq!(h.step(Dir::Forward, Some(&3), all), None, "US1 AS6: at the last");
        assert_eq!(h.position(), 2);
        h.step(Dir::Back, Some(&3), all);
        h.step(Dir::Back, Some(&2), all);
        assert_eq!(h.step(Dir::Back, Some(&1), all), None, "US1 AS6: at the first");
        assert_eq!(h.position(), 0);
        assert_eq!(h.entries(), &[1, 2, 3]);
        let mut empty = History::<u32>::default();
        assert_eq!(empty.step(Dir::Back, None, all), None, "an empty history");
    }

    #[test]
    fn a_gone_entry_is_skipped_and_removed() {
        let mut h = filled(&[1, 2, 3]);
        assert_eq!(h.step(Dir::Back, Some(&3), |p| *p != 2), Some(1), "US4 AS1");
        assert_eq!(h.entries(), &[1, 3], "FR-007: B removed");
        assert_eq!(h.position(), 0);
    }

    #[test]
    fn every_entry_behind_gone_does_nothing_and_removes_them() {
        let mut h = filled(&[1, 2, 3]);
        assert_eq!(h.step(Dir::Back, Some(&3), |p| *p == 3), None, "US4 AS3");
        assert_eq!(h.entries(), &[3], "the gone entries removed");
        assert_eq!(h.position(), 0, "still on C");
    }

    #[test]
    fn removal_merges_neighbouring_equal_entries() {
        let mut h = filled(&[1, 2, 1, 3]);
        let gone = |p: &u32| *p != 2;
        assert_eq!(h.step(Dir::Back, Some(&3), gone), Some(1), "Back from C lands on A");
        assert_eq!(
            h.step(Dir::Back, Some(&1), gone),
            None,
            "A is not landed on twice"
        );
        assert_eq!(h.entries(), &[1, 3], "B removed and the As merged");
        assert_eq!(h.position(), 0, "the position on the merged A");
    }

    #[test]
    fn an_entry_equal_to_shown_is_passed_over_and_kept() {
        let mut h = filled(&[1, 2, 3]);
        h.step(Dir::Back, Some(&3), all);
        h.record(1);
        // [1, 2, 1] at 2; shown is 2 (as when the shown place was not recorded yet)
        assert_eq!(h.step(Dir::Back, Some(&2), all), Some(1), "US4 AS4");
        assert_eq!(h.entries(), &[1, 2, 1], "the shown entry kept");
        assert_eq!(h.position(), 0);
    }

    #[test]
    fn step_never_records() {
        let mut h = filled(&[1, 2, 3]);
        h.step(Dir::Back, Some(&9), all);
        h.step(Dir::Forward, Some(&9), all);
        assert_eq!(h.entries(), &[1, 2, 3], "FR-007: moving records nothing");
    }

    #[test]
    fn places_are_equal_when_what_is_shown_is() {
        let p = |view| Place {
            project: PathBuf::from("/p"),
            view,
        };
        let mut h = History::default();
        h.record(p(View::Overview));
        h.record(p(View::Overview));
        h.record(Place {
            project: PathBuf::from("/q"),
            view: View::Overview,
        });
        h.record(p(View::Review(SessionLocation::Default)));
        assert_eq!(h.entries().len(), 3, "same project and view is one entry");
    }

    #[test]
    fn every_step_lands_on_an_existing_other_entry_or_does_nothing() {
        // SC-002: every subset of gone entries in a five-entry list, every position, both ways.
        for gone_mask in 0u32..32 {
            for start in 0..5usize {
                for dir in [Dir::Back, Dir::Forward] {
                    let mut h = filled(&[0, 1, 2, 3, 4]);
                    for _ in start..4 {
                        let shown = *h.current().unwrap();
                        h.step(Dir::Back, Some(&shown), all);
                    }
                    assert_eq!(h.position(), start);
                    let shown = *h.current().unwrap();
                    let exists = |p: &u32| gone_mask & (1 << p) == 0;
                    let got = h.step(dir, Some(&shown), exists);
                    if let Some(p) = got {
                        assert!(exists(&p), "lands on an existing entry");
                        assert_ne!(p, shown, "never on the place shown");
                        assert_eq!(h.current(), Some(&p), "the position is on it");
                    } else {
                        assert_eq!(h.current(), Some(&shown), "nothing moved");
                    }
                    assert_invariants(&h);
                }
            }
        }
    }
}
