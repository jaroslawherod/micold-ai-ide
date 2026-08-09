//! What a floating surface *is*, so dispatch can be generic over them (feature 021, Tier 2, T028).
//!
//! # This is not a second overlay implementation
//!
//! FR-014 requires building on the existing floating-surface vocabulary rather than introducing a
//! parallel one, and `tests/one_overlay_implementation.rs` holds that line. So it is worth being
//! precise about what already exists and what is actually missing, because at a glance this module
//! looks like a duplicate of both of them.
//!
//! | Layer | Job | Lives in |
//! |---|---|---|
//! | The rule | Which triggers close which kind of surface; which band sits above which | `micold_core::overlay` (feature 017) |
//! | The render-time primitive | A panel, its anchor, its scrim, its backdrop | `ui::cdk::overlay::Surface` (feature 017) |
//! | **The state-time description** | **Which surfaces exist, which is open, and what closes it** | **here (feature 021)** |
//!
//! Feature 017 answered "how is a floating panel drawn and dismissed" once, for all of them. What
//! it did not need — because the `Overlay` enum was doing the job — is a way to talk about a
//! surface *while it is not being rendered*: to say "this one is open", "this is the message that
//! closes it", "this is what it looked like as it faded out", without a central enum listing every
//! possibility. That is the gap Tier 2 fills, and it is the only thing this module adds.
//!
//! A [`FloatingSurface`] therefore produces a `cdk::overlay::Surface` at render time rather than
//! replacing it, and every dismissal question is forwarded to `micold_core::overlay::dismisses`
//! rather than answered here.
//!
//! # Two names this module deliberately does not introduce
//!
//! T028 asks for `StackBand` and `DismissalRules`.
//!
//! **`StackBand` is not here.** It would be a second name for
//! [`micold_core::overlay::Layer`], which already declares the bands bottom-to-top and derives
//! `Ord` as the z-order. A synonym is precisely the "second, parallel vocabulary" FR-014 forbids —
//! two names for one concept is how the five implementations feature 017 consolidated drifted
//! apart in the first place. `Layer` is used directly throughout.
//!
//! **[`DismissalRules`] is here, but it decides nothing.** It records the two things that genuinely
//! vary per surface and that the core cannot know: which [`Surface`] kind this is, and which
//! message its cancellation sends. Whether a given trigger closes it is forwarded to `dismisses`.
//! A struct named "rules" that contained rules would be the parallel rule engine FR-014 rules out.

pub mod registry;

use crate::app::Message;
use micold_core::overlay::{dismisses, Layer, Surface, Trigger};

/// A floating surface's stable identity.
///
/// A `&'static str` rather than an enum variant, because an enum is the thing Tier 2 is removing:
/// the point is that a surface can be added without editing a central list. The string is the
/// surface's own name, supplied at its definition, and never shown to the user.
///
/// Ordered and hashable so surfaces can key a map or be sorted deterministically without the
/// registry having to invent an index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceId(&'static str);

impl SurfaceId {
    /// An identity for a surface named `name`.
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// The name, for diagnostics and guard tests.
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for SurfaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// What closes a surface: the kind it is, and the message its cancellation sends.
///
/// Deliberately thin. The *rule* — which triggers close which kind — is
/// [`micold_core::overlay::dismisses`], and this type forwards to it rather than restating it.
/// What it adds is the per-surface part the core cannot know: a surface's cancel message, and the
/// rare surface that must resist every implicit close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DismissalRules {
    kind: Surface,
    cancel: Option<Message>,
}

impl DismissalRules {
    /// The default rules for a surface in `layer`.
    ///
    /// The kind follows from the band, exactly as it does for `cdk::overlay::Surface::new` — a
    /// caller cannot pair dialog stacking with menu dismissal by accident. No cancel message yet:
    /// a surface with none has no implicit close, which is the safe default for a surface whose
    /// author has not said otherwise.
    pub fn for_layer(layer: Layer) -> Self {
        Self {
            kind: layer.surface(),
            cancel: None,
        }
    }

    /// The message this surface's cancellation sends.
    pub fn cancelled_by(mut self, message: Message) -> Self {
        self.cancel = Some(message);
        self
    }

    /// Declare the surface non-dismissible: it holds input an accidental close would destroy, so
    /// nothing implicit closes it and the user must act inside it.
    ///
    /// Mirrors `cdk::overlay::Surface::non_dismissible`, and for the same reason — the two are the
    /// same decision made at different times, so they must be spellable the same way.
    pub fn protecting_input(mut self) -> Self {
        self.kind = Surface::NonDismissibleDialog;
        self
    }

    /// The surface kind, for callers that need to consult the core rule themselves.
    pub fn kind(&self) -> Surface {
        self.kind
    }

    /// The message to send when `trigger` happens, or `None` when this trigger does not close this
    /// surface — or when the surface has no cancel message at all.
    ///
    /// One function for both cases on purpose: a caller that has to ask "does this dismiss?" and
    /// then separately "what message?" has two chances to get the pairing wrong, and the pairing is
    /// the whole thing dispatch needs.
    pub fn on(&self, trigger: Trigger) -> Option<&Message> {
        dismisses(self.kind, trigger)
            .then_some(self.cancel.as_ref())
            .flatten()
    }
}

/// A surface that can float above the main window.
///
/// Implemented by the feature that owns the surface. Everything generic dispatch needs to render,
/// stack and dismiss a surface without knowing which one it is.
///
/// **Render-free on purpose.** There is no `view` method here, even though the contract sketch had
/// one, because FR-006 requires feature modules to name no rendering framework and
/// `tests/features_are_render_free.rs` enforces it. Tier 1 already settled where the other half
/// goes: views live in `crate::ui`, beside the feature rather than inside it. The registration
/// point (T029) names a surface and its view together, so adding one still costs a single line and
/// FR-009 holds.
///
/// The exit-animation snapshot (contract A1–A3) is not here yet either; it arrives with T036, when
/// `ClosingOverlay` is collapsed into this trait. It is called out rather than forgotten because
/// it is the riskiest obligation in the feature.
pub trait FloatingSurface {
    /// This surface's stable identity.
    fn id(&self) -> SurfaceId;

    /// Which band of the z-order it belongs to.
    fn layer(&self) -> Layer;

    /// What closes it.
    fn dismissal(&self) -> DismissalRules;
}
