//! A real daemon linked to a test orchestrator (#1726): its state, served through its own link
//! task as `micold-daemon` serves it. Shared by the `orchestrator_route` and `orchestrator` suites;
//! each declares the `support` module (`tests/orchestrator/support.rs`) at its root.
#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use micold_core::daemon_config;
use micold_core::orchestrator::LinkStatus;
use micold_daemon::catalog::Catalog;
use micold_daemon::orchestrator_link::{announcement, spawn_with, LinkHandle};
use micold_daemon::state::DaemonState;

use crate::support::{record_when, within, Orchestrator};

/// A real daemon linked to the orchestrator: its state and its link task.
pub struct Daemon {
    pub id: String,
    pub state: Arc<DaemonState>,
    pub link: LinkHandle,
}

/// The configuration of a host daemon named `id` that dials `socket`.
pub fn config(id: &str, socket: &Path) -> Arc<daemon_config::DaemonConfig> {
    let text = format!(
        "schema = 1\n[daemon]\nname = \"Daemon {id}\"\nid = \"{id}\"\nkind = \"host\"\n\
         [orchestrator_link]\nenabled = true\nsocket = \"{}\"\n",
        socket.display()
    );
    Arc::new(
        daemon_config::parse(&text, "daemon.toml")
            .unwrap_or_else(|errors| panic!("a valid test config: {errors:#?}")),
    )
}

impl Daemon {
    /// Start daemon `id` and wait until the orchestrator lists it online.
    pub async fn start(orchestrator: &Orchestrator, id: &str) -> Self {
        let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
        let daemon = Self::link_state(orchestrator, id, state);
        daemon.listed_online(orchestrator).await;
        daemon
    }

    /// Link `state` as daemon `id`, without waiting.
    pub fn link_state(orchestrator: &Orchestrator, id: &str, state: Arc<DaemonState>) -> Self {
        let config = config(id, &orchestrator.socket);
        let link = spawn_with(&config, announcement(&config), Some(Arc::clone(&state)))
            .expect("the link is enabled");
        Self {
            id: id.to_owned(),
            state,
            link,
        }
    }

    pub async fn listed_online(&self, orchestrator: &Orchestrator) {
        record_when(orchestrator, &self.id, |r| r["health"] == "online").await;
        within("the link settles", async {
            while self.link.status() != LinkStatus::Linked {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
    }

    /// Drop the link as a crash would: the task and its socket go, the state stays.
    pub fn drop_link(&self) {
        self.link.abort();
    }
}
