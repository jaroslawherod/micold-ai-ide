//! What the two points of choice say when the place sessions run has no such CLI (feature 027,
//! FR-023b).
//!
//! FR-023a made the published image ship every AI CLI. FR-023b is the other half: an image the
//! user substitutes inherits that obligation, and nothing in this application can make it keep
//! one. So the requirement is not "guarantee it" — it is "say so, where the user is choosing, in
//! terms of the thing that would have to provide it".
//!
//! Three properties, and the second and third are the ones that took thought:
//!
//! - it names the CLI, the place, and the obligation;
//! - it says **nothing at all** before the service has answered — an unanswered service and a
//!   service that answered "none" are different situations, and only the second is a fact about
//!   the user's image;
//! - it is not phrased as this application failing. It is a fact about a machine, with the remedy
//!   on the machine, because that is where the user can act.
//!
//! The wording is asserted by its parts rather than verbatim. Pinning the whole sentence makes
//! every rephrasing a test edit, which trains the edit rather than the reading — but the parts are
//! exactly what FR-023b enumerates, so an assertion that loses one of them is a requirement that
//! stopped being met.

use micold_client::features::session::{AvailabilityKey, AvailabilitySource, CliAvailability};
use micold_client::features::settings::missing_cli_notice;
use micold_core::cli_reason::{explain, AttemptDir, Place, SpawnEnv};
use micold_core::session::AiCli;

const IMAGE: &str = "ghcr.io/example/my-own-image:3";

/// Every state of FR-001, in its order.
const STATES: [SpawnEnv; 6] = [
    SpawnEnv::IncludeOff,
    SpawnEnv::NoScriptPath,
    SpawnEnv::ScriptNotFound,
    SpawnEnv::ScriptFailed,
    SpawnEnv::ScriptTimedOut,
    SpawnEnv::Applied,
];

/// An image's answer with the script applied: the one state in which "the image lacks it" is
/// true, and so the state 027's sentence is kept for (037 FR-005).
fn in_image(available: &[AiCli]) -> CliAvailability {
    CliAvailability {
        available: available.to_vec(),
        source: AvailabilitySource::Image(IMAGE.to_string()),
        env: Some(SpawnEnv::Applied),
        asked_for: AvailabilityKey::Home,
    }
}

/// This computer's answer with the script applied.
fn on_host(available: &[AiCli]) -> CliAvailability {
    CliAvailability {
        available: available.to_vec(),
        source: AvailabilitySource::ThisComputer,
        env: Some(SpawnEnv::Applied),
        asked_for: AvailabilityKey::Home,
    }
}

/// `answer`, in another environment state.
fn in_state(mut answer: CliAvailability, env: Option<SpawnEnv>) -> CliAvailability {
    answer.env = env;
    answer
}

/// What `explain` says for these CLIs, as the note joins it.
fn explained(missing: &[AiCli], env: SpawnEnv, place: Place<'_>) -> String {
    let said = explain(missing, env, place, AttemptDir::Home).expect("something is missing");
    format!("{} {}", said.reason, said.action)
}

#[test]
fn an_unanswered_service_says_nothing() {
    // The reason an answer is an `Option` at all. Before feature 027 the client probed
    // its own `PATH` and so always had an answer; now there is a round trip, and the state between
    // asking and hearing back is real. Saying "GitHub Copilot isn't in your image" during it would
    // be a guess — and a guess that names a specific CLI reads exactly like a finding.
    assert_eq!(missing_cli_notice(None), None);
}

#[test]
fn an_image_with_every_cli_says_nothing_either() {
    // The published image (FR-023a). No notice, and not a reassuring one: a green "all present"
    // line is a second thing to read on every visit to a form that is not about AI CLIs, and the
    // absence of a warning is already the whole of "this is fine".
    assert_eq!(missing_cli_notice(Some(&in_image(&AiCli::ALL))), None);
}

#[test]
fn an_image_missing_one_names_it_the_image_and_the_obligation() {
    let notice = missing_cli_notice(Some(&in_image(&[AiCli::ClaudeCode, AiCli::Pi])))
        .expect("an image without Copilot has something to report");

    // The CLI, by the name a menu would show it under — the same register the picker beside this
    // notice uses, so the user can match the sentence to the missing entry.
    assert!(
        notice.contains(AiCli::Copilot.provider().display_name()),
        "the notice must name the CLI: {notice}"
    );
    // The image, exactly as configured. "your image" would be true and useless to someone who
    // maintains more than one.
    assert!(
        notice.contains(IMAGE),
        "the notice must name the image: {notice}"
    );
    // And what would have to change. FR-023b's third clause: the image is what provides a CLI, so
    // the image is what the user has to fix.
    assert!(
        notice.contains("image"),
        "the notice must say the image is what provides it: {notice}"
    );
    // Not the application's failure. Nothing here apologises, reports an error, or suggests
    // something went wrong on this side — the user substituted an image and it does not contain a
    // thing. That is a fact, and the sentence is in the indicative.
    for blame in ["error", "failed", "sorry", "unable", "couldn't"] {
        assert!(
            !notice.to_lowercase().contains(blame),
            "the notice reads as the app failing (`{blame}`): {notice}"
        );
    }
    // And it does not name a CLI that *is* there.
    for present in [AiCli::ClaudeCode, AiCli::Pi] {
        assert!(
            !notice.contains(present.provider().display_name()),
            "the notice names a CLI the image provides: {notice}"
        );
    }
}

#[test]
fn an_image_with_no_cli_at_all_names_every_one() {
    // The scenario FR-023b is really about: someone points this at a plain `ubuntu`. Every CLI is
    // named, in the declared order, and the sentence still reads as prose rather than as a list.
    let notice = missing_cli_notice(Some(&in_image(&[]))).expect("an empty answer is a real one");

    for which in AiCli::ALL {
        assert!(
            notice.contains(which.provider().display_name()),
            "{which:?} is missing and unnamed: {notice}"
        );
    }
    assert!(
        notice.contains(" and "),
        "several missing CLIs read as a sentence, not a comma list: {notice}"
    );
}

#[test]
fn the_host_placement_gets_a_different_sentence_and_no_image() {
    // FR-023c's other half. With the service on this computer there is no image, and telling the
    // user to fix one would send them to a machine that does not exist. Since feature 037 the
    // sentence is about the PATH sessions get, which is what the user can change.
    let notice = missing_cli_notice(Some(&on_host(&[AiCli::ClaudeCode, AiCli::Pi])))
        .expect("a host without Copilot has something to report");

    assert!(
        notice.contains(AiCli::Copilot.provider().display_name()),
        "the notice must still name the CLI: {notice}"
    );
    assert!(
        !notice.contains("image"),
        "there is no image under the host placement; naming one sends the user to fix nothing: \
         {notice}"
    );
    assert!(
        notice.contains("the PATH sessions get for your home directory"),
        "the notice must say where the CLI was looked for: {notice}"
    );
}

#[test]
fn one_missing_cli_and_two_agree_with_their_verbs() {
    // Sentence-level, and worth a line: the notice appears in a settings form beside the control
    // it is about, and "GitHub Copilot aren't in …" is the kind of thing that makes a user trust
    // the rest of the page less.
    let one = missing_cli_notice(Some(&in_image(&[AiCli::ClaudeCode, AiCli::Pi]))).unwrap();
    let both = missing_cli_notice(Some(&in_image(&[]))).unwrap();

    assert!(one.contains("isn't"), "singular: {one}");
    assert!(both.contains("aren't"), "plural: {both}");
}

#[test]
fn a_missing_pi_is_named_as_pi_coding_agent_and_never_as_its_command() {
    // Feature 029, T032 (FR-001a). The same sentence the other two get, in the menu register: the
    // select beside it lists "Pi Coding Agent", so that is the string the user matches it against.
    // `pi` in a sentence is a two-letter word that reads as a typo.
    for availability in [
        in_image(&[AiCli::ClaudeCode, AiCli::Copilot]),
        on_host(&[AiCli::ClaudeCode, AiCli::Copilot]),
    ] {
        let notice = missing_cli_notice(Some(&availability))
            .expect("a place without pi has something to report");
        assert!(
            notice.starts_with("Pi Coding Agent isn't")
                || notice.starts_with("Pi Coding Agent was not found"),
            "the notice names Pi by its display name, singular: {notice}"
        );
        assert!(
            !notice.contains(" pi "),
            "…and not by its command: {notice}"
        );
    }
}

#[test]
fn a_missing_pi_is_a_presence_fact_with_no_version_in_it() {
    // FR-003a. Availability is the presence check every provider gets, so nothing was spawned to
    // decide Pi is missing and there is no version to report. A number here would be one the
    // application invented — the version belongs to the failure of an *installed* `pi`, where it
    // was read, not to the notice that it is absent.
    let notice = missing_cli_notice(Some(&on_host(&[AiCli::ClaudeCode, AiCli::Copilot]))).unwrap();
    assert!(
        !notice.chars().any(|c| c.is_ascii_digit()),
        "a missing CLI has no version to name: {notice}"
    );
}

// --- Feature 037: the note gives the reason for the environment's state, and the action ---

/// 037 U47 (FR-006, SC-001): on this computer the note is `explain`'s reason and action for the
/// home directory, in each of the six states.
#[test]
fn on_this_computer_the_note_is_the_reason_and_action_of_each_state() {
    let missing = [AiCli::Copilot, AiCli::Pi];
    for env in STATES {
        assert_eq!(
            missing_cli_notice(Some(&in_state(on_host(&[AiCli::ClaudeCode]), Some(env)))),
            Some(explained(&missing, env, Place::ThisComputer)),
            "{env:?}"
        );
    }
}

/// 037 U48 (FR-005, FR-002): with an image, the five states in which no script was applied give
/// the environment reason. "Isn't in the image" would be a claim nobody checked.
#[test]
fn in_an_image_the_note_blames_the_image_only_when_the_script_was_applied() {
    let missing = [AiCli::Copilot];
    for env in STATES.into_iter().filter(|env| !env.script_applied()) {
        let note = missing_cli_notice(Some(&in_state(
            in_image(&[AiCli::ClaudeCode, AiCli::Pi]),
            Some(env),
        )))
        .expect("Copilot is missing");

        assert_eq!(
            note,
            explained(&missing, env, Place::Image(IMAGE)),
            "{env:?}"
        );
        assert!(
            !note.contains("isn't in"),
            "no script was applied ({env:?}), so the image has not been shown to lack it: {note}"
        );
    }
}

/// 037 U53 (W5, FR-011): the other side of U47. A CLI is missing and the service could not say
/// which state the environment is in, so any reason would be a guess.
#[test]
fn a_missing_cli_with_no_known_state_says_nothing() {
    for answer in [
        on_host(&[AiCli::ClaudeCode]),
        in_image(&[AiCli::ClaudeCode]),
    ] {
        assert_eq!(missing_cli_notice(Some(&in_state(answer, None))), None);
    }
}

/// 037 U54 (FR-004): every CLI missing is one sentence naming all three, with the reason once.
#[test]
fn with_nothing_available_every_cli_is_named_and_the_reason_is_given_once() {
    let note = missing_cli_notice(Some(&in_state(on_host(&[]), Some(SpawnEnv::IncludeOff))))
        .expect("an empty answer is a real one");

    assert!(
        note.starts_with(
            "A session would not find Claude Code, GitHub Copilot and Pi Coding Agent:"
        ),
        "{note}"
    );
    assert_eq!(
        note.matches("is off").count(),
        1,
        "one reason for the three, not one each: {note}"
    );
}

/// 037 U55 (FR-015): giving a reason changes nothing. The function borrows the answer.
#[test]
fn asking_for_the_note_leaves_the_answer_as_it_was() {
    let answer = in_state(on_host(&[AiCli::ClaudeCode]), Some(SpawnEnv::ScriptFailed));
    let before = answer.clone();

    let _ = missing_cli_notice(Some(&answer));

    assert_eq!(answer, before);
}
