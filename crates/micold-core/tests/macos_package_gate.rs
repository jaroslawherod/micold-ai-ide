//! The per-pull-request macOS packaging gate exists, and §B still describes it (feature 028, T020).
//!
//! FR-036 asks for one thing: a change that breaks packaging should be caught by the pull request
//! that introduces it, not by the release it would otherwise block. That guarantee lives in a
//! single step of `ci.yml`'s macOS leg, and a step is the easiest thing in a workflow to lose --
//! deleting it leaves the file perfectly valid, every job still green, and the guarantee gone.
//!
//! `specs/028-macos-package/quickstart.md` §B states what that step does, row by row. Two
//! enumerated lists of the same thing drift in one direction and silently: someone drops the
//! socket wait from the workflow because it was flaky, §B keeps promising it, and the document a
//! reader trusts is now wrong about the only automated proof the application starts at all.
//!
//! So this file pins the pair. Each row of [`CHECKS`] names a behaviour, the text that proves the
//! workflow still performs it, and the text that proves §B still claims it. Losing either side
//! fails here.
//!
//! Signing and signature verification are deliberately absent: they belong to
//! `scripts/macos-bundle.sh`, which the step invokes, and `macos_signature_gate.rs` pins them
//! there. Asserting them again over the workflow would pin a copy rather than the original.
//!
//! `quickstart_a_runs_everywhere.rs` is the precedent -- same class of silence, same text-scan
//! answer, and the same reason a YAML dependency on the merge path would buy no more certainty.

use std::fs;
use std::path::{Path, PathBuf};

/// The quickstart whose §B is the claim. Carved out of the documentation set in `.gitattributes`
/// for exactly this reason: were it documentation, editing §B would skip the whole pipeline, so
/// the one edit this gate exists to catch would be the one edit it never sees.
const QUICKSTART: &str = "specs/028-macos-package/quickstart.md";

/// One behaviour of the gate, as the workflow performs it and as §B promises it.
struct Check {
    /// What the step does, in the failure message's terms.
    what: &'static str,
    /// Text the macOS step must contain.
    in_workflow: &'static str,
    /// Text §B must contain.
    in_section_b: &'static str,
}

/// The gate, enumerated once. Adding a row here is how a new behaviour becomes load-bearing;
/// removing one is how it stops being.
const CHECKS: &[Check] = &[
    Check {
        what: "composes and signs the bundle with the packaging script",
        in_workflow: "scripts/macos-bundle.sh",
        in_section_b: "Compose the bundle",
    },
    Check {
        what: "launches the executable from inside the bundle",
        in_workflow: "Contents/MacOS/micold-ai-ide",
        in_section_b: "Contents/MacOS/micold-ai-ide",
    },
    Check {
        what: "waits for the daemon's endpoint to appear",
        in_workflow: ".micold/run/d.sock",
        in_section_b: ".micold/run/d.sock",
    },
    Check {
        what: "terminates what it started",
        in_workflow: "kill",
        in_section_b: "Terminate",
    },
];

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Splitting on needles that contain a newline is safe on all three platforms because
/// `.gitattributes` declares `eol=lf` -- see the note there, and feature 027's T115.
fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The body of the matrix `test:` job -- the only job that runs anywhere but Linux, and so the
/// only one a macOS-restricted step can live in.
fn matrix_job(workflow: &str) -> &str {
    let after = workflow
        .split_once("\n  test:\n")
        .unwrap_or_else(|| {
            panic!("ci.yml has no `test:` job -- that job *is* the three-platform matrix")
        })
        .1;
    // A job ends where the next two-space-indented key begins.
    let mut end = after.len();
    let mut offset = 0;
    for line in after.lines() {
        if line.starts_with("  ") && !line.starts_with("   ") && line.trim_end().ends_with(':') {
            end = offset;
            break;
        }
        offset += line.len() + 1;
    }
    &after[..end]
}

/// Steps of a job that run on macOS alone.
///
/// The restriction is not incidental. `codesign`, `plutil` and `.app` bundles exist on one of the
/// three runners; an unrestricted step would fail the Linux and Windows legs of a matrix whose
/// whole point is that all three stay green.
fn macos_steps(job: &str) -> Vec<&str> {
    job.split("\n      - ")
        .filter(|step| {
            step.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("if:") && line.contains("runner.os") && line.contains("macOS")
            })
        })
        .collect()
}

/// The one macOS step that packages, out of however many macOS steps the job grows.
fn packaging_step(job: &str) -> Option<&str> {
    macos_steps(job)
        .into_iter()
        .find(|step| step.contains(CHECKS[0].in_workflow))
}

/// §B only. §A is the platform-independent suite and §C is the manual pass; neither makes a claim
/// about what a runner does.
fn section_b(quickstart: &str) -> &str {
    let after = quickstart
        .split_once("## §B")
        .unwrap_or_else(|| {
            panic!("{QUICKSTART} has no `## §B` heading -- the scan is reading the wrong document")
        })
        .1;
    match after.split_once("\n## ") {
        Some((body, _)) => body,
        None => after,
    }
}

/// Behaviours [`CHECKS`] names that the given step does not perform. The scan, isolated from the
/// repository so a broken input can be handed to it.
fn missing_from_workflow(step: &str) -> Vec<&'static str> {
    CHECKS
        .iter()
        .filter(|check| !step.contains(check.in_workflow))
        .map(|check| check.what)
        .collect()
}

/// Behaviours [`CHECKS`] names that §B does not promise.
fn missing_from_section_b(section: &str) -> Vec<&'static str> {
    CHECKS
        .iter()
        .filter(|check| !section.contains(check.in_section_b))
        .map(|check| check.what)
        .collect()
}

#[test]
fn the_macos_leg_packages_and_launches_the_bundle() {
    let workflow = read(".github/workflows/ci.yml");
    let job = matrix_job(&workflow);

    let step = packaging_step(job).unwrap_or_else(|| {
        panic!(
            "ci.yml's `test:` job has no macOS-only step running `{}`. That step *is* FR-036: \
             without it, a change that breaks the bundle is caught by the release it blocks \
             rather than by the pull request that introduced it.\n\
             {QUICKSTART} §B describes the step this gate expects.",
            CHECKS[0].in_workflow
        )
    });

    let missing = missing_from_workflow(step);
    assert!(
        missing.is_empty(),
        "ci.yml's macOS packaging step no longer does this, while {QUICKSTART} §B still promises \
         it: {missing:?}\n\
         Either restore the behaviour or stop claiming it -- a documented gate that does less \
         than it says is worse than no gate.\n\
         The step:\n{step}"
    );
}

#[test]
fn section_b_describes_what_the_step_does() {
    let quickstart = read(QUICKSTART);
    let missing = missing_from_section_b(section_b(&quickstart));

    assert!(
        missing.is_empty(),
        "ci.yml's macOS packaging step does this, and {QUICKSTART} §B does not mention it: \
         {missing:?}\n\
         §B is what a reader trusts about the automated coverage; a step nobody documented is a \
         step the next person deletes."
    );
}

/// A step that does everything, so the failures below are the scan finding a real absence rather
/// than the scan being broken.
const HEALTHY_STEP: &str = "name: Package and launch the macOS bundle
        if: runner.os == 'macOS'
        run: |
          scripts/macos-bundle.sh --bin-dir target/debug --out target/macos
          target/macos/micold-ai-ide.app/Contents/MacOS/micold-ai-ide &
          test -S \"$HOME/.micold/run/d.sock\"
          kill \"$!\"
";

#[test]
fn the_healthy_step_passes_the_scan() {
    assert_eq!(missing_from_workflow(HEALTHY_STEP), Vec::<&str>::new());
}

#[test]
fn a_step_that_never_waits_for_the_endpoint_fails() {
    let broken = HEALTHY_STEP.replace("test -S \"$HOME/.micold/run/d.sock\"\n", "");
    assert_eq!(
        missing_from_workflow(&broken),
        vec!["waits for the daemon's endpoint to appear"],
        "dropping the socket wait is the plausible regression -- it is the slow assertion, and \
         the one that proves the daemon was found beside the app"
    );
}

#[test]
fn a_step_that_leaves_the_app_running_fails() {
    let broken = HEALTHY_STEP.replace("kill", "echo done");
    assert_eq!(
        missing_from_workflow(&broken),
        vec!["terminates what it started"]
    );
}

#[test]
fn a_section_b_that_promises_nothing_fails() {
    assert_eq!(
        missing_from_section_b("## §B\n\nRuns somewhere.\n").len(),
        CHECKS.len()
    );
}

#[test]
fn the_step_is_restricted_to_macos() {
    let workflow = read(".github/workflows/ci.yml");
    let job = matrix_job(&workflow);

    let unrestricted = job
        .split("\n      - ")
        .filter(|step| step.contains(CHECKS[0].in_workflow))
        .filter(|step| {
            !step.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("if:") && line.contains("runner.os") && line.contains("macOS")
            })
        })
        .count();

    assert_eq!(
        unrestricted, 0,
        "a step running `{}` is not restricted to macOS. `codesign`, `plutil` and `.app` bundles \
         exist on one of the three runners; unrestricted, it reds the Linux and Windows legs of a \
         matrix whose whole purpose is that all three stay green.",
        CHECKS[0].in_workflow
    );
}
