//! Every input in the application reports its focus (BUG-003 — FR-031, FR-035).
//!
//! # The gap this closes
//!
//! `FormField::active` is *supplied, not observed*, and deliberately: the state that thickens a
//! filled field's active indicator is focus for a text input and **open** for a picker (§7.7), so
//! the wrapper is told which rather than assuming. The design is sound. Nothing supplied it.
//!
//! For two features — 018's active indicator and 022's focus state layer — not one `TextField` in
//! the application passed `.active(…)`, so every field was drawn permanently at rest. Clicking an
//! empty field put a caret directly in front of its own label. The component honoured the flag, the
//! anatomy gates proved it honoured the flag, and the flag was never set.
//!
//! Every one of those gates was green, and structurally had to be: they build a field *with the
//! flag set by the test*. A component doing what it is told is not evidence that anything tells it.
//! The missing check is this one, and it is the same shape as `anatomy_call_sites.rs` — a figure
//! that reaches no call site, here a parameter that reaches no call site.
//!
//! # What is checked
//!
//! Every `TextField::new(` and `Checkbox::new(` in a rendering module is followed, within its own
//! expression, by `.track_focus(`. That helper is where the two halves are joined (`ui/focus.rs`);
//! requiring the pair rather than either half is the point, since an input that reports focus nobody
//! keeps and one told a fact nobody reports are the same input permanently at rest.
//!
//! The checkbox is held to the same rule from the day it *could* be — FR-035 recorded it as beyond
//! reach while the rendering stack's checkbox had no focus at all, and it now has one.
//!
//! # Why source text rather than pixels
//!
//! This is the cheapest possible check and it is the one that would have caught the bug: BUG-003
//! was *found by a grep*. Driving each of the four dialogs through a click and asserting the
//! resulting chrome would be a truer test and a far larger one, and it would still be checking that
//! a call site is joined up — which is a property of the source.

use std::fs;
use std::path::{Path, PathBuf};

/// The library's own module is exempt: `material/` is where `TextField` is *defined*, and
/// `typeahead.rs` composes one for a picker whose indicator follows **open** rather than focus
/// (§7.7, FR-013) — the case `active` exists as a parameter for.
///
/// The showcase is exempt for the opposite reason: a gallery *poses* states on purpose, including
/// the focused one, which is the only place `.active(true)` legitimately appears without a keyboard
/// anywhere near it.
fn rendering_files() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "material") {
                    continue;
                }
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                out.push((name, fs::read_to_string(&path).expect("read source")));
            }
        }
    }

    let mut out = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("ui"),
        &mut out,
    );
    out
}

/// The builder expression starting at `from`: everything up to the `;` that ends the statement.
///
/// Crude on purpose. A `TextField` is built as one chained expression at every call site in this
/// application, and a check that reads the source has to stop somewhere; stopping at the statement
/// is both the smallest span that contains the whole chain and the largest that cannot reach the
/// next field.
///
/// Crude, but not *blind*: a `;` inside a string literal does not end a statement, and supporting
/// text is prose, where a semicolon is ordinary punctuation. Reading one as the end of the chain
/// truncated the span before `.track_focus(` and reported two correctly wired fields as unwired —
/// a gate failing on the English in a caption is worse than one that skips string literals.
fn expression(source: &str, from: usize) -> &str {
    let rest = &source[from..];
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in rest.char_indices() {
        match ch {
            _ if escaped => escaped = false,
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            ';' if !in_string => return &rest[..offset],
            _ => {}
        }
    }
    rest
}

/// The constructors an input is built through. One list, so adding a control that can hold the
/// keyboard is a one-line change here rather than a rule someone has to remember to write.
const INPUTS: [&str; 2] = ["TextField::new(", "Checkbox::new("];

#[test]
fn every_input_in_the_application_reports_its_focus() {
    let mut unwired: Vec<String> = Vec::new();

    for (name, source) in rendering_files() {
        for constructor in INPUTS {
            for (offset, _) in source.match_indices(constructor) {
                if !expression(&source, offset).contains(".track_focus(") {
                    let line = source[..offset].lines().count();
                    unwired.push(format!("{name}:{line} ({constructor})"));
                }
            }
        }
    }

    assert!(
        unwired.is_empty(),
        "these inputs do not report their focus, so nothing they do on focus can ever happen — no \
         floated label, no thickened indicator, no focus state layer (BUG-003). Join each to the \
         application's focus state with `.track_focus(FieldId::…, focused)`: {unwired:?}",
    );
}

/// The helper the rule above is written in terms of. If it is renamed or removed, the rule silently
/// stops meaning anything — a `.contains` against a string that appears nowhere would fail loudly,
/// but a rule spelled against a helper that no longer joins both halves would pass while meaning
/// nothing at all.
#[test]
fn the_helper_the_rule_names_still_joins_both_halves() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("ui")
            .join("focus.rs"),
    )
    .expect("ui/focus.rs is where a field is joined to the application's focus state");

    assert!(
        source.contains("fn track_focus("),
        "the gate above is written in terms of `track_focus`; it must exist",
    );
    for half in [".active(", ".focused(", ".on_focus_change("] {
        assert!(
            source.contains(half),
            "`track_focus` must set {half} — a field that only reports focus, or is only told it, \
             is BUG-003 with an extra step",
        );
    }
}

/// A `TextField` call site outside the rendering layer would slip past [`rendering_files`] and its
/// directory walk without anyone noticing. This is the fixture's own smoke test: the sweep must
/// actually be reaching the surfaces it is supposed to police.
///
/// `ui/settings_form.rs` was on this list until feature 027 turned Settings into a sectioned view;
/// its fields live in `ui/settings/` now, and the three sections that own one are named here so the
/// migration cannot quietly cost the sweep its reach.
#[test]
fn the_sweep_reaches_the_application_dialogs() {
    let found: Vec<PathBuf> = rendering_files()
        .into_iter()
        .filter(|(_, source)| INPUTS.iter().any(|c| source.contains(c)))
        .map(|(name, _)| PathBuf::from(name))
        .collect();

    for expected in [
        "ui/rename.rs",
        "ui/worktree_rename.rs",
        "ui/worktree_form.rs",
        "ui/settings/terminal.rs",
        "ui/settings/environment.rs",
        "ui/settings/daemon.rs",
        "ui/confirm_delete.rs",
    ] {
        assert!(
            found.iter().any(|p| p == Path::new(expected)),
            "{expected} builds an input and the sweep did not see it — found {found:?}",
        );
    }
}

/// The scanner's own case. A semicolon inside a caption is punctuation, and reading it as the end
/// of the builder chain is how this gate once failed on two fields that were wired correctly.
#[test]
fn a_semicolon_inside_a_string_does_not_end_the_expression() {
    let source = r#"    let f = TextField::new("", v, r)
        .supporting("Run in a shell; its variables reach every session")
        .track_focus(FieldId::X, focused);
    let g = TextField::new("", w, r);
"#;
    let offset = source
        .find("TextField::new(")
        .expect("the fixture builds a field");

    let span = expression(source, offset);
    assert!(
        span.contains(".track_focus("),
        "the span stopped inside the caption: {span:?}",
    );
    assert!(
        !span.contains("let g"),
        "the span ran past its own statement and into the next field: {span:?}",
    );
}

// ---------------------------------------------------------------------------------------------
// And something has to *move* it (feature 027, FR-030, T075)
// ---------------------------------------------------------------------------------------------

/// A keyboard event for `key`, with `modifiers`.
fn pressed(key: iced::keyboard::key::Named, modifiers: iced::keyboard::Modifiers) -> KeyEvent {
    KeyEvent::KeyPressed {
        key: iced::keyboard::Key::Named(key),
        modified_key: iced::keyboard::Key::Named(key),
        physical_key: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        location: iced::keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    }
}

use iced::keyboard::Event as KeyEvent;
use micold_client::app::Message;

/// Tab forwards, Shift+Tab back, and nothing for any other key.
///
/// The rule the tests above enforce — every input joined to `focused_field` — describes a focus
/// that is *reported and drawn*. It says nothing about the focus ever moving, and for two features
/// it did not: the application never issued iced's traversal operation, so the keyboard reached
/// exactly the one control a pointer had last clicked and Tab did nothing at all. Found by pressing
/// Tab (T075); no assertion about focus *order* could have caught it, because the order was right.
#[test]
fn tab_asks_the_keyboards_focus_to_move() {
    use iced::keyboard::key::Named;
    use iced::keyboard::Modifiers;

    assert_eq!(
        micold_client::ui::focus_move_message(pressed(Named::Tab, Modifiers::empty())),
        Some(Message::FocusMoved { forward: true })
    );
    assert_eq!(
        micold_client::ui::focus_move_message(pressed(Named::Tab, Modifiers::SHIFT)),
        Some(Message::FocusMoved { forward: false })
    );
    for other in [Named::Escape, Named::Enter, Named::ArrowDown] {
        assert_eq!(
            micold_client::ui::focus_move_message(pressed(other, Modifiers::empty())),
            None,
            "{other:?} was read as a request to move the focus"
        );
    }
}

/// The message reaches the operation that moves the focus.
///
/// A source scan because this is the half that cannot be reached from a test: `update` is a pure
/// reducer and focus lives in the widget tree, so the traversal is issued by the binary's
/// `update_inner` and needs a running renderer to observe. What went wrong here was never subtle
/// behaviour — it was that no line of the application mentioned traversal at all.
#[test]
fn the_focus_message_reaches_the_traversal_operation() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
    let src =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let at = src
        .find("Message::FocusMoved")
        .expect("nothing in `main.rs` handles `Message::FocusMoved`, so Tab moves nothing");
    // The arm itself, not `expression`'s semicolon rule: this arm's body is a bare `if`, so the
    // first semicolon after it is somewhere in a later arm and the scan would read half the
    // reducer as evidence.
    let rest = &src[at..];
    let arm = rest
        .find("\n        Message::")
        .map_or(rest, |end| &rest[..end]);
    for op in ["focus_next()", "focus_previous()"] {
        assert!(
            arm.contains(op),
            "the `Message::FocusMoved` arm does not call `{op}`; Tab (or Shift+Tab) is heard and \
             then dropped:\n{arm}"
        );
    }
}
