//! Record, in a catalogue on disk, that the application created some worktrees (feature 028, for
//! feature 029's sidebar).
//!
//! A capture fixture, not a fourth writer of provenance. Feature 029 lists a worktree under
//! `.claude/worktrees/` only when the application holds a record of having created it
//! (contracts/provenance-store.md §2), and `site/capture/demo-project.sh` makes its worktrees with
//! `git worktree add`, which leaves none — so the published screenshots showed "No worktrees yet".
//! This writes the state the creation writer would have left had the worktrees been made through
//! the application.
//!
//! It goes through [`JsonFileStore`] rather than writing JSON from the shell: the record lives in a
//! per-project file whose name the store derives by hashing the project path, and a shell copy of
//! that rule would be a second definition free to drift from the one the application reads.
//!
//!     micold-demo-provenance <projects.json> <project dir> <worktree dir name>...

use std::path::PathBuf;
use std::process::ExitCode;

use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [catalog, project, worktrees @ ..] = args.as_slice() else {
        eprintln!("usage: micold-demo-provenance <projects.json> <project dir> <worktree>...");
        return ExitCode::from(2);
    };
    let project = PathBuf::from(project);

    let store = JsonFileStore::at(PathBuf::from(catalog));
    let mut outcome = store.load();
    // Only a catalogue that parsed: saving over a missing or recovered one would write an empty
    // catalogue, and the project would not open at all.
    if outcome.status != LoadStatus::Loaded {
        eprintln!(
            "micold-demo-provenance: {catalog} did not load ({:?})",
            outcome.status
        );
        return ExitCode::FAILURE;
    }
    if !outcome.workspace.projects.iter().any(|p| p.path == project) {
        eprintln!(
            "micold-demo-provenance: {} is not in {catalog}",
            project.display()
        );
        return ExitCode::FAILURE;
    }

    outcome
        .workspace
        .worktree_provenance
        .entry(project)
        .or_default()
        .extend(worktrees.iter().cloned());
    match store.save(&outcome.workspace) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("micold-demo-provenance: saving {catalog}: {err}");
            ExitCode::FAILURE
        }
    }
}
