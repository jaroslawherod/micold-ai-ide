//! `DismissalRules` forwards to the core rule rather than restating it (feature 021, T028 —
//! FR-014).
//!
//! The type is thin by design, and thin types are where a parallel rule creeps in: someone adds a
//! `match` here "just for this case", and now there are two answers to when a dialog closes. These
//! tests assert the forwarding itself — that the answers are the core's answers, for every
//! kind/trigger pair — so a local special case fails here rather than being discovered as a
//! surface that closes when its neighbour does not.

use micold_client::app::Message;
use micold_client::features::help::Msg as HelpMsg;
use micold_client::overlay::{DismissalRules, SurfaceId};
use micold_core::overlay::{dismisses, Layer, Surface, Trigger};

fn cancellable(layer: Layer) -> DismissalRules {
    DismissalRules::for_layer(layer).cancelled_by(Message::Help(HelpMsg::AboutClosed))
}

#[test]
fn a_band_implies_its_surface_kind_so_the_two_cannot_be_paired_wrongly() {
    for layer in Layer::ALL {
        assert_eq!(
            DismissalRules::for_layer(*layer).kind(),
            layer.surface(),
            "the kind must follow from the band, exactly as it does for cdk::overlay::Surface — \
             two places deriving it differently is how a dialog ends up dismissing like a menu"
        );
    }
}

#[test]
fn every_kind_and_trigger_answers_exactly_what_the_core_answers() {
    for layer in Layer::ALL {
        for trigger in Trigger::ALL {
            let rules = cancellable(*layer);

            assert_eq!(
                rules.on(*trigger).is_some(),
                dismisses(layer.surface(), *trigger),
                "{layer:?} + {trigger:?}: this type disagreed with micold_core::overlay::dismisses, \
                 which means a second dismissal rule now exists (FR-014)"
            );
        }
    }
}

#[test]
fn a_surface_protecting_input_is_closed_by_nothing_implicit() {
    let rules = cancellable(Layer::Dialog).protecting_input();

    assert_eq!(rules.kind(), Surface::NonDismissibleDialog);
    for trigger in Trigger::ALL {
        assert!(
            rules.on(*trigger).is_none(),
            "{trigger:?} closed a surface holding input that an accidental dismissal would destroy"
        );
    }
}

#[test]
fn a_surface_with_no_cancel_message_has_no_implicit_close() {
    let rules = DismissalRules::for_layer(Layer::Popover);

    for trigger in Trigger::ALL {
        assert!(
            rules.on(*trigger).is_none(),
            "{trigger:?} produced a dismissal for a surface that never said what closing it means"
        );
    }
    assert!(
        dismisses(Surface::NonModal, Trigger::Escape),
        "precondition: the core says this kind *is* dismissible, so the None above is the missing \
         message and not the rule — otherwise this test would pass for the wrong reason"
    );
}

#[test]
fn the_cancel_message_is_the_one_the_surface_named() {
    let rules = DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::SettingsCancelled);

    assert_eq!(
        rules.on(Trigger::Escape),
        Some(&Message::SettingsCancelled),
        "dispatch asks one question — what happens on this trigger — and gets the pairing, so it \
         cannot mismatch a surface's rule with another's message"
    );
}

#[test]
fn a_dialog_survives_a_scroll_behind_it_but_a_menu_does_not() {
    // The one asymmetry in the core rule, restated at this layer because it is the difference most
    // likely to be flattened by a "simplification" of the forwarding above.
    assert!(
        cancellable(Layer::Dialog)
            .on(Trigger::ScrollBeneath)
            .is_none(),
        "a dialog is anchored to nothing, so scrolling the content behind it is not a decision to \
         close it"
    );
    assert!(
        cancellable(Layer::Popover)
            .on(Trigger::ScrollBeneath)
            .is_some(),
        "a menu anchored to a row is meaningless once that row has moved"
    );
}

#[test]
fn a_surface_identity_is_its_own_name_and_nothing_else() {
    let id = SurfaceId::new("about");

    assert_eq!(id.as_str(), "about");
    assert_eq!(id.to_string(), "about");
    assert_ne!(
        id,
        SurfaceId::new("settings"),
        "identities must distinguish surfaces — the registry keys on them"
    );
}
