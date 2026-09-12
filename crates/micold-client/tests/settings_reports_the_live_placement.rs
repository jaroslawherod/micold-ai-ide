//! The Settings note says where sessions run *now* (BUG-003 — T163, FR-035b).
//!
//! FR-035b requires the application to be honest about which placement is in force. The note under
//! the placement select was the one control that claimed to answer it, and it read the *draft* —
//! so the moment the user picked **In a container**, the line beneath the select said "Currently in
//! a container" about a service still running on the host. It reported the question back as its own
//! answer, and the supporting text above it ("Takes effect the next time the application starts")
//! told the user that was normal.
//!
//! Now the note is a function of the placement in force and of nothing else, which is the whole
//! rule: the draft's choice is already on the select directly above it.

use micold_client::features::settings;
use micold_core::sandbox::placement::PlacementKind;

#[test]
fn the_note_names_what_is_running_now() {
    assert_eq!(
        settings::placement_note(PlacementKind::HostProcess),
        "Currently on this computer.",
        "the note has to name the placement the way the select names it, or the two lines \
         disagree about what the choices are called"
    );
    assert_eq!(
        settings::placement_note(PlacementKind::LocalSandbox),
        "Currently in a container.",
    );
}

/// The note distinguishes the placements at all. A note that says the same thing either way is
/// FR-035b satisfied on paper and useless on screen.
#[test]
fn the_note_distinguishes_the_two_placements() {
    assert_ne!(
        settings::placement_note(PlacementKind::HostProcess),
        settings::placement_note(PlacementKind::LocalSandbox),
    );
}

/// The regression itself, pinned where it happened. `placement_note` takes no draft, so the view
/// cannot render one through it — but the view could still build the sentence itself again, which
/// is precisely what it used to do. This is the assertion that the string is gone from the section
/// that shipped the bug.
#[test]
fn the_daemon_section_does_not_build_the_note_from_the_draft() {
    let source = std::fs::read_to_string("src/ui/settings/daemon.rs")
        .expect("the daemon settings section is beside this crate's tests");

    assert!(
        source.contains("placement_note(in_force)"),
        "the note must come from the placement in force, threaded in from `app.placement` — a \
         section that computes it from what it has to hand can only have the draft (FR-035b)"
    );
    assert!(
        !source.contains("Currently {}"),
        "the section is building the note itself again; the draft is the only placement it holds, \
         which is how it said \"Currently in a container\" about a host process (BUG-003)"
    );
    assert!(
        !source.contains("Takes effect the next time the application starts"),
        "the supporting text is the bug written down as documentation — a save now moves the \
         service (FR-033a), so nothing waits for the next launch"
    );
}
