//! Covered states are declared in exactly one place (feature 019, T029 — FR-016).
//!
//! FR-016 promises that registering an additional covered state takes a change in one place. That
//! is a promise about the shape of the code, so it is checked the way feature 017 checks its other
//! structural promises: by reading the source.
//!
//! Why it needs checking at all, when the arrangement is already correct — the same reason
//! `one_overlay_implementation` gives. Five overlay implementations did not arrive at once; they
//! accreted one at a time, each individually reasonable, and nothing noticed until the divergence
//! needed a feature to fix. A second registration site would be added the same way: a test that
//! needs "the main shell but with X" builds one inline rather than registering it, the fixture
//! never learns about it, and FR-016's claim quietly stops being true. This makes the second site
//! something that has to be argued for in a diff.
//!
//! Held both ways, like feature 017's ratchets: it fails when a state is constructed outside the
//! registry, **and** when the registry stops constructing one of the kinds it is supposed to hold.
//! A scan that cannot fire is worth nothing, and this feature has already produced one of those.

use std::fs;
use std::path::{Path, PathBuf};

/// The one file allowed to construct covered states, relative to `tests/`.
const REGISTRY: &str = "support/covered_states.rs";

/// The kinds whose construction counts as registering a state.
///
/// `RevealingState` is here for the same reason as `CoveredState`, though it is a separate list:
/// it is a state pinned partway through an animation, held apart from the fixture because
/// mid-animation geometry is deliberately not recorded (T030). Two lists, one file — FR-016 is
/// about where a state is declared, not about how many lists that file keeps.
const REGISTERED_KINDS: &[&str] = &["CoveredState", "RevealingState"];

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// Every `.rs` file under `tests/`, recursively, as `(path relative to tests/, source)`.
fn test_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .strip_prefix(tests_dir())
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                out.push((name, fs::read_to_string(&path).expect("read source")));
            }
        }
    }
    let mut out = Vec::new();
    walk(&tests_dir(), &mut out);
    out.sort();
    out
}

/// Strips comments, string literals and char literals, leaving executable code.
///
/// All three are load-bearing, and each was added because it produced a wrong answer:
///
/// - **Comments**, because `support/layout.rs` discusses both kinds in the doc comments above their
///   definitions, and this file names them throughout its own prose.
/// - **String literals**, because this file's own assertions quote `CoveredState {` to prove the
///   scan tells a construction from a definition. Without this, the scan reports *itself* as a
///   second registration site — which it did on its first run. Exempting this file by name would
///   have hidden a real registration here later; a state written inside a string is not a
///   registration, in any file.
/// - **Char literals**, because `ui_glyph_literals.rs` contains `'"'`, and a stripper that treated
///   that quote as opening a string would swallow the code after it. No registration hides there
///   today, but a scan whose parse is wrong is a scan whose silence means nothing.
///
/// Newlines inside removed regions are preserved so the result stays line-addressable.
fn code_only(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;

    // Copy every newline in `chars[from..to]` so line structure survives.
    let keep_lines = |out: &mut String, from: usize, to: usize| {
        for c in &chars[from..to.min(chars.len())] {
            if *c == '\n' {
                out.push('\n');
            }
        }
    };

    while i < chars.len() {
        let c = chars[i];

        // Line comment.
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Block comment, nesting-aware as Rust's are.
        if c == '/' && chars.get(i + 1) == Some(&'*') {
            let start = i;
            let mut depth = 1;
            i += 2;
            while i < chars.len() && depth > 0 {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            keep_lines(&mut out, start, i);
            continue;
        }

        // Raw string: `r"..."` or `r#"..."#`, with any number of hashes.
        if c == 'r' {
            let mut hashes = 0;
            while chars.get(i + 1 + hashes) == Some(&'#') {
                hashes += 1;
            }
            if chars.get(i + 1 + hashes) == Some(&'"') {
                let start = i;
                i += 2 + hashes;
                loop {
                    if i >= chars.len() {
                        break;
                    }
                    if chars[i] == '"' && (0..hashes).all(|h| chars.get(i + 1 + h) == Some(&'#')) {
                        i += 1 + hashes;
                        break;
                    }
                    i += 1;
                }
                keep_lines(&mut out, start, i);
                continue;
            }
        }

        // String literal.
        if c == '"' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                i += if chars[i] == '\\' { 2 } else { 1 };
            }
            i += 1;
            keep_lines(&mut out, start, i);
            continue;
        }

        // Char literal, as opposed to a lifetime: `'a'` and `'\n'` are literals, `'static` is not.
        if c == '\'' {
            let escaped = chars.get(i + 1) == Some(&'\\');
            let closes_immediately = chars.get(i + 2) == Some(&'\'');
            if escaped || closes_immediately {
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    i += if chars[i] == '\\' { 2 } else { 1 };
                }
                i += 1;
                continue;
            }
        }

        out.push(c);
        i += 1;
    }

    out
}

/// Whether `line` *constructs* `kind` — `Kind { ... }` — rather than defining it or naming its
/// type.
///
/// The distinction is the whole check. `support/layout.rs` contains `pub struct CoveredState {`
/// and several `&[CoveredState]` signatures; none of them registers anything, and a naive
/// `contains` would report the module that defines the type as the second registration site.
fn constructs(line: &str, kind: &str) -> bool {
    // `struct X {` defines it, `impl T for X {` implements on it, `X {` constructs it. Only the
    // last registers a state.
    const DEFINING: &[&str] = &["struct", "enum", "union", "trait", "impl", "for"];

    let needle = format!("{kind} {{");
    let mut from = 0;
    while let Some(at) = line[from..].find(&needle) {
        let start = from + at;
        from = start + needle.len();

        let before = line[..start].trim_end();

        // `MyCoveredState {` is a different type that happens to end in this name.
        let part_of_longer_name = line[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');

        let defines = before
            .split_whitespace()
            .next_back()
            .is_some_and(|word| DEFINING.contains(&word));

        if !part_of_longer_name && !defines {
            return true;
        }
    }
    false
}

/// Every file that constructs a covered state, with the kinds it constructs.
fn registration_sites() -> Vec<(String, Vec<&'static str>)> {
    let mut sites = Vec::new();
    for (name, src) in test_sources() {
        let code = code_only(&src);
        let kinds: Vec<&'static str> = REGISTERED_KINDS
            .iter()
            .copied()
            .filter(|kind| code.lines().any(|line| constructs(line, kind)))
            .collect();
        if !kinds.is_empty() {
            sites.push((name, kinds));
        }
    }
    sites
}

/// FR-016: one place, and it is the one named.
#[test]
fn covered_states_are_declared_in_exactly_one_place() {
    let sites = registration_sites();
    let elsewhere: Vec<&(String, Vec<&str>)> =
        sites.iter().filter(|(name, _)| name != REGISTRY).collect();
    let n = elsewhere.len();

    assert!(
        elsewhere.is_empty(),
        "covered states are constructed in {n} file(s) besides {REGISTRY}, so FR-016's promise that \
         registering a state takes a change in one place is no longer true. A state built outside \
         the registry is invisible to the fixture, to the containment gate and to the text-overflow \
         gate — it covers nothing while looking like coverage. Move it into the registry, or, if it \
         genuinely is not a covered state, give it a type that is not one of {REGISTERED_KINDS:?}. \
         Found: {:?}",
        elsewhere,
    );
}

/// The scan must be able to *see* a registration, or its silence means nothing.
///
/// Both halves matter. If the registry stopped constructing one of the kinds — renamed, moved,
/// replaced by a builder — the test above would pass over an empty codebase and report FR-016 as
/// held. This is the same ratchet feature 017 puts on its closed lists, and the same lesson this
/// feature learned the hard way from a gate that could not fire.
#[test]
fn the_registry_still_registers_every_kind() {
    let sites = registration_sites();
    let registry = sites
        .iter()
        .find(|(name, _)| name == REGISTRY)
        .unwrap_or_else(|| {
            panic!(
                "{REGISTRY} constructs no covered states at all. Either it is no longer the \
                 registry — in which case point REGISTRY at whatever is — or the scan has stopped \
                 recognising a registration, which would make the one-place check above vacuous."
            )
        });

    for kind in REGISTERED_KINDS {
        assert!(
            registry.1.contains(kind),
            "{REGISTRY} no longer constructs any {kind}. If that kind is gone, strike it from \
             REGISTERED_KINDS; if it moved, the one-place check above is passing because it can no \
             longer see it. Kinds found: {:?}",
            registry.1,
        );
    }
}

/// The scan distinguishes constructing a state from defining or naming its type.
///
/// Asserted directly rather than trusted, because this is the one piece of judgement in the file:
/// `support/layout.rs` both defines the types and mentions them in signatures, and if `constructs`
/// counted those, the check would report the definition site as a second registry and there would
/// be no way to satisfy it.
#[test]
fn defining_or_naming_the_type_is_not_registering_a_state() {
    assert!(constructs("        CoveredState {", "CoveredState"));
    assert!(constructs(
        "    &[CoveredState { name: \"x\" }]",
        "CoveredState"
    ));

    assert!(!constructs("pub struct CoveredState {", "CoveredState"));
    assert!(!constructs("impl Debug for CoveredState {", "CoveredState"));
    assert!(!constructs("    covered: &CoveredState,", "CoveredState"));
    assert!(!constructs(
        "    states: &'static [CoveredState],",
        "CoveredState"
    ));
    assert!(!constructs("struct RevealingState {", "RevealingState"));

    // A different type whose name merely ends in one of these is not a registration.
    assert!(!constructs("    let x = MyCoveredState {", "CoveredState"));
}
