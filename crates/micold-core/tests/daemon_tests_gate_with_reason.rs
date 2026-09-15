//! Every Unix-only daemon test file says why (feature 030, FR-025).
//!
//! The Windows CI leg runs `cargo test -p micold-daemon --all-targets`, but a file that opens with
//! `#![cfg(unix)]` compiles to nothing there and reports no failure. That is how the Windows
//! endpoint stayed a stub while the suite stayed green: the tests that would have caught it were
//! never built on the platform they had to prove. A gate is sometimes right (a test that drives a
//! Unix socket's file mode has nothing to say on Windows), so the rule is not "no gates" but "no
//! silent gates": the line directly above `#![cfg(unix)]` must start with `// unix-only:` and give
//! the reason, so a reviewer can tell a deliberate exclusion from a forgotten one.
//!
//! # Why text, not a parser
//!
//! The attribute and its reason are two adjacent lines at a fixed place in the file. A line scan
//! answers the question exactly; parsing Rust would add a dependency for no extra certainty.

use std::fs;
use std::path::{Path, PathBuf};

/// The inner attribute that compiles a whole test file out on Windows.
const UNIX_GATE: &str = "#![cfg(unix)]";

/// The prefix the line directly above the gate must carry.
const REASON_PREFIX: &str = "// unix-only:";

fn daemon_tests_dir() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> micold-daemon/tests
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../micold-daemon/tests")
}

/// `true` when `source` has a whole-file Unix gate with no reason line directly above it.
fn has_unreasoned_gate(source: &str) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    lines.iter().enumerate().any(|(i, line)| {
        line.trim() == UNIX_GATE
            && (i == 0 || !lines[i - 1].trim_start().starts_with(REASON_PREFIX))
    })
}

#[test]
fn unix_gated_daemon_test_files_state_a_reason() {
    let dir = daemon_tests_dir();
    let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    entries.sort();
    assert!(
        !entries.is_empty(),
        "found no test files in {} — the scan is broken, not the suite",
        dir.display()
    );

    let offenders: Vec<String> = entries
        .iter()
        .filter(|path| {
            let source =
                fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            has_unreasoned_gate(&source)
        })
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();

    assert!(
        offenders.is_empty(),
        "these daemon test files are compiled out on Windows with no stated reason, so the \
         Windows CI leg silently skips them: {offenders:?}\n\
         Remove the `{UNIX_GATE}`, gate only the cases that need Unix, or put a `{REASON_PREFIX} \
         <what it depends on>` line directly above the gate."
    );
}
