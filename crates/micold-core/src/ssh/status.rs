//! What a remote daemon says about itself, and how the client uses it to replace a mismatched one
//! (feature 489, FR-011, research R4).
//!
//! A client of another version cannot speak to the daemon, and a refused handshake carries no
//! catalog. The one thing every build of this feature writes in the same shape is `status.json`
//! beside the daemon's lock file; `micold-daemon remote status | stop` read it. Nothing here is
//! read by the wire path, and nothing here ends a daemon that has a running session.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::argv::{self, Reach};
use super::mismatch::ReplaceOffer;
use super::provision::{ProvisionOptions, SshRunner};

/// The shape version of [`RemoteStatus`]; a reader that meets another one believes nothing of it.
pub const STATUS_VERSION: u32 = 1;
/// The file's name, beside the daemon's lock file.
pub const STATUS_FILE: &str = "status.json";
/// `remote stop` exit code: sessions are running, the daemon was not stopped.
pub const EXIT_SESSIONS_RUNNING: i32 = 3;
/// `remote status | stop` exit code: nothing proves the daemon has no sessions.
pub const EXIT_UNKNOWN: i32 = 4;

/// The `status.json` shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteStatus {
    /// [`STATUS_VERSION`].
    pub status_version: u32,
    /// The daemon's process id.
    pub pid: u32,
    /// The daemon's package version.
    pub package_version: String,
    /// The daemon's build fingerprint.
    pub build: String,
    /// The daemon's protocol version.
    pub protocol_version: u32,
    /// How many sessions run on it.
    pub running_sessions: u32,
    /// The endpoint it listens on.
    pub endpoint: String,
}

impl RemoteStatus {
    /// Read a status file's text. `None` for text that is not JSON, lacks a field, or carries a
    /// `status_version` this build does not know; fields it does not know are ignored.
    pub fn parse(text: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(text).ok()?;
        if value.get("status_version")?.as_u64()? != u64::from(STATUS_VERSION) {
            return None;
        }
        serde_json::from_value(value).ok()
    }

    /// The one-line JSON `remote status` prints.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Whether the daemon may be replaced: only when it has no running session.
    pub fn offer(&self) -> ReplaceOffer {
        match self.running_sessions {
            0 => ReplaceOffer::Available,
            running => ReplaceOffer::Blocked { running },
        }
    }
}

/// The offer for the output of `remote status`: anything but a readable status is `Unknown`.
pub fn offer_of(stdout: &str) -> ReplaceOffer {
    RemoteStatus::parse(stdout.trim()).map_or(ReplaceOffer::Unknown, |s| s.offer())
}

/// Ask the host (a user's SSH host, or an app-owned container over its pinned reach) what its
/// daemon has running (`remote status` of this build's helper).
pub async fn fetch_offer(
    runner: &dyn SshRunner,
    reach: &Reach,
    opts: &ProvisionOptions,
) -> ReplaceOffer {
    let cmd = argv::status(reach, opts.platform, opts.timeout_secs, &opts.build_id);
    let deadline = Some(Duration::from_secs(u64::from(opts.timeout_secs)));
    match runner.run(cmd, Vec::new(), deadline, None).await {
        Ok(out) if out.ok() => offer_of(&String::from_utf8_lossy(&out.stdout)),
        _ => ReplaceOffer::Unknown,
    }
}

/// Replace the host's daemon: end it, only when it proves it has no running session. The caller
/// reconnects afterwards, which starts this build's daemon in its place. `Err` carries why not.
pub async fn stop_if_idle(
    runner: &dyn SshRunner,
    reach: &Reach,
    opts: &ProvisionOptions,
) -> Result<(), ReplaceOffer> {
    let offer = fetch_offer(runner, reach, opts).await;
    if offer != ReplaceOffer::Available {
        return Err(offer);
    }
    let cmd = argv::stop(reach, opts.platform, opts.timeout_secs, &opts.build_id);
    // The helper waits for the endpoint to stop accepting before it exits.
    let deadline = Some(Duration::from_secs(u64::from(opts.timeout_secs) + 10));
    match runner.run(cmd, Vec::new(), deadline, None).await {
        Ok(out) if out.ok() => Ok(()),
        Ok(_) | Err(_) => match fetch_offer(runner, reach, opts).await {
            ReplaceOffer::Available => Err(ReplaceOffer::Unknown),
            other => Err(other),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::provision::{ProvisionProgress, SshOutput};
    use crate::ssh::{ClientPlatform, SshCommand, SshStdio};
    use futures_util::future::BoxFuture;
    use std::collections::VecDeque;
    use std::io;
    use std::sync::Mutex;

    fn json(sessions: u32) -> String {
        format!(
            r#"{{"status_version":1,"pid":42,"package_version":"0.5.2","build":"abc","protocol_version":50,"running_sessions":{sessions},"endpoint":"/tmp/x/d.sock"}}"#
        )
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let text = json(0).replace("\"pid\"", "\"future\":[1,2],\"pid\"");
        let status = RemoteStatus::parse(&text).expect("parses");
        assert_eq!((status.pid, status.running_sessions), (42, 0));
    }

    #[test]
    fn an_unknown_status_version_is_unknown_not_available() {
        let text = json(0).replace("\"status_version\":1", "\"status_version\":2");
        assert_eq!(RemoteStatus::parse(&text), None);
        assert_eq!(offer_of(&text), ReplaceOffer::Unknown);
        assert_eq!(offer_of("not json"), ReplaceOffer::Unknown);
        assert_eq!(offer_of(""), ReplaceOffer::Unknown);
    }

    #[test]
    fn sessions_decide_the_offer() {
        assert_eq!(offer_of(&json(0)), ReplaceOffer::Available);
        assert_eq!(offer_of(&json(3)), ReplaceOffer::Blocked { running: 3 });
    }

    #[test]
    fn json_round_trips() {
        let status = RemoteStatus::parse(&json(2)).unwrap();
        assert_eq!(RemoteStatus::parse(&status.to_json()), Some(status));
    }

    struct Fake {
        runs: Mutex<VecDeque<SshOutput>>,
        seen: Mutex<Vec<String>>,
        args: Mutex<Vec<Vec<String>>>,
    }

    impl Fake {
        fn new(runs: Vec<SshOutput>) -> Self {
            Self {
                runs: Mutex::new(runs.into()),
                seen: Mutex::default(),
                args: Mutex::default(),
            }
        }
    }

    impl SshRunner for Fake {
        fn run(
            &self,
            cmd: SshCommand,
            _stdin: Vec<u8>,
            _deadline: Option<Duration>,
            _progress: Option<ProvisionProgress>,
        ) -> BoxFuture<'_, io::Result<SshOutput>> {
            self.seen
                .lock()
                .unwrap()
                .push(cmd.args.last().cloned().unwrap_or_default());
            self.args.lock().unwrap().push(cmd.args.clone());
            let next = self.runs.lock().unwrap().pop_front();
            Box::pin(async move { Ok(next.expect("unscripted run")) })
        }
        fn open(&self, _cmd: SshCommand) -> io::Result<SshStdio> {
            Err(io::Error::other("no bridge"))
        }
    }

    fn ok(stdout: String) -> SshOutput {
        SshOutput {
            status: Some(0),
            stdout: stdout.into_bytes(),
            ..SshOutput::default()
        }
    }

    fn exit(code: i32) -> SshOutput {
        SshOutput {
            status: Some(code),
            ..SshOutput::default()
        }
    }

    fn opts() -> ProvisionOptions {
        ProvisionOptions {
            platform: ClientPlatform::Linux,
            client_arch: "x86_64".into(),
            timeout_secs: 15,
            build_id: "1.0.0-abcdef123456".into(),
            client_build: "test".into(),
        }
    }

    fn target() -> Reach {
        Reach::UserConfig(crate::ssh::SshTarget::new("build-host", None, None, None).unwrap())
    }

    /// A container's reach: loopback, the app's own key and `known_hosts` (feature 806, BUG-1165).
    fn pinned() -> Reach {
        Reach::Pinned(crate::ssh::argv::PinnedReach {
            host: "127.0.0.1".into(),
            port: 7001,
            user: "node".into(),
            key: "/state/daemons/box/ssh/client_ed25519".into(),
            known_hosts: "/state/daemons/box/ssh/known_hosts".into(),
        })
    }

    #[tokio::test]
    async fn a_container_is_asked_and_stopped_over_its_pinned_reach() {
        let fake = Fake::new(vec![ok(json(0)), exit(0)]);
        assert_eq!(stop_if_idle(&fake, &pinned(), &opts()).await, Ok(()));
        let seen = fake.seen.lock().unwrap().clone();
        assert!(seen[0].ends_with("remote status"), "{seen:?}");
        assert!(seen[1].ends_with("remote stop"), "{seen:?}");
        for args in fake.args.lock().unwrap().iter() {
            let line = args.join(" ");
            assert!(line.contains("-F none"), "{line}");
            assert!(
                line.contains("/state/daemons/box/ssh/client_ed25519"),
                "{line}"
            );
            assert!(
                line.contains("UserKnownHostsFile=/state/daemons/box/ssh/known_hosts"),
                "{line}"
            );
            assert!(line.contains("-l node -p 7001"), "{line}");
        }
    }

    #[tokio::test]
    async fn an_idle_daemon_is_stopped_after_its_status_proves_it() {
        let fake = Fake::new(vec![ok(json(0)), exit(0)]);
        assert_eq!(stop_if_idle(&fake, &target(), &opts()).await, Ok(()));
        let seen = fake.seen.lock().unwrap().clone();
        assert!(seen[0].ends_with("remote status"), "{seen:?}");
        assert!(seen[1].ends_with("remote stop"), "{seen:?}");
    }

    #[tokio::test]
    async fn a_daemon_with_sessions_is_never_sent_a_stop() {
        let fake = Fake::new(vec![ok(json(2))]);
        assert_eq!(
            stop_if_idle(&fake, &target(), &opts()).await,
            Err(ReplaceOffer::Blocked { running: 2 })
        );
        assert_eq!(fake.seen.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn an_unreadable_status_stops_nothing() {
        let fake = Fake::new(vec![exit(4)]);
        assert_eq!(
            stop_if_idle(&fake, &target(), &opts()).await,
            Err(ReplaceOffer::Unknown)
        );
        assert_eq!(fake.seen.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_stop_that_loses_the_race_reports_the_sessions_that_arrived() {
        let fake = Fake::new(vec![ok(json(0)), exit(3), ok(json(1))]);
        assert_eq!(
            stop_if_idle(&fake, &target(), &opts()).await,
            Err(ReplaceOffer::Blocked { running: 1 })
        );
    }
}
