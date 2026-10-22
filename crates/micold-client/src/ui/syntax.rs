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

use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use iced::highlighter::{Settings, Stream, Theme};
use micold_core::review::diff::{FileDiff, LoadedDiff, SchemeSpans, SideLines, SideSpans, Spans};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, contrast, Rgb, Roles, AA_TEXT};

use crate::features::changes::{cap_spans, SPAN_CAP};
use crate::features::transcript_rich::Grammar;

/// The highlighter theme of `scheme`.
pub fn theme(scheme: ColorScheme) -> Theme {
    match scheme {
        ColorScheme::Light => Theme::InspiredGitHub,
        ColorScheme::Dark => Theme::Base16Ocean,
    }
}

/// The fills a syntax colour is read on: the diff pane's surface, the two line tints, and the
/// transcript's code surface (feature 940, SC-006).
pub fn fills(roles: Roles) -> [Rgb; 4] {
    [
        roles.surface,
        roles.diff_added,
        roles.diff_removed,
        roles.surface_container_high,
    ]
}

/// `colour`, moved toward `roles.on_surface` just far enough to read at 4.5:1 on every fill a diff
/// line is drawn on; unchanged when it already does.
pub fn legible(colour: Rgb, roles: Roles) -> Rgb {
    let reads = |c: Rgb| {
        fills(roles)
            .iter()
            .all(|&fill| contrast(c, fill) >= AA_TEXT)
    };
    // `on_surface` itself reads on all four (`diff_line_text_is_legible_on_the_added_and_removed_tints`),
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

/// The grammar set the transcript's render cache decides highlight jobs with (feature 940, R4).
pub const GRAMMAR: Grammar = Grammar { knows, token };

/// Whether a grammar other than plain text exists for `token` (feature 940, FR-009): `token`
/// names one of the highlighter's grammars by a file extension or by its name, ignoring case, as
/// the highlighter's own lookup (`find_syntax_by_token`) matches it.
pub fn knows(token: &str) -> bool {
    static KNOWN: LazyLock<HashSet<String>> = LazyLock::new(|| {
        two_face::syntax::extra_no_newlines()
            .syntaxes()
            .iter()
            .filter(|syntax| syntax.name != "Plain Text")
            .flat_map(|syntax| syntax.file_extensions.iter().chain([&syntax.name]))
            .map(|token| token.to_lowercase())
            .collect()
    });
    !token.is_empty() && KNOWN.contains(&token.to_lowercase())
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
    highlight_lines_until(token, lines, scheme, None).unwrap_or_default()
}

/// [`highlight_lines`], giving up (`None`) once `deadline` has passed: checked per line, so one
/// pathological line is the most it overruns by.
fn highlight_lines_until<'a>(
    token: &str,
    lines: impl IntoIterator<Item = &'a str>,
    scheme: ColorScheme,
    deadline: Option<Instant>,
) -> Option<SideSpans> {
    let roles = tokens::roles(scheme);
    let mut stream = Stream::new(&Settings {
        theme: theme(scheme),
        token: token.to_owned(),
    });
    let mut out = Vec::new();
    for line in lines {
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return None;
        }
        // The grammars match a line with its ending, as the editor feeds them.
        let line = format!("{}\n", capped(line));
        out.push(cap_spans(stream.highlight_line(&line).map(
            |(range, highlight)| {
                let end = range.end.min(line.len() - 1);
                (
                    range.start.min(end)..end,
                    highlight.color().map(|c| legible(rgb(c), roles)),
                )
            },
        )));
        stream.commit();
    }
    Some(SideSpans(out))
}

/// A transcript highlight job's `lines` as `token`'s language in both schemes, each line's colours
/// in `new` (feature 940, research R4); `None` once `budget` is spent.
pub fn highlight_job(token: &str, lines: &[String], budget: Duration) -> Option<Spans> {
    let deadline = Some(Instant::now() + budget);
    let scheme = |scheme| {
        Some(SchemeSpans {
            old: SideSpans::default(),
            new: highlight_lines_until(token, lines.iter().map(String::as_str), scheme, deadline)?,
        })
    };
    Some(Spans {
        light: scheme(ColorScheme::Light)?,
        dark: scheme(ColorScheme::Dark)?,
    })
}

/// All of `lines`, highlighted in `scheme` as `path`'s language; `None` once `deadline` has passed.
/// The one path that colours a whole version, so a line revealed from outside the diff's hunks is
/// coloured like its neighbours (feature 1150, FR-002).
pub fn highlight_file(
    lines: &SideLines,
    path: &str,
    scheme: ColorScheme,
    deadline: Option<Instant>,
) -> Option<SideSpans> {
    highlight_lines_until(
        &token(path),
        (1..).map_while(|n| lines.line(n)),
        scheme,
        deadline,
    )
}

/// A whole file, highlighted in both schemes as `path`'s language, its colours in `new`; `None`
/// once `budget` is spent (feature 1150: a browsed file).
pub fn highlight_text(lines: &SideLines, path: &str, budget: Duration) -> Option<Spans> {
    let deadline = Some(Instant::now() + budget);
    let scheme = |scheme| {
        Some(SchemeSpans {
            old: SideSpans::default(),
            new: highlight_file(lines, path, scheme, deadline)?,
        })
    };
    Some(Spans {
        light: scheme(ColorScheme::Light)?,
        dark: scheme(ColorScheme::Dark)?,
    })
}

/// Both versions of `loaded`, highlighted in both schemes as `path`'s language. The old version is
/// read only up to the last of its lines the diff shows; the new one whole, because hidden lines
/// of it can be revealed.
pub fn highlight(loaded: &LoadedDiff, path: &str) -> Spans {
    highlight_within(loaded, path, None).unwrap_or_default()
}

/// [`highlight`] under a time `budget` (#771): `None` once it is spent, and the caller shows the
/// plain diff rather than wait on a pathological grammar match.
pub fn highlight_within(
    loaded: &LoadedDiff,
    path: &str,
    budget: Option<Duration>,
) -> Option<Spans> {
    let deadline = budget.map(|budget| Instant::now() + budget);
    let token = token(path);
    let old_upto = match &loaded.diff {
        FileDiff::Text(hunks) => hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter_map(|l| l.old)
            .max()
            .unwrap_or(0),
        _ => 0,
    };
    let old = |scheme| match &loaded.old {
        Some(lines) => highlight_lines_until(
            &token,
            (1..=old_upto).map_while(|n| lines.line(n)),
            scheme,
            deadline,
        ),
        None => Some(SideSpans::default()),
    };
    let new = |scheme| match &loaded.new {
        Some(lines) if matches!(loaded.diff, FileDiff::Text(_)) => {
            highlight_file(lines, path, scheme, deadline)
        }
        _ => Some(SideSpans::default()),
    };
    let scheme = |scheme| {
        Some(SchemeSpans {
            old: old(scheme)?,
            new: new(scheme)?,
        })
    };
    Some(Spans {
        light: scheme(ColorScheme::Light)?,
        dark: scheme(ColorScheme::Dark)?,
    })
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
    fn the_old_side_stops_at_the_diffs_last_line_and_the_new_side_is_whole() {
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
        for side in [&spans.light.old, &spans.dark.old] {
            assert_eq!(side.0.len(), 2, "old: lines 1 and 2 only: {side:?}");
            assert!(!side.line(2).is_empty());
        }
        for side in [&spans.light.new, &spans.dark.new] {
            assert_eq!(side.0.len(), 4, "new: every line, to colour revealed ones");
        }
    }

    #[test]
    fn a_spent_budget_gives_up_instead_of_colouring() {
        let old = SideLines::from_bytes(
            b"fn a() {}
",
        )
        .unwrap();
        let new = SideLines::from_bytes(
            b"fn b() {}
",
        )
        .unwrap();
        let loaded = LoadedDiff {
            diff: parse_unified(b"@@ -1,1 +1,1 @@\n-fn a() {}\n+fn b() {}\n"),
            old: Some(old),
            new: Some(new),
            spans: Spans::default(),
        };
        assert!(highlight_within(&loaded, "x.rs", Some(Duration::ZERO)).is_none());
        let spans = highlight_within(&loaded, "x.rs", Some(Duration::from_secs(60)))
            .expect("a roomy budget colours");
        assert!(!spans.dark.new.line(1).is_empty());
    }

    /// FR-004: the whole-file colouring gives up under a spent budget, and colours every line of a
    /// long file under a roomy one.
    #[test]
    fn highlight_file_runs_under_its_budget() {
        let text: String = (1..=500).map(|n| format!("let v{n} = {n};\n")).collect();
        let lines = SideLines::from_bytes(text.as_bytes()).unwrap();
        let spent = Some(Instant::now());
        assert!(highlight_file(&lines, "x.rs", ColorScheme::Dark, spent).is_none());
        let roomy = Some(Instant::now() + Duration::from_secs(60));
        let spans = highlight_file(&lines, "x.rs", ColorScheme::Dark, roomy).expect("colours");
        assert!(!spans.line(500).is_empty());
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
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            let listed = fills(r);
            assert_eq!(
                listed,
                [
                    r.surface,
                    r.diff_added,
                    r.diff_removed,
                    r.surface_container_high
                ],
                "a diff line is drawn on the surface or an added or removed tint, and a \
                 transcript code block on the code surface (feature 940, SC-006) ({scheme:?})"
            );
            for (i, a) in listed.iter().enumerate() {
                for b in &listed[i + 1..] {
                    assert_ne!(
                        a, b,
                        "each fill is a distinct colour ({scheme:?}): {listed:?}"
                    );
                }
            }
        }
    }

    /// Feature 940 T016 (FR-009): a fence's token is known when a grammar other than plain text
    /// exists for it; unknown tokens and no token draw plain monospace.
    #[test]
    fn knows_the_tokens_a_grammar_exists_for() {
        for token in ["rust", "sh", "yaml", "json", "py"] {
            assert!(knows(token), "{token} has a grammar");
        }
        for token in ["teapot", "no-such-language", ""] {
            assert!(!knows(token), "{token:?} has none");
        }
    }

    /// Feature 940 T016 (SC-006): a colour `legible` returns reads on the code surface too.
    #[test]
    fn legible_lifts_colours_on_the_code_surface() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            assert!(fills(r).contains(&r.surface_container_high));
            for raw in [
                Rgb {
                    r: 128,
                    g: 128,
                    b: 128,
                },
                r.surface_container_high,
            ] {
                let lifted = legible(raw, r);
                assert!(contrast(lifted, r.surface_container_high) >= AA_TEXT);
            }
        }
    }
}
