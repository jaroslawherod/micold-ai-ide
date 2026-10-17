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
//! property is one of the call site. The legend and the error message get the same check in later
//! milestones of the feature.

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
