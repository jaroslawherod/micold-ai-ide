//! Every §A gate really does run on all three platforms (feature 027, T115).
//!
//! Feature 027's quickstart opens with a promise: "**§A** is what the machine checks, on every
//! platform, with **no container runtime installed**." Principle VI is that promise, and the fake
//! `exec::CommandRunner` is what makes it affordable — the adapter layer is exercised without
//! anything on `PATH`.
//!
//! For the `micold-core` rows that promise keeps itself, because CI runs
//! `cargo test -p micold-core --all-targets` on each platform: a gate added to this crate is
//! covered the moment it exists. The `micold-client` rows are different. Its suite needs the iced
//! system dependencies, so CI runs the whole of it only on Linux and names the render-free
//! exceptions one `--test` flag at a time.
//!
//! An enumerated list drifts in exactly one direction, and silently. Add a §A gate to the
//! quickstart's table, forget the flag, and nothing fails: the test still runs on Linux, in the
//! full-workspace step, and the summary is green. What is quietly lost is the *three-platform*
//! claim — the one the table is making, and the one T115 was written to verify. That is the
//! failure this file exists to make loud, and it is not hypothetical: it is what T115 found.
//!
//! `ci_gate_covers_every_job.rs` is the precedent, for the same reason and against the same class
//! of silence — a workflow that stays perfectly valid while the guarantee behind it lapses. The
//! text-scan rationale there applies here unchanged: a YAML dependency on the merge path buys no
//! additional certainty.

use std::fs;
use std::path::{Path, PathBuf};

/// The quickstart whose §A table is the claim. Named rather than globbed: this is feature 027's
/// promise, and a later feature making the same promise should say so in its own gate.
const QUICKSTART: &str = "specs/027-sandboxed-daemon-runtime/quickstart.md";

/// The crate whose §A rows are enumerated in the workflow. `micold-core`'s are covered wholesale
/// by `--all-targets`, so only this one can drift.
const ENUMERATED_CRATE: &str = "micold-client";

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The file's text, with line endings normalised to `\n`.
///
/// Every scan below splits on a needle containing a newline (`"\n  test:\n"`, `"\n## "`), and a
/// Windows runner checks these files out with CRLF -- `core.autocrlf` is `true` there by default.
/// Without this, all three needles miss and the panics blame the documents: "ci.yml has no `test:`
/// job", on a ci.yml that plainly has one. That is what T115's first run on the matrix reported.
fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// The §A section only. §B is the manual pass and makes no claim about any platform.
fn section_a(quickstart: &str) -> &str {
    let after = quickstart
        .split_once("## §A")
        .unwrap_or_else(|| {
            panic!(
                "{QUICKSTART} has no `## §A` heading — the scan is looking at the wrong document"
            )
        })
        .1;
    match after.split_once("\n## ") {
        Some((body, _)) => body,
        None => after,
    }
}

/// Test targets named in §A's gate table, as `<crate>/tests/<name>.rs`, for one crate.
///
/// Unit-test rows (`src/...`) are deliberately not collected: they ride along with their crate's
/// own test run and cannot be named with `--test`.
fn targets_claimed(section: &str, krate: &str) -> Vec<String> {
    let needle = format!("{krate}/tests/");
    let mut found = Vec::new();
    for line in section.lines() {
        // Table rows only. Prose mentions a file to explain it, not to claim a gate for it.
        if !line.trim_start().starts_with('|') {
            continue;
        }
        let mut rest = line;
        while let Some((_, after)) = rest.split_once(&needle) {
            let name: String = after.chars().take_while(|c| *c != '.').collect();
            if !name.is_empty() && !found.contains(&name) {
                found.push(name);
            }
            rest = after;
        }
    }
    found
}

/// The body of the `test:` job — the matrix one, the only job that runs anywhere but Linux.
fn matrix_job(workflow: &str) -> &str {
    let after = workflow
        .split_once("\n  test:\n")
        .unwrap_or_else(|| {
            panic!("ci.yml has no `test:` job — that job *is* the three-platform matrix")
        })
        .1;
    // A job ends where the next two-space-indented key begins.
    let mut end = after.len();
    for (offset, line) in line_offsets(after) {
        if line.starts_with("  ") && !line.starts_with("   ") && line.trim_end().ends_with(':') {
            end = offset;
            break;
        }
    }
    &after[..end]
}

fn line_offsets(source: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut offset = 0;
    source.lines().map(move |line| {
        let at = offset;
        offset += line.len() + 1;
        (at, line)
    })
}

/// `--test <name>` flags from the matrix job's steps that are **not** restricted to one platform.
///
/// A step carrying `if: runner.os == ...` runs on that platform alone, so what it names proves
/// nothing about the other two — which is precisely the distinction the whole file turns on.
fn targets_run_everywhere(job: &str) -> Vec<String> {
    let mut found = Vec::new();
    for step in job.split("\n      - ") {
        let platform_restricted = step
            .lines()
            .any(|l| l.trim_start().starts_with("if:") && l.contains("runner.os"));
        if platform_restricted {
            continue;
        }
        let mut rest = step;
        while let Some((_, after)) = rest.split_once("--test ") {
            let name: String = after
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '\\')
                .collect();
            if !name.is_empty() && !found.contains(&name) {
                found.push(name);
            }
            rest = after;
        }
    }
    found
}

#[test]
fn every_client_side_section_a_gate_is_named_in_the_cross_platform_step() {
    let section = read(QUICKSTART);
    let section = section_a(&section);
    let claimed = targets_claimed(section, ENUMERATED_CRATE);

    assert!(
        !claimed.is_empty(),
        "§A's table names no `{ENUMERATED_CRATE}/tests/*.rs` gate. The scan is broken, not the \
         quickstart — this file has nothing to check if that is genuinely true, and should be \
         deleted rather than left passing vacuously."
    );

    let workflow = read(".github/workflows/ci.yml");
    let run_everywhere = targets_run_everywhere(matrix_job(&workflow));

    let missing: Vec<&String> = claimed
        .iter()
        .filter(|t| !run_everywhere.contains(t))
        .collect();

    assert!(
        missing.is_empty(),
        "§A of {QUICKSTART} claims these gates run on Linux, macOS and Windows, but ci.yml's \
         matrix job names them in no cross-platform step — so they run on Linux only, and the \
         claim is false: {missing:?}\n\
         Add `--test <name>` to \"Test (component library + showcase gates, all platforms)\", or \
         stop claiming §A covers them.\n\
         Cross-platform targets found: {run_everywhere:?}"
    );
}

#[test]
fn the_cross_platform_step_names_only_targets_that_exist() {
    let workflow = read(".github/workflows/ci.yml");
    let job = matrix_job(&workflow);

    for target in targets_run_everywhere(job) {
        let path = repo_root().join(format!("crates/{ENUMERATED_CRATE}/tests/{target}.rs"));
        assert!(
            path.exists(),
            "ci.yml runs `--test {target}` on every platform, but \
             crates/{ENUMERATED_CRATE}/tests/{target}.rs does not exist. cargo fails the step \
             rather than skipping it, so this breaks the matrix on all three runners at once."
        );
    }
}
