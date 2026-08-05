//! `cdk` — the behaviour layer (feature 017, FR-006, FR-007).
//!
//! Named after Angular's Component Dev Kit, and split from [`material`](crate::ui::material) for
//! the same reason: *how a thing behaves* and *how a thing looks* change for different reasons and
//! at different rates. Everything here decides behaviour — where a floating surface sits, what
//! captures input, when it closes, what order it stacks in — and nothing here decides appearance.
//!
//! The rule is enforced, not merely stated: `tests/cdk_no_appearance.rs` reads these sources and
//! fails if any of them names a colour role, a type role, a shape size or the styling layer. When
//! a surface needs a scrim colour, the material layer resolves it and hands it over already
//! resolved.
//!
//! Consequence worth stating plainly: a change to how dialogs *look* never opens a file in here,
//! and a change to how they *behave* never opens a style function.

pub mod motion;
pub mod overlay;
pub mod ripple;
pub mod typeahead;
