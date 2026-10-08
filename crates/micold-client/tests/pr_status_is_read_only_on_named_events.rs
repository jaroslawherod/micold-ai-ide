//! Pull request status is read only on the named events (feature 040, contracts/reading-and-wire.md §1).
//!
//! The reading runs `gh`, which reaches GitHub. It may start only through the schedule reducer's
//! `Effect::Read`, and the reducer emits that only on the events the contract names: S1 (the
//! listing arrived), S2 (the setting turned on, including on connect). S3 to S5 are added by US4
//! (milestone M7), which raises the pinned counts below. A read started from a render, a hover or
//! a timer of the view would contact GitHub for a user who never asked.
//!
//! So this scans `crates/micold-client/src` as text, in the pattern of
//! `issues_are_requested_only_on_named_events.rs`, and holds four things:
//! 1. the function that turns a start into a task is called from one line, and the shell entry
//!    points that feed the reducer are only where [`ALLOWED`] says;
//! 2. `Msg::ListingArrived` is built in one place, fed by the catalog arm of `on_daemon_event`;
//! 3. the source's `read` is reached from one place;
//! 4. nothing logs a pull request's title or address.

use std::fs;
use std::path::{Path, PathBuf};

const SHELL: &str = "crates/micold-client/src/shell/pr_status.rs";
const FEATURE: &str = "crates/micold-client/src/features/pr_status.rs";
const SYNC: &str = "crates/micold-client/src/shell/daemon_sync.rs";
const MAIN: &str = "crates/micold-client/src/main.rs";
const CAPS: &str = "crates/micold-client/src/shell/capabilities.rs";
const MAIN_TESTS: &str = "crates/micold-client/src/main_tests.rs";

/// Shell entry points into the reading: `(needle, file, expected lines, why)`. A call that is
/// not here is a new way to start a read (or to change what starts one).
const ALLOWED: &[(&str, &str, usize, &str)] = &[
    (
        "shell::pr_status::listing_arrived(",
        SYNC,
        1,
        "S1: the `CatalogChanged` arm of `on_daemon_event`, the one place a listing arrives.",
    ),
    (
        "shell::pr_status::enabled_changed(",
        SYNC,
        2,
        "S2: the `SettingsChanged` arm, and after `adopt_daemon_settings` on connect.",
    ),
    (
        "shell::pr_status::update(",
        SYNC,
        6,
        "`Msg::Held` / `Msg::Released` only: they hold or release a reading and start nothing.",
    ),
    (
        "shell::pr_status::on_remotes(",
        SYNC,
        2,
        "Feeds the remotes the daemon reported to the reading; it starts nothing itself.",
    ),
    (
        "shell::pr_status::update(",
        MAIN,
        1,
        "`main.rs` routes `Message::PrStatus` (the reducer's own messages) to the shell.",
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repository root")
}

fn client_sources(root: &Path) -> Vec<PathBuf> {
    // An unreadable path fails the scan rather than shrinking it.
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries =
            fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|e| panic!("cannot read an entry of {}: {e}", dir.display()))
                .path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("crates/micold-client/src"), &mut out);
    out.sort();
    out
}

fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with('*')
}

/// `(file relative to the root, 1-based line number, trimmed line)` for every non-comment line
/// of the client's sources.
fn code_lines() -> Vec<(String, usize, String)> {
    let root = repo_root();
    let mut out = Vec::new();
    for source in client_sources(&root) {
        let text = fs::read_to_string(&source)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", source.display()));
        let rel = source
            .strip_prefix(&root)
            .unwrap_or(&source)
            .to_string_lossy()
            .replace('\\', "/");
        for (i, line) in text.lines().enumerate() {
            if !is_comment(line) {
                out.push((rel.clone(), i + 1, line.trim().to_string()));
            }
        }
    }
    out
}

fn in_file<'a>(
    lines: &'a [(String, usize, String)],
    file: &'a str,
) -> impl Iterator<Item = &'a (String, usize, String)> {
    lines.iter().filter(move |(f, _, _)| f == file)
}

fn show(found: &[&(String, usize, String)]) -> String {
    found
        .iter()
        .map(|(f, n, l)| format!("  {f}:{n}: {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A call of the free function `start(`: not a definition, and not the tail of another name.
fn calls_start(line: &str) -> bool {
    if line.contains("fn start(") {
        return false;
    }
    line.match_indices("start(").any(|(i, _)| {
        !line[..i]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == ':')
    })
}

#[test]
fn a_read_starts_from_one_line_and_the_shell_entry_points_are_pinned() {
    let lines = code_lines();

    let calls: Vec<_> = in_file(&lines, SHELL)
        .filter(|(_, _, l)| calls_start(l))
        .collect();
    assert!(
        calls.len() == 1 && calls[0].2 == "Effect::Read { seq } => start(app, seq),",
        "`start` in {SHELL} must be called from exactly one line, the `Effect::Read` arm of \
         `update`; found:\n{}",
        show(&calls)
    );

    let elsewhere: Vec<_> = lines
        .iter()
        .filter(|(f, _, l)| f != MAIN_TESTS && l.contains("pr_status::start"))
        .collect();
    assert!(
        elsewhere.is_empty(),
        "`pr_status::start` must not be named anywhere; found:\n{}",
        show(&elsewhere)
    );

    // Every call of a shell entry point, wherever it is, must be accounted for.
    let needles: Vec<&str> = {
        let mut n: Vec<&str> = ALLOWED.iter().map(|(n, ..)| *n).collect();
        n.sort_unstable();
        n.dedup();
        n
    };
    let mut problems = Vec::new();
    for needle in &needles {
        let mut files: Vec<&str> = lines
            .iter()
            .filter(|(f, _, l)| f != MAIN_TESTS && l.contains(needle))
            .map(|(f, ..)| f.as_str())
            .collect();
        files.sort_unstable();
        files.dedup();
        for file in files {
            let actual = in_file(&lines, file)
                .filter(|(_, _, l)| l.contains(needle))
                .count();
            match ALLOWED.iter().find(|(n, f, ..)| n == needle && *f == file) {
                None => problems.push(format!(
                    "  {file}: {actual} line(s) call `{needle}` and are not in ALLOWED"
                )),
                Some((_, _, want, why)) if *want != actual => problems.push(format!(
                    "  {file}: `{needle}` on {actual} line(s), pinned at {want} ({why})"
                )),
                Some(_) => {}
            }
        }
        for (n, f, want, _) in ALLOWED {
            if n == needle && *want > 0 && !in_file(&lines, f).any(|(_, _, l)| l.contains(n)) {
                problems.push(format!("  {f}: `{n}` pinned at {want} but matches nothing"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "the shell entry points into the reading moved:\n{}\n\nA new caller is a new way to \
         start a read; add it to ALLOWED with its reason (S3 to S5 arrive with US4, milestone M7).",
        problems.join("\n")
    );
}

#[test]
fn the_listing_reaches_the_reducer_from_the_catalog_arm_only() {
    let lines = code_lines();

    let built: Vec<_> = lines
        .iter()
        .filter(|(f, _, l)| f != FEATURE && f != MAIN_TESTS && l.contains("Msg::ListingArrived"))
        .collect();
    assert!(
        built.len() == 1 && built[0].0 == SHELL,
        "`Msg::ListingArrived` must be built on one line, in `listing_arrived` of {SHELL}; \
         found:\n{}",
        show(&built)
    );

    let callers: Vec<_> = lines
        .iter()
        .filter(|(f, _, l)| f != MAIN_TESTS && l.contains("pr_status::listing_arrived("))
        .collect();
    assert!(
        callers.len() == 1 && callers[0].0 == SYNC,
        "`pr_status::listing_arrived(` must be called from one line, in {SYNC}; found:\n{}",
        show(&callers)
    );

    let sync: Vec<_> = in_file(&lines, SYNC).collect();
    let arm = sync
        .iter()
        .find(|(_, _, l)| l.starts_with("DaemonMsg::CatalogChanged { catalog } =>"))
        .expect("the `DaemonMsg::CatalogChanged { catalog } =>` arm of on_daemon_event moved");
    let next = sync
        .iter()
        .find(|(_, n, l)| *n > arm.1 && l.starts_with("DaemonMsg::"))
        .expect("no arm follows the CatalogChanged arm");
    let at = callers[0].1;
    assert!(
        arm.1 < at && at < next.1,
        "`listing_arrived` is called at {SYNC}:{at}, outside the CatalogChanged arm \
         ({}..{}). S1 is the catalog arriving and nothing else.",
        arm.1,
        next.1
    );
}

#[test]
fn the_source_is_read_from_one_place() {
    let lines = code_lines();

    let reads: Vec<_> = lines
        .iter()
        .filter(|(f, _, l)| f != MAIN_TESTS && l.contains("(tooling.pull_requests)("))
        .collect();
    assert!(
        reads.len() == 1 && reads[0].0 == SHELL,
        "`PullRequestSource::read` must be reached from one line, in {SHELL}; found:\n{}",
        show(&reads)
    );

    let factory: Vec<_> = lines
        .iter()
        .filter(|(f, _, l)| f != CAPS && f != MAIN_TESTS && l.contains(".pull_requests"))
        .collect();
    assert!(
        factory.len() == 1 && factory[0].0 == SHELL,
        "the `pull_requests` factory is used only by {SHELL} outside {CAPS}; found:\n{}",
        show(&factory)
    );
}

#[test]
fn a_pull_requests_title_and_address_are_never_logged() {
    let lines = code_lines();
    let logging = [
        "log_line",
        "format!",
        "println!",
        "eprintln!",
        "tracing",
        "dbg!",
    ];
    for file in [SHELL, FEATURE] {
        let own: Vec<_> = in_file(&lines, file).collect();
        let mut offenders = Vec::new();
        for (i, (_, n, l)) in own.iter().enumerate() {
            if l.contains(".title") || l.contains(".url") {
                offenders.push(format!("  {file}:{n}: {l} (reads a title or address)"));
            }
            if logging.iter().any(|m| l.contains(m)) {
                for (_, m, w) in own.iter().skip(i).take(5) {
                    if w.contains("title") || w.contains("url") {
                        offenders.push(format!("  {file}:{m}: {w} (in a logging statement)"));
                    }
                }
            }
        }
        offenders.dedup();
        assert!(
            offenders.is_empty(),
            "a pull request's title or address must never be logged (contract §1):\n{}",
            offenders.join("\n")
        );
    }
}

/// The text of the free function `name` in `file`, comments dropped: from its signature to the
/// first lone `}` at column 0.
fn function_body(file: &str, name: &str) -> String {
    let text = fs::read_to_string(repo_root().join(file))
        .unwrap_or_else(|e| panic!("cannot read {file}: {e}"));
    let start = text
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("`fn {name}(` moved out of {file}"));
    let rest = &text[start..];
    let end = rest
        .find("\n}\n")
        .unwrap_or_else(|| panic!("the end of `fn {name}` not found in {file}"));
    rest[..end]
        .lines()
        .filter(|l| !is_comment(l))
        .collect::<Vec<_>>()
        .join("\n")
}

/// FR-012, SC-008: what a hover draws and what a right-click offers is worked out from what the
/// window already holds. No builder may reach the shell, where the reading, the disk and the
/// opener live; opening the address is a message the shell handles afterwards.
#[test]
fn the_tooltip_and_the_menu_builder_call_nothing_in_the_shell() {
    for (file, name) in [
        (
            "crates/micold-client/src/features/sidebar.rs",
            "worktree_tooltip",
        ),
        (
            "crates/micold-client/src/features/sidebar.rs",
            "pull_request_lines",
        ),
        (
            "crates/micold-client/src/features/sidebar.rs",
            "tooltip_title",
        ),
        ("crates/micold-client/src/ui/mod.rs", "worktree_menu_items"),
    ] {
        let body = function_body(file, name);
        assert!(!body.is_empty());
        for needle in [
            "shell::",
            "std::fs",
            "std::process",
            "Command::",
            "LinkOpener",
            "Task::",
        ] {
            assert!(
                !body.contains(needle),
                "`{name}` in {file} must not reach `{needle}` (FR-012, SC-008)"
            );
        }
    }
}
