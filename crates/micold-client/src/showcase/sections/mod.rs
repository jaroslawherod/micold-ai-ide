//! The catalogue's render functions, grouped so section work can be split (feature 020).
//!
//! Each module here builds the instances for a group of entries. The entries themselves live in one
//! list, [`super::catalogue::COMPONENTS`], because "which components does the gallery contain" should
//! be one file to read rather than six to reconcile.

pub mod atoms;
pub mod controls;
pub mod floating;
pub mod motion;
pub mod surfaces;
pub mod terminal;
