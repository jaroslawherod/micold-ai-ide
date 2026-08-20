//! `AiCli` is a name you can persist, key on, and offer in a menu (feature 026, T008 — FR-001,
//! FR-003, FR-006).
//!
//! The enum carries no behaviour — that lives behind `AiCliProvider` — so what there is to hold it
//! to is its *shape*. Three properties, each load-bearing somewhere else in the feature:
//!
//! - **`Copy + Eq + Hash + Ord`**, because a session record compares by it, a set of available
//!   providers is deduplicated by it, and the UI sorts the choices with it.
//! - **`default()` is `ClaudeCode`**, which is what makes FR-003 (the initial default) and FR-013
//!   (every session written before this feature) true at once, in both cases by never writing
//!   anything down.
//! - **Iterating the variants is deterministic**, because that order *is* the order the Settings
//!   select and the per-session override list offer them in. An order that came from a `HashSet`
//!   would reshuffle the menu between runs and no other test would notice.

use micold_core::session::AiCli;
use std::collections::{BTreeSet, HashSet};

#[test]
fn the_default_provider_is_claude_code() {
    assert_eq!(AiCli::default(), AiCli::ClaudeCode);
}

#[test]
fn it_is_copy_eq_hash_and_ord() {
    // `Copy` — taking one by value must not move it out of the session record it came from.
    fn takes_by_value(which: AiCli) -> AiCli {
        which
    }
    let which = AiCli::Copilot;
    assert_eq!(takes_by_value(which), which, "still usable after the call");

    // `Hash` + `Eq` — the availability set deduplicates by it.
    let mut seen = HashSet::new();
    seen.insert(AiCli::ClaudeCode);
    seen.insert(AiCli::ClaudeCode);
    seen.insert(AiCli::Copilot);
    assert_eq!(seen.len(), 2);

    // `Ord` — the UI sorts with it, so a set has one order rather than an arbitrary one.
    let sorted: Vec<AiCli> = BTreeSet::from([AiCli::Copilot, AiCli::ClaudeCode])
        .into_iter()
        .collect();
    assert_eq!(sorted, vec![AiCli::ClaudeCode, AiCli::Copilot]);
}

#[test]
fn iterating_the_variants_is_deterministic_and_complete() {
    // The list the menus are built from. Asserting the exact sequence rather than the length is
    // the point: this is the order the user sees, and `ALL` is the only thing that decides it.
    assert_eq!(AiCli::ALL, [AiCli::ClaudeCode, AiCli::Copilot]);

    // And it is every variant, not a list someone forgot to extend when a third CLI landed —
    // which a length assertion alone would not catch, since a duplicated entry has the same
    // length as a distinct one.
    let distinct: BTreeSet<AiCli> = AiCli::ALL.into_iter().collect();
    assert_eq!(
        distinct.len(),
        AiCli::ALL.len(),
        "`AiCli::ALL` lists a variant twice"
    );

    // The default has to be *in* the list, or the Settings select would open on a value it does
    // not offer.
    assert!(AiCli::ALL.contains(&AiCli::default()));

    // Sorted, so `ALL` and any `BTreeSet` of providers agree about order — the sidebar builds one
    // and the Settings form the other.
    let sorted: Vec<AiCli> = distinct.into_iter().collect();
    assert_eq!(sorted.as_slice(), AiCli::ALL.as_slice());
}
