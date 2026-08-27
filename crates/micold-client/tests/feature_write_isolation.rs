//! No feature writes another feature's data (feature 021, T059 — FR-020, FR-024a, SC-007,
//! contract O1/O6).
//!
//! # The rule, and why it is asymmetric
//!
//! A feature may **read** any of the shared state — the sidebar renders session data, and
//! FR-003a requires that to stay possible and cheap. A feature may not **write** another
//! feature's data; it returns an [`Outcome`](micold_client::features::Outcome) and the root
//! applies it (FR-020, FR-021).
//!
//! That asymmetry is why this is a guard test and not a type. Partitioning `State` into mutually
//! invisible halves would break the reads the spec's Edge Cases require. plan.md's Complexity
//! Tracking records the deviation from Principle V; this file is what pays for it.
//!
//! # Ownership is read off the declaration, except once
//!
//! Since feature 028 a root field's *type* names its owner: `pub sidebar:
//! crate::features::sidebar::State` is the claim, the compiler checks it, and nobody can forget to
//! update it. Nine of the ten root fields answer that way and this file no longer restates them.
//!
//! The tenth is `workspace`, which holds the project catalog, the session lists and two worktree
//! maps in a single core value. A field-level claim would have to assign it to one feature and
//! would then be wrong for the other two, so [`OWNERS`] survives keyed by *path* and `workspace`
//! appears only through its six members. Anything writing `state.workspace` **whole** is writing
//! all three features' data at once, and is reported as such.
//!
//! # What counts as a feature's code today
//!
//! Tier 1 left each feature's operations as `impl State` blocks inside its module — `features/
//! session.rs` says so in its own header — so a "feature reducer" is currently any `&mut self`
//! method on `State` defined under `src/features/`. When T062 turns these into free functions
//! taking `&mut State`, the scan picks those up too: it reads the binding name out of the
//! signature rather than assuming `self`.
//!
//! # Indirection is followed, because that is where the writes hide
//!
//! `restore_after_activation` does not write the notification queue; it calls `arm_notice`, which
//! calls `push_notification`, which does. A scan that stopped at the first call would report the
//! session feature as clean while it raises notifications. So every `&mut self` method on `State`
//! is resolved to the set of paths it writes, transitively to a fixed point, and a caller inherits
//! its callees' writes.
//!
//! # A core type's own operation is not a cross-feature write
//!
//! The corollary of keying ownership by path is that `Workspace`'s six members answer to three
//! different features — which is right for an ordinary write and wrong for `Workspace`'s own
//! methods. `forget` clears everything held against a project's path because that is its
//! invariant; it is core code writing core's own members. A write a feature reaches *only* through
//! such a call is therefore exempt, and listed in [`CORE_MEDIATED`] rather than [`ALLOWED`] — not
//! debt to be converted, but a fact to be acknowledged (T067a-3).
//!
//! The exemption is narrow in both directions. It does not apply to a path the operation also
//! writes on a line of its own, and it does not apply silently: an unlisted one fails
//! [`core_mediated_writes_are_inventoried`], so a feature reaching a neighbour through some other
//! core method has to say which invariant that method carries.
//!
//! # The allowlist is the point, not an escape hatch
//!
//! The violations below exist today; this test is what makes them countable. T067 transcribes
//! [`ALLOWED`] into `specs/021-mvu-slice-architecture/cross-feature-writes.md` with a proposed
//! outcome for each, and T067a converts them one commit at a time. Two rules keep it honest:
//! nothing may be added without a task reference, and
//! [`the_allowlist_names_only_live_violations`] fails if an entry stops being a violation — so a
//! conversion that forgets to delete its line is caught by the same test that permitted it.
//!
//! # What this cannot see, stated rather than discovered later
//!
//! - **Writes through a function that does not take `&mut State`.** A helper handed
//!   `&mut state.worktree.worktrees` is flagged at the `&mut` site, but one handed an owned value that is
//!   later assigned back is not.
//! - **Interior mutability.** Nothing in `State` uses it today; if that changes, this scan goes
//!   quiet rather than loud.
//! - **Method calls it has never seen.** That one is not silent: an unclassified method on a state
//!   path fails [`every_method_called_on_state_is_classified`] asking to be sorted into
//!   [`state_scan::MUTATORS`] or [`state_scan::READERS`], because guessing would make the guard
//!   leak in whichever direction the guess was wrong.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

/// Which feature owns each member of the one field whose type cannot say (feature 028, T047).
///
/// It used to have 51 rows, and then 15: one per root field, restating in a `const` what the field
/// was already called. Feature 028 gave every feature its own `State` struct, so
/// `pub sidebar: crate::features::sidebar::State` **is** the ownership claim — the compiler checks
/// it, and nobody can forget to update it. [`declared_owners`] reads those nine straight off the
/// declaration, and this table shrank to what the declaration cannot express (SC-007).
///
/// What it cannot express is `workspace`. `Workspace` holds the project catalog, the session lists
/// and two worktree maps in a single core value; a field-level claim would have to name one feature
/// and would then be wrong for the other two. So it is keyed by member, and anything writing
/// `state.workspace` **whole** is writing all three features' data at once and is reported as such.
///
/// The split is an artefact of how this guard is written rather than a boundary through the middle
/// of a core type — which is what [`CORE_MEDIATED`] exists to say.
const OWNERS: &[(&str, &str)] = &[
    ("workspace.projects", "project"),
    ("workspace.active", "project"),
    ("workspace.sessions", "session"),
    ("workspace.foreground_by_project", "session"),
    ("workspace.worktree_names", "worktree"),
    ("workspace.included_worktrees", "worktree"),
];

/// Cross-feature writes that exist today, each with the feature that performs it and the path it
/// reaches into.
///
/// **It is empty, and that is the point.** T059 found eight on the Tier 1 tree; T062 moved the
/// reducer arms inside `src/features/` where the scan could finally see them and it stood at 43;
/// T067 catalogued those into `cross-feature-writes.md` and T067a converted them a commit at a
/// time, ending at zero when group C's twelve popover rows became a declaration (T067a-2). The
/// comments left behind in place of each group are deliberate: what a row *was* is the only record
/// of why the code is shaped the way it is now, and four of the seven groups turned out not to
/// want the outcome the catalogue proposed.
///
/// **Adding an entry requires a task reference in the note**, and an entry is now debt rather than
/// inventory — [`no_feature_writes_another_features_data`] is the test that fails without one, and
/// [`the_allowlist_names_only_live_violations`] is what stops a line outliving the write it
/// permitted. Two things that are *not* violations have their own homes: a feature writing its own
/// field was never one, and a write reached only through a core method is inventoried in
/// [`CORE_MEDIATED`].
const ALLOWED: &[(&str, &str, &str)] = &[
    // (The reveal's eight rows lived here until T067a-6 converted them. They were never the
    // reveal: the revealed row is *derived* by `location_open` and never written, and what these
    // wrote was the moment a reveal ends — the outgoing row folded into the user's own set.
    // `LocationOpened`, `RevealScrollArmed`, `ProjectEntered` and `RevealSuppressed` now carry it,
    // and `restore_after_activation`'s `show_agent_worktrees` row — once its own task — turned out
    // to be the same fact as `ProjectEntered` and went with them.)
    // (T059 recorded a row here — `restore_after_activation` writing `focused_field` via
    // `State::focus_terminal` — and asked whether it was a violation at all, or whether
    // `focus_terminal` was a session operation sitting in the wrong file. **T067a-7 answered: the
    // wrong file.** The function moved into `features/session.rs` and this row, with five others,
    // collapsed into the single `focus_terminal` entry below.)
    // (`session::switch_active` -> `workspace.active` sat here from T059 until T067a-3. It goes
    // through `Workspace::activate`, which made it the same question as the `Workspace::forget`
    // block below rather than a row of its own — see CORE_MEDIATED.)
    // =============================================================================================
    // T062 — the reducer arms, which the guard could not see until they became feature code.
    //
    // None of these is new behaviour. Every one was an arm of `State::update` in `app.rs`, which
    // this scan has never read: it only ever looked under `src/features/`. Tier 3 moved the arms
    // into the modules that own them, and the same writes are now inside the boundary the guard
    // watches. So the count went from 8 to 35 without a line of behaviour changing — the earlier
    // 8 were the violations in the code that had *already* moved, not the violations that exist.
    //
    // They are grouped below by what actually performs the write, because most of them are not a
    // feature reaching into a neighbour at all: they are a **root helper** that writes across
    // features, attributed to whichever feature called it. That distinction is what T067 has to
    // resolve — an outcome per group, not one per line.
    // =============================================================================================

    // --- group A is gone: `clear_for_dialog` moved (T067a-5) --------------------------------------
    // Eight rows sat here, one per feature that opens a dialog, all writing `focused_field`. **None
    // of them wrote it.** `State::clear_for_dialog` did, and it was root code the guard could not
    // attribute, so it reported the callers instead. T067a-5 moved it into `features/window.rs`,
    // which has owned `focused_field` since T063 — and a feature writing its *own* field is not a
    // cross-feature write at all. Eight rows retired, no outcome written, no `DialogOpened` needed.
    //
    // Second time this shape appeared; T067a-7 found the first under `focus_terminal`. The lesson
    // both times: a row count is not a violation count when the writer is root code.
    // --- `session::focus_terminal` (T067a-7) -----------------------------------------------------
    // Putting a terminal in front of the user gives it the keyboard, which clears whatever field
    // held it (FR-011). **This was five rows, plus the one above, until T067a-7 moved the function
    // out of `app.rs`.** Converting first would have given six reducers an outcome apiece for a
    // write none of them performs — the guard reported callers because the writer was root code it
    // could not attribute. One function writes it; one row names it.
    // --- group C is gone: displacement is declared, not assigned (T067a-2) -----------------------
    // Twelve rows sat here, three or four per popover toggle. T067 guessed at "one outcome — a
    // popover opened — applied by the root to every other registered popover", and the guess was
    // right about the shape and wrong about the rule: there is no uniform rule. The project row
    // menu deliberately leaves the switcher it was opened from alone, the worktree menu closes
    // only the project one, and the session and terminal menus close nothing. So a surface now
    // *declares* what it displaces (`FloatingSurface::displaces`), `Outcome::SurfaceOpened` says
    // one opened, and `overlay::registry::displace` closes each through the cancellation that
    // surface already declared. `tests/popover_displacement.rs` states the whole relation
    // independently — all forty-two ordered pairs, including the thirty nobody had a test for.
    // --- group D is gone: `Workspace::forget` is core's own operation (T067a-3) -------------------
    // Four rows sat here, one per `workspace` member that forgetting a project clears. T067 called
    // `ProjectForgotten` "the clearest case in the whole list" and it was the wrong call: `forget`
    // is `micold-core` code writing `Workspace`'s own members, and converting it would have three
    // client features each apply one clause of a core invariant, with a half-applied forget
    // leaving the workspace inconsistent. The guard's model was what needed changing, not the
    // code. They are inventoried in CORE_MEDIATED instead.
    // --- via `State::push_notification`, a root helper (T067) ------------------------------------
    // `session::arm_notice` above is what is left of this group: T067a converted
    // `project::open_refused` to `notifications::error(..)` -> `Outcome::NotificationRaised`, the
    // first conversion of the burn-down, and `arm_notice` follows once its three-function chain
    // (`switch_active` -> `restore_after_activation` -> `arm_notice`) is converted with it.
    // (T066 converted `worktree::loaded` -> `expanded`, the entry that used to sit here. It is the
    // first conversion of this feature and the proof the mechanism works end to end: `set_worktrees`
    // now returns `Outcome::WorktreesReplaced` and the sidebar prunes its own expansion. The
    // remaining entries are what T067 catalogues and T067a burns down.)
    // --- the form creates; the worktree feature owns the list (T067) ------------------------------
    // `worktree_form` is a separate feature precisely because its lifecycle is independent
    // (FR-003), but the thing it creates lands in `worktree`'s list and its failures land in
    // `worktree`'s error slot. `WorktreeCreated` / `WorktreeCreateFailed` are the outcomes, and
    // T064 — which promotes the form to a nested unit with its own message type — is where the
    // seam is most visible.
];

/// Writes a feature performs by calling a method of a core type on that type's own members.
///
/// These are **not** violations, and the reason is the one T067a-3 settled: `Workspace` holds the
/// project catalog, the session lists and two worktree maps, and [`OWNERS`] splits those six
/// members across three features because a field-level map has to say *something*. That split is
/// an artefact of how this guard is written; it is not a boundary running through the middle of a
/// core type. `Workspace::forget` clears everything held against a path — that is its invariant,
/// argued outright in its own comment — and asking three client features to each apply one clause
/// of it would make a half-applied forget expressible for the first time.
///
/// So the rule is the same one that already exempts a feature writing its own field: the code that
/// performs the write owns what it writes. The exemption is narrow — it applies only when the
/// operation reaches the path *solely* through the core call. A feature that also writes
/// `state.workspace.sessions` on a line of its own is reported as before, and
/// [`the_exemption_is_narrow`] holds that.
///
/// **It is inventoried rather than invisible.** This guard has twice gone quiet where it looked
/// green (T067a-6, and the `mut_param` floor before it), so an exemption that reported nothing
/// would be the third. Every core-mediated write is listed here and
/// [`core_mediated_writes_are_inventoried`] fails on any that is not — a feature reaching a
/// neighbour's data through some *other* core method has to add a line and say why, which is the
/// same discipline [`ALLOWED`] imposes without the implication that the line is debt.
const CORE_MEDIATED: &[(&str, &str, &str, &str)] = &[
    // Switching the active project. Step 2 of a three-step sequence the session feature brackets:
    // record the outgoing foreground, activate, restore the incoming one (T067a-7).
    (
        "session",
        "workspace.active",
        "features/session.rs::switch_active",
        "Workspace::activate",
    ),
    // Forgetting a project drops everything held against its path, and three features hold
    // something: its sessions and foreground choice, its worktree names and its inclusions.
    (
        "project",
        "workspace.foreground_by_project",
        "features/project.rs::forget_confirmed",
        "Workspace::forget",
    ),
    (
        "project",
        "workspace.included_worktrees",
        "features/project.rs::forget_confirmed",
        "Workspace::forget",
    ),
    (
        "project",
        "workspace.sessions",
        "features/project.rs::forget_confirmed",
        "Workspace::forget",
    ),
    (
        "project",
        "workspace.worktree_names",
        "features/project.rs::forget_confirmed",
        "Workspace::forget",
    ),
];

/// The shared source-text scan (feature 028, T042): what every operation on `State` writes,
/// resolved transitively. It lived in this file from T059 until G2 needed the same walk to name
/// the single writer of a loose root field; see the module's own header for why it is shared
/// rather than copied.
#[path = "support/state_scan.rs"]
mod state_scan;

use state_scan::{
    code_only, feature_of, operations_in, sources, src_dir, struct_field_types, struct_fields,
    transitive_writes, workspace_rs, Call, Operation,
};

/// `crate::features::<n>::State` -> `<n>`, for the three spellings that compile.
///
/// A guard that accepted only the longest one would fail the day someone added a `use`.
fn feature_of_type(ty: &str) -> Option<String> {
    let ty: String = ty.chars().filter(|c| !c.is_whitespace()).collect();
    let ty = ty.strip_prefix("crate::").unwrap_or(&ty).to_string();
    let ty = ty.strip_prefix("features::").unwrap_or(&ty).to_string();
    let (name, tail) = ty.split_once("::")?;
    (tail == "State").then(|| name.to_string())
}

/// The root fields whose own type names their owner — the nine feature structs.
fn declared_owners() -> BTreeMap<String, String> {
    let app = code_only(&fs::read_to_string(src_dir().join("app.rs")).expect("read app.rs"));
    struct_field_types(&app, "State")
        .into_iter()
        .filter_map(|(field, ty)| feature_of_type(&ty).map(|f| (field, f)))
        .collect()
}

/// Ownership as this guard resolves it: read off the declaration where the type says so, and taken
/// from [`OWNERS`] for the one field whose type cannot.
fn owners() -> BTreeMap<String, String> {
    let mut out = declared_owners();
    out.extend(OWNERS.iter().map(|(p, f)| (p.to_string(), f.to_string())));
    out
}

/// The whole analysis, run once per test that needs it.
struct Scan {
    /// Cross-feature writes, as `(feature, path, "file::operation")`.
    violations: Vec<(String, String, String)>,
    /// Writes a feature reaches only through a core method, as
    /// `(feature, path, "file::operation", "Workspace::method")`. Not violations; see
    /// [`CORE_MEDIATED`].
    core_mediated: Vec<(String, String, String, String)>,
    /// Methods called on a state path that are in neither table.
    unclassified: BTreeSet<String>,
    /// Mutating operations found under `src/features/`.
    feature_ops: usize,
    /// Fields a write can resolve to: `app::State`'s own, plus the members of every
    /// feature struct it holds. Both levels, so feature 028's moves leave it unchanged.
    state_fields: usize,
}

fn scan() -> Scan {
    let sources = sources();
    let workspace_src = code_only(&fs::read_to_string(workspace_rs()).expect("read workspace.rs"));
    let workspace_fields: BTreeSet<String> = struct_fields(&workspace_src, "Workspace")
        .into_iter()
        .collect();

    // `Workspace`'s own API first: its methods write members belonging to three features, so they
    // have to be resolved before any call to one can be judged.
    let ws_ops = operations_in("workspace.rs", &workspace_src, "Workspace");
    let mut unclassified = BTreeSet::new();
    let ws_mutating: Vec<Operation> = ws_ops
        .iter()
        .filter(|o| o.mutating)
        .map(|o| Operation {
            file: o.file.clone(),
            name: o.name.clone(),
            binding: o.binding.clone(),
            mutating: o.mutating,
            body: o.body.clone(),
        })
        .collect();
    let (ws_writes, _, _) = transitive_writes(
        &ws_mutating,
        &workspace_fields,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &mut unclassified,
    );
    // Back to bare method names: `nested_api` is consulted at a call site, which says
    // `state.workspace.forget(…)` and knows nothing about the file the method was found in.
    // `Workspace`'s methods are all in one file, so the qualification carries no information here.
    let ws_writes: BTreeMap<String, BTreeSet<String>> = ws_writes
        .into_iter()
        .map(|(k, v)| {
            let name = k.rsplit("::").next().unwrap_or(&k).to_string();
            (name, v.iter().map(|p| format!("workspace.{p}")).collect())
        })
        .collect();
    let mut workspace_api: BTreeMap<String, Call<'_>> = BTreeMap::new();
    for op in &ws_ops {
        if !op.mutating {
            workspace_api.insert(op.name.clone(), Call::Reads);
        }
    }
    for (name, paths) in &ws_writes {
        workspace_api.insert(name.clone(), Call::Writes(paths));
    }
    let nested_api: BTreeMap<String, BTreeMap<String, Call<'_>>> =
        [("workspace".to_string(), workspace_api)]
            .into_iter()
            .collect();

    let app = sources
        .iter()
        .find(|(f, _)| f == "app.rs")
        .map(|(_, s)| s.clone())
        .expect("src/app.rs");
    let state_field_list = struct_fields(&app, "State");
    let fields: BTreeSet<String> = state_field_list.iter().cloned().collect();

    // What a write can resolve to, for the non-vacuity floor below. Feature 028 moves each
    // feature's fields behind a struct of its own, so the *root's* field count falls as the
    // refactor lands — 60 before it started, headed for the ten-odd members left once every
    // feature owns its own. Counting only the root would turn the floor into a countdown, and a
    // floor lowered every commit stops catching what it was for. So count both levels: the root's
    // fields, plus the members of every feature struct the root holds. The move preserves that
    // total, which is the point of the move.
    let resolvable_fields = state_field_list.len()
        + sources
            .iter()
            .filter(|(file, src)| {
                feature_of(file).is_some_and(|n| fields.contains(&n))
                    && src.contains("pub struct State {")
            })
            .map(|(_, src)| struct_fields(src, "State").len())
            .sum::<usize>();

    let ops: Vec<Operation> = sources
        .iter()
        .flat_map(|(file, src)| operations_in(file, src, "State"))
        .filter(|o| o.mutating)
        .collect();
    // Every field whose own members this scan can name: the core type the root still holds
    // flat, and each of the nine feature structs feature 028 moved the rest behind. Without the
    // second group a reducer's `state.sidebar.expanded.insert(..)` reads as a member access and
    // the write vanishes — see [`state_scan::reach`].
    let mut nested: BTreeMap<String, BTreeSet<String>> =
        [("workspace".to_string(), workspace_fields.clone())]
            .into_iter()
            .collect();
    for (file, src) in &sources {
        let Some(name) = feature_of(file).filter(|n| fields.contains(n)) else {
            continue;
        };
        if src.contains("pub struct State {") {
            nested.insert(name, struct_fields(src, "State").into_iter().collect());
        }
    }
    let (writes, core, direct) =
        transitive_writes(&ops, &fields, &nested, &nested_api, &mut unclassified);

    // Report each write once, at the innermost feature operation that reaches it. Without this a
    // single `self.sidebar.expanded.insert(..)` is reported three times over — at the write, and again at
    // each feature-module caller that inherits it — and the allowlist would then name callers that
    // converting the write would silently fix, which is the opposite of what T067a needs.
    // Every key an operation of that bare name resolves to, and whether any of them is a feature
    // operation. A call site names only the bare method, so both are needed.
    let mut keys_named: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut in_features: BTreeSet<&str> = BTreeSet::new();
    for op in &ops {
        keys_named
            .entry(op.name.as_str())
            .or_default()
            .push(op.key());
        if feature_of(&op.file).is_some() {
            in_features.insert(op.name.as_str());
        }
    }

    let owners = owners();
    let mut violations = Vec::new();
    let mut core_mediated = Vec::new();
    for op in &ops {
        let Some(feature) = feature_of(&op.file) else {
            continue;
        };
        let Some(paths) = writes.get(&op.key()) else {
            continue;
        };
        for path in paths {
            // Keyed by path, then by the path's first segment. `workspace.sessions` is named
            // outright because its five siblings answer to other features; `sidebar.expanded` is
            // not, because feature 028 made the first segment the feature — the field *is* the
            // struct, so naming every member would be a second copy of the declaration.
            let owner = owners
                .get(path)
                .or_else(|| owners.get(path.split('.').next().unwrap_or(path)))
                .unwrap_or_else(|| panic!("`{path}` has no owner — add it to OWNERS"));
            if owner == &feature {
                continue;
            }
            let inherited_from_a_feature = direct
                .get(&op.key())
                .map(|r| {
                    r.calls.iter().any(|callee| {
                        in_features.contains(callee.as_str())
                            && keys_named
                                .get(callee.as_str())
                                .into_iter()
                                .flatten()
                                .any(|k| writes.get(k).is_some_and(|w| w.contains(path)))
                    })
                })
                .unwrap_or(false);
            // ...but only when the call is the *whole* story. An operation that writes the path on
            // its own line and *also* calls a sibling that writes it had its direct write
            // attributed away by this suppression — silently, since a suppressed row looks
            // identical to no row at all. `session::started` is the case that exposed it: it
            // writes `expanded` and `default_expanded` itself and then calls
            // `set_current_session`, which writes them too, so both of its own writes went
            // unreported and unlisted while the guard stayed green (T067a-6).
            let written_directly = direct
                .get(&op.key())
                .is_some_and(|r| r.writes.contains(path));
            // A path this operation reaches *only* through a `Workspace` method is not the feature
            // writing a neighbour: it is core code writing its own members, and the ownership map
            // splits `workspace` across three features for the guard's benefit, not because a
            // boundary runs through the middle of that type. Reporting it asks three client
            // features to each re-implement one clause of a core invariant (T067a-3). It is still
            // counted — in CORE_MEDIATED, so a *new* one has to be acknowledged rather than
            // arriving silently.
            let via_core = core
                .get(&op.key())
                .and_then(|c| c.get(path))
                .filter(|_| !written_directly);
            if let Some(methods) = via_core {
                for method in methods {
                    core_mediated.push((
                        feature.clone(),
                        path.clone(),
                        op.key(),
                        format!("Workspace::{method}"),
                    ));
                }
                continue;
            }
            if inherited_from_a_feature && !written_directly {
                continue;
            }
            violations.push((feature.clone(), path.clone(), op.key()));
        }
    }
    violations.sort();
    violations.dedup();
    core_mediated.sort();
    core_mediated.dedup();
    Scan {
        violations,
        core_mediated,
        unclassified,
        feature_ops: ops.iter().filter(|o| feature_of(&o.file).is_some()).count(),
        state_fields: resolvable_fields,
    }
}

#[test]
fn every_state_field_has_an_owner() {
    let app = code_only(&fs::read_to_string(src_dir().join("app.rs")).expect("read app.rs"));
    let fields = struct_fields(&app, "State");
    let owners = owners();
    let missing: Vec<&String> = fields
        .iter()
        .filter(|f| *f != "workspace" && !owners.contains_key(*f))
        .collect();
    assert!(
        missing.is_empty(),
        "these `State` fields have no owning feature: {missing:?}\n\
         Add each to OWNERS. A field nobody owns is a field this guard cannot police."
    );
    let stale: Vec<&str> = OWNERS
        .iter()
        .map(|(p, _)| *p)
        .filter(|p| !p.starts_with("workspace."))
        .collect();
    assert!(
        stale.is_empty(),
        "OWNERS names {stale:?}, which is not a `workspace` member. Since T047 the table holds \
         only the split of the one field whose type cannot name its owner; a root field says who \
         owns it by being declared `crate::features::<n>::State`."
    );
}

#[test]
fn every_workspace_field_has_an_owner() {
    let src = code_only(&fs::read_to_string(workspace_rs()).expect("read workspace.rs"));
    let fields = struct_fields(&src, "Workspace");
    let owners = owners();
    let missing: Vec<&String> = fields
        .iter()
        .filter(|f| !owners.contains_key(&format!("workspace.{f}")))
        .collect();
    assert!(
        missing.is_empty(),
        "these `Workspace` members have no owning feature: {missing:?}\n\
         `state.workspace` carries three features' data; each member needs its own OWNERS entry."
    );
    let known: BTreeSet<String> = fields.iter().map(|f| format!("workspace.{f}")).collect();
    let stale: Vec<&str> = OWNERS
        .iter()
        .map(|(p, _)| *p)
        .filter(|p| p.starts_with("workspace.") && !known.contains(*p))
        .collect();
    assert!(
        stale.is_empty(),
        "OWNERS names `workspace` members that no longer exist: {stale:?}"
    );
}

/// Separate from the rule below **because a probe collision says it has to be**.
///
/// An unclassified method and a planted cross-feature write both used to fail
/// [`no_feature_writes_another_features_data`] and nothing else, so no failure set could tell the
/// two probes apart. Feature 021 T055 hit that shape and split the test rather than drop a probe;
/// BUG-004 hit it again. This is the third.
#[test]
fn every_method_called_on_state_is_classified() {
    let scan = scan();
    assert!(
        scan.unclassified.is_empty(),
        "these methods are called on state paths and are in neither MUTATORS nor READERS: {:?}\n\
         Classify each in this file. Guessing would make the guard leak in whichever direction the \
         guess was wrong.",
        scan.unclassified
    );
}

#[test]
fn no_feature_writes_another_features_data() {
    let scan = scan();
    let allowed: BTreeSet<(&str, &str, &str)> = ALLOWED.iter().copied().collect();
    let owners = owners();
    let unexpected: Vec<String> = scan
        .violations
        .iter()
        .filter(|(f, p, w)| !allowed.contains(&(f.as_str(), p.as_str(), w.as_str())))
        .map(|(feature, path, site)| {
            let owner = owners.get(path).cloned().unwrap_or_default();
            format!("  {site} — the `{feature}` feature writes `state.{path}`, owned by `{owner}`")
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "cross-feature writes with no entry in ALLOWED (FR-020, contract O1):\n{}\n\n\
         Return an `Outcome` and let the root apply it (FR-021), or — if this is pre-existing work \
         being catalogued — add it to ALLOWED naming the task that will convert it.",
        unexpected.join("\n")
    );
}

#[test]
fn the_allowlist_names_only_live_violations() {
    let scan = scan();
    let live: BTreeSet<(&str, &str, &str)> = scan
        .violations
        .iter()
        .map(|(f, p, w)| (f.as_str(), p.as_str(), w.as_str()))
        .collect();
    let dead: Vec<String> = ALLOWED
        .iter()
        .filter(|entry| !live.contains(*entry))
        .map(|(f, p, w)| format!("  {w} — `{f}` writing `state.{p}`"))
        .collect();
    assert!(
        dead.is_empty(),
        "ALLOWED names writes that no longer happen:\n{}\n\n\
         Delete each line. An allowlist that outlives what it permitted is how the next real \
         violation gets waved through.",
        dead.join("\n")
    );
}

#[test]
fn core_mediated_writes_are_inventoried() {
    let scan = scan();
    let listed: BTreeSet<(&str, &str, &str, &str)> = CORE_MEDIATED.iter().copied().collect();
    let live: BTreeSet<(&str, &str, &str, &str)> = scan
        .core_mediated
        .iter()
        .map(|(f, p, w, m)| (f.as_str(), p.as_str(), w.as_str(), m.as_str()))
        .collect();
    let unlisted: Vec<String> = live
        .difference(&listed)
        .map(|(f, p, w, m)| format!("  {w} — `{f}` reaches `state.{p}` through `{m}`"))
        .collect();
    assert!(
        unlisted.is_empty(),
        "core-mediated writes with no entry in CORE_MEDIATED (T067a-3):\n{}\n\n\
         These do not fail `no_feature_writes_another_features_data`, which is exactly why each \
         has to be named here. Add a line saying which core invariant the method carries — or, if \
         it carries none and is a single-member setter being used to reach into a neighbour, \
         return an `Outcome` instead (FR-021).",
        unlisted.join("\n")
    );
    let dead: Vec<String> = listed
        .difference(&live)
        .map(|(f, p, w, m)| format!("  {w} — `{f}` reaching `state.{p}` through `{m}`"))
        .collect();
    assert!(
        dead.is_empty(),
        "CORE_MEDIATED names writes that no longer happen:\n{}\n\n\
         Delete each line, for the same reason ALLOWED may not outlive what it permitted.",
        dead.join("\n")
    );
}

#[test]
fn the_exemption_is_narrow() {
    // The exemption is the guard's only silent path, so it has to be shown to still fail. These
    // two say what "solely through the core call" means, in the shape a probe would take: a
    // feature that writes a `workspace` member on a line of its own is a violation whether or not
    // it also calls a core method, and every path in CORE_MEDIATED is a real cross-feature path —
    // one whose owner is a different feature — rather than a row the exemption invented.
    let owners = owners();
    for (feature, path, site, method) in CORE_MEDIATED {
        let owner = owners
            .get(*path)
            .unwrap_or_else(|| panic!("`{path}` has no owner"));
        assert_ne!(
            owner, feature,
            "{site} — `{path}` is owned by `{feature}` itself, so `{method}` is not mediating \
             anything and this line does not belong in CORE_MEDIATED"
        );
    }
    let scan = scan();
    let exempted: BTreeSet<(&str, &str)> = scan
        .core_mediated
        .iter()
        .map(|(_, p, w, _)| (p.as_str(), w.as_str()))
        .collect();
    let both: Vec<String> = scan
        .violations
        .iter()
        .filter(|(_, p, w)| exempted.contains(&(p.as_str(), w.as_str())))
        .map(|(f, p, w)| format!("  {w} — `{f}` and `state.{p}`"))
        .collect();
    assert!(
        both.is_empty(),
        "these are reported as violations *and* exempted, which the scan should make \
         impossible:\n{}",
        both.join("\n")
    );
}

#[test]
fn the_scan_finds_the_operations_it_is_meant_to_read() {
    let scan = scan();
    // 8 when T059 wrote this, and it stayed 8 through T062 — which was the whole problem. The
    // ninety-odd reducer functions Tier 3 created were invisible, because `mut_param` compared
    // `&mutState` against `State` and never matched a free function, so the guard reported every
    // feature clean while this floor sat happily above a number that had not moved. A floor that
    // cannot notice ninety operations arriving cannot notice ninety leaving either.
    assert!(
        scan.feature_ops >= 100,
        "the scan found only {} mutating operations under src/features/ — it found 116 after \
         T062, and a scan that has gone quiet reports every feature as clean",
        scan.feature_ops
    );
    assert!(
        scan.state_fields >= 40,
        "`State` and the feature structs it holds parsed to only {} fields between them — the \
         struct scan is not reading what it thinks it is",
        scan.state_fields
    );
}
