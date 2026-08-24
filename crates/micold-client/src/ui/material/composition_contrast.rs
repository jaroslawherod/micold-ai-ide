//! A component is legible against the container it is **rendered in**, not only against the
//! surfaces its own role was enumerated for (feature 018, T151 — FR-004a, FR-027b, SC-008g).
//!
//! `micold-core`'s `tokens_contrast` is the executable form of §1.3, and it is exhaustive over the
//! pairs that table *enumerates*. A table of enumerated pairs can only hold the compositions
//! somebody thought of, and the pair that broke was made by a call site: a `Button` pushed into a
//! `row` inside a styled `container`, three files from anything that names a colour. It appeared in
//! no product any gate took, while `style_snapshot` — which pins the banner's fill and the button's
//! colour, both correct — records styles one at a time and cannot see one inside the other. So
//! `Take over` shipped in `primary` on `error`: **1.00:1** in the light scheme, 1.01:1 in the dark,
//! with an `outline` border at 1.42:1 against a 3:1 threshold (BUG-009).
//!
//! This reads the **composition**. Both halves come from the functions the view calls —
//! [`style::notification_host`] for the fill, the variant's own style for what is drawn on it — for
//! the reason FR-029a gives about a restated number: an inventory of "which component sits on which
//! container", written beside the code rather than read from it, is a copy that nothing links to
//! its original. The two enums are walked in full (`NoticeLevel::ALL`, `Variant::ALL`), so a level
//! or a variant added later is covered without anyone remembering to add it here.
//!
//! **Scope: hosts that impose.** FR-004a is a rule about an *accent* fill — a container filled with
//! a colour §1.3 never enumerated for `primary`. The neutral hosts are walked to find which ones
//! those are and then skipped, because on them §1.3 already speaks and this file would be
//! second-guessing it. That skip is not free of findings: running the walk unscoped reports nine
//! near-misses on the `Info` banner's `surface_variant` — labels at 4.40–4.49:1 under the hover and
//! pressed state layers, and an `outline` border at 2.42–2.96:1 in the dark scheme, all against
//! thresholds of 4.5 and 3. Those are properties of §7.3's neutral table itself, present before this
//! feature and unchanged by it; they are recorded as BUG-010 rather than absorbed here, since
//! widening this gate to cover them would make it fail for a reason BUG-009 did not cause.
//!
//! Inside the crate for the reason `style_snapshot` states — the style layer is `pub(crate)` by
//! design (017 FR-002), so `tests/` cannot reach it.

use super::button::Variant;
use super::style;
use crate::features::notifications::NoticeLevel;
use iced::widget::button;
use iced::{Background, Color};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, contrast, Rgb, AA_NON_TEXT, AA_TEXT};

/// The renderer works in `iced::Color`; the thresholds are stated over the token type. Both are
/// 8-bit sRGB, so this is a change of representation and not of value.
fn rgb(c: Color) -> Rgb {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgb {
        r: q(c.r),
        g: q(c.g),
        b: q(c.b),
    }
}

/// The statuses a button is drawn in. `Disabled` is excluded and the exclusion is stated rather
/// than assumed: Material draws disabled content at 38% deliberately, which is below AA by design
/// and says "you cannot press this" — asserting a ratio there would forbid the disabled state.
const STATES: [(&str, button::Status); 3] = [
    ("active", button::Status::Active),
    ("hovered", button::Status::Hovered),
    ("pressed", button::Status::Pressed),
];

/// What is actually behind the label at `status`: the host's fill, with the button's own background
/// composited over it where it paints one.
///
/// A state layer is a semi-transparent quad drawn over whatever the button sits on, so the label's
/// real background changes under the pointer. Reading `style.background` alone would assert the
/// resting case and miss a hover fill that is `primary` at low alpha on red — which is what T036's
/// state layers were, and is why they were as invisible as the label.
fn behind(host_fill: Rgb, s: &button::Style) -> Rgb {
    match s.background {
        Some(Background::Color(c)) => rgb(style::over(c, style::color(host_fill))),
        _ => host_fill,
    }
}

#[test]
fn a_component_on_an_accent_fill_is_legible_against_it() {
    // Every violation, not the first: a foreground that is wrong on one host is usually wrong on
    // several, in both schemes and at every state, and a gate that stops at the first one turns a
    // single cause into a queue of reruns.
    let mut violations: Vec<String> = Vec::new();

    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        for level in NoticeLevel::ALL {
            let host = style::notification_host(r, level);
            // Read from `imposed`, not from a list of levels: whether a host obliges its children is
            // the host's own answer, so a level added later is classified by the same code that
            // colours it. `the_walk_covers_an_accent_host` guards the case where nothing qualifies.
            if host.imposed().is_none() {
                continue;
            }
            for variant in Variant::ALL {
                let style_fn = variant.style(r, Some(host));
                for (state_name, status) in STATES {
                    let s = style_fn(&theme, status);
                    let bg = behind(host.fill, &s);
                    let where_ = format!("{scheme:?} / {level:?} banner / {variant:?} / {state_name}");

                    let label = contrast(rgb(s.text_color), bg);
                    if label < AA_TEXT {
                        violations.push(format!("{where_}: label {label:.2}:1 (needs {AA_TEXT})"));
                    }
                    if s.border.width > 0.0 && s.border.color.a > 0.0 {
                        let border = contrast(rgb(s.border.color), bg);
                        if border < AA_NON_TEXT {
                            violations.push(format!(
                                "{where_}: {}px border {border:.2}:1 (needs {AA_NON_TEXT})",
                                s.border.width
                            ));
                        }
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "a component is drawn on a container it cannot be read against:\n  {}\n\nA component \
         placed on an accent-filled container draws its foreground from that container's paired \
         `on_*` role, not from the role §7.3 gives it on a neutral surface — and its border from \
         the same role, since `outline` is a neutral-variant tone (FR-004a, FR-027b, contract §7.3 \
         'Host surface'). The background each ratio is measured against is the host's fill with the \
         button's own state layer composited over it, which is what is actually behind the label.",
        violations.join("\n  ")
    );
}

/// The walk has to actually cover an accent host. If every level were neutral — or if `imposed`
/// stopped imposing — the assertions above would pass over nothing at all, which is the failure
/// mode a green gate cannot report about itself.
#[test]
fn the_walk_covers_an_accent_host() {
    let r = tokens::roles(ColorScheme::Light);
    let accents = NoticeLevel::ALL
        .into_iter()
        .filter(|&level| style::notification_host(r, level).imposed().is_some())
        .count();
    assert!(
        accents >= 1,
        "no notification level presents an accent fill, so the composition assertions covered \
         nothing. §1.3 enumerates the neutral surfaces `primary` may be drawn on; a fill outside \
         that set is what obliges a child to take the host's own foreground (FR-004a)."
    );
}
