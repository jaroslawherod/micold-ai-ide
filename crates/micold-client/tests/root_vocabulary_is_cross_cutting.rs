//! G1: the root vocabulary is cross-cutting (feature 028, T019–T022 — FR-013, SC-002).
//!
//! # The rule
//!
//! For each variant of `app::Message` that is not a feature's own vocabulary, resolve the **owner
//! set** of its arms in `State::update` and in `main.rs`'s `update_inner`: the `features::<n>::`
//! calls they make, else the `shell::<n>::` calls, else `overlay::registry::`. The guard **fails
//! when that set is exactly one feature**, and names the feature that should have declared the
//! variant.
//!
//! A variant produced and consumed by one feature is that feature's vocabulary sitting at the
//! junction that dispatches to it. Feature 021 shrank the root enum by hand and it grew back;
//! FR-013 exists so the next one fails on the day it lands.
//!
//! # Three verdicts, not two
//!
//! - **Two or more features, or the registry alone** — cross-cutting. Passes. `EscapePressed`
//!   reaches whatever surface is topmost, which is the registry's answer and no feature's.
//! - **Exactly one feature** — fails, named, unless [`ALLOWED`] carries a written reason.
//! - **No producer** — reported, never failed, and carries a written reason in [`NO_OWNER`]
//!   (FR-013). A variant nothing emits is a different defect from a variant in the wrong place,
//!   and folding it into a feature would be the wrong fix: it would give a dead message a home
//!   instead of a decision.
//!
//! # Why wrapper variants are exempt rather than allowlisted
//!
//! `Message::Session(session::Msg)` resolves to exactly one feature, and must — that is what
//! feature 028 built. The exemption is structural, not a judgement call: a variant whose payload
//! is `crate::features::<n>::Msg` **is** feature `<n>`'s declared vocabulary, so the rule has
//! nothing left to say about it. Ten allowlist entries saying "this one is fine" would be ten
//! places for an eleventh to hide.
//!
//! [`every_wrapper_variant_resolves_to_its_own_feature`] turns that exemption into the guard's
//! sharpest vacuity check. The resolver is asked to produce ten single-feature owner sets and name
//! each one correctly; a resolver that had stopped reading the reducer could not.
//!
//! # How an exception is granted
//!
//! [`ALLOWED`] — variant, written reason. Empty today. [`the_allowlist_names_only_live_violations`]
//! deletes it from the other side: an entry whose violation has been fixed fails the guard, because
//! an allowlist that outlives its reason is how the next real violation gets waved through.
//! [`NO_OWNER`] carries the same reverse check.
//!
//! # The probe that showed this non-vacuous (FR-017, T022)
//!
//! A variant was added to `app::Message` whose only arm called `features::help::about_opened`. The
//! guard failed, naming `help`. The message is recorded in
//! `specs/028-feature-encapsulation/assertion-adjudications.md`.
//!
//! # What this cannot see
//!
//! A decision hidden behind a root helper that is *not* defined in `app.rs`. Helper calls on
//! `self` are followed transitively while their bodies are in `app.rs` — which is how
//! `EscapePressed` resolves to the registry at all — and stop there.
//! `tests/root_is_routing_only.rs` is what watches whether those helpers should exist.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Variants that resolve to exactly one feature and stay at the root anyway — variant, reason.
///
/// Empty, and the intent is that it stays empty: feature 028 converted every feature that had a
/// vocabulary, so a violation here is a new one rather than a leftover.
const ALLOWED: &[(&str, &str)] = &[];

/// Variants nothing in `src/` emits — variant, reason (FR-013).
///
/// Empty since feature 021 T081 deleted `ScrolledBeneathOverlay`, the one entry it ever held.
/// T025 recorded it here rather than deleting it, on the grounds that the missing half was the
/// emitter and not the decision; T081 established on a running build that the live doors —
/// `SidebarScrolled` and `TabStripScrolled` — route to the same `close_on_scroll_beneath`, and
/// asked `tests/overlay_dismissal_delta.rs` through those instead. With the assertions moved onto
/// a message the user can actually send, the unreachable variant had nothing left to record.
const NO_OWNER: &[(&str, &str)] = &[];

// ---- Reading the source ------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
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

/// The text between the braces that `head` opens, `head` itself excluded.
fn block_after(src: &str, head: &str, what: &str) -> String {
    let at = src
        .find(head)
        .unwrap_or_else(|| panic!("`{head}` not found — has {what} been renamed?"));
    let open = at + head.len();
    let mut depth = 1usize;
    for (i, c) in src[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return src[open..open + i].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces in {what}");
}

/// Each arm of a `match message { … }` body, as `(pattern, body)`.
fn arms(body: &str) -> Vec<(String, String)> {
    let b: Vec<char> = body.chars().collect();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < n {
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

/// The identifier starting at `from`, if there is one.
fn ident_at(s: &str, from: usize) -> Option<&str> {
    let bytes = s.as_bytes();
    let mut end = from;
    while end < s.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    (end > from).then(|| &s[from..end])
}

/// Every `<prefix><ident>::` in `text`, as the identifiers.
fn segments_after(text: &str, prefix: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut from = 0usize;
    while let Some(at) = text[from..].find(prefix) {
        let start = from + at + prefix.len();
        from = start;
        let Some(name) = ident_at(text, start) else {
            continue;
        };
        if text[start + name.len()..].starts_with("::") {
            found.insert(name.to_string());
        }
    }
    found
}

// ---- The root vocabulary -----------------------------------------------------------------------

/// A variant of `app::Message`: its name, and the feature whose `Msg` it wraps, if any.
struct Variant {
    name: String,
    wraps: Option<String>,
}

/// The declared variants, in declaration order.
fn variants(app_rs: &str) -> Vec<Variant> {
    let body = block_after(app_rs, "pub enum Message {", "`app::Message`");
    let chars: Vec<char> = body.chars().collect();
    let mut out = Vec::new();
    let (mut i, mut start, mut depth) = (0usize, 0usize, 0i32);
    while i <= chars.len() {
        let split = i == chars.len() || (chars[i] == ',' && depth == 0);
        if i < chars.len() {
            match chars[i] {
                '{' | '(' | '[' | '<' => depth += 1,
                '}' | ')' | ']' | '>' => depth -= 1,
                _ => {}
            }
        }
        if split {
            let item: String = chars[start..i].iter().collect();
            let item = item.trim();
            if !item.is_empty() {
                let name: String = item
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                let payload = &item[name.len()..];
                let wraps = segments_after(payload, "crate::features::")
                    .into_iter()
                    .find(|_| payload.trim_end_matches([')', ' ']).ends_with("::Msg"));
                out.push(Variant { name, wraps });
            }
            start = i + 1;
        }
        i += 1;
    }
    out
}

/// The feature modules on disk — `src/features/*.rs`, `mod.rs` excluded.
fn feature_modules() -> BTreeSet<String> {
    fs::read_dir(crate_root().join("src/features"))
        .expect("read src/features")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter_map(|n| n.strip_suffix(".rs").map(str::to_string))
        .filter(|n| n != "mod")
        .collect()
}

/// `app.rs`'s methods on `&mut self`, by name — the root helpers an arm can hide behind.
fn root_helpers(app_rs: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut from = 0usize;
    while let Some(at) = app_rs[from..].find("fn ") {
        let start = from + at + "fn ".len();
        from = start;
        let Some(name) = ident_at(app_rs, start) else {
            continue;
        };
        let after = &app_rs[start + name.len()..];
        let Some(open) = after.find('(') else {
            continue;
        };
        if !after[open + 1..].trim_start().starts_with("&mut self") {
            continue;
        }
        let Some(brace) = after.find('{') else {
            continue;
        };
        let head = &after[..=brace];
        out.insert(
            name.to_string(),
            block_after(&app_rs[start + name.len()..], head, "a root helper"),
        );
    }
    out
}

/// An arm's body with every root helper it calls on `self` spliced in, transitively.
fn expanded(body: &str, helpers: &BTreeMap<String, String>) -> String {
    let mut text = body.to_string();
    let mut seen = BTreeSet::new();
    loop {
        let mut names: Vec<String> = Vec::new();
        for name in helpers.keys() {
            if text.contains(&format!("self.{name}(")) && seen.insert(name.clone()) {
                names.push(name.clone());
            }
        }
        if names.is_empty() {
            return text;
        }
        for name in names {
            text.push('\n');
            text.push_str(&helpers[&name]);
        }
    }
}

/// What each variant's arms resolve to, and how many places emit it.
struct Scan {
    /// Variant → the owner names its arms resolve to (features, shell modules, `registry`).
    owners: BTreeMap<String, BTreeSet<String>>,
    /// Variant → how many sites in `src/` construct it.
    producers: BTreeMap<String, usize>,
    variants: Vec<Variant>,
    features: BTreeSet<String>,
}

impl Scan {
    /// The features — as opposed to shell modules or the registry — a variant resolves to.
    fn owning_features(&self, variant: &str) -> BTreeSet<String> {
        self.owners
            .get(variant)
            .map(|o| o.intersection(&self.features).cloned().collect())
            .unwrap_or_default()
    }
}

/// The pattern's leading variant name.
fn label(pattern: &str) -> String {
    pattern
        .split(['(', '{', '|'])
        .next()
        .unwrap_or(pattern)
        .trim()
        .trim_start_matches("Message::")
        .to_string()
}

fn scan() -> Scan {
    let root = crate_root();
    let app_rs = code_only(&fs::read_to_string(root.join("src/app.rs")).expect("read app.rs"));
    let main_rs = code_only(&fs::read_to_string(root.join("src/main.rs")).expect("read main.rs"));

    let variants = variants(&app_rs);
    let features = feature_modules();
    let helpers = root_helpers(&app_rs);

    let mut owners: BTreeMap<String, BTreeSet<String>> = variants
        .iter()
        .map(|v| (v.name.clone(), BTreeSet::new()))
        .collect();

    let pure = block_after(
        &app_rs,
        "pub fn update(&mut self, message: Message) {",
        "the root reducer",
    );
    let shell = block_after(
        &main_rs,
        "fn update_inner(app: &mut App, message: Message) -> Task<Message> {",
        "the shell reducer",
    );
    for reducer in [&pure, &shell] {
        for (pattern, body) in arms(&block_after(reducer, "match message {", "a reducer")) {
            let name = label(&pattern);
            let Some(set) = owners.get_mut(&name) else {
                continue;
            };
            let text = expanded(&body, &helpers);
            set.extend(segments_after(&text, "features::"));
            set.extend(segments_after(&text, "shell::"));
            if text.contains("registry::") {
                set.insert("registry".to_string());
            }
        }
    }

    let producers = variants
        .iter()
        .map(|v| (v.name.clone(), producer_sites(&root, &v.name)))
        .collect();

    Scan {
        owners,
        producers,
        variants,
        features,
    }
}

/// How many sites under `src/` construct `Message::<variant>` rather than match on it.
///
/// A match pattern is followed by `=>` or `|` once its payload is skipped; anything else is a
/// construction. The declaration in `app::Message` itself is not a mention of `Message::`, so it
/// never reaches here.
fn producer_sites(root: &Path, variant: &str) -> usize {
    let needle = format!("Message::{variant}");
    let mut count = 0usize;
    let mut stack = vec![root.join("src")];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read src").filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let src = code_only(&fs::read_to_string(&path).expect("read a source file"));
            let mut from = 0usize;
            while let Some(at) = src[from..].find(&needle) {
                let start = from + at;
                from = start + needle.len();
                // Reject `Message::SessionFoo` when looking for `Message::Session`.
                if ident_at(&src, start + "Message::".len()) != Some(variant) {
                    continue;
                }
                let mut tail = src[from..].trim_start();
                if tail.starts_with('(') || tail.starts_with('{') {
                    let (open, close) = if tail.starts_with('(') {
                        ('(', ')')
                    } else {
                        ('{', '}')
                    };
                    let mut depth = 0i32;
                    let mut end = tail.len();
                    for (i, c) in tail.char_indices() {
                        if c == open {
                            depth += 1;
                        } else if c == close {
                            depth -= 1;
                            if depth == 0 {
                                end = i + c.len_utf8();
                                break;
                            }
                        }
                    }
                    tail = tail[end..].trim_start();
                }
                if !tail.starts_with("=>") && !tail.starts_with('|') {
                    count += 1;
                }
            }
        }
    }
    count
}

// ---- The rule ----------------------------------------------------------------------------------

#[test]
fn no_root_variant_belongs_to_one_feature() {
    let scan = scan();
    let allowed: BTreeSet<&str> = ALLOWED.iter().map(|(v, _)| *v).collect();
    let violations: Vec<String> = scan
        .variants
        .iter()
        .filter(|v| v.wraps.is_none() && !allowed.contains(v.name.as_str()))
        .filter_map(|v| {
            let owners = scan.owning_features(&v.name);
            (owners.len() == 1).then(|| {
                let feature = owners.iter().next().expect("one owner").clone();
                format!("  `Message::{}` — `{feature}`", v.name)
            })
        })
        .collect();
    assert!(
        violations.is_empty(),
        "the root vocabulary holds {} variant(s) produced and consumed by exactly one feature \
         (FR-013):\n{}\n\n\
         Each belongs in that feature's own `Msg`, behind the wrapper variant the root already \
         has for it. The root routes; a variant only one feature ever answers is not routing.",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn variants_with_no_producer_are_reported_not_failed() {
    let scan = scan();
    let recorded: BTreeSet<&str> = NO_OWNER.iter().map(|(v, _)| *v).collect();
    let orphans: BTreeSet<&str> = scan
        .variants
        .iter()
        .filter(|v| scan.producers.get(&v.name).copied().unwrap_or(0) == 0)
        .map(|v| v.name.as_str())
        .collect();
    let unrecorded: Vec<&str> = orphans.difference(&recorded).copied().collect();
    assert!(
        unrecorded.is_empty(),
        "nothing in `src/` emits {:?}, and NO_OWNER does not say why (FR-013).\n\n\
         This is reported, never failed: a variant nobody produces is a different defect from a \
         variant in the wrong place, and folding it into a feature would give a dead message a \
         home instead of a decision. Add an entry with the reason, or delete the variant.",
        unrecorded
    );
    assert_eq!(
        orphans, recorded,
        "the reported no-owner set and NO_OWNER disagree. Both are empty since feature 021 T081 \
         deleted `ScrolledBeneathOverlay`, the only entry this list ever held (T025); a variant \
         appearing on either side is a new fact about the root, not a leftover"
    );
}

#[test]
fn the_allowlist_names_only_live_violations() {
    let scan = scan();
    let dead: Vec<String> = ALLOWED
        .iter()
        .filter(|(variant, _)| scan.owning_features(variant).len() != 1)
        .map(|(variant, why)| format!("  `Message::{variant}` — {why}"))
        .collect();
    assert!(
        dead.is_empty(),
        "ALLOWED permits variants that no longer resolve to one feature:\n{}\n\n\
         Delete each line. An allowlist that outlives what it permitted is how the next real \
         violation gets waved through.",
        dead.join("\n")
    );
}

#[test]
fn the_no_owner_list_names_only_live_orphans() {
    let scan = scan();
    let dead: Vec<String> = NO_OWNER
        .iter()
        .filter(|(variant, _)| scan.producers.get(*variant).copied().unwrap_or(0) > 0)
        .map(|(variant, _)| format!("  `Message::{variant}`"))
        .collect();
    assert!(
        dead.is_empty(),
        "NO_OWNER records variants that something now emits:\n{}\n\n\
         The variant became reachable, so the written reason is no longer true. Delete the entry \
         — and if it now resolves to one feature, the rule above has something to say about it.",
        dead.join("\n")
    );
}

// ---- Vacuity -------------------------------------------------------------------------------

/// The exemption above is structural, and this is the bill it pays.
///
/// Every wrapper variant *must* resolve to exactly one feature — its own. That is ten chances for
/// a resolver that has stopped reading the reducer to be caught, and it is the same resolution the
/// rule uses, so a resolver that satisfies this cannot be silently returning nothing for the flat
/// variants either.
#[test]
fn every_wrapper_variant_resolves_to_its_own_feature() {
    let scan = scan();
    let wrappers: Vec<&Variant> = scan.variants.iter().filter(|v| v.wraps.is_some()).collect();
    assert!(
        wrappers.len() >= 10,
        "found {} wrapper variants; feature 028 ends with 10 (T016)",
        wrappers.len()
    );
    for v in wrappers {
        let feature = v.wraps.clone().expect("a wrapper");
        let owners = scan.owning_features(&v.name);
        assert!(
            owners.contains(&feature),
            "`Message::{}` wraps `{feature}::Msg`, but its arms resolve to {:?}. Either the \
             wrapper routes somewhere other than its own feature, or this scan is no longer \
             reading the reducer it thinks it is",
            v.name,
            owners
        );
    }
}

#[test]
fn the_scan_finds_the_vocabulary_it_is_meant_to_read() {
    let scan = scan();
    assert_eq!(
        scan.variants.len(),
        16,
        "the root vocabulary is 11 feature wrappers and 5 cross-cutting variants (SC-002); the \
         scan found {:?}",
        scan.variants.iter().map(|v| &v.name).collect::<Vec<_>>()
    );
    assert!(
        scan.features.len() >= 11,
        "found {} feature modules; there are at least 11",
        scan.features.len()
    );
    // Cross-cutting by the registry rather than by a feature, which is the verdict that would
    // disappear first if helper following broke.
    //
    // One name, not two. `ScrolledBeneathOverlay` was the other, and feature 021 T081 deleted it:
    // the live scroll doors (`SidebarScrolled`, `TabStripScrolled`) route to the same
    // `close_on_scroll_beneath`, but they are feature vocabulary and reach it from inside their
    // own modules, so the root has exactly one arm left that resolves through a registry helper.
    let expected = "EscapePressed";
    assert!(
        scan.owners
            .get(expected)
            .is_some_and(|o| o.contains("registry")),
        "`Message::{expected}` no longer resolves to the overlay registry, so the helper \
         following this guard depends on has stopped working"
    );
    assert!(
        scan.producers.values().any(|&n| n > 0),
        "no variant has a producer, so the no-producer verdict is unreachable by construction"
    );
}
