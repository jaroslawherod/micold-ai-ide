//! What each named role actually resolves to (feature 018, T017–T021 — FR-007, FR-010).
//!
//! `TypeRole` is the application's whole typographic vocabulary: twelve names, each standing for one
//! of Material's fifteen roles. The names are what call sites say, so the *mapping* is the thing
//! that decides how the application reads — and it is one `match` arm away from being changed by
//! accident while renaming something nearby.
//!
//! So it is pinned here rather than left to be re-derived by reading call sites. A deliberate
//! re-anchoring updates this file and says why in the commit; an accidental one fails the build.
//!
//! # Why the weights matter more than the sizes
//!
//! Several roles share a size and differ only in weight — `Caption` and `Label` are both 12dp,
//! `Body` and `Action` are both 14dp. That is not redundancy, it is the distinction Material draws
//! between text you *read* (prose, weight 400) and text you *scan* (interface labels, weight 500).
//! A mapping that collapsed either pair would look almost right and would quietly undo the split,
//! which is precisely the kind of change a size-only check would wave through.

use super::TypeRole;
use micold_core::tokens::typography;

/// The mapping, spelled out. Left column is what a call site says; right is contract §2.2's name.
const MAPPING: &[(TypeRole, &str)] = &[
    (TypeRole::Display, "headline_large"),
    (TypeRole::Headline, "headline_small"),
    (TypeRole::Title, "title_large"),
    (TypeRole::Section, "title_medium"),
    (TypeRole::Body, "body_medium"),
    (TypeRole::Caption, "body_small"),
    (TypeRole::Action, "label_large"),
    (TypeRole::Label, "label_medium"),
    (TypeRole::SidebarName, "body_small"),
    (TypeRole::SidebarTag, "label_small"),
    (TypeRole::SidebarSession, "body_small"),
    // The current session's row: `body_small`'s size and line height at weight 500 — which is
    // `label_medium`, already in the scale. Pinned here so "the mark does not depend on colour
    // alone" (feature 024, FR-003a) cannot be quietly undone by re-anchoring it onto the same
    // role as every other session row.
    (TypeRole::SidebarSessionCurrent, "label_medium"),
];

#[test]
fn every_role_resolves_to_the_material_role_it_names() {
    for (role, expected) in MAPPING {
        assert_eq!(
            role.resolved().name,
            *expected,
            "`TypeRole::{}` resolves to `{}`, but the application's typographic vocabulary says it \
             is `{expected}` (contract §2.2). If this is a deliberate re-anchoring, update the \
             table here and say why; if it is a side effect of an edit nearby, it is a bug.",
            role.name(),
            role.resolved().name,
        );
    }
}

/// The table covers the enum. Without this, a new variant is simply absent from the check above and
/// the pin silently stops covering it.
#[test]
fn the_mapping_covers_every_role() {
    for role in TypeRole::ALL {
        assert!(
            MAPPING.iter().any(|(r, _)| *r == role),
            "`TypeRole::{}` is not in this file's mapping table, so nothing pins what it resolves \
             to. Add it.",
            role.name()
        );
    }
    assert_eq!(
        MAPPING.len(),
        TypeRole::ALL.len(),
        "the mapping table and `TypeRole::ALL` disagree on how many roles there are"
    );
}

/// Every role lands *inside* the scale rather than inventing a size (FR-007).
///
/// The sidebar roles are the ones at risk: contract §2.4 maps the sidebar's ~80% density onto the
/// nearest smaller role in the scale precisely so it does not become three loose numbers again,
/// which is what feature 003 had.
#[test]
fn no_role_invents_a_size_outside_the_scale() {
    for role in TypeRole::ALL {
        let resolved = role.resolved();
        assert!(
            typography::ALL.contains(&resolved),
            "`TypeRole::{}` resolves to {}dp/{}dp at weight {}, which is not one of the fifteen \
             roles. Every role resolves into the scale — a size that sits outside it is the drift \
             the scale exists to prevent.",
            role.name(),
            resolved.size,
            resolved.line_height,
            resolved.weight,
        );
    }
}

/// The two same-size pairs really do differ in weight.
///
/// This is the split the whole vocabulary rests on, and it is invisible to a size comparison: if
/// `Caption` were mapped onto `label_small` or `Action` onto `body_medium`, every size in the
/// application would still be right and every sentence would be set in the wrong voice.
#[test]
fn prose_and_interface_labels_differ_in_weight_at_the_same_size() {
    for (prose, label) in [
        (TypeRole::Caption, TypeRole::Label),
        (TypeRole::Body, TypeRole::Action),
    ] {
        assert_eq!(
            prose.resolved().size,
            label.resolved().size,
            "`{}` and `{}` are meant to be the same size and differ only in weight",
            prose.name(),
            label.name()
        );
        assert_eq!(
            prose.resolved().weight,
            400,
            "`{}` is prose and must be weight 400 — a sentence at 500 reads as shouting",
            prose.name()
        );
        assert_eq!(
            label.resolved().weight,
            500,
            "`{}` names something in the interface and must be weight 500; Material's label roles \
             are medium, and button text at the body weight is one of the loudest reasons an \
             application does not read as Material",
            label.name()
        );
    }
}

/// The current session's row is marked by weight, not by size (feature 024, FR-003a).
///
/// This is the non-colour half of the current-session mark, and it is the half a reviewer cannot
/// check by looking: the row is also filled with `secondary_container`, so on a colour display the
/// weight is the less obvious of the two signals. It is the only one that survives greyscale, a
/// colour-vision deficit, and the hover state layer that shares the row.
///
/// Same size and line height, because a heavier *and* larger name would reflow the list as the
/// current session moved — the mark would then move rows that have nothing to do with it.
#[test]
fn the_current_session_row_differs_from_its_siblings_only_in_weight() {
    let ordinary = TypeRole::SidebarSession.resolved();
    let current = TypeRole::SidebarSessionCurrent.resolved();

    assert_eq!(
        (ordinary.size, ordinary.line_height),
        (current.size, current.line_height),
        "the current session's name is the same size as every other session name; only its weight \
         differs, so marking it cannot reflow the rows around it"
    );
    assert_eq!(
        (ordinary.weight, current.weight),
        (400, 500),
        "and the difference is exactly the scale's two weights — an ordinary session row at 400, \
         the current one at 500. Equal weights would leave the mark carried by colour alone, which \
         is what FR-003a forbids"
    );
}

/// Only the two weights that ship are reachable (contract §2.1).
///
/// Roboto ships as exactly two static instances. A role at any other weight would be resolved by
/// the font matcher to whichever face it considered closest — silently, and differently depending
/// on the platform's matcher.
#[test]
fn every_role_is_a_weight_that_ships() {
    for role in TypeRole::ALL {
        let weight = role.resolved().weight;
        assert!(
            weight == 400 || weight == 500,
            "`TypeRole::{}` is weight {weight}, but only Roboto Regular (400) and Medium (500) \
             ship. The matcher would substitute a face rather than fail.",
            role.name()
        );
    }
}

/// Line height is never smaller than the size, at any role.
///
/// A line height below the size overlaps consecutive lines. Every role states both numbers
/// independently, so this is a property worth holding rather than assuming.
#[test]
fn no_role_sets_a_line_height_below_its_size() {
    for role in TypeRole::ALL {
        let r = role.resolved();
        assert!(
            r.line_height >= r.size,
            "`TypeRole::{}` is {}dp on a {}dp line — consecutive lines would overlap",
            role.name(),
            r.size,
            r.line_height
        );
    }
}
