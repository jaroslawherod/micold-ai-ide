//! The overlay layer is built the way every component is (feature 021, T030 — Principle VIII,
//! FR-030).
//!
//! # Why this layer, when Principle VIII is about components
//!
//! It is not a component: nothing under `src/overlay/` becomes an `iced::Element`, and T028 removed
//! the `view` method the contract sketch gave [`FloatingSurface`] precisely so it could not. The
//! element terminator for a floating surface is `ui::cdk::overlay::Surface`, which is already a
//! Principle VIII component and already ends in `.into()`; the registry reaches it at T035.
//!
//! What carries over is the *shape*, and the reason for it. A surface is described by handing
//! required inputs to a constructor and optional ones to chainable steps —
//! `DismissalRules::for_layer(band).cancelled_by(msg).protecting_input()` — and that chain
//! terminates in a conversion, `(&surface).into()`. Sixteen surfaces arrive over T031–T036, each
//! one an opportunity for a `pub` field or a `set_` method to appear "just for this case", and the
//! second way to configure something is how the five overlay implementations feature 017
//! consolidated drifted apart in the first place.
//!
//! # One definition, not a second one
//!
//! The scan is `tests/inventory/mod.rs`'s, pointed at another directory. Restating "what a
//! constructor is" here would be the same silent-drift arrangement FR-014 objects to — two
//! scanners agreeing today and quietly diverging later — so `declarations_in` was added there
//! rather than a parser being written here.
//!
//! # Where this layer differs, deliberately
//!
//! In the library, a public method that is neither construction nor a builder step is an exception
//! needing a name on a sanctioned list. Here it is the norm: `kind`, `id`, `layer`, `on` and
//! `as_str` exist to be *asked*, because generic dispatch's whole job is asking. So the rule is
//! stated as the two things that would actually break the shape — a public field, and a method
//! that mutates through `&mut self` instead of consuming `self`.

mod inventory;

use inventory::{code_only, declarations_in, sources_in, src_dir};
use micold_client::app::Message;
use micold_client::overlay::registry::Open;
use micold_client::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::overlay::{Layer, Surface, Trigger};

fn overlay_dirs() -> Vec<std::path::PathBuf> {
    vec![src_dir("overlay")]
}

#[test]
fn nothing_in_the_overlay_layer_is_assembled_field_by_field() {
    let exposed: Vec<String> = declarations_in(&overlay_dirs())
        .into_iter()
        .filter(|d| d.public_fields)
        .map(|d| format!("{} `{}`", d.module, d.name))
        .collect();

    assert!(
        exposed.is_empty(),
        "these expose public fields:\n{}\n\nConfiguration goes through chainable steps, so adding \
         one cannot break a caller and there is only ever one way to set it (Principle VIII).",
        exposed.join("\n")
    );
}

#[test]
fn nothing_in_the_overlay_layer_is_configured_by_mutation() {
    let mut offenders = Vec::new();
    for (module, src) in sources_in(&overlay_dirs()) {
        for line in code_only(&src).lines() {
            let Some(sig) = line.trim_start().strip_prefix("pub fn ") else {
                continue;
            };
            if sig.contains("(&mut self") {
                let name: String = sig
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                offenders.push(format!("{module} `{name}`"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "these configure by mutation:\n{}\n\nA builder step takes `self` and returns `Self`. A \
         `&mut self` setter is a second way to reach the same field, and a surface half-configured \
         at one call site and half at another is the drift this layer exists to end.",
        offenders.join("\n")
    );
}

#[test]
fn a_surface_description_terminates_in_a_conversion() {
    // The chain, end to end: describe the surface, then convert. `Open` has no constructor, so
    // this is not one of two ways to build it — it is the only one.
    struct Probe;
    impl FloatingSurface for Probe {
        fn id(&self) -> SurfaceId {
            SurfaceId::new("probe")
        }
        fn layer(&self) -> Layer {
            Layer::Dialog
        }
        fn dismissal(&self) -> DismissalRules {
            DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::AboutClosed)
        }
    }

    let open: Open = (&Probe).into();

    assert_eq!(open.id(), SurfaceId::new("probe"));
    assert_eq!(open.layer(), Layer::Dialog);
    assert_eq!(open.on(Trigger::Escape), Some(&Message::AboutClosed));
}

#[test]
fn the_optional_steps_are_chainable_in_any_order() {
    // The property that makes a step optional: it composes, and it does not care what came before.
    // A step implemented as anything other than `self -> Self` would not survive being reordered.
    let a = DismissalRules::for_layer(Layer::Dialog)
        .cancelled_by(Message::SettingsCancelled)
        .protecting_input();
    let b = DismissalRules::for_layer(Layer::Dialog)
        .protecting_input()
        .cancelled_by(Message::SettingsCancelled);

    assert_eq!(a, b);
    assert_eq!(a.kind(), Surface::NonDismissibleDialog);
}

#[test]
fn the_scan_actually_finds_the_overlay_layer() {
    let found: Vec<String> = declarations_in(&overlay_dirs())
        .into_iter()
        .map(|d| d.name)
        .collect();

    for expected in ["SurfaceId", "DismissalRules", "Open", "ModalSurface"] {
        assert!(
            found.contains(&expected.to_string()),
            "expected `{expected}` among the overlay layer's structs, found {found:?} — a scan \
             that finds nothing passes both rules above"
        );
    }
}
