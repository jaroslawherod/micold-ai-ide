//! Every animation is timed and eased by name (feature 018, T063 — FR-033, FR-034, FR-035).
//!
//! Feature 017 gave the application one animation *primitive*. It did not give it one set of
//! *timings*: `90`, `114`, `200`, `250`, `800` — five numbers picked by whoever wrote each wrapper,
//! none of them from a scale, and no two of the three fade-like transitions agreeing.
//!
//! A duration written as a literal is invisible to every other check here. It does not follow when
//! the scale is re-valued, nothing relates it to the transition beside it, and — the part that
//! matters — nothing about `90` looks wrong. It only becomes wrong next to `100`, in a different
//! file, months later.
//!
//! # What counts
//!
//! A `Duration::from_millis(..)` given a bare number, anywhere in the rendering layer. Named tokens
//! pass; `duration::SHORT_2` is the destination, not a violation.
//!
//! Test scaffolding is exempt by path, not by shape: `ripple_clipping.rs` drives the primitive with
//! a made-up 200ms because it is asserting *drawing*, and holding a test's stand-in clock to the
//! design system would be pedantry rather than a check.

use std::fs;
use std::path::Path;

/// Everything that renders, and is therefore animated by the design system's clock.
fn rendering_sources() -> Vec<(String, String)> {
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
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for dir in [src.join("ui"), src.join("showcase")] {
        walk(&dir, &mut out);
    }
    out.sort();
    out
}

/// In-crate test modules that drive the primitive with a stand-in clock.
///
/// Exempt because what they assert is *drawing*, not timing: the number they pass is a fixture, and
/// the design system has no opinion about a fixture. Listed by path so the exemption cannot widen
/// to a rendering file by accident.
const CLOCK_FIXTURES: &[&str] = &["ui/material/ripple_clipping.rs"];

/// The behaviour layer, which holds the *frame period* and no design-system timing at all.
///
/// `cdk` is forbidden from naming a token — `tests/cdk_no_appearance.rs` fails the build if it
/// names `motion::` — so every transition duration reaches it as a parameter. The one constant it
/// owns is how long a frame is assumed to last, which is a property of the display rather than of
/// the design system, and there is no token for it because there should not be one.
const FRAME_PERIOD: &[&str] = &["ui/cdk/motion.rs"];

/// The showcase deliberately slows every demonstration down, and says so.
///
/// A 90ms transition is correct in the application and useless in a gallery — you cannot review
/// what you cannot see. That is a *presentation* choice about the gallery, not a timing the
/// application ships, so `sections/motion.rs` states its own duration.
const GALLERY_PACING: &[&str] = &["showcase/sections/motion.rs"];

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

/// The argument of each `from_millis(` on a line, up to the matching paren.
fn millis_args(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(i) = rest.find("from_millis(") {
        let after = &rest[i + "from_millis(".len()..];
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

fn is_bare_number(arg: &str) -> bool {
    let a = arg.trim();
    !a.is_empty() && a.chars().all(|c| c.is_ascii_digit() || c == '_')
}

#[test]
fn every_duration_is_a_named_token() {
    let mut offenders = Vec::new();
    for (path, src) in rendering_sources() {
        if CLOCK_FIXTURES.contains(&path.as_str())
            || GALLERY_PACING.contains(&path.as_str())
            || FRAME_PERIOD.contains(&path.as_str())
        {
            continue;
        }
        for (i, line) in code_only(&src).lines().enumerate() {
            for arg in millis_args(line) {
                if is_bare_number(arg) {
                    offenders.push(format!("  {path}:{}  from_millis({arg})", i + 1));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "these transitions state a duration instead of naming one:\n{}\n\nA literal does not \
         follow when the scale is re-valued, relates to nothing beside it, and never looks wrong \
         on its own — `90` only becomes wrong next to `100`, in another file, months later. Name a \
         `duration::*` token (contract §6.1, FR-033).",
        offenders.join("\n")
    );
}

/// The scan reads the rendering layer rather than nothing.
#[test]
fn the_scan_actually_reads_the_rendering_layer() {
    let sources = rendering_sources();
    assert!(
        sources.len() > 25,
        "found only {} sources — the check above would be near-vacuous",
        sources.len()
    );
    assert!(
        sources.iter().any(|(_, src)| src.contains("from_millis(")),
        "no source states a duration at all, so the scan is matching nothing"
    );
}

/// The rule fires on a literal and stays quiet on a named token.
#[test]
fn a_literal_is_caught_and_a_token_is_not() {
    assert_eq!(
        millis_args("const F: Duration = Duration::from_millis(90);"),
        vec!["90"]
    );
    assert!(is_bare_number("90"));
    assert!(is_bare_number("1_000"));

    assert!(!is_bare_number("duration::SHORT_2"));
    assert!(!is_bare_number("duration::MEDIUM_2"));
}

/// Every exemption names a file that exists.
///
/// An exemption for a deleted or renamed file is an exemption that has quietly stopped applying to
/// anything — and the next file to take that path inherits it without anyone deciding so.
#[test]
fn every_exemption_is_live() {
    let paths: Vec<String> = rendering_sources().into_iter().map(|(p, _)| p).collect();
    for exempt in CLOCK_FIXTURES
        .iter()
        .chain(GALLERY_PACING)
        .chain(FRAME_PERIOD)
    {
        assert!(
            paths.iter().any(|p| p == exempt),
            "{exempt} is exempted from the duration rule and does not exist — delete the \
             exemption, or point it at wherever the file went"
        );
    }
}

/// Every duration token in the scale is a whole number of milliseconds on the 50ms grid §6.1 uses.
///
/// Cheap, and it catches a transcription slip that would otherwise look plausible: the table is
/// twelve values, every one a multiple of 50, and a `35` or `250` in the wrong row reads as fine.
#[test]
fn the_duration_scale_is_on_its_own_grid() {
    use micold_core::tokens::motion::duration;
    for (name, ms) in [
        ("SHORT_1", duration::SHORT_1),
        ("SHORT_2", duration::SHORT_2),
        ("SHORT_3", duration::SHORT_3),
        ("SHORT_4", duration::SHORT_4),
        ("MEDIUM_1", duration::MEDIUM_1),
        ("MEDIUM_2", duration::MEDIUM_2),
        ("MEDIUM_3", duration::MEDIUM_3),
        ("MEDIUM_4", duration::MEDIUM_4),
        ("LONG_1", duration::LONG_1),
        ("LONG_2", duration::LONG_2),
        ("LONG_3", duration::LONG_3),
        ("LONG_4", duration::LONG_4),
    ] {
        assert_eq!(ms % 50, 0, "{name} is {ms}ms, off §6.1's 50ms grid");
        assert!(ms > 0, "{name} is not a duration");
    }
}
