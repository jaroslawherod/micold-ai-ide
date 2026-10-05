//! Attaching provider worktrees from the app (feature 582, FR-001 – FR-004, FR-016).
//!
//! The sidebar's "Attach existing…" button opens a modal list of the project's unattached provider
//! worktrees. The user ticks some and presses "Attach selected", or presses "Attach all". Nothing
//! about the listing is decided here: the daemon discovers the candidates (`AttachDiscover`) and
//! applies the choice (`AttachApply`); this feature holds only what the dialog needs to draw.
//!
//! # The vocabulary
//!
//! [`Msg`] carries the user's actions (`Opened`, `Toggled`, `AttachSelected`, `AttachAll`,
//! `Cancelled`) and the daemon's answers (`Listed`, `ListFailed`, `Applied`, `ApplyFailed`),
//! routed by [`update`], which is pure. The two requests leave the process from the shell, which
//! intercepts `Opened`, `AttachSelected` and `AttachAll`, runs this reducer, then reads
//! [`State::dialog`] (`in_flight` for the targets) to build the wire message.
//!
//! # The state
//!
//! One `Option<Dialog>` in [`State`], reached as `state.attach`: present while the dialog is open.

use crate::app::Message;
use crate::features::Outcome;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::attach::{
    AttachItem, AttachOutcome, AttachResult, AttachableWorktree, Availability, DiscoveryReport,
};
use micold_core::notify::{Level, Notification};
use micold_core::overlay::Layer;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// What this feature remembers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The open dialog, if any.
    pub dialog: Option<Dialog>,
}

/// The attach dialog's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    /// The project the listing was asked for. A reply for another project is dropped.
    pub project: PathBuf,
    /// What to draw in the list.
    pub listing: Listing,
    /// The `dir_name`s ticked.
    pub selected: BTreeSet<String>,
    /// The targets of an `AttachApply` that has been sent and not yet answered; empty when idle.
    pub in_flight: Vec<AttachItem>,
}

/// The dialog's list area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listing {
    /// Waiting for the daemon's report.
    Loading,
    /// The report: worktrees offered, attachable first.
    Listed(Vec<AttachableWorktree>),
    /// The listing could not be read; the text says why.
    Failed(String),
}

impl Dialog {
    /// Whether an apply is waiting for its answer.
    pub fn applying(&self) -> bool {
        !self.in_flight.is_empty()
    }

    /// The offered worktrees that can be attached (not missing or invalid).
    pub fn attachable(&self) -> Vec<&AttachableWorktree> {
        match &self.listing {
            Listing::Listed(rows) => rows
                .iter()
                .filter(|w| w.availability == Availability::Attachable)
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// The dialog, as a floating surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachDialog;

impl FloatingSurface for AttachDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("attach_worktrees")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::Attach(Msg::Cancelled))
    }
}

impl Registered for AttachDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.attach.dialog.as_ref().map(|_| AttachDialog)
    }
}

/// Everything the user and the daemon can say to this feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// "Attach existing…" was pressed.
    Opened,
    /// The dialog was dismissed.
    Cancelled,
    /// The daemon's report for `project`.
    Listed {
        /// The project the report is about.
        project: PathBuf,
        /// What it offers.
        report: DiscoveryReport,
    },
    /// The listing request failed.
    ListFailed(String),
    /// A row's checkbox changed.
    Toggled {
        /// The row's directory name.
        dir_name: String,
        /// The new state.
        checked: bool,
    },
    /// "Attach selected" was pressed.
    AttachSelected,
    /// "Attach all" was pressed.
    AttachAll,
    /// The daemon's answer to the apply.
    Applied(Vec<AttachResult>),
    /// The apply request failed.
    ApplyFailed(String),
}

/// Apply `msg`.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<Outcome> {
    match msg {
        Msg::Opened => {
            let Some(project) = state.workspace.active.clone() else {
                return Vec::new();
            };
            state.clear_for_dialog();
            state.attach.dialog = Some(Dialog {
                project,
                listing: Listing::Loading,
                selected: BTreeSet::new(),
                in_flight: Vec::new(),
            });
            Vec::new()
        }
        Msg::Cancelled => {
            state.attach.dialog = None;
            Vec::new()
        }
        Msg::Listed { project, report } => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                if dialog.project == project {
                    dialog.listing = Listing::Listed(report.worktrees);
                    dialog.selected.clear();
                }
            }
            Vec::new()
        }
        Msg::ListFailed(reason) => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                dialog.listing = Listing::Failed(reason);
            }
            Vec::new()
        }
        Msg::Toggled { dir_name, checked } => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                let offered = dialog.attachable().iter().any(|w| w.dir_name == dir_name);
                if offered && checked {
                    dialog.selected.insert(dir_name);
                } else {
                    dialog.selected.remove(&dir_name);
                }
            }
            Vec::new()
        }
        Msg::AttachSelected => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                let targets = worktree_targets(dialog.selected.iter().cloned());
                begin_apply(dialog, targets);
            }
            Vec::new()
        }
        Msg::AttachAll => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                let targets =
                    worktree_targets(dialog.attachable().iter().map(|w| w.dir_name.clone()));
                begin_apply(dialog, targets);
            }
            Vec::new()
        }
        Msg::Applied(results) => {
            if state.attach.dialog.take().is_none() {
                return Vec::new();
            }
            vec![Outcome::NotificationRaised(Notification::new(
                summary_level(&results),
                summary(&results),
            ))]
        }
        Msg::ApplyFailed(reason) => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                dialog.in_flight.clear();
                dialog.listing = Listing::Failed(reason);
            }
            Vec::new()
        }
    }
}

fn worktree_targets(names: impl Iterator<Item = String>) -> Vec<AttachItem> {
    names
        .map(|dir_name| AttachItem::Worktree { dir_name })
        .collect()
}

/// An apply with nothing to attach, or one already running, sends nothing.
fn begin_apply(dialog: &mut Dialog, targets: Vec<AttachItem>) {
    if dialog.applying() || targets.is_empty() {
        return;
    }
    dialog.in_flight = targets;
}

fn summary_level(results: &[AttachResult]) -> Level {
    if results
        .iter()
        .any(|r| matches!(r.outcome, AttachOutcome::Refused(_)))
    {
        Level::Error
    } else {
        Level::Info
    }
}

/// The sentence the user reads after an apply (FR-004: "already attached" is said, not silent).
pub fn summary(results: &[AttachResult]) -> String {
    let count =
        |want: fn(&AttachOutcome) -> bool| results.iter().filter(|r| want(&r.outcome)).count();
    let attached = count(|o| matches!(o, AttachOutcome::Attached));
    let already = count(|o| matches!(o, AttachOutcome::AlreadyAttached));
    let refused = count(|o| matches!(o, AttachOutcome::Refused(_)));
    let plural = |n: usize| if n == 1 { "worktree" } else { "worktrees" };
    let mut parts = Vec::new();
    if attached > 0 {
        parts.push(format!("Attached {attached} {}.", plural(attached)));
    }
    if already > 0 {
        parts.push(format!("{already} already attached."));
    }
    if refused > 0 {
        parts.push(format!("{refused} could not be attached."));
    }
    if parts.is_empty() {
        parts.push("Nothing to attach.".to_string());
    }
    parts.join(" ")
}
