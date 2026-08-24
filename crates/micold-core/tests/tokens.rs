//! Design-token invariants (SC-005): every `on_*` foreground role must meet WCAG AA
//! contrast (>= 4.5:1 for normal text) against its paired surface, in BOTH the light and
//! dark schemes. Pure — no renderer. Lives in `micold-core`, which declares no rendering dependency, so this
//! gate cannot accidentally come to depend on one (feature 017, FR-020/FR-022).
//!
//! Contrast is computed with the WCAG 2.x relative-luminance formula so a future palette
//! tweak that breaks legibility fails CI (contracts/design-tokens.md).

use micold_core::naming::ConventionalType;
use micold_core::theme::ColorScheme;
// `contrast` comes from the crate that owns the colours, not from a copy here — see
// the note in `tokens_contrast.rs`.
use micold_core::tokens::{contrast, roles, typography, Rgb, Roles, AA_TEXT};

/// The foreground/surface pairs that carry text and must meet AA. Includes the worktree tag
/// chips (feature 008): every per-type fill and the issue fill, paired with `on_tag`.
fn pairs(r: &Roles) -> Vec<(&'static str, Rgb, Rgb)> {
    let mut out = vec![
        ("on_background/background", r.on_background, r.background),
        ("on_surface/surface", r.on_surface, r.surface),
        (
            "on_surface_variant/surface_variant",
            r.on_surface_variant,
            r.surface_variant,
        ),
        ("on_primary/primary", r.on_primary, r.primary),
        ("on_error/error", r.on_error, r.error),
    ];
    for &t in ConventionalType::ALL {
        let (fill, on) = r.type_tag(t);
        out.push((t.as_str(), on, fill));
    }
    let (issue_fill, issue_on) = r.issue_tag();
    out.push(("tag_issue", issue_on, issue_fill));

    // The type-ahead's match emphasis (feature 021, FR-011). Emphasis is a colour role rather than
    // a fill, so it is *only* legible if it clears AA against whatever the row is drawn on — and a
    // row can be drawn on two different things: the menu surface, or the selected row's tonal
    // fill. Both are checked, in both schemes, because "the emphasis is legible" is a promise the
    // feature makes and nothing else here would catch a palette change that broke it.
    out.push(("emphasis/menu_surface", r.primary, r.surface));
    out.push(("emphasis/selected_row", r.primary, r.secondary_container));
    // An unavailable row is muted, and muted is not the same as unreadable: its reason has to stay
    // readable or FR-012 loses the point of listing it at all.
    out.push((
        "unavailable_row/menu_surface",
        r.on_surface_variant,
        r.surface,
    ));

    // The select's trigger, at rest (feature 022, FR-029). Its container is
    // `surface_container_highest` rather than `surface` — the filled field's, not the page's — so
    // neither pair above covers it, and the chevron is the one glyph in the application whose only
    // background is that container. `pick_list` drew both of these too and nothing measured them:
    // the colours came from a style closure that named roles and was never read back.
    out.push((
        "select_value/field",
        r.on_surface,
        r.surface_container_highest,
    ));
    out.push((
        "select_chevron/field",
        r.on_surface_variant,
        r.surface_container_highest,
    ));
    out
}

const AA_NORMAL: f64 = AA_TEXT;

#[test]
fn light_scheme_meets_aa_contrast() {
    let r = roles(ColorScheme::Light);
    for (name, fg, bg) in pairs(&r) {
        let ratio = contrast(fg, bg);
        assert!(
            ratio >= AA_NORMAL,
            "light {name}: contrast {ratio:.2} < {AA_NORMAL} (fg {fg:?} on bg {bg:?})"
        );
    }
}

#[test]
fn dark_scheme_meets_aa_contrast() {
    let r = roles(ColorScheme::Dark);
    for (name, fg, bg) in pairs(&r) {
        let ratio = contrast(fg, bg);
        assert!(
            ratio >= AA_NORMAL,
            "dark {name}: contrast {ratio:.2} < {AA_NORMAL} (fg {fg:?} on bg {bg:?})"
        );
    }
}

/// Alpha-composite `fg` at opacity `a` over opaque `bg` (straight-alpha "over").
fn composite(fg: Rgb, bg: Rgb, a: f64) -> Rgb {
    let mix = |f: u8, b: u8| ((f as f64) * a + (b as f64) * (1.0 - a)).round() as u8;
    Rgb {
        r: mix(fg.r, bg.r),
        g: mix(fg.g, bg.g),
        b: mix(fg.b, bg.b),
    }
}

/// The tag chip's tint opacity — MUST match `alpha(accent, _)` in `ui::style::chip`.
const CHIP_TINT_ALPHA: f64 = 0.20;

/// The sidebar tags render as TONAL chips: accent-colored text on a faint accent tint over the
/// sidebar surface (see `ui::style::chip`). Verify that rendered text meets AA in both schemes,
/// for every type accent and the issue accent (SC-007). This is the combination actually drawn;
/// the solid pairs above cover the filter chips' active (filled) state.
#[test]
fn tonal_tag_chips_meet_aa_contrast() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        let mut accents: Vec<(&str, Rgb)> = ConventionalType::ALL
            .iter()
            .map(|&t| (t.as_str(), r.tag_fill(t)))
            .collect();
        // `tag_issue` was a bare field until Phase 0 gave every tag its own text tone; the fill now
        // comes from the same accessor as the typed tags (contract §1.4).
        accents.push(("issue", r.issue_tag().0));
        for (name, accent) in accents {
            let bg = composite(accent, r.surface, CHIP_TINT_ALPHA);
            let ratio = contrast(accent, bg);
            assert!(
                ratio >= AA_NORMAL,
                "{scheme:?} tonal tag {name}: contrast {ratio:.2} < {AA_NORMAL} (accent {accent:?} on tint {bg:?})"
            );
        }
    }
}

/// Every worktree type has a distinct fill in both schemes (FR-005).
#[test]
fn type_tag_fills_are_distinct() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        let fills: Vec<Rgb> = ConventionalType::ALL
            .iter()
            .map(|&t| r.tag_fill(t))
            .collect();
        for i in 0..fills.len() {
            for j in (i + 1)..fills.len() {
                assert_ne!(
                    fills[i], fills[j],
                    "{scheme:?}: type tag fills {i} and {j} collide"
                );
            }
        }
    }
}

/// The sidebar stays denser than the text it nests under (FR-011).
///
/// It used to be "exactly 80% of the app-wide scale, rounded". Feature 018 §2.4 replaces that
/// derivation: each sidebar role now maps to the *nearest smaller role in the Material scale*
/// rather than to a computed size, which is what keeps it inside the scale instead of beside it.
/// `body_small` is 12 against `body_medium`'s 14 — a reduction, but not 80% of it.
///
/// So what is asserted is the decision that must survive, not the arithmetic that used to express
/// it: sidebar text is smaller than body text, and a session line shares its worktree name's role.
/// `micold-core/tests/tokens_scales.rs` pins which roles those are.
#[test]
#[allow(clippy::assertions_on_constants)] // the point is to guard the values; clippy can see the answer
fn the_sidebar_stays_denser_than_the_text_it_nests_under() {
    assert!(
        typography::SIDEBAR_NAME.size < typography::BODY_MEDIUM.size,
        "sidebar name {} is not smaller than body {}",
        typography::SIDEBAR_NAME.size,
        typography::BODY_MEDIUM.size
    );
    assert!(typography::SIDEBAR_TAG.size < typography::LABEL_MEDIUM.size);
    assert_eq!(typography::SIDEBAR_SESSION, typography::SIDEBAR_NAME);
}

/// Every field state layer stays legible, on every field (feature 022, T047 — FR-029, FR-036a).
///
/// T031 checked the select's two states because they were the only ones that existed. BUG-002 gave
/// the layer to the shared container, so a **text field** now carries it too — and added focus,
/// whose opacity had been sitting in the scale unused. Three states over one container, under both
/// of the roles that sit on it.
///
/// The point of measuring `focus` in particular: it is the state a person leaves a field in while
/// reading what they typed, so it is the one where a contrast failure would be looked at longest.
#[test]
fn every_field_state_layer_stays_legible() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        for (state, opacity) in [
            ("hovered", micold_core::tokens::state::HOVER as f64),
            ("focused", micold_core::tokens::state::FOCUS as f64),
            ("pressed", micold_core::tokens::state::PRESSED as f64),
        ] {
            let background = composite(r.on_surface, r.surface_container_highest, opacity);
            for (what, fg) in [
                ("value", r.on_surface),
                ("label", r.on_surface_variant),
                ("placeholder", r.on_surface_variant),
            ] {
                let ratio = contrast(fg, background);
                assert!(
                    ratio >= AA_NORMAL,
                    "{scheme:?} field {what} while {state}: contrast {ratio:.2} < {AA_NORMAL}. \
                     Every field wears this layer now, not only the select — so a new opacity or a \
                     moved container role fails here for all of them at once (FR-029, FR-036a)"
                );
            }
        }
    }
}

/// The checkbox's hover layer, composited into its fill (feature 022, T047 — FR-029, FR-036).
///
/// The layer is blended into an opaque `background` rather than drawn as its own quad, because
/// `checkbox::Style` has nowhere to put a translucent one. Both fills are checked: `surface` while
/// unchecked and `primary` while checked, each under the mark that sits on it.
#[test]
fn the_checkboxs_hover_layer_stays_legible() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        let hover = micold_core::tokens::state::HOVER as f64;
        for (what, base, fg) in [
            ("unchecked box", r.surface, r.on_surface),
            ("checked box", r.primary, r.on_primary),
        ] {
            let background = composite(r.on_surface, base, hover);
            let ratio = contrast(fg, background);
            assert!(
                ratio >= AA_NORMAL,
                "{scheme:?} checkbox {what} while hovered: contrast {ratio:.2} < {AA_NORMAL}. \
                 The hover layer darkens the fill the mark is read against (FR-029, FR-036)"
            );
        }
    }
}

/// The checkbox's **focus** layer, on the same fills (BUG-003 — FR-029, FR-035).
///
/// Added rather than folded into the hover gate above: focus is the stronger of the two opacities,
/// so it moves the fill further toward the mark read against it and is the one that would fail
/// first. A single loop over both would have been tidier and would have rewritten an assertion that
/// was already earning its keep, which FR-027 does not allow and which is the wrong trade anyway —
/// the hover gate is why hover is safe, and this is why focus is.
///
/// It exists at all because the checkbox had no focus to shade until it was given a keyboard.
#[test]
fn the_checkboxs_focus_layer_stays_legible() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        let focus = micold_core::tokens::state::FOCUS as f64;
        for (what, base, fg) in [
            ("unchecked box", r.surface, r.on_surface),
            ("checked box", r.primary, r.on_primary),
        ] {
            let background = composite(r.on_surface, base, focus);
            let ratio = contrast(fg, background);
            assert!(
                ratio >= AA_NORMAL,
                "{scheme:?} checkbox {what} while focused: contrast {ratio:.2} < {AA_NORMAL}. \
                 The focus layer is the stronger of the two and moves the fill the mark is read \
                 against furthest (FR-029, FR-035)"
            );
        }
    }
}

/// The select's trigger **under its own state layer** (feature 022, T031 — FR-029).
///
/// §5 draws hover and open as `on_surface` over the container at the state's opacity, and §7.7
/// asks the select to carry both that way — so the background its value and chevron are read
/// against is not the container but the container *with the layer on it*. Two of the three colours
/// in that sum are the same role, which is exactly the arrangement where contrast quietly falls:
/// the layer moves the background toward the foreground.
///
/// Checked as the composite actually drawn, in both schemes, like the tag chips above. The at-rest
/// pair is in [`pairs`]; this is the two states a person puts the control into to use it.
#[test]
fn the_selects_state_layers_stay_legible() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        for (state, opacity) in [
            ("hovered", micold_core::tokens::state::HOVER as f64),
            ("open", micold_core::tokens::state::PRESSED as f64),
        ] {
            let background = composite(r.on_surface, r.surface_container_highest, opacity);
            for (what, fg) in [("value", r.on_surface), ("chevron", r.on_surface_variant)] {
                let ratio = contrast(fg, background);
                assert!(
                    ratio >= AA_NORMAL,
                    "{scheme:?} select {what} while {state}: contrast {ratio:.2} < {AA_NORMAL}. \
                     The state layer is `on_surface` over the field container, so it pulls the \
                     background toward the text it sits under — a layer opacity or a container \
                     role that moved would show up here first (FR-029)"
                );
            }
        }
    }
}
