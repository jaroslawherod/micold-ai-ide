//! Source-level guard: a feature module may not build an icon+label composite by hand.
//!
//! **The hole this closes.** `anatomy_call_sites.rs` asks whether each §7 figure reaches *a*
//! component. `material_boundary.rs` asks whether a feature module names a *styled* widget — and
//! deliberately exempts layout primitives, because a row has nothing to style. Between the two
//! sits a shape neither can see: `row![Glyph::new(..), Text::new(..)]`. It names no styled widget,
//! reaches no styling layer, picks no raw text size, and every figure it gets wrong is still bound
//! somewhere else. Both gates stay green while the call site draws the wrong thing.
//!
//! That is not hypothetical. `shell.rs` carried five buttons in exactly this shape behind a local
//! `labeled()` helper, each sizing its glyph at the label's role — 14dp, where §7.3 puts a
//! button's leading icon at 18. Every gate passed. The manual quickstart §B4 pass found it, by
//! reading. This test is that reading, made repeatable.
//!
//! **The rule.** A `Glyph` and a `Text` may not be siblings in a row or column a feature module
//! writes. An icon with a label beside it is a component:
//!
//! - a leading icon inside a control → [`Button::leading`], which applies §7.3's 18dp;
//! - a labelled icon in its own right → [`IconLabel`], where the glyph takes the label's role.
//!
//! **What this does not claim.** A list row with a leading status marker is not caught here, and
//! should not be: in `shell.rs`'s known-projects entry the glyph and the name are separate
//! children of a §7.2 row, not a labelled icon, and §7.2's geometry has its own gates
//! (`material/anatomy_size.rs`, the covered states). The property here is narrow on purpose — it
//! is about a *composite handed on as one thing*, which is the shape a component owns. A gate
//! that also banned list rows could not reach zero without a `ListRow` component, and a gate that
//! cannot reach zero is a budget, not a rule.
//!
//! Text scanning, not type inspection, for the same reason `material_boundary.rs` gives: the
//! property is about what a source file is allowed to write.
//!
//! [`Button::leading`]: micold_client::ui::material::Button::leading
//! [`IconLabel`]: micold_client::ui::material::IconLabel

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The library layers, which *own* the composite and so are exempt. `button.rs` builds
/// `row![icon(..), b.content]` for its leading slot — that is the slot, not a violation of it.
const LIBRARY: &[&str] = &["material", "cdk"];

fn ui_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui")
}

fn showcase_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/showcase")
}

/// Every feature module's source, keyed by a path that says which side of the crate it came from.
///
/// Same two roots as `material_boundary.rs`: `src/ui/` minus the library layers, and all of
/// `src/showcase/`, which composes the library exactly as a feature module does (FR-021).
fn feature_modules() -> BTreeMap<String, String> {
    fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).expect("read dir") {
            let path = entry.expect("dir entry").path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            if path.is_dir() {
                walk(&path, &format!("{prefix}{name}/"), out);
            } else if name.ends_with(".rs") {
                out.insert(
                    format!("{prefix}{name}"),
                    fs::read_to_string(&path).expect("read source"),
                );
            }
        }
    }

    let mut out = BTreeMap::new();
    for entry in fs::read_dir(ui_dir()).expect("read ui dir") {
        let path = entry.expect("dir entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if path.is_dir() {
            if LIBRARY.contains(&name.as_str()) {
                continue;
            }
            walk(&path, &format!("ui/{name}/"), &mut out);
        } else if name.ends_with(".rs") {
            out.insert(
                format!("ui/{name}"),
                fs::read_to_string(&path).expect("read source"),
            );
        }
    }
    walk(&showcase_dir(), "showcase/", &mut out);
    out
}

/// Strip comments, so the commentary explaining a composite is free to name its parts.
fn code_only(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_block = false;
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            } else if c == '\n' {
                out.push('\n');
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

/// A `row![..]` / `column![..]` body, with its nested macro bodies blanked out.
///
/// Blanking the nesting is what makes the children *direct*. Without it an outer row containing a
/// nested labelled icon matches too, and a single violation is reported once per enclosing layer —
/// which reads as several problems where there is one.
struct Composite {
    line: usize,
    direct_children: String,
}

fn composites(code: &str) -> Vec<Composite> {
    let bytes = code.as_bytes();
    let mut spans: Vec<(usize, usize, usize)> = Vec::new(); // (open, close, start-of-macro)
    let mut at = 0;
    while at < bytes.len() {
        let Some(rel) = code[at..].find('!') else {
            break;
        };
        let bang = at + rel;
        let name_end = bang;
        let name_start = code[..name_end]
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
            .last()
            .map(|(i, _)| i)
            .unwrap_or(name_end);
        let name = &code[name_start..name_end];
        let after = code[bang + 1..]
            .char_indices()
            .find(|(_, c)| !c.is_whitespace())
            .map(|(i, c)| (bang + 1 + i, c));
        match (name, after) {
            ("row" | "column", Some((open, '['))) => {
                let mut depth = 1usize;
                let mut i = open + 1;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'[' => depth += 1,
                        b']' => depth -= 1,
                        _ => {}
                    }
                    i += 1;
                }
                spans.push((open + 1, i.saturating_sub(1), name_start));
                at = open + 1;
            }
            _ => at = bang + 1,
        }
    }

    spans
        .iter()
        .map(|&(open, close, start)| {
            let mut body: Vec<u8> = code[open..close].bytes().collect();
            // Blank any nested span that lies strictly inside this one.
            for &(_n_open, n_close, n_start) in &spans {
                if n_start > open && n_close <= close {
                    for b in body
                        .iter_mut()
                        .take(n_close.min(close) - open)
                        .skip(n_start - open)
                    {
                        if *b != b'\n' {
                            *b = b' ';
                        }
                    }
                }
            }
            Composite {
                line: code[..start].matches('\n').count() + 1,
                direct_children: String::from_utf8_lossy(&body).into_owned(),
            }
        })
        .collect()
}

/// Does this text construct a glyph? Both spellings: the `Glyph` builder and the bare `icon(..)`
/// helper the library also exposes.
fn builds_glyph(s: &str) -> bool {
    s.contains("Glyph::new(") || names_call(s, "icon")
}

/// Does this text construct a text widget? `Text::new` is the component; `text(` is the rendering
/// stack's, which `material_boundary.rs` lets a feature module reach only through the component —
/// but a showcase caption may name it, so both count here.
fn builds_text(s: &str) -> bool {
    s.contains("Text::new(") || names_call(s, "text")
}

/// `name(` at a word boundary, so `icon_role(` or `subtext(` is not mistaken for `icon(`/`text(`.
fn names_call(s: &str, name: &str) -> bool {
    let needle = format!("{name}(");
    let mut from = 0;
    while let Some(at) = s[from..].find(&needle) {
        let start = from + at;
        let preceded = start > 0
            && s[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == ':');
        if !preceded {
            return true;
        }
        from = start + needle.len();
    }
    false
}

/// Every hand-built composite, as `module:line`.
fn violations() -> Vec<String> {
    let mut out = Vec::new();
    for (name, src) in feature_modules() {
        let code = code_only(&src);
        for c in composites(&code) {
            if builds_glyph(&c.direct_children) && builds_text(&c.direct_children) {
                out.push(format!("{name}:{}", c.line));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The gate.
// ---------------------------------------------------------------------------

#[test]
fn no_feature_module_builds_an_icon_and_a_label_by_hand() {
    let found = violations();
    assert!(
        found.is_empty(),
        "a feature module composed a glyph and a label into a row of its own, at:\n  {}\n\n\
         An icon with a label beside it is a component, not a layout. Use `Button::leading` for a \
         leading icon inside a control (§7.3 sizes it at 18dp whatever the label's role), or \
         `IconLabel` for a labelled icon in its own right (the glyph takes the label's role). \
         Spelling the composite out is how `shell.rs`'s five buttons drew a 14dp glyph with every \
         other gate green.",
        found.join("\n  ")
    );
}

#[test]
fn the_scan_actually_reads_the_feature_modules() {
    let modules = feature_modules();
    assert!(
        modules.len() > 10,
        "expected the feature modules, found {}",
        modules.len()
    );
    for expected in ["ui/shell.rs", "ui/project_selector.rs"] {
        assert!(
            modules.contains_key(expected),
            "{expected} is not in the scan; the rule guards nothing there"
        );
    }
    assert!(
        modules.keys().any(|k| k.starts_with("showcase/")),
        "the showcase composes the library as a feature module does (FR-021) and must be scanned"
    );
}

#[test]
fn the_library_is_excluded_because_it_owns_the_composite() {
    let modules = feature_modules();
    for layer in LIBRARY {
        assert!(
            !modules
                .keys()
                .any(|k| k.starts_with(&format!("ui/{layer}/"))),
            "ui/{layer}/ is in the scan, but it is the layer that builds the composite — \
             `button.rs`'s leading slot is `row![icon(..), content]` and would fail its own rule"
        );
    }
    // Not merely absent from the scan: the exempted layer really does build a glyph into a row of
    // its own, so the exemption is load-bearing rather than decorative. Only the glyph half is
    // asserted — the slot's other child is `b.content`, an already-built `Element`, so `button.rs`
    // would not trip the full rule anyway. What matters is that the leading slot is still a row
    // here; if it stops being one, this exemption is protecting nothing and should be re-examined.
    let button = fs::read_to_string(ui_dir().join("material/button.rs")).expect("read button.rs");
    let code = code_only(&button);
    assert!(
        composites(&code)
            .iter()
            .any(|c| builds_glyph(&c.direct_children)),
        "button.rs no longer builds its leading slot as a row; re-check what this exemption is for"
    );
}

#[test]
fn the_guard_actually_works() {
    // The shape `shell.rs` shipped, reduced to its essentials.
    let offending = r#"
        fn labeled<'a>(glyph: Icon, role: TypeRole, label: &str, r: Roles) -> Row<'a, Message> {
            row![
                Glyph::new(glyph, role, r).tint(tint),
                Text::new(label.to_string(), role, r)
            ]
        }
    "#;
    let found = composites(&code_only(offending));
    assert_eq!(found.len(), 1, "expected one row span");
    assert!(
        builds_glyph(&found[0].direct_children) && builds_text(&found[0].direct_children),
        "the guard does not flag the shape it exists to flag"
    );

    // A row of labels only, and a row of glyphs only, are both fine.
    for benign in [
        "row![Text::new(a, role, r), Text::new(b, role, r)]",
        "row![Glyph::new(a, role, r), Glyph::new(b, role, r)]",
    ] {
        let c = composites(&code_only(benign));
        assert_eq!(c.len(), 1);
        assert!(
            !(builds_glyph(&c[0].direct_children) && builds_text(&c[0].direct_children)),
            "{benign} is not a labelled icon and must not be flagged"
        );
    }
}

#[test]
fn a_commented_out_composite_is_not_a_violation() {
    let commented = r#"
        // row![Glyph::new(a, role, r), Text::new(b, role, r)]
        /* row![Glyph::new(a, role, r), Text::new(b, role, r)] */
        let x = 1;
    "#;
    assert!(
        composites(&code_only(commented)).is_empty(),
        "comments are stripped before the scan, so prose about the shape is free to name it"
    );
}

#[test]
fn a_nested_composite_is_reported_once_and_against_the_inner_row() {
    // The outer row holds a labelled icon and nothing else of its own. Only the inner row is a
    // violation; reporting the outer one too would turn one defect into two.
    let nested = "row![some_button, row![Glyph::new(a, role, r), Text::new(b, role, r)]]";
    let found: Vec<usize> = composites(&code_only(nested))
        .into_iter()
        .filter(|c| builds_glyph(&c.direct_children) && builds_text(&c.direct_children))
        .map(|c| c.line)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one report for one nested composite, got {found:?}"
    );
}

#[test]
fn a_call_whose_name_merely_ends_in_a_widgets_is_not_a_construction() {
    // `icon_role(..)` resolves a tint and `subtext(..)` is somebody's helper; neither builds one.
    let benign = "row![icon_role(IconSurface::Badge, r), subtext(label)]";
    let c = composites(&code_only(benign));
    assert_eq!(c.len(), 1);
    assert!(
        !builds_glyph(&c[0].direct_children),
        "`icon_role(` was read as `icon(`"
    );
    assert!(
        !builds_text(&c[0].direct_children),
        "`subtext(` was read as `text(`"
    );
}
