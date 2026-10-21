//! The plan-usage sources' shared parts in the service (feature 490; feature 944, research R4):
//! the once-per-answer log of readings not used (FR-016), and the feed that carries a structured
//! session's plan-limit events from its reader thread to [`DaemonState::offer_plan_usage`].

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use micold_core::plan_usage::{rate_limit_event, UsageReading};
use micold_core::session::SessionId;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::state::{unix_secs, DaemonState};

/// The answers already logged, per session, since its last accepted reading (feature 490, S3.1,
/// FR-016): an unchanged failure is logged once, whichever source it came from.
#[derive(Clone, Default)]
pub struct Rejections(Arc<Mutex<HashSet<(Uuid, &'static str)>>>);

impl Rejections {
    /// Log a reading `session` gave that was not used, at debug and once per (session, answer)
    /// until that session's next accepted reading (S3.1, FR-016). Nothing about it is a warning:
    /// plan usage that cannot be read is simply not shown (FR-015).
    pub fn reject(&self, session: Uuid, answer: &'static str) {
        if self.lock().insert((session, answer)) {
            tracing::debug!(%session, answer, "plan usage reading not used");
        }
    }

    /// `session`'s reading was accepted: its next failure is news again (FR-016).
    pub fn accepted(&self, session: Uuid) {
        self.lock().retain(|(uuid, _)| *uuid != session);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashSet<(Uuid, &'static str)>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// A reading a structured session's running CLI reported, on its way to the service.
pub type Offer = (SessionId, UsageReading);

/// The feed's two ends: the sender every structured session's reader holds, and the receiver the
/// service's feed task takes once at start ([`crate::server::spawn_plan_usage_feed`]).
pub struct Feed {
    sender: mpsc::UnboundedSender<Offer>,
    receiver: Mutex<Option<mpsc::UnboundedReceiver<Offer>>>,
}

impl Default for Feed {
    fn default() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
        }
    }
}

impl Feed {
    /// A sender for one structured session's reader.
    pub fn sender(&self) -> mpsc::UnboundedSender<Offer> {
        self.sender.clone()
    }

    /// The receiving end, the first time only.
    pub fn take_receiver(&self) -> Option<mpsc::UnboundedReceiver<Offer>> {
        self.receiver
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }
}

/// What a structured session's reader does with one line its running CLI wrote that the event
/// model does not type (contracts/plan-usage-source.md *Feed* 1, 3): a plan-limit event with a
/// usable window is sent to the feed; one without is logged once and otherwise ignored (FR-012).
/// Any other line is left alone. Events of a restored history never come here (FR-013).
pub fn offer_line(
    session: SessionId,
    raw: &str,
    feed: &mpsc::UnboundedSender<Offer>,
    rejections: &Rejections,
) {
    if !raw.contains("rate_limit_event") {
        return;
    }
    match rate_limit_event::parse(raw, unix_secs()) {
        // A send fails only once the service is stopping; the reading is moot then.
        Some(reading) => {
            let _ = feed.send((session, reading));
        }
        None if is_plan_limit_event(raw) => {
            rejections.reject(session.0, "unreadable plan-limit event")
        }
        None => {}
    }
}

fn is_plan_limit_event(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .is_some_and(|line| line.get("type").and_then(|t| t.as_str()) == Some("rate_limit_event"))
}

/// Drain `feed` into `state` until every sender is gone.
pub async fn drain(state: Arc<DaemonState>, mut feed: mpsc::UnboundedReceiver<Offer>) {
    while let Some((session, reading)) = feed.recv().await {
        state.offer_plan_usage(session, reading);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logged(rejections: &Rejections) -> Vec<(Uuid, &'static str)> {
        rejections.lock().iter().copied().collect()
    }

    /// Feature 944 T021 (FR-012, feature 490 FR-016): a readable plan-limit event is sent to the
    /// feed; an unreadable one is logged once per session until its next accepted reading; any
    /// other line is neither.
    #[test]
    fn a_line_is_sent_logged_once_or_left_alone() {
        let (feed, mut received) = mpsc::unbounded_channel();
        let rejections = Rejections::default();
        let session = SessionId::from_uuid(Uuid::from_u128(0x944));
        let readable = format!(
            r#"{{"type":"rate_limit_event","rate_limit_info":{{"unifiedWindows":{{"five_hour":{{"utilization":0.2,"resetsAt":{}}}}}}}}}"#,
            unix_secs() + 3600
        );
        let unreadable = r#"{"type":"rate_limit_event","rate_limit_info":"none"}"#;

        offer_line(session, r#"{"type":"active_goal"}"#, &feed, &rejections);
        offer_line(
            session,
            r#"{"type":"other","note":"rate_limit_event"}"#,
            &feed,
            &rejections,
        );
        assert!(received.try_recv().is_err());
        assert_eq!(logged(&rejections), []);

        offer_line(session, unreadable, &feed, &rejections);
        offer_line(session, unreadable, &feed, &rejections);
        assert!(received.try_recv().is_err(), "no reading");
        assert_eq!(
            logged(&rejections),
            [(session.0, "unreadable plan-limit event")]
        );

        offer_line(session, &readable, &feed, &rejections);
        let (from, reading) = received.try_recv().expect("a reading");
        assert_eq!(from, session);
        assert_eq!(reading.windows[0].used_tenths, 200);
        rejections.accepted(session.0);
        assert_eq!(
            logged(&rejections),
            [],
            "news again after an accepted reading"
        );
    }
}
