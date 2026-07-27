//! No component API leaks how it works (feature 017, T038 — FR-013, SC-004).
//!
//! A call site should describe *what it wants* — a label, a variant, a message to emit — and
//! nothing about how the result is rendered or animated. The moment a signature carries a progress
//! value, every caller becomes responsible for owning and threading one, which is exactly the
//! arrangement this feature removes: `ui::view` used to take an `Animator<MotionKey>` and pass a
//! float down through fifteen functions.
//!
//! Enforced by reading the library's own public signatures. A rule that lives only in review
//! decays between reviews.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn library_dirs() -> Vec<PathBuf> {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    vec![ui.join("material"), ui.join("cdk")]
}

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

/// Strips comments, so prose explaining the rule is not mistaken for breaking it.
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

/// The styling layer, which is deliberately exempt.
///
/// Contract §6: it is not a component and has no builder — it exists precisely to produce style
/// functions, so "names a style function" describes its job rather than a leak. It is `pub(crate)`
/// and unreachable from a call site, which is what actually keeps callers away from it.
const STYLE_MODULE: &str = "material/style.rs";

/// Every `pub fn` signature in the library, as `(module, whitespace-normalised signature)`.
///
/// Signatures are gathered whole rather than line by line. Most of this library's are wrapped
/// across several lines by the formatter, and a line-based scan sees only `pub fn slide<'a, M: 'a>(`
/// — the parameters, which are the entire point of the check, sit on their own lines and go
/// unread. That made an empty [`REMAINING`] provable without being true.
fn public_signatures() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (module, src) in library_sources() {
        if module == STYLE_MODULE {
            continue;
        }
        let code = code_only(&src);
        for (start, _) in code
            .match_indices("pub fn ")
            .chain(code.match_indices("pub const fn "))
        {
            // A signature runs to the body's opening brace (or `;` for a trait method). Neither
            // appears earlier: a return type may name generics and lifetimes, never a block.
            let end = code[start..]
                .find(['{', ';'])
                .map(|offset| start + offset)
                .unwrap_or(code.len());
            let signature = code[start..end]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            out.push((module.clone(), signature));
        }
    }
    out
}

/// Things a public component signature must not name, and what each would mean if it did.
const FORBIDDEN: &[(&str, &str)] = &[
    ("MotionKey", "an animation key the caller has to allocate"),
    (
        "Animator",
        "an animator handle the caller has to own and tick",
    ),
    ("Track", "the animator's internal per-element state"),
    (
        "progress: f32",
        "a progress value the caller has to compute and thread through its view",
    ),
    (
        "impl Fn(&Theme",
        "a style function — the caller deciding an appearance the component owns",
    ),
    (
        "container::Style",
        "a resolved rendering style rather than a semantic input",
    ),
    (
        "button::Status",
        "a rendering-stack widget status the caller would have to interpret",
    ),
];

/// Signatures that still take a progress value **and should not**.
///
/// Each is a component whose presentation state has not yet moved into it (T039–T042). They are
/// listed rather than exempted: `the_remaining_leaks_are_exactly_these` fails both when one is
/// fixed without being struck off and when a new one appears, so the list can only shrink. Empty
/// is the finish line for FR-013 and SC-004.
///
/// There is no companion list of *sanctioned* leaks. There used to be: the four animation wrappers
/// were exempt on the grounds that a transition primitive is legitimately about its own progress.
/// That reasoning dissolved once they began owning their tracks — `fade(content, shown, over,
/// backdrop)` says where the element should be, not how far along it is — so the exemption went
/// with it. Nothing in the library is now allowed to name a progress value it does not own.
const REMAINING: &[(&str, &str)] = &[
    (
        "material/animation.rs",
        "the sidebar drawer's slide is still central: at zero width the sidebar is replaced by \
         the collapsed rail, and owning that decision means owning both elements (T039)",
    ),
    (
        "material/divider.rs",
        "the resize handle's hover track is still central (T041)",
    ),
];

/// Every module whose public surface still names something forbidden.
fn leaking_modules() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (module, sig) in public_signatures() {
        for (needle, meaning) in FORBIDDEN {
            if sig.contains(needle) {
                out.push((
                    module.clone(),
                    format!("`{sig}` names `{needle}` — {meaning}"),
                ));
            }
        }
    }
    out
}

#[test]
fn no_public_component_signature_exposes_how_it_animates() {
    let known: Vec<&str> = REMAINING.iter().map(|(m, _)| *m).collect();
    let unexpected: Vec<String> = leaking_modules()
        .into_iter()
        .filter(|(module, _)| !known.contains(&module.as_str()))
        .map(|(module, detail)| format!("{module}: {detail}"))
        .collect();
    assert!(
        unexpected.is_empty(),
        "a component's API describes what the caller wants, never how it is rendered or animated \
         (FR-013). These are new — add the state to the component, do not add it to REMAINING:\n{}",
        unexpected.join("\n")
    );
}

/// The ratchet. Fixing one of the three without striking it off fails here, so the list cannot
/// quietly outlive the problem it describes.
#[test]
fn the_remaining_leaks_are_exactly_these() {
    let mut actual: Vec<String> = leaking_modules().into_iter().map(|(m, _)| m).collect();
    actual.sort();
    actual.dedup();
    let mut expected: Vec<String> = REMAINING.iter().map(|(m, _)| m.to_string()).collect();
    expected.sort();
    assert_eq!(
        actual,
        expected,
        "REMAINING is out of date. If a component was fixed, remove its entry; if one regressed, \
         fix the component rather than the list.\nStill outstanding:\n{}",
        REMAINING
            .iter()
            .map(|(m, why)| format!("  {m} — {why}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The global enumeration and the central animators must not reach into the library at all — not
/// in a signature, not in a body. Their existence is what this phase removes.
#[test]
fn the_library_never_names_the_global_animation_machinery() {
    let mut violations = Vec::new();
    for (module, src) in library_sources() {
        for (line_no, line) in code_only(&src).lines().enumerate() {
            for needle in ["MotionKey", "Animator<", "crate::motion::Animator"] {
                if line.contains(needle) {
                    violations.push(format!("{module}:{} names `{needle}`", line_no + 1));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "components own their presentation state; nothing in the library may reach for a central \
         animator (FR-014, FR-015):\n{}",
        violations.join("\n")
    );
}

/// A scan that finds nothing passes both assertions above.
#[test]
fn the_scan_finds_the_signatures_it_should() {
    let sigs = public_signatures();
    assert!(
        sigs.len() >= 40,
        "expected the library's public surface, found {} signatures",
        sigs.len()
    );
    // The wrapped-signature case specifically: `fade`'s parameters live on their own lines, and a
    // scan that stopped at the first newline would report it as taking nothing at all.
    let fade = sigs
        .iter()
        .find(|(_, s)| s.starts_with("pub fn fade"))
        .expect("expected `fade` among the signatures");
    assert!(
        fade.1.contains("backdrop: micold_core::tokens::Rgb"),
        "the scan must read a signature's parameters, not just its first line: {}",
        fade.1
    );
}
