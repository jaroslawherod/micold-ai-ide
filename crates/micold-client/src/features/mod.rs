//! Feature modules (feature 021, Tier 1).
//!
//! One module per feature, each holding that feature's types **together with** the helper
//! functions that operate on them. A feature is never split across parallel state / update / view
//! files — a type and the functions over it stay in one place (FR-001a).
//!
//! These modules are render-free: they must not name the rendering framework in code, which
//! `tests/features_are_render_free.rs` holds. That property is what lets the application's state
//! live in this crate rather than the core, so it is checked rather than assumed.
//!
//! Views live in `crate::ui`, beside — not inside — the feature they draw. Reducer modules arrive
//! in Tier 3; until then a feature module holds data and operations only, with no message
//! vocabulary of its own.
//!
//! Modules are declared here as each is extracted from `crate::app`, one migration step at a time.

use crate::overlay::SurfaceId;
use micold_core::session::SessionId;

/// A consequence a feature returns rather than applies (FR-021).
///
/// **One variant so far, and it is here in Phase 5 rather than at T065 because FR-015a needs it.**
/// Clipboard access cannot be a service capability: all of its call sites return an `iced::Task`
/// rather than a value, so a synchronous port cannot wrap them without blocking. FR-015a's
/// alternative for exactly that case is an explicit effect request in this vocabulary, *interpreted
/// by the shell*.
///
/// That is also why introducing it here is not the Tier 3 work SC-004b's checkpoint forbids.
/// FR-021 sits under "cross-feature coordination" because its subject there is a feature reducer
/// returning a consequence for the **root** to interpret (FR-022) and route to another feature.
/// None of that arrives with this variant: there is no root interpreter, no reducer split, and no
/// feature learns anything about another. T065 adds `SessionsClosed`, `OverlayDismissed` and
/// `NotificationRaised` along with the root's draining interpreter — it extends this enum rather
/// than introducing it, which is the one edit its text needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Put `text` on the system clipboard.
    ///
    /// The shell translates this to `iced::clipboard::write` and decides nothing on the way
    /// (contract C3) — whether anything should be written at all was settled by the feature that
    /// emitted the request.
    ClipboardWrite(String),
    /// These sessions are gone; the session feature should drop them (FR-021, contract O2).
    ///
    /// Emitted by the worktree delete path, which owns *worktrees* and must not reach into session
    /// state to close what lived in one — the anti-pattern the contract names by name. T066 is the
    /// conversion; this variant is the vocabulary it needs to exist first.
    SessionsClosed(Vec<SessionId>),
    /// This surface should close, whoever owns it.
    ///
    /// Also the worktree delete path: confirming a delete dismisses its own dialog, and the
    /// registry — not the deleting feature — is what knows how a surface closes.
    OverlayDismissed(SurfaceId),
    /// Raise a notification, from any feature.
    ///
    /// The one outcome the contract lists as "emitted by: any feature", and the reason is that a
    /// notification is nobody's feature: every path that can fail wants one, and `notify` belongs
    /// to none of them. Carries the queue's own `Notification` rather than the banner's
    /// `NoticeLevel`, so the translation stays where `State::push_notification` already put it.
    NotificationRaised(micold_core::notify::Notification),
    /// Discovery replaced the worktree list; these `dir_name`s are what survived (T066).
    ///
    /// **The first outcome anything actually emits**, and the one that gave `app::drain` its first
    /// caller. The worktree feature owns the list and prunes its own state when it changes; the
    /// sidebar's expansion set is not its to prune, so it reports what survived and the root routes
    /// that to `sidebar::worktrees_replaced`.
    WorktreesReplaced(std::collections::BTreeSet<String>),
    /// The user asked to review this entry's changes (feature 482, V1).
    ///
    /// The sidebar owns the row menu that asks; the view is `changes`'s, and the menu is
    /// `worktree`'s, so the root opens the one and closes the other.
    ChangesRequested(micold_core::session::SessionLocation),
    /// Compare opened (feature 483, C1): it takes the Changes view's place, so the root closes
    /// that view. `runs` owns Compare and `changes` owns the view.
    ChangesClosedForCompare,
    /// **Run in parallel** was chosen (feature 483): the root opens the dialog.
    RunInParallelRequested,
    /// The shell created this worktree; the list it joins is not the form's to write (T067a-4).
    ///
    /// `worktree_form` is a separate feature because its lifecycle is independent (FR-003), but
    /// what it creates lands in `worktree`'s list. The form reports the creation and closes; the
    /// worktree feature performs the insert, keeping its own two identities distinct — a create
    /// names the directory, a daemon include answers with a path.
    WorktreeCreated(micold_core::worktree::Worktree),
    /// This location belongs in the user's own open set (feature 024; T067a-6).
    ///
    /// **Not "reveal this row".** The revealed row is *derived* by `State::location_open` as the
    /// union of the user's set with the one current session's location, and is never written. What
    /// is written is the moment a reveal stops being one: when the current session changes, the row
    /// it opened would silently close, so the outgoing location is folded into the user's set
    /// first. `session::started` emits the same fact a moment earlier for the session it creates.
    LocationOpened(micold_core::session::SessionLocation),
    /// A view this feature just opened takes the main area: whatever covers it (the worktree
    /// Overview, feature 813 FR-023) gives way. `session::started` emits it for the session it
    /// creates and shows.
    MainAreaTaken,
    /// A selection ends the Overview's reveal mark on the sidebar row (feature 813, R12).
    RevealMarkCleared,
    /// An Overview card sent the user to this row: mark it, and open it (feature 813, R12).
    LocationRevealed(micold_core::session::SessionLocation),
    /// The sidebar should scroll to the current session's row once one exists (research R7, I4).
    RevealScrollArmed,
    /// The current session changed, so the sidebar drops a navigation stop and the row a
    /// navigation chord gave the keyboard (feature 803, contracts/ui.md U5).
    KeyboardRowLeft,
    /// The user arrived in a project, so view state switched on for the previous one resets.
    ///
    /// `default_expanded` is not keyed per project (unlike `expanded`, pruned by `dir_name`), so it
    /// would otherwise render in a project that never asked for it.
    ProjectEntered,
    /// The user closed (or reopened) the row holding the current session (feature 024, I2).
    ///
    /// The sidebar owns the row and knows the twisty was clicked; whether that suppresses the
    /// reveal is a fact about the *session*, which is why it travels rather than being written.
    RevealSuppressed(bool),
    /// This surface opened, so whatever it displaces should close (T067a-2).
    ///
    /// The feature reports that *its* surface is now open and stops there. Which other surfaces
    /// that closes is the relation between surfaces, which no single feature owns: the three panel
    /// popovers are mutually exclusive, a context menu displaces the other one and two of the
    /// three panels, and the project row menu deliberately leaves the switcher it was opened from
    /// alone. `FloatingSurface::displaces` declares it and `overlay::registry::displace` applies
    /// it, so a toggle no longer assigns its neighbours' fields — the twelve cross-feature writes
    /// that were the last of the T067 catalogue.
    SurfaceOpened(crate::overlay::SurfaceId),
    /// A terminal took the keyboard, so no text field holds it any more (FR-018; T067a-9).
    ///
    /// Unconditional, unlike `window::field_focus_changed`'s guarded blur: a press on the pane is
    /// a request for *that* terminal and must not be defeated by whichever field still believes it
    /// has focus. The session feature knows the terminal was asked for; the window feature owns
    /// what having focus means.
    FieldFocusCleared,
    /// Hand this address to the operating system's opener (feature 031, FR-010).
    ///
    /// An effect request like [`Outcome::ClipboardWrite`], and performed the same way: the shell's
    /// `shell/links.rs` takes it out of the queue before the root drains the rest, because opening
    /// is I/O on a blocking task and its result comes back as `LinkOpenFinished`.
    OpenLink(OpenRequest),
    /// Type `text` at this terminal's input, without submitting it (feature 487, FR-003).
    ///
    /// An effect request like [`Outcome::OpenLink`]: the bytes go to the daemon addressed to
    /// `terminal`, whichever pane has focus (FR-005), and whether the terminal asked for bracketed
    /// paste is the shell's fact, so the shell turns the text into bytes.
    Insert {
        /// The terminal that receives the text.
        terminal: micold_core::protocol::messages::TerminalRef,
        /// The quoted paths, joined by single spaces. No line break the user did not put in a name.
        text: String,
    },
    /// The daemon registry changed (feature 491, FR-011): the shell persists it
    /// (`Settings::set_registry`) and brings the links in step (`sync_links`).
    DaemonRegistryChanged(micold_core::daemons::DaemonRegistry),
    /// The Kubernetes form opened: list the kubeconfig's contexts off the UI thread and answer
    /// with `daemons::Msg::ContextsListed` (feature 686, FR-002). The shell's.
    KubeContextsRequested,
    /// Stop this Kubernetes daemon's pod (feature 686, FR-014): the shell runs
    /// `core::kube::lifecycle::stop` over its cluster. Sessions already ended and the link stopped.
    KubeStopRequested {
        /// The daemon.
        daemon: micold_core::daemons::DaemonId,
        /// Its settings at the time of the request.
        settings: micold_core::kube::settings::KubeSettings,
        /// Its name.
        name: String,
    },
    /// Stop the start of this Kubernetes daemon that is under way (feature 686, FR-012): the shell
    /// cancels it, and it ends `start cancelled`.
    KubeStartCancelRequested {
        /// The daemon.
        daemon: micold_core::daemons::DaemonId,
        /// Its name, which the start is registered under.
        name: String,
    },
    /// Delete every cluster object of this Kubernetes daemon (FR-015); the shell answers with
    /// `daemons::Msg::KubeRemoved`. The entry stays until it does.
    KubeRemoveRequested {
        /// The daemon.
        daemon: micold_core::daemons::DaemonId,
        /// Its settings at the time of the request.
        settings: micold_core::kube::settings::KubeSettings,
        /// Its name.
        name: String,
    },
    /// Delete this client's key files and pinned host key for a Kubernetes daemon that was removed
    /// or forgotten locally (FR-015, FR-024).
    KubeKeysDelete {
        /// The daemon forgotten.
        daemon: micold_core::daemons::DaemonId,
    },
    /// Fetch the last `tail` lines of the pod's log; the shell answers with
    /// `daemons::Msg::LogsFetched` (FR-018).
    KubeLogsRequested {
        /// The daemon.
        daemon: micold_core::daemons::DaemonId,
        /// Its settings.
        settings: micold_core::kube::settings::KubeSettings,
        /// How many lines from the end.
        tail: u32,
    },
    /// These sessions ended because their daemon was removed or its runtime settings changed:
    /// the shell stops them on that daemon and shows them as ended (FR-012, FR-014).
    DaemonSessionsEnded {
        /// The daemon they ran on.
        daemon: micold_core::daemons::DaemonId,
        /// The sessions that ended.
        sessions: Vec<SessionId>,
    },
    /// A worktree was bound to a daemon by an explicit action (FR-006, FR-014): the shell persists
    /// the binding and registers the project on that daemon.
    DaemonBound {
        /// The worktree's project.
        project: std::path::PathBuf,
        /// The worktree's `dir_name`, `""` for Default.
        worktree: String,
        /// The daemon it now runs on.
        daemon: micold_core::daemons::DaemonId,
    },
    /// The user chose which issue source the add-worktree dialog lists in `project` (feature 533,
    /// FR-016): the root keeps it in memory for the next open and the shell writes it to the
    /// project's state file (`ProjectStore::save_issue_source`).
    IssueSourceChosen {
        /// The project the dialog is open in.
        project: std::path::PathBuf,
        /// The source chosen.
        provider: micold_core::jira::IssueProvider,
    },
    /// A repository on a remote daemon's host was named (feature 489, FR-012): the shell asks that
    /// daemon whether it is one and opens it as a project when it is.
    RemoteRepositoryRequested {
        /// The repository's identity (`micold-remote://<daemon><path>`).
        path: std::path::PathBuf,
    },
    /// A mismatched remote daemon that proved it has no running session is to be ended so this
    /// build's can take its place (feature 489, FR-011): the shell asks its host to stop it and
    /// reports `Replaced`.
    RemoteReplaceRequested {
        /// The daemon.
        daemon: micold_core::daemons::DaemonId,
        /// Its host.
        target: micold_core::ssh::SshTarget,
    },
}

/// What an activated link asks the operating system to open (feature 031, data-model §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenRequest {
    /// A web or mail address, handed over verbatim.
    Url(String),
    /// A file or folder on this machine, with the address the program printed for it.
    Path { path: String, address: String },
}

/// Why an open did not happen (feature 031, contract link-opening §2, §5).
///
/// Here rather than in the shell so a render-free feature can name it in `LinkOpenFinished`;
/// `shell/link_opener.rs` re-exports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenFailure {
    /// Nothing on this machine is set up to open the address.
    NoApplication,
    /// The opener could not be started, or reported another failure.
    LaunchFailed(String),
    /// The file does not exist. Set by `shell/links.rs`, never by an opener.
    NotFound,
}

/// `Outcome::SurfaceOpened` when a toggle left its surface open, and nothing when it closed it.
///
/// Five reducers spell the same two lines rather than each writing them.
///
/// **The check is not what makes the behaviour correct**, and a probe was what established that:
/// reporting an opening unconditionally changes nothing observable, because
/// `overlay::registry::displace` resolves the displacing surface out of the set that is *open* and
/// a closed one is silently skipped. What this guards is the vocabulary, not the state — an
/// `Outcome::SurfaceOpened` for a surface that did not open is a false statement about what
/// happened, and outcomes are the one place in this design where a feature says what happened.
/// `tests/popover_displacement.rs::a_toggle_that_shut_its_surface_reports_nothing` is what holds
/// it, and it has to read the return value, since no state can tell the two apart.
pub(crate) fn surface_opened(open: bool, id: crate::overlay::SurfaceId) -> Vec<Outcome> {
    if open {
        vec![Outcome::SurfaceOpened(id)]
    } else {
        Vec::new()
    }
}

pub mod agent_confirm;
pub mod attach;
pub mod attention;
pub mod changes;
pub mod connection;
pub mod daemons;
pub mod daemons_kube;
pub mod help;
pub mod kube_clone;
pub mod navigation;
pub mod notifications;
pub mod overview;
pub mod plan_usage;
pub mod pr_status;
pub mod project;
pub mod quick_switch;
pub mod runs;
pub mod sandbox;
pub mod session;
pub mod settings;
pub mod sidebar;
pub mod window;
pub mod worktree;
pub mod worktree_form;
