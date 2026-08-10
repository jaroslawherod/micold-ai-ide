//! What the component library contains — **one** definition, shared by every gate that needs it
//! (feature 020, T038/T039 — FR-014).
//!
//! This code was `tests/material_builder_api.rs`'s. It moved here so the completeness check
//! (`tests/showcase_completeness.rs`) can hold the gallery against the *same* idea of "a component"
//! that the builder-API gate holds the library to. FR-014 asks for exactly that, and for a reason:
//! two scanners that happen to agree today is the arrangement the requirement exists to prevent,
//! because the drift is silent — the completeness check keeps passing while its notion of the library
//! quietly diverges. Sharing the code makes agreement structural rather than coincidental.
//!
//! A directory under `tests/` is not compiled as its own test binary, so this is included with
//! `mod inventory;` by the files that need it — the same arrangement `tests/support/mod.rs` uses.
//!
//! # The definition
//!
//! A **component** is a `pub struct` declared under `src/ui/material/` or `src/ui/cdk/` that either
//! converts into something (`From<Self> for …`) or is one of the documented [`TERMINAL_TYPES`]. A
//! struct with neither is a plain record the caller fills in (`MenuItem`, `TreeItem`).
//! Nothing about that changed in the move.
//!
//! # Two things the move made load-bearing
//!
//! Both were already true; neither mattered until something keyed off the result.
//!
//! 1. **Names are not unique.** `material/animation.rs`'s private `mod tags` declares
//!    `pub struct Fade; Expand; Scale; Scrim;` as widget-tree tags, so a name-keyed scan yields two
//!    `Fade`s — the wrapper and its tag. Separately, `material/surface.rs` and `cdk/overlay.rs` each
//!    declare a `Surface`, and they are **different components**.
//! 2. So [`components`] keys by **(module, name)** and collapses duplicates within one module. Keying
//!    by name alone would let the two `Surface`s satisfy each other's requirement, which is precisely
//!    the "passes while wrong" shape FR-016 guards against.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// The two library layers.
pub fn library_dirs() -> Vec<PathBuf> {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    vec![ui.join("material"), ui.join("cdk")]
}

/// A directory under `src/`, for a caller holding a layer other than the library to the same rule.
///
/// Added by feature 021 T030: `src/overlay/` is not part of the component library and never becomes
/// an element, but it is built the same way — required inputs to a constructor, options through
/// chainable steps — and the argument for one definition of that shape is the same argument that
/// moved this scanner here in the first place.
pub fn src_dir(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(relative)
}

/// Every library source, keyed by a display path (`material/button.rs`).
pub fn library_sources() -> BTreeMap<String, String> {
    sources_in(&library_dirs())
}

/// Every `.rs` file directly under each of `dirs`, keyed by a display path (`overlay/registry.rs`).
pub fn sources_in(dirs: &[PathBuf]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for dir in dirs {
        for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
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

/// Every `.rs` file at or under `root`, recursively, keyed by a path relative to `src/`
/// (`features/help.rs`, `ui/material/button.rs`).
///
/// [`sources_in`] deliberately does not recurse — it answers "the files of this one layer", which
/// is what the library gates ask. This answers "everywhere in the client", which is what a
/// reachability guard has to ask, since a surface named from `ui/material/` would be just as much
/// a violation as one named from `ui/`. Added at feature 021 T039 rather than written a second
/// time in the guard that needed it (FR-014).
pub fn sources_under(root: &Path) -> BTreeMap<String, String> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let key = path
                    .strip_prefix(&src)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(key, fs::read_to_string(&path).expect("read source"));
            }
        }
    }
    out
}

/// The seven service ports (spec §"What already exists"): the I/O needs the render-free core
/// declares and the shell supplies.
///
/// Here rather than in the guard that first needed it, because two guards now do — T041's
/// reachability check and T042's fake-coverage check — and two copies of "what a capability is" is
/// exactly the drift FR-014 objects to.
pub const PORTS: &[&str] = &[
    "Git",
    "ProjectStore",
    "SettingsStore",
    "FolderScanner",
    "TerminalBackend",
    "TerminalHandle",
    "AiCliProvider",
    // Declared at T046, alongside T043's behaviour test. Listed the moment it existed: a
    // capability the guards do not know about is one FR-017 and FR-019 are not holding.
    "EnvIncludeResolver",
    // T047's. Its real implementation lives in the *client*, not the core — `dark-light` is a
    // client dependency and the core deliberately has none on it — so the coverage guard finds
    // its fake here while `no_concrete_implementations` never sees the real one. Recorded at
    // both sites rather than left as a surprise.
    "OsThemeProbe",
];

/// One `impl <Port> for <Type>`, and where it was found.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortImpl {
    /// The capability being implemented, e.g. `Git`.
    pub port: String,
    /// The implementing type, e.g. `GitCli`.
    pub ty: String,
    /// Display path of the file it was found in.
    pub file: String,
}

impl PortImpl {
    /// Whether this is a test double, by the codebase's `Fake` naming convention.
    pub fn is_fake(&self) -> bool {
        self.ty.starts_with("Fake")
    }
}

/// Every implementation of a service port under `root`, from the source text.
///
/// Matches `impl <Port> for <Type>`, tolerating a fully-qualified port path
/// (`impl micold_core::git::Git for X`) so a scan of test files finds what they write.
pub fn port_impls_under(root: &Path) -> Vec<PortImpl> {
    let mut found = Vec::new();
    for (file, text) in sources_under(root) {
        for line in code_only(&text).lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("impl ") else {
                continue;
            };
            let Some((port, ty)) = rest.split_once(" for ") else {
                continue;
            };
            let port = port.rsplit("::").next().unwrap_or(port).trim();
            if !PORTS.contains(&port) {
                continue;
            }
            let ty = ty
                .trim_end_matches('{')
                .trim()
                .split('<')
                .next()
                .unwrap_or(ty)
                .trim();
            found.push(PortImpl {
                port: port.to_string(),
                ty: ty.to_string(),
                file: file.clone(),
            });
        }
    }
    found.sort();
    found.dedup();
    found
}

/// The render-free core's `src` directory.
pub fn core_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .join("micold-core/src")
}

/// Strips comments so prose is never mistaken for a declaration.
pub fn code_only(src: &str) -> String {
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

/// Components that terminate somewhere other than `.into()` on an element, with where.
///
/// `Surface` is the one: it is what the material overlays convert *into*, and it is consumed by
/// `Overlay::push`. Same shape — required inputs, chainable options, one termination — with the
/// element conversion one level further out. Listing it keeps the exception visible rather than
/// letting the rule quietly not apply to it.
pub const TERMINAL_TYPES: &[&str] = &["Surface"];

/// Methods a component may expose that are neither construction nor a builder step.
///
/// `dismisses_on` reads a rule the caller cannot otherwise reach: an Escape press arrives through the
/// keyboard subscription, outside the widget tree entirely. `stacking` and `push`/`push_maybe` belong
/// to the overlay *host*, which collects surfaces rather than being one.
pub const SANCTIONED_NON_BUILDER_METHODS: &[&str] =
    &["dismisses_on", "stacking", "push", "push_maybe"];

/// A public struct declared in the library, with the source it came from.
#[derive(Debug)]
pub struct Declared {
    pub name: String,
    pub module: String,
    /// Has a `From<Self> for …` conversion — i.e. it terminates in `.into()`.
    pub convertible: bool,
    /// Exposes at least one public field — i.e. it is a data record the caller fills in.
    pub public_fields: bool,
    /// Has at least one constructor — a public method taking no `self`, or a public free function in
    /// the same module that returns it.
    pub has_constructor: bool,
    /// Public methods that are neither `new` nor a `self`-consuming builder step.
    pub non_builder_methods: Vec<String>,
}

impl Declared {
    /// A component is anything the library asks a caller to *build*: it converts into an element, or
    /// it is one of the documented terminal types. A struct with neither is a plain record.
    pub fn is_component(&self) -> bool {
        self.convertible || TERMINAL_TYPES.contains(&self.name.as_str())
    }
}

pub fn declarations() -> Vec<Declared> {
    declarations_in(&library_dirs())
}

/// Every public struct declared directly under each of `dirs`, with the same reading of
/// "constructor", "builder step" and "public field" the library gate uses.
pub fn declarations_in(dirs: &[PathBuf]) -> Vec<Declared> {
    let mut out = Vec::new();
    for (module, src) in sources_in(dirs) {
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

            let convertible =
                code.contains(&format!("From<{name}<")) || code.contains(&format!("From<{name}>"));
            let body = struct_body(&code, &name);
            let public_fields = body.lines().any(|l| l.trim_start().starts_with("pub "));
            let (associated, non_builder_methods) = inherent_methods(&code, &name);

            out.push(Declared {
                name: name.clone(),
                module: module.clone(),
                convertible,
                public_fields,
                has_constructor: associated || has_free_constructor(&code, &name),
                non_builder_methods,
            });
        }
    }
    out
}

/// Every component in the library, keyed by `(module, name)` and deduplicated within a module.
///
/// The key is a pair because names collide — see this module's own docs. Deduplication is what folds
/// `animation.rs`'s widget-tree tag into the wrapper it tags; they are the same name in the same
/// module and neither the builder gate nor the gallery has any use for the distinction.
pub fn components() -> BTreeSet<(String, String)> {
    declarations()
        .into_iter()
        .filter(|d| d.is_component())
        .map(|d| (d.module, d.name))
        .collect()
}

/// One `pub enum` in the library and the variant **names** it declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDecl {
    pub module: String,
    pub name: String,
    pub variants: Vec<String>,
}

/// Every `pub enum` in the library, with its variant names.
///
/// Names only: `Kind::Notification(NoticeLevel)` yields `Notification`, and
/// `Anchor::TopEnd { top, end }` yields `TopEnd`. A variant's payload is not part of its identity —
/// posing one representative payload is what "this variant has an instance" means for a variant that
/// carries one.
pub fn enums() -> Vec<EnumDecl> {
    let mut out = Vec::new();
    for (module, src) in library_sources() {
        let code = code_only(&src);
        let mut search = 0usize;
        while let Some(found) = code[search..].find("pub enum ") {
            let at = search + found + "pub enum ".len();
            let name: String = code[at..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let Some(open_rel) = code[at..].find('{') else {
                break;
            };
            let open = at + open_rel;
            let mut depth = 0usize;
            let mut end = open;
            for (i, c) in code[open..].char_indices() {
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
            if !name.is_empty() {
                out.push(EnumDecl {
                    module: module.clone(),
                    name,
                    variants: variant_names(&code[open + 1..end]),
                });
            }
            search = end.max(at);
        }
    }
    out
}

/// The variant names declared at the top level of an enum body.
///
/// Line-based with a nesting counter: a struct-like variant's fields sit at depth 1 and are skipped,
/// and a variant name is an identifier starting with an uppercase letter — which is what distinguishes
/// `Notification(NoticeLevel),` from `reason: String,`.
fn variant_names(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if depth == 0 && trimmed.starts_with(|c: char| c.is_ascii_uppercase()) {
            let name: String = trimmed
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                out.push(name);
            }
        }
        depth += line.matches(['{', '(', '[']).count() as i32;
        depth -= line.matches(['}', ')', ']']).count() as i32;
        depth = depth.max(0);
    }
    out
}

/// The library's animation helpers: the **free** `pub fn`s in `material/animation.rs`.
///
/// FR-013a's second category, enumerated deliberately rather than inherited. The component definition
/// above recognises things that convert into an element, so it does not see a helper offered as a plain
/// function — which would leave motion outside the check by accident, the silent omission FR-012
/// exists to prevent.
///
/// Column 0 is what selects them: a builder step lives inside an `impl` block or a macro body and is
/// indented, so `fade` is found and `animate_in` is not.
pub fn animation_helpers() -> BTreeSet<String> {
    let src = library_sources()
        .get("material/animation.rs")
        .cloned()
        .unwrap_or_default();
    code_only(&src)
        .lines()
        .filter_map(|line| line.strip_prefix("pub fn "))
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

/// A struct's field list: between the braces of a braced struct, or the parentheses of a tuple one.
///
/// The three forms are distinguished by whichever of `{`, `(` and `;` closes the header first. Only
/// the braced form existed in the library, and taking the next `{` unconditionally was fine for it —
/// but a *tuple* struct's next `{` is its `impl` block, so the "fields" came back as a list of
/// methods and every one of them read as `pub`. Feature 021 T030 pointed this scanner at
/// `src/overlay/`, whose `SurfaceId(&'static str)` is exactly that shape, and it reported a public
/// field the type does not have.
fn struct_body(code: &str, name: &str) -> String {
    let Some(start) = code.find(&format!("pub struct {name}")) else {
        return String::new();
    };
    let after = &code[start..];
    let (open, close) = match after.find(['{', '(', ';']) {
        Some(at) if after.as_bytes()[at] == b'{' => (at, ('{', '}')),
        Some(at) if after.as_bytes()[at] == b'(' => (at, ('(', ')')),
        // A unit struct — `pub struct SidebarFilterPanel;` — has no fields to report.
        _ => return String::new(),
    };
    let mut depth = 0usize;
    let mut end = open;
    for (i, c) in after[open..].char_indices() {
        if c == close.0 {
            depth += 1;
        } else if c == close.1 {
            depth -= 1;
            if depth == 0 {
                end = open + i;
                break;
            }
        }
    }
    // Past the opening delimiter, so a tuple struct's `(pub String)` reads as a public field the
    // way a braced struct's `pub name: String` line does.
    after[open + 1..end].to_string()
}

/// Whether a public free function in the same module returns `name`, i.e. constructs it.
///
/// The animation wrappers are built this way — `fade(content, shown, over, backdrop)` rather than
/// `Fade::new(...)` — matching how the rendering stack names its own widget constructors. What
/// Principle VIII rules out is a component assembled field by field, not a lowercase name.
fn has_free_constructor(code: &str, name: &str) -> bool {
    code.match_indices("pub fn ").any(|(at, _)| {
        let end = code[at..].find('{').map(|o| at + o).unwrap_or(code.len());
        let Some((_, returns)) = code[at..end].split_once("->") else {
            return false;
        };
        let returns = returns.trim_start();
        // Guard the boundary so `Scale` is not credited to a `Scaled` constructor.
        returns.starts_with(name)
            && !returns[name.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
    })
}

/// Scans every `impl … Name…` block for public methods, returning `(has_constructor, offenders)`.
///
/// Every public method has to be one of two things: a **constructor** (takes no `self`) or a **builder
/// step** (takes `self`, returns `Self`). Anything else is a third way to configure a component, and
/// three ways is how variants drift.
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

        let targets_type =
            header.contains(&format!(" {name}<")) || header.ends_with(&format!(" {name} "));
        if !targets_type || header.contains(" for ") {
            continue;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    /// The keying property this module's move made load-bearing: two different `Surface`s.
    ///
    /// Keyed by name alone they would collapse into one, and each would satisfy the other's
    /// requirement — a gallery could pose the material one, omit the cdk one, and pass.
    #[test]
    fn the_two_surfaces_are_different_components() {
        let found = components();
        assert!(
            found.contains(&("material/surface.rs".to_string(), "Surface".to_string())),
            "material/surface.rs::Surface missing from {found:?}"
        );
        assert!(
            found.contains(&("cdk/overlay.rs".to_string(), "Surface".to_string())),
            "cdk/overlay.rs::Surface missing — keyed by name alone, it would have been the other one"
        );
    }

    /// The other half: a duplicate name *within* one module collapses.
    ///
    /// `animation.rs` declares `Fade` twice — the wrapper and the private widget-tree tag that guards
    /// its state. The gallery has one entry for it, and should not be asked for two.
    #[test]
    fn a_widget_tree_tag_does_not_become_a_second_component() {
        let fades = declarations()
            .into_iter()
            .filter(|d| d.name == "Fade" && d.module == "material/animation.rs")
            .count();
        assert!(
            fades >= 2,
            "expected the wrapper and its tag to both be declared; found {fades}. If `mod tags` was \
             removed, this test has served its purpose and can go with it."
        );
        let keyed = components()
            .into_iter()
            .filter(|(m, n)| m == "material/animation.rs" && n == "Fade")
            .count();
        assert_eq!(keyed, 1, "the tag and the wrapper must collapse to one key");
    }

    /// The three struct forms, since only the braced one existed when this parser was written.
    ///
    /// A tuple struct's next `{` is its `impl` block; before feature 021 T030 that block came back
    /// as the field list and every `pub fn` in it read as a public field.
    #[test]
    fn a_tuple_structs_methods_are_not_its_fields() {
        let code = "pub struct Id(&'static str);\n\nimpl Id {\n    pub fn new() -> Self {}\n}\n";
        assert_eq!(struct_body(code, "Id"), "&'static str");

        let public = "pub struct Pair(pub String);\n";
        assert!(struct_body(public, "Pair").trim_start().starts_with("pub "));

        assert_eq!(struct_body("pub struct Marker;\n", "Marker"), "");
    }

    /// Variant names, payloads stripped.
    #[test]
    fn variant_payloads_are_not_part_of_a_variants_name() {
        let kinds = enums()
            .into_iter()
            .find(|e| e.name == "Kind" && e.module == "material/surface.rs")
            .expect("material/surface.rs declares `Kind`");
        assert!(kinds.variants.contains(&"Notification".to_string()));
        assert!(kinds.variants.contains(&"Chip".to_string()));
        assert!(
            !kinds.variants.iter().any(|v| v.contains('(')),
            "a payload leaked into a variant name: {:?}",
            kinds.variants
        );
    }

    /// A struct-like variant's fields are not variants.
    #[test]
    fn a_struct_like_variants_fields_are_not_variants() {
        let anchor = enums()
            .into_iter()
            .find(|e| e.name == "Anchor")
            .expect("cdk/overlay.rs declares `Anchor`");
        assert_eq!(
            anchor.variants,
            vec!["Point", "TopEnd", "Center", "BottomCenter"],
            "the scanner is meant to read a struct-like variant's *name* and skip its fields — \
             `BottomCenter {{ bottom }}` was added for the snackbar, and if its field leaked in as \
             a variant this list would say so"
        );
    }

    /// Only the free helpers, not the builder steps they share.
    #[test]
    fn the_animation_helpers_are_the_free_functions() {
        let helpers = animation_helpers();
        for expected in ["fade", "expand", "scale", "scrim"] {
            assert!(
                helpers.contains(expected),
                "expected `{expected}` among the animation helpers, found {helpers:?}"
            );
        }
        for builder_step in [
            "animate_in",
            "restart_on",
            "exiting_over",
            "on_hidden",
            "new",
        ] {
            assert!(
                !helpers.contains(builder_step),
                "`{builder_step}` is a builder step, not an animation — it would demand a motion \
                 entry of its own"
            );
        }
    }
}
