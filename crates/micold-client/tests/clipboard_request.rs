//! Copying is a request a feature emits, not a call it makes (feature 021, T045 — FR-015a,
//! contract C1/C2/C3).
//!
//! # Why the clipboard is not a service capability
//!
//! Every other I/O concern in this codebase is a narrow trait the shell supplies (FR-015). The
//! clipboard cannot be, and the reason is the framework's: `iced::clipboard::write` returns a
//! `Task` to be handed back to the runtime, not a value. A synchronous port would have to block on
//! it. FR-015a sanctions the alternative for exactly this case — express the I/O as an explicit
//! effect request in the outcome vocabulary and let the shell interpret it — and this file holds
//! the three obligations that come with taking it.
//!
//! # The three obligations
//!
//! - **C1** — a feature never calls `iced::clipboard` directly. Held by
//!   [`nothing_outside_the_shell_reaches_the_clipboard`], which reads the source rather than
//!   trusting the current arrangement, because the temptation is one `use` away.
//! - **C2** — the request is assertable with zero real clipboard access. Held by every behaviour
//!   test here: they call [`selection::copy_request`] and compare an `Outcome`. Nothing in this
//!   file touches a window, a runtime or a system clipboard, which is also why it runs in CI on
//!   three platforms without one.
//! - **C3** — the shell's translation contains no decision logic. Held by
//!   [`the_shells_translation_decides_nothing`], which reads `interpret`'s body: one arm per
//!   variant, no branch that could have gone the other way.
//!
//! # What C2 is actually worth here
//!
//! "Zero real clipboard access" is easy to satisfy vacuously — a test that asserts nothing about
//! the content also touches no clipboard. So these assert the *decisions* the shell used to make
//! inline: no selection is not the same as an empty one, and an empty one is not the same as a
//! selection of blanks. Each was a line of shell code above the write before T045; each is now
//! somewhere a test can reach without a display.

use micold_client::features::Outcome;
use micold_client::selection::{copy_request, Anchor, SelectGranularity, Selection};
use micold_core::protocol::grid::LineId;

mod inventory;

use std::path::Path;

/// A line-text provider over a fixed list of lines, starting at `LineId(0)`.
fn lines<'a>(rows: &'a [&'a str]) -> impl Fn(LineId) -> Option<String> + 'a {
    move |id| {
        usize::try_from(id.0)
            .ok()
            .and_then(|i| rows.get(i))
            .map(|s| s.to_string())
    }
}

/// A character-granularity selection over the inclusive cell range `(l0, c0)..=(l1, c1)`.
fn select(rows: &[&str], l0: i64, c0: u16, l1: i64, c1: u16) -> Selection {
    let provider = lines(rows);
    let mut sel = Selection::start(
        Anchor::new(LineId(l0), c0),
        SelectGranularity::Char,
        &provider,
    );
    sel.update(Anchor::new(LineId(l1), c1), &provider);
    sel
}

#[test]
fn a_selection_emits_the_write_it_wants() {
    let rows = ["hello world"];
    let sel = select(&rows, 0, 0, 0, 4);

    assert_eq!(
        copy_request(Some(&sel), lines(&rows)),
        Some(Outcome::ClipboardWrite("hello".to_string())),
        "the feature's answer to \"copy\" is the request, not the write"
    );
}

#[test]
fn a_multi_line_selection_carries_the_joined_text() {
    // The request is whatever the selection resolves to, unedited by the shell — including the
    // newline, which is the one part a shell tempted to "tidy" the string would drop.
    let rows = ["first", "second"];
    let sel = select(&rows, 0, 0, 1, 5);

    assert_eq!(
        copy_request(Some(&sel), lines(&rows)),
        Some(Outcome::ClipboardWrite("first\nsecond".to_string()))
    );
}

#[test]
fn no_selection_asks_for_nothing() {
    // Distinct from asking for an empty write. `iced::clipboard::write("")` clears the system
    // clipboard; right-clicking with nothing selected must not.
    let rows = ["hello world"];
    assert_eq!(copy_request(None, lines(&rows)), None);
}

#[test]
fn a_selection_that_resolves_to_nothing_asks_for_nothing() {
    // The other way to reach an empty string: a selection over blank cells. `Selection::text`
    // trims trailing whitespace per line, so this comes back empty even though something *is*
    // selected — and it must be as silent as no selection at all, for the same reason.
    let rows = ["     "];
    let sel = select(&rows, 0, 0, 0, 4);
    assert_eq!(sel.text(lines(&rows)), "", "precondition for this test");

    assert_eq!(copy_request(Some(&sel), lines(&rows)), None);
}

#[test]
fn a_selection_the_provider_cannot_resolve_asks_for_nothing() {
    // What the shell hands over when the displayed session has no cached grid yet. The selection
    // outlives the lines it points at — it is anchored to absolute `LineId`s precisely so it can —
    // so the feature, not the shell, has to be the one that shrugs.
    let rows = ["hello world"];
    let sel = select(&rows, 0, 0, 0, 4);

    assert_eq!(copy_request(Some(&sel), |_| None), None);
}

/// Client sources outside the shell, with comments stripped.
fn non_shell_sources() -> Vec<(String, String)> {
    inventory::sources_under(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
        .into_iter()
        .filter(|(path, _)| path != "main.rs" && !path.starts_with("shell/"))
        .map(|(path, text)| (path, inventory::code_only(&text)))
        .collect()
}

#[test]
fn nothing_outside_the_shell_reaches_the_clipboard() {
    // C1. `shell/` does not exist until T050; naming it now means this guard answers the same
    // question after the split, rather than needing to be relaxed at the moment it matters.
    //
    // The two operations, not the module: `ui/material/` legitimately passes an
    // `iced::advanced::clipboard::Null` into widget event plumbing, which is a type it is handed,
    // not a system clipboard it reaches for.
    let offenders: Vec<String> = non_shell_sources()
        .into_iter()
        .flat_map(|(path, code)| {
            ["clipboard::write(", "clipboard::read("]
                .into_iter()
                .filter(|op| code.contains(op))
                .map(|op| format!("`{path}` calls `{op}…)`"))
                .collect::<Vec<_>>()
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "clipboard access escaped the shell (FR-017, contract C1):\n  - {}\n\nEmit \
         `Outcome::ClipboardWrite` instead and let the shell perform it — a feature that calls the \
         framework directly cannot be tested without one.",
        offenders.join("\n  - ")
    );
}

#[test]
fn the_guard_is_reading_the_shell_it_exempts() {
    // The vacuity check for the one above. It passes trivially if the scan reads nothing, if
    // `main.rs` stopped being the shell, or if the operation spellings drifted — and the sole
    // evidence that any of the three is false is that the exempted file still matches.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let all = inventory::sources_under(&src);
    // Feature 021 T055 moved the write out of `main.rs` and into `shell/clipboard.rs`, which is
    // the move the failure message below anticipated. The exemption follows the code: what this
    // asserts is that *somewhere the guard exempts* still performs the write, or the guard above
    // is passing because nothing does.
    let shell = all
        .get("shell/clipboard.rs")
        .expect("shell/clipboard.rs is where the shell reaches the clipboard");

    assert!(
        shell.contains("clipboard::write("),
        "the shell no longer performs the write this guard exempts it for; either it moved to \
         `shell/` — update both tests together — or the spelling this scan looks for is stale and \
         `nothing_outside_the_shell_reaches_the_clipboard` is now vacuous"
    );
    assert!(
        all.len() > 1,
        "the scan found only one source file; it is not reading the tree"
    );
}

#[test]
fn the_shells_translation_decides_nothing() {
    // C3. Read from the source, because the property is about the *shape* of the translation, and
    // a body that grew an `if` would still compile and still pass every behaviour test above.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shell/clipboard.rs");
    let code =
        inventory::code_only(&std::fs::read_to_string(&src).expect("read shell/clipboard.rs"));
    let at = code
        .find("fn interpret(")
        .expect("the shell interprets effect requests in `interpret`");
    let body = &code[at..at + code[at..].find("\n}").expect("`interpret` body") + 2];

    for branch in ["if ", "else", "while ", "unwrap_or", "is_empty"] {
        assert!(
            !body.contains(branch),
            "`interpret` contains `{branch}` — the shell is deciding something. Whether an effect \
             should happen belongs to the feature that emits the request; translating it is one \
             arm per variant.\n\n{body}"
        );
    }

    let arms = body.matches("Outcome::").count();
    assert!(
        arms >= 1,
        "no `Outcome::` arm found in `interpret`; this test is reading the wrong function\n\n{body}"
    );
}
