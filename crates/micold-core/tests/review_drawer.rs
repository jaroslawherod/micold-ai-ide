//! Feature 1270, T023/T024 [US3]: the comments drawer's rules — which tab holds which comment, the
//! tabs' labels, counts and empty texts, row order, width, and the drawer's state machine with Esc
//! and auto-open (FR-008, FR-010–FR-014, FR-018–FR-020, SC-006; research R5, R6, R9; contract
//! D1–D5, D9–D11).

#[path = "support/review_comments.rs"]
mod review_comments;

use std::collections::BTreeSet;

use micold_core::review::comment::{CommentId, ReviewComment, Scope, Status};
use micold_core::review::drawer::{
    newly_reported, rows_of, tab_counts, width, Drawer, DrawerEvent, DrawerTab, SendWatch,
    START_HINT,
};
use micold_core::review::Side;
use review_comments::{comment, many, state, STATUSES};

// --- DrawerTab ----------------------------------------------------------------------------------

#[test]
fn drafts_hold_drafts_and_replies_hold_the_rest() {
    assert_eq!(DrawerTab::of(Status::Draft), DrawerTab::Drafts);
    for status in [Status::Sent, Status::Applied, Status::Answered] {
        assert_eq!(DrawerTab::of(status), DrawerTab::Replies, "{status:?}");
    }
}

#[test]
fn tab_labels_carry_their_count() {
    assert_eq!(DrawerTab::Drafts.label(3), "Drafts 3");
    assert_eq!(DrawerTab::Replies.label(4), "Replies 4");
    assert_eq!(DrawerTab::Drafts.label(0), "Drafts 0");
}

#[test]
fn empty_tabs_say_so() {
    assert_eq!(DrawerTab::Drafts.empty_text(), "No drafts");
    assert_eq!(DrawerTab::Replies.empty_text(), "No replies yet");
    assert_eq!(START_HINT, "Click a line number in the diff to write one.");
}

#[test]
fn tab_counts_cover_every_scope_of_the_entry() {
    let mut comments = many(Scope::WorkingTree, Status::Draft, 2);
    comments.extend(many(Scope::PullRequest(1), Status::Draft, 1));
    comments.extend(many(Scope::WorkingTree, Status::Sent, 1));
    comments.extend(many(Scope::PullRequest(1), Status::Applied, 2));
    comments.extend(many(Scope::PullRequest(2), Status::Answered, 1));
    assert_eq!(tab_counts(&comments), (3, 4));
    assert_eq!(tab_counts(&[]), (0, 0));
}

/// SC-006: over all four statuses and both scopes, every comment is in exactly one tab.
#[test]
fn every_comment_is_in_exactly_one_tab() {
    let line_of = |c: &ReviewComment| c.range.start();
    let mut comments = Vec::new();
    for (i, scope) in [Scope::WorkingTree, Scope::PullRequest(5)]
        .into_iter()
        .enumerate()
    {
        for (j, status) in STATUSES.into_iter().enumerate() {
            comments.push(comment(scope, status, "f", 1 + j as u32, i as u64));
        }
    }
    let drafts = rows_of(&comments, DrawerTab::Drafts, line_of);
    let replies = rows_of(&comments, DrawerTab::Replies, line_of);
    assert_eq!(drafts.len() + replies.len(), comments.len());
    for c in &comments {
        let in_drafts = drafts.iter().any(|r| r.id == c.id);
        let in_replies = replies.iter().any(|r| r.id == c.id);
        assert!(in_drafts != in_replies, "{:?} in both or neither", c.state);
        assert_eq!(in_drafts, c.state.status() == Status::Draft);
    }
    assert_eq!(tab_counts(&comments), (drafts.len(), replies.len()));
}

#[test]
fn rows_are_ordered_by_scope_path_line_side_and_creation() {
    let pr = |n| Scope::PullRequest(n);
    let mut old_side = comment(Scope::WorkingTree, Status::Sent, "b", 4, 0);
    old_side.side = Side::Old;
    let comments = vec![
        comment(pr(9), Status::Sent, "a", 1, 0),
        comment(pr(2), Status::Answered, "z", 1, 0),
        old_side,
        comment(Scope::WorkingTree, Status::Applied, "b", 4, 7),
        comment(Scope::WorkingTree, Status::Sent, "b", 4, 3),
        comment(Scope::WorkingTree, Status::Sent, "b", 10, 0),
        comment(Scope::WorkingTree, Status::Sent, "B", 99, 0),
        comment(Scope::WorkingTree, Status::Draft, "a", 1, 0),
    ];
    // The shown line comes from the caller: here, twice the stored start.
    let rows = rows_of(&comments, DrawerTab::Replies, |c| c.range.start() * 2);
    let order: Vec<(Scope, &str, u32, Side, u64)> = rows
        .iter()
        .map(|c| {
            (
                c.scope(),
                c.path.as_str(),
                c.range.start(),
                c.side,
                c.created,
            )
        })
        .collect();
    assert_eq!(
        order,
        vec![
            (Scope::WorkingTree, "B", 99, Side::New, 0),
            (Scope::WorkingTree, "b", 4, Side::New, 3),
            (Scope::WorkingTree, "b", 4, Side::New, 7),
            (Scope::WorkingTree, "b", 4, Side::Old, 0),
            (Scope::WorkingTree, "b", 10, Side::New, 0),
            (pr(2), "z", 1, Side::New, 0),
            (pr(9), "a", 1, Side::New, 0),
        ]
    );
}

#[test]
fn rows_order_by_the_shown_line_not_the_stored_one() {
    let comments = vec![
        comment(Scope::WorkingTree, Status::Draft, "a", 1, 0),
        comment(Scope::WorkingTree, Status::Draft, "a", 2, 0),
    ];
    let rows = rows_of(&comments, DrawerTab::Drafts, |c| 10 - c.range.start());
    assert_eq!(rows[0].range.start(), 2);
}

#[test]
fn width_is_360_but_at_most_half_the_area() {
    assert_eq!(width(1_000.0), 360.0);
    assert_eq!(width(720.0), 360.0);
    assert_eq!(width(500.0), 250.0);
}

// --- Drawer -------------------------------------------------------------------------------------

fn ids(n: usize) -> BTreeSet<CommentId> {
    (0..n).map(|_| CommentId::new()).collect()
}

fn watching(ids: &BTreeSet<CommentId>) -> Option<SendWatch> {
    Some(SendWatch { ids: ids.clone() })
}

fn open() -> Drawer {
    let mut d = Drawer::default();
    assert!(d.apply(DrawerEvent::Toggle));
    d
}

#[test]
fn a_new_drawer_is_closed_on_drafts_without_a_watch() {
    let d = Drawer::default();
    assert!(!d.open);
    assert_eq!(d.tab, DrawerTab::Drafts);
    assert_eq!(d.offsets, [0, 0]);
    assert_eq!(d.watch, None);
}

#[test]
fn toggle_opens_and_closes_and_closing_drops_the_watch() {
    let sent = ids(2);
    let mut d = Drawer::default();
    assert!(d.apply(DrawerEvent::SendStarted(sent.clone())));
    assert!(d.apply(DrawerEvent::Toggle));
    assert!(d.open);
    assert_eq!(d.watch, watching(&sent), "opening keeps the watch");
    assert!(d.apply(DrawerEvent::Toggle));
    assert!(!d.open);
    assert_eq!(d.watch, None);
}

#[test]
fn close_closes_and_drops_the_watch() {
    let mut d = open();
    d.apply(DrawerEvent::SendStarted(ids(1)));
    assert!(d.apply(DrawerEvent::Close));
    assert!(!d.open);
    assert_eq!(d.watch, None);
}

#[test]
fn escape_closes_an_open_drawer_and_is_unused_when_closed() {
    let mut d = open();
    d.apply(DrawerEvent::SendStarted(ids(1)));
    assert!(d.apply(DrawerEvent::Escape));
    assert!(!d.open);
    assert_eq!(d.watch, None);
    assert!(
        !d.apply(DrawerEvent::Escape),
        "Esc on a closed drawer is not consumed"
    );
}

#[test]
fn escape_on_a_closed_drawer_keeps_a_pending_watch() {
    let sent = ids(1);
    let mut d = Drawer::default();
    d.apply(DrawerEvent::SendStarted(sent.clone()));
    assert!(!d.apply(DrawerEvent::Escape));
    assert_eq!(d.watch, watching(&sent));
}

#[test]
fn select_tab_and_per_tab_scroll() {
    let mut d = open();
    assert!(d.apply(DrawerEvent::SelectTab(DrawerTab::Replies)));
    assert_eq!(d.tab, DrawerTab::Replies);
    assert!(!d.apply(DrawerEvent::SelectTab(DrawerTab::Replies)));
    assert!(d.apply(DrawerEvent::Scrolled {
        tab: DrawerTab::Replies,
        offset: 120,
    }));
    assert!(d.apply(DrawerEvent::Scrolled {
        tab: DrawerTab::Drafts,
        offset: 40,
    }));
    assert_eq!(d.offsets, [40, 120]);
    assert_eq!(d.offset(DrawerTab::Replies), 120);
    assert!(d.apply(DrawerEvent::SelectTab(DrawerTab::Drafts)));
    assert_eq!(
        d.offset(DrawerTab::Drafts),
        40,
        "each tab keeps its own offset"
    );
    assert!(!d.apply(DrawerEvent::Scrolled {
        tab: DrawerTab::Drafts,
        offset: 40,
    }));
}

#[test]
fn a_new_send_replaces_the_watch_and_a_failed_one_drops_it() {
    let (first, second) = (ids(1), ids(2));
    let mut d = Drawer::default();
    d.apply(DrawerEvent::SendStarted(first));
    assert!(d.apply(DrawerEvent::SendStarted(second.clone())));
    assert_eq!(d.watch, watching(&second));
    assert!(d.apply(DrawerEvent::SendFailed));
    assert_eq!(d.watch, None);
    assert!(!d.apply(DrawerEvent::SendFailed));
}

#[test]
fn the_first_report_of_the_watched_send_opens_on_replies_once() {
    let sent = ids(3);
    let one: BTreeSet<_> = sent.iter().take(1).copied().collect();
    let other: BTreeSet<_> = sent.iter().skip(1).copied().collect();
    let mut d = Drawer::default();
    d.apply(DrawerEvent::SendStarted(sent));
    assert!(d.apply(DrawerEvent::Reported {
        ids: one,
        shown: true,
    }));
    assert!(d.open);
    assert_eq!(d.tab, DrawerTab::Replies);
    assert_eq!(d.watch, None, "consumed on the first report");

    d.apply(DrawerEvent::Close);
    assert!(!d.apply(DrawerEvent::Reported {
        ids: other,
        shown: true,
    }));
    assert!(!d.open, "a second report of the same send does not reopen");
}

#[test]
fn a_report_while_the_tab_is_not_shown_consumes_the_watch() {
    let sent = ids(1);
    let mut d = Drawer::default();
    d.apply(DrawerEvent::SendStarted(sent.clone()));
    assert!(d.apply(DrawerEvent::Reported {
        ids: sent.clone(),
        shown: false,
    }));
    assert!(!d.open);
    assert_eq!(d.watch, None);
    assert!(!d.apply(DrawerEvent::Reported {
        ids: sent,
        shown: true,
    }));
    assert!(!d.open);
}

#[test]
fn a_drawer_closed_since_the_send_stays_closed() {
    let sent = ids(1);
    let mut d = open();
    d.apply(DrawerEvent::SendStarted(sent.clone()));
    d.apply(DrawerEvent::Toggle);
    assert!(!d.apply(DrawerEvent::Reported {
        ids: sent,
        shown: true,
    }));
    assert!(!d.open);
}

#[test]
fn an_open_drawer_keeps_its_tab() {
    let sent = ids(1);
    let mut d = open();
    d.apply(DrawerEvent::SendStarted(sent.clone()));
    assert!(d.apply(DrawerEvent::Reported {
        ids: sent,
        shown: true,
    }));
    assert!(d.open);
    assert_eq!(d.tab, DrawerTab::Drafts);
    assert_eq!(d.watch, None);
}

#[test]
fn a_report_of_another_send_changes_nothing() {
    let sent = ids(1);
    let mut d = Drawer::default();
    d.apply(DrawerEvent::SendStarted(sent.clone()));
    let before = d.clone();
    assert!(!d.apply(DrawerEvent::Reported {
        ids: ids(2),
        shown: true,
    }));
    assert_eq!(d, before);
    assert!(!d.apply(DrawerEvent::Reported {
        ids: BTreeSet::new(),
        shown: true,
    }));
    assert_eq!(d, before);
}

// --- newly_reported -----------------------------------------------------------------------------

fn with_status(c: &ReviewComment, status: Status) -> ReviewComment {
    ReviewComment {
        state: state(status),
        ..c.clone()
    }
}

#[test]
fn a_report_of_a_draft_or_sent_comment_is_new() {
    let draft = comment(Scope::WorkingTree, Status::Draft, "a", 1, 0);
    let sent = comment(Scope::PullRequest(2), Status::Sent, "a", 2, 0);
    let still = comment(Scope::WorkingTree, Status::Sent, "a", 3, 0);
    let before = vec![draft.clone(), sent.clone(), still.clone()];
    let after = vec![
        with_status(&draft, Status::Applied),
        with_status(&sent, Status::Answered),
        still,
    ];
    assert_eq!(
        newly_reported(&before, &after),
        [draft.id, sent.id].into_iter().collect()
    );
}

#[test]
fn a_report_of_a_comment_not_there_before_is_new() {
    let c = comment(Scope::WorkingTree, Status::Answered, "a", 1, 0);
    assert_eq!(
        newly_reported(&[], &[c.clone()]),
        [c.id].into_iter().collect()
    );
}

#[test]
fn a_report_replacing_a_report_is_not_new() {
    let applied = comment(Scope::WorkingTree, Status::Applied, "a", 1, 0);
    let after = vec![with_status(&applied, Status::Answered)];
    assert!(newly_reported(&[applied.clone()], &after).is_empty());
    assert!(newly_reported(&[applied.clone()], &[applied]).is_empty());
}
