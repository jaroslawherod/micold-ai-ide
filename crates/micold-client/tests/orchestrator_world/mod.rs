//! The orchestrator world the #1726 tests share: a real `micold-daemon orchestrator` on a temp
//! runtime dir, started by the app's connection itself, and real daemons linked to it in this
//! process (no socket of their own).
#![allow(dead_code)] // each test binary uses part of it

#[path = "../daemon_bin/mod.rs"]
mod daemon_bin;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use micold_client::orchestrator_conn::Starter;
use micold_core::endpoint::Endpoint;
use micold_daemon::catalog::Catalog;
use micold_daemon::orchestrator_link::{announcement, spawn_with, LinkHandle};
use micold_daemon::state::DaemonState;

pub const WITHIN: Duration = Duration::from_secs(20);

pub fn daemon_binary() -> PathBuf {
    daemon_bin::built_daemon().unwrap_or_else(|| {
        panic!(
            "build micold-daemon first; none above {:?}",
            std::env::current_exe()
        )
    })
}

/// A temp runtime and data dir, and every orchestrator the starter started.
pub struct World {
    _root: tempfile::TempDir,
    children: Arc<Mutex<Vec<Child>>>,
    pub starter: Starter,
}

impl World {
    pub fn new(binary: PathBuf) -> Self {
        let root = tempfile::tempdir().expect("temp dir");
        let runtime = root.path().join("run");
        std::fs::create_dir(&runtime).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let dir = runtime.join("micold");
        let endpoint = Endpoint {
            socket_path: dir.join("orchestrator.sock"),
            lock_path: dir.join("orchestrator.lock"),
        };
        let children: Arc<Mutex<Vec<Child>>> = Arc::default();
        let (spawned, home) = (Arc::clone(&children), root.path().to_path_buf());
        let starter = Starter {
            endpoint,
            spawn: Arc::new(move || {
                let child = Command::new(&binary)
                    .arg("orchestrator")
                    .env("XDG_RUNTIME_DIR", home.join("run"))
                    .env("XDG_DATA_HOME", home.join("data"))
                    .env("HOME", &home)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| {
                        std::io::Error::new(
                            e.kind(),
                            format!("cannot start {}: {e}", binary.display()),
                        )
                    })?;
                spawned.lock().unwrap().push(child);
                Ok(())
            }),
        };
        Self {
            _root: root,
            children,
            starter,
        }
    }

    pub fn socket(&self) -> &Path {
        &self.starter.endpoint.socket_path
    }

    /// How many of the started orchestrators still run.
    pub fn running(&self) -> usize {
        self.children
            .lock()
            .unwrap()
            .iter_mut()
            .map(|c| matches!(c.try_wait(), Ok(None)))
            .filter(|running| *running)
            .count()
    }

    pub fn started(&self) -> usize {
        self.children.lock().unwrap().len()
    }

    /// Wait until exactly `n` orchestrators run.
    pub async fn settles_on(&self, n: usize) {
        let deadline = Instant::now() + WITHIN;
        while self.running() != n {
            assert!(
                Instant::now() < deadline,
                "{} orchestrators run, not {n}",
                self.running()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub fn kill_all(&self) {
        for child in self.children.lock().unwrap().iter_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        self.kill_all();
    }
}

/// A real daemon linked to the world's orchestrator under the wire id `id` (a provisioned one).
pub fn link_as(world: &World, id: &str) -> (Arc<DaemonState>, LinkHandle) {
    let text = format!(
        "schema = 1\n[daemon]\nname = \"Box\"\nid = \"{id}\"\nkind = \"ssh\"\n\
         [orchestrator_link]\nenabled = true\nsocket = \"{}\"\n",
        world.socket().display()
    );
    let config =
        Arc::new(micold_core::daemon_config::parse(&text, "daemon.toml").expect("a valid config"));
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let link = spawn_with(&config, announcement(&config), Some(Arc::clone(&state)))
        .expect("the link is enabled");
    (state, link)
}
