//! The daemon connection actor (client side of the render/drive switch, task T041).
//!
//! A single long-lived iced [`Subscription`] owns the socket to the daemon: it `connect_or_spawn`s
//! (auto-starting the daemon on a cold start), performs the strict handshake, and then runs a
//! bidirectional pump — forwarding the App's outgoing [`ClientMsg`]s to the daemon and turning
//! every incoming control message / grid frame into an app [`Message`]. The App drives sessions by
//! sending through the [`Outbox`] handed to it on connect; it never owns a PTY or the socket itself.
//!
//! Keeping the connection in a subscription (not scattered `Task`s) means exactly one socket, one
//! ordered inbound stream, and one place a disconnect is observed — the single-source-of-truth rule
//! the whole re-architecture rests on.

use crate::features::connection::Msg as ConnectionMsg;
use std::time::Instant;

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream, StreamExt};
use iced::Subscription;

use micold_core::connect::{connect_or_spawn, Connected, SPAWN_TIMEOUT};
use micold_core::endpoint;
use micold_core::protocol::codec::{CodecError, Frame};
use micold_core::protocol::keepalive::{self, Keepalive, KeepaliveAction};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};

use crate::app::Message;

/// The build string this client announces in the handshake (diagnostics only).
const CLIENT_BUILD: &str = concat!("micold-ai-ide/", env!("CARGO_PKG_VERSION"));

/// How many messages may queue in each direction before backpressure. Input is small and rare
/// relative to grid frames; a few hundred slots is ample without unbounded growth.
const CHANNEL_CAPACITY: usize = 256;

/// A cloneable handle the App uses to send [`ClientMsg`]s to the daemon. The channel is **unbounded**
/// so a stamped `SessionInput` is never dropped under backpressure — input is the lossless, ordered
/// log the whole feature rests on (G2); dropping a frame here would be an unrecoverable gap because
/// the [`crate::input::SessionInputStamper`] has already consumed its serial. (Control messages are
/// small and rare, so the unbounded channel cannot grow meaningfully from them.)
///
/// `PartialEq`/`Eq` compare *identity* (a shared `Arc` token) so [`Message`] stays `Eq` — clones of
/// one handle are equal (they feed the same connection); handles from different connections differ.
#[derive(Debug, Clone)]
pub struct Outbox {
    tx: mpsc::UnboundedSender<ClientMsg>,
    /// Identity token: cloned (pointer-shared) with the handle, so `Arc::ptr_eq` distinguishes
    /// connections without needing `Sender` to be `Eq` (it is not).
    id: std::sync::Arc<()>,
}

impl Outbox {
    /// `pub` (not private) so the binary's own tests (`main.rs`'s `update_inner` tests, T100) can
    /// build a real `Outbox` over a manually-created channel and assert what gets sent, without
    /// needing a live connection — the binary is a separate crate from this library, so
    /// `pub(crate)` would not reach it.
    pub fn new(tx: mpsc::UnboundedSender<ClientMsg>) -> Self {
        Self {
            tx,
            id: std::sync::Arc::new(()),
        }
    }

    /// Queue a message to the daemon (never blocks the UI thread; a closed channel is ignored).
    pub fn send(&self, msg: ClientMsg) {
        let _ = self.tx.unbounded_send(msg);
    }
}

impl PartialEq for Outbox {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.id, &other.id)
    }
}
impl Eq for Outbox {}

/// The connection subscription. Add it to the app's subscription set; iced keeps one instance alive
/// for the app's lifetime (the builder is a plain `fn`, so its identity is stable — a capturing
/// closure would make iced restart it every frame).
pub fn connection() -> Subscription<Message> {
    Subscription::run(actor)
}

/// How long to wait after a lost connection before trying to re-establish it. Short enough that a
/// restarted (or freed) daemon is picked up promptly; long enough not to spin on a hard failure. The
/// connection banner is what the user sees during the gap (FR-027); the reconnect itself is automatic
/// (FR-028 — the fresh `Welcome` catalog is the resync).
const RECONNECT_BACKOFF: std::time::Duration = std::time::Duration::from_secs(1);

/// One inbound event of the bidirectional pump, unifying the source streams so a single loop can
/// service all of them without a `select!` (which would need fused, pinned futures).
enum Io {
    /// A message the App wants sent to the daemon.
    Outgoing(ClientMsg),
    /// A frame that arrived from the daemon.
    Incoming(Result<Frame<DaemonMsg>, CodecError>),
    /// The keepalive check timer fired (FR-026).
    Tick,
}

/// Why the pump loop ended — the outer loop uses this to decide between reconnecting and giving up.
enum PumpEnd {
    /// The connection dropped (EOF, decode error, or a half-open link the keepalive reaped). The
    /// outer loop backs off and reconnects.
    Disconnected,
    /// The iced side is gone; stop the whole subscription.
    AppGone,
}

fn actor() -> impl Stream<Item = Message> {
    // iced 0.14 takes an `AsyncFnOnce` here (0.13 took an `FnOnce` returning a future), so the
    // sender's type no longer falls out of inference and has to be named.
    iced::stream::channel(
        CHANNEL_CAPACITY,
        |mut output: mpsc::Sender<Message>| async move {
            // Resolve the endpoint once; it does not change over the app's life.
            let endpoint = match endpoint::resolve() {
                Ok(e) => e,
                Err(err) => {
                    let _ = output
                        .send(Message::Connection(ConnectionMsg::ConnectFailed(
                            err.to_string(),
                        )))
                        .await;
                    return;
                }
            };

            // Reconnect loop: connect, pump until the link drops, surface it, back off, repeat. A
            // half-open connection is caught by the keepalive inside `pump`, so the client never sits
            // forever presenting stale content as live (FR-026/027).
            loop {
                match connect_and_pump(&endpoint, &mut output).await {
                    PumpEnd::AppGone => return,
                    PumpEnd::Disconnected => {
                        if output
                            .send(Message::Connection(ConnectionMsg::Disconnected))
                            .await
                            .is_err()
                        {
                            return;
                        }
                        tokio::time::sleep(RECONNECT_BACKOFF).await;
                    }
                }
            }
        },
    )
}

/// Connect (spawning the daemon on a cold start), announce `DaemonConnected`, then pump both
/// directions with a keepalive until the connection ends. A connect failure is surfaced as
/// `DaemonConnectFailed` and reported as a disconnect so the outer loop retries.
async fn connect_and_pump(
    endpoint: &endpoint::Endpoint,
    output: &mut mpsc::Sender<Message>,
) -> PumpEnd {
    let connected = match connect_or_spawn(endpoint, CLIENT_BUILD, SPAWN_TIMEOUT).await {
        Ok(c) => c,
        Err(err) => {
            return report_connect_failure(output, err.to_string()).await;
        }
    };
    let (conn, welcome) = match connected {
        Connected::Ready(conn, welcome) => (conn, welcome),
        // A contract mismatch is its own recoverable state (US6, FR-021/022): surface both versions
        // so the app can show the "restart service" action, not a generic connect error.
        Connected::Refused(micold_core::protocol::messages::RefusalReason::VersionMismatch {
            client,
            daemon,
            daemon_build,
            ..
        }) => {
            let sent = output
                .send(Message::Connection(ConnectionMsg::VersionMismatch {
                    client,
                    daemon,
                    daemon_build,
                }))
                .await
                .is_ok();
            return if sent {
                PumpEnd::Disconnected
            } else {
                PumpEnd::AppGone
            };
        }
        // Same contract, different package version — most releases don't touch the wire schema, so
        // this is the common shape a `.deb` upgrade takes (US6, FR-022a, BUG-002). Its own
        // recoverable state, distinct from a contract mismatch: nothing is actually incompatible,
        // only stale.
        Connected::Refused(micold_core::protocol::messages::RefusalReason::BuildMismatch {
            client_build,
            daemon_build,
        }) => {
            let sent = output
                .send(Message::Connection(ConnectionMsg::BuildMismatch {
                    client_build,
                    daemon_build,
                }))
                .await
                .is_ok();
            return if sent {
                PumpEnd::Disconnected
            } else {
                PumpEnd::AppGone
            };
        }
        Connected::Refused(reason) => {
            return report_connect_failure(
                output,
                format!("daemon refused the connection: {reason:?}"),
            )
            .await;
        }
    };

    // Hand the App a fresh Outbox and the welcome state (the resync on every (re)connect, FR-028).
    let (mut sink, incoming) = conn.split();
    let (tx, rx) = mpsc::unbounded::<ClientMsg>();
    if output
        .send(Message::Connection(ConnectionMsg::Connected {
            outbox: Outbox::new(tx),
            catalog: welcome.catalog,
            settings: welcome.settings,
        }))
        .await
        .is_err()
    {
        return PumpEnd::AppGone;
    }

    // Keepalive: poll on a fixed cadence; any inbound frame proves life, and the deadline turns a
    // silent half-open link into an explicit disconnect within the SC-011 budget (FR-026).
    let mut keepalive = Keepalive::new(Instant::now());
    let ticks = iced::futures::stream::unfold((), |()| async {
        tokio::time::sleep(keepalive::CHECK_INTERVAL).await;
        Some((Io::Tick, ()))
    });
    let mut events = iced::futures::stream::select(
        iced::futures::stream::select(rx.map(Io::Outgoing), incoming.map(Io::Incoming)),
        ticks.boxed(),
    );

    while let Some(io) = events.next().await {
        match io {
            Io::Outgoing(msg) => {
                if sink.send(Frame::Control(msg)).await.is_err() {
                    return PumpEnd::Disconnected; // socket closed under us
                }
            }
            Io::Incoming(Ok(frame)) => {
                keepalive.on_daemon_frame(Instant::now());
                let msg = match frame {
                    Frame::Control(dm) => Message::Connection(ConnectionMsg::Event(dm)),
                    Frame::Grid(frame) => Message::Connection(ConnectionMsg::GridFrame(frame)),
                };
                if output.send(msg).await.is_err() {
                    return PumpEnd::AppGone;
                }
            }
            Io::Incoming(Err(_)) => return PumpEnd::Disconnected, // decode error / EOF
            Io::Tick => match keepalive.poll(Instant::now()) {
                KeepaliveAction::SendPing => {
                    // Nonce is diagnostic only; the deadline is reset by *any* inbound frame, so we
                    // don't need to correlate the matching Pong.
                    if sink
                        .send(Frame::Control(ClientMsg::Ping { nonce: 0 }))
                        .await
                        .is_err()
                    {
                        return PumpEnd::Disconnected;
                    }
                }
                KeepaliveAction::Expired => return PumpEnd::Disconnected, // half-open (FR-026)
                KeepaliveAction::Idle => {}
            },
        }
    }

    PumpEnd::Disconnected
}

/// Report a connect failure to the app and map it to a disconnect (so the outer loop retries). If the
/// app is gone, that surfaces as `AppGone` instead.
async fn report_connect_failure(output: &mut mpsc::Sender<Message>, reason: String) -> PumpEnd {
    if output
        .send(Message::Connection(ConnectionMsg::ConnectFailed(reason)))
        .await
        .is_err()
    {
        PumpEnd::AppGone
    } else {
        PumpEnd::Disconnected
    }
}
