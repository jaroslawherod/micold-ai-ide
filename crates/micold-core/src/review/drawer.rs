//! The comments drawer over the diff (feature 1270, data model § Core: `review::drawer`): which
//! tab holds which comment, the tabs' labels, counts and empty texts, row order, the drawer's
//! width, and its state machine with Esc and auto-open, decided here without drawing (FR-008,
//! FR-010–FR-014, FR-018–FR-020, FR-023, SC-006).
//! See `specs/1270-review-bar-drawer/data-model.md`.

use std::collections::{BTreeMap, BTreeSet};

use super::comment::{CommentId, ReviewComment, Status};
use crate::tokens::anatomy;

/// The drawer's two tabs (FR-013).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DrawerTab {
    /// The draft comments.
    #[default]
    Drafts,
    /// The sent, applied and answered comments.
    Replies,
}

impl DrawerTab {
    /// The tab a comment with `status` is listed in: every comment is in exactly one (SC-006).
    pub fn of(status: Status) -> Self {
        match status {
            Status::Draft => Self::Drafts,
            Status::Sent | Status::Applied | Status::Answered => Self::Replies,
        }
    }

    /// The tab's label with its row count: `"Drafts 3"`, `"Replies 4"` (US3 #7).
    pub fn label(self, n: usize) -> String {
        match self {
            Self::Drafts => format!("Drafts {n}"),
            Self::Replies => format!("Replies {n}"),
        }
    }

    /// What the tab says when it has no rows (FR-018).
    pub fn empty_text(self) -> &'static str {
        match self {
            Self::Drafts => "No drafts",
            Self::Replies => "No replies yet",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Drafts => 0,
            Self::Replies => 1,
        }
    }
}

/// Today's hint on writing a comment, shown under "No drafts" when the entry has no comments at
/// all (Edge Cases "No comments at all").
pub const START_HINT: &str = "Click a line number in the diff to write one.";

/// The (drafts, replies) row counts over every scope of the entry.
pub fn tab_counts(comments: &[ReviewComment]) -> (usize, usize) {
    let drafts = comments
        .iter()
        .filter(|c| DrawerTab::of(c.state.status()) == DrawerTab::Drafts)
        .count();
    (drafts, comments.len() - drafts)
}

/// `tab`'s comments ordered by scope (working tree first, then pull requests by number), path
/// bytewise, `line_of` (the client's shown range start), side, then creation (FR-014).
pub fn rows_of<'a>(
    comments: &'a [ReviewComment],
    tab: DrawerTab,
    line_of: impl Fn(&ReviewComment) -> u32,
) -> Vec<&'a ReviewComment> {
    let mut rows: Vec<_> = comments
        .iter()
        .filter(|c| DrawerTab::of(c.state.status()) == tab)
        .map(|c| {
            (
                (c.scope(), c.path.as_str(), line_of(c), c.side, c.created),
                c,
            )
        })
        .collect();
    rows.sort_by(|(a, _), (b, _)| a.cmp(b));
    rows.into_iter().map(|(_, c)| c).collect()
}

/// The drawer's width over a diff area `area` px wide: 360, but at most half (FR-010, Edge Cases
/// "Narrow window").
pub fn width(area: f32) -> f32 {
    anatomy::comments_drawer::WIDTH.min(area / 2.0).max(0.0)
}

/// A send this window started and has not yet seen a report of (research R9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendWatch {
    /// The drafts the send carried.
    pub ids: BTreeSet<CommentId>,
}

/// The drawer of one entry's review view (FR-011, FR-019): a plain value, so it lives and dies
/// with that view.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Drawer {
    /// Whether it is drawn; closed when the view is created (FR-011).
    pub open: bool,
    /// The selected tab.
    pub tab: DrawerTab,
    /// The scroll offset of each tab in px, indexed Drafts then Replies (FR-019, research R6).
    pub offsets: [u32; 2],
    /// The send whose first report opens the drawer (FR-020).
    pub watch: Option<SendWatch>,
}

/// What happens to a drawer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawerEvent {
    /// The bar's **Comments** toggle: open ↔ closed (FR-008).
    Toggle,
    /// The drawer's ×.
    Close,
    /// Esc with nothing open above the drawer (FR-012).
    Escape,
    /// A tab was chosen.
    SelectTab(DrawerTab),
    /// A tab's list was scrolled to `offset` px.
    Scrolled {
        /// The tab scrolled.
        tab: DrawerTab,
        /// Its new offset.
        offset: u32,
    },
    /// This window started a send of these drafts; it replaces any earlier watch.
    SendStarted(BTreeSet<CommentId>),
    /// This window's send failed.
    SendFailed,
    /// These comments were newly reported ([`newly_reported`]); `shown` is whether this entry's
    /// Review tab is shown.
    Reported {
        /// The newly reported comments.
        ids: BTreeSet<CommentId>,
        /// Whether the entry's Review tab is shown.
        shown: bool,
    },
}

impl Drawer {
    /// `tab`'s scroll offset.
    pub fn offset(&self, tab: DrawerTab) -> u32 {
        self.offsets[tab.index()]
    }

    /// Applies `event` and says whether anything changed. Closing drops the watch, so a drawer
    /// closed since a send never opens itself for it; Esc on a closed drawer changes nothing and
    /// returns `false`, so the caller knows Esc was not used.
    pub fn apply(&mut self, event: DrawerEvent) -> bool {
        let before = self.clone();
        match event {
            DrawerEvent::Toggle if self.open => self.close(),
            DrawerEvent::Toggle => self.open = true,
            DrawerEvent::Close | DrawerEvent::Escape => {
                if self.open {
                    self.close();
                }
            }
            DrawerEvent::SelectTab(tab) => self.tab = tab,
            DrawerEvent::Scrolled { tab, offset } => self.offsets[tab.index()] = offset,
            DrawerEvent::SendStarted(ids) => self.watch = Some(SendWatch { ids }),
            DrawerEvent::SendFailed => self.watch = None,
            DrawerEvent::Reported { ids, shown } => {
                let hit = self
                    .watch
                    .as_ref()
                    .is_some_and(|watch| !watch.ids.is_disjoint(&ids));
                if hit {
                    self.watch = None;
                    if shown && !self.open {
                        self.open = true;
                        self.tab = DrawerTab::Replies;
                    }
                }
            }
        }
        *self != before
    }

    fn close(&mut self) {
        self.open = false;
        self.watch = None;
    }
}

/// The comments of `after` that are Applied or Answered and were Draft or Sent in `before`, or
/// not in it at all; a report replacing an earlier report is not new (research R9).
pub fn newly_reported(before: &[ReviewComment], after: &[ReviewComment]) -> BTreeSet<CommentId> {
    let was: BTreeMap<CommentId, Status> =
        before.iter().map(|c| (c.id, c.state.status())).collect();
    let reported = |status: Status| matches!(status, Status::Applied | Status::Answered);
    after
        .iter()
        .filter(|c| reported(c.state.status()))
        .filter(|c| !was.get(&c.id).copied().is_some_and(reported))
        .map(|c| c.id)
        .collect()
}
