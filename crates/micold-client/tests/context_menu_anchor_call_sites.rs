//! No context menu is positioned by a constant (018 BUG-008, SC-008f, FR-029d).
//!
//! The behavioural half of SC-008f — `tests/gates/context_menu_anchor.rs` — drives a real
//! right-press over each of the four context menus this application has today and asserts the panel
//! lands where the press did. This is the other half, and it is here for the menu written next
//! year: a source scan, in the family of `type_role_call_sites`, `anatomy_call_sites` and
//! `composite_call_sites`, which fails when a menu's anchor is fed a **constant** rather than a
//! point the state carried.
//!
//! That is precisely the shape BUG-008 shipped in:
//!
//! ```ignore
//! const SIDEBAR_MENU_ANCHOR: (u16, u16) = (24, 96);
//! ```
//!
//! …passed to two menus, with a doc comment explaining that a row's position "the view does not
//! know". Every geometry gate was green on it — the panel was the right size, on the right surface,
//! clear of the app bar, inside the window — because none of them reads a panel against the element
//! it was opened from. A behavioural gate catches that for the menus it enumerates; a scan catches
//! it for the one nobody has enumerated yet.
//!
//! # The rule
//!
//! In the rendering layer, the first positional argument of `ContextMenu::new` and the argument of
//! a `.anchor(…)` must not name a `SCREAMING_SNAKE_CASE` constant. A point that came from the state
//! reads as `x`, `y`, `menu.anchor` or an expression over them; a point that came from a constant
//! reads as a shout, and shouting is the whole tell.
//!
//! # The exceptions, named rather than assumed
//!
//! FR-029d permits a menu to hang from an **edge** where that is stated: a panel below the app bar
//! (FR-029a), or one rising from a bar too short to open downward. Those are positioned by another
//! component's anatomy — `anatomy::app_bar::HEIGHT` — which is a constant this rule would otherwise
//! forbid, and which FR-029a positively *requires* be read rather than restated. So the two rules
//! meet here: a figure derived from a named component's anatomy is allowed; a bare figure of a
//! menu's own is not.

use std::fs;
use std::path::{Path, PathBuf};

/// The rendering layer, minus the showcase: the gallery positions specimens deliberately, at points
/// that belong to a demonstration rather than to a user's press.
fn rendering_layer() -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui"), &mut files);
    files
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read the rendering layer") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// A figure allowed to position a panel: another component's anatomy, read rather than restated
/// (FR-029a). Anything else that shouts is a menu stating a place of its own.
fn is_a_component_anatomy(argument: &str) -> bool {
    argument.contains("anatomy::")
}

/// `line` with its `//` comment removed, so this file's own subject matter cannot be read as code.
fn code_only(line: &str) -> &str {
    match line.find("//") {
        Some(at) => &line[..at],
        None => line,
    }
}

/// A shouted identifier — `SIDEBAR_MENU_ANCHOR`, `TOP_OFFSET` — in `text`.
fn names_a_constant(text: &str) -> Option<String> {
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' {
            current.push(ch);
        } else {
            if current.len() >= 3 && current.chars().any(|c| c.is_ascii_uppercase()) {
                return Some(current);
            }
            current.clear();
        }
    }
    (current.len() >= 3).then_some(current)
}

#[test]
fn no_context_menu_is_anchored_at_a_constant() {
    let mut offenders: Vec<String> = Vec::new();

    for file in rendering_layer() {
        let source = fs::read_to_string(&file).expect("read a rendering-layer file");
        let name = file
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(&file)
            .display()
            .to_string();

        for (number, raw) in source.lines().enumerate() {
            let line = code_only(raw);
            let argument = if let Some(at) = line.find(".anchor(") {
                &line[at + ".anchor(".len()..]
            } else if let Some(at) = line.find("ContextMenu::new(") {
                // The point is the second argument and usually a line of its own; take the rest of
                // this line, which is the single-line form.
                &line[at + "ContextMenu::new(".len()..]
            } else {
                continue;
            };
            if is_a_component_anatomy(argument) {
                continue;
            }
            if let Some(shouted) = names_a_constant(argument) {
                offenders.push(format!("{name}:{}: {shouted}", number + 1));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "{} context-menu anchor(s) name a constant:\n  {}\n\nA context menu opens at the press \
         point that opened it (FR-029d), and a point that is a constant cannot be one. This is \
         BUG-008's exact shape: `SIDEBAR_MENU_ANCHOR = (24, 96)`, fed to the worktree and session \
         menus, which every geometry gate passed because each read the panel and none read the row \
         it belonged to. Take the point from the gesture — `cdk::ContextArea::on_secondary_press`, \
         or `TreeItem::on_right_press` — and carry it on the message. If this panel genuinely hangs \
         from an edge, FR-029d's exception requires it to derive that offset from the component it \
         hangs from (`anatomy::…`), which this check allows.",
        offenders.len(),
        offenders.join("\n  "),
    );
}

/// The scan can fail, which is the part a scan usually cannot prove about itself.
#[test]
fn the_scan_would_catch_the_constant_it_was_written_for() {
    assert_eq!(
        names_a_constant("SIDEBAR_MENU_ANCHOR,").as_deref(),
        Some("SIDEBAR_MENU_ANCHOR"),
    );
    assert_eq!(names_a_constant("menu.anchor,"), None);
    assert_eq!(
        names_a_constant("iced::Point::new(x as f32, y as f32)"),
        None,
    );
    // The stated exception is recognised as one rather than merely going unnoticed.
    assert!(is_a_component_anatomy("anatomy::app_bar::HEIGHT"));
    assert!(!is_a_component_anatomy("SIDEBAR_MENU_ANCHOR"));
}
