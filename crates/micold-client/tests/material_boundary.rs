//! The library owns the rendering stack (feature 017, T016 — FR-001–FR-004, SC-001).
//!
//! Thirteen feature modules build rendering widgets and style them by hand. Each one is free to
//! render a slightly different button, and several do. The fix is structural rather than editorial:
//! the component library wraps the rendering stack, and a feature module composes components.
//!
//! This test measures the boundary and refuses to let it get worse. Each budget below is a ceiling
//! that only ever moves down; **all of them must be zero at T036**, when this test becomes the
//! blocking gate for SC-001. Keeping it advisory until then is deliberate — flipping it to zero on
//! the first migrated module would make every intermediate state unbuildable.
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

/// The styling module itself, which *is* the layer the rules point at.
const STYLE_MODULE: &str = "style.rs";

/// Every feature module's source, keyed by file name.
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
            assert!(
                LIBRARY.contains(&name.as_str()),
                "unexpected directory `ui/{name}/` — is it a new library layer? Add it to LIBRARY, \
                 or it will be silently exempt from the boundary rules"
            );
            continue;
        }
        if name == STYLE_MODULE || !name.ends_with(".rs") {
            continue;
        }
        out.insert(name, fs::read_to_string(&path).expect("read source"));
    }
    out
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
const WRAPPED_WIDGETS: &[&str] = &[
    "button", "text_input", "pick_list", "checkbox", "scrollable", "container", "progress_bar",
    "tooltip",
];

/// How many times a feature module names a wrapped rendering widget as a constructor call.
fn widget_calls(code: &str) -> usize {
    code.lines()
        .filter(|line| {
            WRAPPED_WIDGETS
                .iter()
                .any(|w| line.contains(&format!("{w}(")))
        })
        .count()
}

/// How many times a feature module reaches into the styling layer.
fn style_references(code: &str) -> usize {
    code.lines().filter(|line| line.contains("style::")).count()
}

/// How many times a feature module selects a raw text size instead of naming a type role.
fn raw_size_references(code: &str) -> usize {
    code.lines()
        .filter(|line| line.contains("type_scale::") || line.contains(".size("))
        .count()
}

// ---------------------------------------------------------------------------
// Budgets. Each must reach 0 at T036 (Phase 5). Lower them as modules migrate;
// never raise one.
// ---------------------------------------------------------------------------

/// Feature-module lines constructing a wrapped rendering widget. Measured baseline: **86** across
/// the 13 modules research R2 counted; seven remain. Lines rather than modules, because "13 modules leak" does
/// not shrink until a module reaches zero, and this needs to move on every migration.
const WIDGET_BUDGET: usize = 65;

/// Feature-module lines referencing the styling layer. Measured baseline: **113**.
const STYLE_BUDGET: usize = 83;

/// Feature-module lines selecting a raw text size. Measured baseline: **114**.
const RAW_SIZE_BUDGET: usize = 86;

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
            (w + s + z, format!("  {name:<28} widgets={w:<4} style={s:<4} sizes={z}"))
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
fn no_feature_module_builds_a_styled_widget_beyond_its_budget() {
    let (widgets, _, _) = totals();
    assert!(
        widgets <= WIDGET_BUDGET,
        "feature modules construct wrapped rendering widgets at {widgets} lines, above the \
         {WIDGET_BUDGET} ceiling — the boundary got worse.\n{}",
        breakdown()
    );
}

#[test]
fn no_feature_module_reaches_the_styling_layer_beyond_its_budget() {
    let (_, styles, _) = totals();
    assert!(
        styles <= STYLE_BUDGET,
        "feature modules reference the styling layer at {styles} lines, above the {STYLE_BUDGET} \
         ceiling. If a call site needs an appearance a wrapper cannot express, the wrapper gains \
         the capability (FR-002) — the call site must not reach around it.\n{}",
        breakdown()
    );
}

#[test]
fn no_feature_module_picks_a_raw_text_size_beyond_its_budget() {
    let (_, _, sizes) = totals();
    assert!(
        sizes <= RAW_SIZE_BUDGET,
        "feature modules select a raw text size at {sizes} lines, above the {RAW_SIZE_BUDGET} \
         ceiling — a size belongs to a type role, chosen by the component.\n{}",
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
}

/// The one that says what this is all for. Currently reports the distance to zero; at T036 the
/// budgets are zeroed and this becomes the blocking statement of SC-001.
#[test]
fn report_the_distance_to_zero() {
    let (widgets, styles, sizes) = totals();
    let remaining = widgets + styles + sizes;
    println!(
        "boundary: widgets={widgets}/{WIDGET_BUDGET} style={styles}/{STYLE_BUDGET} \
         sizes={sizes}/{RAW_SIZE_BUDGET} — {remaining} lines left to migrate\n{}",
        breakdown()
    );
    // Nothing to assert beyond the ceilings above until T036 zeroes them; this exists so the
    // number is visible in every run rather than only when something regresses.
}
