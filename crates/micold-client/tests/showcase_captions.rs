//! Each section says which of its states are live (feature 020, T028 — FR-005).
//!
//! Hover, pressed and focus cannot be posed — they follow the pointer and the keyboard — so the gallery
//! exercises them on the real instances rather than faking them with static approximations (FR-004). But
//! that leaves a reader with a question the page has to answer: *is this state missing, or is it merely
//! live?* FR-005 answers it by making every section name what has to be produced by hand.
//!
//! `Entry::interactive` is what makes that answer checkable. The two must agree in both directions:
//!
//! - an interactive entry with an empty `live` list is a caption bug that tells a developer nothing is
//!   expected where something is;
//! - a non-interactive entry with a populated `live` list promises a response that never comes, which
//!   is worse — it sends someone hunting for behaviour that does not exist.
//!
//! **A note on Red.** This gate arrived green, and the record should say so plainly: the `live` lists
//! were populated in the same pass that wrote the sections, so the "fails before `live` is populated"
//! Red the plan expected had already gone by. The demonstrations at the bottom are therefore the honest
//! evidence that the rule fires. A gate nobody has watched fail is the thing this feature exists to
//! remove, and that applies to this one too.

use micold_client::showcase::catalogue::COMPONENTS;

/// One entry, reduced to what this rule needs.
#[derive(Debug, Clone)]
struct Captioned {
    name: String,
    interactive: bool,
    live: Vec<String>,
}

fn real() -> Vec<Captioned> {
    COMPONENTS
        .iter()
        .map(|e| Captioned {
            name: format!("{}::{}", e.module, e.component),
            interactive: e.interactive,
            live: e.live.iter().map(|s| s.to_string()).collect(),
        })
        .collect()
}

/// **Agreement** — a non-empty `live` if and only if `interactive`.
fn disagreements(entries: &[Captioned]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|e| match (e.interactive, e.live.is_empty()) {
            (true, true) => Some(format!(
                "{} is interactive but names no live states. A reader cannot tell whether hover and \
                 pressed are missing or merely live, which is the confusion FR-005 exists to remove.",
                e.name
            )),
            (false, false) => Some(format!(
                "{} names live states ({}) but is not interactive — the caption promises a response \
                 that never comes.",
                e.name,
                e.live.join(", ")
            )),
            _ => None,
        })
        .collect()
}

/// **Non-vacuity** — how many entries are interactive.
///
/// The agreement rule is satisfied by a catalogue in which everything claims to be static, which would
/// say nothing at all: the same vacuous pass FR-016 guards against elsewhere, in miniature.
fn interactive_count(entries: &[Captioned]) -> usize {
    entries.iter().filter(|e| e.interactive).count()
}

// ---------------------------------------------------------------------------------------------
// The real catalogue
// ---------------------------------------------------------------------------------------------

#[test]
fn every_entry_agrees_with_itself_about_what_is_live() {
    let found = disagreements(&real());
    assert!(
        found.is_empty(),
        "FR-005: a section states which of its states are posed as separate instances and which must \
         be exercised live, so a state absent from the page is understood as live rather than \
         missing:\n{}",
        found
            .iter()
            .map(|f| format!("  {f}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_catalogue_is_not_empty_and_something_in_it_is_interactive() {
    let entries = real();
    assert!(!entries.is_empty(), "the catalogue lists no components");
    assert!(
        interactive_count(&entries) > 0,
        "no entry is interactive, so the agreement rule above holds over nothing — and a component \
         library with no interactive component would be a surprising thing to have"
    );
}

/// Worth stating as a number rather than a boolean: SC-005 asks that *every* interactive component in
/// the library be hoverable and pressable within one scrolling page, and a catalogue where a single
/// entry was interactive would satisfy the guard above while making that pass meaningless.
#[test]
fn a_substantial_share_of_the_gallery_is_interactive() {
    let entries = real();
    let interactive = interactive_count(&entries);
    assert!(
        interactive >= 10,
        "only {interactive} of {} entries are interactive; SC-005's single-pass hover-and-press walk \
         needs the library's interactive components actually to be on the page",
        entries.len()
    );
}

// ---------------------------------------------------------------------------------------------
// The demonstrations: the rule really does fire
// ---------------------------------------------------------------------------------------------

fn entry(name: &str, interactive: bool, live: &[&str]) -> Captioned {
    Captioned {
        name: name.to_string(),
        interactive,
        live: live.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn an_interactive_entry_with_no_caption_fails_and_names_it() {
    let found = disagreements(&[entry("material/button.rs::Button", true, &[])]);
    assert_eq!(found.len(), 1, "got {found:?}");
    assert!(
        found[0].contains("Button") && found[0].contains("names no live states"),
        "{}",
        found[0]
    );
}

#[test]
fn a_static_entry_that_promises_a_response_fails_and_names_it() {
    let found = disagreements(&[entry("material/divider.rs::Divider", false, &["hover"])]);
    assert_eq!(found.len(), 1, "got {found:?}");
    assert!(
        found[0].contains("Divider") && found[0].contains("never comes"),
        "{}",
        found[0]
    );
}

#[test]
fn a_consistent_pair_produces_nothing() {
    assert!(disagreements(&[
        entry("material/button.rs::Button", true, &["hover", "pressed"]),
        entry("material/divider.rs::Divider", false, &[]),
    ])
    .is_empty());
}

#[test]
fn a_catalogue_that_claims_to_be_entirely_static_is_vacuous() {
    let all_static = [entry("a::A", false, &[]), entry("b::B", false, &[])];
    assert!(
        disagreements(&all_static).is_empty(),
        "precondition: it satisfies the agreement rule"
    );
    assert_eq!(
        interactive_count(&all_static),
        0,
        "…and says nothing, which is why the guard above exists"
    );
}
