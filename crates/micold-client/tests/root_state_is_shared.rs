//! G2 — nothing sits loose in the root state (feature 028, T042/T044 — FR-014, FR-007a, SC-003).
//!
//! # The rule
//!
//! Every public field of [`micold_client::app::State`] is one of two things:
//!
//! 1. a **feature struct** — its type resolves to `crate::features::<n>::State` for a module that
//!    exists under `src/features/`; or
//! 2. a **declared shared member** — named in [`SHARED`] with a written reason saying which
//!    features read it and why it cannot be assigned to one of them.
//!
//! A flat field that is neither fails, and the failure names the single feature that writes it,
//! resolved through the same transitive `&mut State` scan `feature_write_isolation.rs` performs
//! (`support/state_scan.rs`, shared between the two so neither can drift into agreeing with a
//! stale copy).
//!
//! # Why the rule is stated over types
//!
//! Feature 021 answered the same question with `OWNERS` — 51 hand-written `(path, feature)` rows
//! that a maintainer had to extend every time a field was added, and that said nothing at all
//! about a field nobody remembered to add. Stating it over the field's *type* is what feature 028
//! buys: after the migration "this belongs to the sidebar" is spelled
//! `pub sidebar: crate::features::sidebar::State`, which the compiler already checks and which no
//! one can forget to update. The guard reads the declaration rather than a parallel list of
//! claims about it (SC-007).
//!
//! So the allowlist shrinks to the exceptions, and there is one: `workspace`. Its six members
//! answer to three features and its own type carries invariants across all six — `Workspace::forget`
//! applies one rule to every member at once — so it can be neither folded into one feature's
//! struct nor split across three. It is declared instead, which is what [`SHARED`] means.
//!
//! # The component rule (FR-007a)
//!
//! [`COMPONENT_LOCAL`] holds the second half, and it moves nothing today. A path with exactly one
//! writing feature and no reader outside that feature's module and its own view is state the
//! *component* could hold — it never leaves the pair of files that render it. FR-007a says such a
//! path should move into the component, **unless an existing assertion pins it to the
//! application**, and every one that qualifies today is pinned — by `tests/logical_state_ownership.rs`
//! (feature 017), which exists to assert that these facts outlive the widget that shows them, and
//! by the overlay suite, which asks the whole window which surface is open. FR-021 forbids
//! relaxing those tests to let a path move, so the allowlist is the honest record — the rule is
//! live, it has thirteen hits, and all thirteen are answered.
//!
//! # Neither list may outlive its reason
//!
//! [`the_allowlist_names_only_live_violations`] fails on a [`SHARED`] entry that is no longer a
//! flat field, and on a [`COMPONENT_LOCAL`] entry that no longer qualifies under the rule. An
//! allowlist entry that outlives what it permitted is the same failure as no guard at all, only
//! quieter — the shape `feature_write_isolation.rs` already guards against, for the same reason.
//!
//! # Non-vacuity (FR-017, SC-005)
//!
//! T043's probe: add `pub scratch_pad: String` to `app::State`, written only from
//! `features/help.rs`, and run `cargo test -p micold-client --test root_state_is_shared`. The
//! observed failure is recorded in
//! `specs/028-feature-encapsulation/assertion-adjudications.md`. A guard nobody has seen fail is a
//! guard nobody knows works.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// The shared source-text scan — see its own header for why it is not a copy.
#[path = "support/state_scan.rs"]
mod state_scan;

use state_scan::{
    code_only, feature_of, operations_in, sources, src_dir, struct_field_types, struct_fields,
    transitive_writes, workspace_rs, Operation,
};

/// Flat fields of `app::State` that are not a feature's struct, each with the reason it is shared.
///
/// One entry, and the reason is argued at length on the field itself in `src/app.rs`. In short:
/// three features read disjoint pairs of its six members, so naming any one of them as the owner
/// would put the other two's data behind the wrong name (FR-001); and `Workspace`'s own operations
/// span all six, so splitting it would make a half-applied `forget` expressible for the first
/// time (`CORE_MEDIATED` in `tests/feature_write_isolation.rs`).
const SHARED: &[(&str, &str)] = &[(
    "workspace",
    "the project catalog, the session lists and the two worktree maps in one core type: \
     `project` reads `projects`/`active`, `session` reads `sessions`/`foreground_by_project`, \
     `worktree` reads `worktree_names`/`included_worktrees`, and `Workspace::forget` writes all \
     six at once (feature 028, FR-008, contract S2)",
)];

/// Paths FR-007a's component rule reaches, each with the assertion that pins it to the application.
///
/// Every entry is a path that *qualifies*: one writing feature, no reader outside that feature's
/// module and its view. None of them moves, because the second half of FR-007a is the exception —
/// an existing assertion says the fact outlives the widget — and FR-021 forbids editing that
/// assertion to make room for a move. The list is measured, not chosen: it is exactly what
/// [`component_local_candidates`] returns, and every candidate had a pinning assertion already.
///
/// Nine of the thirteen are *which surface is open* — a dialog, a menu, a switcher, a panel. That is
/// not a coincidence: modality is the one thing a component cannot own, because deciding what
/// Escape does and what a scroll dismisses is a question about the whole window. The overlay suite
/// (`overlay_dismissal_delta.rs`, `overlay_dispatch_ordering.rs`) asks it of `app::State` for every
/// surface at once, so each of those flags is pinned by construction. The other four are what the
/// sidebar remembers across a re-discovery, the tag filter that decides which rows exist, how large
/// the window is, and whether the Settings rail is collapsed to its icons.
const COMPONENT_LOCAL: &[(&str, &str)] = &[
    (
        "help.about_open",
        "tests/logical_state_ownership.rs::open_overlay_identity_is_application_owned — which \
         dialog is open decides what Escape does, read back through `overlay::registry`",
    ),
    (
        "help.help_menu_open",
        "tests/about_open.rs::help_menu_toggles_open_and_closed — the menu's open flag is asserted \
         on `app::State`, and `overlay_dismissal_delta.rs` dismisses it from there",
    ),
    (
        "project.menu_open",
        "tests/switcher_forget_menu.rs::the_menu_anchors_at_the_press_point — whose menu is open \
         and where it was opened from is application state (018 FR-029d)",
    ),
    (
        "project.switcher_open",
        "tests/project_switcher.rs::toggling_switcher_opens_and_closes_it — the switcher and the \
         help menu close each other, which neither component can decide alone",
    ),
    (
        "session.menu_open",
        "tests/overlay_dismissal_delta.rs::every_non_modal_surface_closes_on_a_scroll_beneath — a \
         scroll beneath the list closes it, and the list is not the menu",
    ),
    (
        "session.shell_instance_menu",
        "tests/app_state.rs::the_tab_menu_belongs_to_the_tab_it_was_opened_on — the menu records \
         *which* tab, so it outlives the tab widget that opened it",
    ),
    (
        "session.start_press",
        "tests/session_start_press.rs::the_list_the_primary_half_opens_hangs_from_the_press — the \
         point is recorded on the press and read back on the release, a message later, so it \
         outlives the button that reported it (018 BUG-008)",
    ),
    (
        "settings.settings_rail_collapsed",
        "tests/settings_rail.rs::neither_save_nor_cancel_reopens_a_rail_the_user_closed — Save and \
         Cancel both end the form the rail is drawn in, and the flag has to be found as the user \
         left it when Settings is opened again (feature 027, FR-026d)",
    ),
    (
        "sidebar.default_expanded",
        "tests/switch_active.rs::view_state_does_not_carry_from_the_project_you_left — the flag is \
         reset by a project switch, which happens above the sidebar",
    ),
    (
        "sidebar.expanded",
        "tests/logical_state_ownership.rs::expanded_nodes_are_application_owned — expansion \
         survives a worktree re-discovery, which rebuilds the tree widget",
    ),
    (
        "sidebar.filter_open",
        "tests/sidebar_state.rs::escape_dismisses_the_open_filter_panel_when_no_overlay_is_open — \
         Escape reaches the panel only because the application knows it is open",
    ),
    (
        "sidebar.filters",
        "tests/logical_state_ownership.rs::tag_filters_are_application_owned — a filter decides \
         which rows exist, so it survives the panel that set it being dismissed",
    ),
    (
        "sidebar.hidden",
        "tests/logical_state_ownership.rs::sidebar_visibility_is_application_owned — whether the \
         sidebar is shown is the window's layout decision, not the sidebar's",
    ),
    (
        "window.window_size",
        "tests/features_window.rs::the_window_size_is_recorded_as_reported — the size is what the \
         sidebar width is clamped against, so it is read before any component exists",
    ),
];

/// Every module under `src/features/` other than `mod.rs`.
fn feature_modules() -> BTreeSet<String> {
    let dir = src_dir().join("features");
    let mut out = BTreeSet::new();
    for entry in fs::read_dir(&dir).expect("read src/features") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            let stem = path
                .file_stem()
                .expect("file stem")
                .to_string_lossy()
                .into_owned();
            if stem != "mod" {
                out.insert(stem);
            }
        }
    }
    assert!(
        !out.is_empty(),
        "no feature modules found under src/features"
    );
    out
}

/// The feature whose `State` this type is, for the three spellings that compile.
///
/// `crate::features::sidebar::State` is what `app.rs` writes today; `features::sidebar::State` and
/// a bare `sidebar::State` are the same declaration with a `use` in front of it, and a guard that
/// accepted only the longest one would fail the day someone shortened an import.
fn feature_struct(ty: &str, features: &BTreeSet<String>) -> Option<String> {
    let ty: String = ty.chars().filter(|c| !c.is_whitespace()).collect();
    let ty = ty.strip_prefix("crate::").unwrap_or(&ty).to_string();
    let ty = ty.strip_prefix("features::").unwrap_or(&ty).to_string();
    let (name, tail) = ty.split_once("::")?;
    (tail == "State" && features.contains(name)).then(|| name.to_string())
}

/// Which features write each path of the root state, resolved transitively.
///
/// `workspace`'s members are deliberately *not* decomposed here: G2 asks about root fields, and
/// `workspace` is answered whole by [`SHARED`]. Attributing its six members is
/// `feature_write_isolation.rs`'s question, and it uses the same scan to ask it.
fn writers() -> BTreeMap<String, BTreeSet<String>> {
    let sources = sources();
    let app = sources
        .iter()
        .find(|(f, _)| f == "app.rs")
        .map(|(_, s)| s.clone())
        .expect("src/app.rs");
    let fields: BTreeSet<String> = struct_fields(&app, "State").into_iter().collect();
    // The nine feature structs, so the walk can follow `state.sidebar.expanded` into the struct
    // rather than stopping at the field and losing the write.
    let mut nested: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (file, src) in &sources {
        let Some(name) = feature_of(file).filter(|n| fields.contains(n)) else {
            continue;
        };
        if src.contains("pub struct State {") {
            nested.insert(name, struct_fields(src, "State").into_iter().collect());
        }
    }
    let ops: Vec<Operation> = sources
        .iter()
        .flat_map(|(file, src)| operations_in(file, src, "State"))
        .filter(|o| o.mutating)
        .collect();
    let mut unclassified = BTreeSet::new();
    let (writes, _, _) =
        transitive_writes(&ops, &fields, &nested, &BTreeMap::new(), &mut unclassified);
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for op in &ops {
        let Some(feature) = feature_of(&op.file) else {
            continue;
        };
        for path in writes.get(&op.key()).into_iter().flatten() {
            out.entry(path.clone()).or_default().insert(feature.clone());
        }
    }
    out
}

/// [`writers`] folded to the root field each path starts with.
fn writers_by_root_field() -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (path, features) in writers() {
        let root = path.split('.').next().unwrap_or(&path).to_string();
        out.entry(root).or_default().extend(features);
    }
    out
}

fn app_src() -> String {
    code_only(&fs::read_to_string(src_dir().join("app.rs")).expect("read app.rs"))
}

#[test]
fn every_root_field_is_a_feature_struct_or_a_declared_shared_member() {
    let features = feature_modules();
    let shared: BTreeMap<&str, &str> = SHARED.iter().copied().collect();
    let writers = writers_by_root_field();

    let mut loose = Vec::new();
    for (field, ty) in struct_field_types(&app_src(), "State") {
        if feature_struct(&ty, &features).is_some() || shared.contains_key(field.as_str()) {
            continue;
        }
        let by = writers.get(&field).cloned().unwrap_or_default();
        let blame = match by.len() {
            0 => "no feature writes it — it is root-only state, which FR-002 says the root does \
                  not decide about"
                .to_string(),
            1 => format!(
                "written only by `{}` — move it into `features/{}.rs`'s `State`",
                by.iter().next().expect("one"),
                by.iter().next().expect("one")
            ),
            _ => format!(
                "written by {} — if that is right, declare it in SHARED with the reason",
                by.iter()
                    .map(|f| format!("`{f}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        loose.push(format!("  `state.{field}: {ty}` — {blame}"));
    }

    assert!(
        loose.is_empty(),
        "{} loose field(s) in `app::State`. Every public field must be a feature's own `State` \
         or a declared shared member in SHARED (feature 028, FR-014, contract S2):\n{}",
        loose.len(),
        loose.join("\n")
    );
}

/// Files under `src/` whose code mentions `.<path>` — the readers, plus the writers among them.
///
/// Text, not types: after feature 028 every access to a feature's state is spelled through the
/// feature's own name (`state.sidebar.expanded`), which is exactly what makes a textual reader
/// scan honest here. Comments and string literals are stripped first, so this file's own prose
/// and any fixture quoting a field name cannot manufacture a reader.
fn readers(path: &str) -> BTreeSet<String> {
    let needle = format!(".{path}");
    sources()
        .into_iter()
        .filter(|(_, src)| src.contains(&needle))
        .map(|(file, _)| file)
        .collect()
}

/// Whether a path qualifies under FR-007a: one writing feature, and no reader outside that
/// feature's module and a single view.
///
/// "Its view" is resolved as *at most one file under `src/ui/`* rather than by name, because the
/// views are not named after features — the sidebar's filter panel is drawn by `ui/sidebar.rs`,
/// the About dialog by `ui/about.rs`, and neither mapping is derivable from the feature's name. A
/// path read by two views is by definition not component-local: no one component could hold it.
fn qualifies(path: &str, writers: &BTreeMap<String, BTreeSet<String>>) -> Option<String> {
    let by = writers.get(path)?;
    if by.len() != 1 {
        return None;
    }
    let feature = by.iter().next().expect("one").clone();
    let home = format!("features/{feature}.rs");
    let mut views = BTreeSet::new();
    for file in readers(path) {
        if file == home {
            continue;
        }
        if file.starts_with("ui/") {
            views.insert(file);
        } else {
            return None; // read from the root, the shell or another feature
        }
    }
    (views.len() <= 1).then_some(feature)
}

/// Every path FR-007a's rule reaches, with the feature that writes it.
fn component_local_candidates() -> Vec<(String, String)> {
    let writers = writers();
    let mut out: Vec<(String, String)> = writers
        .keys()
        .filter(|path| path.contains('.'))
        .filter_map(|path| qualifies(path, &writers).map(|f| (path.clone(), f)))
        .collect();
    out.sort();
    out
}

#[test]
fn component_local_paths_are_pinned_or_moved() {
    let allowed: BTreeMap<&str, &str> = COMPONENT_LOCAL.iter().copied().collect();
    let unanswered: Vec<String> = component_local_candidates()
        .into_iter()
        .filter(|(path, _)| !allowed.contains_key(path.as_str()))
        .map(|(path, feature)| {
            format!(
                "  `state.{path}` — written only by `{feature}`, read only by that module and one \
                 view. Move it into the component that renders it, or add it to COMPONENT_LOCAL \
                 naming the assertion that pins it to the application."
            )
        })
        .collect();

    assert!(
        unanswered.is_empty(),
        "{} path(s) are component-local and neither moved nor pinned (feature 028, FR-007a):\n{}",
        unanswered.len(),
        unanswered.join("\n")
    );
}

#[test]
fn the_allowlist_names_only_live_violations() {
    let features = feature_modules();
    let fields: BTreeMap<String, String> = struct_field_types(&app_src(), "State")
        .into_iter()
        .collect();

    let mut stale = Vec::new();
    for (path, _) in SHARED {
        match fields.get(*path) {
            None => stale.push(format!(
                "  `{path}` is in SHARED but is no longer a field of `app::State` — delete the entry"
            )),
            Some(ty) if feature_struct(ty, &features).is_some() => stale.push(format!(
                "  `{path}` is in SHARED but its type is now a feature's `State` — the exception \
                 has been resolved, so delete the entry"
            )),
            Some(_) => {}
        }
    }

    let live: BTreeSet<String> = component_local_candidates()
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    for (path, _) in COMPONENT_LOCAL {
        if !live.contains(*path) {
            stale.push(format!(
                "  `{path}` is in COMPONENT_LOCAL but no longer qualifies under FR-007a — either \
                 it moved, or it gained a second writer or a reader outside its view. Delete the \
                 entry."
            ));
        }
    }

    assert!(
        stale.is_empty(),
        "{} allowlist entry/entries no longer describe a live exception:\n{}",
        stale.len(),
        stale.join("\n")
    );
}

#[test]
fn the_scan_finds_the_operations_it_is_meant_to_read() {
    // The floor that stops this guard passing because it read nothing. Both numbers are counts of
    // things that exist rather than targets: if a refactor genuinely removes half the feature
    // modules, the floor is what makes that a conversation instead of a silent green.
    let features = feature_modules();
    assert!(
        features.len() >= 9,
        "only {} feature module(s) found — the scan is not reading `src/features/`",
        features.len()
    );
    // Two floors, because this scan has twice gone quiet in a way that looked like a pass. The
    // first is the one it had: are the reducers being read at all. The second is the one it
    // needed: are their *writes* being resolved. Feature 028 moved every field behind a struct,
    // and only the second floor would have caught the walk losing sight of them (T042).
    let writers = writers();
    let roots = writers_by_root_field();
    assert!(
        roots.len() >= 9,
        "only {} root field(s) resolved to a writing feature — the `&mut State` scan is not \
         seeing the reducers",
        roots.len()
    );
    assert!(
        writers.len() >= 40,
        "only {} distinct path(s) written across the whole app — feature 028 left 43 fields \
         behind nine feature structs, so the walk into them has stopped resolving",
        writers.len()
    );
    assert!(
        Path::new(&workspace_rs()).exists(),
        "micold-core's workspace.rs is not where the scan expects it"
    );
}
