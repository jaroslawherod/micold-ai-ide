//! The Material crate names no Micold domain type (feature 1262).
//!
//! `micold-material` holds the generic building blocks; Micold's own widgets — sessions, daemons,
//! worktrees, the queue, diffs, pull requests — compose them from the client. The crate boundary
//! keeps the client out (this crate cannot name `micold_client`), but it still depends on
//! `micold-core` for the design tokens and a few render-free models, and that crate holds the domain
//! too. This test closes the gap: every `micold_core::` path under `src/` must start with one of
//! the modules below.
//!
//! Text scanning, like the client's boundary gates: the property is about what a source file is
//! allowed to name.

use std::fs;
use std::path::{Path, PathBuf};

/// The `micold_core` modules a generic component may read: design tokens and the colour scheme,
/// and the render-free models of a form's required marker, a typeahead's filter, an overlay's
/// placement, a tooltip's rest, a notification's level and a markdown document. None of them knows
/// what a session or a worktree is.
const ALLOWED: &[&str] = &[
    "tokens",
    "theme",
    "required",
    "typeahead",
    "overlay",
    "tooltip",
    "notify",
    "markdown",
];

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every `micold_core::<module>` the crate's sources name, with where, comments included: a doc
/// link to a domain type is the first step towards a use of one.
fn core_modules() -> Vec<(String, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rs_files(&src, &mut files);
    assert!(
        files.len() > 50,
        "scanned only {} files under src/",
        files.len()
    );
    let mut found = Vec::new();
    for path in files {
        let text = fs::read_to_string(&path).expect("read source");
        for (n, line) in text.lines().enumerate() {
            for (at, _) in line.match_indices("micold_core::") {
                let rest = &line[at + "micold_core::".len()..];
                let module: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                let key = format!(
                    "{}:{}",
                    path.strip_prefix(&src).unwrap_or(&path).display(),
                    n + 1
                );
                found.push((key, module));
            }
            // An alias (`use micold_core as core;`) would hide every later path from the scan.
            assert!(
                !line.contains("micold_core as "),
                "{}:{} aliases `micold_core`; name its modules by their full path",
                path.display(),
                n + 1
            );
            assert!(
                !line.contains("micold_client"),
                "{}:{} names the client crate",
                path.display(),
                n + 1
            );
        }
    }
    found
}

#[test]
fn the_material_crate_reads_only_render_free_core_modules() {
    let found = core_modules();
    assert!(
        !found.is_empty(),
        "the scan found no `micold_core::` path at all"
    );
    let domain: Vec<String> = found
        .iter()
        .filter(|(_, module)| !ALLOWED.contains(&module.as_str()))
        .map(|(at, module)| format!("  {at}: micold_core::{module}"))
        .collect();
    assert!(
        domain.is_empty(),
        "the Material crate names a Micold domain module:\n{}\n\nA component that needs it is \
         Micold-specific: build it in the client's `ui::material` from this crate's building \
         blocks, or pass the value in as a plain type",
        domain.join("\n")
    );
}
