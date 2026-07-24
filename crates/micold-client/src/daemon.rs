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

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream, StreamExt};
use iced::Subscription;

use micold_core::connect::{connect_or_spawn, Connected, SPAWN_TIMEOUT};
use micold_core::endpoint;
use micold_core::protocol::codec::{CodecError, Frame};
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
    fn new(tx: mpsc::UnboundedSender<ClientMsg>) -> Self {
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

/// One inbound event of the bidirectional pump, unifying the two source streams so a single loop
/// can service both without a `select!` (which would need fused, pinned futures).
enum Io {
    /// A message the App wants sent to the daemon.
    Outgoing(ClientMsg),
    /// A frame that arrived from the daemon.
    Incoming(Result<Frame<DaemonMsg>, CodecError>),
}

fn actor() -> impl Stream<Item = Message> {
    iced::stream::channel(CHANNEL_CAPACITY, |mut output| async move {
        // Resolve the endpoint and connect, spawning the daemon on a cold start (connect_or_spawn
        // owns the spawn+poll-until-accepting dance internally).
        let endpoint = match endpoint::resolve() {
            Ok(e) => e,
            Err(err) => {
                let _ = output
                    .send(Message::DaemonConnectFailed(err.to_string()))
                    .await;
                return;
            }
        };
        let connected = match connect_or_spawn(&endpoint, CLIENT_BUILD, SPAWN_TIMEOUT).await {
            Ok(c) => c,
            Err(err) => {
                let _ = output
                    .send(Message::DaemonConnectFailed(err.to_string()))
                    .await;
                return;
            }
        };
        let (conn, welcome) = match connected {
            Connected::Ready(conn, welcome) => (conn, welcome),
            Connected::Refused(reason) => {
                let _ = output
                    .send(Message::DaemonConnectFailed(format!(
                        "daemon refused the connection: {reason:?}"
                    )))
                    .await;
                return;
            }
        };

        // Hand the App an Outbox and the welcome state, then pump both directions.
        let (mut sink, incoming) = conn.split();
        let (tx, rx) = mpsc::unbounded::<ClientMsg>();
        if output
            .send(Message::DaemonConnected {
                outbox: Outbox::new(tx),
                catalog: welcome.catalog,
                settings: welcome.settings,
            })
            .await
            .is_err()
        {
            return; // the app is gone
        }

        let mut events =
            iced::futures::stream::select(rx.map(Io::Outgoing), incoming.map(Io::Incoming));
        while let Some(io) = events.next().await {
            match io {
                Io::Outgoing(msg) => {
                    if sink.send(Frame::Control(msg)).await.is_err() {
                        break; // socket closed under us
                    }
                }
                Io::Incoming(Ok(Frame::Control(dm))) => {
                    if output.send(Message::DaemonEvent(dm)).await.is_err() {
                        break;
                    }
                }
                Io::Incoming(Ok(Frame::Grid(frame))) => {
                    if output.send(Message::DaemonGridFrame(frame)).await.is_err() {
                        break;
                    }
                }
                Io::Incoming(Err(_)) => break, // decode error / EOF — treat as disconnect
            }
        }

        let _ = output.send(Message::DaemonDisconnected).await;
    })
}
