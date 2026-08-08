//! Feature modules name no rendering framework (feature 021, T014a — FR-006).
//!
//! Feature 021 sites the application's feature modules in this crate rather than in the render-free
//! core, and the entire argument for doing so is that they stay render-free anyway (spec, Q2). That
//! argument is only worth anything if something checks it. `app.rs` has held the line by convention
//! for twenty features — it mentions iced in comments and never in code — but convention is exactly
//! what stops holding once eight modules exist instead of one and each looks small enough to be an
//! exception.
//!
//! So this reads the source and fails on the reach. It scans text rather than types deliberately,
//! for the same reason `cdk_no_appearance.rs` does: the point is that the *name* must not appear in
//! code, which is a property of the source rather than of what it compiles to.
//!
//! Comments are exempt. Explaining why a reducer avoids capturing state in a subscription closure
//! requires naming the framework, and forbidding that would push out useful commentary while
//! catching nothing.

use std::fs;
use std::path::{Path, PathBuf};

fn features_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/features")
}

/// Every `.rs` file under `src/features/`, recursively, as `(display path, source)`.
fn feature_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                out.push((name, fs::read_to_string(&path).expect("read source")));
            }
        }
    }
    let mut out = Vec::new();
    walk(&features_dir(), &mut out);
    out
}

/// Strip `//`-style comments so a mention in prose does not read as a dependency.
///
/// Deliberately crude: it does not understand `//` inside a string literal. A false positive there
/// would be a feature module embedding "//" in a literal *and* naming the framework beside it,
/// which is not a thing that happens quietly — and erring toward flagging is the right direction
/// for a guard.
fn code_only(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn no_feature_module_names_the_rendering_framework_in_code() {
    let offenders: Vec<String> = feature_sources()
        .into_iter()
        .filter_map(|(name, source)| {
            let code = code_only(&source);
            code.contains("iced").then(|| {
                let line = code
                    .lines()
                    .enumerate()
                    .find(|(_, l)| l.contains("iced"))
                    .map(|(i, l)| format!("{}: {}", i + 1, l.trim()))
                    .unwrap_or_default();
                format!("{name} ({line})")
            })
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "feature modules must stay render-free (FR-006), but these name the rendering framework \
         in code:\n  {}\n\nFeature 021 puts these modules in the client rather than the core on \
         the grounds that they need no renderer. A module that imports one belongs behind a view \
         in `src/ui/`, not here.",
        offenders.join("\n  ")
    );
}

#[test]
fn the_guard_is_actually_looking_at_something() {
    let sources = feature_sources();

    assert!(
        !sources.is_empty(),
        "found no sources under src/features/ — a guard that scans nothing passes vacuously, \
         which is worse than no guard at all"
    );
}
