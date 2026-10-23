//! What a client and remote daemon of different versions are told (feature 489, FR-011, SC-005).
//!
//! The handshake refusal names both sides; this turns them into one sentence that says which side
//! is older and the way out. Pure, so each direction and each way out is a test with no `ssh`.
//! Nothing here ends a running session: an older daemon is offered a replacement only when its
//! sessions are known to be none (M7 supplies that knowledge; until then it is [`ReplaceOffer::Unknown`]).

use std::cmp::Ordering;

/// Which side runs the older version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// This app is older than the daemon.
    Client,
    /// The remote daemon is older than this app.
    Daemon,
    /// The two versions cannot be ordered.
    Unknown,
}

/// Whether the remote daemon may be replaced, from what it reports about its sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaceOffer {
    /// The daemon proves it has no running session.
    Available,
    /// The daemon has `running` sessions, which must not be ended to resolve a mismatch.
    Blocked {
        /// How many sessions run on it.
        running: u32,
    },
    /// Its sessions could not be counted.
    Unknown,
}

/// The way out of a mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WayOut {
    /// The daemon is newer: update this app. Replacing it would downgrade the host.
    UpdateApp,
    /// The daemon is older and has no sessions: replace it.
    ReplaceDaemon,
    /// The daemon is older and `running` sessions run on it: end them, then replace it.
    EndSessionsThenReplace {
        /// How many sessions run on it.
        running: u32,
    },
    /// Nothing is known that makes replacing it safe: end the daemon on the host by hand.
    EndDaemonByHand,
}

/// A version mismatch, explained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MismatchExplanation {
    /// The client's version.
    pub client: String,
    /// The daemon's version.
    pub daemon: String,
    /// Which side is older.
    pub older: Side,
    /// What the user does about it.
    pub way_out: WayOut,
}

/// The way out for the older side and what is known of the daemon's sessions.
pub fn way_out(older: Side, offer: ReplaceOffer) -> WayOut {
    match (older, offer) {
        (Side::Client, _) => WayOut::UpdateApp,
        (Side::Daemon, ReplaceOffer::Available) => WayOut::ReplaceDaemon,
        (Side::Daemon, ReplaceOffer::Blocked { running }) => {
            WayOut::EndSessionsThenReplace { running }
        }
        (Side::Daemon, ReplaceOffer::Unknown) | (Side::Unknown, _) => WayOut::EndDaemonByHand,
    }
}

/// The version a handshake string names: `micold-ai-ide/0.5.3` and `micold-daemon 0.5.3` are
/// `0.5.3`, a bare number is itself.
pub fn version_of(text: &str) -> &str {
    text.rsplit(|c: char| c == '/' || c.is_whitespace())
        .next()
        .unwrap_or(text)
}

/// A dotted number and whether it carries a pre-release suffix (`0.5.3-old`).
fn parse(version: &str) -> Option<(Vec<u64>, bool)> {
    let (core, pre) = match version.split_once('-') {
        Some((core, _)) => (core, true),
        None => (version, false),
    };
    let parts = core
        .split('.')
        .map(|p| p.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;
    Some((parts, pre))
}

/// Which of the two versions is older, `Unknown` when either does not read as a dotted number.
/// A pre-release is older than its release; a longer dotted version is newer than its prefix.
pub fn older_side(client: &str, daemon: &str) -> Side {
    let (Some(c), Some(d)) = (parse(version_of(client)), parse(version_of(daemon))) else {
        return Side::Unknown;
    };
    // A release outranks its pre-release, so compare "is a pre-release" reversed.
    match c.0.cmp(&d.0).then(d.1.cmp(&c.1)) {
        Ordering::Less => Side::Client,
        Ordering::Greater => Side::Daemon,
        Ordering::Equal => Side::Unknown,
    }
}

impl MismatchExplanation {
    /// Explain the refusal that named `client` and `daemon`.
    pub fn new(client: &str, daemon: &str, offer: ReplaceOffer) -> Self {
        let older = older_side(client, daemon);
        Self {
            client: version_of(client).to_string(),
            daemon: version_of(daemon).to_string(),
            older,
            way_out: way_out(older, offer),
        }
    }

    /// The sentence shown for daemon `name` on `host`, a host the user owns: a daemon that must not
    /// be replaced by the app is ended there by the user.
    pub fn text(&self, name: &str, host: &str) -> String {
        self.compose(name, Holder::UserHost(host))
    }

    /// The sentence shown for daemon `name` in a container the app owns (spec 806 FR-007,
    /// BUG-1165): the user ends such a daemon with its row's Stop, and Start brings up this app's.
    pub fn text_app_owned(&self, name: &str) -> String {
        self.compose(name, Holder::AppContainer)
    }

    /// The sentence shown for daemon `name` in an app-owned container still on the released image
    /// (spec 806 FR-017): its daemon is not started by the bridge, so Stop and Start would meet it
    /// again; the banner's Restart service replaces the container.
    pub fn text_released_image(&self, name: &str) -> String {
        self.compose(name, Holder::ReleasedContainer)
    }

    fn compose(&self, name: &str, holder: Holder<'_>) -> String {
        let head = format!(
            "daemon {name} version mismatch: client {}, daemon {}",
            self.client, self.daemon
        );
        let age = match self.older {
            Side::Daemon => " (daemon is older)",
            Side::Client => " (daemon is newer)",
            Side::Unknown => "",
        };
        let advice = match (self.way_out, holder) {
            (WayOut::UpdateApp, _) => "Update this app.".to_string(),
            (WayOut::ReplaceDaemon, _) => {
                "No sessions are running on it: replace it to continue.".to_string()
            }
            (WayOut::EndSessionsThenReplace { running }, Holder::UserHost(host)) => format!(
                "{running} session(s) are running; end them, or end the daemon on {host}, \
                 then reconnect."
            ),
            (WayOut::EndDaemonByHand, Holder::UserHost(host)) => format!(
                "Its sessions could not be counted; end the daemon on {host} yourself, \
                 then reconnect."
            ),
            (WayOut::EndSessionsThenReplace { running }, Holder::AppContainer) => {
                format!("{running} session(s) are running; end them, or {APP_OWNED_WAY_OUT}")
            }
            (WayOut::EndDaemonByHand, Holder::AppContainer) => {
                format!("Its sessions could not be counted; {APP_OWNED_WAY_OUT}")
            }
            (WayOut::EndSessionsThenReplace { running }, Holder::ReleasedContainer) => {
                format!("{running} session(s) are running; end them, or {RELEASED_WAY_OUT}")
            }
            (WayOut::EndDaemonByHand, Holder::ReleasedContainer) => {
                format!("Its sessions could not be counted; {RELEASED_WAY_OUT}")
            }
        };
        format!("{head}{age}. {advice}")
    }
}

/// Where a mismatched daemon runs, which decides how the user ends it.
#[derive(Debug, Clone, Copy)]
enum Holder<'a> {
    /// A host the user owns, named for the sentence.
    UserHost(&'a str),
    /// A container the app owns, on the basic image.
    AppContainer,
    /// A container the app owns, still on the released image.
    ReleasedContainer,
}

/// How the user moves a mismatched released-image container to this app's version.
const RELEASED_WAY_OUT: &str =
    "use Restart service in the banner to move it to this app's version.";

/// How the user ends a mismatched daemon the app owns: its row's Stop ends the container, and Start
/// brings up this app's version in it.
const APP_OWNED_WAY_OUT: &str = "Stop this daemon, then Start it to run this app's version.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_older_daemon_is_told_apart_from_a_newer_one_by_version() {
        assert_eq!(older_side("0.5.3", "0.5.2"), Side::Daemon);
        assert_eq!(older_side("0.5.2", "0.5.3"), Side::Client);
        assert_eq!(older_side("0.10.0", "0.9.0"), Side::Daemon);
        assert_eq!(older_side("12", "11"), Side::Daemon);
    }

    #[test]
    fn handshake_build_strings_are_read_by_their_version() {
        assert_eq!(version_of("micold-ai-ide/0.5.3"), "0.5.3");
        assert_eq!(version_of("micold-daemon 0.5.3"), "0.5.3");
        assert_eq!(version_of("11"), "11");
        assert_eq!(
            older_side("micold-ai-ide/0.5.3", "micold-ai-ide/0.5.2"),
            Side::Daemon
        );
    }

    #[test]
    fn a_prerelease_is_older_than_its_release_and_a_longer_dotted_version_is_newer() {
        assert_eq!(older_side("0.5.3", "0.5.3-old"), Side::Daemon);
        assert_eq!(older_side("0.5.3-old", "0.5.3"), Side::Client);
        assert_eq!(older_side("0.5.3", "0.5.3.999"), Side::Client);
    }

    #[test]
    fn versions_that_do_not_read_as_numbers_or_are_equal_have_no_older_side() {
        assert_eq!(older_side("abc", "0.5.3"), Side::Unknown);
        assert_eq!(older_side("0.5.3", "0.5.3"), Side::Unknown);
        assert_eq!(older_side("", ""), Side::Unknown);
    }

    #[test]
    fn the_way_out_is_a_function_of_the_older_side_and_the_offer() {
        use ReplaceOffer::*;
        assert_eq!(way_out(Side::Daemon, Available), WayOut::ReplaceDaemon);
        assert_eq!(
            way_out(Side::Daemon, Blocked { running: 2 }),
            WayOut::EndSessionsThenReplace { running: 2 }
        );
        assert_eq!(way_out(Side::Daemon, Unknown), WayOut::EndDaemonByHand);
        for offer in [Available, Blocked { running: 1 }, Unknown] {
            assert_eq!(way_out(Side::Client, offer), WayOut::UpdateApp, "{offer:?}");
            assert_eq!(
                way_out(Side::Unknown, offer),
                WayOut::EndDaemonByHand,
                "{offer:?}"
            );
        }
    }

    fn text(client: &str, daemon: &str, offer: ReplaceOffer) -> String {
        MismatchExplanation::new(client, daemon, offer).text("Box", "dev@host")
    }

    #[test]
    fn an_older_daemon_with_no_sessions_is_offered_a_replacement() {
        assert_eq!(
            text("0.5.3", "0.5.2", ReplaceOffer::Available),
            "daemon Box version mismatch: client 0.5.3, daemon 0.5.2 (daemon is older). \
             No sessions are running on it: replace it to continue."
        );
    }

    #[test]
    fn an_older_daemon_with_sessions_says_how_many_and_never_offers_to_end_them() {
        assert_eq!(
            text("0.5.3", "0.5.2", ReplaceOffer::Blocked { running: 3 }),
            "daemon Box version mismatch: client 0.5.3, daemon 0.5.2 (daemon is older). \
             3 session(s) are running; end them, or end the daemon on dev@host, then reconnect."
        );
    }

    #[test]
    fn a_newer_daemon_says_to_update_the_app_whatever_the_offer() {
        for offer in [
            ReplaceOffer::Available,
            ReplaceOffer::Blocked { running: 1 },
            ReplaceOffer::Unknown,
        ] {
            assert_eq!(
                text("0.5.2", "0.5.3", offer),
                "daemon Box version mismatch: client 0.5.2, daemon 0.5.3 (daemon is newer). \
                 Update this app."
            );
        }
    }

    #[test]
    fn an_older_daemon_whose_sessions_are_unknown_is_ended_by_hand() {
        assert_eq!(
            text("0.5.3", "0.5.2", ReplaceOffer::Unknown),
            "daemon Box version mismatch: client 0.5.3, daemon 0.5.2 (daemon is older). \
             Its sessions could not be counted; end the daemon on dev@host yourself, then reconnect."
        );
    }

    #[test]
    fn an_app_owned_daemon_names_its_rows_stop_instead_of_a_host() {
        let owned = |offer| MismatchExplanation::new("0.5.3", "0.5.2", offer).text_app_owned("Box");
        assert_eq!(
            owned(ReplaceOffer::Blocked { running: 2 }),
            "daemon Box version mismatch: client 0.5.3, daemon 0.5.2 (daemon is older). \
             2 session(s) are running; end them, or Stop this daemon, then Start it to run this \
             app's version."
        );
        assert_eq!(
            owned(ReplaceOffer::Unknown),
            "daemon Box version mismatch: client 0.5.3, daemon 0.5.2 (daemon is older). \
             Its sessions could not be counted; Stop this daemon, then Start it to run this \
             app's version."
        );
        assert_eq!(
            MismatchExplanation::new("0.5.3", "0.5.2", ReplaceOffer::Unknown)
                .text_released_image("Box"),
            "daemon Box version mismatch: client 0.5.3, daemon 0.5.2 (daemon is older). \
             Its sessions could not be counted; use Restart service in the banner to move it to \
             this app's version."
        );
        // The other ways out read the same for both kinds of daemon.
        assert_eq!(
            owned(ReplaceOffer::Available),
            text("0.5.3", "0.5.2", ReplaceOffer::Available)
        );
    }

    #[test]
    fn versions_that_cannot_be_ordered_still_name_both_and_the_way_out() {
        assert_eq!(
            text("abc", "def", ReplaceOffer::Unknown),
            "daemon Box version mismatch: client abc, daemon def. \
             Its sessions could not be counted; end the daemon on dev@host yourself, then reconnect."
        );
    }

    #[test]
    fn the_explanation_keeps_both_versions_without_the_build_prefix() {
        let e = MismatchExplanation::new(
            "micold-ai-ide/0.5.3",
            "micold-ai-ide/0.5.2",
            ReplaceOffer::Unknown,
        );
        assert_eq!((e.client.as_str(), e.daemon.as_str()), ("0.5.3", "0.5.2"));
        assert_eq!(
            (e.older, e.way_out),
            (Side::Daemon, WayOut::EndDaemonByHand)
        );
    }
}
