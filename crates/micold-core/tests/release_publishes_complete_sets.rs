//! Every job that attaches an artifact holds up publication (feature 028, FR-011).
//!
//! release-please creates the GitHub release as a **draft**, and the `publish` job is what flips it
//! to published — at which point it is immutable and can never be corrected. GitHub Actions runs a
//! job with `needs:` only when all of its dependencies succeeded, so listing every artifact-
//! producing job there is the whole of FR-011: if one of them fails, `publish` never runs, the
//! release stays an unpublished draft, and the failed job is red for someone to re-run. Nothing
//! partial goes out and no version number is consumed.
//!
//! The failure this file exists to prevent is not a mistake in that reasoning. It is the *next*
//! artifact job — a Windows build, a checksum file — added with its own upload step and not added
//! to `needs:`. The workflow stays perfectly valid, the release publishes without waiting for it,
//! and the first evidence is an incomplete release that cannot be fixed. This feature is exactly
//! that scenario happening once already: `macos` is the job being added today.
//!
//! # What counts as an artifact-producing job
//!
//! One whose body contains `gh release upload`. That is the only way an asset reaches the draft
//! (contracts/release-artifacts.md), so it is the definition rather than a heuristic — a job that
//! attaches nothing needs no entry in `needs:`, and a job that attaches something cannot avoid the
//! command.
//!
//! # Why text, not YAML
//!
//! Same reason as `ci_gate_covers_every_job.rs`: top-level job ids sit at a known indent and
//! `needs:` is a flat inline list, so a two-rule scan answers the question exactly. It also lets
//! the scan be pointed at a *synthetic* workflow, which is what proves the check can fail — a
//! checker only ever run against a passing input is not evidence of anything.

use std::fs;
use std::path::{Path, PathBuf};

/// The job that publishes the draft. Its `needs:` is the gate.
const PUBLISH_ID: &str = "publish";

/// The command that attaches an asset to the draft release.
const UPLOAD: &str = "gh release upload";

fn workflow_path() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/workflows/release.yml")
}

fn workflow() -> String {
    let path = workflow_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every top-level job as `(id, body)`, the body running to the next job id.
///
/// Two-space-indented keys inside `jobs:`; anything deeper belongs to a job, anything at column
/// zero has ended the block.
fn jobs(source: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut in_jobs = false;
    for line in source.lines() {
        if line.starts_with("jobs:") {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        if !line.starts_with(' ') && !line.trim().is_empty() && !line.trim_start().starts_with('#')
        {
            break;
        }
        let is_job_header = line
            .strip_prefix("  ")
            .filter(|rest| !rest.starts_with(' '))
            .filter(|rest| !rest.trim().is_empty() && !rest.trim_start().starts_with('#'))
            .and_then(|rest| rest.strip_suffix(':'))
            .map(|id| id.trim().to_string());

        match is_job_header {
            Some(id) => out.push((id, String::new())),
            None => {
                if let Some((_, body)) = out.last_mut() {
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }
    }
    out
}

/// The jobs that attach something to the release.
///
/// Comment lines are dropped before the match. Without that, a job whose comment *mentions* the
/// upload command counts as running it -- which is exactly what happened to `publish`, whose comment
/// explains that its `needs:` must equal the set of jobs that run `gh release upload` and thereby
/// nominated itself as one of them. The note that says why a rule exists must not be read as the
/// thing the rule is about.
fn artifact_jobs(source: &str) -> Vec<String> {
    jobs(source)
        .into_iter()
        .filter(|(_, body)| {
            body.lines()
                .filter(|l| !l.trim_start().starts_with('#'))
                .any(|l| l.contains(UPLOAD))
        })
        .map(|(id, _)| id)
        .collect()
}

/// `publish`'s `needs:` entries, from either the inline-list or block spelling.
fn publish_needs(source: &str) -> Vec<String> {
    let body = jobs(source)
        .into_iter()
        .find(|(id, _)| id == PUBLISH_ID)
        .unwrap_or_else(|| {
            panic!(
                "no `{PUBLISH_ID}:` job in {} — the draft would never be published at all",
                workflow_path().display()
            )
        })
        .1;

    let needs_line = body
        .lines()
        .find(|l| l.trim_start().starts_with("needs:"))
        .unwrap_or_else(|| {
            panic!("`{PUBLISH_ID}` has no `needs:` — it would publish before anything was built")
        });

    needs_line
        .split_once("needs:")
        .expect("needs: line contains needs:")
        .1
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The artifact jobs `publish` does not wait for. Empty is the only acceptable answer.
fn unwaited(source: &str) -> Vec<String> {
    let needs = publish_needs(source);
    artifact_jobs(source)
        .into_iter()
        .filter(|id| !needs.contains(id))
        .collect()
}

// ---------------------------------------------------------------------------
// The gate.
// ---------------------------------------------------------------------------

#[test]
fn publish_waits_for_every_job_that_attaches_an_artifact() {
    let source = workflow();
    let found = unwaited(&source);
    assert!(
        found.is_empty(),
        "these release.yml jobs upload a release asset but are not in `{PUBLISH_ID}`'s `needs:`: \
         {found:?}\n\n\
         `{PUBLISH_ID}` would flip the draft to published without waiting for them, and a published \
         release is immutable — the missing asset can never be added and the version number is \
         spent. Add each of them to `needs:` (contracts/release-artifacts.md)."
    );
}

// ---------------------------------------------------------------------------
// The scan reads the real thing, and can fail.
// ---------------------------------------------------------------------------

/// A workflow of the shape this file is about: two artifact jobs, one publisher.
fn synthetic(publish_needs: &str) -> String {
    format!(
        "name: release\n\
         jobs:\n\
        \x20 deb:\n\
        \x20   runs-on: ubuntu-latest\n\
        \x20   steps:\n\
        \x20     - run: gh release upload \"$TAG_NAME\" target/debian/*.deb --clobber\n\
        \x20 macos:\n\
        \x20   runs-on: macos-latest\n\
        \x20   steps:\n\
        \x20     - run: gh release upload \"$TAG_NAME\" target/macos/*.dmg --clobber\n\
        \x20 notes:\n\
        \x20   runs-on: ubuntu-latest\n\
        \x20   # Talks about `gh release upload` without running it -- see `artifact_jobs`.\n\
        \x20   steps:\n\
        \x20     - run: echo 'builds nothing'\n\
        \x20 publish:\n\
        \x20   needs: {publish_needs}\n\
        \x20   runs-on: ubuntu-latest\n\
        \x20   steps:\n\
        \x20     - run: gh release edit \"$TAG_NAME\" --draft=false --latest\n"
    )
}

#[test]
fn the_scan_actually_reads_the_release_workflow() {
    let source = workflow();
    let ids: Vec<String> = jobs(&source).into_iter().map(|(id, _)| id).collect();
    assert!(
        ids.iter().any(|id| id == "release-please") && ids.iter().any(|id| id == PUBLISH_ID),
        "parsed {ids:?} from release.yml — the scan is broken, not the workflow"
    );
    assert!(
        !artifact_jobs(&source).is_empty(),
        "release.yml appears to attach no artifacts at all; parsed jobs: {ids:?}"
    );
}

#[test]
fn a_complete_needs_list_passes() {
    let source = synthetic("[release-please, deb, macos]");
    assert_eq!(artifact_jobs(&source), vec!["deb", "macos"]);
    assert!(unwaited(&source).is_empty());
}

#[test]
fn an_artifact_job_left_out_of_needs_is_caught() {
    // The whole point: `macos` builds and uploads, `publish` does not wait for it, and the workflow
    // is still valid YAML that GitHub would run happily.
    let source = synthetic("[release-please, deb]");
    assert_eq!(unwaited(&source), vec!["macos".to_string()]);
}

#[test]
fn a_comment_naming_the_upload_command_is_not_an_upload() {
    // `notes` mentions the command in a comment and runs `echo`. Counting it would put every job
    // that documents this rule -- `publish` first among them -- into the list of jobs `publish` must
    // wait for, which is both wrong and unsatisfiable.
    let source = synthetic("[release-please, deb, macos]");
    assert_eq!(artifact_jobs(&source), vec!["deb", "macos"]);
    assert!(unwaited(&source).is_empty());
}

#[test]
fn a_job_that_uploads_nothing_needs_no_entry() {
    // The check must not degenerate into "every job must be in `needs:`" — that would be a
    // different, wrong rule, and the first person to add a lint job would delete it.
    let source = synthetic("[release-please, deb, macos]");
    assert!(
        !artifact_jobs(&source).contains(&"notes".to_string()),
        "a job with no upload step must not be treated as artifact-producing"
    );
}
