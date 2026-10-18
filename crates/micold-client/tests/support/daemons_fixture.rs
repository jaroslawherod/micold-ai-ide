//! One host daemon and one container daemon with bound worktrees and sessions, for the Daemons
//! feature tests (feature 491, T030/T031).

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use micold_client::features::daemons::Model;
use micold_client::links::DaemonLinks;
use micold_core::daemons::{
    os_free_loopback_port, ContainerSettings, DaemonId, DaemonRegistry, DaemonRuntime, WorktreeRef,
};
use micold_core::sandbox::SandboxProfile;
use micold_core::session::SessionId;

pub fn container(name: &str, port: u16) -> DaemonRuntime {
    DaemonRuntime::Container(ContainerSettings {
        profile: SandboxProfile::default(),
        container_name: name.into(),
        port,
        migration: None,
    })
}

/// A Kubernetes daemon's runtime on context `ctx`, namespace `ns`.
pub fn kubernetes(ctx: &str, ns: &str) -> DaemonRuntime {
    DaemonRuntime::Kubernetes(micold_core::kube::settings::KubeSettings::new(
        ctx,
        ns,
        &[micold_core::session::AiCli::ClaudeCode],
    ))
}

pub struct World {
    pub registry: DaemonRegistry,
    pub links: DaemonLinks,
    pub bindings: BTreeMap<PathBuf, BTreeMap<String, DaemonId>>,
    pub locations: Vec<WorktreeRef>,
    pub sessions: BTreeMap<DaemonId, Vec<SessionId>>,
    pub host: DaemonId,
    pub boxed: DaemonId,
}

pub fn wt(project: &str, worktree: &str) -> WorktreeRef {
    WorktreeRef {
        project: project.into(),
        worktree: worktree.into(),
    }
}

impl World {
    /// Host bound to `/p/a` Default; Box bound to `/p/a` `wt1` and `/p/b` Default. Both stopped.
    pub fn new() -> Self {
        let mut registry = DaemonRegistry::new(Vec::new(), 1);
        let host = registry.add("Host", DaemonRuntime::Host, false).unwrap();
        let boxed = registry
            .add("Box", container("box-ctr", 7001), false)
            .unwrap();
        let links = DaemonLinks::at_launch(&registry);
        let mut bindings: BTreeMap<PathBuf, BTreeMap<String, DaemonId>> = BTreeMap::new();
        bindings
            .entry("/p/a".into())
            .or_default()
            .insert("".into(), host);
        bindings
            .entry("/p/a".into())
            .or_default()
            .insert("wt1".into(), boxed);
        bindings
            .entry("/p/b".into())
            .or_default()
            .insert("".into(), boxed);
        let locations = vec![wt("/p/a", ""), wt("/p/a", "wt1"), wt("/p/b", "")];
        let mut sessions = BTreeMap::new();
        sessions.insert(host, vec![SessionId::new()]);
        sessions.insert(boxed, vec![SessionId::new(), SessionId::new()]);
        Self {
            registry,
            links,
            bindings,
            locations,
            sessions,
            host,
            boxed,
        }
    }

    /// The model with an injected port source and the ports other windows hold.
    pub fn model_with<'a>(
        &'a mut self,
        taken_elsewhere: Vec<u16>,
        next_port: &'a dyn Fn() -> std::io::Result<u16>,
    ) -> Model<'a> {
        Model {
            taken_elsewhere,
            next_port,
            ..self.model()
        }
    }

    pub fn model(&mut self) -> Model<'_> {
        Model {
            registry: &mut self.registry,
            links: &mut self.links,
            bindings: &mut self.bindings,
            legacy_default: None,
            legacy_container: None,
            locations: &self.locations,
            sessions: &self.sessions,
            taken_elsewhere: Vec::new(),
            next_port: &os_free_loopback_port,
        }
    }
}
