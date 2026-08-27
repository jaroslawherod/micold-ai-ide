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

use micold_client::features::session::{AvailabilitySource, CliAvailability};
use micold_client::features::settings::missing_cli_notice;
use micold_core::session::AiCli;

const IMAGE: &str = "ghcr.io/example/my-own-image:3";

fn in_image(available: &[AiCli]) -> CliAvailability {
    CliAvailability {
        available: available.to_vec(),
        source: AvailabilitySource::Image(IMAGE.to_string()),
    }
}

fn on_host(available: &[AiCli]) -> CliAvailability {
    CliAvailability {
        available: available.to_vec(),
        source: AvailabilitySource::ThisComputer,
    }
}

#[test]
fn an_unanswered_service_says_nothing() {
    // The reason `available_providers` is an `Option` at all. Before feature 027 the client probed
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
    let notice = missing_cli_notice(Some(&in_image(&[AiCli::ClaudeCode])))
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
    assert!(
        !notice.contains(AiCli::ClaudeCode.provider().display_name()),
        "the notice names a CLI the image provides: {notice}"
    );
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
        "two missing CLIs read as a sentence, not a comma list: {notice}"
    );
}

#[test]
fn the_host_placement_gets_a_different_sentence_and_no_image() {
    // FR-023c's other half. With the service on this computer there is no image, and telling the
    // user to fix one would send them to a machine that does not exist. The remedy is installing
    // the CLI, so the sentence is about this computer.
    let notice = missing_cli_notice(Some(&on_host(&[AiCli::ClaudeCode])))
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
        notice.contains("this computer"),
        "the notice must say where sessions run: {notice}"
    );
}

#[test]
fn one_missing_cli_and_two_agree_with_their_verbs() {
    // Sentence-level, and worth a line: the notice appears in a settings form beside the control
    // it is about, and "GitHub Copilot aren't in …" is the kind of thing that makes a user trust
    // the rest of the page less.
    let one = missing_cli_notice(Some(&in_image(&[AiCli::ClaudeCode]))).unwrap();
    let both = missing_cli_notice(Some(&in_image(&[]))).unwrap();

    assert!(one.contains("isn't"), "singular: {one}");
    assert!(both.contains("aren't"), "plural: {both}");
}
