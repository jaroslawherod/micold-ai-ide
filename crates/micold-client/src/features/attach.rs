//! Attaching provider worktrees from the app (feature 582, FR-001 – FR-004, FR-016).
//!
//! The sidebar's "Attach existing…" button opens a modal list of the project's stored provider
//! sessions the catalog lacks. The user resumes one, or presses "Resume all". Nothing
//! about the listing is decided here: the daemon discovers the candidates (`AttachDiscover`) and
//! applies the choice (`AttachApply`); this feature holds only what the dialog needs to draw.
//!
//! # The vocabulary
//!
//! [`Msg`] carries the user's actions (`Opened`, `Resume`, `ResumeAll`, `Cancelled`) and the daemon's answers (`Listed`, `ListFailed`, `Applied`, `ApplyFailed`),
//! routed by [`update`], which is pure. The two requests leave the process from the shell, which
//! intercepts `Opened`, `Resume` and `ResumeAll`, runs this reducer, then reads
//! [`State::dialog`] (`in_flight` for the targets) to build the wire message.
//!
//! # The state
//!
//! One `Option<Dialog>` in [`State`], reached as `state.attach`: present while the dialog is open.

use crate::app::Message;
use crate::features::Outcome;
use crate::floating_surface;
use crate::overlay::registry::Registered;
use crate::overlay::SurfaceId;
use micold_core::attach::{
    cap_per_location, AttachItem, AttachOutcome, AttachResult, AttachTarget, DiscoveryNote,
    DiscoveryReport, RefuseReason, ResumableSession, ResumableStatus, SkipReason,
    UnresumableReason,
};
use micold_core::notify::{Level, Notification};
use micold_core::occupancy::{occupancy, ExistingSession, Occupancy};
use micold_core::session::{Session, SessionLocation};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// What this feature remembers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The open dialog, if any.
    pub dialog: Option<Dialog>,
    /// The start-up offer (Story 4).
    pub offer: Offer,
}

/// The start-up offer: what discovery found when a project opened, and what the user did with it.
/// In memory only; a dismissal lasts until the app restarts (research R5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Offer {
    /// The latest report for one project, as asked at project open.
    pub report: Option<(PathBuf, DiscoveryReport)>,
    /// Projects whose offer was dismissed this run.
    pub dismissed: BTreeSet<PathBuf>,
    /// The targets of an `AttachApply` sent from the banner and not yet answered.
    pub in_flight: Vec<AttachItem>,
}

impl Offer {
    /// The report's stored sessions that "Resume all" can catalogue, as apply targets: the newest
    /// resumable session of each worktree that has none (feature 776, FR-012; BUG-1164).
    pub fn targets(&self, catalog: &[Session]) -> Vec<AttachItem> {
        let resumable: Vec<&ResumableSession> = self
            .offerable(catalog)
            .into_iter()
            .filter(|s| can_resume(s))
            .collect();
        cap_per_location(resumable, &BTreeSet::new(), |s| {
            (s.target.clone(), s.last_activity)
        })
        .into_iter()
        .map(|s| AttachItem::Session {
            id: s.id,
            replace: false,
            allow_additional_session: false,
        })
        .collect()
    }

    /// The report's stored sessions the offer shows: those whose worktree has no session in
    /// `catalog`. A session of an occupied worktree is resumed from "Attach existing…", which
    /// asks replace or keep both (776 FR-008b/c); "Resume all" could only be refused for it.
    pub fn offerable(&self, catalog: &[Session]) -> Vec<&ResumableSession> {
        self.report
            .iter()
            .flat_map(|(_, r)| r.sessions.iter())
            .filter(|s| {
                let location = match &s.target {
                    AttachTarget::Default => SessionLocation::Default,
                    AttachTarget::Worktree { dir_name } => {
                        SessionLocation::Worktree(dir_name.clone())
                    }
                };
                matches!(occupancy(catalog, &location), Occupancy::Free)
            })
            .collect()
    }

    /// How many stored sessions the offer shows.
    pub fn session_count(&self, catalog: &[Session]) -> usize {
        self.offerable(catalog).len()
    }
}

/// Whether the banner is shown for the active project: discovery found stored provider sessions
/// the catalog lacks, and the offer was not dismissed this run (FR-009). A project whose records
/// could not be read is never offered.
pub fn offer_visible(state: &crate::app::State) -> bool {
    let Some(project) = state.workspace.active.as_ref() else {
        return false;
    };
    let Some((asked, report)) = state.attach.offer.report.as_ref() else {
        return false;
    };
    asked == project
        && !report.is_empty()
        && state.attach.offer.session_count(state.active_sessions()) > 0
        && !state.attach.offer.dismissed.contains(project)
        && !state.workspace.unreadable_projects.contains(project)
}

/// The attach dialog's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    /// The project the listing was asked for. A reply for another project is dropped.
    pub project: PathBuf,
    /// Whether the listing has arrived.
    pub listing: Listing,
    /// The targets of an `AttachApply` that has been sent and not yet answered; empty when idle.
    pub in_flight: Vec<AttachItem>,
    /// Why the last apply failed; the listing stays so the user can retry.
    pub error: Option<String>,
    /// Stored provider sessions the project's catalog does not hold, newest first (Story 2).
    pub sessions: Vec<ResumableSession>,
    /// What discovery skipped, and why.
    pub notes: Vec<DiscoveryNote>,
    /// The question asked when a resume met a worktree that already has a session (feature 776,
    /// FR-008b, FR-008c); `None` when none is open.
    pub confirm: Option<Confirm>,
}

/// A resume the daemon refused because its worktree has a session: replace that session, keep
/// both, or leave things as they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirm {
    /// The stored session the user chose to resume.
    pub id: uuid::Uuid,
    /// What the dialog called it.
    pub title: String,
    /// The session the worktree has, as the daemon named it.
    pub existing: ExistingSession,
}

impl Confirm {
    /// The question, naming both sessions.
    pub fn text(&self) -> String {
        format!(
            "This worktree already has the session \"{}\". Replace it with \"{}\" (the old \
             session is stopped and removed), or keep both?",
            self.existing.name, self.title
        )
    }
}

/// The dialog's list area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listing {
    /// Waiting for the daemon's report.
    Loading,
    /// The report arrived; the sessions and notes are on the dialog.
    Listed,
    /// The listing could not be read; the text says why.
    Failed(String),
}

impl Dialog {
    /// Whether an apply is waiting for its answer.
    pub fn applying(&self) -> bool {
        !self.in_flight.is_empty()
    }

    /// The notes worth showing. An absent store is the ordinary case, so it is said only when it
    /// explains an empty list; an unreadable store, a corrupt entry or a sandbox is always said.
    pub fn footer_notes(&self) -> Vec<String> {
        let nothing = self.sessions.is_empty();
        self.notes
            .iter()
            .filter(|n| nothing || n.reason != SkipReason::StoreMissing)
            .map(note_text)
            .collect()
    }

    /// The stored sessions that can be resumed from the dialog.
    pub fn resumable(&self) -> Vec<&ResumableSession> {
        self.sessions.iter().filter(|s| can_resume(s)).collect()
    }
}

/// One discovery note as a sentence.
pub fn note_text(note: &DiscoveryNote) -> String {
    let who = note
        .provider
        .map_or("A provider", |p| p.provider().display_name());
    match note.reason {
        SkipReason::StoreMissing => format!("{who} has no stored sessions here."),
        SkipReason::StoreUnreadable => format!("{who}'s session store could not be read."),
        SkipReason::EntryCorrupt => format!("{who} has a session entry that could not be read."),
        SkipReason::SandboxStoreNotReadable => {
            "This project runs in a sandbox, so the sessions stored on this computer cannot be \
             listed."
                .to_string()
        }
    }
}

/// Whether a listed session can be resumed from the dialog.
pub fn can_resume(session: &ResumableSession) -> bool {
    !matches!(session.status, ResumableStatus::Unresumable(_))
}

/// The line under a session row: what resuming it does, or why it cannot be resumed.
pub fn status_text(session: &ResumableSession) -> String {
    match session.status {
        ResumableStatus::Resumable => "Ready to resume".into(),
        ResumableStatus::Unresumable(UnresumableReason::WorktreeMissing) => {
            "Cannot resume: its worktree folder is missing".into()
        }
        ResumableStatus::Unresumable(UnresumableReason::WorktreeInvalid) => {
            "Cannot resume: its folder is not a usable worktree".into()
        }
        ResumableStatus::Unresumable(UnresumableReason::NoLocation) => {
            "Cannot resume: no worktree of this project matches where it ran".into()
        }
    }
}

/// The dialog, as a floating surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachDialog;

floating_surface!(AttachDialog, id: SurfaceId::new("attach_worktrees"), layer: Dialog, cancelled_by: Message::Attach(Msg::Cancelled));

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
    /// "Resume all" was pressed.
    ResumeAll,
    /// "Resume" was pressed on a stored session.
    Resume {
        /// The session's id.
        id: uuid::Uuid,
    },
    /// "Replace" was pressed on the confirmation: resume again with `replace` (FR-008b).
    ConfirmReplace,
    /// "Keep both" was pressed on the confirmation: resume again beside the session (FR-008c).
    ConfirmKeepBoth,
    /// The discovery report asked for at project open.
    OfferListed {
        /// The project the report is about.
        project: PathBuf,
        /// What it offers.
        report: DiscoveryReport,
    },
    /// The banner's dismiss button.
    OfferDismissed,
    /// The banner's "Attach all" button.
    OfferAttachAll,
    /// The daemon's answer to the banner's apply.
    OfferApplied(Vec<AttachResult>),
    /// The banner's apply request failed.
    OfferApplyFailed(String),
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
                in_flight: Vec::new(),
                error: None,
                sessions: Vec::new(),
                notes: Vec::new(),
                confirm: None,
            });
            Vec::new()
        }
        Msg::Cancelled => {
            // The confirmation is answered "no": back to the list, nothing sent.
            if let Some(dialog) = state.attach.dialog.as_mut().filter(|d| d.confirm.is_some()) {
                dialog.confirm = None;
                return Vec::new();
            }
            // While an apply is outstanding the dialog stays, so its answer has somewhere to land
            // and a reopened dialog can never receive a stale one.
            if state.attach.dialog.as_ref().is_some_and(Dialog::applying) {
                return Vec::new();
            }
            state.attach.dialog = None;
            Vec::new()
        }
        Msg::Listed { project, report } => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                if dialog.project == project && dialog.listing == Listing::Loading {
                    dialog.listing = Listing::Listed;
                    dialog.sessions = report.sessions;
                    dialog.notes = report.notes;
                }
            }
            Vec::new()
        }
        Msg::ListFailed(reason) => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                if dialog.listing == Listing::Loading {
                    dialog.listing = Listing::Failed(reason);
                }
            }
            Vec::new()
        }
        Msg::ResumeAll => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                let targets = dialog
                    .resumable()
                    .iter()
                    .map(|s| AttachItem::Session {
                        id: s.id,
                        replace: false,
                        allow_additional_session: false,
                    })
                    .collect();
                begin_apply(dialog, targets);
            }
            Vec::new()
        }
        Msg::Resume { id } => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                if dialog.sessions.iter().any(|s| s.id == id && can_resume(s)) {
                    begin_apply(
                        dialog,
                        vec![AttachItem::Session {
                            id,
                            replace: false,
                            allow_additional_session: false,
                        }],
                    );
                }
            }
            Vec::new()
        }
        Msg::ConfirmReplace | Msg::ConfirmKeepBoth => {
            if let Some(dialog) = state.attach.dialog.as_mut() {
                if let Some(confirm) = dialog.confirm.take() {
                    let replace = msg == Msg::ConfirmReplace;
                    begin_apply(
                        dialog,
                        vec![AttachItem::Session {
                            id: confirm.id,
                            replace,
                            allow_additional_session: !replace,
                        }],
                    );
                }
            }
            Vec::new()
        }
        Msg::OfferListed { project, report } => {
            if state.workspace.active.as_ref() == Some(&project) {
                state.attach.offer.report = Some((project, report));
            }
            Vec::new()
        }
        Msg::OfferDismissed => {
            if let Some(project) = state.workspace.active.clone() {
                state.attach.offer.dismissed.insert(project);
            }
            Vec::new()
        }
        Msg::OfferAttachAll => {
            if offer_visible(state) && state.attach.offer.in_flight.is_empty() {
                let targets = state.attach.offer.targets(state.active_sessions());
                state.attach.offer.in_flight = targets;
            }
            Vec::new()
        }
        Msg::OfferApplied(results) => {
            if state.attach.offer.in_flight.is_empty() {
                return Vec::new();
            }
            state.attach.offer.in_flight.clear();
            // The banner ends only when something was attached; a batch that all failed keeps it
            // so the user can retry.
            if results.iter().any(|r| {
                matches!(
                    r.outcome,
                    AttachOutcome::Attached | AttachOutcome::AlreadyAttached
                )
            }) {
                state.attach.offer.report = None;
            }
            vec![Outcome::NotificationRaised(Notification::new(
                summary_level(&results),
                summary(&results),
            ))]
        }
        Msg::OfferApplyFailed(reason) => {
            if state.attach.offer.in_flight.is_empty() {
                return Vec::new();
            }
            state.attach.offer.in_flight.clear();
            vec![Outcome::NotificationRaised(Notification::new(
                Level::Error,
                format!("Couldn't resume the found sessions: {reason}"),
            ))]
        }
        Msg::Applied(results) => {
            if !state.attach.dialog.as_ref().is_some_and(Dialog::applying) {
                return Vec::new();
            }
            if let Some(dialog) = state.attach.dialog.as_mut() {
                if let Some(confirm) = confirmation_for(dialog, &results) {
                    dialog.in_flight.clear();
                    dialog.confirm = Some(confirm);
                    return Vec::new();
                }
            }
            state.attach.dialog = None;
            vec![Outcome::NotificationRaised(Notification::new(
                summary_level(&results),
                summary(&results),
            ))]
        }
        Msg::ApplyFailed(reason) => {
            if let Some(dialog) = state.attach.dialog.as_mut().filter(|d| d.applying()) {
                dialog.in_flight.clear();
                dialog.error = Some(reason);
            }
            Vec::new()
        }
    }
}

/// The question to ask when the one session the user chose to resume was refused because its
/// worktree has a session. A batch is never asked about: its refusals are in the summary.
fn confirmation_for(dialog: &Dialog, results: &[AttachResult]) -> Option<Confirm> {
    let [AttachResult {
        item: AttachItem::Session { id, .. },
        outcome: AttachOutcome::Refused(RefuseReason::LocationHasSession),
        existing: Some(existing),
    }] = results
    else {
        return None;
    };
    (dialog.in_flight.len() == 1).then(|| Confirm {
        id: *id,
        title: dialog
            .sessions
            .iter()
            .find(|s| s.id == *id)
            .and_then(|s| s.title.clone())
            .unwrap_or_else(|| "Untitled session".into()),
        existing: existing.clone(),
    })
}

/// An apply with nothing to attach, or one already running, sends nothing.
fn begin_apply(dialog: &mut Dialog, targets: Vec<AttachItem>) {
    if dialog.applying() || targets.is_empty() {
        return;
    }
    dialog.error = None;
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
    let noun = ("session", "sessions");
    let plural = |n: usize| if n == 1 { noun.0 } else { noun.1 };
    let mut parts = Vec::new();
    if attached > 0 {
        parts.push(format!("Attached {attached} {}.", plural(attached)));
    }
    if already > 0 {
        parts.push(format!("{already} already attached."));
    }
    if refused > 0 {
        // A session refusal says why: "already running" is the one the user can act on (FR-016).
        let why: Vec<&str> = results
            .iter()
            .filter_map(|r| match &r.outcome {
                AttachOutcome::Refused(reason) => Some(reason.text()),
                _ => None,
            })
            .collect();
        if why.is_empty() {
            parts.push(format!("{refused} could not be attached."));
        } else {
            parts.push(format!(
                "{refused} could not be attached ({}).",
                why.join("; ")
            ));
        }
    }
    if parts.is_empty() {
        parts.push("Nothing to attach.".to_string());
    }
    parts.join(" ")
}
