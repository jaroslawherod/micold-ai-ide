//! Shared by the Kubernetes daemon start tests (feature 686): a `FakeCluster`, a scripted pod SSH
//! host and the client's key directory, wired the way the shell wires the live ones.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use micold_client::daemon::{kube_attempt, KubeEnv};
use micold_client::kube::KubeConnector;
use micold_core::connect::Connected;
use micold_core::kube::fake::{FakeCluster, FakeKeyGen};
use micold_core::kube::fake_host::{welcoming_bridge, Host};
use micold_core::kube::settings::KubeSettings;
use micold_core::kube::start::DriftChoice;
use micold_core::kube::Cluster;
use micold_core::provision::{EventSink, ProvisionEvent};

pub const NS: &str = "dev";
pub const CONTEXT: &str = "kind-dev";
pub const NAME: &str = "Build box";
pub const IMAGE: &str = "ghcr.io/acme/ssh:1";

/// The settings of the daemon under test: a stated image, since #806's pinned one is M6's.
pub fn settings() -> KubeSettings {
    let mut s = KubeSettings::new(CONTEXT, NS, &[]);
    s.image = Some(micold_core::sandbox::image::ImageRef::parse(IMAGE).unwrap());
    s
}

/// A cluster with the daemon's namespace.
pub fn cluster() -> Arc<FakeCluster> {
    let fake = Arc::new(FakeCluster::new());
    fake.add_namespace(NS);
    fake
}

/// One client: its key directory, and a pod host that connects and accepts the handshake.
pub struct Client {
    pub root: tempfile::TempDir,
    pub host: Arc<Host>,
    pub env: KubeEnv,
}

pub fn client(cluster: &Arc<FakeCluster>, host: Host) -> Client {
    let root = tempfile::tempdir().unwrap();
    let host = Arc::new(host);
    let shared = cluster.clone();
    let connect: KubeConnector = Arc::new(move |_context: String| {
        let cluster = shared.clone() as Arc<dyn Cluster>;
        Box::pin(async move { Ok(cluster) })
    });
    let env = KubeEnv::new(
        connect,
        Arc::new(FakeKeyGen::default()),
        host.clone(),
        root.path().to_path_buf(),
        root.path().to_path_buf(),
    );
    Client { root, host, env }
}

/// A pod host that has the build and whose daemon welcomes the client.
pub fn welcoming() -> Host {
    Host::with_build().bridge(welcoming_bridge())
}

/// An event sink that keeps what it saw.
pub fn sink() -> (EventSink, Arc<Mutex<Vec<ProvisionEvent>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let keep = seen.clone();
    (Arc::new(move |e| keep.lock().unwrap().push(e)), seen)
}

/// The settings of a daemon added with the default image (none stated: #806's pinned one) and
/// these AI CLIs to install.
pub fn default_image_settings(clis: &[micold_core::session::AiCli]) -> KubeSettings {
    KubeSettings::new(CONTEXT, NS, clis)
}

/// One start of the daemon by `client`.
pub async fn attempt(
    client: &Client,
) -> Result<(Connected, micold_client::kube::relay::Relay), String> {
    attempt_with(client, &settings()).await.0
}

/// One start of a daemon with `settings`, and what its sink saw.
pub async fn attempt_with(
    client: &Client,
    settings: &KubeSettings,
) -> (
    Result<(Connected, micold_client::kube::relay::Relay), String>,
    Arc<Mutex<Vec<ProvisionEvent>>>,
) {
    let (sink, seen) = sink();
    (
        kube_attempt(&client.env, NAME, settings, DriftChoice::Ask, &sink)
            .await
            .map_err(|e| e.to_string()),
        seen,
    )
}
