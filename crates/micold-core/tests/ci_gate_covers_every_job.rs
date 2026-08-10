//! Every CI job is covered by the aggregate gate (feature 023, FR-015).
//!
//! The default branch requires exactly one status check, `ci complete`, which summarises the run
//! instead of enumerating the pipeline's internals. That is what lets jobs be skipped on a
//! documentation-only change without making pull requests unmergeable — but it moves the risk: a
//! job added later and left out of the gate's `needs:` stops blocking merges, silently. Nothing in
//! GitHub Actions notices, because the workflow stays perfectly valid.
//!
//! So the coverage is asserted here rather than left to review, the same way `showcase_glue.rs`
//! asserts the precondition of Principle I's widened exemption instead of trusting a reviewer to
//! police it.
//!
//! # Why text, not YAML
//!
//! Top-level job ids sit at a known indent and `needs:` is a flat inline list, so a two-rule text
//! scan answers the question exactly. A real YAML parser would be a new dependency on the merge
//! path for no additional certainty, which the constitution's dependency constraint asks us not to
//! take.

use std::fs;
use std::path::{Path, PathBuf};

/// The job whose `needs:` list is the coverage, and the check the branch ruleset requires.
const GATE_ID: &str = "ci-complete";

/// Jobs deliberately outside the gate, with the reason. Empty today; an entry here is a claim that
/// a job's failure need not block a merge, which is exactly the kind of claim worth writing down.
const UNCOVERED_BY_DESIGN: &[(&str, &str)] = &[];

fn workflow_path() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/workflows/ci.yml")
}

fn workflow() -> String {
    let path = workflow_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Top-level job ids: two-space-indented keys inside the `jobs:` block.
///
/// Anything more deeply indented belongs to a job's body (`steps:`, `strategy:`, `with:`), and
/// anything at column zero has ended the block.
fn job_ids(source: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut in_jobs = false;
    for line in source.lines() {
        if line.starts_with("jobs:") {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        // A non-indented, non-blank, non-comment line ends the jobs block.
        if !line.starts_with(' ') && !line.trim().is_empty() && !line.trim_start().starts_with('#')
        {
            break;
        }
        let Some(rest) = line.strip_prefix("  ") else {
            continue;
        };
        if rest.starts_with(' ') || rest.trim().is_empty() || rest.trim_start().starts_with('#') {
            continue;
        }
        if let Some(id) = rest.strip_suffix(':') {
            ids.push(id.trim().to_string());
        }
    }
    ids
}

/// The gate's `needs:` entries, read from its inline list.
fn gate_needs(source: &str) -> Vec<String> {
    let gate_header = format!("  {GATE_ID}:");
    let body = source
        .split_once(&gate_header)
        .unwrap_or_else(|| {
            panic!(
                "no `{GATE_ID}:` job in {}. The branch ruleset requires the check this job \
                 produces; without it every pull request waits for a status nothing emits.",
                workflow_path().display()
            )
        })
        .1;

    let needs_line = body
        .lines()
        .take_while(|l| l.starts_with("    ") || l.trim().is_empty())
        .find(|l| l.trim_start().starts_with("needs:"))
        .unwrap_or_else(|| panic!("`{GATE_ID}` has no `needs:` — it would cover nothing"));

    let list = needs_line
        .split_once("needs:")
        .expect("needs: line contains needs:")
        .1
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');

    list.split(',')
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn every_job_is_covered_by_the_gate() {
    let source = workflow();
    let ids = job_ids(&source);
    assert!(
        ids.len() > 1,
        "parsed {} job id(s) from ci.yml — the scan is broken, not the workflow",
        ids.len()
    );
    assert!(
        ids.iter().any(|id| id == GATE_ID),
        "ci.yml has no `{GATE_ID}` job; parsed ids: {ids:?}"
    );

    let needs = gate_needs(&source);
    let exempt: Vec<&str> = UNCOVERED_BY_DESIGN.iter().map(|(id, _)| *id).collect();

    let uncovered: Vec<&String> = ids
        .iter()
        .filter(|id| id.as_str() != GATE_ID)
        .filter(|id| !needs.contains(id))
        .filter(|id| !exempt.contains(&id.as_str()))
        .collect();

    assert!(
        uncovered.is_empty(),
        "these ci.yml jobs are not in `{GATE_ID}`'s `needs:` — their failures would not block a \
         merge, because `{GATE_ID}` is the only check the default branch requires: {uncovered:?}\n\
         Add them to `needs:`, or record them in UNCOVERED_BY_DESIGN with a reason."
    );
}

#[test]
fn the_gate_runs_even_when_upstream_fails() {
    let source = workflow();
    let gate_header = format!("  {GATE_ID}:");
    let body = source
        .split_once(&gate_header)
        .unwrap_or_else(|| panic!("no `{GATE_ID}:` job in ci.yml"))
        .1;

    let has_always = body
        .lines()
        .take_while(|l| l.starts_with("    ") || l.trim().is_empty())
        .any(|l| l.trim_start().starts_with("if:") && l.contains("always()"));

    assert!(
        has_always,
        "`{GATE_ID}` must be `if: always()`. Without it the implicit `success()` applies, the gate \
         is *skipped* whenever a covered job fails — and a skipped check reports success. The gate \
         would go green exactly when the run went red."
    );
}

#[test]
fn every_exemption_names_a_real_job() {
    let ids = job_ids(&workflow());
    for (id, reason) in UNCOVERED_BY_DESIGN {
        assert!(
            ids.iter().any(|j| j == id),
            "UNCOVERED_BY_DESIGN lists `{id}` ({reason}) but ci.yml has no such job — a stale \
             exemption hides the next real one"
        );
    }
}
