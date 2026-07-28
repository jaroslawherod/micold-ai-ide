//! The component showcase (feature 020): every shared component on one page.
//!
//! A developer who wants to see what a component looks like should not have to launch the
//! application, let it spawn a session daemon, open a project, create a worktree, and then find a
//! screen that happens to use the component — and if the component only appears in an error state,
//! produce that state first. This renders the whole library at once, live and interactive, in either
//! colour scheme, with no daemon, no git repository and no saved application state.
//!
//! # What it is not
//!
//! It is not a second implementation of anything (FR-021). It composes components from
//! [`crate::ui::material`] and [`crate::ui::cdk`] exactly as a feature module does, and supplies
//! sample content; it holds no styling, no layout rule and no interaction behaviour that belongs in
//! the library. `tests/material_boundary.rs` scans this directory at the same zero budgets it holds
//! `src/ui/`'s feature modules to, so a hand-styled copy of a component is a build failure rather
//! than a review note. Where the gallery reveals that something is missing from the library, the fix
//! is to add it to the library.
//!
//! It is also not installed. It is a second binary (`micold-showcase`), absent from the packaging
//! manifest and the desktop entry, and `tests/packaging_excludes_showcase.rs` fails the build if
//! either ever names it.
//!
//! # How it holds itself complete
//!
//! [`catalogue`] is one list of entries. The page is that list traversed, and each entry carries the
//! function that renders its own instances — so an entry cannot exist without something to show, nor
//! an instance appear without being declared. `tests/showcase_completeness.rs` reads the same list
//! and fails, naming the component, when the library and the gallery disagree in either direction.
//!
//! # Layout of this module
//!
//! - [`state`] — the render-free reducer. Every state transition lives here and is tested directly.
//! - [`catalogue`] — the one list: components, motion entries, and recorded exemptions.
//! - [`sections`] — the render functions the catalogue's entries point at, grouped by kind.
//! - [`samples`] — fixed, invented content for components that would otherwise need real data.
//! - [`gallery`] — the view: the catalogue, traversed.
//!
//! `main.rs` sits beside these as the binary's entry point. It is not part of this module tree.

pub mod catalogue;
pub mod gallery;
pub mod samples;
pub mod sections;
pub mod state;
