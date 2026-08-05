//! No call site states a text size (feature 018, T014 — FR-010, SC-003).
//!
//! Feature 017 replaced 135 raw sizes with named roles, and feature 018 gave those roles a weight
//! and a line height. Both are undone by a single `.size(14.0)` slipping back in: that text is now
//! outside the scale, it will not follow when a role is re-valued, and nothing about it looks wrong
//! — 14 *is* the body size today. It only becomes wrong later, silently, in one place.
//!
//! So the rule is structural rather than aesthetic: a text size is spelled as a role, always. The
//! scale is only a scale for as long as everything is in it.
//!
//! # What counts
//!
//! `.size(...)` and `.line_height(...)` on text, given a bare number. Widget *geometry* — a
//! `Length`, a canvas rect, a measured bound — also goes through methods called `size`, and those
//! are not type-scale decisions, so the check looks for numeric literals specifically rather than
//! for the method name.
//!
//! # The hole this used to have
//!
//! A literal is only the *obvious* way to state a size. Feature 003's `type_scale::BODY` was a
//! **named** constant, so it sailed past a scan looking for numbers — and eleven shared components
//! were still using it when feature 018 came to assign roles. They got the right size and, because
//! a bare number carries neither, the default weight and the renderer's default line spacing. The
//! menu, the toolbar, every chip and the whole project switcher were outside the type system while
//! passing a test named "no call site states a raw text size".
//!
//! So the second rule below is structural rather than a blocklist: the Material scale is named in
//! exactly **one** file, the one that turns a role into a size. Anywhere else, a call site can only
//! reach the scale through a role — which is the property the first rule was trying to buy.

use std::fs;
use std::path::{Path, PathBuf};

/// Everything that renders: the shared library, the feature modules, and the showcase — which is
/// held to the same rules as the application it demonstrates (feature 020, FR-023).
fn rendering_dirs() -> Vec<PathBuf> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    vec![src.join("ui"), src.join("showcase")]
}

fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                out.push((name, fs::read_to_string(&path).expect("read source")));
            }
        }
    }
    let mut out = Vec::new();
    for dir in rendering_dirs() {
        walk(&dir, &mut out);
    }
    out.sort();
    out
}

/// Strips comments, so prose about sizes is not mistaken for setting one.
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

/// `true` when `arg` is a bare numeric literal rather than a named value.
fn is_numeric_literal(arg: &str) -> bool {
    let a = arg.trim().trim_end_matches("f32").trim_end_matches("f64");
    !a.is_empty()
        && a.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '_')
}

/// The argument text of each `call(` occurrence, up to the matching close paren.
fn args_of<'a>(line: &'a str, call: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(i) = rest.find(call) {
        let after = &rest[i + call.len()..];
        let mut depth = 1usize;
        let mut end = after.len();
        for (j, c) in after.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = j;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push(&after[..end]);
        rest = &after[end.min(after.len())..];
    }
    out
}

/// The calls that set a type-scale property. `.size(` is included even though widget geometry
/// shares the name: a *literal* passed to it is a raw size either way, and a `Length` is spelled
/// `Length::Fixed(..)` or `Fill`, never a bare number.
const TYPE_SETTERS: &[&str] = &[".size(", ".line_height(", ".weight("];

fn offences() -> Vec<String> {
    let mut out = Vec::new();
    for (path, src) in sources() {
        for (i, line) in code_only(&src).lines().enumerate() {
            for setter in TYPE_SETTERS {
                for arg in args_of(line, setter) {
                    if is_numeric_literal(arg) {
                        out.push(format!(
                            "  {path}:{}  {}{arg})",
                            i + 1,
                            setter.trim_start_matches('.')
                        ));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn no_call_site_states_a_raw_text_size() {
    let found = offences();
    assert!(
        found.is_empty(),
        "a raw number is being passed where a type role belongs:\n{}\n\nThe scale is only a scale \
         while everything is in it. Name a `TypeRole` — the role carries size, weight and line \
         height together, and follows when the scale is re-valued (FR-010, SC-003).",
        found.join("\n")
    );
}

/// The one file allowed to name the Material scale: the table that turns a role into a size.
const RESOLVES_ROLES: &str = "ui/material/text.rs";

/// Ways of naming the scale directly instead of going through a role. `type_scale::*` and the
/// `sidebar::*` sizes were feature 003's flat constants — deleted at T017–T021, and listed here so
/// re-adding one is a failure rather than a quiet return to two sources of truth.
///
/// The sidebar entries name their constants rather than the module: `crate::ui::sidebar` is a
/// feature module, and `sidebar::view(..)` is not a type-scale decision.
const NAMES_THE_SCALE: &[&str] = &[
    "typography::",
    "type_scale::",
    "sidebar::NAME",
    "sidebar::TAG",
    "sidebar::SESSION",
];

#[test]
fn only_the_role_table_names_the_type_scale() {
    let mut found = Vec::new();
    for (path, src) in sources() {
        if path == RESOLVES_ROLES || path.ends_with("type_role_mapping.rs") {
            continue;
        }
        for (i, line) in code_only(&src).lines().enumerate() {
            for needle in NAMES_THE_SCALE {
                if line.contains(needle) {
                    found.push(format!("  {path}:{}  names `{needle}`", i + 1));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "the Material type scale is named outside `{RESOLVES_ROLES}`:\n{}\n\nA size reached this \
         way carries no weight and no line height, so the text renders at the renderer's defaults \
         while looking, at the call site, like it is in the scale. Name a `TypeRole` instead — it \
         carries all three together (FR-007, FR-010).",
        found.join("\n")
    );
}

/// The rule above is only worth anything if the file it exempts is really the resolution table.
/// If `text.rs` stops naming the scale, the exemption is stale and the rule is guarding nothing.
#[test]
fn the_exempt_file_is_the_one_that_resolves_roles() {
    let table = sources()
        .into_iter()
        .find(|(path, _)| path == RESOLVES_ROLES)
        .map(|(_, src)| src)
        .unwrap_or_else(|| panic!("{RESOLVES_ROLES} is not in the rendering layer any more"));
    assert!(
        table.contains("typography::"),
        "{RESOLVES_ROLES} no longer names the Material scale, so exempting it from the rule above \
         guards nothing. Move the exemption to wherever roles now resolve."
    );
}

/// The scale rule fires on a real offence and stays quiet on the module that shares a name.
///
/// `sidebar::view(..)` is a feature module's entry point and `sidebar::TAG` was a text size; a
/// needle of `sidebar::` cannot tell them apart, and the version of this rule that used one failed
/// on the application's own layout code.
#[test]
fn the_scale_rule_tells_a_size_from_a_module_of_the_same_name() {
    let fires = |line: &str| NAMES_THE_SCALE.iter().any(|n| line.contains(n));

    assert!(fires("        .label_size(sidebar::NAME)"));
    assert!(fires(
        "    pub const BODY: f32 = typography::BODY_MEDIUM.size;"
    ));
    assert!(fires("        text(t.title).size(type_scale::BODY),"));

    assert!(!fires("            sidebar::view(state, scheme),"));
    assert!(!fires("use crate::ui::sidebar;"));
    assert!(!fires(
        "        Text::new(\"Worktrees\", TypeRole::Section, r)"
    ));
}

/// A scan that reads nothing passes trivially.
#[test]
fn the_scan_actually_reads_the_rendering_layer() {
    let sources = sources();
    assert!(
        sources.len() > 25,
        "found only {} sources — the check above would be near-vacuous",
        sources.len()
    );
}

/// The synthetic Red: the rule fires on a real offence, and names it.
#[test]
fn a_raw_size_would_be_caught() {
    let line = "text(label).size(14.0)";
    let args = args_of(line, ".size(");
    assert_eq!(args, vec!["14.0"]);
    assert!(is_numeric_literal("14.0"));
    assert!(is_numeric_literal("11"));
    assert!(is_numeric_literal("16f32"));
}

/// …and does not fire on the things that legitimately look similar: a named role, a computed
/// value, or widget geometry expressed as a `Length`.
#[test]
fn named_roles_and_widget_geometry_are_not_offences() {
    for arg in [
        "TypeRole::Body.size()",
        "role.size()",
        "t.role.size()",
        "Length::Fixed(24.0)",
        "typography::BODY_MEDIUM.size",
        "sidebar::TAG",
    ] {
        assert!(
            !is_numeric_literal(arg),
            "{arg} is a named value, not a raw literal"
        );
    }
}
