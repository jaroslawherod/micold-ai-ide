//! No test file, and no recipe that launches the real binaries, may reach the developer's own data
//! directory (029 T069).
//!
//! `~/.local/share/micold-ai-ide` is where the application the developer actually uses keeps
//! `settings.json`, `projects.json`, the client log and the materialised Pi component. A test or a
//! visual pass that resolves that directory instead of a private one does not fail — it silently
//! edits the developer's live configuration, and the damage surfaces much later as a bug report
//! about a setting nobody chose. That is exactly what happened before #368, when two client save
//! tests wrote their fixture draft (`env_include_script_path: "/tmp/does-not-exist.sh"` among it)
//! into the real file on every client test run.
//!
//! #368 closed that one writer and pinned it with `the_test_app_cannot_reach_the_real_settings_file`
//! in `micold-client/src/main_tests.rs`, which also covers the project catalog since T069. These two
//! cover what that guard cannot see: the rest of the test files, and the one recipe outside the suite
//! that launches the real binaries.
//!
//! # What is scanned, and what is not
//!
//! The three crates' `tests/` trees, including their helper subdirectories, plus the client's unit
//! tests, which live in `main_tests.rs` rather than inside `main.rs`. Not the `#[cfg(test)]` modules
//! under `crates/*/src`: production code in those same files *must* resolve the real directory —
//! `Capabilities::real`, `Catalog::load_default`, `JsonFileSettingsStore::default_location` itself —
//! so a file-level text scan there reports the assembly points it is supposed to leave alone. The
//! client's unit tests are the one `src` file worth scanning because the whole file is tests.
//!
//! This is a text scan, so it sees a test that *names* a resolver, not a test that reaches one
//! through production code. `materialise_pi_activity_component` in the daemon is the case in point:
//! a test that starts a Pi session writes the real `pi/` directory without naming anything, and only
//! knowing to set `XDG_DATA_HOME` keeps it private (`micold-daemon/tests/pi_launch_wiring.rs` does).
//! A scan cannot close that; what it can do is make the direct route impossible and say so here.
//!
//! # Why text, not behaviour
//!
//! Both invariants are about what a source file says. Running the tests to see what they touch only
//! works on a machine that has a real data directory to damage, which is the wrong place to find out.

use std::fs;
use std::path::{Path, PathBuf};

/// The ways a file reaches the conventional per-user data directory: the two store constructors
/// (`JsonFileStore` for `projects.json`, `JsonFileSettingsStore` for `settings.json`) and the
/// `directories` call they and the daemon's log and Pi paths are all built on.
const RESOLVERS: &[&str] = &["default_location()", "ProjectDirs::from("];

/// The calls that redirect that resolution somewhere private, as a call rather than a mention: a
/// comment naming the variable is not isolation. `HOME` is required alongside `XDG_DATA_HOME`
/// because `directories` ignores the XDG variables on macOS and resolves
/// `$HOME/Library/Application Support` instead, so an `XDG_DATA_HOME`-only test is isolated on the
/// Linux leg and writes the real directory on the macOS one.
const ISOLATORS: &[(&str, &str)] = &[
    ("XDG_DATA_HOME", "the data directory on Linux"),
    (
        "HOME",
        "the data directory on macOS, which ignores XDG_DATA_HOME",
    ),
];

/// Files that name a resolver without calling one.
///
/// `(file relative to the repository root, why it is not a call)`. A stale entry fails too: a list
/// that can rot is a list that stops meaning anything.
const ALLOWED: &[(&str, &str)] = &[(
    "crates/micold-client/tests/showcase_isolation.rs",
    "A Rust snippet inside a string literal, fed to that test's own source scanner as the example \
     of a forbidden construction. It is text the test greps, not code it runs.",
)];

/// This file, which is exempt from its own scan.
///
/// `RESOLVERS` has to spell out the needles, so scanning this file finds every one of them and
/// reports the gate tripping over its own bookkeeping — the precedent
/// `documentation_is_not_read.rs` set for the same problem. The cost is that a genuine resolution
/// *inside this file* would go unnoticed; this file resolves nothing, and the two tests below say so.
const SELF: &str = "crates/micold-core/tests/tests_never_write_the_real_data_directory.rs";

fn workspace_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> workspace root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file that holds tests, in a stable order, so an offender list reads the same twice.
fn test_sources() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut sources = vec![root.join("crates/micold-client/src/main_tests.rs")];
    for crate_name in ["micold-client", "micold-core", "micold-daemon"] {
        collect_rust_files(
            &root.join("crates").join(crate_name).join("tests"),
            &mut sources,
        );
    }
    sources
}

fn collect_rust_files(dir: &Path, into: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_rust_files(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
}

/// `true` when `source` names a resolver on a line that is not a comment.
///
/// Comments are skipped because two files discuss the constructors in prose. String literals are
/// *not* skipped — a line carrying a `"` is the idiomatic call
/// (`default_location().expect("no data dir")`), so skipping those would skip the common case. The
/// one file that really does quote a resolver is in `ALLOWED` instead.
fn names_a_resolver(source: &str) -> bool {
    source
        .lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with("//"))
        .any(|line| RESOLVERS.iter().any(|needle| line.contains(needle)))
}

/// `true` when `source` redirects `variable` with a call, rather than merely mentioning it.
///
/// Both forms the suite uses: `std::env::set_var(..)` for the test's own process, and `.env(..)` on
/// a `Command` for a daemon it spawns.
fn redirects(source: &str, variable: &str) -> bool {
    [
        format!("set_var(\"{variable}\""),
        format!(".env(\"{variable}\""),
    ]
    .iter()
    .any(|call| source.contains(call.as_str()))
}

#[test]
fn a_test_that_resolves_the_real_data_directory_redirects_it_first() {
    let root = workspace_root();
    let sources = test_sources();
    assert!(
        sources.len() > 50,
        "found only {} test files — the scan is broken, not the suite",
        sources.len()
    );

    let mut allowed: Vec<PathBuf> = Vec::new();
    for (entry, _) in ALLOWED {
        let path = root.join(entry);
        assert!(
            path.exists(),
            "`ALLOWED` names {entry}, which no longer exists — drop the entry"
        );
        allowed.push(path);
    }

    let this_file = root.join(SELF);
    assert!(
        this_file.exists(),
        "`SELF` names {SELF}, which does not exist — the exemption is misspelled, and this file is \
         then scanned for its own needles"
    );

    let mut offenders: Vec<String> = Vec::new();
    let mut stale: Vec<String> = Vec::new();
    for path in &sources {
        if same_file(&this_file, path) {
            continue;
        }
        let source =
            fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let exempt = allowed.iter().any(|a| same_file(a, path));
        if !names_a_resolver(&source) {
            if exempt {
                stale.push(path.display().to_string());
            }
            continue;
        }
        if exempt {
            continue;
        }
        let missing: Vec<&str> = ISOLATORS
            .iter()
            .filter(|(variable, _)| !redirects(&source, variable))
            .map(|(variable, _)| *variable)
            .collect();
        if !missing.is_empty() {
            offenders.push(format!("{} (missing {missing:?})", path.display()));
        }
    }

    assert!(
        stale.is_empty(),
        "these files are in `ALLOWED` but no longer name a resolver: {stale:?} — drop the entries, \
         so the list keeps meaning something"
    );
    assert!(
        offenders.is_empty(),
        "these test files resolve the developer's own `~/.local/share/micold-ai-ide` — they name \
         one of {RESOLVERS:?} without redirecting it: {offenders:?}\n\
         Build the store with `::at(<tempdir>/settings.json)` instead, or call \
         `std::env::set_var` (or `Command::env`) for both `XDG_DATA_HOME` and `HOME` with a \
         temporary directory before the resolution happens — `XDG_DATA_HOME` alone leaves the \
         macOS leg writing the real directory."
    );
}

/// Compare paths by their canonical form: `test_sources` builds `crates/micold-core/../../crates/…`
/// while `ALLOWED` builds `crates/…`, and those are the same file.
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// The visual-pass skill launches the real client and the real daemon, so its recipe writes the data
/// directory in exactly the way a test does — and unlike a test, nothing else stops it. Its own
/// cleanup step identifies the processes it may stop by their `XDG_RUNTIME_DIR`, so a launch that
/// omits the isolation is both a write into the developer's configuration and a process nobody can
/// recognise as theirs afterwards.
///
/// The absolute-path check is part of what is pinned, not decoration. `directories` discards an
/// empty or relative `XDG_DATA_HOME` and falls back to `$HOME/.local/share` without a word, so a
/// recipe that sets the variable to a placeholder is indistinguishable from one that never set it.
#[test]
fn the_visual_pass_launch_recipe_isolates_the_data_directory() {
    let path = workspace_root().join(".claude/skills/visual-pass/SKILL.md");
    let skill =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

    let launch = skill
        .split("### 4. Launch")
        .nth(1)
        .unwrap_or_else(|| panic!("{} has no `### 4. Launch` section", path.display()))
        .split("```")
        .nth(1)
        .unwrap_or_else(|| panic!("{}'s launch section has no code block", path.display()));

    for required in [
        "XDG_DATA_HOME=",
        "XDG_RUNTIME_DIR=",
        "must be absolute",
        "chmod 700",
    ] {
        assert!(
            launch.contains(required),
            "the visual-pass launch recipe in {} is missing `{required}`, so a pass that follows it \
             can run the real client and daemon against the developer's own \
             `~/.local/share/micold-ai-ide`:\n{launch}",
            path.display()
        );
    }

    assert!(
        !launch.contains("=<"),
        "the visual-pass launch recipe in {} assigns an angle-bracket placeholder. Pasted as \
         written, `x=<a path>` is not an assignment at all — it is an empty assignment plus two \
         redirections — so the variable ends up empty and `directories` falls back to the real data \
         directory without a word. Spell the paths out instead. (`<binary>` elsewhere in the block \
         is fine: a missing binary fails loudly.)\n{launch}",
        path.display()
    );
}
