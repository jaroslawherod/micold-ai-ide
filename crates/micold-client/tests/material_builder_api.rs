//! Every shared component has the same shape (feature 017, T017 — Principle VIII, contract §4).
//!
//! Constitution Principle VIII requires a chainable builder terminating in `.into()` — construct with
//! the *required* inputs, configure with `self`-consuming methods, convert to an element. Not free
//! functions with a long tail of positional parameters.
//!
//! The reason is legibility at the call site: `Button::filled(label, msg, roles).disabled(true)` says
//! what it is, and adding an option to a component never breaks a caller that did not want it. A
//! seven-argument function reaches the same place and reads as nothing at all.
//!
//! A rule that only lives in a review checklist decays between reviews, so this reads the library's own
//! source and holds it to the rule.
//!
//! **The scanner moved** (feature 020, T038). It lives in `tests/inventory/mod.rs` now, because the
//! component showcase's completeness check has to hold the gallery against the *same* definition of "a
//! component" this file holds the library to — FR-014, which asks that a change to that definition take
//! effect in both at once. Two scanners that happen to agree today is the arrangement that requirement
//! exists to prevent. The definition itself is unchanged; only its address is.

mod inventory;

use inventory::{declarations, Declared};

/// A component — something that becomes an element — must be built and terminated the same way every
/// other one is.
#[test]
fn every_component_is_constructed_by_a_constructor_and_terminates_in_into() {
    let mut violations = Vec::new();
    for d in declarations() {
        if !d.is_component() {
            continue; // a record, checked separately below
        }
        if !d.has_constructor {
            violations.push(format!(
                "{} `{}` converts into an element but has no constructor — a component is built from \
                 its required inputs, not assembled field by field",
                d.module, d.name
            ));
        }
        if d.public_fields {
            violations.push(format!(
                "{} `{}` converts into an element but exposes public fields — configuration goes \
                 through chainable methods, so adding one cannot break a caller",
                d.module, d.name
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "Principle VIII (contract §4):\n{}",
        violations.join("\n")
    );
}

/// Optional configuration is a `self`-consuming method returning `Self`. Anything else is not
/// chainable, which is the whole point.
#[test]
fn every_optional_input_is_a_chainable_builder_step() {
    let mut violations = Vec::new();
    for d in declarations() {
        if !d.is_component() {
            continue;
        }
        for method in &d.non_builder_methods {
            violations.push(format!(
                "{} `{}::{}` is public but neither construction nor a chainable step — it must take \
                 `self` and return `Self`, or be justified in SANCTIONED_NON_BUILDER_METHODS",
                d.module, d.name, method
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "Principle VIII (contract §4):\n{}",
        violations.join("\n")
    );
}

/// The partition has to be clean: a struct is either a component (built, chained, converted) or a
/// record the caller fills in (`MenuItem`, `TreeItem`). A struct that is both would give
/// call sites two ways to configure the same thing, which is how variants drift apart.
#[test]
fn nothing_is_both_a_component_and_a_record() {
    let both: Vec<String> = declarations()
        .into_iter()
        .filter(|d: &Declared| d.is_component() && d.public_fields)
        .map(|d| format!("{} `{}`", d.module, d.name))
        .collect();
    assert!(
        both.is_empty(),
        "these are both a component and a record:\n{}",
        both.join("\n")
    );
}

/// A scan that finds nothing would pass all three assertions above.
#[test]
fn the_scan_actually_finds_the_library_components() {
    let declared = declarations();
    let components: Vec<&str> = declared
        .iter()
        .filter(|d| d.is_component())
        .map(|d| d.name.as_str())
        .collect();
    assert!(
        components.len() >= 10,
        "expected the component library, found {components:?}"
    );
    for expected in ["Modal", "Tooltip", "IconButton", "Surface", "Overlay"] {
        assert!(
            components.contains(&expected),
            "expected `{expected}` among the components, found {components:?}"
        );
    }
}

/// The tooltip's rest mode is configured like everything else: three chainable steps (feature 038,
/// contracts/rest-tooltip.md §6).
#[test]
fn the_tooltips_rest_mode_is_three_chainable_steps() {
    let sources = inventory::library_sources();
    let (_, src) = sources
        .iter()
        .find(|(module, _)| module.ends_with("material/mod.rs"))
        .expect("the material module");
    let code = inventory::code_only(src);
    let start = code
        .find("impl<'a, M: 'a> Tooltip<'a, M>")
        .expect("`material::Tooltip`'s builder impl");
    let end = start
        + code[start..]
            .find("impl<'a, M: 'a> From<Tooltip<'a, M>>")
            .expect("the conversion that ends the builder");
    let builder = &code[start..end];

    for step in ["after_rest", "show_delay", "max_lines", "subject"] {
        let Some(at) = builder.find(&format!("pub fn {step}(")) else {
            panic!("`material::Tooltip::{step}` is missing");
        };
        let signature = &builder[at..at + builder[at..].find('{').expect("a body")];
        let params = &signature[signature.find('(').expect("parameters") + 1..];
        assert!(
            params.trim_start().starts_with("mut self") || params.trim_start().starts_with("self"),
            "`Tooltip::{step}` must consume `self`: {signature}",
        );
        assert!(
            signature.trim_end().ends_with("-> Self"),
            "`Tooltip::{step}` must return `Self` to stay chainable: {signature}",
        );
    }
}
