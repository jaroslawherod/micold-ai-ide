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
//! # Ownership is a map of *paths*, not of fields
//!
//! `State` has 41 fields, but one of them — `workspace` — holds the project catalog, the session
//! lists and two worktree maps in a single value. A field-level map would have to assign it to one
//! feature and would then be wrong for the other two, so [`OWNERS`] is keyed by path and
//! `workspace` appears only through its six members. Anything writing `state.workspace` **whole**
//! is writing all three features' data at once, and is reported as such.
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
//!   `&mut state.worktrees` is flagged at the `&mut` site, but one handed an owned value that is
//!   later assigned back is not.
//! - **Interior mutability.** Nothing in `State` uses it today; if that changes, this scan goes
//!   quiet rather than loud.
//! - **Method calls it has never seen.** That one is not silent: an unclassified method on a state
//!   path fails [`every_method_called_on_state_is_classified`] asking to be sorted into
//!   [`MUTATORS`] or [`READERS`], because guessing would make the guard leak in whichever
//!   direction the guess was wrong.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Which feature owns each writable path of the shared state.
///
/// `root` means no feature owns it: the pointer, the window and the single focus slot are facts
/// about the application, and the root reducer is entitled to write them. Every other entry names
/// a module under `src/features/`.
const OWNERS: &[(&str, &str)] = &[
    // --- help ---------------------------------------------------------------------------------
    ("about_open", "help"),
    ("help_menu_open", "help"),
    // --- project ------------------------------------------------------------------------------
    ("workspace.projects", "project"),
    ("workspace.active", "project"),
    ("selector", "project"),
    ("rename_draft", "project"),
    ("project_switcher_open", "project"),
    ("project_menu_open", "project"),
    ("forget_target", "project"),
    // --- session ------------------------------------------------------------------------------
    ("workspace.sessions", "session"),
    ("workspace.foreground_by_project", "session"),
    ("active_session", "session"),
    ("reveal_suppressed_for", "session"),
    ("last_foreground_choice", "session"),
    ("restarted_while_inactive", "session"),
    ("session_menu_open", "session"),
    ("session_remove_target", "session"),
    // The terminal is the session's pane: both fields are written only by `focus_terminal` /
    // `release_terminal`, and `tests/terminal_bar_stability.rs` already holds that line.
    ("terminal_released", "session"),
    ("terminal_context_menu", "session"),
    // --- worktree -----------------------------------------------------------------------------
    ("workspace.worktree_names", "worktree"),
    ("workspace.included_worktrees", "worktree"),
    ("worktrees", "worktree"),
    ("worktree_error", "worktree"),
    ("worktree_menu_open", "worktree"),
    ("worktree_delete_target", "worktree"),
    ("worktree_delete_keep_branch", "worktree"),
    ("worktree_rename_draft", "worktree"),
    // Hover is on a *worktree* row and drives that row's actions (add-session, delete). It is
    // named here rather than under `sidebar` because what it identifies is a worktree; the sidebar
    // is where it happens to be drawn.
    ("hovered_worktree", "worktree"),
    // --- worktree_form ------------------------------------------------------------------------
    ("worktree_form", "worktree_form"),
    // --- sidebar ------------------------------------------------------------------------------
    ("expanded", "sidebar"),
    ("default_expanded", "sidebar"),
    ("sidebar_viewport_height", "sidebar"),
    ("pending_reveal_scroll", "sidebar"),
    ("sidebar_hidden", "sidebar"),
    ("sidebar_width", "sidebar"),
    ("sidebar_scroll_offset", "sidebar"),
    ("sidebar_filters", "sidebar"),
    ("sidebar_filter_open", "sidebar"),
    // Which worktrees the sidebar lists, not a fact about any worktree — its own doc calls it view
    // state and contrasts it with `sidebar_filters` beside it.
    ("show_agent_worktrees", "sidebar"),
    // --- settings -----------------------------------------------------------------------------
    ("theme_pref", "settings"),
    ("system_scheme", "settings"),
    ("settings_draft", "settings"),
    // --- notifications ------------------------------------------------------------------------
    ("notify", "notifications"),
    // --- root ---------------------------------------------------------------------------------
    // "one fact about the application, not four" — its own doc comment, explaining why the focused
    // field is not held per dialog. No feature owns it.
    ("focused_field", "root"),
    ("cursor", "root"),
    ("window_size", "root"),
];

/// Cross-feature writes that exist today, each with the feature that performs it and the path it
/// reaches into.
///
/// **Adding an entry requires a task reference in the note.** Removing one is what T067a does.
///
/// The first eight are what the guard found on the tree T059 was written against — Tier 1 code,
/// already inside `src/features/`. T062 added the rest by moving the reducer arms in, which is the
/// only reason the guard can see them; see the banner in the middle of the list. Every entry is
/// pre-existing behaviour and none is converted here: T059 is the guard, T067 catalogues these
/// into `cross-feature-writes.md` with a proposed outcome for each, and T067a converts them one
/// commit at a time.
const ALLOWED: &[(&str, &str, &str)] = &[
    // The reveal: displaying a session expands the row that holds it. Three sidebar fields written
    // from one session operation, and the likeliest single `Outcome` in the list — the session
    // feature's consequence is "this session became current", and expanding to show it is the
    // sidebar's response to that.
    (
        "session",
        "default_expanded",
        "features/session.rs::set_current_session",
    ),
    (
        "session",
        "expanded",
        "features/session.rs::set_current_session",
    ),
    (
        "session",
        "pending_reveal_scroll",
        "features/session.rs::set_current_session",
    ),
    // Via `State::focus_terminal`, which is root code: it clears the focused field and marks the
    // terminal held. **Whether this is a violation at all is a genuine question for T067** — the
    // path is `root`-owned rather than another feature's, and the alternative reading is that
    // `focus_terminal` is a session operation sitting in the wrong file. Recorded as found rather
    // than resolved by the guard that found it.
    (
        "session",
        "focused_field",
        "features/session.rs::restore_after_activation",
    ),
    // Via `State::push_notification`. The contract already names the outcome for this one:
    // `NotificationRaised`, listed under "emitted by: any feature".
    ("session", "notify", "features/session.rs::arm_notice"),
    // Feature 014 (FR-010e): arriving in a project must not carry the previous one's reveal of
    // agent worktrees. A sidebar fact, reset from the session's activation path.
    (
        "session",
        "show_agent_worktrees",
        "features/session.rs::restore_after_activation",
    ),
    // Via `Workspace::activate`. Switching the active project *is* a project operation; the
    // session feature calls it because the switch is what its own step 1 and step 3 bracket.
    (
        "session",
        "workspace.active",
        "features/session.rs::switch_active",
    ),
    // The mirror of the reveal above, in the other direction: collapsing a row cancels the
    // suppression that a session close armed. `SessionsClosed`'s neighbourhood.
    (
        "sidebar",
        "reveal_suppressed_for",
        "features/sidebar.rs::toggle_location",
    ),
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

    // --- via `State::clear_for_dialog`, a root helper (T067) -------------------------------------
    // Opening a dialog clears the focus slot, because the widget tree that reported focus is being
    // torn down and will never report losing it (feature 006 BUG-003). `focused_field` is
    // `root`-owned, so **whether these are violations at all is the open question**, the same one
    // `restore_after_activation` above raises: either the root grows an outcome for "a dialog
    // opened", or `focused_field` stops being root-owned. Recorded as found, not resolved here.
    ("help", "focused_field", "features/help.rs::about_opened"),
    (
        "project",
        "focused_field",
        "features/project.rs::forget_requested",
    ),
    (
        "project",
        "focused_field",
        "features/project.rs::rename_started",
    ),
    (
        "session",
        "focused_field",
        "features/session.rs::remove_requested",
    ),
    ("settings", "focused_field", "features/settings.rs::opened"),
    (
        "worktree",
        "focused_field",
        "features/worktree.rs::delete_requested",
    ),
    (
        "worktree",
        "focused_field",
        "features/worktree.rs::rename_started",
    ),
    (
        "worktree_form",
        "focused_field",
        "features/worktree_form.rs::opened",
    ),
    // --- via `State::focus_terminal`, a root helper (T067) ---------------------------------------
    // Putting a terminal in front of the user gives it the keyboard, which clears whatever field
    // held it (FR-011). Same `root`-owned slot as the group above and the same open question; the
    // trigger differs, which is why they are listed apart rather than merged.
    (
        "session",
        "focused_field",
        "features/session.rs::mode_toggled",
    ),
    ("session", "focused_field", "features/session.rs::selected"),
    (
        "session",
        "focused_field",
        "features/session.rs::shell_instance_close_requested",
    ),
    (
        "session",
        "focused_field",
        "features/session.rs::shell_instance_selected",
    ),
    ("session", "focused_field", "features/session.rs::started"),
    // --- the popover mutual-exclusion rule (features 009 and 015; T067) --------------------------
    // At most one lightweight popover is open, and the project context menu is exclusive with all
    // of them. It is **one rule about the toolbar** that no single feature owns, so each toggle
    // writes its neighbours' openness. The likeliest shape for T067 is one outcome — "a popover
    // opened" — applied by the root to every other registered popover, which would delete this
    // whole block at once. `overlay::registry::close_popovers` already exists for the dialog path;
    // these arms predate it and still assign by hand.
    (
        "help",
        "project_menu_open",
        "features/help.rs::menu_toggled",
    ),
    (
        "help",
        "project_switcher_open",
        "features/help.rs::menu_toggled",
    ),
    (
        "help",
        "sidebar_filter_open",
        "features/help.rs::menu_toggled",
    ),
    (
        "project",
        "help_menu_open",
        "features/project.rs::menu_toggled",
    ),
    (
        "project",
        "sidebar_filter_open",
        "features/project.rs::menu_toggled",
    ),
    (
        "project",
        "worktree_menu_open",
        "features/project.rs::menu_toggled",
    ),
    (
        "project",
        "help_menu_open",
        "features/project.rs::switcher_toggled",
    ),
    (
        "project",
        "sidebar_filter_open",
        "features/project.rs::switcher_toggled",
    ),
    (
        "sidebar",
        "help_menu_open",
        "features/sidebar.rs::filter_menu_toggled",
    ),
    (
        "sidebar",
        "project_menu_open",
        "features/sidebar.rs::filter_menu_toggled",
    ),
    (
        "sidebar",
        "project_switcher_open",
        "features/sidebar.rs::filter_menu_toggled",
    ),
    (
        "worktree",
        "project_menu_open",
        "features/worktree.rs::menu_toggled",
    ),
    // --- via `Workspace::forget`, which is `micold-core` code (T067) -----------------------------
    // Forgetting a project drops everything held against its path, and three features hold
    // something: its sessions and foreground choice, its worktree names and inclusions. The write
    // is one call in core; the four members it reaches are what make it four entries here.
    // `ProjectForgotten` is the obvious outcome, and it is the clearest case in the whole list.
    (
        "project",
        "workspace.foreground_by_project",
        "features/project.rs::forget_confirmed",
    ),
    (
        "project",
        "workspace.included_worktrees",
        "features/project.rs::forget_confirmed",
    ),
    (
        "project",
        "workspace.sessions",
        "features/project.rs::forget_confirmed",
    ),
    (
        "project",
        "workspace.worktree_names",
        "features/project.rs::forget_confirmed",
    ),
    // --- via `State::push_notification`, a root helper (T067) ------------------------------------
    // The contract already names this one: `NotificationRaised`, listed under "emitted by: any
    // feature". Same as `session::arm_notice` above, reached from the project side.
    ("project", "notify", "features/project.rs::open_refused"),
    // --- via `State::set_worktrees`, a root helper (T067) ----------------------------------------
    // Discovery answering with a new worktree list prunes the sidebar's expansion of rows that no
    // longer exist (feature 008). A genuine consequence rather than a helper accident: the sidebar
    // has to respond to worktrees disappearing, and `WorktreesReplaced` is the outcome shape.
    ("worktree", "expanded", "features/worktree.rs::loaded"),
    // --- the form creates; the worktree feature owns the list (T067) ------------------------------
    // `worktree_form` is a separate feature precisely because its lifecycle is independent
    // (FR-003), but the thing it creates lands in `worktree`'s list and its failures land in
    // `worktree`'s error slot. `WorktreeCreated` / `WorktreeCreateFailed` are the outcomes, and
    // T064 — which promotes the form to a nested unit with its own message type — is where the
    // seam is most visible.
    (
        "worktree_form",
        "worktree_error",
        "features/worktree_form.rs::create_failed",
    ),
    (
        "worktree_form",
        "worktree_error",
        "features/worktree_form.rs::created",
    ),
    (
        "worktree_form",
        "worktree_error",
        "features/worktree_form.rs::opened",
    ),
    (
        "worktree_form",
        "worktrees",
        "features/worktree_form.rs::created",
    ),
];

/// Methods that mutate the receiver, for state paths whose type this file does not decompose.
///
/// `Workspace` is not in here: its members belong to three different features, so a call on it is
/// resolved to the members it actually writes rather than treated as one opaque mutation.
const MUTATORS: &[&str] = &[
    "advance",
    "append",
    "clear",
    "dismiss",
    "drain",
    "entry",
    "extend",
    "get_mut",
    "get_or_insert",
    "get_or_insert_with",
    "insert",
    "iter_mut",
    "last_mut",
    "pop",
    "push",
    "push_str",
    "remove",
    "replace",
    "retain",
    "sort",
    "sort_by",
    "sort_by_key",
    "take",
    "truncate",
    "values_mut",
];

/// Methods that only read the receiver.
const READERS: &[&str] = &[
    "all",
    "and_then",
    "any",
    "as_deref",
    "as_ref",
    "as_str",
    "clone",
    "cloned",
    "contains",
    "contains_key",
    "copied",
    "count",
    "filter",
    "find",
    "first",
    "get",
    "is_empty",
    "is_none",
    "is_some",
    "iter",
    "keys",
    "last",
    "len",
    "map",
    // `ThemePreference::next` takes `self` by value and returns the next one — the assignment that
    // stores it is what this scan flags, at its own site.
    "next",
    "position",
    "to_string",
    "to_vec",
    "unwrap_or",
    "unwrap_or_default",
    "values",
];

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn workspace_rs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../micold-core/src/workspace.rs")
}

/// Every `.rs` file under `src/`, as `(path relative to src/, source with comments stripped)`.
fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .strip_prefix(src_dir())
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                let src = fs::read_to_string(&path).expect("read source");
                out.push((name, code_only(&src)));
            }
        }
    }
    let mut out = Vec::new();
    walk(&src_dir(), &mut out);
    out.sort();
    out
}

/// Strips comments and string literals, so the doc comments explaining this rule — and any test
/// fixture quoting a field name — cannot trip it.
fn code_only(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_block = false;
    let mut in_line = false;
    let mut in_str = false;
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

/// Length of the braced block whose opening brace has already been consumed.
fn block_len(src: &str) -> usize {
    delimited_len(src, '{', '}')
}

/// Length of the parenthesised list whose opening paren has already been consumed.
fn paren_len(src: &str) -> usize {
    delimited_len(src, '(', ')')
}

fn delimited_len(src: &str, open: char, close: char) -> usize {
    let mut depth = 1usize;
    for (i, c) in src.char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
    }
    panic!("unbalanced `{open}{close}`");
}

/// The field names of `pub struct <name>`, in declaration order.
fn struct_fields(src: &str, name: &str) -> Vec<String> {
    let needle = format!("pub struct {name} {{");
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("`{needle}` not found — has the struct been renamed?"))
        + needle.len();
    let body = &src[start..start + block_len(&src[start..])];
    let mut out = Vec::new();
    for line in body.lines() {
        let t = line.trim();
        let Some((head, _)) = t.split_once(':') else {
            continue;
        };
        let head = head
            .trim_start_matches("pub(crate) ")
            .trim_start_matches("pub ")
            .trim();
        if !head.is_empty()
            && head
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            out.push(head.to_string());
        }
    }
    out
}

/// One method or free function that can mutate the struct under scrutiny.
struct Operation {
    /// `features/session.rs`
    file: String,
    /// `restore_after_activation`
    name: String,
    /// What the value is bound to inside the body — `self`, or the parameter's name.
    binding: String,
    /// Whether it can mutate at all. Read-only methods are collected so a call to one classifies
    /// as a read rather than as an unclassified method.
    mutating: bool,
    body: String,
}

impl Operation {
    /// `features/session.rs::restore_after_activation` — the identity of one operation.
    ///
    /// The file has to be part of it: T062 gives five feature modules a `menu_toggled` each, and a
    /// bare name merges them. See [`transitive_writes`].
    fn key(&self) -> String {
        format!("{}::{}", self.file, self.name)
    }
}

/// Byte ranges of each `impl … <name> { … }` block body, inherent or trait.
///
/// **The type is matched by its last path segment, and a probe is why.** An earlier version looked
/// for the literal `impl State {`, so a module writing `impl crate::app::State { … }` — which
/// compiles identically and is what a fresh feature module is most likely to write, having no
/// `use` for it yet — was invisible to the whole scan. Two live-fire probes planted a cross-feature
/// write that way and fired nothing at all.
///
/// Trait impls count too: `impl SomeTrait for State` can carry `&mut self` methods just as an
/// inherent block can. `impl Default for State` is swept in by the same rule and contributes
/// nothing, its `default()` taking no receiver.
fn impl_blocks(src: &str, name: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(at) = src[from..].find("\nimpl ") {
        let head_start = from + at + "\nimpl ".len();
        let Some(brace) = src[head_start..].find('{') else {
            break;
        };
        let body_start = head_start + brace + 1;
        from = body_start;
        let header = src[head_start..head_start + brace].trim();
        // `impl Trait for Type` — the target is what follows `for`.
        let target = header.rsplit(" for ").next().unwrap_or(header).trim();
        let target = target.split_whitespace().next().unwrap_or(target);
        let target = target.split('<').next().unwrap_or(target);
        if target.rsplit("::").next() == Some(name) {
            let len = block_len(&src[body_start..]);
            out.push((body_start, body_start + len));
            from = body_start + len;
        }
    }
    out
}

/// Every operation on `struct_name` in one source file: its inherent methods, plus free functions
/// taking `&mut <struct_name>`.
fn operations_in(file: &str, src: &str, struct_name: &str) -> Vec<Operation> {
    let blocks = impl_blocks(src, struct_name);
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(at) = src[from..].find("fn ") {
        let start = from + at;
        from = start + 3;
        let Some(paren) = src[start..].find('(') else {
            break;
        };
        let name = src[start + 3..start + paren].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let args_start = start + paren + 1;
        let args_len = paren_len(&src[args_start..]);
        let args = &src[args_start..args_start + args_len];
        let trimmed = args.trim_start();
        let in_impl = blocks.iter().any(|(s, e)| start >= *s && start < *e);
        let (binding, mutating) = if in_impl && trimmed.starts_with("&mut self") {
            ("self".to_string(), true)
        } else if in_impl && trimmed.starts_with("&self") {
            ("self".to_string(), false)
        } else if let Some(param) = mut_param(args, struct_name) {
            (param, true)
        } else {
            continue;
        };
        let Some(brace) = src[args_start + args_len..].find('{') else {
            continue;
        };
        let body_start = args_start + args_len + brace + 1;
        let body_len = block_len(&src[body_start..]);
        out.push(Operation {
            file: file.to_string(),
            name: name.to_string(),
            binding,
            mutating,
            body: src[body_start..body_start + body_len].to_string(),
        });
        from = body_start + body_len;
    }
    out
}

/// The name of the first parameter declared `&mut <struct_name>`.
///
/// The reference has to be *peeled*, not merely detected. Whitespace is stripped first, so the
/// type reads `&mutState` — and asking whether that ends in `State` after splitting on `::` is
/// asking whether `&mutState` equals `State`, which it never does. That was the shape of this
/// function until T062, and it is why the guard reported every feature clean the moment Tier 3
/// turned `impl State` methods into free functions: it could not see a single one of them, while
/// this file's own header promised it could. An optional lifetime is peeled too, since `&'a mut
/// State` is the same parameter written differently.
fn mut_param(args: &str, struct_name: &str) -> Option<String> {
    for arg in args.split(',') {
        let Some((name, ty)) = arg.split_once(':') else {
            continue;
        };
        let ty = ty.replace([' ', '\n'], "");
        let Some(rest) = ty.strip_prefix('&') else {
            continue;
        };
        let rest = match rest.strip_prefix('\'') {
            Some(after) => after.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_'),
            None => rest,
        };
        let Some(ty) = rest.strip_prefix("mut") else {
            continue;
        };
        if ty.split("::").last() == Some(struct_name) {
            return Some(name.trim().to_string());
        }
    }
    None
}

/// What one operation writes directly, and which sibling operations it calls.
#[derive(Default)]
struct Reach {
    writes: BTreeSet<String>,
    calls: BTreeSet<String>,
}

/// How a `path.method(` call is resolved.
enum Call<'a> {
    /// Writes exactly these paths.
    Writes(&'a BTreeSet<String>),
    /// Reads only.
    Reads,
}

/// Scan one body for writes to `<binding>.<field>` and for calls to sibling operations.
///
/// `nested` names a field whose own members are tracked separately — `workspace` for `State` — so
/// `state.workspace.sessions` resolves to one path rather than to the whole struct. `nested_api`
/// resolves method calls on that field: `state.workspace.forget(p)` writes five members across
/// three features, which no single mutator/reader verdict could express.
fn reach(
    op: &Operation,
    fields: &BTreeSet<String>,
    nested: Option<(&str, &BTreeSet<String>)>,
    nested_api: &BTreeMap<String, Call<'_>>,
    siblings: &BTreeSet<String>,
    unclassified: &mut BTreeSet<String>,
) -> Reach {
    let mut r = Reach::default();
    let anchor = format!("{}.", op.binding);
    let bytes = op.body.as_bytes();
    let mut i = 0usize;
    while let Some(at) = op.body[i..].find(&anchor) {
        let start = i + at;
        i = start + anchor.len();
        if start > 0 {
            let prev = bytes[start - 1] as char;
            if prev.is_alphanumeric() || prev == '_' {
                continue; // the tail of a longer identifier
            }
        }
        let (ident, after) = read_ident(&op.body, i);
        if ident.is_empty() {
            continue;
        }
        if siblings.contains(&ident) && op.body[after..].trim_start().starts_with('(') {
            r.calls.insert(ident);
            continue;
        }
        if !fields.contains(&ident) {
            continue;
        }
        let mut path = ident;
        let mut after = after;
        if let Some((nested_name, nested_fields)) = nested {
            if path == nested_name && op.body[after..].starts_with('.') {
                let (sub, next) = read_ident(&op.body, after + 1);
                if nested_fields.contains(&sub) {
                    path = format!("{nested_name}.{sub}");
                    after = next;
                } else if !sub.is_empty() && op.body[next..].trim_start().starts_with('(') {
                    match nested_api.get(&sub) {
                        Some(Call::Writes(paths)) => r.writes.extend(paths.iter().cloned()),
                        Some(Call::Reads) => {}
                        None => {
                            unclassified.insert(format!("{nested_name}.{sub}"));
                        }
                    }
                    i = next;
                    continue;
                }
            }
        }
        let preceded_by_mut = op.body[..start].trim_end().ends_with("&mut");
        let tail = op.body[after..].trim_start();
        let is_write = if preceded_by_mut {
            true
        } else if let Some(rest) = tail.strip_prefix('=') {
            !rest.starts_with('=')
        } else if ["+=", "-=", "*=", "|=", "&="]
            .iter()
            .any(|o| tail.starts_with(o))
        {
            true
        } else if let Some(rest) = tail.strip_prefix('.') {
            let (method, next) = read_ident(rest, 0);
            if method.is_empty() || !rest[next..].trim_start().starts_with('(') {
                false // a tuple or struct member access, not a call
            } else if MUTATORS.contains(&method.as_str()) {
                true
            } else if READERS.contains(&method.as_str()) {
                false
            } else {
                unclassified.insert(format!("{path}.{method}"));
                false
            }
        } else {
            false
        };
        if is_write {
            r.writes.insert(path);
        }
        i = after;
    }
    r
}

/// The identifier starting at `src[from]`, and the index just past it.
fn read_ident(src: &str, from: usize) -> (String, usize) {
    let mut end = from;
    for (i, c) in src[from..].char_indices() {
        if c.is_alphanumeric() || c == '_' {
            end = from + i + c.len_utf8();
        } else {
            break;
        }
    }
    (src[from..end].to_string(), end)
}

/// Resolve every operation to the full set of paths it writes, following calls to a fixed point.
///
/// # Keyed by `file::name`, because names stopped being unique at T062
///
/// Under Tier 1 every operation was an `impl State` method, so a bare name identified one function
/// and this map was keyed by it. Tier 3 gives each feature module a free function per reducer arm,
/// and five of them are called `menu_toggled` — one each in `help`, `project`, `session`,
/// `worktree` and `worktree_form` — with `opened`, `cancelled`, `rename_started` and others
/// repeating too.
///
/// Keyed by bare name those five became **one** entry holding the union of all five bodies'
/// writes, and the guard then reported each of them writing the other four's fields. It is a
/// failure in the direction that looks like diligence — a wall of violations, every one of them
/// false — and the symmetry gave it away: `settings::opened` was accused of writing
/// `worktree_form`, and `worktree_form::opened` of writing `settings_draft`.
///
/// Calls are still *written* by bare name (`state.set_current_session(…)` says nothing about which
/// file), so a callee resolves to every operation sharing that name. No colliding name is ever
/// called that way — the reducer free functions are called only from the root, which is not a
/// feature operation — but unioning is the conservative direction if one ever is: it over-reports
/// rather than going quiet.
fn transitive_writes(
    ops: &[Operation],
    fields: &BTreeSet<String>,
    nested: Option<(&str, &BTreeSet<String>)>,
    nested_api: &BTreeMap<String, Call<'_>>,
    unclassified: &mut BTreeSet<String>,
) -> (BTreeMap<String, BTreeSet<String>>, BTreeMap<String, Reach>) {
    let siblings: BTreeSet<String> = ops.iter().map(|o| o.name.clone()).collect();
    let mut by_name: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut direct: BTreeMap<String, Reach> = BTreeMap::new();
    for op in ops {
        let r = reach(op, fields, nested, nested_api, &siblings, unclassified);
        by_name.entry(op.name.clone()).or_default().insert(op.key());
        let entry = direct.entry(op.key()).or_default();
        entry.writes.extend(r.writes);
        entry.calls.extend(r.calls);
    }
    let mut writes: BTreeMap<String, BTreeSet<String>> = direct
        .iter()
        .map(|(k, v)| (k.clone(), v.writes.clone()))
        .collect();
    // Bounded by the call graph, so a cycle terminates rather than spinning.
    loop {
        let mut changed = false;
        let snapshot = writes.clone();
        for (key, r) in &direct {
            for callee in &r.calls {
                for callee_key in by_name.get(callee).into_iter().flatten() {
                    let Some(inherited) = snapshot.get(callee_key) else {
                        continue;
                    };
                    let target = writes.entry(key.clone()).or_default();
                    for path in inherited {
                        changed |= target.insert(path.clone());
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    (writes, direct)
}

/// The feature a source file belongs to, or `None` for shell, view and root code.
fn feature_of(file: &str) -> Option<String> {
    let stem = file.strip_prefix("features/")?.strip_suffix(".rs")?;
    (stem != "mod").then(|| stem.to_string())
}

fn owners() -> BTreeMap<String, String> {
    OWNERS
        .iter()
        .map(|(p, f)| (p.to_string(), f.to_string()))
        .collect()
}

/// The whole analysis, run once per test that needs it.
struct Scan {
    /// Cross-feature writes, as `(feature, path, "file::operation")`.
    violations: Vec<(String, String, String)>,
    /// Methods called on a state path that are in neither table.
    unclassified: BTreeSet<String>,
    /// Mutating operations found under `src/features/`.
    feature_ops: usize,
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
    let (ws_writes, _) = transitive_writes(
        &ws_mutating,
        &workspace_fields,
        None,
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
    let mut nested_api: BTreeMap<String, Call<'_>> = BTreeMap::new();
    for op in &ws_ops {
        if !op.mutating {
            nested_api.insert(op.name.clone(), Call::Reads);
        }
    }
    for (name, paths) in &ws_writes {
        nested_api.insert(name.clone(), Call::Writes(paths));
    }

    let app = sources
        .iter()
        .find(|(f, _)| f == "app.rs")
        .map(|(_, s)| s.clone())
        .expect("src/app.rs");
    let state_field_list = struct_fields(&app, "State");
    let fields: BTreeSet<String> = state_field_list.iter().cloned().collect();

    let ops: Vec<Operation> = sources
        .iter()
        .flat_map(|(file, src)| operations_in(file, src, "State"))
        .filter(|o| o.mutating)
        .collect();
    let (writes, direct) = transitive_writes(
        &ops,
        &fields,
        Some(("workspace", &workspace_fields)),
        &nested_api,
        &mut unclassified,
    );

    // Report each write once, at the innermost feature operation that reaches it. Without this a
    // single `self.expanded.insert(..)` is reported three times over — at the write, and again at
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
    for op in &ops {
        let Some(feature) = feature_of(&op.file) else {
            continue;
        };
        let Some(paths) = writes.get(&op.key()) else {
            continue;
        };
        for path in paths {
            let owner = owners
                .get(path)
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
            if inherited_from_a_feature {
                continue;
            }
            violations.push((feature.clone(), path.clone(), op.key()));
        }
    }
    violations.sort();
    violations.dedup();
    Scan {
        violations,
        unclassified,
        feature_ops: ops.iter().filter(|o| feature_of(&o.file).is_some()).count(),
        state_fields: state_field_list.len(),
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
    let known: BTreeSet<&str> = fields.iter().map(String::as_str).collect();
    let stale: Vec<&str> = OWNERS
        .iter()
        .map(|(p, _)| *p)
        .filter(|p| !p.starts_with("workspace.") && !known.contains(p))
        .collect();
    assert!(
        stale.is_empty(),
        "OWNERS names paths that are not `State` fields any more: {stale:?}"
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
        "`State` parsed to only {} fields — the struct scan is not reading what it thinks it is",
        scan.state_fields
    );
}
