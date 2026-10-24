//! What the sandbox is doing while it comes up — T043, SC-004.
//!
//! Bring-up has four stages and one of them, acquiring the image, can take minutes on a first
//! enable. SC-004 allows those minutes; what it does not allow is spending them in silence, because
//! a window that has not changed in three minutes is indistinguishable from a hung one and the
//! user's remedy for a hang is to kill the application. The container contract makes the runtime's
//! progress callbacks an obligation (C-8) for exactly this reason, and this is where they surface.
//!
//! The wording lives here; the status bar's sandbox item draws it (feature 1124, US2) — a busy
//! mark, the stage and the download's percentage, with the runtime's line in the tooltip. There is
//! no row of its own above the body: SC-003 holds the body's top edge still through a bring-up.

use micold_core::sandbox::lifecycle::SandboxState;

/// What to say about a bring-up in flight.
///
/// Separate from the rendering so the wording is testable without a renderer — the label is a claim
/// about what the application is waiting for, and it is the part that is easy to get wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageLine {
    /// The stage, in plain language.
    pub label: String,
    /// The runtime's own most recent line, when there is one.
    pub detail: Option<String>,
}

/// What the sandbox is doing, or `None` when it is not coming up.
///
/// `Running`, `Stale`, `Failed` and `Disabled` are all *standing conditions* rather than work in
/// flight; the banner speaks for those (FR-035b). This speaks only for the wait.
pub fn stage_line(state: &SandboxState) -> Option<StageLine> {
    match state {
        SandboxState::Probing => Some(StageLine {
            label: "Checking the container runtime".to_string(),
            detail: None,
        }),
        SandboxState::Acquiring(progress) => Some(StageLine {
            label: "Getting the sandbox image".to_string(),
            detail: Some(acquisition_detail(progress)),
        }),
        SandboxState::Starting => Some(StageLine {
            label: "Starting the sandbox".to_string(),
            detail: None,
        }),
        SandboxState::Disabled
        | SandboxState::Running(_)
        | SandboxState::Stale(_)
        | SandboxState::Failed(_) => None,
    }
}

/// What the sandbox is doing, including why the application is trying again, or `None` when it is
/// not coming up.
///
/// The reason goes on the detail line after whatever the runtime last said, so a pull's progress
/// and the failure that led to it are both on screen.
pub fn attempt_line(sandbox: &crate::features::sandbox::Sandbox) -> Option<StageLine> {
    let mut line = stage_line(&sandbox.state)?;
    if let Some(failed) = &sandbox.previous_attempt {
        let again = format!("Trying again: {}", failed.reason());
        line.detail = Some(match line.detail {
            Some(detail) => format!("{detail} — {again}"),
            None => again,
        });
    }
    Some(line)
}

/// The runtime's line, assembled from whatever parts it gave.
///
/// Built by appending rather than by formatting a fixed shape, because a runtime reports the parts
/// it happens to know: Docker names a layer while downloading and stops naming one while
/// extracting, and a fixed `"{stage} {percent}% — {detail}"` would leave a line trailing a dash
/// with nothing after it.
fn acquisition_detail(progress: &micold_core::sandbox::runtime::Progress) -> String {
    let mut line = progress.stage.clone();
    if let Some(percent) = progress.percent {
        line.push_str(&format!(" {percent}%"));
    }
    if let Some(item) = &progress.detail {
        line.push_str(&format!(" — {item}"));
    }
    line
}
