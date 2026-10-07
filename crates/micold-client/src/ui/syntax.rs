//! Syntax colours for the Changes view's diff (feature 482, R10, FR-007, US1 s8).
//!
//! Each version of a file is highlighted with `iced::highlighter` by its extension —
//! `InspiredGitHub` for the light scheme, `Base16Ocean` for the dark — off the update thread, and
//! the colours are kept as [`Spans`] beside the diff, so a theme switch needs no new read. A line
//! is cut to [`SPAN_CAP`] bytes before it is highlighted, and its spans go through [`cap_spans`]:
//! an unknown extension highlights as plain text, which has no colour, and so keeps no spans.
//!
//! A theme's own colours are not all legible on this app's surfaces (a comment grey falls below
//! 4.5:1), so every colour is moved toward the scheme's `on_surface` by [`legible`] until it reads
//! at 4.5:1 on the surface and on both diff tints.

use iced::highlighter::{Settings, Stream, Theme};
use micold_core::review::diff::{FileDiff, LoadedDiff, SchemeSpans, SideLines, SideSpans, Spans};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, contrast, Rgb, Roles, AA_TEXT};

use crate::features::changes::{cap_spans, SPAN_CAP};

/// The highlighter theme of `scheme`.
pub fn theme(scheme: ColorScheme) -> Theme {
    match scheme {
        ColorScheme::Light => Theme::InspiredGitHub,
        ColorScheme::Dark => Theme::Base16Ocean,
    }
}

/// The fills a syntax colour is read on: the diff pane's surface and the two line tints.
pub fn fills(roles: Roles) -> [Rgb; 3] {
    [roles.surface, roles.diff_added, roles.diff_removed]
}

/// `colour`, moved toward `roles.on_surface` just far enough to read at 4.5:1 on every fill a diff
/// line is drawn on; unchanged when it already does.
pub fn legible(colour: Rgb, roles: Roles) -> Rgb {
    let reads = |c: Rgb| {
        fills(roles)
            .iter()
            .all(|&fill| contrast(c, fill) >= AA_TEXT)
    };
    // `on_surface` itself reads on all three (`diff_line_text_is_legible_on_the_added_and_removed_tints`),
    // so the walk ends there at the latest.
    (0..=STEPS)
        .map(|step| mix(colour, roles.on_surface, step as f32 / STEPS as f32))
        .find(|&c| reads(c))
        .unwrap_or(roles.on_surface)
}

/// How finely [`legible`] walks from a colour to `on_surface`.
const STEPS: u16 = 40;

/// `from` moved `t` of the way to `to`, per sRGB channel.
fn mix(from: Rgb, to: Rgb, t: f32) -> Rgb {
    let channel = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
    Rgb {
        r: channel(from.r, to.r),
        g: channel(from.g, to.g),
        b: channel(from.b, to.b),
    }
}

/// An `iced` colour as the token type: both are 8-bit sRGB.
fn rgb(c: iced::Color) -> Rgb {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgb {
        r: q(c.r),
        g: q(c.g),
        b: q(c.b),
    }
}

/// The first [`SPAN_CAP`] bytes of `line`, cut back to a character boundary.
fn capped(line: &str) -> &str {
    let mut end = line.len().min(SPAN_CAP);
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    &line[..end]
}

/// The grammar token of `path`: its extension, else its file name (`Makefile`, `Dockerfile`).
pub fn token(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => extension.to_owned(),
        _ => name.to_owned(),
    }
}

/// The colours of `lines`, highlighted in order as `token`'s language in `scheme`.
pub fn highlight_lines<'a>(
    token: &str,
    lines: impl IntoIterator<Item = &'a str>,
    scheme: ColorScheme,
) -> SideSpans {
    let roles = tokens::roles(scheme);
    let mut stream = Stream::new(&Settings {
        theme: theme(scheme),
        token: token.to_owned(),
    });
    SideSpans(
        lines
            .into_iter()
            .map(|line| {
                // The grammars match a line with its ending, as the editor feeds them.
                let line = format!("{}\n", capped(line));
                let spans = cap_spans(stream.highlight_line(&line).map(|(range, highlight)| {
                    let end = range.end.min(line.len() - 1);
                    (
                        range.start.min(end)..end,
                        highlight.color().map(|c| legible(rgb(c), roles)),
                    )
                }));
                stream.commit();
                spans
            })
            .collect(),
    )
}

/// Both versions of `loaded`, highlighted in both schemes as `path`'s language. Each version is
/// read only up to the last of its lines the diff shows, so a small change to a long file does not
/// highlight all of it.
pub fn highlight(loaded: &LoadedDiff, path: &str) -> Spans {
    let token = token(path);
    let shown = |old: bool| {
        let FileDiff::Text(hunks) = &loaded.diff else {
            return 0;
        };
        hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter_map(|l| if old { l.old } else { l.new })
            .max()
            .unwrap_or(0)
    };
    let side = |lines: &Option<SideLines>, upto: u32, scheme| match lines {
        Some(lines) => highlight_lines(&token, (1..=upto).map_while(|n| lines.line(n)), scheme),
        None => SideSpans::default(),
    };
    let (old, new) = (shown(true), shown(false));
    let scheme = |scheme| SchemeSpans {
        old: side(&loaded.old, old, scheme),
        new: side(&loaded.new, new, scheme),
    };
    Spans {
        light: scheme(ColorScheme::Light),
        dark: scheme(ColorScheme::Dark),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::review::diff::parse_unified;

    const RUST: [&str; 4] = [
        "// a comment",
        "fn main() {",
        "    let s = \"text\"; // 42",
        "}",
    ];

    #[test]
    fn a_recognised_language_is_coloured_in_both_schemes() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let spans = highlight_lines("rs", RUST, scheme);
            assert_eq!(spans.0.len(), RUST.len(), "one entry per line");
            assert!(
                !spans.line(2).is_empty(),
                "`fn main` is coloured in {scheme:?}"
            );
        }
    }

    #[test]
    fn an_unknown_extension_keeps_no_spans() {
        let spans = highlight_lines("no-such-language", RUST, ColorScheme::Light);
        assert!(spans.0.iter().all(Vec::is_empty), "{spans:?}");
    }

    #[test]
    fn a_long_line_is_coloured_only_up_to_the_cap() {
        let long = format!("let s = \"{}\";", "é".repeat(3 * SPAN_CAP));
        let spans = highlight_lines("rs", [long.as_str()], ColorScheme::Dark);
        let line = spans.line(1);
        assert!(!line.is_empty(), "the start of the line is coloured");
        assert!(line.iter().all(|(r, _)| r.end <= SPAN_CAP), "{line:?}");
    }

    #[test]
    fn the_token_is_the_extension_or_else_the_file_name() {
        assert_eq!(token("crates/a/src/lib.rs"), "rs");
        assert_eq!(token("Makefile"), "Makefile");
        assert_eq!(token("dir/.gitignore"), ".gitignore");
    }

    #[test]
    fn only_the_lines_the_diff_shows_are_highlighted() {
        let raw = "@@ -2,1 +2,1 @@\n-fn a() {}\n+fn b() {}\n";
        let old = SideLines::from_bytes(b"// one\nfn a() {}\nfn c() {}\nfn d() {}\n").unwrap();
        let new = SideLines::from_bytes(b"// one\nfn b() {}\nfn c() {}\nfn d() {}\n").unwrap();
        let loaded = LoadedDiff {
            diff: parse_unified(raw.as_bytes()),
            old: Some(old),
            new: Some(new),
            spans: Spans::default(),
        };
        let spans = highlight(&loaded, "x.rs");
        for side in [
            &spans.light.old,
            &spans.light.new,
            &spans.dark.old,
            &spans.dark.new,
        ] {
            assert_eq!(
                side.0.len(),
                2,
                "lines 1 and 2, not the 4 of the file: {side:?}"
            );
            assert!(!side.line(2).is_empty());
        }
    }

    #[test]
    fn legible_leaves_a_colour_that_already_reads() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            assert_eq!(legible(r.on_surface, r), r.on_surface);
        }
    }

    #[test]
    fn every_fill_is_listed_once() {
        let r = tokens::roles(ColorScheme::Light);
        assert_eq!(fills(r).len(), 3);
        let _ = (
            contrast(r.on_surface, r.surface),
            AA_TEXT,
            SchemeSpans::default(),
        );
    }
}
