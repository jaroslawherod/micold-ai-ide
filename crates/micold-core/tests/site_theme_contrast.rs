//! WCAG 2.2 AA over the *emitted* theme, in both schemes (feature 028 — FR-033, SC-013).
//!
//! `tokens_contrast.rs` proves the pairs the **application** draws. This proves the pairs the
//! **published site** draws, and it is a separate obligation for two reasons.
//!
//! The site pairs roles the application never pairs: prose set in `on_surface` over a code block at
//! `surface_container_low`, a link in `primary` over a callout at `surface_container`. A palette can
//! be perfectly sound for the application and still fail on a page.
//!
//! And the site's failure mode is worse. A contrast fault in the application is visible to whoever
//! is running it; a contrast fault on the site ships to everyone who reads the documentation and is
//! discovered by the reader who cannot read it. Measuring here means the failure lands in the change
//! that moved the token — in the ordinary suite, on any machine, with no site build at all — rather
//! than in the publication that would have carried it out of the door.
//!
//! Deliberately **not** asserted: `outline_variant`. It is the divider role, it is decorative by
//! construction, and WCAG exempts decoration. `tokens_contrast.rs` declines to assert it for the
//! same reason; asserting it here would be a stricter rule for the documentation than for the
//! product it documents.

use micold_core::tokens::css;
use micold_core::tokens::{contrast, Rgb, AA_NON_TEXT, AA_TEXT};
use std::collections::BTreeMap;

/// The surfaces the site puts content on: the page, code blocks, callouts and the table of
/// contents, and the app bar once content scrolls under it.
const BACKGROUNDS: [&str; 4] = [
    "surface",
    "surface-container-low",
    "surface-container",
    "surface-container-high",
];

/// Foreground, threshold, and what draws it — the "what" is what a failure message needs to be
/// actionable, since `--micold-secondary` failing means "the focus ring is invisible", not "a
/// colour is wrong".
const FOREGROUNDS: [(&str, f64, &str); 5] = [
    ("on-surface", AA_TEXT, "body text and headings"),
    ("on-surface-variant", AA_TEXT, "captions and secondary text"),
    ("primary", AA_TEXT, "links"),
    ("outline", AA_NON_TEXT, "borders of outlined controls"),
    ("secondary", AA_NON_TEXT, "the focus indicator"),
];

/// The declarations of one scheme's block, as emitted.
fn scheme(name: &str) -> BTreeMap<String, Rgb> {
    let sheet = css::stylesheet();
    let opener = match name {
        "light" => ":root {",
        "dark" => ":root[data-scheme=\"dark\"] {",
        other => panic!("no such scheme: {other}"),
    };
    let start = sheet
        .find(opener)
        .unwrap_or_else(|| panic!("the emitted sheet has no `{opener}` block"))
        + opener.len();
    let end = start + sheet[start..].find('}').expect("the block is never closed");
    let mut out = BTreeMap::new();
    for line in sheet[start..end].lines() {
        let Some((key, value)) = line.trim().split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim().trim_end_matches(';').trim());
        let Some(token) = key.strip_prefix("--micold-") else {
            continue;
        };
        let Some(hex) = value.strip_prefix('#') else {
            continue;
        };
        if hex.len() == 6 {
            if let Ok(rgb) = u32::from_str_radix(hex, 16) {
                out.insert(token.to_string(), Rgb::hex(rgb));
            }
        }
    }
    out
}

fn color(declarations: &BTreeMap<String, Rgb>, token: &str, scheme_name: &str) -> Rgb {
    *declarations.get(token).unwrap_or_else(|| {
        panic!("--micold-{token} is not emitted in the {scheme_name} scheme, so the site cannot use it")
    })
}

#[test]
fn every_pair_the_site_draws_meets_wcag_aa_in_both_schemes() {
    let mut failures = Vec::new();
    for scheme_name in ["light", "dark"] {
        let declarations = scheme(scheme_name);
        for background in BACKGROUNDS {
            let bg = color(&declarations, background, scheme_name);
            for (foreground, threshold, drawn_by) in FOREGROUNDS {
                let fg = color(&declarations, foreground, scheme_name);
                let ratio = contrast(fg, bg);
                if ratio < threshold {
                    failures.push(format!(
                        "{scheme_name}: --micold-{foreground} on --micold-{background} is {ratio:.2}:1, \
                         below {threshold}:1 ({drawn_by})"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The scrim dims the page behind a modal surface. The site has no modal surfaces, but it does dim
/// the page behind its mobile navigation drawer, and a scrim heavy enough to hide the content it
/// covers is a different bug from one too light to read through.
#[test]
fn the_two_schemes_are_not_the_same_palette() {
    let light = scheme("light");
    let dark = scheme("dark");
    assert_ne!(
        color(&light, "surface", "light"),
        color(&dark, "surface", "dark"),
        "both blocks carry the same surface: the dark scheme is not being emitted"
    );
}
