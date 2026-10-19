//! The client's half of scoped git credentials (#1002, contracts/pat-source.md, protocol.md): the
//! pasted-token source [`PatSource`], its HTTP [`UreqTransport`], and the per-daemon [`Manager`]
//! with the worker thread that drives it.
//!
//! The [`Manager`] is plain data and logic. Everything that reaches the network goes through the
//! source's [`Transport`], everything that reaches a daemon through a [`Sink`], and the time is an
//! argument, so its tests run with a clock they own and no network. The token is read from the
//! store only when a check needs it and never reaches a log line.
//!
//! [`GitCredentials`] is the handle the `App` holds. Its worker thread runs the manager's commands
//! in the order they were posted and sleeps until the earliest check falls due, so the blocking
//! HTTP calls never touch the UI thread and a disconnect followed by a reconnect cannot be
//! reordered.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use iced::futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use iced::futures::stream::{self, BoxStream, StreamExt};

use micold_core::daemons::DaemonId;
use micold_core::github::GithubRepo;
use micold_core::protocol::messages::ClientMsg;
use micold_core::scoped_credential::pat::{self, Reply, Transport};
use micold_core::scoped_credential::pat_store::{PatStore, StoredPat};
use micold_core::scoped_credential::schedule::{RenewPolicy, Schedule};
use micold_core::scoped_credential::{
    CredentialSource, CredentialStatus, CredentialTarget, Grant, NoCredentialReason,
    ScopedCredential, SecretToken, UnixTime, EXPIRY_WARNING_SECS,
};

/// The whole request's budget: connect, send and read.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The clock the source checks against.
pub type Clock = Arc<dyn Fn() -> UnixTime + Send + Sync>;

/// GitHub's REST API over HTTPS (`ureq` 3), as `jira_http` does Jira: 10 second budget, no redirect,
/// a non-2xx status is an `Ok` reply (contract §2).
#[derive(Debug, Clone)]
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl Default for UreqTransport {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

impl Transport for UreqTransport {
    fn get(&self, url: &str, token: &SecretToken) -> Result<Reply, String> {
        let failure = |e: ureq::Error| match e {
            ureq::Error::Timeout(_) => "timed out".to_string(),
            other => pat::short_error(&other.to_string(), token),
        };
        let mut response = self
            .agent
            .get(url)
            .header("Authorization", format!("Bearer {}", token.expose()))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "micold-ai-ide")
            .call()
            .map_err(failure)?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                Some((name.as_str().to_string(), value.to_str().ok()?.to_string()))
            })
            .collect();
        let body = response.body_mut().read_to_string().map_err(failure)?;
        Ok(Reply {
            status,
            headers,
            body,
        })
    }
}

/// The pasted tokens as a [`CredentialSource`] (contract §1): `issue` is the stored token of the
/// repository, checked, with the expiry GitHub reports written back; `revoke` does nothing, never a
/// request, because the token is the user's.
pub struct PatSource {
    store: Option<PatStore>,
    transport: Arc<dyn Transport>,
    now: Clock,
}

impl PatSource {
    /// The source over `store` (none: every repository reads *not set up*).
    pub fn new(store: Option<PatStore>, transport: Arc<dyn Transport>, now: Clock) -> Self {
        Self {
            store,
            transport,
            now,
        }
    }

    /// The device's store, GitHub over HTTPS and the system clock.
    pub fn on_device() -> Self {
        Self::new(
            PatStore::on_device(),
            Arc::new(UreqTransport::default()),
            Arc::new(micold_core::unix_time::unix_secs),
        )
    }
}

impl CredentialSource for PatSource {
    fn issue(
        &self,
        repo: &GithubRepo,
        _grant: Grant,
    ) -> Result<ScopedCredential, NoCredentialReason> {
        let store = self.store.as_ref();
        let stored = store
            .and_then(|s| s.get(repo))
            .ok_or(NoCredentialReason::SourceNotSetUp)?;
        let now = (self.now)();
        if stored.expires_at <= now {
            return Err(NoCredentialReason::PatExpired);
        }
        let expires_at = pat::check(&*self.transport, repo, &stored.token, now)?;
        let credential = ScopedCredential::pasted(repo, stored.token.clone(), expires_at, now)?;
        if expires_at != stored.expires_at {
            let refreshed = StoredPat {
                token: stored.token,
                expires_at,
            };
            if let Some(store) = store {
                let _ = store.put(repo, &refreshed);
            }
        }
        Ok(credential)
    }

    fn revoke(&self, _credential: &ScopedCredential) -> Result<(), NoCredentialReason> {
        Ok(())
    }
}

/// Check a pasted token for `repo` and, only when GitHub accepts it, store it (Settings, Add or
/// Replace). Returns the expiry, or the reason for the form; nothing is stored on a refusal.
pub fn check_and_store(
    transport: &dyn Transport,
    store: Option<&PatStore>,
    repo: &GithubRepo,
    token: SecretToken,
    now: UnixTime,
) -> Result<UnixTime, String> {
    let store = store.ok_or("No data directory to keep the token in.")?;
    let expires_at = pat::check(transport, repo, &token, now).map_err(|r| r.message())?;
    // The same acceptance the source applies, so a token stored here is one `issue` will hold.
    ScopedCredential::pasted(repo, token.clone(), expires_at, now).map_err(|r| r.message())?;
    store
        .put(repo, &StoredPat { token, expires_at })
        .map_err(|e| format!("Could not store the token: {e}"))?;
    Ok(expires_at)
}

/// Where a daemon's messages go: its connection.
pub type Sink = Arc<dyn Fn(ClientMsg) + Send + Sync>;

/// Where a line of diagnosis goes. Never given a token.
pub type Log = Arc<dyn Fn(&str) + Send + Sync>;

struct Entry {
    schedule: Schedule,
    send: Sink,
}

/// Each project's credential status, by the daemon's path for it.
pub type Statuses = BTreeMap<PathBuf, CredentialStatus>;

/// Where a daemon's changed statuses go, with the time they hold at: the window (FR-013).
/// `false` when it could not be delivered, so it is sent again.
pub type Publish = Arc<dyn Fn(DaemonId, Statuses, UnixTime) -> bool + Send + Sync>;

/// Every connected sandboxed daemon's [`Schedule`], and the source they issue through.
pub struct Manager {
    source: Arc<dyn CredentialSource>,
    log: Log,
    daemons: BTreeMap<DaemonId, Entry>,
    /// What [`Manager::changed_statuses`] last reported, per daemon, with which of them were
    /// within [`EXPIRY_WARNING_SECS`] of their expiry.
    reported: BTreeMap<DaemonId, (Statuses, Vec<bool>)>,
}

impl Manager {
    /// A manager issuing through `source` with no daemon.
    pub fn new(source: Arc<dyn CredentialSource>, log: Log) -> Self {
        Self {
            source,
            log,
            daemons: BTreeMap::new(),
            reported: BTreeMap::new(),
        }
    }

    /// The daemons whose statuses at `now` differ from what this last reported, with their new
    /// statuses; a held credential coming within [`EXPIRY_WARNING_SECS`] of its expiry counts as a
    /// change (FR-008), and a daemon whose schedule was dropped (disconnected, stopped) reports an
    /// empty map once, so the window clears its lines (FR-013).
    pub fn changed_statuses(&mut self, now: UnixTime) -> Vec<(DaemonId, Statuses)> {
        let mut changed: Vec<(DaemonId, Statuses)> = Vec::new();
        self.reported.retain(|id, _| {
            let kept = self.daemons.contains_key(id);
            if !kept {
                changed.push((*id, Statuses::new()));
            }
            kept
        });
        for (id, entry) in &self.daemons {
            let statuses = entry.schedule.statuses(now);
            let warned = statuses
                .values()
                .map(|s| s.expires_within(now, EXPIRY_WARNING_SECS))
                .collect();
            let current = (statuses, warned);
            if self.reported.get(id) != Some(&current) {
                changed.push((*id, current.0.clone()));
                self.reported.insert(*id, current);
            }
        }
        changed
    }

    /// `daemon`'s statuses did not reach the window: report them again next time.
    pub fn unreported(&mut self, daemon: DaemonId) {
        self.reported.remove(&daemon);
    }

    /// `daemon` reported `targets` (the whole list), after an `Attach` or a change. Creates its
    /// schedule on first sight (so a reconnect re-grants everything), tells the daemon to delete
    /// the files of repositories no project uses any more, and leaves the rest due.
    pub fn targets(
        &mut self,
        daemon: DaemonId,
        send: Sink,
        targets: Vec<CredentialTarget>,
        git_too_old: Option<String>,
        now: UnixTime,
    ) {
        let entry = self.daemons.entry(daemon).or_insert_with(|| Entry {
            schedule: Schedule::new(Grant::default(), RenewPolicy::AtExpiry),
            send: send.clone(),
        });
        entry.send = send;
        let gone = entry
            .schedule
            .set_targets(&targets, git_too_old.as_deref(), &*self.source, now);
        for repo in gone {
            (entry.send)(ClientMsg::GitCredentialRevoke { repo });
        }
    }

    /// Check every token that is due at `now`. A good one is granted to its own daemon; one that
    /// leaves nothing valid to hold (not stored, expired, refused) is withdrawn from it, so the
    /// container keeps no copy (FR-010). A token that could not be checked for lack of network is
    /// kept.
    pub fn run_due(&mut self, now: UnixTime) {
        for entry in self.daemons.values_mut() {
            for mut renewal in entry.schedule.run(&*self.source, now) {
                match &renewal.result {
                    Ok(credential) => {
                        (entry.send)(ClientMsg::GitCredentialGrant {
                            repo: renewal.repo.clone(),
                            token: credential.token().clone(),
                            expires_at: credential.expires_at(),
                        });
                        renewal.revoke_superseded(&*self.source);
                    }
                    Err(reason) => {
                        (self.log)(&format!(
                            "git credential: {}: {}",
                            renewal.repo,
                            reason.message()
                        ));
                        if renewal.withdraw {
                            (entry.send)(ClientMsg::GitCredentialRevoke {
                                repo: renewal.repo.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    /// The token of `repo` was replaced or removed: every daemon using it checks it again now.
    pub fn refresh(&mut self, repo: &GithubRepo, now: UnixTime) {
        for entry in self.daemons.values_mut() {
            entry.schedule.refresh(repo, now);
        }
    }

    /// When [`Manager::run_due`] next has something to do, or a held credential comes within
    /// [`EXPIRY_WARNING_SECS`] of its expiry so its status is reported again, across every daemon.
    pub fn next_wake(&self, now: UnixTime) -> Option<UnixTime> {
        let warnings = self.daemons.values().flat_map(|e| {
            e.schedule
                .statuses(now)
                .into_values()
                .filter_map(|status| match status {
                    CredentialStatus::Held { expires_at, .. } => {
                        Some(expires_at.saturating_sub(EXPIRY_WARNING_SECS))
                    }
                    _ => None,
                })
                .filter(move |warn_at| *warn_at > now)
        });
        self.daemons
            .values()
            .filter_map(|e| e.schedule.next_wake(now))
            .chain(warnings)
            .min()
    }

    /// The connection of `daemon` ended: forget its schedule without sending anything (the
    /// connection is gone). The daemon keeps its files, and a reconnect re-grants because the
    /// daemon announces its targets again after `Attach`.
    pub fn disconnected(&mut self, daemon: DaemonId) {
        self.daemons.remove(&daemon);
    }

    /// `daemon` was stopped or deleted: tell it to delete every copy while its connection may still
    /// carry the message, and drop the schedule. Nothing reaches GitHub: the tokens are the user's.
    pub fn stopped(&mut self, daemon: DaemonId) {
        if let Some(mut entry) = self.daemons.remove(&daemon) {
            for repo in entry.schedule.revoke_all(&*self.source) {
                (entry.send)(ClientMsg::GitCredentialRevoke { repo });
            }
        }
    }

    /// The status of `project` on `daemon`, when the daemon has reported.
    #[cfg(test)]
    pub fn status(
        &self,
        daemon: DaemonId,
        project: &std::path::Path,
        now: UnixTime,
    ) -> Option<CredentialStatus> {
        self.daemons
            .get(&daemon)
            .map(|e| e.schedule.status(project, now))
    }
}

type Command = Box<dyn FnOnce(&mut Manager, UnixTime) + Send>;

/// The `App`'s handle on the manager. Every method posts a command to the worker thread and
/// returns at once.
#[derive(Clone)]
pub struct GitCredentials {
    tx: mpsc::Sender<Command>,
}

impl GitCredentials {
    /// Start the worker over the device's pasted tokens, reporting statuses to the window.
    pub fn start() -> Self {
        let log: Log = Arc::new(crate::log_line);
        let publish: Publish = Arc::new(|daemon, statuses, now| {
            status_channel()
                .0
                .unbounded_send((daemon, statuses, now))
                .is_ok()
        });
        Self::start_with(Arc::new(PatSource::on_device()), log, publish)
    }

    /// Start the worker over `source`.
    pub fn start_with(source: Arc<dyn CredentialSource>, log: Log, publish: Publish) -> Self {
        let (tx, rx) = mpsc::channel::<Command>();
        std::thread::Builder::new()
            .name("scoped-credentials".into())
            .spawn(move || worker(rx, Manager::new(source, log), publish))
            .ok();
        Self { tx }
    }

    /// A handle with no worker: every command is dropped. For tests that build an `App`.
    #[cfg(test)]
    pub fn idle() -> Self {
        Self {
            tx: mpsc::channel().0,
        }
    }

    fn post(&self, command: impl FnOnce(&mut Manager, UnixTime) + Send + 'static) {
        let _ = self.tx.send(Box::new(command));
    }

    /// `daemon` announced its push targets; `send` reaches that daemon's connection.
    pub fn targets(
        &self,
        daemon: DaemonId,
        send: Sink,
        targets: Vec<CredentialTarget>,
        git_too_old: Option<String>,
    ) {
        self.post(move |m, now| m.targets(daemon, send, targets, git_too_old, now));
    }

    /// `daemon`'s connection ended: drop its schedule, send nothing.
    pub fn disconnected(&self, daemon: DaemonId) {
        self.post(move |m, _| m.disconnected(daemon));
    }

    /// `daemon` was stopped or deleted: delete its copies and drop its schedule.
    pub fn stopped(&self, daemon: DaemonId) {
        self.post(move |m, _| m.stopped(daemon));
    }

    /// The token of `repo` was replaced or removed in Settings: check it again for every daemon.
    pub fn refresh(&self, repo: GithubRepo) {
        self.post(move |m, now| m.refresh(&repo, now));
    }
}

fn worker(rx: mpsc::Receiver<Command>, mut manager: Manager, publish: Publish) {
    loop {
        let now = micold_core::unix_time::unix_secs();
        let received = match manager.next_wake(now) {
            Some(wake) => rx.recv_timeout(Duration::from_secs(wake.saturating_sub(now).max(1))),
            None => rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        };
        match received {
            Ok(command) => command(&mut manager, micold_core::unix_time::unix_secs()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
        let now = micold_core::unix_time::unix_secs();
        manager.run_due(now);
        for (daemon, statuses) in manager.changed_statuses(now) {
            if !publish(daemon, statuses, now) {
                manager.unreported(daemon);
            }
        }
    }
}

/// One report: the daemon, its statuses, and the time they hold at.
pub type StatusReport = (DaemonId, Statuses, UnixTime);
type StatusChannel = (
    UnboundedSender<StatusReport>,
    Mutex<Option<UnboundedReceiver<StatusReport>>>,
);

/// The process's one channel of credential statuses: the worker sends, [`statuses`] receives.
fn status_channel() -> &'static StatusChannel {
    static CHANNEL: OnceLock<StatusChannel> = OnceLock::new();
    CHANNEL.get_or_init(|| {
        let (sent, received) = unbounded();
        (sent, Mutex::new(Some(received)))
    })
}

/// The credential statuses the worker reports, as the window receives them (FR-013). Nothing
/// arrives, and nothing wakes the window, until a status changes.
pub fn statuses() -> iced::Subscription<StatusReport> {
    iced::Subscription::run(received_statuses)
}

/// The receiving end, handed out once: the subscription lives as long as the window.
fn received_statuses() -> BoxStream<'static, StatusReport> {
    let taken = status_channel()
        .1
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    match taken {
        Some(received) => received.boxed(),
        None => stream::pending().boxed(),
    }
}

#[cfg(test)]
mod tests;
