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
//! **Every pair still meets AA with its state layer composited.** §1.3 proves pairs and §5 proves
//! opacities, and until BUG-010 nothing multiplied them together. A state layer is the content
//! colour drawn over the container, so it always pulls the background toward the foreground: a pair
//! proved at rest is not proved. Every pair below is therefore measured twice — at rest, and with
//! the heaviest layer the element drawing it can carry (FR-004b, SC-008h).
//!
//! **Every ramp rises monotonically in luminance.** The ramps are checked-in data (§1.1: no
//! build-time generation), so a transcription slip is possible in principle. Such a slip usually
//! produces a *subtly* wrong colour that no contrast assertion would catch — tone 50 a shade darker
//! than tone 40 still clears AA against white. Monotonicity is the property that catches it
//! (plan.md risk 1, research R7).

use micold_core::naming::ConventionalType;
use micold_core::theme::ColorScheme;
use micold_core::tokens::palette::{self, Ramp};
use micold_core::tokens::{contrast, luminance, over, roles, Rgb, Roles, AA_NON_TEXT, AA_TEXT};

// ---------------------------------------------------------------------------------------------
// WCAG contrast
// ---------------------------------------------------------------------------------------------
//
// `contrast`, `luminance` and the two thresholds live in `micold_core::tokens` rather than here.
// This file had its own copy, and so did `tokens.rs`, `icon_roles.rs` and — once BUG-009 added a
// composition gate — a fourth. Four transcriptions of the sRGB linearisation is four chances for
// one of them to drift, and a drifted copy does not fail: it quietly measures something else and
// still passes. One definition, in the crate that owns the colours (FR-029a's rule, in maths).

/// Normal text. Named `AA_NORMAL` here because the assertions below read that way; it is
/// `tokens::AA_TEXT`.
const AA_NORMAL: f64 = AA_TEXT;

/// The heaviest state layer an element of each class can carry (§5, FR-004b).
///
/// §5's states are mutually exclusive — `Layer` is an ordered enum, not a set of flags — so layers
/// never sum and each of these is a maximum rather than a running total.
mod heaviest {
    use micold_core::tokens::state;

    /// A button: `pressed`, with `focus` at the same figure.
    pub const BUTTON: f64 = state::PRESSED as f64;
    /// A row, menu item, chip or tag: `selected`, the heaviest §5 names, and the only one that
    /// persists after the pointer leaves.
    pub const ROW: f64 = state::SELECTED as f64;
    /// Static prose — a window background, a snackbar's message, helper text under a field. Nothing
    /// draws a layer over these, and pretending otherwise would forbid colours nobody can press.
    pub const NONE: f64 = 0.0;
}

/// A §1.3 obligation: a foreground on a background, plus the state layer that can come between them.
struct Pair {
    name: String,
    fg: Rgb,
    bg: Rgb,
    /// The layer's colour and opacity. The colour is the **element's** content, which is usually
    /// `fg` and is not always: a row's secondary text is `on_surface_variant` while the row's own
    /// layer is `on_surface`, so the two are carried separately rather than assumed equal.
    layer: (Rgb, f64),
}

impl Pair {
    fn new(name: impl Into<String>, fg: Rgb, bg: Rgb, layer_color: Rgb, opacity: f64) -> Self {
        Self {
            name: name.into(),
            fg,
            bg,
            layer: (layer_color, opacity),
        }
    }

    /// What is behind `fg` once the layer is drawn.
    fn composited(&self) -> Rgb {
        let (color, opacity) = self.layer;
        over(color, opacity, self.bg)
    }
}

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

/// §1.3's table, with the state layer §5 puts between each foreground and its background.
///
/// The layer is what BUG-010 added. Which one each row carries is a statement about the *element*
/// that draws it, so it is written next to the pair rather than applied uniformly: `on_primary` on
/// `primary` is a filled button and a chip, `primary` on a surface is a text button, `on_background`
/// on `background` is the window itself and can be pressed by nobody.
fn text_pairs(r: &Roles) -> Vec<Pair> {
    use heaviest::{BUTTON, NONE, ROW};

    let mut out: Vec<Pair> = vec![
        // The window's own ground, and a snackbar's message. Neither is an interactive element.
        Pair::new(
            "on_background/background",
            r.on_background,
            r.background,
            r.on_background,
            NONE,
        ),
        Pair::new(
            "inverse_on_surface/inverse_surface",
            r.inverse_on_surface,
            r.inverse_surface,
            r.inverse_on_surface,
            NONE,
        ),
        // Accent fills: a filled button, and the chips and tags that reach `selected`.
        Pair::new(
            "on_primary/primary",
            r.on_primary,
            r.primary,
            r.on_primary,
            ROW,
        ),
        Pair::new(
            "on_primary_container/primary_container",
            r.on_primary_container,
            r.primary_container,
            r.on_primary_container,
            ROW,
        ),
        Pair::new(
            "on_secondary/secondary",
            r.on_secondary,
            r.secondary,
            r.on_secondary,
            ROW,
        ),
        Pair::new(
            "on_secondary_container/secondary_container",
            r.on_secondary_container,
            r.secondary_container,
            r.on_secondary_container,
            ROW,
        ),
        Pair::new(
            "on_tertiary/tertiary",
            r.on_tertiary,
            r.tertiary,
            r.on_tertiary,
            ROW,
        ),
        Pair::new(
            "on_tertiary_container/tertiary_container",
            r.on_tertiary_container,
            r.tertiary_container,
            r.on_tertiary_container,
            ROW,
        ),
        Pair::new("on_error/error", r.on_error, r.error, r.on_error, ROW),
        Pair::new(
            "on_error_container/error_container",
            r.on_error_container,
            r.error_container,
            r.on_error_container,
            ROW,
        ),
        // The snackbar's action: a text button, so `pressed` rather than `selected`.
        Pair::new(
            "inverse_primary/inverse_surface",
            r.inverse_primary,
            r.inverse_surface,
            r.inverse_primary,
            BUTTON,
        ),
        // The untyped `ToggleChip` — a neutral chip that carries its own text tone as its layer.
        // `BUTTON` and not `ROW`, and that is read from the widget rather than assumed from §5's
        // table: `toggle_chip` draws `hover` and `pressed` only, and expresses *on* by swapping the
        // fill instead of holding a `selected` layer. So its heaviest is 10%.
        Pair::new(
            "on_surface_variant/surface_variant",
            r.on_surface_variant,
            r.surface_variant,
            r.on_surface_variant,
            BUTTON,
        ),
    ];

    // `on_surface` and `on_surface_variant` are drawn on every container level, not just `surface`.
    // Both sit in rows and menu items, and a row's layer is its *primary* content colour — which is
    // `on_surface` for the secondary line too, since the layer belongs to the row and not to the
    // text inside it.
    for (name, bg) in all_surfaces(r) {
        out.push(Pair::new(
            format!("on_surface/{name}"),
            r.on_surface,
            bg,
            r.on_surface,
            ROW,
        ));
        out.push(Pair::new(
            format!("on_surface_variant/{name}"),
            r.on_surface_variant,
            bg,
            r.on_surface,
            ROW,
        ));
    }

    // Text buttons and links draw `primary` directly on a surface. §1.3 permits exactly these four,
    // and the enumeration is the *result* of the composited measurement rather than an input to it:
    // `surface_variant` and `surface_container_highest` left it because they fail below (BUG-010).
    for (name, bg) in [
        ("surface", r.surface),
        ("surface_container_low", r.surface_container_low),
        ("surface_container", r.surface_container),
        ("surface_container_high", r.surface_container_high),
    ] {
        out.push(Pair::new(
            format!("primary/{name}"),
            r.primary,
            bg,
            r.primary,
            BUTTON,
        ));
    }
    // Error helper text under a field: prose, not a control.
    for (name, bg) in [
        ("surface", r.surface),
        ("surface_container", r.surface_container),
    ] {
        out.push(Pair::new(
            format!("error/{name}"),
            r.error,
            bg,
            r.error,
            NONE,
        ));
    }

    // All eleven tags: each tag's own text tone on its own fill tone (§1.4).
    for &t in ConventionalType::ALL {
        let (fill, text) = r.tag(t);
        out.push(Pair::new(
            format!("tag_{}", t.as_str()),
            text,
            fill,
            text,
            ROW,
        ));
    }
    let (issue_fill, issue_text) = r.issue_tag();
    out.push(Pair::new(
        "tag_issue",
        issue_text,
        issue_fill,
        issue_text,
        ROW,
    ));

    out
}

fn assert_aa(scheme: ColorScheme) {
    let r = roles(scheme);
    let mut failures = Vec::new();
    for pair in text_pairs(&r) {
        let ratio = contrast(pair.fg, pair.bg);
        if ratio < AA_NORMAL {
            let (fg, bg) = (pair.fg, pair.bg);
            failures.push(format!(
                "  {}: {ratio:.2} < {AA_NORMAL} (fg {fg:?} on bg {bg:?})",
                pair.name
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

// ---------------------------------------------------------------------------------------------
// The same pairs, with §5's state layer composited (FR-004b, SC-008h — BUG-010)
// ---------------------------------------------------------------------------------------------

/// The pairs that do **not** clear AA once the heaviest layer is composited, with the ratio each
/// one actually measures.
///
/// This is a pin, not an exemption. The assertion below fails if a pair joins this set **and if one
/// leaves it** — a fixed pair breaks the build until its row is deleted, and a drifted one breaks it
/// as soon as the measurement moves by more than a hundredth. That shape is deliberate: BUG-010
/// existed because its predecessor recorded an excluded class in a module doc, and a doc comment
/// reports nothing when the exclusion stops being true (plan.md, "a gate's recorded scope is a claim
/// about the rest"). Both rows below are filed as BUG-011, which is where the remedy is decided —
/// each needs a role change with a visible cost, and neither is BUG-010's to make.
const UNDER_AA_COMPOSITED: [(&str, &str, f64); 2] = [
    // The snackbar's `Dismiss`, pressed. `inverse_primary` on `inverse_surface` is 4.99:1 at rest;
    // FR-004b's usual remedy does not reach it, because a snackbar has exactly one fill. The figure
    // is 4.37 and not the 4.34 or 4.40 a hand calculation gives, because §5's opacities are `f32`
    // and one channel here lands on a rounding boundary — which is itself the argument for reading
    // the constant rather than restating it.
    ("Dark", "inverse_primary/inverse_surface", 4.37),
    // The untyped `ToggleChip`, selected. 5.48:1 at rest, and here the remedy *does* reach: a
    // neutral chip can take a different neutral fill, as the `Info` banner just did.
    ("Dark", "on_surface_variant/surface_variant", 4.47),
];

/// §1.3's pairs, measured the way a user meets them: with the heaviest state layer the element can
/// carry drawn over the container (FR-004b).
///
/// The layer is the *content* colour over the container (§5), so it always pulls the background
/// toward the foreground and the ratio is always lower than the resting one this file asserted
/// alone until BUG-010. Four labels sat at 4.40–4.49:1 behind a green suite because of it.
#[test]
fn every_pair_still_meets_aa_with_its_state_layer_composited() {
    let mut measured: Vec<(&str, String, f64)> = Vec::new();

    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let name = match scheme {
            ColorScheme::Light => "Light",
            ColorScheme::Dark => "Dark",
        };
        let r = roles(scheme);
        for pair in text_pairs(&r) {
            let ratio = contrast(pair.fg, pair.composited());
            if ratio < AA_NORMAL {
                measured.push((name, pair.name, ratio));
            }
        }
    }

    let unknown: Vec<String> = measured
        .iter()
        .filter(|(scheme, name, _)| {
            !UNDER_AA_COMPOSITED
                .iter()
                .any(|(s, n, _)| s == scheme && n == name)
        })
        .map(|(scheme, name, ratio)| format!("  {scheme} / {name}: {ratio:.2} < {AA_NORMAL}"))
        .collect();
    assert!(
        unknown.is_empty(),
        "{} pair(s) clear AA at rest and fail once the state layer is composited:\n{}\n\nA state \
         layer is the content colour over the container (§5), so it always moves the background \
         toward the foreground: proving a pair at rest does not prove it. The remedy FR-004b names \
         is to narrow the host — the pair leaves §1.3's enumeration and the container takes a fill \
         that passes — not to retune the ramp, which is checked-in Material data (§1.1).",
        unknown.len(),
        unknown.join("\n")
    );

    for (scheme, name, expected) in UNDER_AA_COMPOSITED {
        let found = measured.iter().find(|(s, n, _)| *s == scheme && n == name);
        match found {
            Some((_, _, ratio)) => assert!(
                (ratio - expected).abs() < 0.005,
                "{scheme} / {name} now measures {ratio:.2}, pinned at {expected:.2}. The pin \
                 records what a known-bad pair is worth so that a change to it is visible; update \
                 it deliberately, with the bug report it belongs to."
            ),
            None => panic!(
                "{scheme} / {name} now clears AA composited, and is still pinned as a known miss \
                 at {expected:.2}. Delete its row from `UNDER_AA_COMPOSITED` and close BUG-011's \
                 corresponding entry — a pin that outlives its defect is the stale scope note this \
                 test exists to prevent."
            ),
        }
    }
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

/// An outlined button's 1dp border is `outline` over the same fill its label stands on, so it owes
/// the 3:1 with the button's own state layer composited — the divider case above measures the same
/// role where nothing can press it (FR-004b).
///
/// Over the four hosts §1.3 permits, and only those: `outline` on `surface_variant` is 2.96:1 in the
/// dark scheme **at rest**, which is why that host left the enumeration (BUG-010).
#[test]
fn an_outlined_buttons_border_clears_the_non_text_threshold_with_its_layer_composited() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        for (name, bg) in [
            ("surface", r.surface),
            ("surface_container_low", r.surface_container_low),
            ("surface_container", r.surface_container),
            ("surface_container_high", r.surface_container_high),
        ] {
            let behind = over(r.primary, heaviest::BUTTON, bg);
            let ratio = contrast(r.outline, behind);
            assert!(
                ratio >= AA_NON_TEXT,
                "{scheme:?} outline border on {name}, pressed: {ratio:.2} < {AA_NON_TEXT}. The \
                 border is measured against the host with the button's own layer over it, which is \
                 what is actually behind it under the pointer (FR-004b)."
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
    // And it has to actually carry layers, or the composited assertion above measures the resting
    // colours a second time and reports it as a stronger property than it checked.
    let layered = pairs.iter().filter(|p| p.layer.1 > 0.0).count();
    assert!(
        layered >= 30,
        "only {layered} of {} pairs carry a state layer — the composited walk would be measuring \
         the resting table again (FR-004b)",
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
