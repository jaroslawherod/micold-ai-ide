//! Every writer of `active_session` goes through one function (feature 024, contract §3.0).
//!
//! The reveal is a consequence of `active_session` *changing*, not of any particular message being
//! handled: the row holding the current session is opened, the outgoing one is committed so it does
//! not snap shut, a stale suppression is dropped, and a scroll is armed. Spread that across the
//! arms that move the pointer and it is four things to remember at each of them.
//!
//! So there is one function, `State::set_current_session`, and this is the check that a future
//! writer cannot quietly skip it. It matters more than it looks: planning this feature found the
//! spec claiming a "restore at launch" trigger the application does not have, and found two writers
//! — a forgotten project and the daemon catalog dropping a dangling pointer — that no clause
//! covered. A rule stated as a list of call sites is a rule that goes out of date silently; a rule
//! stated as "any transition" needs something to enforce the *any*.
//!
//! # The one exemption
//!
//! `Message::Session(SessionMsg::Selected)` writes the field directly. The user clicked a row they could already
//! see, so revealing it would open nothing they had not opened and scroll a list they were reading
//! (FR-006). It is named here rather than inferred, so adding a second exemption is a deliberate
//! edit to this file.
//!
//! # Where the exemption lives after T062
//!
//! It used to be an arm of `State::update` in `app.rs`, and the second test below counted the
//! writes in that file. T062 moved every feature's arms into its own module, so the arm is now
//! `features/session::selected` and `app.rs` has no direct write at all.
//!
//! Rather than re-point the count at `features/session.rs` — where `set_current_session` also
//! writes the field, so a count would be 2 and would say nothing about *which* two — the test now
//! names the enclosing functions. That is the property the file was always after: `app.rs` writes
//! it nowhere, and inside the session feature exactly two functions do, each for a stated reason.

use std::fs;
use std::path::{Path, PathBuf};

/// The crate's own sources — the reducer and the binary both write this field.
fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                if let Ok(text) = fs::read_to_string(&path) {
                    out.push((path.display().to_string(), production_only(&text)));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut out,
    );
    out
}

/// A file's production code: everything before its own `#[cfg(test)]` module.
///
/// A test arranging a `State` writes fields directly by design — that is what arranging is. Holding
/// fixtures to the reducer's rule would only teach people to write fixtures through the reducer,
/// which makes them worse at saying what they are setting up.
fn production_only(text: &str) -> String {
    match text.split_once("\n#[cfg(test)]") {
        Some((production, _)) => production.to_string(),
        None => text.to_string(),
    }
}

/// A line assigning to `active_session` — `self.active_session = …` or `core.active_session = …`.
///
/// Comparisons (`==`, `!=`) and reads are not writes, so the check is for a single `=` that is not
/// part of one. Deliberately textual: the alternative is a full parse, and a writer this check
/// cannot see is a writer a reviewer would not see either.
fn is_a_write(line: &str) -> bool {
    let Some(rest) = line.split_once(".active_session") else {
        return false;
    };
    let after = rest.1.trim_start();
    after.starts_with('=') && !after.starts_with("==")
}

/// Where the field is allowed to be written directly, and why.
///
/// `app.rs` was on this list until T062, for the `SessionSelected` arm. The arm moved into the
/// session feature, so the root no longer needs an exemption — and dropping it puts the root back
/// under this test rather than leaving a permission nothing uses.
const EXEMPT: &[(&str, &str)] = &[(
    "features/session.rs",
    "`set_current_session`, the one function every other writer goes through, and `selected`, \
     which must not reveal (FR-006). Both checked below by name",
)];

#[test]
fn nothing_writes_the_current_session_pointer_outside_the_one_function() {
    let mut offenders = Vec::new();
    for (path, text) in sources() {
        if EXEMPT.iter().any(|(file, _)| path.ends_with(file)) {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            if is_a_write(line) {
                offenders.push(format!("{path}:{} — {}", n + 1, line.trim()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "these write `active_session` directly instead of calling `set_current_session`:\n  {}\n\n\
         A direct write moves the pointer without opening the row that holds the new session, \
         without committing the outgoing one (so it snaps shut, taking its siblings out of view), \
         and without clearing a suppression made against a session that is no longer current. If \
         the write genuinely must not reveal — as `SessionSelected` must not — add it to `EXEMPT` \
         with the reason, so the exemption is a decision rather than an omission.",
        offenders.join("\n  ")
    );
}

/// The function a line sits inside: the nearest `fn <name>(` at or above it.
fn enclosing_fn(lines: &[&str], at: usize) -> String {
    for line in lines[..=at].iter().rev() {
        let trimmed = line.trim_start();
        let after_vis = trimmed
            .strip_prefix("pub(crate) ")
            .or_else(|| trimmed.strip_prefix("pub "))
            .unwrap_or(trimmed);
        if let Some(rest) = after_vis.strip_prefix("fn ") {
            if let Some(name) = rest.split(['(', '<']).next() {
                return name.trim().to_string();
            }
        }
    }
    "<no enclosing fn>".to_string()
}

#[test]
fn the_reducers_only_direct_write_is_the_one_the_user_asked_for() {
    let src = |file: &str| {
        fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join(file),
        )
        .unwrap_or_else(|_| panic!("{file} is readable"))
    };

    let app = production_only(&src("app.rs"));
    let in_app: Vec<&str> = app.lines().filter(|line| is_a_write(line)).collect();
    assert!(
        in_app.is_empty(),
        "the root reducer routes and does not write `active_session` (feature 021, FR-002). \
         Found:\n  {}",
        in_app.join("\n  ")
    );

    let session = production_only(&src("features/session.rs"));
    let lines: Vec<&str> = session.lines().collect();
    let writers: Vec<String> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| is_a_write(line))
        .map(|(n, _)| enclosing_fn(&lines, n))
        .collect();

    assert_eq!(
        writers,
        ["set_current_session", "selected"],
        "exactly two functions in the session feature may write `active_session` directly: \
         `set_current_session`, which is the funnel, and `selected`, which is the click the user \
         made on a row already in front of them (FR-006). A third — or either of these losing its \
         write — is the shape this whole check exists to catch, because it looks entirely \
         ordinary in review."
    );
}
