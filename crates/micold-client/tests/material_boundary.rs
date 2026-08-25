//! The library owns the rendering stack (feature 017, T016 — FR-001–FR-004, SC-001).
//!
//! Thirteen feature modules build rendering widgets and style them by hand. Each one is free to
//! render a slightly different button, and several do. The fix is structural rather than editorial:
//! the component library wraps the rendering stack, and a feature module composes components.
//!
//! **All three counts are now zero.** This is the blocking gate for SC-001: a feature module that
//! constructs a styled widget, reaches the styling layer, or picks a raw text size fails here.
//!
//! It ratcheted down rather than starting at zero — flipping the budget on the first migrated
//! module would have made every intermediate state unbuildable — but the ratchet is finished and
//! the numbers below must not move again.
//!
//! The count is now belt to the structure's braces: `material::style` is `pub(crate)`, so a call
//! site *cannot* reach it. What this still catches is the thing visibility cannot — a feature
//! module building a raw widget, or naming a size instead of a type role.
//!
//! Text scanning, not type inspection: the property is about what a *source file* is allowed to
//! name. A module that cannot name `text_input` cannot render an off-spec one.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn ui_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui")
}

/// The two library layers. Everything else under `ui/` is a feature module and is bound by the
/// rules below.
const LIBRARY: &[&str] = &["material", "cdk"];

/// Feature modules that outgrew a single file and became a directory (feature 027: Settings is a
/// sectioned view, one module per section). They are **not** a library layer — naming them here
/// keeps them bound by every rule below, where adding them to [`LIBRARY`] would exempt them, which
/// is exactly what the directory assertion's message warns against.
const FEATURE_DIRS: &[&str] = &["settings"];

/// The styling module itself, which *is* the layer the rules point at.
const STYLE_MODULE: &str = "style.rs";

/// The component showcase (feature 020), which composes the library exactly as a feature module does
/// and is bound by the same rules (FR-021). Keyed with a `showcase/` prefix so a failure says which
/// side of the crate it came from.
fn showcase_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/showcase")
}

/// Every feature module's source, keyed by file name.
///
/// Two roots: `src/ui/`'s own modules (everything that is not a library layer), and all of
/// `src/showcase/`, recursively. The showcase is not under `ui/` deliberately — putting it there would
/// have made it either a library layer, and so exempt from these rules, or a sibling of the layers the
/// directory assertion below polices. Scanning it from here binds it to the rules without pretending
/// it is part of the application's view.
fn feature_modules() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(ui_dir()).expect("read ui dir") {
        let path = entry.expect("dir entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if path.is_dir() {
            if FEATURE_DIRS.contains(&name.as_str()) {
                collect_tree(&path, &mut out);
                continue;
            }
            assert!(
                LIBRARY.contains(&name.as_str()),
                "unexpected directory `ui/{name}/` — is it a new library layer? Add it to LIBRARY, \
                 a feature module that grew into a directory? Add it to FEATURE_DIRS. Left out of \
                 both it is silently exempt from the boundary rules"
            );
            continue;
        }
        if name == STYLE_MODULE || !name.ends_with(".rs") {
            continue;
        }
        out.insert(name, fs::read_to_string(&path).expect("read source"));
    }
    collect_tree(&showcase_dir(), &mut out);
    out
}

/// Every `.rs` file under `dir`, recursively, keyed by its path relative to `src/` — so
/// `showcase/gallery.rs` and `ui/settings/daemon.rs`. The key says which side of the crate a
/// failure came from, which a bare file name would not.
fn collect_tree(dir: &Path, out: &mut BTreeMap<String, String>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_tree(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let key = path
                .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/");
            out.insert(key, fs::read_to_string(&path).expect("read source"));
        }
    }
}

/// Strips `//` line comments and `/* */` blocks, so prose naming a widget is not a violation.
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

/// Rendering widgets that carry an appearance, and so must come from the library (contract §1).
///
/// Layout primitives are deliberately absent: rows, columns, spacers, stacks and pointer-area
/// wrappers position other widgets and have nothing to style (FR-003). Wrapping them would add
/// indirection for no gain, so naming them is not a violation.
///
/// `container` is absent for a subtler reason: it is *both*. A container with a style is a
/// surface and belongs to the library; a container that only pads or aligns is layout and does
/// not. The two are indistinguishable by name, and what separates them — the `.style(...)` — is
/// what the styling-layer count already catches. Counting `container(` here would flag every
/// padding wrapper in the codebase and could never reach zero.
const WRAPPED_WIDGETS: &[&str] = &[
    "button",
    "text_input",
    // `pick_list` was here until feature 022. It leaves because nothing wraps it any more: the
    // select is the library's own control now, so naming the stack widget in a feature module is
    // no longer a boundary crossing — it is a reference to something this application does not use
    // at all (contract §5).
    "checkbox",
    "scrollable",
    "progress_bar",
    "tooltip",
];

/// How many times a feature module names a wrapped rendering widget as a constructor call.
///
/// Matched at a word boundary, so a *method* whose name ends in a widget's — `row_tooltip(`,
/// `menu_panel(` — is not mistaken for constructing one. Without that the count can never reach
/// zero, because the offending name belongs to a builder step the library itself provides.
///
/// A **declaration** is not a call either. `fn scrollable(...)` in an `impl Operation` is iced's
/// own callback name — the module is being *told* about a scrollable someone else built, which is
/// the opposite of building one. Nothing here can rename it.
fn widget_calls(code: &str) -> usize {
    fn declares(line: &str) -> bool {
        let line = line.trim_start();
        line.starts_with("fn ") || line.starts_with("pub fn ") || line.starts_with("pub(crate) fn ")
    }

    fn names_widget(line: &str, widget: &str) -> bool {
        let needle = format!("{widget}(");
        let mut from = 0;
        while let Some(at) = line[from..].find(&needle) {
            let start = from + at;
            let preceded_by_ident = start > 0
                && line[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_');
            if !preceded_by_ident {
                return true;
            }
            from = start + needle.len();
        }
        false
    }
    code.lines()
        .filter(|line| !declares(line) && WRAPPED_WIDGETS.iter().any(|w| names_widget(line, w)))
        .count()
}

/// How many times a feature module reaches into the styling layer.
fn style_references(code: &str) -> usize {
    code.lines().filter(|line| line.contains("style::")).count()
}

/// How many times a feature module selects a **raw** text size instead of naming a type role.
///
/// `.size(TypeRole::Label)` is the destination, not a violation — the whole point is that a call
/// site says what the text *is* and the role supplies the number. What counts is reaching for the
/// number itself: naming a scale constant, or passing anything to `.size(...)` that is not a role.
fn raw_size_references(code: &str) -> usize {
    code.lines()
        .filter(|line| {
            // A *named* scale constant is still reaching for the number. `type_scale` was
            // feature 003's and is gone; `typography` is the Material scale, resolved in exactly
            // one file that is not a feature module.
            if line.contains("type_scale::") || line.contains("typography::") {
                return true;
            }
            match line.split_once(".size(") {
                Some((_, arg)) => !arg.trim_start().starts_with("TypeRole::"),
                None => false,
            }
        })
        .count()
}

// ---------------------------------------------------------------------------
// The gate. All three reached zero at T036 (SC-001). Raising one is a
// regression, not a budget adjustment.
// ---------------------------------------------------------------------------

/// Feature-module lines constructing a wrapped rendering widget. Baseline was **86** across the 13
/// modules research R2 counted.
const WIDGET_BUDGET: usize = 0;

/// Feature-module lines referencing the styling layer. Baseline was **113**.
const STYLE_BUDGET: usize = 0;

/// Feature-module lines selecting a raw text size. Baseline was **114**.
const RAW_SIZE_BUDGET: usize = 0;

fn totals() -> (usize, usize, usize) {
    let mut widgets = 0;
    let mut styles = 0;
    let mut sizes = 0;
    for src in feature_modules().values() {
        let code = code_only(src);
        widgets += widget_calls(&code);
        styles += style_references(&code);
        sizes += raw_size_references(&code);
    }
    (widgets, styles, sizes)
}

/// Per-module breakdown, so a failure names where the work is rather than only how much is left.
fn breakdown() -> String {
    let mut rows: Vec<(usize, String)> = feature_modules()
        .iter()
        .map(|(name, src)| {
            let code = code_only(src);
            let (w, s, z) = (
                widget_calls(&code),
                style_references(&code),
                raw_size_references(&code),
            );
            (
                w + s + z,
                format!("  {name:<28} widgets={w:<4} style={s:<4} sizes={z}"),
            )
        })
        .filter(|(total, _)| *total > 0)
        .collect();
    rows.sort_by_key(|(total, _)| std::cmp::Reverse(*total));
    rows.into_iter()
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn no_feature_module_builds_a_styled_widget() {
    let (widgets, _, _) = totals();
    assert!(
        widgets == WIDGET_BUDGET,
        "feature modules construct wrapped rendering widgets at {widgets} lines; the boundary is \
         closed at {WIDGET_BUDGET}. Compose a component from `ui/material/` instead — and if none \
         of them expresses what this needs, the component gains the capability (FR-002).\n{}",
        breakdown()
    );
}

#[test]
fn no_feature_module_reaches_the_styling_layer() {
    let (_, styles, _) = totals();
    assert!(
        styles == STYLE_BUDGET,
        "feature modules reference the styling layer at {styles} lines; the boundary is closed at \
         {STYLE_BUDGET}. `material::style` is `pub(crate)`, so reaching it means the reference is \
         inside the library — or someone widened its visibility.\n{}",
        breakdown()
    );
}

#[test]
fn no_feature_module_picks_a_raw_text_size() {
    let (_, _, sizes) = totals();
    assert!(
        sizes == RAW_SIZE_BUDGET,
        "feature modules select a raw text size at {sizes} lines; the boundary is closed at \
         {RAW_SIZE_BUDGET}. Name a `TypeRole` — the role owns the number, which is what makes the \
         type scale a single edit.\n{}",
        breakdown()
    );
}

/// The budgets are only meaningful against a real scan. If `ui/` is restructured and this stops
/// finding modules, three assertions would pass by measuring nothing.
#[test]
fn the_scan_actually_finds_the_feature_modules() {
    let modules = feature_modules();
    assert!(
        modules.len() >= 10,
        "expected the feature modules under ui/, found {}: {:?}",
        modules.len(),
        modules.keys().collect::<Vec<_>>()
    );
    for expected in ["shell.rs", "sidebar.rs", "worktree_form.rs", "mod.rs"] {
        assert!(modules.contains_key(expected), "missing {expected}");
    }
    // Feature 020: the component showcase composes components exactly as a feature module does, and
    // FR-021 says it must not become a second implementation of anything. A gallery is where the
    // temptation to hand-style "just this one wrapper so it reads better" is strongest, and where a
    // styled copy would do the most damage — a developer comparing the showcase's button to the
    // application's would be comparing two different buttons. A directory this scan cannot see is
    // exempt in fact, whatever the spec says.
    // Feature 027: Settings became a directory of sections. A feature module that grows into one
    // is the case this scan's flat `read_dir` could not see, and the sections are where the
    // sandbox's controls live — the newest hand-styling temptation in the crate.
    for expected in ["ui/settings/daemon.rs", "ui/settings/terminal.rs"] {
        assert!(
            modules.contains_key(expected),
            "the settings sections are not being scanned — a feature module that became a \
             directory is bound by the same boundary as one that stayed a file. Found: {:?}",
            modules.keys().collect::<Vec<_>>()
        );
    }
    for expected in ["showcase/gallery.rs", "showcase/catalogue.rs"] {
        assert!(
            modules.contains_key(expected),
            "the showcase's sources are not being scanned — FR-021 binds them to the same boundary as \
             the application's feature modules. Found: {:?}",
            modules.keys().collect::<Vec<_>>()
        );
    }
}

/// SC-001 in one statement: three measured counts, all zero, from a baseline of 86 / 113 / 114.
#[test]
fn the_boundary_is_closed() {
    let (widgets, styles, sizes) = totals();
    assert_eq!(
        (widgets, styles, sizes),
        (0, 0, 0),
        "SC-001 requires all three counts at zero; the offenders are:\n{}",
        breakdown()
    );
}
