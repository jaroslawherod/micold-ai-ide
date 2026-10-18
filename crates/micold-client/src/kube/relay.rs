//! The loopback relay between the client's SSH connection and the pod's SSH port (feature 686,
//! FR-006, research K0). It listens on `127.0.0.1` only, and each accepted connection opens one API
//! port-forward stream and copies bytes both ways until either side ends.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use micold_core::kube::Cluster;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// A running relay. Dropping it closes the listener and every connection it carries.
pub struct Relay {
    addr: SocketAddr,
    task: JoinHandle<()>,
}

impl Relay {
    /// Listen on a free loopback port and relay each connection to `port` of `pod`.
    pub async fn open(
        cluster: Arc<dyn Cluster>,
        namespace: String,
        pod: String,
        port: u16,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let addr = listener.local_addr()?;
        let task = tokio::spawn(async move {
            // Connections live in this set, so aborting the task drops them with it.
            let mut connections = tokio::task::JoinSet::new();
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let (cluster, namespace, pod) = (cluster.clone(), namespace.clone(), pod.clone());
                connections.spawn(async move {
                    // A refused port-forward ends this connection only.
                    if let Ok(mut stream) = cluster.port_forward(&namespace, &pod, port).await {
                        let _ = tokio::io::copy_bidirectional(&mut socket, &mut stream).await;
                    }
                });
            }
        });
        Ok(Self { addr, task })
    }

    /// The loopback address SSH connects to.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::kube::fake::FakeCluster;
    use micold_core::kube::manifest::SSH_PORT;
    use micold_core::kube::ObjectKind;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    async fn relay_over(fake: &Arc<FakeCluster>) -> Relay {
        fake.create(
            "default",
            ObjectKind::Pod,
            serde_json::json!({"kind": "Pod", "metadata": {"name": "p"}}),
        )
        .await
        .unwrap();
        fake.set_banner(None);
        Relay::open(fake.clone(), "default".into(), "p".into(), SSH_PORT)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn it_listens_on_loopback_only_on_a_free_port() {
        let fake = Arc::new(FakeCluster::new());
        let relay = relay_over(&fake).await;
        assert!(relay.addr().ip().is_loopback());
        assert_ne!(relay.addr().port(), 0);
    }

    #[tokio::test]
    async fn a_connection_opens_one_port_forward_and_copies_both_ways() {
        let fake = Arc::new(FakeCluster::new());
        let relay = relay_over(&fake).await;
        let mut client = TcpStream::connect(relay.addr()).await.unwrap();
        client.write_all(b"ping").await.unwrap();
        let mut daemon = loop {
            if let Some(peer) = fake.take_tunnel_peer() {
                break peer;
            }
            tokio::task::yield_now().await;
        };
        let mut got = [0u8; 4];
        daemon.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"ping");
        daemon.write_all(b"pong").await.unwrap();
        client.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"pong");
        let forwards = fake
            .calls()
            .iter()
            .filter(|c| c.starts_with("port_forward p 2222"))
            .count();
        assert_eq!(forwards, 1);
    }

    #[tokio::test]
    async fn two_connections_open_two_streams() {
        let fake = Arc::new(FakeCluster::new());
        let relay = relay_over(&fake).await;
        let mut a = TcpStream::connect(relay.addr()).await.unwrap();
        let mut b = TcpStream::connect(relay.addr()).await.unwrap();
        a.write_all(b"a").await.unwrap();
        b.write_all(b"b").await.unwrap();
        let mut seen = Vec::new();
        while seen.len() < 2 {
            match fake.take_tunnel_peer() {
                Some(mut peer) => {
                    let mut byte = [0u8; 1];
                    peer.read_exact(&mut byte).await.unwrap();
                    seen.push(byte[0]);
                }
                None => tokio::task::yield_now().await,
            }
        }
        seen.sort();
        assert_eq!(seen, [b'a', b'b']);
    }

    #[tokio::test]
    async fn dropping_the_relay_closes_the_listener_and_its_connections() {
        let fake = Arc::new(FakeCluster::new());
        let relay = relay_over(&fake).await;
        let addr = relay.addr();
        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(b"x").await.unwrap();
        while fake.calls().iter().all(|c| !c.starts_with("port_forward")) {
            tokio::task::yield_now().await;
        }
        drop(relay);
        let mut rest = Vec::new();
        client.read_to_end(&mut rest).await.unwrap_or(0);
        assert!(rest.is_empty());
        assert!(TcpStream::connect(addr).await.is_err());
    }

    #[tokio::test]
    async fn a_refused_port_forward_closes_only_that_connection() {
        let fake = Arc::new(FakeCluster::new());
        let relay = relay_over(&fake).await;
        fake.fail_next(
            "port_forward",
            micold_core::kube::ApiFailure {
                code: 403,
                reason: "Forbidden".into(),
                message: "no".into(),
            },
        );
        let mut refused = TcpStream::connect(relay.addr()).await.unwrap();
        let mut rest = Vec::new();
        refused.read_to_end(&mut rest).await.unwrap_or(0);
        assert!(rest.is_empty(), "closed");
        let mut next = TcpStream::connect(relay.addr()).await.unwrap();
        next.write_all(b"ok").await.unwrap();
        let mut peer = loop {
            if let Some(peer) = fake.take_tunnel_peer() {
                break peer;
            }
            tokio::task::yield_now().await;
        };
        let mut got = [0u8; 2];
        peer.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"ok");
    }
}
