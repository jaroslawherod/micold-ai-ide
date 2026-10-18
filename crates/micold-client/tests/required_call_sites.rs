//! No form draws its own required marker (feature 937, contract C5, FR-008).
//!
//! The marker is the shared field's to draw: a call site declares a field required with
//! `.required(..)` and `FormField` appends `*` to the label. A call site that writes the asterisk
//! into the label string instead gets a marker that ignores the field's requirement, doubles when
//! the field is also declared required, and is the per-form convention the issue asks to replace.
//!
//! # What is checked
//!
//! Every `.label(` in a rendering module outside `ui/material/` whose argument is a string literal
//! must not end in `*`. Source text rather than pixels, as `field_focus_call_sites.rs` argues: the
//! property is one of the call site. The legend gets the same check (C5, SC-003, SC-006): no file
//! outside `ui/material/` spells `* required`, and every form that has a required field calls
//! `RequiredLegend`. The error message gets the same check (C5, SC-006): no file outside `ui/material/` spells
//! `This field is required`.

#[path = "support/source_scan.rs"]
mod source_scan;

use source_scan::{rs_files, src_dir, strip_comments_keep_lines};

/// Every `.rs` file under `ui/` outside `ui/material/`, as `(path under src, source)`.
fn rendering_files() -> Vec<(String, String)> {
    let ui = src_dir().join("ui");
    rs_files(&ui)
        .into_iter()
        .filter(|path| {
            let rel = path.strip_prefix(&ui).unwrap_or(path);
            !rel.components().any(|c| c.as_os_str() == "material")
        })
        .map(|path| {
            let name = path
                .strip_prefix(src_dir())
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&path).expect("read source");
            (name, src)
        })
        .collect()
}

/// The string-literal arguments of every `.label(` in `source` that end in `*`, with their line.
fn starred_labels(source: &str) -> Vec<(usize, String)> {
    let code = strip_comments_keep_lines(source);
    let mut found = Vec::new();
    let mut rest = code.as_str();
    let mut offset = 0;
    while let Some(at) = rest.find(".label(") {
        let start = offset + at + ".label(".len();
        let after = code[start..].trim_start();
        if let Some(body) = after.strip_prefix('"') {
            let mut literal = String::new();
            let mut chars = body.chars();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => {
                        if let Some(next) = chars.next() {
                            literal.push('\\');
                            literal.push(next);
                        }
                    }
                    '"' => break,
                    _ => literal.push(c),
                }
            }
            if literal.trim_end().ends_with('*') {
                let line = code[..start].matches('\n').count() + 1;
                found.push((line, literal));
            }
        }
        offset = start;
        rest = &code[start..];
    }
    found
}

#[test]
fn no_form_writes_the_marker_into_a_label() {
    let offenders: Vec<String> = rendering_files()
        .iter()
        .flat_map(|(name, src)| {
            starred_labels(src)
                .into_iter()
                .map(move |(line, label)| format!("{name}:{line}: .label(\"{label}\")"))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "these labels draw their own required marker; declare the field required with \
         `.required(..)` instead and let the shared field draw it (feature 937, FR-008):\n{}",
        offenders.join("\n")
    );
}

/// The views that draw a form with a required field: the dialogs and the Settings pages (R5).
const LEGEND_FORMS: [&str; 8] = [
    "ui/worktree_form.rs",
    "ui/parallel_dialog.rs",
    "ui/rename_dialog.rs",
    "ui/settings/terminal.rs",
    "ui/settings/environment.rs",
    "ui/settings/github.rs",
    "ui/settings/jira.rs",
    "ui/settings/daemons.rs",
];

/// Whether `source` writes the legend words as a string literal, comments aside.
fn spells_the_legend(source: &str) -> bool {
    strip_comments_keep_lines(source).contains("\"* required\"")
}

/// Whether `source` calls the shared legend component, comments aside.
fn calls_the_legend(source: &str) -> bool {
    strip_comments_keep_lines(source).contains("RequiredLegend::")
}

#[test]
fn no_form_spells_the_legend() {
    let offenders: Vec<String> = rendering_files()
        .into_iter()
        .filter(|(_, src)| spells_the_legend(src))
        .map(|(name, _)| name)
        .collect();
    assert!(
        offenders.is_empty(),
        "these files write the required legend themselves; push `RequiredLegend` instead \
         (feature 937, FR-008):\n{}",
        offenders.join("\n")
    );
}

#[test]
fn every_form_with_a_required_field_calls_the_legend() {
    let files = rendering_files();
    let missing: Vec<&str> = LEGEND_FORMS
        .iter()
        .copied()
        .filter(|form| {
            !files
                .iter()
                .any(|(name, src)| name == form && calls_the_legend(src))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "these forms have a required field and do not call `RequiredLegend` (feature 937, \
         FR-004, SC-003):\n{}",
        missing.join("\n")
    );
}

/// Whether `source` writes the required message as a string literal, comments aside.
fn spells_the_message(source: &str) -> bool {
    strip_comments_keep_lines(source).contains("\"This field is required\"")
}

/// The rendering modules and the feature modules, where a form could write its own message.
#[test]
fn no_form_spells_the_required_message() {
    let features = rs_files(&src_dir().join("features"))
        .into_iter()
        .map(|path| {
            let src = std::fs::read_to_string(&path).expect("read source");
            (path.display().to_string(), src)
        });
    let offenders: Vec<String> = rendering_files()
        .into_iter()
        .chain(features)
        .filter(|(_, src)| spells_the_message(src))
        .map(|(name, _)| name)
        .collect();
    assert!(
        offenders.is_empty(),
        "these files write the required message themselves; declare the field required and let \
         the shared field draw it (feature 937, FR-008, SC-006):\n{}",
        offenders.join("\n")
    );
    assert!(spells_the_message(
        r#"Text::new("This field is required", r)"#
    ));
    assert!(!spells_the_message(
        "// \"This field is required\" is the library's"
    ));
}

#[test]
fn the_legend_scan_tells_the_literal_from_the_call() {
    assert!(spells_the_legend(r#"Text::new("* required", r)"#));
    assert!(!spells_the_legend(
        "// \"* required\" is drawn by the library"
    ));
    assert!(calls_the_legend("fields.push(RequiredLegend::new(r))"));
    assert!(!calls_the_legend("// RequiredLegend goes here"));
}

#[test]
fn the_sweep_reaches_the_forms() {
    let names: Vec<String> = rendering_files().into_iter().map(|(n, _)| n).collect();
    for form in [
        "ui/worktree_form.rs",
        "ui/parallel_dialog.rs",
        "ui/rename_dialog.rs",
    ] {
        assert!(
            names.iter().any(|n| n == form),
            "the sweep no longer reads {form}"
        );
    }
    assert!(
        !names.iter().any(|n| n.contains("/material/")),
        "the sweep reads the library itself, where the marker is drawn"
    );
}

#[test]
fn the_scan_finds_a_starred_label_and_nothing_else() {
    let source = r#"
        let a = TextField::new("", v, r).label("Name*");
        let b = TextField::new("", v, r).label( "Ticket *" );
        let c = TextField::new("", v, r).label("Name");
        let d = TextField::new("", v, r).label(spec.label);
        // .label("Comment*") is not code
        let e = Text::new("a * b", role, r).label("Rate \"x\"");
    "#;
    let found: Vec<String> = starred_labels(source).into_iter().map(|(_, l)| l).collect();
    assert_eq!(found, vec!["Name*".to_string(), "Ticket *".to_string()]);
}

/// The views that build a required `Select` (feature 937, research R5): Type and Base branch.
const SELECT_FORMS: [&str; 2] = ["ui/worktree_form.rs", "ui/parallel_dialog.rs"];

/// The source of every `Select::new(..)` expression in `source`, up to the end of its builder
/// chain: the `;` that ends the statement, or the `)` that closes the call it is an argument of.
fn select_expressions(source: &str) -> Vec<(usize, String)> {
    let code = strip_comments_keep_lines(source);
    code.match_indices("Select::new(")
        .map(|(at, _)| {
            let mut depth = 0i32;
            let mut end = code.len();
            for (i, c) in code[at..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                if depth < 0 || (depth == 0 && c == ';') {
                    end = at + i;
                    break;
                }
            }
            (
                code[..at].matches('\n').count() + 1,
                code[at..end].to_string(),
            )
        })
        .collect()
}

/// A required select that does not `track_focus` never reports being left, so FR-006's blur rule
/// silently covers text fields only (contract C4).
#[test]
fn a_required_select_in_an_inventory_view_tracks_focus() {
    let mut required_selects = 0;
    let mut offenders = Vec::new();
    for (name, src) in rendering_files()
        .iter()
        .filter(|(name, _)| SELECT_FORMS.contains(&name.trim_start_matches("src/")))
    {
        for (line, expr) in select_expressions(src) {
            if !expr.contains(".required(") {
                continue;
            }
            required_selects += 1;
            if !expr.contains(".track_focus(") {
                offenders.push(format!("{name}:{line}"));
            }
        }
    }
    assert_eq!(
        required_selects, 3,
        "Type in the add-worktree form, Type and Base branch in Run in parallel: the scan moved"
    );
    assert!(
        offenders.is_empty(),
        "these required selects are built without `.track_focus(FieldId, focused)`, so leaving \
         them never marks them (feature 937, C4):\n{}",
        offenders.join("\n")
    );
}
