//! When the window reads pull request status, in isolation (feature 040, T023,
//! contracts/reading-and-wire.md §2, data-model §3).
//!
//! Builds only `features::pr_status::State` and drives `update` with the messages the shell sends.
//! Every case names its behaviour from `tdd/test-list.md` (U70–U84).

use std::collections::{BTreeMap, BTreeSet};

use micold_client::features::pr_status::{self, Effect, Msg, Outcome, Phase, State};
use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReadingFailure, ReviewState};

const NOW: u64 = 1_000;
const LATER: u64 = 1_100;
const UNTIL: u64 = 5_000;

fn status(number: u64) -> PullRequestStatus {
    PullRequestStatus {
        number,
        title: format!("Pull request {number}"),
        url: format!("https://github.com/o/r/pull/{number}"),
        state: PrState::Open {
            checks: CheckStatus::Passing,
        },
        review: ReviewState::None,
        head: "a".repeat(40),
    }
}

fn statuses() -> BTreeMap<String, PullRequestStatus> {
    BTreeMap::from([("feat/a".to_string(), status(7))])
}

fn ok(started_at: u64) -> Outcome {
    Outcome::Ok {
        statuses: statuses(),
        removable: BTreeSet::new(),
        started_at,
    }
}

/// Switch on, not held.
fn enabled() -> State {
    let mut st = State::default();
    assert_eq!(update(&mut st, enable()), Effect::None);
    st
}

fn update(st: &mut State, msg: Msg) -> Effect {
    pr_status::update(st, msg)
}

fn enable() -> Msg {
    Msg::EnabledChanged {
        enabled: true,
        now: NOW,
    }
}

fn listing() -> Msg {
    Msg::ListingArrived { now: NOW }
}

/// Switch on, held and listed: the first reading has started. Returns its `seq`.
fn reading(st: &mut State) -> u64 {
    update(st, Msg::Held);
    match update(st, listing()) {
        Effect::Read { seq } => seq,
        Effect::None => panic!("held and listed with the switch on starts a reading"),
    }
}

/// A state that holds statuses from a finished reading.
fn holding_statuses() -> State {
    let mut st = enabled();
    let seq = reading(&mut st);
    update(
        &mut st,
        Msg::Finished {
            seq,
            outcome: ok(NOW),
            now: LATER,
        },
    );
    assert_eq!(st.statuses, statuses());
    st
}

fn finish(st: &mut State, seq: u64, outcome: Outcome) -> Effect {
    update(
        st,
        Msg::Finished {
            seq,
            outcome,
            now: LATER,
        },
    )
}

// ---- U70, U84: the listing after `Held` ----

#[test]
fn held_then_listing_reads_once_and_a_second_listing_reads_nothing() {
    let mut st = enabled();
    assert_eq!(update(&mut st, Msg::Held), Effect::None);
    assert!(matches!(update(&mut st, listing()), Effect::Read { .. }));
    assert_eq!(update(&mut st, listing()), Effect::None);
}

#[test]
fn held_then_listing_with_the_switch_off_reads_nothing() {
    let mut st = State::default();
    update(&mut st, Msg::Held);
    assert_eq!(update(&mut st, listing()), Effect::None);
    assert_eq!(st.phase, Phase::Idle);
}

// ---- U71–U73: the switch ----

#[test]
fn switching_on_while_held_and_listed_reads() {
    let mut st = State::default();
    update(&mut st, Msg::Held);
    update(&mut st, listing());
    assert!(matches!(update(&mut st, enable()), Effect::Read { .. }));
    assert!(st.enabled);
}

#[test]
fn switching_on_while_not_held_or_awaiting_the_listing_reads_nothing() {
    let mut not_held = State::default();
    assert_eq!(update(&mut not_held, enable()), Effect::None);
    assert!(not_held.enabled);

    let mut awaiting = State::default();
    update(&mut awaiting, Msg::Held);
    assert_eq!(update(&mut awaiting, enable()), Effect::None);
    assert!(awaiting.enabled);
    // S1 follows: the listing reads.
    assert!(matches!(update(&mut awaiting, listing()), Effect::Read { .. }));
}

#[test]
fn the_same_value_twice_changes_nothing() {
    let mut st = State::default();
    update(&mut st, Msg::Held);
    update(&mut st, listing());
    assert!(matches!(update(&mut st, enable()), Effect::Read { .. }));
    let before = st.clone();
    assert_eq!(update(&mut st, enable()), Effect::None);
    assert_eq!(st, before);

    let mut off = State::default();
    let before = off.clone();
    let effect = update(
        &mut off,
        Msg::EnabledChanged {
            enabled: false,
            now: NOW,
        },
    );
    assert_eq!(effect, Effect::None);
    assert_eq!(off, before);
}

// ---- U74: switching off clears ----

#[test]
fn switching_off_clears_everything_and_goes_idle() {
    let mut st = holding_statuses();
    st.removable.insert("feat/a".into());
    st.pause_until = Some(UNTIL);
    let seq = reading_again(&mut st);
    assert!(seq > 0);

    let effect = update(
        &mut st,
        Msg::EnabledChanged {
            enabled: false,
            now: LATER,
        },
    );
    assert_eq!(effect, Effect::None);
    assert!(st.statuses.is_empty());
    assert!(st.removable.is_empty());
    assert_eq!(st.read_at, None);
    assert_eq!(st.pause_until, None);
    assert_eq!(st.phase, Phase::Idle);
    assert!(!st.enabled);
}

/// Force a reading under way on a state that already read once, by turning the switch off and on.
fn reading_again(st: &mut State) -> u64 {
    let pause = st.pause_until.take();
    update(
        st,
        Msg::EnabledChanged {
            enabled: false,
            now: NOW,
        },
    );
    let kept = (st.statuses.clone(), st.removable.clone());
    let Effect::Read { seq } = update(st, enable()) else {
        panic!("switching on while held and listed reads");
    };
    (st.statuses, st.removable) = kept;
    st.pause_until = pause;
    seq
}

// ---- U75–U79: how a reading ends ----

#[test]
fn an_ok_answer_replaces_the_statuses_and_records_when_the_reading_started() {
    let mut st = enabled();
    let seq = reading(&mut st);
    assert_eq!(finish(&mut st, seq, ok(NOW)), Effect::None);
    assert_eq!(st.statuses, statuses());
    assert_eq!(st.read_at, Some(NOW));
    assert_eq!(st.phase, Phase::Idle);
}

#[test]
fn unavailable_clears_the_statuses() {
    let mut st = holding_statuses();
    let seq = reading_again(&mut st);
    finish(&mut st, seq, Outcome::Err(ReadingFailure::Unavailable));
    assert!(st.statuses.is_empty());
    assert_eq!(st.read_at, None);
    assert_eq!(st.phase, Phase::Idle);
}

#[test]
fn a_passing_failure_changes_nothing() {
    let mut st = holding_statuses();
    let seq = reading_again(&mut st);
    let read_at = st.read_at;
    finish(&mut st, seq, Outcome::Err(ReadingFailure::Passing));
    assert_eq!(st.statuses, statuses());
    assert_eq!(st.read_at, read_at);
    assert_eq!(st.phase, Phase::Idle);
}

#[test]
fn a_rate_limit_keeps_the_statuses_and_pauses() {
    let mut st = holding_statuses();
    let seq = reading_again(&mut st);
    finish(
        &mut st,
        seq,
        Outcome::Err(ReadingFailure::RateLimited { until: UNTIL }),
    );
    assert_eq!(st.statuses, statuses());
    assert_eq!(st.pause_until, Some(UNTIL));
    assert_eq!(st.phase, Phase::Idle);
}

#[test]
fn an_answer_with_another_seq_is_dropped() {
    let mut st = enabled();
    let seq = reading(&mut st);
    let before = st.clone();
    assert_eq!(finish(&mut st, seq + 1, ok(NOW)), Effect::None);
    assert_eq!(st, before);
}

// ---- U80–U82: the hold ends ----

#[test]
fn released_clears_the_statuses_and_keeps_the_pause() {
    let mut st = holding_statuses();
    st.pause_until = Some(UNTIL);
    assert_eq!(update(&mut st, Msg::Released), Effect::None);
    assert!(st.statuses.is_empty());
    assert!(!st.held);
    assert!(!st.awaiting_listing);
    assert_eq!(st.read_at, None);
    assert_eq!(st.phase, Phase::Idle);
    assert_eq!(st.pause_until, Some(UNTIL));
    assert!(st.enabled);
}

#[test]
fn after_release_only_held_and_a_listing_read_and_exactly_once() {
    let mut st = holding_statuses();
    update(&mut st, Msg::Released);
    assert_eq!(update(&mut st, listing()), Effect::None);
    assert_eq!(update(&mut st, enable()), Effect::None);
    assert_eq!(update(&mut st, Msg::Released), Effect::None);
    assert_eq!(update(&mut st, Msg::Held), Effect::None);
    assert!(matches!(update(&mut st, listing()), Effect::Read { .. }));
    assert_eq!(update(&mut st, listing()), Effect::None);
}

#[test]
fn a_listing_while_paused_reads_nothing() {
    let mut st = enabled();
    st.pause_until = Some(UNTIL);
    update(&mut st, Msg::Held);
    assert_eq!(update(&mut st, listing()), Effect::None);
    assert_eq!(st.phase, Phase::Idle);
    assert!(!st.awaiting_listing);
}

#[test]
fn an_answer_after_release_is_dropped() {
    let mut st = enabled();
    let seq = reading(&mut st);
    update(&mut st, Msg::Released);
    finish(&mut st, seq, ok(NOW));
    assert!(st.statuses.is_empty());
}

// ---- U83: one reading at a time ----

/// Every message, in a fixed order, with the `seq` of the reading under way where one is needed.
fn messages_for(st: &State, pick: usize) -> Msg {
    let seq = match st.phase {
        Phase::Reading { seq, .. } => seq,
        Phase::Idle => st.next_seq,
    };
    match pick % 8 {
        0 => Msg::Held,
        1 => listing(),
        2 => enable(),
        3 => Msg::EnabledChanged {
            enabled: false,
            now: NOW,
        },
        4 => Msg::Finished {
            seq,
            outcome: ok(NOW),
            now: LATER,
        },
        5 => Msg::Finished {
            seq,
            outcome: Outcome::Err(ReadingFailure::RateLimited { until: NOW + 1 }),
            now: LATER,
        },
        6 => Msg::Released,
        _ => Msg::RemotesTimedOut { seq, req: 1 },
    }
}

#[test]
fn no_sequence_yields_two_reads_without_an_end_between_them() {
    // A small linear congruential walk: deterministic, and it visits long mixed sequences.
    let mut x: u64 = 0x2545_f491;
    for _ in 0..200 {
        let mut st = State::default();
        for _ in 0..40 {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let msg = messages_for(&st, (x >> 33) as usize);
            let was_reading = matches!(st.phase, Phase::Reading { .. });
            let ends = matches!(
                msg,
                Msg::Finished { .. } | Msg::Released | Msg::EnabledChanged { enabled: false, .. }
            );
            if let Effect::Read { .. } = update(&mut st, msg) {
                assert!(
                    !was_reading || ends,
                    "a second reading started while one was under way"
                );
                assert!(matches!(st.phase, Phase::Reading { .. }));
            }
        }
    }
}
