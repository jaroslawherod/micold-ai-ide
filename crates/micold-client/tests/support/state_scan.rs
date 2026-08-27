//! The source-text scan two guards share: what every operation on `app::State` writes.
//!
//! # Why it is here rather than in one of them
//!
//! `feature_write_isolation.rs` (feature 021, T059) asks *who writes whose data*; feature 028's G2
//! in `root_state_is_shared.rs` asks *which single feature writes this loose field*. Both answers
//! come from the same walk: find every function that can mutate `State`, resolve what its own
//! lines write, then follow its calls to a fixed point so a caller inherits its callees' writes.
//!
//! [contracts/guards.md](../../../../specs/028-feature-encapsulation/contracts/guards.md) says G2
//! resolves a field's writer "through the same transitive `&mut State` scan
//! `feature_write_isolation.rs` already performs". A copy would satisfy that sentence for one
//! commit and then drift — and the drift would be silent in the direction that matters, since a
//! scan that has stopped seeing a shape reports *nothing*, which reads exactly like a pass. That
//! failure has happened twice in this file's history already (`mut_param`'s unpeeled reference,
//! and `impl_blocks` matching only the literal `impl State {`), both recorded below where they
//! were fixed. So the walk lives once, and both guards call it.
//!
//! # What it deliberately does not decide
//!
//! Nothing here knows about ownership, allowlists or exemptions. It reports paths written and
//! calls made; each guard applies its own rule to that. The two tables it does carry — [`MUTATORS`]
//! and [`READERS`] — are not policy but vocabulary: a method call on a state path has to resolve
//! to a write or a read before either guard can say anything, and an unrecognised one is returned
//! as unclassified rather than guessed at.
//!
//! Included with `#[path = "support/state_scan.rs"] mod state_scan;` rather than through
//! `support/mod.rs`, which pulls in the renderer-backed layout fixtures these two guards have no
//! use for.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Methods that mutate the receiver, for state paths whose type this file does not decompose.
///
/// `Workspace` is not in here: its members belong to three different features, so a call on it is
/// resolved to the members it actually writes rather than treated as one opaque mutation.
pub const MUTATORS: &[&str] = &[
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
pub const READERS: &[&str] = &[
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
    "is_some_and",
    "is_some",
    "iter",
    "keys",
    // `session.known_available` takes `&self` and hands back a slice of the availability set it
    // already holds — a read of two fields, not a write to either.
    "known_available",
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

pub fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

pub fn workspace_rs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../micold-core/src/workspace.rs")
}

/// Every `.rs` file under `src/`, as `(path relative to src/, source with comments stripped)`.
pub fn sources() -> Vec<(String, String)> {
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
pub fn code_only(src: &str) -> String {
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
pub fn block_len(src: &str) -> usize {
    delimited_len(src, '{', '}')
}

/// Length of the parenthesised list whose opening paren has already been consumed.
pub fn paren_len(src: &str) -> usize {
    delimited_len(src, '(', ')')
}

pub fn delimited_len(src: &str, open: char, close: char) -> usize {
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
pub fn struct_fields(src: &str, name: &str) -> Vec<String> {
    struct_field_types(src, name)
        .into_iter()
        .map(|(field, _)| field)
        .collect()
}

/// The fields of `pub struct <name>` as `(field, type)`, in declaration order.
///
/// The type is the text between the colon and the trailing comma, whitespace-collapsed — enough
/// for feature 028's G2 to ask whether a root field's type resolves to a feature's `State`, and
/// not an attempt at parsing Rust. A field whose type spans lines is not read; `app::State` has
/// none, and one appearing would show up as an unrecognised type rather than as a silent pass.
pub fn struct_field_types(src: &str, name: &str) -> Vec<(String, String)> {
    let needle = format!("pub struct {name} {{");
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("`{needle}` not found — has the struct been renamed?"))
        + needle.len();
    let body = &src[start..start + block_len(&src[start..])];
    let mut out = Vec::new();
    for line in body.lines() {
        let t = line.trim();
        let Some((head, ty)) = t.split_once(':') else {
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
            out.push((
                head.to_string(),
                ty.trim().trim_end_matches(',').to_string(),
            ));
        }
    }
    out
}

/// One method or free function that can mutate the struct under scrutiny.
pub struct Operation {
    /// `features/session.rs`
    pub file: String,
    /// `restore_after_activation`
    pub name: String,
    /// What the value is bound to inside the body — `self`, or the parameter's name.
    pub binding: String,
    /// Whether it can mutate at all. Read-only methods are collected so a call to one classifies
    /// as a read rather than as an unclassified method.
    pub mutating: bool,
    pub body: String,
}

impl Operation {
    /// `features/session.rs::restore_after_activation` — the identity of one operation.
    ///
    /// The file has to be part of it: T062 gives five feature modules a `menu_toggled` each, and a
    /// bare name merges them. See [`transitive_writes`].
    pub fn key(&self) -> String {
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
pub fn impl_blocks(src: &str, name: &str) -> Vec<(usize, usize)> {
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
pub fn operations_in(file: &str, src: &str, struct_name: &str) -> Vec<Operation> {
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
pub fn mut_param(args: &str, struct_name: &str) -> Option<String> {
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
///
/// `writes` is what the operation's own lines write. `core_writes` is what it writes by calling a
/// method of the nested core type — `state.workspace.forget(p)` — mapped to the method names that
/// carried it, because that is the fact the message needs to state.
#[derive(Default)]
pub struct Reach {
    pub writes: BTreeSet<String>,
    pub core_writes: BTreeMap<String, BTreeSet<String>>,
    pub calls: BTreeSet<String>,
}

/// How a `path.method(` call is resolved.
pub enum Call<'a> {
    /// Writes exactly these paths.
    Writes(&'a BTreeSet<String>),
    /// Reads only.
    Reads,
}

/// Scan one body for writes to `<binding>.<field>` and for calls to sibling operations.
///
/// # The access is walked, not glanced at
///
/// `nested` maps a field to the members of its own type, so `state.workspace.sessions` and
/// `state.sidebar.expanded` each resolve to one path rather than to the whole struct. The walk
/// continues through members of types this scan does *not* decompose — `state.settings.draft.
/// scrollback = x` keeps the path at `settings.draft` and still reports the write — because
/// stopping at the first unrecognised segment is how the write disappears.
///
/// **That is not hypothetical.** Until feature 028 T042 this function knew about exactly one
/// nested field, `workspace`, and treated any other `a.b.c` as a member access rather than as a
/// write. Feature 028 then moved all 43 root fields behind nine feature structs, which turned
/// every reducer's `state.sidebar.expanded.insert(..)` into exactly that shape — and the guard
/// went green because it had stopped seeing writes at all, not because there were none. Third
/// occurrence of this file's characteristic failure, and the reason both guards now assert a floor
/// on the number of writes found rather than only on the number of operations read.
///
/// `nested_api` resolves method calls on a nested field: `state.workspace.forget(p)` writes five
/// members across three features, which no single mutator/reader verdict could express. It is
/// keyed by the field the methods belong to, then by method name.
pub fn reach(
    op: &Operation,
    fields: &BTreeSet<String>,
    nested: &BTreeMap<String, BTreeSet<String>>,
    nested_api: &BTreeMap<String, BTreeMap<String, Call<'_>>>,
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
        // Walk the access as far as it goes: `.member` extends the path when the member belongs
        // to a type this scan decomposes, and is stepped over when it does not, so the verdict is
        // taken at the end of the chain rather than at its first unfamiliar segment.
        let mut path = ident;
        let mut after = after;
        let mut call: Option<String> = None;
        while op.body[after..].starts_with('.') {
            let (sub, next) = read_ident(&op.body, after + 1);
            if sub.is_empty() {
                break;
            }
            if op.body[next..].trim_start().starts_with('(') {
                call = Some(sub);
                after = next;
                break;
            }
            if nested.get(&path).is_some_and(|m| m.contains(&sub)) {
                path = format!("{path}.{sub}");
            }
            after = next;
        }
        let preceded_by_mut = op.body[..start].trim_end().ends_with("&mut");
        let is_write = if let Some(method) = call {
            // A method of the nested type's own API, which may write several paths at once.
            if let Some(api) = nested_api.get(&path) {
                match api.get(&method) {
                    Some(Call::Writes(paths)) => {
                        for written in paths.iter() {
                            r.core_writes
                                .entry(written.clone())
                                .or_default()
                                .insert(method.clone());
                        }
                    }
                    Some(Call::Reads) => {}
                    None => {
                        unclassified.insert(format!("{path}.{method}"));
                    }
                }
                i = after;
                continue;
            }
            if MUTATORS.contains(&method.as_str()) {
                true
            } else if READERS.contains(&method.as_str()) {
                false
            } else {
                unclassified.insert(format!("{path}.{method}"));
                false
            }
        } else if preceded_by_mut {
            true
        } else {
            let tail = op.body[after..].trim_start();
            if let Some(rest) = tail.strip_prefix('=') {
                !rest.starts_with('=')
            } else {
                ["+=", "-=", "*=", "|=", "&="]
                    .iter()
                    .any(|o| tail.starts_with(o))
            }
        };
        if is_write {
            r.writes.insert(path);
        }
        i = after;
    }
    r
}

/// The identifier starting at `src[from]`, and the index just past it.
pub fn read_ident(src: &str, from: usize) -> (String, usize) {
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

/// What each operation writes, keyed by `file::name`.
pub type Writes = BTreeMap<String, BTreeSet<String>>;

/// What each operation writes *through a core method*, and which methods carried it — the same
/// keying, with the path mapped to the `Workspace` methods responsible.
pub type CoreWrites = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;

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
pub fn transitive_writes(
    ops: &[Operation],
    fields: &BTreeSet<String>,
    nested: &BTreeMap<String, BTreeSet<String>>,
    nested_api: &BTreeMap<String, BTreeMap<String, Call<'_>>>,
    unclassified: &mut BTreeSet<String>,
) -> (Writes, CoreWrites, BTreeMap<String, Reach>) {
    let siblings: BTreeSet<String> = ops.iter().map(|o| o.name.clone()).collect();
    let mut by_name: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut direct: BTreeMap<String, Reach> = BTreeMap::new();
    for op in ops {
        let r = reach(op, fields, nested, nested_api, &siblings, unclassified);
        by_name.entry(op.name.clone()).or_default().insert(op.key());
        let entry = direct.entry(op.key()).or_default();
        entry.writes.extend(r.writes);
        for (path, methods) in r.core_writes {
            entry.core_writes.entry(path).or_default().extend(methods);
        }
        entry.calls.extend(r.calls);
    }
    // Two closures over the same call graph. `core` is what a core method wrote, carried outward
    // so a caller two steps up is judged on the same footing as the operation that made the call;
    // `writes` is everything, which is what every other consumer of this scan means by a write.
    let core = close(
        direct
            .iter()
            .map(|(k, v)| (k.clone(), v.core_writes.clone()))
            .collect(),
        &direct,
        &by_name,
    );
    let seed: BTreeMap<String, BTreeSet<String>> = direct
        .iter()
        .map(|(k, v)| {
            let mut paths = v.writes.clone();
            paths.extend(v.core_writes.keys().cloned());
            (k.clone(), paths)
        })
        .collect();
    let writes = close(seed, &direct, &by_name);
    (writes, core, direct)
}

/// Everything a value in `seed` reaches by following calls, to a fixed point.
///
/// Generic over the value so the same walk serves the set of paths written and the map of paths to
/// the core methods that wrote them: bounded by the call graph, so a cycle terminates rather than
/// spinning.
pub fn close<T: Merge + Clone>(
    seed: BTreeMap<String, T>,
    direct: &BTreeMap<String, Reach>,
    by_name: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeMap<String, T> {
    let mut out = seed;
    loop {
        let mut changed = false;
        let snapshot = out.clone();
        for (key, r) in direct {
            for callee in &r.calls {
                for callee_key in by_name.get(callee).into_iter().flatten() {
                    let Some(inherited) = snapshot.get(callee_key) else {
                        continue;
                    };
                    changed |= out.entry(key.clone()).or_default().merge(inherited);
                }
            }
        }
        if !changed {
            break;
        }
    }
    out
}

/// Absorb another value of the same shape, answering whether anything was new.
pub trait Merge: Default {
    fn merge(&mut self, other: &Self) -> bool;
}

impl Merge for BTreeSet<String> {
    fn merge(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for path in other {
            changed |= self.insert(path.clone());
        }
        changed
    }
}

impl Merge for BTreeMap<String, BTreeSet<String>> {
    fn merge(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for (path, methods) in other {
            changed |= self.entry(path.clone()).or_default().merge(methods);
        }
        changed
    }
}

/// The feature a source file belongs to, or `None` for shell, view and root code.
pub fn feature_of(file: &str) -> Option<String> {
    let stem = file.strip_prefix("features/")?.strip_suffix(".rs")?;
    (stem != "mod").then(|| stem.to_string())
}
