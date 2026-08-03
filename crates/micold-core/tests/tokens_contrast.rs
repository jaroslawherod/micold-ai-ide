//! Colour invariants for the Material 3 role set (feature 018, T000a/T000b — FR-004, FR-005, SC-001).
//!
//! Two properties, both of which have to hold for the palette to be trustworthy, and neither of
//! which a person can check by looking.
//!
//! **Every pair that carries text meets WCAG AA.** `contracts/design-tokens.md` §1.3 is the
//! normative list and this is its executable form. Under the tonal system contrast is *structural*
//! — a role at tone 40 paired with one at tone 100 clears AA because of the tone delta, not because
//! someone checked — so a failure here means a role was given the wrong tone, which is exactly the
//! error the system is meant to make impossible.
//!
//! **Every ramp rises monotonically in luminance.** The ramps are checked-in data (§1.1: no
//! build-time generation), so a transcription slip is possible in principle. Such a slip usually
//! produces a *subtly* wrong colour that no contrast assertion would catch — tone 50 a shade darker
//! than tone 40 still clears AA against white. Monotonicity is the property that catches it
//! (plan.md risk 1, research R7).

use micold_core::naming::ConventionalType;
use micold_core::theme::ColorScheme;
use micold_core::tokens::palette::{self, Ramp};
use micold_core::tokens::{roles, Rgb, Roles};

// ---------------------------------------------------------------------------------------------
// WCAG contrast
// ---------------------------------------------------------------------------------------------

fn linearize(channel: u8) -> f64 {
    let c = channel as f64 / 255.0;
    if c <= 0.039_28 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(color: Rgb) -> f64 {
    0.2126 * linearize(color.r) + 0.7152 * linearize(color.g) + 0.0722 * linearize(color.b)
}

fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Normal text.
const AA_NORMAL: f64 = 4.5;
/// Non-text — what a divider or an outlined control's border must clear.
const AA_NON_TEXT: f64 = 3.0;

/// Every surface level `on_surface` and `on_surface_variant` are drawn over (§1.3).
fn all_surfaces(r: &Roles) -> Vec<(&'static str, Rgb)> {
    vec![
        ("surface", r.surface),
        ("surface_dim", r.surface_dim),
        ("surface_bright", r.surface_bright),
        ("surface_container_lowest", r.surface_container_lowest),
        ("surface_container_low", r.surface_container_low),
        ("surface_container", r.surface_container),
        ("surface_container_high", r.surface_container_high),
        ("surface_container_highest", r.surface_container_highest),
    ]
}

/// §1.3's table, as `(label, foreground, background)`.
fn text_pairs(r: &Roles) -> Vec<(String, Rgb, Rgb)> {
    let mut out: Vec<(String, Rgb, Rgb)> = vec![
        (
            "on_background/background".into(),
            r.on_background,
            r.background,
        ),
        ("on_primary/primary".into(), r.on_primary, r.primary),
        (
            "on_primary_container/primary_container".into(),
            r.on_primary_container,
            r.primary_container,
        ),
        ("on_secondary/secondary".into(), r.on_secondary, r.secondary),
        (
            "on_secondary_container/secondary_container".into(),
            r.on_secondary_container,
            r.secondary_container,
        ),
        ("on_tertiary/tertiary".into(), r.on_tertiary, r.tertiary),
        (
            "on_tertiary_container/tertiary_container".into(),
            r.on_tertiary_container,
            r.tertiary_container,
        ),
        ("on_error/error".into(), r.on_error, r.error),
        (
            "on_error_container/error_container".into(),
            r.on_error_container,
            r.error_container,
        ),
        (
            "inverse_on_surface/inverse_surface".into(),
            r.inverse_on_surface,
            r.inverse_surface,
        ),
        (
            "inverse_primary/inverse_surface".into(),
            r.inverse_primary,
            r.inverse_surface,
        ),
        (
            "on_surface_variant/surface_variant".into(),
            r.on_surface_variant,
            r.surface_variant,
        ),
    ];

    // `on_surface` and `on_surface_variant` are drawn on every container level, not just `surface`.
    for (name, bg) in all_surfaces(r) {
        out.push((format!("on_surface/{name}"), r.on_surface, bg));
        out.push((
            format!("on_surface_variant/{name}"),
            r.on_surface_variant,
            bg,
        ));
    }

    // Text buttons, links and error helper text draw an accent role directly on a surface.
    for (name, bg) in [
        ("surface", r.surface),
        ("surface_container_low", r.surface_container_low),
        ("surface_container", r.surface_container),
    ] {
        out.push((format!("primary/{name}"), r.primary, bg));
    }
    for (name, bg) in [
        ("surface", r.surface),
        ("surface_container", r.surface_container),
    ] {
        out.push((format!("error/{name}"), r.error, bg));
    }

    // All eleven tags: each tag's own text tone on its own fill tone (§1.4).
    for &t in ConventionalType::ALL {
        let (fill, text) = r.tag(t);
        out.push((format!("tag_{}", t.as_str()), text, fill));
    }
    let (issue_fill, issue_text) = r.issue_tag();
    out.push(("tag_issue".into(), issue_text, issue_fill));

    out
}

fn assert_aa(scheme: ColorScheme) {
    let r = roles(scheme);
    let mut failures = Vec::new();
    for (name, fg, bg) in text_pairs(&r) {
        let ratio = contrast(fg, bg);
        if ratio < AA_NORMAL {
            failures.push(format!(
                "  {name}: {ratio:.2} < {AA_NORMAL} (fg {fg:?} on bg {bg:?})"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{scheme:?} scheme has {} pair(s) below WCAG AA.\n{}\n\nUnder the tonal system contrast \
         follows from the tone delta (§1.1), so this means a role was given the wrong tone rather \
         than that a colour needs hand-tuning.",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn light_scheme_meets_aa_on_every_text_pair() {
    assert_aa(ColorScheme::Light);
}

#[test]
fn dark_scheme_meets_aa_on_every_text_pair() {
    assert_aa(ColorScheme::Dark);
}

/// `outline` carries no text, but it separates and bounds, so it owes the non-text 3:1 (§1.3).
#[test]
fn outline_clears_the_non_text_threshold_in_both_schemes() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        for (name, bg) in all_surfaces(&r) {
            let ratio = contrast(r.outline, bg);
            assert!(
                ratio >= AA_NON_TEXT,
                "{scheme:?} outline/{name}: {ratio:.2} < {AA_NON_TEXT}"
            );
        }
    }
}

/// A gate that checks nothing passes trivially. If the pair table is ever gutted, this fails rather
/// than reporting a clean bill of health over an empty set.
#[test]
fn the_pair_table_actually_covers_the_role_set() {
    let r = roles(ColorScheme::Light);
    let pairs = text_pairs(&r);
    assert!(
        pairs.len() >= 40,
        "only {} pairs checked — §1.3 lists far more than that",
        pairs.len()
    );
}

// ---------------------------------------------------------------------------------------------
// Ramp monotonicity
// ---------------------------------------------------------------------------------------------

fn every_ramp() -> Vec<(&'static str, &'static Ramp)> {
    vec![
        ("primary", &palette::PRIMARY),
        ("secondary", &palette::SECONDARY),
        ("tertiary", &palette::TERTIARY),
        ("error", &palette::ERROR),
        ("neutral", &palette::NEUTRAL),
        ("neutral_variant", &palette::NEUTRAL_VARIANT),
    ]
}

/// Tone is perceptual lightness, so a ramp must get strictly lighter as tone rises. A transcription
/// slip that swaps two stops, or mistypes one digit, shows up here and nowhere else — the wrong
/// colour would still clear AA against the surfaces it is used on.
#[test]
fn every_ramp_rises_monotonically_in_luminance() {
    for (name, ramp) in every_ramp() {
        let mut previous: Option<(u8, f64)> = None;
        for &tone in palette::TONES.iter() {
            let l = luminance(ramp.at(tone));
            if let Some((prev_tone, prev_l)) = previous {
                assert!(
                    l > prev_l,
                    "{name} ramp is not monotonic: tone {tone} (luminance {l:.5}) is not lighter \
                     than tone {prev_tone} (luminance {prev_l:.5})"
                );
            }
            previous = Some((tone, l));
        }
    }
}

/// The ends are fixed by definition: tone 0 is black and tone 100 is white (§1.1).
#[test]
fn every_ramp_runs_from_black_to_white() {
    for (name, ramp) in every_ramp() {
        assert_eq!(
            ramp.at(0),
            Rgb::hex(0x000000),
            "{name} tone 0 must be black"
        );
        assert_eq!(
            ramp.at(100),
            Rgb::hex(0xFFFFFF),
            "{name} tone 100 must be white"
        );
    }
}

/// The primary ramp is generated *from* the seed, so tone 40 must be the seed itself. This is the
/// single strongest check that the ramps belong to the scheme they claim to: it fails if the seed
/// changed, if the generator drifted, or if the wrong palette was pasted in.
#[test]
fn the_primary_ramp_reproduces_the_seed_at_tone_40() {
    assert_eq!(
        palette::PRIMARY.at(40),
        Rgb::hex(0x6750A4),
        "tone 40 of the primary ramp must be the Material 3 baseline seed #6750A4"
    );
}

/// Values taken from material-color-utilities' own tests rather than from this project's
/// generator, so the ramps are anchored to the reference the contract claims they can be checked
/// against (§1.1), not merely to themselves.
#[test]
fn the_error_ramp_matches_the_published_baseline() {
    for (tone, expected) in [
        (30u8, 0x93000A),
        (40, 0xBA1A1A),
        (80, 0xFFB4AB),
        (90, 0xFFDAD6),
    ] {
        assert_eq!(
            palette::ERROR.at(tone),
            Rgb::hex(expected),
            "error tone {tone} must match the published Material baseline"
        );
    }
}

/// Every tone stop the contract names is present, and `at` resolves each one.
#[test]
fn the_ramp_carries_every_tone_stop_the_contract_names() {
    assert_eq!(
        palette::TONES.as_slice(),
        &[
            0u8, 4, 6, 10, 12, 17, 20, 22, 24, 30, 40, 50, 60, 70, 80, 87, 90, 92, 94, 95, 96, 98,
            100
        ],
        "the tone stops are normative (§1.1)"
    );
}
