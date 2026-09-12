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

/// Feature 028's quickstart, which makes the same promise in the same shape (T063).
///
/// Its Part A is a list of *commands*, not of test targets, so what can rot is different: a command
/// that no longer exists, or — worse — one that still runs and selects nothing. See
/// [`every_command_part_a_gives_is_one_the_repo_provides`] and
/// [`a_test_filter_part_a_gives_selects_at_least_one_test`].
const QUICKSTART_028: &str = "specs/028-client-managed-daemon/quickstart.md";

/// The crate whose §A rows are enumerated in the workflow. `micold-core`'s are covered wholesale
/// by `--all-targets`, so only this one can drift.
const ENUMERATED_CRATE: &str = "micold-client";

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every scan below splits on a needle containing a newline (`"\n  test:\n"`, `"\n## "`), which
/// is safe on all three platforms because `.gitattributes` declares `eol=lf` -- see the note there.
/// It was not, before T115 first ran this on the Windows leg.
fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
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

// ---------------------------------------------------------------------------------------------
// Feature 028 (T063) — Part A's *commands*
// ---------------------------------------------------------------------------------------------

/// The section of `md` under a heading starting with `needle`, up to the next heading of that depth.
fn section(md: &str, needle: &str, source: &str) -> String {
    let after = md
        .split_once(needle)
        .unwrap_or_else(|| {
            panic!("{source} has no `{needle}` heading — the scan is looking at the wrong document")
        })
        .1;
    match after.split_once("\n## ") {
        Some((body, _)) => body.to_string(),
        None => after.to_string(),
    }
}

/// Every command line inside the ```` ```bash ```` blocks of `section`.
///
/// Comment-only lines are dropped and a trailing `# …` is trimmed, which is the one thing this
/// parser assumes about shell: that a `#` after whitespace starts a comment. The quickstart is
/// written that way, and a command that needed a literal `#` would be caught by the assertions
/// below rather than silently mis-read.
fn commands_in(section: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = section;
    while let Some((_, after)) = rest.split_once("```bash") {
        let (block, tail) = after.split_once("```").unwrap_or((after, ""));
        for line in block.lines() {
            let line = match line.split_once(" #") {
                Some((command, _)) => command,
                None => line,
            };
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            found.push(line.to_string());
        }
        rest = tail;
    }
    found
}

/// FR-023 / Principle VI in the form a reader meets it: Part A's commands have to be real.
///
/// A quickstart is the one document someone follows literally, and a `mise run` task that was
/// renamed leaves it failing at the first step with an error about the *tool*, which reads as the
/// repo being broken rather than the document being stale. Nothing else in the suite reads this
/// file, so nothing else would notice.
#[test]
fn every_command_part_a_gives_is_one_the_repo_provides() {
    let quickstart = read(QUICKSTART_028);
    let part_a = section(&quickstart, "## Part A", QUICKSTART_028);
    let commands = commands_in(&part_a);
    assert!(
        !commands.is_empty(),
        "Part A of {QUICKSTART_028} contains no commands — the scan is broken, not the quickstart"
    );

    let mise = read("mise.toml");
    for command in &commands {
        let Some(task) = command.strip_prefix("mise run ") else {
            continue;
        };
        let task = task.split_whitespace().next().unwrap_or("");
        assert!(
            mise.contains(&format!("[tasks.{task}]")),
            "Part A of {QUICKSTART_028} says to run `mise run {task}`, and mise.toml defines no \
             such task — the first thing anyone validating this feature does would fail"
        );
    }
}

/// The trap `mise.toml` documents at length, asserted rather than explained (T062).
///
/// `cargo test … <filter>` matches the filter against **test names**, not file names, and a filter
/// that selects nothing exits 0 and prints `test result: ok`. A quickstart line naming a file stem
/// that is not also a test-name prefix therefore reports success while checking nothing — which is
/// exactly what this feature's quickstart said (`sandbox_idle`) before T063.
#[test]
fn a_test_filter_part_a_gives_selects_at_least_one_test() {
    let quickstart = read(QUICKSTART_028);
    let part_a = section(&quickstart, "## Part A", QUICKSTART_028);

    let mut checked = 0;
    for command in commands_in(&part_a) {
        let Some(rest) = command.strip_prefix("cargo test ") else {
            continue;
        };
        // The filter is the last bare word: everything else in these lines is a flag or its value.
        let Some(filter) = rest.split_whitespace().last() else {
            continue;
        };
        if filter.starts_with('-') {
            continue; // no filter on this line; it runs the whole target.
        }
        let krate = crate_of(&command).unwrap_or_else(|| {
            panic!("`{command}` names no crate with -p, so this gate cannot find its tests")
        });
        assert!(
            some_test_name_matches(&krate, filter),
            "Part A of {QUICKSTART_028} says to run `{command}`, but no test in \
             crates/{krate}/tests/ has a name containing `{filter}`. cargo would filter every test \
             out, exit 0, and print `test result: ok` — a validation step that validates nothing."
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no `cargo test … <filter>` line found in Part A of {QUICKSTART_028}; if the quickstart \
         stopped giving one, delete this test rather than leave it passing vacuously"
    );
}

/// The crate named by `-p <name>` in a cargo command line.
fn crate_of(command: &str) -> Option<String> {
    let (_, after) = command.split_once("-p ")?;
    after.split_whitespace().next().map(str::to_string)
}

/// Whether any function in `krate`'s integration tests has a name containing `filter`.
///
/// Every function, not only the annotated ones: parsing out which are tests would need more of a
/// Rust parser than a text scan has, and the extra ones only make this *permissive*. What it is
/// for is the filter that matches nothing at all, which is the failure that reports success.
fn some_test_name_matches(krate: &str, filter: &str) -> bool {
    let dir = repo_root().join(format!("crates/{krate}/tests"));
    let Ok(entries) = fs::read_dir(&dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let mut rest = source.as_str();
        while let Some((_, after)) = rest.split_once("fn ") {
            let name: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name.contains(filter) {
                return true;
            }
            rest = after;
        }
    }
    false
}
