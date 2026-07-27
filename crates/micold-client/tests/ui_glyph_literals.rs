//! Source-level guard: no UI surface may hardcode an icon-like glyph as a character literal
//! (BUG-004; feature 010 FR-016e / SC-018; feature 004 FR-002, SC-005).
//!
//! `tests/icons_font.rs` proves every `Icon::ALL` codepoint resolves to a real glyph in the
//! shipped font — but it can only see callers that go *through* the enum. A surface that skips
//! `Icon` and writes the codepoint inline escapes that guard entirely, and a plain `text(..)`
//! draws in `iced::Font::DEFAULT` (Fira Sans), not the Material Symbols font. That is exactly how
//! the activity badge shipped `"\u{25CF}"`/`"\u{25CB}"`: neither Fira Sans nor
//! `MaterialSymbolsOutlined.ttf` maps those codepoints, so it rendered as a blank box ("tofu")
//! beside every session name.
//!
//! This test states the invariant the icon vocabulary previously only *documented*: every glyph
//! the UI draws comes from `Icon`, so it is font-checked at build time.
//!
//! **Scope.** Only characters in the symbol/pictograph blocks below — the ranges a *text* font is
//! not expected to cover — and only inside string/char literals in `src/ui/`. Ordinary non-ASCII
//! prose (em dashes, ellipses, accented letters) is deliberately untouched: the concern is glyphs
//! drawn as icons, not non-ASCII text as such. Comments are skipped, so the commentary explaining
//! a glyph is free to name it.
//!
//! Pure file inspection — no iced, runs under `cargo test --no-default-features`.

use std::path::{Path, PathBuf};

/// Unicode blocks whose members are icon-like: a text font may map none of them, and anything the
/// app draws from here belongs in the `Icon` vocabulary backed by the Material Symbols font.
const GLYPH_BLOCKS: &[(u32, u32, &str)] = &[
    (0x2190, 0x21FF, "Arrows"),
    (0x2300, 0x23FF, "Miscellaneous Technical"),
    (0x2500, 0x257F, "Box Drawing"),
    (0x2580, 0x259F, "Block Elements"),
    (0x25A0, 0x25FF, "Geometric Shapes"),
    (0x2600, 0x26FF, "Miscellaneous Symbols"),
    (0x2700, 0x27BF, "Dingbats"),
    (0x2800, 0x28FF, "Braille Patterns"),
    (0xE000, 0xF8FF, "Private Use Area"),
    (0x1F300, 0x1FAFF, "Miscellaneous Symbols and Pictographs"),
];

fn glyph_block(c: char) -> Option<&'static str> {
    let cp = c as u32;
    GLYPH_BLOCKS
        .iter()
        .find(|&&(lo, hi, _)| cp >= lo && cp <= hi)
        .map(|&(_, _, name)| name)
}

/// Every `.rs` file under `src/ui/`, recursively.
fn ui_sources() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("src/ui must be readable") {
            let path = entry.expect("dir entry must be readable").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    let mut out = Vec::new();
    walk(&root, &mut out);
    out.sort();
    out
}

/// One literal found in a source file: its decoded text and the 1-based line it starts on.
struct Literal {
    text: String,
    line: usize,
}

/// Extract every string and char literal from Rust source, skipping comments (so prose *about* a
/// glyph is free to name it) and decoding `\u{..}` escapes (so an escaped codepoint is caught just
/// like a pasted character). Raw strings (`r"..."`) are handled; nesting-aware `r#"..."#` is not
/// needed here and is treated as a plain string, which is conservative.
fn literals(src: &str) -> Vec<Literal> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;

    while i < chars.len() {
        let c = chars[i];
        match c {
            '\n' => {
                line += 1;
                i += 1;
            }
            // Line comment.
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            // Block comment (nesting-aware, as Rust's are).
            '/' if chars.get(i + 1) == Some(&'*') => {
                let mut depth = 1;
                i += 2;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '\n' {
                        line += 1;
                    }
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
            }
            // String literal, possibly raw (`r"..."`).
            '"' | 'r' if c == '"' || chars.get(i + 1) == Some(&'"') => {
                let raw = c == 'r';
                let start_line = line;
                if raw {
                    i += 1;
                }
                i += 1; // past the opening quote
                let mut lit = String::new();
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    if !raw && chars[i] == '\\' {
                        // Decode `\u{..}`; skip any other escape's payload.
                        if chars.get(i + 1) == Some(&'u') && chars.get(i + 2) == Some(&'{') {
                            let mut hex = String::new();
                            let mut j = i + 3;
                            while j < chars.len() && chars[j] != '}' {
                                hex.push(chars[j]);
                                j += 1;
                            }
                            if let Some(decoded) =
                                u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)
                            {
                                lit.push(decoded);
                            }
                            i = j + 1;
                        } else {
                            i += 2;
                        }
                        continue;
                    }
                    lit.push(chars[i]);
                    i += 1;
                }
                i += 1; // past the closing quote
                out.push(Literal {
                    text: lit,
                    line: start_line,
                });
            }
            // Char literal — distinguished from a lifetime (`'a`) by the closing quote.
            '\'' => {
                let mut j = i + 1;
                let mut lit = String::new();
                if chars.get(j) == Some(&'\\') {
                    if chars.get(j + 1) == Some(&'u') && chars.get(j + 2) == Some(&'{') {
                        let mut hex = String::new();
                        let mut k = j + 3;
                        while k < chars.len() && chars[k] != '}' {
                            hex.push(chars[k]);
                            k += 1;
                        }
                        if let Some(decoded) =
                            u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)
                        {
                            lit.push(decoded);
                        }
                        j = k + 1;
                    } else {
                        j += 2;
                    }
                } else if j < chars.len() {
                    lit.push(chars[j]);
                    j += 1;
                }
                if chars.get(j) == Some(&'\'') {
                    out.push(Literal { text: lit, line });
                    i = j + 1;
                } else {
                    i += 1; // a lifetime, not a literal
                }
            }
            _ => i += 1,
        }
    }
    out
}

#[test]
fn no_ui_surface_hardcodes_an_icon_glyph() {
    let mut offenders = Vec::new();

    for path in ui_sources() {
        let src = std::fs::read_to_string(&path).expect("source must be valid UTF-8");
        let rel = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(&path)
            .display()
            .to_string();

        for lit in literals(&src) {
            for c in lit.text.chars() {
                if let Some(block) = glyph_block(c) {
                    offenders.push(format!("{rel}:{} — U+{:04X} ({block})", lit.line, c as u32));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "a UI surface hardcodes an icon-like glyph instead of using the shared `Icon` vocabulary. \
         Such a glyph bypasses the build-time missing-glyph guard (tests/icons_font.rs) and is \
         drawn in the default text font rather than the icon font — this is how BUG-004 shipped \
         tofu:\n  {}\n\n\
         Fix: add the glyph to `Icon` (src/icons.rs), pin its codepoint in tests/icons.rs and \
         assets/fonts/PROVENANCE.md, and render it with `crate::ui::icon(..)`.",
        offenders.join("\n  ")
    );
}

/// The guard is only meaningful if it actually inspects sources and can actually detect the thing
/// it claims to — a refactor that moved `src/ui/` or broke the scanner would otherwise leave it
/// vacuously green.
#[test]
fn the_guard_actually_works() {
    let sources = ui_sources();
    assert!(
        sources.len() > 10,
        "expected the UI module tree under src/ui/, found {} file(s) — the guard would be vacuous",
        sources.len()
    );
    assert!(
        sources.iter().any(|p| p.ends_with("activity_badge.rs")),
        "activity_badge.rs must be in scope — it is the surface BUG-004 was found in"
    );

    // The exact shape BUG-004 shipped: an escaped Geometric Shapes codepoint in a string literal.
    let found = literals(r#"let dot = "\u{25CF}";"#);
    assert_eq!(found.len(), 1, "the scanner must find the literal");
    assert_eq!(
        found[0].text.chars().next().and_then(glyph_block),
        Some("Geometric Shapes"),
        "the guard must flag the codepoint BUG-004 shipped"
    );

    // Ordinary prose must NOT trip it — otherwise the guard gets muted rather than obeyed.
    for prose in [
        r#"let s = "Nothing on disk is deleted — the folder is left untouched.";"#,
        r#"let s = "Loading…";"#,
        r#"let s = "café";"#,
    ] {
        for lit in literals(prose) {
            assert!(
                lit.text.chars().all(|c| glyph_block(c).is_none()),
                "prose must not be flagged: {:?}",
                lit.text
            );
        }
    }

    // Comments are skipped, so commentary may name a glyph.
    let commented = literals("// the `●` dot (U+25CF)\nlet x = 1;");
    assert!(
        commented
            .iter()
            .all(|l| l.text.chars().all(|c| glyph_block(c).is_none())),
        "comments must be skipped"
    );
}
