//! Nothing a developer runs may write the developer's own data directory (029 T069).
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
//! in `micold-client/src/main_tests.rs`. That guard only covers the client's `base_app()`. These two
//! cover the rest of the surface: every test file in the workspace, and the one recipe outside the
//! suite that launches the real binaries.
//!
//! # Why a text scan
//!
//! Both invariants are about what a source file *says*, not what it computes. A test that resolves
//! the conventional data directory names `default_location()`, and a test that keeps that resolution
//! private names `XDG_DATA_HOME`. Reading the two names off the lines answers the question exactly,
//! where running the tests to see what they touch would not — a test only writes the real file on
//! the machine that has one.

use std::fs;
use std::path::{Path, PathBuf};

/// The constructors that resolve the conventional per-user data directory: `JsonFileStore` for
/// `projects.json`, `JsonFileSettingsStore` for `settings.json`.
const RESOLVES_THE_REAL_DIR: &str = "default_location()";

/// The variable that redirects that resolution somewhere private. A test file that sets it is
/// isolated; the `HOME` fallback macOS uses travels with it, so naming this one is enough.
const ISOLATES_IT: &str = "XDG_DATA_HOME";

fn workspace_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> workspace root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file in the workspace that holds tests: the three crates' integration test directories
/// including their shared-helper subdirectories, plus the client's unit tests, which live in their
/// own file rather than inside `main.rs`.
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

/// Append every `.rs` file under `dir`, in a stable order, so an offender list reads the same twice.
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

/// `true` when `source` calls a `default_location()` constructor.
///
/// Comment lines and lines carrying a `"` are skipped, because two files name the constructor
/// without calling it: `no_concrete_implementations.rs` discusses it in a module comment, and
/// `showcase_isolation.rs` embeds it in a string literal it feeds to its own source scanner. A real
/// call site is neither a comment nor inside a string.
fn resolves_the_real_data_dir(source: &str) -> bool {
    source
        .lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with("//") && !line.contains('"'))
        .any(|line| line.contains(RESOLVES_THE_REAL_DIR))
}

#[test]
fn a_test_that_resolves_the_real_data_directory_redirects_it_first() {
    let sources = test_sources();
    assert!(
        sources.len() > 50,
        "found only {} test files — the scan is broken, not the suite",
        sources.len()
    );

    let offenders: Vec<String> = sources
        .iter()
        .filter(|path| {
            let source =
                fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            resolves_the_real_data_dir(&source) && !source.contains(ISOLATES_IT)
        })
        .map(|path| path.display().to_string())
        .collect();

    assert!(
        offenders.is_empty(),
        "these test files call `{RESOLVES_THE_REAL_DIR}` without setting `{ISOLATES_IT}`, so they \
         read and write the developer's own `~/.local/share/micold-ai-ide`: {offenders:?}\n\
         Build the store with `::at(<tempdir>/settings.json)` instead, or set `{ISOLATES_IT}` (and \
         `HOME`, for macOS) to a temporary directory before the resolution happens."
    );
}

/// The visual-pass skill launches the real client and the real daemon, so its recipe is a writer of
/// the data directory in exactly the way a test is — and unlike a test, nothing else stops it. Its
/// own cleanup step matches processes by their `XDG_RUNTIME_DIR`, so a launch that omits the
/// isolation is both a write into the developer's configuration and a process nobody can identify
/// as theirs afterwards.
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

    for variable in [ISOLATES_IT, "XDG_RUNTIME_DIR"] {
        assert!(
            launch.contains(variable),
            "the visual-pass launch recipe in {} does not set `{variable}`, so a pass that follows \
             it runs the real client and daemon against the developer's own \
             `~/.local/share/micold-ai-ide`:\n{launch}",
            path.display()
        );
    }
}
