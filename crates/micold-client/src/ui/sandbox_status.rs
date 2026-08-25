//! What the sandbox is doing while it comes up — T043, SC-004.
//!
//! Bring-up has four stages and one of them, acquiring the image, can take minutes on a first
//! enable. SC-004 allows those minutes; what it does not allow is spending them in silence, because
//! a window that has not changed in three minutes is indistinguishable from a hung one and the
//! user's remedy for a hang is to kill the application. The container contract makes the runtime's
//! progress callbacks an obligation (C-8) for exactly this reason, and this is where they surface.
//!
//! # Why the bar claims nothing
//!
//! [`StageProgress`](crate::ui::material::StageProgress) is indeterminate by construction — see its
//! own header. A pull *does* report a percentage, and it is shown, but as text on the detail line
//! rather than as a fill: the percentage a runtime reports is per-layer and restarts, so a bar
//! driven from it would run forwards and then jump back, which reads as a fault rather than as
//! progress.
//!
//! # Why it is absent rather than empty at rest
//!
//! The bar animates for as long as it exists, so one rendered while nothing is happening would
//! request a frame every tick for the life of the process (SC-017). The states that are not a
//! bring-up in flight therefore render *nothing*, and `tests/sandbox_progress.rs` lays the view out
//! to check it.

use iced::widget::Space;
use iced::{Element, Length};
use micold_core::sandbox::lifecycle::SandboxState;
use micold_core::tokens::{spacing, Roles};

use crate::app::Message;
use crate::ui::material;

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

/// The indicator itself, or nothing at all.
pub fn view<'a>(state: &SandboxState, roles: Roles) -> Element<'a, Message> {
    match stage_line(state) {
        Some(line) => material::Surface::new(
            material::StageProgress::new(line.label, roles).detail(line.detail),
            material::SurfaceKind::Window,
            roles,
        )
        .padding(spacing::MD)
        .width(Length::Fill)
        .into(),
        None => Space::new()
            .width(Length::Fixed(0.0))
            .height(Length::Fixed(0.0))
            .into(),
    }
}
