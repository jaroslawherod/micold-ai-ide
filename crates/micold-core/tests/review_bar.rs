//! Feature 1270, T004/T005/T017 [US1, US2]: what the review bar shows — the counts of the shown
//! diff, the send button and the overflow menu's items (FR-002, FR-003, FR-005, FR-006, FR-007;
//! research R2–R4; contract B2, B4, B7).

#[path = "support/review_comments.rs"]
mod review_comments;

use micold_core::review::bar::{
    counts_text, error_text, overflow, send_button, CountsForm, Overflow, ScopeCounts, SendButton,
};
use micold_core::review::comment::{Scope, Status};
use review_comments::{comment, many};

fn counts(draft: usize, sent: usize, applied: usize, answered: usize) -> ScopeCounts {
    ScopeCounts {
        draft,
        sent,
        applied,
        answered,
    }
}

// --- ScopeCounts -------------------------------------------------------------------------------

#[test]
fn counts_take_only_the_shown_scope() {
    let pr = Scope::PullRequest(7);
    let mut comments = many(Scope::WorkingTree, Status::Draft, 3);
    comments.extend(many(Scope::WorkingTree, Status::Sent, 2));
    comments.extend(many(pr, Status::Draft, 1));
    comments.extend(many(pr, Status::Applied, 4));
    comments.extend(many(pr, Status::Answered, 5));
    comments.extend(many(Scope::PullRequest(8), Status::Sent, 6));

    assert_eq!(
        ScopeCounts::of(&comments, Scope::WorkingTree),
        counts(3, 2, 0, 0)
    );
    assert_eq!(ScopeCounts::of(&comments, pr), counts(1, 0, 4, 5));
    assert_eq!(
        ScopeCounts::of(&comments, Scope::PullRequest(8)),
        counts(0, 6, 0, 0)
    );
    assert_eq!(
        ScopeCounts::of(&comments, Scope::PullRequest(9)),
        ScopeCounts::default()
    );
}

#[test]
fn empty_and_reported_or_sent() {
    assert!(ScopeCounts::default().is_empty());
    assert!(!counts(1, 0, 0, 0).is_empty());
    assert!(!counts(0, 0, 0, 1).is_empty());
    assert_eq!(counts(9, 2, 3, 4).reported_or_sent(), 9);
    assert_eq!(counts(9, 0, 0, 0).reported_or_sent(), 0);
}

// --- counts_text --------------------------------------------------------------------------------

#[test]
fn full_counts_with_nothing_say_so() {
    assert_eq!(
        counts_text(&ScopeCounts::default(), CountsForm::Full).as_deref(),
        Some("No review comments yet")
    );
}

#[test]
fn full_counts_list_the_non_zero_statuses_in_order() {
    let text = |c: ScopeCounts| counts_text(&c, CountsForm::Full).unwrap();
    assert_eq!(
        text(counts(3, 2, 1, 1)),
        "3 drafts · 2 sent · 1 applied · 1 answered"
    );
    assert_eq!(text(counts(1, 0, 0, 0)), "1 draft");
    assert_eq!(text(counts(0, 1, 0, 0)), "1 sent");
    assert_eq!(text(counts(0, 0, 2, 0)), "2 applied");
    assert_eq!(text(counts(0, 0, 0, 2)), "2 answered");
    assert_eq!(text(counts(2, 0, 0, 3)), "2 drafts · 3 answered");
    assert_eq!(text(counts(0, 5, 6, 0)), "5 sent · 6 applied");
}

#[test]
fn drafts_only_counts_show_drafts_or_nothing() {
    let text = |c: ScopeCounts| counts_text(&c, CountsForm::DraftsOnly);
    assert_eq!(text(counts(3, 2, 1, 1)).as_deref(), Some("3 drafts"));
    assert_eq!(text(counts(1, 0, 0, 0)).as_deref(), Some("1 draft"));
    assert_eq!(text(counts(0, 2, 1, 1)), None);
    assert_eq!(text(ScopeCounts::default()), None);
}

#[test]
fn hidden_counts_show_nothing() {
    for c in [ScopeCounts::default(), counts(3, 2, 1, 1)] {
        assert_eq!(counts_text(&c, CountsForm::Hidden), None);
    }
}

#[test]
fn the_forms_are_longest_first() {
    let c = counts(3, 2, 1, 1);
    let len = |form| counts_text(&c, form).map_or(0, |t| t.chars().count());
    assert!(len(CountsForm::Full) > len(CountsForm::DraftsOnly));
    assert!(len(CountsForm::DraftsOnly) > len(CountsForm::Hidden));
}

// --- send_button --------------------------------------------------------------------------------

fn button(label: &str, enabled: bool) -> SendButton {
    SendButton {
        label: label.to_owned(),
        enabled,
        primary: enabled,
    }
}

#[test]
fn no_drafts_is_send_review_0_disabled() {
    assert_eq!(
        send_button(&counts(0, 4, 1, 1), false, true),
        button("Send review (0)", false)
    );
}

#[test]
fn drafts_that_can_be_sent_enable_the_primary_button() {
    assert_eq!(
        send_button(&counts(3, 0, 0, 0), false, true),
        button("Send review (3)", true)
    );
    assert_eq!(
        send_button(&counts(999, 0, 0, 0), false, true),
        button("Send review (999)", true)
    );
}

#[test]
fn a_thousand_drafts_cap_the_label() {
    assert_eq!(
        send_button(&counts(1_000, 0, 0, 0), false, true),
        button("Send review (999+)", true)
    );
}

#[test]
fn a_send_under_way_reads_sending_and_is_disabled() {
    for c in [
        ScopeCounts::default(),
        counts(3, 2, 1, 1),
        counts(1_000, 0, 0, 0),
    ] {
        for sendable in [true, false] {
            assert_eq!(send_button(&c, true, sendable), button("Sending…", false));
        }
    }
}

#[test]
fn a_pull_request_not_in_the_list_cannot_send() {
    assert_eq!(
        send_button(&counts(3, 0, 0, 0), false, false),
        button("Send review (3)", false)
    );
}

#[test]
fn primary_follows_enabled_in_every_case() {
    for draft in [0, 1, 1_000] {
        for sending in [false, true] {
            for sendable in [false, true] {
                let b = send_button(&counts(draft, 1, 0, 0), sending, sendable);
                assert_eq!(b.primary, b.enabled, "{draft} {sending} {sendable}");
                assert_eq!(b.enabled, draft > 0 && !sending && sendable);
            }
        }
    }
}

#[test]
fn the_error_wording_is_todays() {
    assert_eq!(error_text("why"), "Couldn't send the review: why");
}

// --- overflow -----------------------------------------------------------------------------------

#[test]
fn clear_sent_needs_a_sent_or_reported_comment() {
    for c in [
        counts(0, 1, 0, 0),
        counts(0, 0, 1, 0),
        counts(0, 0, 0, 1),
        counts(2, 1, 1, 1),
    ] {
        assert!(overflow(&c, false).clear_sent, "{c:?}");
        assert!(overflow(&c, true).clear_sent, "{c:?} while sending");
    }
    for c in [ScopeCounts::default(), counts(4, 0, 0, 0)] {
        assert!(!overflow(&c, false).clear_sent, "{c:?}");
    }
}

#[test]
fn discard_drafts_needs_drafts_and_no_send() {
    assert!(overflow(&counts(1, 0, 0, 0), false).discard_drafts);
    assert!(!overflow(&counts(1, 0, 0, 0), true).discard_drafts);
    assert!(!overflow(&counts(0, 3, 1, 1), false).discard_drafts);
    assert_eq!(
        overflow(&ScopeCounts::default(), false),
        Overflow {
            clear_sent: false,
            discard_drafts: false,
        }
    );
}

#[test]
fn the_overflow_of_a_pull_request_counts_only_its_comments() {
    let shown = Scope::PullRequest(3);
    let comments = vec![
        comment(Scope::WorkingTree, Status::Sent, "a", 1, 1),
        comment(Scope::WorkingTree, Status::Draft, "a", 2, 2),
        comment(Scope::PullRequest(4), Status::Applied, "a", 3, 3),
    ];
    let c = ScopeCounts::of(&comments, shown);
    assert_eq!(
        overflow(&c, false),
        Overflow {
            clear_sent: false,
            discard_drafts: false,
        }
    );
}
