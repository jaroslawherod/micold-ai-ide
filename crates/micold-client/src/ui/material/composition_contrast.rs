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
//! **Scope: every host, accent or neutral (SC-008h).** It was narrower once. FR-004a is a rule about
//! an *accent* fill, so the walk skipped the neutral hosts and said so in this comment, naming the
//! nine violations the skip was hiding and the bug they were filed as. That note was accurate,
//! prominent, and useless: an excluded class stays excluded until somebody chooses to go back for
//! it, while a green gate reads to everyone — including its own author three commits later — as
//! "compositions are covered". It is worse than an untested area, because an untested area does not
//! report success. The filter is gone; a neutral fill outside §1.3's enumeration is caught here now,
//! which is how the `Info` banner's host came to move (BUG-010).
//!
//! Inside the crate for the reason `style_snapshot` states — the style layer is `pub(crate)` by
//! design (017 FR-002), so `tests/` cannot reach it.

use super::button::Variant;
use super::style;
use crate::features::notifications::NoticeLevel;
use iced::widget::button;
use iced::Background;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, contrast, Rgb, AA_NON_TEXT, AA_TEXT};

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
        Some(Background::Color(c)) => style::to_rgb(style::over(c, style::color(host_fill))),
        _ => host_fill,
    }
}

#[test]
fn a_component_is_legible_against_the_container_it_is_drawn_in() {
    // Every violation, not the first: a foreground that is wrong on one host is usually wrong on
    // several, in both schemes and at every state, and a gate that stops at the first one turns a
    // single cause into a queue of reruns.
    let mut violations: Vec<String> = Vec::new();

    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        for level in NoticeLevel::ALL {
            let host = style::notification_host(r, level);
            for variant in Variant::ALL {
                let style_fn = variant.style(r, Some(host));
                for (state_name, status) in STATES {
                    let s = style_fn(&theme, status);
                    let bg = behind(host.fill, &s);
                    let where_ =
                        format!("{scheme:?} / {level:?} banner / {variant:?} / {state_name}");

                    let label = contrast(style::to_rgb(s.text_color), bg);
                    if label < AA_TEXT {
                        violations.push(format!("{where_}: label {label:.2}:1 (needs {AA_TEXT})"));
                    }
                    if s.border.width > 0.0 && s.border.color.a > 0.0 {
                        let border = contrast(style::to_rgb(s.border.color), bg);
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
         'Host surface'). A **neutral** fill fails this for the other reason: §1.3 enumerates the \
         four surfaces `primary` may be drawn on, and a container filled with a neutral role outside \
         that set is not a host — it takes a permitted fill instead (FR-004b). The background each \
         ratio is measured against is the host's fill with the button's own state layer composited \
         over it, which is what is actually behind the label.",
        violations.join("\n  ")
    );
}

/// Feature 039, U146 (contract `unread-mark.md` U3, FR-018, FR-023): the unread mark is a non-text
/// mark, so it needs 3:1 against every fill a host draws it on, in both schemes. The fills are read
/// from the functions the hosts call: the sidebar's surface, the selected row's pill (a window
/// without keyboard focus has its selected session unread), the menu's panel and the app bar.
#[test]
fn the_unread_mark_is_legible_on_every_fill_a_host_draws_it_on() {
    let mut violations: Vec<String> = Vec::new();

    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let mark = super::unread_mark::fill(r);
        for (host, fill) in [
            ("the sidebar", style::to_rgb(style::sidebar_fill(r))),
            // A hovered sidebar row (feature 575, FR-013): the location row's count is hovered as
            // often as it is read.
            ("a hovered sidebar row", hovered_sidebar_row(r)),
            ("a selected row", r.secondary_container),
            ("a menu panel", r.surface_container),
            ("the app bar", r.surface),
        ] {
            let ratio = contrast(mark, fill);
            if ratio < AA_NON_TEXT {
                violations.push(format!(
                    "{scheme:?} / {host}: {ratio:.2}:1 (needs {AA_NON_TEXT})"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "the unread mark cannot be told from the fill it is drawn on:\n  {}",
        violations.join("\n  ")
    );
}

/// Feature 905 (FR-007, contract A6): each state icon's glyph is non-text, so it needs 3:1 against
/// the sidebar row at rest, hovered and selected, in both schemes. The colour is the one the row
/// draws, `state_icon::role`.
#[test]
fn every_state_icon_is_legible_on_every_fill_of_a_sidebar_row() {
    use micold_core::location_state::{BlockedReason, LocationState};
    let mut violations: Vec<String> = Vec::new();

    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        for state in [
            LocationState::Running,
            LocationState::Blocked(BlockedReason::NeedsPermission),
            LocationState::Idle,
        ] {
            let glyph = super::state_icon::role(state, &r).expect("a drawn state has a role");
            for (host, fill) in [
                ("the sidebar", style::to_rgb(style::sidebar_fill(r))),
                ("a hovered sidebar row", hovered_sidebar_row(r)),
                ("a selected row", r.secondary_container),
            ] {
                let ratio = contrast(glyph, fill);
                if ratio < AA_NON_TEXT {
                    violations.push(format!(
                        "{scheme:?} / {state:?} on {host}: {ratio:.2}:1 (needs {AA_NON_TEXT})"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "a state icon cannot be told from the row it is drawn on:\n  {}",
        violations.join("\n  ")
    );
}

/// The fill behind a hovered sidebar row: the sidebar's surface with the hover state layer of the
/// row's content colour over it (feature 575, FR-013).
fn hovered_sidebar_row(r: tokens::Roles) -> Rgb {
    style::to_rgb(style::state_layer(
        style::sidebar_fill(r),
        style::color(r.on_surface),
        tokens::state::HOVER,
    ))
}

/// Feature 575 (FR-013, contract A9, R9): the number beside a location row's unread mark is text,
/// so it needs 4.5:1 against the row at rest, hovered and selected, in both schemes. The colour is
/// the one the row draws it in, `tree_view::count_tint`, also on an error-tinted row.
#[test]
fn a_location_rows_unread_count_is_legible_in_every_row_state() {
    let mut violations: Vec<String> = Vec::new();

    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        for row_tint in [r.on_surface, r.error] {
            let count = super::tree_view::count_tint(r, row_tint);
            for (state, fill) in [
                ("at rest", style::to_rgb(style::sidebar_fill(r))),
                ("hovered", hovered_sidebar_row(r)),
                ("selected", r.secondary_container),
            ] {
                let ratio = contrast(count, fill);
                if ratio < AA_TEXT {
                    violations.push(format!(
                        "{scheme:?} / row {state}: {ratio:.2}:1 (needs {AA_TEXT})"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "a location row's unread count cannot be read on its row:\n  {}",
        violations.join("\n  ")
    );
}

/// The walk has to cover **both** kinds of host. If every level were accent the neutral rule would
/// go unexercised, and if every level were neutral the imposition would — either way the assertions
/// above pass over a case nobody is checking, which is the failure mode a green gate cannot report
/// about itself.
#[test]
fn the_walk_covers_a_host_of_each_kind() {
    let r = tokens::roles(ColorScheme::Light);
    let accents = NoticeLevel::ALL
        .into_iter()
        .filter(|&level| style::notification_host(r, level).imposed().is_some())
        .count();
    assert!(
        accents >= 1,
        "no notification level presents an accent fill, so the imposition rule was asserted over \
         nothing. §1.3 enumerates the neutral surfaces `primary` may be drawn on; a fill outside \
         that set is what obliges a child to take the host's own foreground (FR-004a)."
    );
    assert!(
        NoticeLevel::ALL.len() - accents >= 1,
        "no notification level presents a neutral fill, so removing this walk's host filter proved \
         nothing. The filter came out because an excluded class stays excluded and a green gate \
         still reads as coverage (SC-008h, BUG-010); a walk with no neutral host left in it is that \
         exclusion again, written differently."
    );
}

/// Diff rows (feature 482, US1 s8): a line's text (`on_surface`) and its muted line numbers
/// (`on_surface_variant`) are read on the added and the removed tint, in both schemes, at 4.5:1.
#[test]
fn diff_line_text_is_legible_on_the_added_and_removed_tints() {
    let mut violations: Vec<String> = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        for (tint_name, tint) in [
            ("diff_added", r.diff_added),
            ("diff_removed", r.diff_removed),
        ] {
            for (text_name, text) in [
                ("on_surface", r.on_surface),
                ("on_surface_variant", r.on_surface_variant),
            ] {
                let ratio = contrast(text, tint);
                if ratio < AA_TEXT {
                    violations.push(format!(
                        "{scheme:?} / {text_name} on {tint_name}: {ratio:.2}:1 (needs {AA_TEXT})"
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "diff text cannot be read on its row tint:\n  {}",
        violations.join("\n  ")
    );
}

/// Warning and success roles (feature 813, research R11): each `on_*` role is read on its fill at
/// 4.5:1 in both schemes, like the M3 role pairs they sit beside.
#[test]
fn warning_and_success_roles_are_legible_on_their_fills() {
    let mut violations: Vec<String> = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        for (pair, text, fill) in [
            ("on_warning / warning", r.on_warning, r.warning),
            (
                "on_warning_container / warning_container",
                r.on_warning_container,
                r.warning_container,
            ),
            ("on_success / success", r.on_success, r.success),
            (
                "on_success_container / success_container",
                r.on_success_container,
                r.success_container,
            ),
        ] {
            let ratio = contrast(text, fill);
            if ratio < AA_TEXT {
                violations.push(format!(
                    "{scheme:?} / {pair}: {ratio:.2}:1 (needs {AA_TEXT})"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "a warning or success role cannot be read on its fill:\n  {}",
        violations.join("\n  ")
    );
}

/// Overview cards (feature 1120, T031 — FR-002, FR-003, FR-005, SC-001; BUG-1298 FR-022): in both
/// schemes and the five states, the header's words and icons, the type word at 85 % included, read
/// at 4.5:1 on the header, and the title and the
/// other body lines at 4.5:1 on the body; the body is one fill for every state, and Idle's grey
/// header differs from the body and from every coloured header, so Idle stays de-emphasised by its
/// header alone. The roles come from the card's `palette`, the function the card is drawn with.
#[test]
fn an_overview_cards_header_and_body_text_is_legible_on_their_fills() {
    use micold_core::overview::CardState;
    let mut violations: Vec<String> = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let idle = super::overview_card::palette(CardState::Idle, &r);
        for state in CardState::ALL {
            let p = super::overview_card::palette(state, &r);
            for (what, ink, ground) in [
                ("header text", p.on_header, p.header),
                ("type word at 85 %", p.type_word, p.header),
                ("title", p.on_body, p.body),
                ("body line", p.on_body_variant, p.body),
            ] {
                let ratio = contrast(ink, ground);
                if ratio < AA_TEXT {
                    violations.push(format!(
                        "{scheme:?} / {state:?} {what}: {ratio:.2}:1 (needs {AA_TEXT})"
                    ));
                }
            }
            assert_eq!(
                p.body, idle.body,
                "{scheme:?} / {state:?}: FR-002, one body"
            );
            if state != CardState::Idle {
                assert_ne!(
                    p.header, idle.header,
                    "{scheme:?} / {state:?}: Idle's header must differ (FR-005)"
                );
            }
        }
        assert_eq!(idle.header, r.surface_variant, "{scheme:?}: Idle's header");
        assert_eq!(idle.on_header, r.on_surface_variant, "{scheme:?}");
        assert_eq!(
            idle.body, r.surface,
            "{scheme:?}: the body (BUG-1298 FR-021)"
        );
        assert_ne!(
            idle.header, idle.body,
            "{scheme:?}: Idle's header is a band"
        );
    }
    assert!(
        violations.is_empty(),
        "an Overview card cannot be read:\n  {}",
        violations.join("\n  ")
    );
}

/// Overview card issue link (feature 1122, T009 — FR-015): the `TextLink`'s `primary` label reads at
/// feature 1120's FR-003 4.5:1 on the card's body in both schemes and every state. Collapsed and
/// expanded draw the same body fill (`palette`), and a targeted card lays the selected layer over
/// it, so both grounds are checked.
#[test]
fn an_overview_cards_issue_link_is_legible_on_its_body() {
    use micold_core::overview::CardState;
    let mut violations: Vec<String> = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        for state in CardState::ALL {
            let p = super::overview_card::palette(state, &r);
            let selected = tokens::over(p.on_body, f64::from(tokens::state::SELECTED), p.body);
            for (what, ground) in [("body", p.body), ("body + selected layer", selected)] {
                let ratio = contrast(r.primary, ground);
                if ratio < AA_TEXT {
                    violations.push(format!(
                        "{scheme:?} / {state:?} link on {what}: {ratio:.2}:1 (needs {AA_TEXT})"
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "an issue link cannot be read on a card:\n  {}",
        violations.join("\n  ")
    );
}

/// Overview card pull request rows (feature 1123, T011 — FR-015, research R9; BUG-1298 FR-026):
/// each state's pill word in its container's on-colour on the tonal pill (`overview_card::pill`,
/// drawn by `style::chip_solid`), the status words in their tone (*bad* in `error`, *waiting* in
/// `warning`, *good* in `success`, *plain* muted), the number in `primary` and the "+N more" line
/// in the muted body role,
/// on the body bare and under the selected layer, in every card state and both schemes, at
/// feature 1120 FR-003's 4.5:1.
#[test]
fn an_overview_cards_pull_request_rows_are_legible_on_its_body() {
    use super::PrMark;
    use micold_core::overview::CardState;
    let mut violations: Vec<String> = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        for state in CardState::ALL {
            let p = super::overview_card::palette(state, &r);
            let selected = tokens::over(p.on_body, f64::from(tokens::state::SELECTED), p.body);
            for (ground_name, ground) in [("body", p.body), ("body + selected layer", selected)] {
                let mut pairs: Vec<(String, Rgb, Rgb)> = vec![
                    ("bad words".into(), r.error, ground),
                    ("waiting words".into(), r.warning, ground),
                    ("good words".into(), r.success, ground),
                    ("plain words".into(), p.on_body_variant, ground),
                    ("+N more".into(), p.on_body_variant, ground),
                    ("number".into(), r.primary, ground),
                ];
                for mark in [PrMark::Open, PrMark::Draft, PrMark::Merged, PrMark::Closed] {
                    let (fill, on_fill) = super::overview_card::pill(mark, &r);
                    let chip = style::chip_solid(fill, on_fill)(&iced::Theme::Light);
                    let Some(Background::Color(fill)) = chip.background else {
                        panic!("a pill paints a colour");
                    };
                    let fill = if ground == p.body {
                        style::to_rgb(fill)
                    } else {
                        tokens::over(
                            p.on_body,
                            f64::from(tokens::state::SELECTED),
                            style::to_rgb(fill),
                        )
                    };
                    let ink = style::to_rgb(chip.text_color.expect("a pill's word colour"));
                    pairs.push((format!("{} pill", mark.word()), ink, fill));
                }
                for (what, ink, fill) in pairs {
                    let ratio = contrast(ink, fill);
                    if ratio < AA_TEXT {
                        violations.push(format!(
                            "{scheme:?} / {state:?} {what} on {ground_name}: {ratio:.2}:1 (needs {AA_TEXT})"
                        ));
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "a pull request row cannot be read on a card:\n  {}",
        violations.join("\n  ")
    );
}

/// Overview card target mark (feature 972, T015 — FR-006; feature 1120, T031): the `primary`
/// outline reads at 3:1 against each card state's header and body, bare and with the selected
/// layer over them, both schemes.
#[test]
fn the_card_target_outline_is_visible_on_every_state_fill() {
    use micold_core::overview::CardState;
    let mut violations: Vec<String> = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        for state in CardState::ALL {
            let p = super::overview_card::palette(state, &r);
            let layer = |fill| tokens::over(p.on_body, f64::from(tokens::state::SELECTED), fill);
            for (what, ground) in [
                ("header", p.header),
                ("header + selected layer", layer(p.header)),
                ("body", p.body),
                ("body + selected layer", layer(p.body)),
            ] {
                let ratio = contrast(r.primary, ground);
                if ratio < AA_NON_TEXT {
                    violations.push(format!(
                        "{scheme:?} / {state:?} outline on {what}: {ratio:.2}:1 (needs {AA_NON_TEXT})"
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "the target outline cannot be seen on a card:\n  {}",
        violations.join("\n  ")
    );
}

/// Syntax colours (feature 482, T043, US1 s8): every colour a highlighter theme can give, as the
/// diff draws it through `syntax::legible`, reads at 4.5:1 on the diff's surface and on both line
/// tints of its scheme. The themes' palettes are not public, so the walk sweeps a grid of the whole
/// colour cube, and then the colours `InspiredGitHub` and `Base16Ocean` actually give a Rust sample.
#[test]
fn syntax_colours_are_legible_on_the_diff_surface_and_both_tints() {
    use crate::ui::syntax;
    let mut violations: Vec<String> = Vec::new();
    let steps = [0u8, 51, 102, 153, 204, 255];
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let mut check = |what: String, colour: Rgb| {
            for fill in syntax::fills(r) {
                let ratio = contrast(colour, fill);
                if ratio < AA_TEXT {
                    violations.push(format!("{scheme:?} / {what} on {fill:?}: {ratio:.2}:1"));
                }
            }
        };
        for red in steps {
            for green in steps {
                for blue in steps {
                    let raw = Rgb {
                        r: red,
                        g: green,
                        b: blue,
                    };
                    check(format!("{raw:?}"), syntax::legible(raw, r));
                }
            }
        }
        let sample = [
            "// a comment, in the theme's faintest colour",
            "#[derive(Debug)]",
            "pub fn main() -> Result<(), String> {",
            "    let s = \"text\"; let n = 42_u8 as f32;",
            "    Ok(())",
            "}",
        ];
        let spans = syntax::highlight_lines("rs", sample, scheme);
        let mut colours: Vec<Rgb> = spans.0.iter().flatten().map(|(_, c)| *c).collect();
        colours.sort_by_key(|c| (c.r, c.g, c.b));
        colours.dedup();
        assert!(
            colours.len() >= 3,
            "{scheme:?}: the Rust sample gave {} colours, so the walk checked nothing",
            colours.len()
        );
        for colour in colours {
            check(format!("highlighted {colour:?}"), colour);
        }
    }
    assert!(
        violations.is_empty(),
        "a syntax colour cannot be read on the diff:\n  {}",
        violations.join("\n  ")
    );
}

/// `DisclosureCard` (feature 939, research R11, SC-005): on the card's own fill, in both schemes,
/// the header label reads at 4.5:1 at rest, hovered and pressed; the muted summary and the failed
/// status's label and icon (the `error` role) at 4.5:1; and a failed card's outline at 3:1. The
/// fill and the outline come from `style::disclosure_card`, the style the card is drawn with, and the
/// label from the text button the header is.
#[test]
fn a_disclosure_cards_header_summary_and_failed_status_are_legible_on_its_fill() {
    let mut violations: Vec<String> = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        let fill = match style::disclosure_card(r, false)(&theme).background {
            Some(Background::Color(c)) => style::to_rgb(c),
            other => panic!("{scheme:?}: the card paints a colour fill, not {other:?}"),
        };
        let failed = style::disclosure_card(r, true)(&theme);
        let header = Variant::Text.style(r, None);
        for (state, status) in STATES {
            let s = header(&theme, status);
            let ground = behind(fill, &s);
            let ratio = contrast(style::to_rgb(s.text_color), ground);
            if ratio < AA_TEXT {
                violations.push(format!(
                    "{scheme:?} / header label {state}: {ratio:.2}:1 (needs {AA_TEXT})"
                ));
            }
        }
        for (what, ink, needs) in [
            ("summary", r.on_surface_variant, AA_TEXT),
            ("failed label and icon", r.error, AA_TEXT),
            (
                "failed outline",
                style::to_rgb(failed.border.color),
                AA_NON_TEXT,
            ),
        ] {
            let ratio = contrast(ink, fill);
            if ratio < needs {
                violations.push(format!("{scheme:?} / {what}: {ratio:.2}:1 (needs {needs})"));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "a disclosure card cannot be read:\n  {}",
        violations.join("\n  ")
    );
}

/// 944 FR-002, FR-003 (research R12): a `ProgressBar`'s fill — `primary`, or `error` for a warning
/// bar — is visible on its track at 3:1 in both schemes. The marker is drawn in whichever of the two
/// it does not sit on, so this pair is also the marker's.
#[test]
fn a_progress_bars_fill_and_marker_are_visible_on_its_track() {
    use super::progress::track_color;
    use super::progress_bar::fill_color;
    let mut violations: Vec<String> = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        for warning in [false, true] {
            let ratio = contrast(fill_color(&r, warning), track_color(&r));
            if ratio < AA_NON_TEXT {
                violations.push(format!(
                    "{scheme:?} / warning {warning}: {ratio:.2}:1 (needs {AA_NON_TEXT})"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "a progress bar's fill cannot be told from its track:\n  {}",
        violations.join("\n  ")
    );
}

/// Feature 940 (research R9, SC-006): in both schemes, the transcript's rich text reads at 4.5:1 on
/// what it is drawn on — code and inline code on the code fill, link and plain text (muted or not)
/// on the transcript's surface and on a card's fill, the copy control's words (and the failed
/// error role) and a hunk's note on the code fill, and the `+`/`−` markers on both diff tints.
#[test]
fn rich_transcript_text_is_legible_on_its_fills() {
    use super::code_block::copy_look;
    use super::diff_view::marker_ink;
    use super::link_text::{ink, link_ink};
    use crate::features::transcript_rich::{CopyState, Ticket};
    let mut violations: Vec<String> = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        let card = match style::disclosure_card(r, false)(&theme).background {
            Some(Background::Color(c)) => style::to_rgb(c),
            other => panic!("{scheme:?}: the card paints a colour fill, not {other:?}"),
        };
        let code = style::code_fill(r);
        let mut pairs: Vec<(String, Rgb, Rgb)> = vec![
            ("code text on the code fill".into(), r.on_surface, code),
            (
                "a hunk's note on the code fill".into(),
                r.on_surface_variant,
                code,
            ),
            ("added marker".into(), marker_ink(r), r.diff_added),
            ("removed marker".into(), marker_ink(r), r.diff_removed),
        ];
        for (name, fill) in [("surface", r.surface), ("card", card)] {
            pairs.push((format!("link on the {name}"), link_ink(r), fill));
            pairs.push((format!("text on the {name}"), ink(r, false), fill));
            pairs.push((format!("muted text on the {name}"), ink(r, true), fill));
            pairs.push((
                format!("inline code on the {name}'s code fill"),
                ink(r, false),
                code,
            ));
        }
        for state in [
            None,
            Some(CopyState::Copied(Ticket::posed(1))),
            Some(CopyState::Failed(Ticket::posed(1))),
        ] {
            let (_, tint, words) = copy_look(state, r);
            pairs.push((format!("copy control {words:?}"), tint, code));
        }
        for (what, ink, fill) in pairs {
            let ratio = contrast(ink, fill);
            if ratio < AA_TEXT {
                violations.push(format!(
                    "{scheme:?} / {what}: {ratio:.2}:1 (needs {AA_TEXT})"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "rich transcript text cannot be read:\n  {}",
        violations.join("\n  ")
    );
}

/// `QueueRow` and `AttachmentChip` (feature 941, T048): on each one's own plate, in both schemes,
/// the text reads at 4.5:1 — a queued message, its muted position and count, and its text-button
/// actions at rest, hovered and pressed; a chip's name and its remove glyph.
#[test]
fn a_queue_rows_and_an_attachment_chips_words_are_legible_on_their_plates() {
    let mut violations: Vec<String> = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        let plate = |s: iced::widget::container::Style, what: &str| match s.background {
            Some(Background::Color(c)) => style::to_rgb(c),
            other => panic!("{scheme:?}: the {what} paints a colour fill, not {other:?}"),
        };
        let row = plate(style::queue_row(r)(&theme), "queue row");
        let chip = plate(style::attachment_chip(r)(&theme), "attachment chip");
        let action = Variant::Text.style(r, None);
        for (state, status) in STATES {
            let s = action(&theme, status);
            let ratio = contrast(style::to_rgb(s.text_color), behind(row, &s));
            if ratio < AA_TEXT {
                violations.push(format!(
                    "{scheme:?} / queue action {state}: {ratio:.2}:1 (needs {AA_TEXT})"
                ));
            }
        }
        for (what, ink, ground) in [
            ("queued text", r.on_surface, row),
            ("position and count", r.on_surface_variant, row),
            ("chip name", r.on_surface, chip),
            ("chip remove glyph", r.on_surface_variant, chip),
        ] {
            let ratio = contrast(ink, ground);
            if ratio < AA_TEXT {
                violations.push(format!(
                    "{scheme:?} / {what}: {ratio:.2}:1 (needs {AA_TEXT})"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "composer parts below contrast:\n{}",
        violations.join("\n")
    );
}
