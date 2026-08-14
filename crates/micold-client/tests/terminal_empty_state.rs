//! A structural gate keeping the terminal's empty state honest (feature 025, BUG-001, T036).
//!
//! # Why a source-level test
//!
//! `ui/terminal.rs`'s unit tests drive `empty_terminal_message` directly and pin what it answers
//! for every lifecycle. What they cannot see is whether the pane still *asks* it. Reverting the
//! `grid: None` arm to a literal `"Starting…"` — the exact line this bugfix replaced — leaves all
//! three of them green, because the function they test would still be right and simply unused.
//!
//! Asserting on the rendered element instead is not available: iced builds an `Element` tree with
//! no text-node accessor, and the pane's other empty state has never been assertable either. So
//! this follows `tests/terminal_bar_stability.rs` and `tests/showcase_glue.rs`, under Principle I's
//! GUI-wiring exception: *a precondition nobody checks is a precondition nobody keeps.*
//!
//! # What it guards
//!
//! The empty state's own literal may appear in exactly one place in `ui/terminal.rs` — inside
//! `empty_terminal_message`, which is where the decision is made. Anywhere else it is a second
//! answer to a question that already has one, and a second answer is how the body and the bar came
//! to contradict each other in the first place: the bar read `attached_process_restartable`, the
//! body assumed a cause, and nothing made them agree.
//!
//! The *word* is not forbidden, only the literal. `session_status` maps the `Starting` lifecycle to
//! a lowercase `"starting…"` for the bar, and the enum variant has to be spelled somewhere; a rule
//! broad enough to catch those would be a rule nobody could keep.
//!
//! This deliberately does not check the message's wording. That is `empty_terminal_message`'s own
//! tests' job, and pinning prose in two places makes the prose unchangeable rather than correct.

use std::fs;
use std::path::Path;

/// Strips `//` line comments and `/* */` blocks, so the doc comments in `ui/terminal.rs` — which
/// discuss this bug at length, quoting the string — cannot read as violations. Same helper as
/// `terminal_bar_stability.rs`, for the same reason.
fn code_only(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_block = false;
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                in_block = true;
            }
            _ => out.push(c),
        }
    }
    out
}

/// The pane source, comments stripped.
fn terminal_code() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("ui")
        .join("terminal.rs");
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    code_only(&src)
}

/// The name of the one function allowed to decide what an empty terminal says.
const DECIDER: &str = "fn empty_terminal_message";

/// The empty state's own literal, capitalised and with its ellipsis — deliberately not the bare
/// word. `session_status` maps `SessionLifecycle::Starting` to a lowercase `"starting…"` for the
/// bar, and that is a different sentence about a different thing: it reports a state, where the
/// empty state explains an absence. Forbidding the word outright would forbid the bar from having
/// a vocabulary, and the enum variant from having a name.
const EMPTY_STATE_LITERAL: &str = "\"Starting…\"";

#[test]
fn only_one_place_decides_what_an_empty_terminal_says() {
    let src = terminal_code();
    // Only the shipping code. The unit tests below `mod tests` name every lifecycle and assert on
    // the wording — that is their job, and it is not a second decision.
    let lines: Vec<&str> = src
        .lines()
        .take_while(|l| !l.trim_start().starts_with("mod tests"))
        .collect();

    let start = lines
        .iter()
        .position(|l| l.contains(DECIDER))
        .expect("empty_terminal_message must exist — it is where FR-014 is decided");
    // The function body ends at the next line that closes a top-level item.
    let end = lines[start..]
        .iter()
        .position(|l| *l == "}")
        .map(|offset| start + offset)
        .expect("empty_terminal_message must be a closed item");

    let offenders: Vec<String> = lines
        .iter()
        .enumerate()
        .filter(|(i, line)| line.contains(EMPTY_STATE_LITERAL) && !(*i >= start && *i <= end))
        .map(|(i, line)| format!("  line {}: {}", i + 1, line.trim()))
        .collect();

    assert!(
        offenders.is_empty(),
        "the empty state's literal appears in ui/terminal.rs outside `empty_terminal_message`:\n{}\n\n\
         That function is the single place that decides what an empty terminal says, and it \
         answers from `attached_process_restartable` so the body and the bar cannot disagree. A \
         literal elsewhere is a second answer, which is exactly the shape of BUG-001: a restored \
         session with no process was told it was starting, while the bar beside it said \
         `interrupted` and offered `restart` (feature 025 FR-014, contract §4.3).",
        offenders.join("\n")
    );
}

#[test]
fn the_pane_asks_the_decider_rather_than_naming_a_state_itself() {
    let src = terminal_code();
    assert!(
        src.contains("empty_terminal_message(state, active)"),
        "the pane's `grid: None` arm must call `empty_terminal_message`. Without the call the \
         unit tests in ui/terminal.rs still pass — they drive the function directly — while the \
         screen goes back to asserting a cause it cannot know (BUG-001)."
    );
}
