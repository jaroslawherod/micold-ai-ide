//! Adding a floating surface costs one module and one line — permanently (feature 021, T039 —
//! SC-001, SC-002a).
//!
//! # Why this is a test and not a measurement
//!
//! SC-001 was originally to be verified by counting the files a new surface touches. A count proves
//! the property on the day it is taken and says nothing about the day after, which is the whole
//! problem it was meant to describe: nothing in the old structure *resisted* accretion, and a
//! number in a spec resists it no better. SC-002a made the guard permanent for exactly that reason,
//! and this file is it.
//!
//! # What "costs one module and one line" means operationally
//!
//! A surface that only its own module and the registry can name is a surface nothing else can be
//! special-cased on. That is the whole mechanism: if `ui/mod.rs` cannot write `AboutDialog`, it
//! cannot grow an arm for the About dialog, and the six central matches Tier 2 deleted cannot come
//! back one surface at a time. So the reachability check *is* the SC-001 check, and the "zero
//! central match statements" half follows from it rather than needing a separate count.
//!
//! Three ways a surface could leak, and one test each:
//!
//! 1. **By path.** Any cross-module reference to a Rust item names it — `use crate::features::…::X`
//!    or `crate::features::…::X` inline. Caught by [`a_surface_is_named_only_where_it_lives_and_where_it_registers`].
//! 2. **By glob.** `use crate::features::help::*;` then a bare `HelpMenu` names no path, so the
//!    scan above would miss it. Caught by [`nothing_glob_imports_a_feature_module`], which closes
//!    the hole rather than making the first test cleverer.
//! 3. **By id string.** A central match can be written over `open.id().as_str()` instead of over
//!    types, and would name no surface *type* at all. Caught by
//!    [`no_file_enumerates_surfaces_by_id`].
//!
//! # The list comes from the registry, not from here
//!
//! Parsed out of `register!` in `overlay/registry.rs`, so a surface added tomorrow is covered
//! without anyone remembering this file exists — which is what makes the guard permanent rather
//! than a second list to keep in step. A parser that silently found nothing would make all three
//! tests pass vacuously, so [`the_registration_list_is_actually_being_read`] holds the parse
//! against `registry::probes()`, the runtime article.

mod inventory;

use micold_client::features::help;
use micold_client::features::project;
use micold_client::features::settings;
use micold_client::features::worktree_form;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use micold_client::overlay::registry;

/// A registered surface, as the registration line names it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Surface {
    /// The feature module that owns it, e.g. `help`.
    module: String,
    /// The type, e.g. `AboutDialog`.
    name: String,
}

impl Surface {
    /// The one file allowed to define it.
    fn home(&self) -> String {
        format!("features/{}.rs", self.module)
    }
}

const REGISTRY: &str = "overlay/registry.rs";

/// Every surface named in `register!`, read out of the registry's own source.
fn registered() -> Vec<Surface> {
    let src = inventory::src_dir("overlay");
    let text = std::fs::read_to_string(src.join("registry.rs")).expect("read registry.rs");
    let code = inventory::code_only(&text);

    // The macro *definition* mentions `register!` too, so take the invocation: the last one.
    let at = code
        .rfind("register! {")
        .expect("`register!` invocation has moved or been renamed");
    let body = &code[at..];
    let end = body.find("\n}").expect("unterminated register! invocation");

    // Depth-tracked, because a registration line is no longer the only line naming a surface.
    // T067a-2 gave the macro a `{ displaces: … }` clause, which puts *other* surfaces' type names
    // on lines of their own one level in; parsing those would count each displaced surface as a
    // second registration and every test here would be reading a list that does not exist.
    // `the_registration_list_is_actually_being_read` is what caught it, which is what it is for.
    let mut depth = 0usize;
    let mut surfaces = Vec::new();
    for line in body[..end].lines() {
        let line = line.trim();
        if depth == 1 {
            if let Some(surface) = registration_on(line) {
                surfaces.push(surface);
            }
        }
        depth = depth + line.matches('{').count() - line.matches('}').count();
    }
    surfaces
}

/// The surface a top-level registration line names, or `None` for anything else.
///
/// `crate::features::<module>::<Type>`, optionally followed by `=> <view path>` or by the opening
/// brace of a `{ displaces: … }` clause.
fn registration_on(line: &str) -> Option<Surface> {
    let rest = line
        .trim_end_matches(',')
        .trim_end()
        .trim_end_matches('{')
        .trim_end()
        .strip_prefix("crate::features::")?;
    let path = rest.split("=>").next()?.trim();
    let (module, name) = path.split_once("::")?;
    Some(Surface {
        module: module.to_string(),
        name: name.to_string(),
    })
}

/// Every client source, keyed by a path relative to `src/`, with comments stripped.
///
/// Comments are stripped because prose must be free to name a surface: this file's own subject is
/// discussed in half a dozen doc comments, and a guard that made documentation a violation would
/// be traded away the first time it got in the way.
fn sources() -> BTreeMap<String, String> {
    inventory::sources_under(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
        .into_iter()
        .map(|(path, text)| (path, inventory::code_only(&text)))
        .collect()
}

#[test]
fn the_registration_list_is_actually_being_read() {
    // The vacuity guard. Every other test here iterates the parse, so a parse returning nothing —
    // because the macro was renamed, reformatted onto one line, or moved — would turn all of them
    // into loops over an empty list that pass without looking at anything.
    let surfaces = registered();

    assert_eq!(
        surfaces.len(),
        registry::probes().len(),
        "parsed {} surfaces out of `register!` but the registry reports {} probes — the parse in \
         this file has drifted from the macro it reads, and every other test here is now vacuous:\
         \n  {:?}",
        surfaces.len(),
        registry::probes().len(),
        surfaces
    );
    assert!(
        surfaces.len() >= 16,
        "fewer surfaces than the sixteen Tier 2 registered. If one was genuinely removed, lower \
         this number deliberately; it is here so an empty or half-read parse cannot look healthy"
    );

    let unique: BTreeSet<&Surface> = surfaces.iter().collect();
    assert_eq!(
        unique.len(),
        surfaces.len(),
        "the same surface is registered twice"
    );

    for surface in &surfaces {
        let home = surface.home();
        assert!(
            sources().contains_key(&home),
            "`{}` is registered from `{home}`, which does not exist — the parse is reading \
             something other than module paths",
            surface.name
        );
    }
}

/// Whether `code` names `surface` by path, in either form Rust allows.
///
/// `module::Name` covers the inline path and the single-item `use`. The braced form —
/// `use crate::features::help::{AboutDialog, HelpMenu};` — writes `help::{AboutDialog`, so the
/// straight match misses it; that is not a theoretical gap, it is the shape rustfmt produces the
/// moment a second item is imported from the same module.
fn names(code: &str, surface: &Surface) -> bool {
    if code.contains(&format!("{}::{}", surface.module, surface.name)) {
        return true;
    }
    let opener = format!("{}::{{", surface.module);
    let mut rest = code;
    while let Some(at) = rest.find(&opener) {
        let after = &rest[at + opener.len()..];
        let group = after.find('}').map_or(after, |end| &after[..end]);
        if group
            .split(',')
            .any(|item| item.trim().trim_start_matches("self as ") == surface.name)
        {
            return true;
        }
        rest = after;
    }
    false
}

#[test]
fn a_surface_is_named_only_where_it_lives_and_where_it_registers() {
    // SC-001 itself. A surface no other module can name is a surface no other module can be
    // special-cased on, which is what makes "one module plus one line" a property rather than a
    // measurement.
    let sources = sources();
    let mut leaked = Vec::new();

    for surface in registered() {
        let home = surface.home();
        for (path, code) in &sources {
            if path == &home || path == REGISTRY {
                continue;
            }
            if names(code, &surface) {
                leaked.push(format!(
                    "`{}` is named in `{path}` — it belongs to `{home}`, and the only other place \
                     allowed to know it exists is `{REGISTRY}`",
                    surface.name
                ));
            }
        }
    }

    assert!(
        leaked.is_empty(),
        "a floating surface has escaped its module, so adding one no longer costs one module and \
         one line (SC-001):\n  - {}",
        leaked.join("\n  - ")
    );
}

#[test]
fn nothing_glob_imports_a_feature_module() {
    // Closes the one hole in the test above: a glob import lets another module write a bare
    // `HelpMenu` with no path anywhere for the scan to find. Stated as its own rule rather than by
    // teaching the scan to chase globs, because "no file reaches wholesale into a feature" is worth
    // holding on its own — it is how a boundary stops being a boundary.
    let mut globs = Vec::new();

    for (path, code) in sources() {
        for line in code.lines() {
            let line = line.trim();
            if line.starts_with("use ") && line.contains("features::") && line.contains("::*") {
                globs.push(format!("{path}: {line}"));
            }
        }
    }

    assert!(
        globs.is_empty(),
        "a glob import reaches into a feature module, which would let a surface be named with no \
         path for the reachability guard to see:\n  - {}",
        globs.join("\n  - ")
    );
}

#[test]
fn no_file_enumerates_surfaces_by_id() {
    // The leak that names no type. Dispatch could be re-centralised over identities instead —
    // `match open.id().as_str() { "about" => …, "settings" => … }` — and every other test here
    // would be satisfied.
    //
    // Two ids in one file is the signal, not one: a surface id is an ordinary lowercase word, and
    // `showcase/sections/atoms.rs` has a button labelled "settings" that means nothing of the kind.
    // One incidental literal is a coincidence; two is a list. A feature module holding several of
    // its own surfaces is not enumerating anything, so it is allowed the ones it owns.
    let ids: BTreeMap<String, String> = registry::probes()
        .iter()
        .filter_map(|probe| {
            let mut open = None;
            // Every surface reports its id from a constant, so any state that opens it will do;
            // none is needed — `Open::from` is built from the surface, and the id travels with it.
            for state in states_opening_each_surface() {
                if let Some(found) = probe(&state) {
                    open = Some(found);
                    break;
                }
            }
            open.map(|open| open.id().as_str().to_string())
        })
        .map(|id| (id.clone(), id))
        .collect();

    // Fall back to the registration parse for the home of each id: the surface type's module.
    let home_of: BTreeMap<String, String> = registered()
        .iter()
        .map(|surface| (surface.name.clone(), surface.home()))
        .collect();

    let mut enumerating = Vec::new();
    for (path, code) in sources() {
        if path == REGISTRY {
            continue;
        }
        let found: BTreeSet<&String> = ids
            .keys()
            .filter(|id| code.contains(&format!("\"{id}\"")))
            .collect();
        if found.len() < 2 {
            continue;
        }
        // Allowed only if this file is the home of every surface whose id it names.
        let owns_all = found.iter().all(|id| {
            home_of.iter().any(|(_, home)| {
                home == &path && code.contains(&format!("SurfaceId::new(\"{id}\")"))
            })
        });
        if !owns_all {
            enumerating.push(format!("{path}: {found:?}"));
        }
    }

    assert!(
        enumerating.is_empty(),
        "a file names two or more surface ids without owning them, which is a central match over \
         identities wearing a different hat:\n  - {}",
        enumerating.join("\n  - ")
    );
}

/// One state per surface, enough to make each probe answer.
///
/// Deliberately crude — every popover flag and every dialog's state set at once. This file does not
/// care *which* surface a probe reports, only that it reports one, so it needs no per-surface table
/// and cannot go stale when a surface is added.
fn states_opening_each_surface() -> Vec<micold_client::app::State> {
    use micold_client::app::State;
    use micold_client::features::project::{ProjectMenu, RenameDraft};
    use micold_client::features::worktree::WorktreeRenameDraft;
    use micold_core::selector::Selector;
    use micold_core::session::SessionId;
    use std::path::PathBuf;

    // Everything open at once, then one dialog at a time, because `open_in` for a dialog reads its
    // own state and several would otherwise be shadowed by nothing — they are independent, so a
    // single maximal state answers every probe.
    let all = State {
        project: project::State {
            switcher_open: true,
            menu_open: Some(ProjectMenu {
                path: PathBuf::from("/tmp/p"),
                anchor: (10, 10),
            }),
            rename_draft: Some(RenameDraft {
                path: PathBuf::from("/tmp"),
                text: String::new(),
                error: None,
            }),
            selector: Some(Selector::open_at(PathBuf::from("/tmp"))),
            forget_target: Some(PathBuf::from("/p")),
            ..Default::default()
        },

        settings: settings::State {
            settings_draft: Some(Default::default()),
            ..Default::default()
        },

        worktree_form: worktree_form::State {
            form: Some(Default::default()),
            ..Default::default()
        },
        help: help::State {
            about_open: true,
            help_menu_open: true,
            ..Default::default()
        },
        sidebar_filter_open: true,
        worktree_menu_open: Some(micold_client::features::worktree::WorktreeMenu {
            dir_name: "wt".to_string(),
            anchor: (120, 300),
        }),
        session_menu_open: Some(micold_client::features::session::SessionMenu {
            id: SessionId::new(),
            anchor: (120, 340),
        }),
        terminal_context_menu: Some((4, 2)),
        worktree_delete_target: Some("wt".to_string()),
        worktree_rename_draft: Some(WorktreeRenameDraft {
            dir_name: "wt".to_string(),
            text: String::new(),
            error: None,
        }),
        session_remove_target: Some(SessionId::new()),
        ..State::default()
    };
    vec![all]
}
