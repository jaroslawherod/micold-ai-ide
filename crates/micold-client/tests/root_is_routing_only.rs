//! The root reducer routes; it does not decide (feature 021, T061 — FR-002).
//!
//! # What "routing only" means here, precisely
//!
//! An arm of `State::update` is **routing** when it hands the message on without answering
//! anything itself: an empty body, or calls, and nothing else. It is **deciding** when it contains
//! control flow (`if`, `match`, `while`, `for`, `else`), a comparison or boolean operator, or an
//! assignment into the state. Those are the shapes a feature's rule takes, and FR-002 wants them in
//! the feature rather than at the junction that dispatches to it.
//!
//! Under Tier 1 a feature's operations are `impl State` methods living in its own module, so
//! `self.set_current_session(id)` is already a routing call — the decision is in
//! `features/session.rs`, which is where it belongs. After T062 the same arm becomes a call into a
//! reducer module. The classifier does not need to tell those two apart, and deliberately does not
//! try: both are "a call", and the property FR-002 asks about is what the *root* still answers.
//!
//! # This is a burn-down counter, not a line in the sand
//!
//! 93 of 110 arms decided something when this was written. Landing this test red is not an option
//! — SC-009 requires every commit to build and pass — and asserting "no more than today" would let
//! the number sit where it is forever. So [`ROOT_DECISION_ARMS`] is an exact count: T062 lowered it
//! to 3 as each feature's arms moved out.
//!
//! An exact count also catches the other direction, which is the one FR-002 is really about: a new
//! feature adding one more decision to the root fails this test the day it lands, not at some
//! review months later.
//!
//! # The floor is 0, and T063 is why
//!
//! T062 left this at 3: `FieldFocusChanged`, `CursorMoved` and `WindowResized` wrote `focused_field`,
//! `cursor` and `window_size`, which no feature owned. That was recorded as an honest floor rather
//! than a target, because moving them was not a matter of finding the right feature — it was a
//! matter of deciding whether a *window* feature should exist.
//!
//! T063 decided it should. `features/window.rs` owns those three fields and the `FieldId` type,
//! and the count reaches 0. The reasoning is in that module's header; the short version is that a
//! field the root still decides about is a feature nobody has named, and FR-001's own precedent —
//! T031 creating `features/help.rs` for the homeless overflow menu — says to name it.
//!
//! # What this cannot see
//!
//! - **A decision hidden inside a call.** An arm reading `self.do_the_thing()` is routing by this
//!   rule wherever `do_the_thing` happens to live, including `app.rs` itself. Tier 3 moves those
//!   bodies into feature reducers; until then a root helper called from one arm is counted as
//!   routing. `tests/feature_write_isolation.rs` is what watches where those bodies write.
//! - **`update_inner`**, the shell's reducer in `main.rs`. FR-004a splits a feature's effectful
//!   arms into shell modules by external system, which Phase 5 did; this file is about the pure
//!   reducer only.

use std::fs;
use std::path::{Path, PathBuf};

/// How many arms of the root reducer still decide something.
///
/// **0. The root routes and decides nothing.** Any increase means a feature put a rule back into
/// the root, which is exactly what FR-002 forbids and what this test exists to catch on the day it
/// lands rather than at a review months later.
const ROOT_DECISION_ARMS: usize = 0;

fn app_rs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app.rs")
}

/// Strips comments and string literals, so prose about this rule cannot trip it.
fn code_only(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let (mut in_block, mut in_line, mut in_str) = (false, false, false);
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        if in_line {
            if c == '\n' {
                in_line = false;
                out.push('\n');
            }
            continue;
        }
        if in_str {
            if c == '\\' {
                chars.next();
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                continue;
            }
            '/' if chars.peek() == Some(&'/') => {
                in_line = true;
                continue;
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                in_block = true;
                continue;
            }
            _ => {}
        }
        out.push(c);
    }
    out
}

/// The body of the root reducer's `match message { … }`.
fn match_body(src: &str) -> String {
    let head = "pub fn update(&mut self, message: Message) {";
    let at = src
        .find(head)
        .expect("`State::update` not found — has the root reducer been renamed?");
    let open = src[at..]
        .find("match message {")
        .expect("`State::update` no longer opens with `match message`")
        + at
        + "match message {".len();
    let mut depth = 1usize;
    for (i, c) in src[open..].char_indices() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return src[open..open + i].to_string();
            }
        }
    }
    panic!("unbalanced braces in the root reducer");
}

/// Each arm of the match, as `(pattern, body)`.
fn arms(body: &str) -> Vec<(String, String)> {
    let b: Vec<char> = body.chars().collect();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < n {
        // The pattern runs to the first `=>` at depth zero.
        let start = i;
        let mut depth = 0i32;
        let mut arrow = None;
        while i < n {
            match b[i] {
                '{' | '(' | '[' => depth += 1,
                '}' | ')' | ']' => depth -= 1,
                '=' if depth == 0 && i + 1 < n && b[i + 1] == '>' => {
                    arrow = Some(i);
                    i += 2;
                    break;
                }
                _ => {}
            }
            i += 1;
        }
        let Some(arrow) = arrow else { break };
        let pattern: String = b[start..arrow].iter().collect();
        while i < n && b[i].is_whitespace() {
            i += 1;
        }
        let (body_start, body_end) = if i < n && b[i] == '{' {
            depth = 1;
            i += 1;
            let body_start = i;
            while i < n && depth > 0 {
                match b[i] {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            (body_start, i - 1)
        } else {
            let body_start = i;
            depth = 0;
            while i < n {
                match b[i] {
                    '{' | '(' | '[' => depth += 1,
                    '}' | ')' | ']' => depth -= 1,
                    ',' if depth == 0 => break,
                    _ => {}
                }
                i += 1;
            }
            (body_start, i)
        };
        if i < n && b[i] == ',' {
            i += 1;
        }
        out.push((
            pattern.trim().to_string(),
            b[body_start..body_end].iter().collect(),
        ));
    }
    out
}

/// Whether an arm hands the message on without answering anything itself.
fn is_routing(body: &str) -> bool {
    let t = body.trim();
    if t.is_empty() {
        return true;
    }
    for keyword in ["if ", "match ", "while ", "for ", "else"] {
        if t.contains(keyword) {
            return false;
        }
    }
    for operator in ["==", "!=", "&&", "||"] {
        if t.contains(operator) {
            return false;
        }
    }
    !assigns_into_state(t)
}

/// `self.field = …`, `self.a.b += …` — an assignment whose target is the state.
fn assigns_into_state(body: &str) -> bool {
    let bytes = body.as_bytes();
    let mut from = 0usize;
    while let Some(at) = body[from..].find("self.") {
        let start = from + at;
        from = start + "self.".len();
        if start > 0 {
            let prev = bytes[start - 1] as char;
            if prev.is_alphanumeric() || prev == '_' {
                continue;
            }
        }
        // Walk the path: identifiers joined by dots.
        let mut i = from;
        loop {
            let before = i;
            while i < body.len() && {
                let c = body.as_bytes()[i] as char;
                c.is_alphanumeric() || c == '_'
            } {
                i += 1;
            }
            if i == before {
                break;
            }
            if i < body.len() && body.as_bytes()[i] == b'.' {
                i += 1;
                continue;
            }
            break;
        }
        let tail = body[i..].trim_start();
        if tail.starts_with("+=") || tail.starts_with("-=") {
            return true;
        }
        if let Some(rest) = tail.strip_prefix('=') {
            if !rest.starts_with('=') {
                return true;
            }
        }
    }
    false
}

/// The pattern's leading variant name, for a readable failure list.
fn label(pattern: &str) -> String {
    pattern
        .split(['(', '{', '|'])
        .next()
        .unwrap_or(pattern)
        .trim()
        .trim_start_matches("Message::")
        .to_string()
}

fn classified() -> (Vec<String>, Vec<String>) {
    let src = code_only(&fs::read_to_string(app_rs()).expect("read app.rs"));
    let mut routing = Vec::new();
    let mut deciding = Vec::new();
    for (pattern, body) in arms(&match_body(&src)) {
        if is_routing(&body) {
            routing.push(label(&pattern));
        } else {
            deciding.push(label(&pattern));
        }
    }
    (routing, deciding)
}

#[test]
fn the_root_reducer_decides_no_more_than_it_did() {
    let (_, deciding) = classified();
    assert_eq!(
        deciding.len(),
        ROOT_DECISION_ARMS,
        "the root reducer has {} arms that decide something; the recorded count is \
         {ROOT_DECISION_ARMS} (FR-002).\n\n\
         Fewer than 0 is impossible; more means a rule was added to the root instead of to a \
         feature, which is what this test exists to catch.\n\n\
         Currently deciding:\n  {}",
        deciding.len(),
        deciding.join("\n  ")
    );
}

/// Vacuity: the scan reads the reducer it thinks it does, and "routing" is a state an arm can
/// actually be in.
///
/// Both halves matter. A scan that finds no arms reports the root as perfectly routed; a
/// classifier nothing can satisfy makes the burn-down above unreachable.
///
/// The `!routing.is_empty()` half was the load-bearing one while 93 of 110 arms decided. Now that
/// none do, the exact count above can no longer catch a classifier that calls *everything* routing
/// — 0 is the answer such a classifier and a correct one both give. That is what the arm total
/// below is for: a scan that has stopped parsing reports 0 arms, and fails there.
#[test]
fn the_arm_scan_finds_the_reducer_it_is_meant_to_read() {
    let (routing, deciding) = classified();
    let total = routing.len() + deciding.len();
    // 110 when written; 89 after T064 folded the add-worktree wizard's 22 variants into one
    // `Message::WorktreeForm`. 110 - 22 + 1 = 89 exactly, which is the arithmetic that says the
    // collapse lost nothing — a scan that had merely stopped seeing some arms would not land on
    // the number the change predicts.
    //
    // The floor was 85 until feature 028 (FR-020). T064 was one feature folding its vocabulary;
    // 028 is all ten doing it, and the root ends at 15 arms — 10 wrapper variants and 5
    // cross-cutting ones — so a floor derived from the pre-028 count would fail every conversion
    // on its way down, and would have to be re-derived nine times to say nothing new. It is 12
    // now, comfortably under the 15 this feature arrives at and still far above the 0 a scan that
    // has stopped parsing reports.
    //
    // Lowering a floor weakens it, so the named check below is what takes over the work. It does
    // not decay: `EscapePressed` and `OverlayTransitionFinished` are cross-cutting, which is
    // exactly why 028 leaves them at the root, and a scan that has gone quiet cannot produce
    // either name. It named `ScrolledBeneathOverlay` as the first of the pair until feature 021
    // T081 deleted that variant; `OverlayTransitionFinished` replaces it and is cross-cutting for
    // the same reason — a transition ends for whichever surface was animating, not for a feature.
    assert!(
        total >= 12,
        "the scan found only {total} arms in the root reducer — the root ends feature 028 with \
         15, and a scan that has gone quiet reports the root as routing only"
    );
    for expected in ["OverlayTransitionFinished", "EscapePressed"] {
        assert!(
            routing.iter().chain(&deciding).any(|arm| arm == expected),
            "the scan did not find the root's `{expected}` arm, so it is not reading the reducer \
             it thinks it is. That arm is cross-cutting — it belongs to no feature and no \
             conversion moves it — so its absence means the parse broke, not that the root shrank"
        );
    }
    assert!(
        !routing.is_empty(),
        "no arm classified as routing, so the burn-down target is unreachable by construction"
    );
}
