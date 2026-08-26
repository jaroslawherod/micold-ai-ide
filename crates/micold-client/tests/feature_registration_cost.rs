//! Adding a feature costs one module and one line — permanently (feature 021, T070 — SC-002,
//! SC-002a).
//!
//! # The sibling of `surface_registration_cost.rs`, and why it is a different shape
//!
//! SC-001 and SC-002 make the same claim about two different things, and SC-002a asks for both to
//! be held by a guard rather than a count. For *surfaces* the leak has an obvious shape: a central
//! `match` growing an arm per surface, which is caught by forbidding any file but the surface's
//! own module and the registry to name it.
//!
//! A feature cannot be guarded that way, and it is worth saying why rather than writing a weaker
//! version of the same test and calling it done. Features are named all over the crate on purpose:
//! `crate::ui` draws them, and FR-003a *requires* cross-feature reads to stay possible and cheap.
//! There is no `FeatureId` and no central match over one. So "names a feature" is not the leak.
//!
//! What would actually make adding a feature cost more than a module and a line is a feature being
//! **driven** from somewhere other than the root, or a feature module being **edited** because a
//! different feature was added. Those are the two this file checks, plus the two bookkeeping facts
//! that make the rest non-vacuous: every module is registered exactly once, and every module has
//! the isolation test SC-004 asks for.
//!
//! # Why the last one is here rather than left to a count
//!
//! T071 verified SC-004 by hand and found two feature modules with no isolation test —
//! `features/help.rs` and `features/window.rs`, both created *after* the criterion's "eight feature
//! modules" was written. A number in a spec is satisfied by the day it was taken. That is the same
//! argument SC-002a makes about SC-001 and SC-002, so the same answer applies.
//!
//! # G3: every feature module has a reducer entry point (feature 028, T023–T024 — FR-015, SC-004)
//!
//! ## The rule
//!
//! For each module under `src/features/` other than `mod.rs`: if it declares `pub enum Msg`, it
//! must expose **shape A** — `pub fn update(&mut State, Msg) -> Vec<Outcome>` in the same module —
//! or **shape B** — `pub fn update(&mut App, Msg) -> Task<Message>` in `src/shell/<n>.rs`. A module
//! declaring no `Msg` passes and needs no allowlist entry; that is FR-005's no-ceremony case, and
//! `features/mod.rs`'s own vocabulary types are why it exists.
//!
//! A vocabulary with nowhere to be handled is the shape feature 021 left behind nine times: the
//! variants sat in `app::Message` and the arms sat in the root, which is exactly the coupling
//! FR-001 is about. Declaring `Msg` is the claim that a feature owns its transitions; `update` is
//! where it answers them.
//!
//! ## Why here rather than a new file
//!
//! This file already enumerates feature modules from the filesystem rather than from a list — its
//! own header records that a hardcoded count went stale the day two modules were added — and
//! already parses signatures taking the state mutably, in both the `&mut self` and `&mut State`
//! spellings. Re-deriving either in a new file would be a second answer to the same question.
//!
//! ## How an exception is granted
//!
//! There is none, and none is planned. The two shapes already cover a feature that is entirely
//! effects (`connection`, shape B with no pure `update` at all) and a feature that is entirely
//! pure. If a third shape is ever needed it takes the same `(&str, &str)` allowlist form the other
//! guards use, with a written reason.
//!
//! ## The probe that showed this non-vacuous (FR-017, T024)
//!
//! `src/features/probe.rs` was added declaring `pub enum Msg { Tick }` and no `update`. The guard
//! failed, naming `probe`. The message is recorded in
//! `specs/028-feature-encapsulation/assertion-adjudications.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Cross-feature names inside `src/features/` that are not the shared outcome vocabulary.
///
/// **Adding an entry requires a reason, in the note.** A feature naming another feature's item is
/// not a write and not a violation of FR-020 — FR-003a permits reads outright — but it is the
/// coupling SC-002 is about: it is how "adding a feature" starts to mean "and edit that one too".
///
/// One entry today, and it is a read of a pure helper: the sidebar renders a worktree's tags, and
/// what a worktree's tags *are* is the worktree feature's to say. The alternative — the sidebar
/// deriving tags itself — would be a second answer to the same question, which is worse.
const ALLOWED_CROSS_FEATURE_NAMES: &[(&str, &str, &str)] = &[(
    "sidebar",
    "worktree",
    "worktree_tags — the sidebar renders a worktree row's tags and does not get to decide what \
     they are (feature 021, T007)",
)];

/// The shared vocabulary any feature may name, because it belongs to no feature.
///
/// `notifications` is the one the contract names outright — "emitted by: any feature" — and the
/// reason is that a notification is nobody's feature: every path that can fail wants one, and
/// `state.notify` belongs to none of them. `mod.rs` holds `Outcome` itself and the helpers over it.
const SHARED_VOCABULARY: &[&str] = &["notifications", "mod"];

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// Source with `//` comments stripped, so a name in prose is not a name in code.
fn code_only(src: &str) -> String {
    src.lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `.rs` file under `src/`, as `(path relative to src/, code)`.
fn sources() -> BTreeMap<String, String> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).expect("read src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path
                    .strip_prefix(root)
                    .expect("under src")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(
                    rel,
                    code_only(&fs::read_to_string(&path).expect("read source")),
                );
            }
        }
    }
    let root = src_dir();
    let mut out = BTreeMap::new();
    walk(&root, &root, &mut out);
    out
}

/// The feature modules, by the files that exist.
fn feature_modules() -> BTreeSet<String> {
    fs::read_dir(src_dir().join("features"))
        .expect("read src/features")
        .filter_map(|entry| {
            let path = entry.expect("dir entry").path();
            let stem = path.file_stem()?.to_string_lossy().to_string();
            (path.extension().is_some_and(|e| e == "rs") && stem != "mod").then_some(stem)
        })
        .collect()
}

/// Every `fn` in `src` whose parameters take the state mutably, by name.
///
/// Both spellings, because Tier 1 left some operations as `impl State` methods and Tier 3 made the
/// rest free functions: `&mut self` and `&mut State` are the same thing to a caller that wants to
/// change something.
fn mutating_fns(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = src;
    while let Some(at) = rest.find("fn ") {
        let after = &rest[at + 3..];
        let name_end = after
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(after.len());
        let name = &after[..name_end];
        if let Some(open) = after.find('(') {
            if let Some(close) = after[open..].find(')') {
                let params = &after[open..open + close];
                if params.contains("&mut self") || params.contains("&mut State") {
                    out.insert(name.to_string());
                }
            }
        }
        rest = after;
    }
    out
}

#[test]
fn every_feature_module_is_registered_exactly_once() {
    // The "one registration line" half of SC-002, and the floor that keeps the rest of this file
    // from passing over an empty list.
    let modules = feature_modules();
    assert!(
        modules.len() >= 10,
        "found only {} feature modules — src/features/ is not being read",
        modules.len()
    );

    let declared: BTreeSet<String> = code_only(
        &fs::read_to_string(src_dir().join("features/mod.rs")).expect("read features/mod.rs"),
    )
    .lines()
    .filter_map(|line| {
        line.trim()
            .strip_prefix("pub mod ")?
            .strip_suffix(';')
            .map(str::to_string)
    })
    .collect();

    assert_eq!(
        declared, modules,
        "`src/features/mod.rs` and `src/features/` disagree about which features exist. That file \
         is the single registration point SC-002 allows; a module missing from it is unreachable, \
         and a declaration with no module does not compile."
    );

    // ...and nowhere else declares one, which is what makes it *the* registration point.
    //
    // **`shell/mod.rs` declaring the same *name* is not a second registration** (feature 028,
    // FR-020). A feature that must return an `iced::Task` puts that half in `src/shell/<n>.rs`
    // and the pure half in `src/features/<n>.rs` — two different modules, in two different trees,
    // that happen to share a name because they are two halves of one feature. `settings` is the
    // first; `connection` is the second. The check below is textual, so without this it reads
    // `pub mod settings;` in `shell/mod.rs` as a duplicate of `features/mod.rs`'s, which it is
    // not. The exception is deliberately narrow: only `shell/mod.rs`, and only when the file it
    // is declaring is really there, so a stray declaration naming no module still fails.
    let shell_half = |m: &str| src_dir().join("shell").join(format!("{m}.rs")).is_file();
    let elsewhere: Vec<String> = sources()
        .iter()
        .filter(|(path, _)| *path != "features/mod.rs")
        .flat_map(|(path, code)| {
            modules
                .iter()
                .filter(move |m| code.contains(&format!("pub mod {m};")))
                .filter(move |m| !(path == "shell/mod.rs" && shell_half(m)))
                .map(move |m| format!("  {path} declares `{m}`"))
        })
        .collect();
    assert!(
        elsewhere.is_empty(),
        "a feature module is declared outside the registration point:\n{}",
        elsewhere.join("\n")
    );
}

/// The `shell/mod.rs` exception above, pinned so it cannot widen into "any file may declare a
/// feature module" (feature 028, FR-020).
///
/// Two claims: the exception applies only where the shell half really exists, and every shell
/// module that borrows a feature's name is a shell *half* of that feature rather than an
/// unrelated module that happens to collide. The second is what would go wrong first — a
/// `shell/session.rs` full of something other than `session`'s effects would pass the guard above
/// and mean nothing by the name.
#[test]
fn the_shell_half_exception_covers_only_real_shell_halves() {
    let modules = feature_modules();
    let declared: BTreeSet<String> =
        code_only(&fs::read_to_string(src_dir().join("shell/mod.rs")).expect("read shell/mod.rs"))
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("pub mod ")?
                    .strip_suffix(';')
                    .map(str::to_string)
            })
            .collect();

    for name in declared.intersection(&modules) {
        let path = src_dir().join("shell").join(format!("{name}.rs"));
        assert!(
            path.is_file(),
            "`shell/mod.rs` declares `{name}`, which is a feature module name, but \
             `src/shell/{name}.rs` does not exist — the exception in \
             `every_feature_module_is_registered_exactly_once` would be excusing a declaration \
             that is not a shell half at all"
        );
        let code = code_only(&fs::read_to_string(&path).expect("read shell half"));
        assert!(
            code.contains(&format!("features::{name}::Msg")),
            "`src/shell/{name}.rs` borrows the `{name}` feature's name without naming \
             `features::{name}::Msg`. A shell half routes that feature's vocabulary; a module \
             that does not is a name collision, and the registration guard should be failing on \
             it rather than excusing it"
        );
    }
}

#[test]
fn only_the_root_drives_a_feature() {
    // The leak that would make adding a feature cost more than a module and a line. A feature's
    // *reducers* — the functions that take the state mutably — are what the feature does, and if
    // anything but the root calls one, that caller has to learn about every feature that arrives
    // afterwards. Reading is deliberately untouched: `crate::ui` names feature types to draw them
    // and calls their pure query helpers (`clamp_menu_anchor`, `help_actions`,
    // `worktree_location_label`, `connection_status`), which is FR-003a working as intended.
    let sources = sources();
    let reducers: BTreeMap<String, BTreeSet<String>> = feature_modules()
        .into_iter()
        .map(|m| {
            let code = sources
                .get(&format!("features/{m}.rs"))
                .unwrap_or_else(|| panic!("no source for feature `{m}`"));
            (m, mutating_fns(code))
        })
        .collect();

    let total: usize = reducers.values().map(BTreeSet::len).sum();
    assert!(
        total >= 100,
        "the scan found only {total} reducers across the feature modules — it found 122 when this \
         was written, and a scan that has gone quiet reports every caller clean"
    );

    let mut driven = Vec::new();
    for (path, code) in &sources {
        if path.starts_with("features/") || path == "app.rs" {
            continue;
        }
        for (module, fns) in &reducers {
            for name in fns {
                if code.contains(&format!("features::{module}::{name}(")) {
                    driven.push(format!("  {path} calls `{module}::{name}`"));
                }
            }
        }
    }
    assert!(
        driven.is_empty(),
        "a feature is driven from outside the root reducer (SC-002, FR-002):\n{}\n\n\
         Route it through a `Message` arm in `app.rs`. A second driver is a second place that has \
         to learn about every feature added after it.",
        driven.join("\n")
    );
}

#[test]
fn a_feature_module_names_no_other_feature_beyond_the_shared_vocabulary() {
    // "Zero edits to any other feature's module" (SC-002), from the other side: what a feature is
    // *entitled* to know about its neighbours. Not a write check — `feature_write_isolation.rs` is
    // that — but the coupling that makes one feature's change land in another's file.
    let sources = sources();
    let modules = feature_modules();
    let allowed: BTreeSet<(&str, &str)> = ALLOWED_CROSS_FEATURE_NAMES
        .iter()
        .map(|(from, to, _)| (*from, *to))
        .collect();

    let mut coupled = Vec::new();
    for module in &modules {
        let code = &sources[&format!("features/{module}.rs")];
        for other in &modules {
            if other == module || SHARED_VOCABULARY.contains(&other.as_str()) {
                continue;
            }
            if code.contains(&format!("features::{other}::"))
                && !allowed.contains(&(module.as_str(), other.as_str()))
            {
                coupled.push(format!("  `{module}` names `{other}`"));
            }
        }
    }
    assert!(
        coupled.is_empty(),
        "a feature module names another feature's items (SC-002):\n{}\n\n\
         If it is a read of a pure helper, add it to ALLOWED_CROSS_FEATURE_NAMES with the reason. \
         If it wants that feature's data *changed*, return an `Outcome` instead (FR-021).",
        coupled.join("\n")
    );

    let live: BTreeSet<(&str, &str)> = modules
        .iter()
        .flat_map(|m| {
            let code = &sources[&format!("features/{m}.rs")];
            ALLOWED_CROSS_FEATURE_NAMES
                .iter()
                .filter(move |(from, to, _)| {
                    *from == m.as_str() && code.contains(&format!("features::{to}::"))
                })
                .map(|(from, to, _)| (*from, *to))
        })
        .collect();
    let dead: Vec<String> = allowed
        .difference(&live)
        .map(|(from, to)| format!("  `{from}` no longer names `{to}`"))
        .collect();
    assert!(
        dead.is_empty(),
        "ALLOWED_CROSS_FEATURE_NAMES permits a coupling that no longer exists:\n{}\n\n\
         Delete each line, for the same reason no allowlist in this suite may outlive what it \
         permitted.",
        dead.join("\n")
    );
}

#[test]
fn every_feature_module_has_an_isolation_test() {
    // SC-004, made permanent. It was verified by hand at T071 and two modules had drifted out of
    // its "eight feature modules" — both created after the criterion was written. This is the
    // check that a count cannot be.
    let missing: Vec<String> = feature_modules()
        .into_iter()
        .filter(|m| !tests_dir().join(format!("features_{m}.rs")).exists())
        .map(|m| format!("  `{m}` has no tests/features_{m}.rs"))
        .collect();
    assert!(
        missing.is_empty(),
        "a feature module has no isolation test (SC-004):\n{}\n\n\
         One file per feature, constructing only that feature's types. See tests/features_help.rs \
         for the smallest example.",
        missing.join("\n")
    );
}

// ---- G3: every feature module has a reducer entry point (FR-015) --------------------------------

/// Whether `code` declares a `pub fn update` whose parameters and return type match.
fn declares_update(code: &str, takes: &str, returns: &str) -> bool {
    let mut rest = code;
    while let Some(at) = rest.find("pub fn update") {
        let after = &rest[at + "pub fn update".len()..];
        rest = after;
        let Some(open) = after.find('(') else {
            continue;
        };
        let Some(close) = after[open..].find(')') else {
            continue;
        };
        let params = &after[open..open + close];
        let signature_end = after[open + close..]
            .find('{')
            .map(|i| open + close + i)
            .unwrap_or(after.len());
        let ret = &after[open + close..signature_end];
        if params.contains(takes) && params.contains("Msg") && ret.contains(returns) {
            return true;
        }
    }
    false
}

/// The entry shape a feature module exposes, if any (data-model.md §1.1).
fn entry_shape(module: &str, sources: &BTreeMap<String, String>) -> Option<&'static str> {
    let pure = sources
        .get(&format!("features/{module}.rs"))
        .is_some_and(|code| declares_update(code, "&mut State", "Outcome"));
    if pure {
        return Some("A");
    }
    let effectful = sources
        .get(&format!("shell/{module}.rs"))
        .is_some_and(|code| declares_update(code, "&mut App", "Task<Message>"));
    effectful.then_some("B")
}

#[test]
fn every_feature_with_a_vocabulary_has_a_reducer_entry_point() {
    let sources = sources();
    let modules = feature_modules();

    let with_vocabulary: Vec<&String> = modules
        .iter()
        .filter(|m| {
            sources
                .get(&format!("features/{m}.rs"))
                .is_some_and(|code| code.contains("pub enum Msg"))
        })
        .collect();

    // The floor. Feature 028 converted ten; a scan that has stopped finding `pub enum Msg` would
    // report every module compliant by finding nothing to check.
    assert!(
        with_vocabulary.len() >= 10,
        "found only {} feature modules declaring `pub enum Msg` — there are at least 10 \
         (feature 028, T016), so this scan is no longer reading what it thinks it is",
        with_vocabulary.len()
    );

    let stranded: Vec<String> = with_vocabulary
        .iter()
        .filter(|m| entry_shape(m, &sources).is_none())
        .map(|m| format!("  `{m}` declares `pub enum Msg` and no `update`"))
        .collect();
    assert!(
        stranded.is_empty(),
        "a feature declares a vocabulary with nowhere to handle it (FR-015):\n{}\n\n\
         Add `pub fn update(&mut State, Msg) -> Vec<Outcome>` to the feature module (shape A), or \
         `pub fn update(&mut App, Msg) -> Task<Message>` to `src/shell/<feature>.rs` when the \
         transitions must return an `iced::Task` (shape B). A vocabulary answered by the root \
         instead is the coupling FR-001 exists to remove.",
        stranded.join("\n")
    );
}

/// Both shapes are in use, and a module with no `Msg` is not quietly failing.
///
/// The rule above passes if every module happens to be shape A. That would still be true if the
/// shape-B detector had rotted, and `connection` — the one feature that is entirely effects — is
/// the module that would go unnoticed.
#[test]
fn both_entry_shapes_are_live_and_a_feature_may_have_neither() {
    let sources = sources();
    let modules = feature_modules();
    let shapes: Vec<(&String, Option<&str>)> = modules
        .iter()
        .map(|m| (m, entry_shape(m, &sources)))
        .collect();

    for shape in ["A", "B"] {
        assert!(
            shapes.iter().any(|(_, s)| *s == Some(shape)),
            "no feature module exposes shape {shape}, so half of the rule above is unreachable by \
             construction. `connection` is shape B and every other converted feature is shape A; \
             if that has genuinely changed, this check needs rewriting rather than relaxing"
        );
    }

    // FR-005's no-ceremony case, stated as a fact rather than as an absence: `connection` declares
    // a vocabulary and no pure `update`, and it is compliant. If someone "fixes" that by adding an
    // empty shape-A reducer, the rule above would still pass and this would catch the drift.
    assert_eq!(
        entry_shape("connection", &sources),
        Some("B"),
        "`connection` is the feature that is entirely effects — shape B, with no pure `update` in \
         `features/connection.rs` (feature 028, T016). A feature is not required to have both."
    );
}
