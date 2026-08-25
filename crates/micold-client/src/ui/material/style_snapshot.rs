//! Style parity snapshot (feature 017, T001a).
//!
//! This feature's defining property is that it changes **where** an appearance is decided, never
//! **what** it is (FR-005, FR-023). Verifying that by eye means photographing ~40 screens before
//! the refactor and comparing afterwards — slow, incomplete, and impossible to re-run.
//!
//! Instead this test records the *resolved output* of every style function, across every widget
//! status, in both colour schemes, as a committed fixture. Wrapping a widget must not change a
//! single byte of it. Where a screenshot says "something looks off", a failure here names the
//! component, the status and the scheme.
//!
//! **What it does not cover**: layout and spacing. If a wrapper restructures the widget tree
//! rather than restyling it, this stays green — so the reduced visual pass in `quickstart.md`
//! remains necessary.
//!
//! Regenerate deliberately (only when a change to appearance is *intended*, i.e. never in this
//! feature) with `UPDATE_STYLE_SNAPSHOT=1 cargo test -p micold-client style_snapshot`.
//!
//! Lives inside the crate rather than in `tests/`: T036 made the styling layer crate-internal, so
//! an integration test can no longer see the thing this asserts. That is the boundary working.
//!
//! **Regenerated once, for iced 0.13 → 0.14.** The renderer's style structs changed shape: the
//! container gained `snap`, the pick-list menu gained a `shadow`, the scrollable gained an
//! `auto_scroll` indicator, and `Rail.color` became `Rail.background: Option<Background>`. Every
//! recorded colour, border, radius and alpha survived unchanged — verified by normalising the
//! added fields away and diffing against the previous fixture, which left **zero** substantive
//! differences. Fields were added, none altered or removed. Had any value moved, that would have
//! been an appearance change to explain rather than a fixture to refresh.
//!
//! **Regenerated a second time, for feature 027 (T086).** Two lines moved, both
//! `text_input[disabled]`: its `placeholder` and `value` now carry `DISABLED_OPACITY`. This is an
//! appearance change and is the point of the change — a disabled field was previously drawn
//! identically to an editable one, so a resource limit the container runtime cannot enforce looked
//! like one you could type into. Nothing else in the fixture differs.

use std::fmt::Write as _;

use super::style;
use crate::features::notifications::NoticeLevel;
use iced::widget::{button, checkbox, container, scrollable, text_input};
use iced::Theme;
use micold_core::naming::ConventionalType;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, Roles};

const FIXTURE: &str = include_str!("../../../tests/fixtures/style_snapshot.txt");

/// Each `impl Fn` returned by the style layer is its own opaque type, so they are boxed behind
/// these aliases to be iterated as one list.
type ContainerStyleFn = Box<dyn Fn(&Theme) -> container::Style>;
type ButtonStyleFn = Box<dyn Fn(&Theme, button::Status) -> button::Style>;

/// Both schemes, named so a diff says which one drifted.
const SCHEMES: [(&str, ColorScheme); 2] =
    [("light", ColorScheme::Light), ("dark", ColorScheme::Dark)];

fn render_all() -> String {
    let mut out = String::new();
    for (scheme_name, scheme) in SCHEMES {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        let s = &mut out;

        writeln!(s, "# scheme: {scheme_name}").unwrap();

        // --- Bare colours ---------------------------------------------------
        writeln!(s, "separator = {:?}", style::separator(r)).unwrap();
        writeln!(
            s,
            "disabled_color(on_surface) = {:?}",
            style::disabled_color(r.on_surface)
        )
        .unwrap();

        // --- Container surfaces ---------------------------------------------
        let containers: Vec<(&str, ContainerStyleFn)> = vec![
            ("window_bg", Box::new(style::window_bg(r))),
            ("surface", Box::new(style::surface(r))),
            ("dialog", Box::new(style::dialog(r))),
            ("sidebar_surface", Box::new(style::sidebar_surface(r))),
            ("toolbar_surface", Box::new(style::toolbar_surface(r))),
            ("menu_surface", Box::new(style::menu_surface(r))),
            ("list_item", Box::new(style::list_item(r))),
        ];
        for (name, f) in &containers {
            writeln!(s, "container.{name} = {:?}", f(&theme)).unwrap();
        }

        // Notification carries a severity, so both levels are recorded.
        for (lvl_name, lvl) in [("info", NoticeLevel::Info), ("error", NoticeLevel::Error)] {
            let f = style::notification(r, lvl);
            writeln!(s, "container.notification[{lvl_name}] = {:?}", f(&theme)).unwrap();
        }

        // Chips are keyed by conventional-commit type, plus the issue tag.
        for &t in ConventionalType::ALL {
            let f = style::chip(r.tag_fill(t));
            writeln!(s, "container.chip[{t:?}] = {:?}", f(&theme)).unwrap();
        }
        writeln!(
            s,
            "container.chip[issue] = {:?}",
            style::chip(r.issue_tag().0)(&theme)
        )
        .unwrap();

        // --- Text ------------------------------------------------------------
        writeln!(s, "text.muted = {:?}", style::muted(r)(&theme)).unwrap();

        // --- Buttons: every variant x every status ---------------------------
        let button_statuses = [
            ("active", button::Status::Active),
            ("hovered", button::Status::Hovered),
            ("pressed", button::Status::Pressed),
            ("disabled", button::Status::Disabled),
        ];
        let buttons: Vec<(&str, ButtonStyleFn)> = vec![
            ("filled", Box::new(style::filled(r))),
            ("outlined", Box::new(style::outlined(r))),
            ("text", Box::new(style::text_button(r))),
            ("circular_icon", Box::new(style::circular_icon_button(r))),
        ];
        for (name, f) in &buttons {
            for (st_name, st) in button_statuses {
                writeln!(s, "button.{name}[{st_name}] = {:?}", f(&theme, st)).unwrap();
            }
        }

        // --- Text input: the filled field's input, which draws no chrome of its own ---
        let input = style::field_input(r);
        for (st_name, st) in [
            ("active", text_input::Status::Active),
            ("hovered", text_input::Status::Hovered),
            ("focused", text_input::Status::Focused { is_hovered: false }),
            ("disabled", text_input::Status::Disabled),
        ] {
            writeln!(s, "text_input[{st_name}] = {:?}", input(&theme, st)).unwrap();
        }

        // The select's three `pick_list` status poses and its `pick_list.menu` line were here, and
        // left with the widget (feature 022, contract §5). Nothing replaces them, and that is the
        // point: the select's state layer is `state_fill` — already recorded through the shared
        // state-layer checks — and its list is `menu_panel`, already recorded as `container.menu`.
        // Re-recording them under a select-shaped name would put one appearance in the fixture
        // twice, which is how two entries for one decision come to disagree.

        // --- Checkbox: status x checked x focused -----------------------------
        // `focused` is a third axis rather than a fourth status, because the rendering stack's
        // checkbox has no focused status to pose — it is supplied by the wrapper that gives the
        // control a keyboard at all (BUG-003).
        for focused in [false, true] {
            let cb = style::checkbox(r, focused);
            for checked in [false, true] {
                for (st_name, st) in [
                    (
                        "active",
                        checkbox::Status::Active {
                            is_checked: checked,
                        },
                    ),
                    (
                        "hovered",
                        checkbox::Status::Hovered {
                            is_checked: checked,
                        },
                    ),
                    (
                        "disabled",
                        checkbox::Status::Disabled {
                            is_checked: checked,
                        },
                    ),
                ] {
                    writeln!(
                        s,
                        "checkbox[{st_name},checked={checked},focused={focused}] = {:?}",
                        cb(&theme, st)
                    )
                    .unwrap();
                }
            }
        }

        // --- Scrollbar --------------------------------------------------------
        let sb = style::scrollbar(r);
        writeln!(
            s,
            "scrollable[active] = {:?}",
            sb(
                &theme,
                scrollable::Status::Active {
                    is_horizontal_scrollbar_disabled: false,
                    is_vertical_scrollbar_disabled: false,
                }
            )
        )
        .unwrap();
        for hovered in [false, true] {
            writeln!(
                s,
                "scrollable[hovered,v={hovered}] = {:?}",
                sb(
                    &theme,
                    scrollable::Status::Hovered {
                        is_horizontal_scrollbar_hovered: false,
                        is_vertical_scrollbar_hovered: hovered,
                        is_horizontal_scrollbar_disabled: false,
                        is_vertical_scrollbar_disabled: false,
                    }
                )
            )
            .unwrap();
            writeln!(
                s,
                "scrollable[dragged,v={hovered}] = {:?}",
                sb(
                    &theme,
                    scrollable::Status::Dragged {
                        is_horizontal_scrollbar_dragged: false,
                        is_vertical_scrollbar_dragged: hovered,
                        is_horizontal_scrollbar_disabled: false,
                        is_vertical_scrollbar_disabled: false,
                    }
                )
            )
            .unwrap();
        }

        writeln!(s).unwrap();
    }
    out
}

/// The parity gate. Wrapping a widget must not change what it resolves to.
#[test]
fn resolved_styles_match_the_recorded_baseline() {
    let current = render_all();

    if std::env::var_os("UPDATE_STYLE_SNAPSHOT").is_some() {
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/style_snapshot.txt"
            ),
            &current,
        )
        .expect("write snapshot fixture");
        return;
    }

    if current != FIXTURE {
        let diff: Vec<String> = FIXTURE
            .lines()
            .zip(current.lines())
            .filter(|(a, b)| a != b)
            .take(20)
            .map(|(a, b)| format!("  baseline: {a}\n  current:  {b}"))
            .collect();
        panic!(
            "Resolved styles drifted from the recorded baseline.\n\n\
             Feature 017 must not change any appearance (FR-005, FR-023) — a failure here means a \
             wrapper is not at parity with what it replaced.\n\n\
             First differing entries:\n{}\n\n\
             If (and only if) an appearance change is intended, regenerate with:\n  \
             UPDATE_STYLE_SNAPSHOT=1 cargo test -p micold-client style_snapshot",
            diff.join("\n---\n")
        );
    }
}

/// The snapshot is worthless if it silently covers nothing, so assert it is substantive and
/// actually spans both schemes.
#[test]
fn snapshot_covers_both_schemes_and_every_component() {
    let s = render_all();
    assert!(s.contains("# scheme: light"), "light scheme missing");
    assert!(s.contains("# scheme: dark"), "dark scheme missing");
    for probe in [
        "button.filled[pressed]",
        "button.outlined[disabled]",
        "button.text[hovered]",
        "button.circular_icon[active]",
        "text_input[focused]",
        "checkbox[hovered,checked=true,focused=false]",
        // The state the rendering stack's checkbox cannot express, and the library now can
        // (BUG-003). Probed by name so a refactor that dropped the focused axis would be caught
        // here rather than by a baseline that silently shrank.
        "checkbox[active,checked=false,focused=true]",
        "scrollable[dragged,v=true]",
        "container.dialog",
        "container.notification[error]",
        "text.muted",
    ] {
        assert!(
            s.contains(probe),
            "snapshot is missing coverage for `{probe}`"
        );
    }
    // A truncated fixture must fail loudly rather than pass by covering nothing.
    let entries = s.lines().filter(|l| l.contains(" = ")).count();
    assert!(
        entries >= 100,
        "snapshot covers only {entries} resolved styles — coverage regressed"
    );
    // Both schemes must contribute equally; a one-sided snapshot would hide scheme-specific drift.
    let per_scheme: Vec<usize> = s
        .split("# scheme: ")
        .skip(1)
        .map(|block| block.lines().filter(|l| l.contains(" = ")).count())
        .collect();
    assert_eq!(per_scheme.len(), 2, "expected exactly two scheme blocks");
    assert_eq!(
        per_scheme[0], per_scheme[1],
        "schemes cover different numbers of styles ({per_scheme:?}) — one is incomplete"
    );
}

/// Roles resolve differently per scheme; if this ever passes trivially the snapshot proves nothing.
#[test]
fn the_two_schemes_actually_differ() {
    let light: Roles = tokens::roles(ColorScheme::Light);
    let dark: Roles = tokens::roles(ColorScheme::Dark);
    assert_ne!(light.surface, dark.surface);
    assert_ne!(light.on_surface, dark.on_surface);
}
