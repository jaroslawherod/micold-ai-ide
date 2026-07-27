//! Every shared component has the same shape (feature 017, T017 — Principle VIII, contract §4).
//!
//! Constitution Principle VIII requires a chainable builder terminating in `.into()` — construct
//! with the *required* inputs, configure with `self`-consuming methods, convert to an element. Not
//! free functions with a long tail of positional parameters.
//!
//! The reason is legibility at the call site: `Button::filled(label, msg, roles).disabled(true)`
//! says what it is, and adding an option to a component never breaks a caller that did not want it.
//! A seven-argument function reaches the same place and reads as nothing at all.
//!
//! A rule that only lives in a review checklist decays between reviews, so this reads the library's
//! own source and holds it to the rule.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn library_dirs() -> Vec<PathBuf> {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    vec![ui.join("material"), ui.join("cdk")]
}

/// Every library source, keyed by a display path.
fn library_sources() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for dir in library_dirs() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|e| e == "rs") {
                let key = format!(
                    "{}/{}",
                    dir.file_name().unwrap().to_string_lossy(),
                    path.file_name().unwrap().to_string_lossy()
                );
                out.insert(key, fs::read_to_string(&path).expect("read source"));
            }
        }
    }
    out
}

/// A public struct declared in the library, with the source it came from.
#[derive(Debug)]
struct Declared {
    name: String,
    module: String,
    /// Has a `From<Self> for …` conversion — i.e. it terminates in `.into()`.
    convertible: bool,
    /// Exposes at least one public field — i.e. it is a data record the caller fills in.
    public_fields: bool,
    /// Has at least one constructor — a public method taking no `self`.
    has_constructor: bool,
    /// Public methods that are neither `new` nor a `self`-consuming builder step.
    non_builder_methods: Vec<String>,
}

impl Declared {
    /// A component is anything the library asks a caller to *build*: it converts into an element,
    /// or it is one of the documented terminal types. A struct with neither is a plain record.
    fn is_component(&self) -> bool {
        self.convertible || TERMINAL_TYPES.contains(&self.name.as_str())
    }
}

/// Strips comments so prose is never mistaken for a declaration.
fn code_only(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_block = false;
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                in_block = true;
            }
            _ => out.push(c),
        }
    }
    out
}

/// Methods a component may expose that are neither construction nor a builder step, with the
/// reason each is legitimate. Anything not listed has to be one or the other.
///
/// `dismisses_on` reads a rule the caller cannot otherwise reach: an Escape press arrives through
/// the keyboard subscription, outside the widget tree entirely, so the owner of that trigger has to
/// be able to ask. `stacking` and `push`/`push_maybe` belong to the overlay *host*, which collects
/// surfaces rather than being one.
const SANCTIONED_NON_BUILDER_METHODS: &[&str] = &["dismisses_on", "stacking", "push", "push_maybe"];

/// Components that terminate somewhere other than `.into()` on an element, with where.
///
/// `Surface` is the one: it is what the material overlays convert *into*, and it is consumed by
/// `Overlay::push`. Same shape — required inputs, chainable options, one termination — with the
/// element conversion one level further out. Listing it keeps the exception visible rather than
/// letting the rule quietly not apply to it.
const TERMINAL_TYPES: &[&str] = &["Surface"];

fn declarations() -> Vec<Declared> {
    let mut out = Vec::new();
    for (module, src) in library_sources() {
        let code = code_only(&src);
        for line in code.lines() {
            let trimmed = line.trim_start();
            let Some(rest) = trimmed.strip_prefix("pub struct ") else {
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name.is_empty() {
                continue;
            }

            // Anything mentioning the type in a `From<...>` position terminates in `.into()`.
            let convertible =
                code.contains(&format!("From<{name}<")) || code.contains(&format!("From<{name}>"));

            // Public fields make it a record the caller populates, not a builder.
            let body = struct_body(&code, &name);
            let public_fields = body.lines().any(|l| l.trim_start().starts_with("pub "));

            let (has_constructor, non_builder_methods) = inherent_methods(&code, &name);

            out.push(Declared {
                name,
                module: module.clone(),
                convertible,
                public_fields,
                has_constructor,
                non_builder_methods,
            });
        }
    }
    out
}

/// The text between `pub struct Name … {` and its closing brace, at brace depth.
fn struct_body(code: &str, name: &str) -> String {
    let Some(start) = code.find(&format!("pub struct {name}")) else {
        return String::new();
    };
    let after = &code[start..];
    let Some(open) = after.find('{') else {
        return String::new();
    };
    let mut depth = 0usize;
    let mut end = open;
    for (i, c) in after[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = open + i;
                    break;
                }
            }
            _ => {}
        }
    }
    after[open..end].to_string()
}

/// Scans every `impl … Name…` block for public methods, returning `(has_constructor, offenders)`.
///
/// Every public method has to be one of two things:
///
/// - a **constructor** — takes no `self`, so it is how the component comes into existence. Named
///   rather than always `new`, because `Button::filled(...)` and `Button::outlined(...)` say what
///   they build; forcing them through `Button::new(label, Variant::Filled, roles)` would move the
///   meaning into an argument and read worse at every call site.
/// - a **builder step** — takes `self` and returns `Self`, which is what makes it chainable and
///   what stops a half-configured component being reused by accident.
///
/// Anything else is a third way to configure a component, and three ways is how variants drift.
fn inherent_methods(code: &str, name: &str) -> (bool, Vec<String>) {
    let mut has_constructor = false;
    let mut offenders = Vec::new();

    let mut search = 0usize;
    while let Some(found) = code[search..].find("impl") {
        let at = search + found;
        let Some(line_end) = code[at..].find('{') else {
            break;
        };
        let header = &code[at..at + line_end];
        search = at + line_end + 1;

        // Only inherent impls of this type; trait impls (`impl From<X> for Y`) are not the
        // builder surface.
        let targets_type =
            header.contains(&format!(" {name}<")) || header.ends_with(&format!(" {name} "));
        if !targets_type || header.contains(" for ") {
            continue;
        }

        // The impl body, at brace depth.
        let body_start = at + line_end;
        let mut depth = 0usize;
        let mut body_end = body_start;
        for (i, c) in code[body_start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        body_end = body_start + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        let body = &code[body_start..body_end];

        for line in body.lines() {
            let trimmed = line.trim_start();
            let Some(sig) = trimmed.strip_prefix("pub fn ") else {
                continue;
            };
            let method: String = sig
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if SANCTIONED_NON_BUILDER_METHODS.contains(&method.as_str()) {
                continue;
            }
            // The parameter list, up to the closing paren of the signature.
            let params = sig.split_once('(').map(|(_, rest)| rest).unwrap_or("");
            let takes_self = params.trim_start().starts_with("self")
                || params.trim_start().starts_with("mut self")
                || params.trim_start().starts_with("&self");

            if !takes_self {
                has_constructor = true;
                continue;
            }
            if !sig.contains("-> Self") {
                offenders.push(method);
            }
        }
    }
    (has_constructor, offenders)
}

/// A component — something that becomes an element — must be built and terminated the same way
/// every other one is.
#[test]
fn every_component_is_constructed_by_a_constructor_and_terminates_in_into() {
    let mut violations = Vec::new();
    for d in declarations() {
        if !d.is_component() {
            continue; // a record, checked separately below
        }
        if !d.has_constructor {
            violations.push(format!(
                "{} `{}` converts into an element but has no constructor — a component is built \
                 from its required inputs, not assembled field by field",
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
                "{} `{}::{}` is public but neither construction nor a chainable step — it must \
                 take `self` and return `Self`, or be justified in SANCTIONED_NON_BUILDER_METHODS",
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
/// record the caller fills in (`MenuItem`, `ProjectRow`, `TreeItem`). A struct that is both would
/// give call sites two ways to configure the same thing, which is how variants drift apart.
#[test]
fn nothing_is_both_a_component_and_a_record() {
    let both: Vec<String> = declarations()
        .into_iter()
        .filter(|d| d.is_component() && d.public_fields)
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
