//! The gallery cannot fall out of date unnoticed (feature 020, T040–T045 — FR-011–FR-016, FR-013a,
//! SC-002, SC-003, SC-003a, SC-004).
//!
//! A catalogue that silently omits things is worse than no catalogue, because it is consulted as though
//! it were complete. This compares what the library contains against what the gallery declares, and
//! fails in **both** directions: a component with no entry, and an entry naming a component that no
//! longer exists. Either way the failure names the thing.
//!
//! # Where the two sides come from
//!
//! The library side is [`inventory`] — the same scanner `tests/material_builder_api.rs` uses, so the
//! two gates cannot disagree about what a component is (FR-014). The gallery side is read as **data**,
//! through `micold_client::showcase::catalogue`'s own `const`s, rather than by scanning the gallery's
//! source: a source scan can only approximate what the gallery contains, and this is the feature's
//! load-bearing claim.
//!
//! # Why every rule is a function
//!
//! SC-004 requires each failure direction to be *observed*, not assumed. So each rule takes its two
//! sides as arguments and the tests at the bottom drive them against deliberately-broken synthetic
//! inputs. The alternative — breaking the tree by hand, watching it fail, fixing it — proves the check
//! works for as long as it takes to read the commit. This re-proves it on every run.
//!
//! # What this check deliberately does not reach
//!
//! - **Three element-producing free functions** — `material::menu_panel`, `glyph::icon` and
//!   `glyph::icon_colored`. They are neither a `pub struct` (so the component definition does not see
//!   them) nor animation helpers (so FR-013a's category does not either). FR-014 widens the definition
//!   by exactly one category and says so; these three stay outside both. They are not invisible in
//!   practice — `Glyph` is a component, and the popover panels are rendered by the overlay entries that
//!   use `menu_panel` — but no check holds them, and pretending otherwise would be the vacuous coverage
//!   claim this feature exists to remove. If that gap matters later, it is a third category, added
//!   deliberately.
//! - **Density.** `Entry::density` is empty on every entry because no component honours a density step:
//!   the scale is feature 018's FR-026b and this feature lands first (FR-003a, dormant). When 018 adds
//!   the axis it adds a row per honouring component, and the rule that holds them belongs in that
//!   change, not this one.
//! - **Appearance.** Image diffing is out of scope. This holds the gallery *complete*; a person holds
//!   it *correct*, via `quickstart.md` §B.

mod inventory;

use std::collections::{BTreeMap, BTreeSet};

use micold_client::showcase::catalogue::{Section, COMPONENTS, EXEMPTIONS, MOTION};

/// The module a component whose appearance *is* an animation is declared in.
const ANIMATION_MODULE: &str = "material/animation.rs";

// ---------------------------------------------------------------------------------------------
// The two sides
// ---------------------------------------------------------------------------------------------

/// What the library contains, as names.
#[derive(Debug, Default)]
struct Library {
    /// Every component, keyed by `(module, name)`.
    components: BTreeSet<(String, String)>,
    /// Every `pub enum` variant name, from anywhere in the library.
    variants: BTreeSet<String>,
    /// Every animation helper.
    animations: BTreeSet<String>,
}

impl Library {
    fn real() -> Self {
        Self {
            components: inventory::components(),
            variants: inventory::enums()
                .into_iter()
                .flat_map(|e| e.variants)
                .collect(),
            animations: inventory::animation_helpers(),
        }
    }
}

/// One gallery entry, as the check reads it.
#[derive(Debug, Clone)]
struct Entry {
    module: String,
    component: String,
    variants: Vec<String>,
    in_motion_section: bool,
}

/// What the gallery declares, as names.
#[derive(Debug, Default)]
struct Catalogue {
    entries: Vec<Entry>,
    /// The animation each motion entry names.
    motion: Vec<String>,
    /// `(module, component, reason)`.
    exemptions: Vec<(String, String, String)>,
}

impl Catalogue {
    fn real() -> Self {
        Self {
            entries: COMPONENTS
                .iter()
                .map(|e| Entry {
                    module: e.module.to_string(),
                    component: e.component.to_string(),
                    variants: e.variants.iter().map(|v| v.to_string()).collect(),
                    in_motion_section: e.section == Section::Motion,
                })
                .collect(),
            motion: MOTION.iter().map(|m| m.animation.to_string()).collect(),
            exemptions: EXEMPTIONS
                .iter()
                .map(|x| {
                    (
                        x.module.to_string(),
                        x.component.to_string(),
                        x.reason.to_string(),
                    )
                })
                .collect(),
        }
    }

    fn keys(&self) -> BTreeSet<(String, String)> {
        self.entries
            .iter()
            .map(|e| (e.module.clone(), e.component.clone()))
            .collect()
    }

    fn exempt_keys(&self) -> BTreeSet<(String, String)> {
        self.exemptions
            .iter()
            .map(|(m, c, _)| (m.clone(), c.clone()))
            .collect()
    }
}

// ---------------------------------------------------------------------------------------------
// The rules (contracts/completeness-check.md §2)
// ---------------------------------------------------------------------------------------------

/// **C1** — every component in the library has an entry or a recorded exemption (FR-011, SC-002).
fn c1_nothing_is_missing(lib: &Library, cat: &Catalogue) -> Vec<String> {
    let covered: BTreeSet<_> = cat.keys().union(&cat.exempt_keys()).cloned().collect();
    lib.components
        .difference(&covered)
        .map(|(module, component)| {
            format!(
                "{module}::{component} exists in the library and has no instance in the gallery. Add \
                 an `Entry` for it, or an `Exemption` with the reason it cannot be shown (FR-011)."
            )
        })
        .collect()
}

/// **C2** — every entry names a component the library still has (FR-012).
fn c2_nothing_is_stale(lib: &Library, cat: &Catalogue) -> Vec<String> {
    cat.keys()
        .difference(&lib.components)
        .map(|(module, component)| {
            format!(
                "the gallery lists {module}::{component}, which the library no longer contains. A \
                 catalogue that outlives its contents misleads in the opposite direction — remove the \
                 entry (FR-012)."
            )
        })
        .collect()
}

/// **C3** — every library variant name is named by some entry (FR-013, SC-003).
///
/// Library-wide rather than per-module: `cdk/overlay.rs` declares `Anchor` and both of its components
/// are exempted as behaviour-layer hosts, so a module-scoped rule would be unsatisfiable there. An
/// anchor is posed where it is actually visible — the floating section, because every floating
/// component converts into a `cdk::Surface` with one.
fn c3_every_variant_is_posed(lib: &Library, cat: &Catalogue) -> Vec<String> {
    let posed: BTreeSet<String> = cat
        .entries
        .iter()
        .flat_map(|e| e.variants.iter().cloned())
        .collect();
    lib.variants
        .difference(&posed)
        .map(|variant| {
            format!(
                "the variant `{variant}` has no instance in the gallery. Every named variant is posed \
                 as a separate instance, so a difference between two of them is visible by comparison \
                 rather than by memory (FR-013)."
            )
        })
        .collect()
}

/// **C4** — every variant an entry names still exists (FR-013).
fn c4_no_variant_outlives_itself(lib: &Library, cat: &Catalogue) -> Vec<String> {
    let mut out = Vec::new();
    for entry in &cat.entries {
        for variant in &entry.variants {
            if !lib.variants.contains(variant) {
                out.push(format!(
                    "{}::{} names the variant `{variant}`, which no library enum declares any more \
                     (FR-013).",
                    entry.module, entry.component
                ));
            }
        }
    }
    out
}

/// **C5** — every animation helper has exactly one motion entry (FR-013a, SC-003a).
fn c5_every_animation_is_shown(lib: &Library, cat: &Catalogue) -> Vec<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for name in &cat.motion {
        *counts.entry(name.as_str()).or_default() += 1;
    }
    let mut out = Vec::new();
    for animation in &lib.animations {
        match counts.get(animation.as_str()).copied().unwrap_or(0) {
            0 => out.push(format!(
                "the animation `{animation}` has no entry in the motion section. An animation that can \
                 only be seen by catching it once is not reviewable (FR-013a)."
            )),
            1 => {}
            n => out.push(format!(
                "the animation `{animation}` has {n} motion entries; one demonstration per animation."
            )),
        }
    }
    out
}

/// **C6** — every motion entry names an animation that still exists (FR-013a).
fn c6_no_motion_entry_outlives_itself(lib: &Library, cat: &Catalogue) -> Vec<String> {
    cat.motion
        .iter()
        .filter(|name| !lib.animations.contains(name.as_str()))
        .map(|name| {
            format!(
                "the motion section lists `{name}`, which the library no longer provides (FR-013a)."
            )
        })
        .collect()
}

/// **C7** — every exemption names something that exists, and says why (FR-015).
fn c7_exemptions_are_live_and_reasoned(lib: &Library, cat: &Catalogue) -> Vec<String> {
    let mut out = Vec::new();
    for (module, component, reason) in &cat.exemptions {
        if !lib
            .components
            .contains(&(module.clone(), component.clone()))
        {
            out.push(format!(
                "{module}::{component} is on the exemption list but no longer exists. An exemption that \
                 outlives its component is a stale claim (FR-015)."
            ));
        }
        if reason.trim().is_empty() {
            out.push(format!(
                "{module}::{component} is exempted with no reason. An exemption without one is \
                 indistinguishable from an oversight (FR-015)."
            ));
        }
    }
    out
}

/// **C8** — nothing is both listed and exempted, and each key appears once (FR-011, FR-015).
fn c8_the_partition_is_clean(cat: &Catalogue) -> Vec<String> {
    let mut out = Vec::new();
    for key in cat.keys().intersection(&cat.exempt_keys()) {
        out.push(format!(
            "{}::{} is both posed and exempted — the gallery cannot claim both that it shows a \
             component and that it cannot.",
            key.0, key.1
        ));
    }
    let mut seen: BTreeMap<(String, String), usize> = BTreeMap::new();
    for entry in &cat.entries {
        *seen
            .entry((entry.module.clone(), entry.component.clone()))
            .or_default() += 1;
    }
    for (key, count) in seen {
        if count > 1 {
            out.push(format!(
                "{}::{} has {count} entries; one per component.",
                key.0, key.1
            ));
        }
    }
    out
}

/// **C9** — a motion-section entry is a component the library implements as an animation, and vice
/// versa (FR-007a).
///
/// The two have to agree, in both directions. A wrapper posed among the static components would be a
/// picture of a transition; a static component in the motion section would sit under a replay control
/// that does nothing.
fn c9_the_motion_section_holds_the_animations(cat: &Catalogue) -> Vec<String> {
    let mut out = Vec::new();
    for entry in &cat.entries {
        let is_animation = entry.module == ANIMATION_MODULE;
        if is_animation && !entry.in_motion_section {
            out.push(format!(
                "{}::{} is an animation posed among the static components — a still of a transition is \
                 a picture of it (FR-007a). Move it to the motion section.",
                entry.module, entry.component
            ));
        }
        if !is_animation && entry.in_motion_section {
            out.push(format!(
                "{}::{} is in the motion section but is not an animation, so its replay control has \
                 nothing to replay.",
                entry.module, entry.component
            ));
        }
    }
    out
}

/// Every rule at once, for the headline test.
fn all_rules(lib: &Library, cat: &Catalogue) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(c1_nothing_is_missing(lib, cat));
    out.extend(c2_nothing_is_stale(lib, cat));
    out.extend(c3_every_variant_is_posed(lib, cat));
    out.extend(c4_no_variant_outlives_itself(lib, cat));
    out.extend(c5_every_animation_is_shown(lib, cat));
    out.extend(c6_no_motion_entry_outlives_itself(lib, cat));
    out.extend(c7_exemptions_are_live_and_reasoned(lib, cat));
    out.extend(c8_the_partition_is_clean(cat));
    out.extend(c9_the_motion_section_holds_the_animations(cat));
    out
}

// ---------------------------------------------------------------------------------------------
// The real library against the real gallery
// ---------------------------------------------------------------------------------------------

/// SC-002, SC-003 and SC-003a in one statement.
#[test]
fn the_gallery_is_complete() {
    let found = all_rules(&Library::real(), &Catalogue::real());
    assert!(
        found.is_empty(),
        "the gallery and the library disagree ({} finding(s)):\n{}",
        found.len(),
        found
            .iter()
            .map(|f| format!("  {f}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Each rule separately, so a failure points at a requirement rather than at a pile.
#[test]
fn every_component_has_an_instance_or_a_recorded_exemption() {
    assert_eq!(
        c1_nothing_is_missing(&Library::real(), &Catalogue::real()),
        Vec::<String>::new()
    );
}

#[test]
fn no_entry_outlives_the_component_it_names() {
    assert_eq!(
        c2_nothing_is_stale(&Library::real(), &Catalogue::real()),
        Vec::<String>::new()
    );
}

#[test]
fn every_named_variant_has_an_instance() {
    assert_eq!(
        c3_every_variant_is_posed(&Library::real(), &Catalogue::real()),
        Vec::<String>::new()
    );
    assert_eq!(
        c4_no_variant_outlives_itself(&Library::real(), &Catalogue::real()),
        Vec::<String>::new()
    );
}

#[test]
fn every_animation_has_a_replayable_entry() {
    assert_eq!(
        c5_every_animation_is_shown(&Library::real(), &Catalogue::real()),
        Vec::<String>::new()
    );
    assert_eq!(
        c6_no_motion_entry_outlives_itself(&Library::real(), &Catalogue::real()),
        Vec::<String>::new()
    );
}

#[test]
fn every_exemption_is_live_and_carries_its_reason() {
    assert_eq!(
        c7_exemptions_are_live_and_reasoned(&Library::real(), &Catalogue::real()),
        Vec::<String>::new()
    );
}

#[test]
fn the_catalogue_partition_is_clean() {
    assert_eq!(
        c8_the_partition_is_clean(&Catalogue::real()),
        Vec::<String>::new()
    );
}

#[test]
fn the_motion_section_holds_exactly_the_animations() {
    assert_eq!(
        c9_the_motion_section_holds_the_animations(&Catalogue::real()),
        Vec::<String>::new()
    );
}

// ---------------------------------------------------------------------------------------------
// §3 The vacuity guards (FR-016)
// ---------------------------------------------------------------------------------------------

/// **V1** — a moved library must fail rather than pass over an empty set.
///
/// A floor, not a count. It exists so a relocation fails; it must never be tightened into a number
/// somebody has to edit every time a component is added.
#[test]
fn v1_the_inventory_finds_the_library() {
    let lib = Library::real();
    assert!(
        lib.components.len() >= 30,
        "found only {} components — the library moved or was renamed, and every rule above would pass \
         over almost nothing (FR-016)",
        lib.components.len()
    );
}

/// **V2** — named landmarks, one from each layer. A scan that finds *some* files but not the cdk has
/// half-moved, and would report a clean bill of health for the half it can see.
#[test]
fn v2_both_library_layers_are_present() {
    let lib = Library::real();
    for (module, component) in [
        ("material/surface.rs", "Surface"),
        ("cdk/overlay.rs", "Overlay"),
    ] {
        assert!(
            lib.components
                .contains(&(module.to_string(), component.to_string())),
            "expected {module}::{component} in the inventory; found {:?}",
            lib.components
        );
    }
}

/// **V3** — the motion category is enumerated from one file. If it moves, C5 would hold vacuously.
#[test]
fn v3_the_animation_module_is_where_it_is_expected() {
    let helpers = inventory::animation_helpers();
    assert!(
        !helpers.is_empty(),
        "no animation helpers found — if `{ANIMATION_MODULE}` moved, the inventory must move with it, \
         or the whole motion category goes unchecked (FR-016)"
    );
}

/// **V4** — a gallery emptied by a refactor must fail rather than agree with an empty library.
#[test]
fn v4_the_catalogue_is_not_empty() {
    let cat = Catalogue::real();
    assert!(!cat.entries.is_empty(), "the catalogue lists no components");
    assert!(!cat.motion.is_empty(), "the motion section is empty");
}

// ---------------------------------------------------------------------------------------------
// §5 The demonstrations: each rule really does fail, and names the thing (SC-004)
// ---------------------------------------------------------------------------------------------

fn lib_of(components: &[(&str, &str)], variants: &[&str], animations: &[&str]) -> Library {
    Library {
        components: components
            .iter()
            .map(|(m, c)| (m.to_string(), c.to_string()))
            .collect(),
        variants: variants.iter().map(|v| v.to_string()).collect(),
        animations: animations.iter().map(|a| a.to_string()).collect(),
    }
}

fn entry_of(module: &str, component: &str, variants: &[&str], motion: bool) -> Entry {
    Entry {
        module: module.to_string(),
        component: component.to_string(),
        variants: variants.iter().map(|v| v.to_string()).collect(),
        in_motion_section: motion,
    }
}

/// SC-004, first direction: adding a component to the library without adding it to the gallery fails
/// the build, and the failure names the component.
#[test]
fn a_component_with_no_entry_fails_and_names_it() {
    let lib = lib_of(
        &[
            ("material/button.rs", "Button"),
            ("material/new.rs", "Shiny"),
        ],
        &[],
        &[],
    );
    let cat = Catalogue {
        entries: vec![entry_of("material/button.rs", "Button", &[], false)],
        ..Default::default()
    };
    let found = c1_nothing_is_missing(&lib, &cat);
    assert_eq!(
        found.len(),
        1,
        "expected one missing component, got {found:?}"
    );
    assert!(
        found[0].contains("material/new.rs::Shiny"),
        "the failure must name the component: {}",
        found[0]
    );
}

/// SC-004, second direction: deleting a component the gallery lists fails the build, and the message
/// names the stale entry.
#[test]
fn an_entry_for_a_deleted_component_fails_and_names_it() {
    let lib = lib_of(&[("material/button.rs", "Button")], &[], &[]);
    let cat = Catalogue {
        entries: vec![
            entry_of("material/button.rs", "Button", &[], false),
            entry_of("material/gone.rs", "Removed", &[], false),
        ],
        ..Default::default()
    };
    let found = c2_nothing_is_stale(&lib, &cat);
    assert_eq!(found.len(), 1, "expected one stale entry, got {found:?}");
    assert!(
        found[0].contains("material/gone.rs::Removed"),
        "the failure must name the stale entry: {}",
        found[0]
    );
}

/// An exemption also satisfies C1 — that is the point of FR-015.
#[test]
fn an_exempted_component_satisfies_the_first_rule() {
    let lib = lib_of(&[("cdk/overlay.rs", "Overlay")], &[], &[]);
    let cat = Catalogue {
        exemptions: vec![(
            "cdk/overlay.rs".to_string(),
            "Overlay".to_string(),
            "no appearance of its own".to_string(),
        )],
        ..Default::default()
    };
    assert!(c1_nothing_is_missing(&lib, &cat).is_empty());
}

#[test]
fn a_variant_with_no_instance_fails_and_names_it() {
    let lib = lib_of(&[], &["Filled", "Outlined", "Tonal"], &[]);
    let cat = Catalogue {
        entries: vec![entry_of(
            "material/button.rs",
            "Button",
            &["Filled", "Outlined"],
            false,
        )],
        ..Default::default()
    };
    let found = c3_every_variant_is_posed(&lib, &cat);
    assert_eq!(
        found.len(),
        1,
        "expected one missing variant, got {found:?}"
    );
    assert!(found[0].contains("Tonal"), "{}", found[0]);
}

#[test]
fn an_instance_of_a_vanished_variant_fails_and_names_it() {
    let lib = lib_of(&[], &["Filled"], &[]);
    let cat = Catalogue {
        entries: vec![entry_of(
            "material/button.rs",
            "Button",
            &["Filled", "Ghost"],
            false,
        )],
        ..Default::default()
    };
    let found = c4_no_variant_outlives_itself(&lib, &cat);
    assert_eq!(found.len(), 1, "got {found:?}");
    assert!(found[0].contains("Ghost"), "{}", found[0]);
}

/// A variant may be posed by an entry from a *different* module — the `Anchor` case, which is why C3
/// is library-wide (see its doc).
#[test]
fn a_variant_may_be_posed_by_an_entry_from_another_module() {
    let lib = lib_of(&[], &["Center"], &[]);
    let cat = Catalogue {
        entries: vec![entry_of("material/modal.rs", "Modal", &["Center"], false)],
        ..Default::default()
    };
    assert!(
        c3_every_variant_is_posed(&lib, &cat).is_empty(),
        "`Anchor`'s variants live in a module whose every component is exempted; a module-scoped rule \
         would be unsatisfiable there"
    );
}

#[test]
fn an_animation_with_no_motion_entry_fails_and_names_it() {
    let lib = lib_of(&[], &[], &["fade", "slide"]);
    let cat = Catalogue {
        motion: vec!["fade".to_string()],
        ..Default::default()
    };
    let found = c5_every_animation_is_shown(&lib, &cat);
    assert_eq!(found.len(), 1, "got {found:?}");
    assert!(found[0].contains("slide"), "{}", found[0]);
}

#[test]
fn two_entries_for_one_animation_fail() {
    let lib = lib_of(&[], &[], &["fade"]);
    let cat = Catalogue {
        motion: vec!["fade".to_string(), "fade".to_string()],
        ..Default::default()
    };
    assert_eq!(c5_every_animation_is_shown(&lib, &cat).len(), 1);
}

#[test]
fn a_motion_entry_for_a_removed_animation_fails_and_names_it() {
    let lib = lib_of(&[], &[], &["fade"]);
    let cat = Catalogue {
        motion: vec!["fade".to_string(), "dissolve".to_string()],
        ..Default::default()
    };
    let found = c6_no_motion_entry_outlives_itself(&lib, &cat);
    assert_eq!(found.len(), 1, "got {found:?}");
    assert!(found[0].contains("dissolve"), "{}", found[0]);
}

#[test]
fn a_stale_exemption_fails_and_names_it() {
    let lib = lib_of(&[], &[], &[]);
    let cat = Catalogue {
        exemptions: vec![(
            "cdk/gone.rs".to_string(),
            "Vanished".to_string(),
            "a reason".to_string(),
        )],
        ..Default::default()
    };
    let found = c7_exemptions_are_live_and_reasoned(&lib, &cat);
    assert_eq!(found.len(), 1, "got {found:?}");
    assert!(found[0].contains("cdk/gone.rs::Vanished"), "{}", found[0]);
}

#[test]
fn an_exemption_without_a_reason_fails() {
    let lib = lib_of(&[("cdk/overlay.rs", "Overlay")], &[], &[]);
    let cat = Catalogue {
        exemptions: vec![(
            "cdk/overlay.rs".to_string(),
            "Overlay".to_string(),
            "   ".to_string(),
        )],
        ..Default::default()
    };
    let found = c7_exemptions_are_live_and_reasoned(&lib, &cat);
    assert_eq!(found.len(), 1, "got {found:?}");
    assert!(found[0].contains("no reason"), "{}", found[0]);
}

#[test]
fn a_component_both_posed_and_exempted_fails() {
    let cat = Catalogue {
        entries: vec![entry_of("material/tag.rs", "Tag", &[], false)],
        exemptions: vec![(
            "material/tag.rs".to_string(),
            "Tag".to_string(),
            "a reason".to_string(),
        )],
        ..Default::default()
    };
    let found = c8_the_partition_is_clean(&cat);
    assert!(
        found.iter().any(|f| f.contains("both posed and exempted")),
        "got {found:?}"
    );
}

#[test]
fn two_entries_for_one_component_fail() {
    let cat = Catalogue {
        entries: vec![
            entry_of("material/tag.rs", "Tag", &[], false),
            entry_of("material/tag.rs", "Tag", &[], false),
        ],
        ..Default::default()
    };
    assert_eq!(c8_the_partition_is_clean(&cat).len(), 1);
}

#[test]
fn an_animation_posed_as_a_still_fails() {
    let cat = Catalogue {
        entries: vec![entry_of(ANIMATION_MODULE, "Fade", &[], false)],
        ..Default::default()
    };
    let found = c9_the_motion_section_holds_the_animations(&cat);
    assert_eq!(found.len(), 1, "got {found:?}");
    assert!(found[0].contains("picture of it"), "{}", found[0]);
}

#[test]
fn a_static_component_in_the_motion_section_fails() {
    let cat = Catalogue {
        entries: vec![entry_of("material/tag.rs", "Tag", &[], true)],
        ..Default::default()
    };
    assert_eq!(c9_the_motion_section_holds_the_animations(&cat).len(), 1);
}

/// A healthy pair produces nothing, so the demonstrations above are showing a real difference rather
/// than a rule that fails on everything.
#[test]
fn a_healthy_pair_produces_no_findings() {
    let lib = lib_of(
        &[("material/button.rs", "Button"), (ANIMATION_MODULE, "Fade")],
        &["Filled"],
        &["fade"],
    );
    let cat = Catalogue {
        entries: vec![
            entry_of("material/button.rs", "Button", &["Filled"], false),
            entry_of(ANIMATION_MODULE, "Fade", &[], true),
        ],
        motion: vec!["fade".to_string()],
        exemptions: vec![],
    };
    assert_eq!(all_rules(&lib, &cat), Vec::<String>::new());
}
