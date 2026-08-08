//! Notification severity, in isolation (feature 021, SC-004).
//!
//! This file names exactly one feature module and the core queue its API translates into. It builds
//! no `State`, references no other feature's types, and needs no application shell.

use micold_client::features::notifications::NoticeLevel;
use micold_core::notify::Level;

#[test]
fn each_banner_severity_maps_to_the_matching_queue_level() {
    assert_eq!(NoticeLevel::Info.to_queue_level(), Level::Info);
    assert_eq!(NoticeLevel::Error.to_queue_level(), Level::Error);
}

#[test]
fn the_translation_is_injective_so_a_failure_cannot_arrive_as_an_aside() {
    assert_ne!(
        NoticeLevel::Info.to_queue_level(),
        NoticeLevel::Error.to_queue_level(),
        "collapsing the two severities would let a failed action render as a neutral notice, \
         which is the silent-failure pattern this surface exists to end"
    );
}

#[test]
fn severity_does_not_carry_a_duration() {
    // `NoticeLevel` is the banner's vocabulary: it picks a fill. How long a message lingers is
    // the queue's business, and `notify::Level::duration` is where that lives. This test cannot
    // assert the absence of a method, so it asserts the seam instead — the banner has to go
    // through the queue level to learn anything about timing.
    assert!(
        NoticeLevel::Error.to_queue_level().duration()
            > NoticeLevel::Info.to_queue_level().duration(),
        "an error is worth reading, so it stays longer — but only the queue level knows that"
    );
}
