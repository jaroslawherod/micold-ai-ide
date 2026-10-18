//! The start sequence of a Kubernetes daemon (feature 686, research K6, contract §2–§3). This
//! milestone runs it through `Tunnel`; `Copy`, `Install` and `Connect` come later.

use std::fmt;
use std::future::Future;
use std::path::Path;
use std::time::Duration;

use tokio::io::AsyncReadExt;

use crate::connect::Connected;
use crate::kube::access::{AccessSecret, KeyGen};
use crate::kube::classify::{CallTarget, KubeFailure};
use crate::kube::credentials::{self, ReadFile};
use crate::kube::manifest::{self, PodStamp, SSH_PORT};
use crate::kube::names::{is_ours, ResourceNames};
use crate::kube::settings::KubeSettings;
use crate::kube::{Cluster, Created, ObjectKind, ObjectView};
use crate::provision::{
    AiCliTable, CancelToken, Egress, EventSink, FailureCause, ProvisionEvent, ProvisionTarget,
    Provisioned, Step, StepBounds,
};
use crate::sandbox::AiCliSelection;
use crate::ssh::argv::{PinnedReach, Reach};
use crate::ssh::provision::{ProvisionOptions, SshHost};
use crate::ssh::{ReasonContext, SshFailure};

/// What an SSH server's identification line starts with (RFC 4253 §4.2).
const SSH_IDENT: &str = "SSH-2.0-";

/// One phase of a start, shown as the row's activity while it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartPhase {
    /// Read the kubeconfig and build the client.
    Config,
    /// Check the namespace and permissions.
    Preflight,
    /// Create or adopt the volume claim.
    Volume,
    /// Create or adopt the access secret and add this client's key.
    Secrets,
    /// Create or adopt the pod.
    Pod,
    /// Wait for the pod to be scheduled.
    Schedule,
    /// Wait for the image to be pulled and the container to run.
    Pull,
    /// Open the port-forward tunnel.
    Tunnel,
    /// Copy the daemon into the volume and start it.
    Copy,
    /// Install the AI CLIs.
    Install,
    /// Connect to the daemon.
    Connect,
}

impl StartPhase {
    /// How long the phase may take before the start fails (FR-012).
    pub fn bound(self) -> Duration {
        match self {
            StartPhase::Pull | StartPhase::Install => Duration::from_secs(15 * 60),
            _ => Duration::from_secs(120),
        }
    }

    /// The row's activity text.
    pub fn activity(self) -> &'static str {
        match self {
            StartPhase::Config => "reading the kubeconfig",
            StartPhase::Preflight => "checking the namespace",
            StartPhase::Volume => "preparing the volume",
            StartPhase::Secrets => "writing the secrets",
            StartPhase::Pod => "creating the pod",
            StartPhase::Schedule => "scheduling the pod",
            StartPhase::Pull => "pulling the image",
            StartPhase::Tunnel => "opening the tunnel",
            StartPhase::Copy => "copying the daemon",
            StartPhase::Install => "installing the AI CLIs",
            StartPhase::Connect => "connecting to the daemon",
        }
    }
}

impl fmt::Display for StartPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.activity())
    }
}

/// How often a phase that waits on the cluster asks again (research K6: a `GET` a second).
pub const POLL: Duration = Duration::from_secs(1);

/// The phases a start runs, in order (data model "StartPlan").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartPlan(Vec<StartPhase>);

impl StartPlan {
    /// The fixed plan of a Kubernetes daemon.
    pub fn kube() -> Self {
        use StartPhase::*;
        Self(vec![
            Config, Preflight, Volume, Secrets, Pod, Schedule, Pull, Tunnel, Copy, Install, Connect,
        ])
    }

    /// The phases.
    pub fn phases(&self) -> &[StartPhase] {
        &self.0
    }
}

/// An object this start attempt created, to delete if the attempt fails (FR-013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedResource {
    /// Its kind.
    pub kind: ObjectKind,
    /// Its name.
    pub name: String,
    /// Its UID, the delete precondition.
    pub uid: String,
}

/// What one start attempt created or added, for rollback. Adopted objects are never in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attempt {
    /// The objects, in creation order.
    pub created: Vec<CreatedResource>,
    /// The client whose key this attempt added to an adopted access secret.
    pub added_key: Option<String>,
}

/// What to do when the adopted pod runs with other settings (FR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftChoice {
    /// Not asked yet: end the start with [`StartOutcome::Drift`].
    Ask,
    /// Delete the pod as Stop does and create a new one.
    Recreate,
    /// Continue with the pod as it is.
    Keep,
}

/// This client's SSH identity for the daemon (its private half stays on the client).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientKey {
    /// The client's id, the key's comment in `authorized_keys`.
    pub id: String,
    /// `ssh-ed25519 <base64>`.
    pub public_key: String,
}

/// Everything one start needs.
pub struct StartRequest<'a> {
    /// The cluster.
    pub cluster: &'a dyn Cluster,
    /// Key generation, for the host key.
    pub keys: &'a dyn KeyGen,
    /// The API server URL, for an unreachable-cluster failure.
    pub server: &'a str,
    /// The daemon's settings.
    pub settings: &'a KubeSettings,
    /// The daemon's object names.
    pub names: &'a ResourceNames,
    /// The image the pod runs: the setting, or the default.
    pub image: &'a str,
    /// This client's key.
    pub client: &'a ClientKey,
    /// What to do about a drifted pod.
    pub drift: DriftChoice,
    /// This machine's home directory, where the allowed credentials' source files are.
    pub home: &'a Path,
    /// The remote hosts whose `known_hosts` lines go with an allowed SSH key.
    pub hosts: &'a [String],
    /// How the `Secrets` phase reads a source file (the only place one is read).
    pub read: ReadFile<'a>,
}

/// A start that reached a running container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    /// The pod's name.
    pub pod: String,
    /// What this start created (empty when it adopted everything).
    pub attempt: Attempt,
}

/// How a start ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartOutcome {
    /// The pod runs.
    Ready(Started),
    /// The pod runs with other settings: ask the user; this attempt was rolled back.
    Drift {
        /// The settings that differ, in form order.
        fields: Vec<&'static str>,
    },
    /// The start failed; this attempt was rolled back.
    Failed(KubeFailure),
}

/// Run one phase: report it as the activity, then fail `PhaseTimeout` at its bound.
pub async fn run_phase<T>(
    phase: StartPhase,
    on_phase: &mut (dyn FnMut(StartPhase) + Send),
    work: impl Future<Output = Result<T, KubeFailure>>,
) -> Result<T, KubeFailure> {
    on_phase(phase);
    tokio::time::timeout(phase.bound(), work)
        .await
        .unwrap_or(Err(KubeFailure::PhaseTimeout { phase }))
}

/// A start that failed in `Config`, before there was a cluster to call: the phase is reported and
/// nothing was created, so nothing is rolled back.
pub fn config_failed(
    failure: KubeFailure,
    on_phase: &mut (dyn FnMut(StartPhase) + Send),
) -> StartOutcome {
    on_phase(StartPhase::Config);
    StartOutcome::Failed(failure)
}

/// Start the daemon: every phase after `Config` up to `Tunnel`. A failure, or a drift to ask about,
/// first undoes what this attempt created (FR-013).
pub async fn start(
    request: &StartRequest<'_>,
    on_phase: &mut (dyn FnMut(StartPhase) + Send),
) -> StartOutcome {
    let mut attempt = Attempt::default();
    let outcome = sequence(request, on_phase, &mut attempt).await;
    match outcome {
        Ok(fields) if fields.is_empty() => StartOutcome::Ready(Started {
            pod: request.names.pod(),
            attempt,
        }),
        Ok(fields) => {
            rollback(request, &attempt).await;
            StartOutcome::Drift { fields }
        }
        Err(failure) => {
            rollback(request, &attempt).await;
            StartOutcome::Failed(failure.at(&CallTarget {
                context: &request.settings.context.0,
                server: request.server,
                namespace: namespace(request),
            }))
        }
    }
}

/// The phases in order; the settings that drifted when the user is to be asked, else empty.
async fn sequence(
    r: &StartRequest<'_>,
    on_phase: &mut (dyn FnMut(StartPhase) + Send),
    attempt: &mut Attempt,
) -> Result<Vec<&'static str>, KubeFailure> {
    use StartPhase::*;
    run_phase(Preflight, on_phase, preflight(r)).await?;
    run_phase(Volume, on_phase, volume(r, attempt)).await?;
    run_phase(Secrets, on_phase, secrets(r, attempt)).await?;
    let drifted = run_phase(Pod, on_phase, pod(r, attempt)).await?;
    if !drifted.is_empty() {
        return Ok(drifted);
    }
    run_phase(
        Schedule,
        on_phase,
        await_pod(r, attempt, |s| s.scheduled == Some(true) || s.running),
    )
    .await?;
    run_phase(Pull, on_phase, await_pod(r, attempt, |s| s.running)).await?;
    on_phase(Tunnel);
    tunnel(r).await?;
    Ok(Vec::new())
}

fn namespace<'a>(r: &'a StartRequest<'_>) -> &'a str {
    &r.settings.namespace.0
}

/// The namespace exists and the user may do everything the start does, checked before anything is
/// created (US4.1).
async fn preflight(r: &StartRequest<'_>) -> Result<(), KubeFailure> {
    if !r.cluster.namespace_exists(namespace(r)).await? {
        return Err(KubeFailure::NamespaceMissing {
            namespace: namespace(r).to_string(),
        });
    }
    for (verb, resource) in manifest::access_reviews() {
        if !r
            .cluster
            .access_review(namespace(r), verb, resource)
            .await?
        {
            return Err(KubeFailure::Forbidden {
                verb: verb.to_string(),
                resource: resource.to_string(),
                namespace: namespace(r).to_string(),
            });
        }
    }
    Ok(())
}

/// How an object of the daemon's name came to be there.
enum Ensured {
    Created,
    Adopted(ObjectView),
}

/// Create `manifest`, or adopt the object of that name when it is ours (contract §2).
async fn ensure(
    r: &StartRequest<'_>,
    kind: ObjectKind,
    manifest: serde_json::Value,
    attempt: &mut Attempt,
) -> Result<Ensured, KubeFailure> {
    let name = manifest["metadata"]["name"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let ns = namespace(r);
    loop {
        if let Created::Created { uid } = r.cluster.create(ns, kind, manifest.clone()).await? {
            attempt.created.push(CreatedResource { kind, name, uid });
            return Ok(Ensured::Created);
        }
        match r.cluster.get(ns, kind, &name).await? {
            // Gone between the create and the read: create again.
            None => {}
            Some(view) => {
                require_ours(r, kind, &name, &view)?;
                if !view.deleting {
                    return Ok(Ensured::Adopted(view));
                }
                // Being deleted: wait it out (within the phase's bound), then create (contract §2).
                tokio::time::sleep(POLL).await;
            }
        }
    }
}

fn require_ours(
    r: &StartRequest<'_>,
    kind: ObjectKind,
    name: &str,
    view: &ObjectView,
) -> Result<(), KubeFailure> {
    if is_ours(&view.labels, r.names.key()) {
        return Ok(());
    }
    Err(KubeFailure::Conflict {
        kind: kind.noun().to_string(),
        name: name.to_string(),
        namespace: namespace(r).to_string(),
    })
}

async fn volume(r: &StartRequest<'_>, attempt: &mut Attempt) -> Result<(), KubeFailure> {
    let claim = manifest::pvc(r.settings, r.names);
    if let Ensured::Created = ensure(r, ObjectKind::PersistentVolumeClaim, claim, attempt).await? {
        // A provisioner that binds at once and fails shows it now; one that waits for the pod
        // shows it while the pod is awaited.
        check_claim(r).await?;
    }
    Ok(())
}

/// Fail `ClaimNotBound` when the claim's events show the provisioner gave up.
async fn check_claim(r: &StartRequest<'_>) -> Result<(), KubeFailure> {
    let claim = r.names.pvc();
    let events = r.cluster.events(namespace(r), &claim).await?;
    KubeFailure::from_pvc(&claim, &events).map_or(Ok(()), Err)
}

/// The `Secrets` phase: the access secret, then the allowed credentials' secrets (FR-008).
async fn secrets(r: &StartRequest<'_>, attempt: &mut Attempt) -> Result<(), KubeFailure> {
    access_secret(r, attempt).await?;
    credential_secrets(r, attempt).await
}

/// Create or refresh one secret per allowed credential, and delete the secrets of withdrawn ones
/// that are ours. With nothing allowed it only reads, to find what a withdrawal left behind.
async fn credential_secrets(
    r: &StartRequest<'_>,
    attempt: &mut Attempt,
) -> Result<(), KubeFailure> {
    for credential in &r.settings.allowed {
        let name = r.names.secret(credential);
        let data = credential.secret_data(r.home, r.hosts, r.read)?;
        let manifest = manifest::typed_secret(r.names, &name, credential.secret_type(), &data);
        if let Ensured::Adopted(view) =
            ensure(r, ObjectKind::Secret, manifest.clone(), attempt).await?
        {
            // The source may have changed since the secret was written.
            if view.data != data {
                let version = &view.resource_version;
                r.cluster
                    .replace(namespace(r), ObjectKind::Secret, &name, version, manifest)
                    .await?;
            }
        }
    }
    for kind in credentials::every_kind() {
        let name = r.names.secret(&kind);
        let allowed = r.settings.allowed.iter().any(|a| r.names.secret(a) == name);
        if allowed {
            continue;
        }
        if let Some(view) = r
            .cluster
            .get(namespace(r), ObjectKind::Secret, &name)
            .await?
        {
            if is_ours(&view.labels, r.names.key()) {
                delete_and_wait(r, ObjectKind::Secret, &name, &view.uid).await?;
            }
        }
    }
    Ok(())
}

/// The access secret, created with a generated host key, or this client's key added to it.
async fn access_secret(r: &StartRequest<'_>, attempt: &mut Attempt) -> Result<(), KubeFailure> {
    let name = r.names.access_secret();
    let view = match r
        .cluster
        .get(namespace(r), ObjectKind::Secret, &name)
        .await?
    {
        Some(view) => {
            require_ours(r, ObjectKind::Secret, &name, &view)?;
            view
        }
        None => {
            let fresh = fresh_secret(r)
                .with_client(&r.client.id, &r.client.public_key)
                .0;
            let manifest = manifest::secret(r.names, &name, &fresh.to_data());
            match ensure(r, ObjectKind::Secret, manifest, attempt).await? {
                Ensured::Created => return Ok(()),
                Ensured::Adopted(view) => view,
            }
        }
    };
    let id = &r.client.id;
    let had = AccessSecret::parse(&view.data)
        .is_some_and(|s| s.authorized.iter().any(|k| &k.client_id == id));
    let wrote = update_access(r, view, |current| {
        let (updated, changed) = current
            .clone()
            .unwrap_or_else(|| fresh_secret(r))
            .with_client(id, &r.client.public_key);
        (changed || current.is_none()).then_some(updated)
    })
    .await?;
    // Rollback removes the client's line only when this attempt gave it one.
    if wrote && !had {
        attempt.added_key = Some(id.clone());
    }
    Ok(())
}

/// Write the access secret as `change` makes it from the content read (`None` when unreadable);
/// `change` returns `None` when nothing needs writing. The write is guarded by the
/// `resourceVersion` read; on `409` the secret is read again and `change` applied to the new
/// content, so another client's key survives (FR-023). Whether it wrote.
async fn update_access(
    r: &StartRequest<'_>,
    mut view: ObjectView,
    change: impl Fn(Option<AccessSecret>) -> Option<AccessSecret>,
) -> Result<bool, KubeFailure> {
    let name = r.names.access_secret();
    for _ in 0..KEY_WRITE_TRIES {
        let Some(updated) = change(AccessSecret::parse(&view.data)) else {
            return Ok(false);
        };
        let manifest = manifest::secret(r.names, &name, &updated.to_data());
        let version = &view.resource_version;
        let write = r
            .cluster
            .replace(namespace(r), ObjectKind::Secret, &name, version, manifest);
        match write.await {
            Ok(()) => return Ok(true),
            Err(failure) if failure.code == 409 => {
                view = r
                    .cluster
                    .get(namespace(r), ObjectKind::Secret, &name)
                    .await?
                    .ok_or_else(|| KubeFailure::from(failure))?;
            }
            Err(failure) => return Err(failure.into()),
        }
    }
    Err(KubeFailure::Api {
        status: 409,
        message: format!("secret {name} kept changing; this client's key could not be written"),
    })
}

/// How often a key write retries after a conflicting write by another client.
const KEY_WRITE_TRIES: usize = 8;

fn fresh_secret(r: &StartRequest<'_>) -> AccessSecret {
    AccessSecret {
        host_key: r
            .keys
            .generate(&format!("micold-host-{}", r.names.daemon_name())),
        authorized: Vec::new(),
    }
}

/// Create or adopt the pod; the drifted settings when the user is to be asked.
async fn pod(
    r: &StartRequest<'_>,
    attempt: &mut Attempt,
) -> Result<Vec<&'static str>, KubeFailure> {
    let stamp = PodStamp::of(r.settings, r.image);
    loop {
        let manifest = manifest::pod(r.settings, r.names, r.image, &stamp);
        let view = match ensure(r, ObjectKind::Pod, manifest, attempt).await? {
            Ensured::Created => return Ok(Vec::new()),
            Ensured::Adopted(view) => view,
        };
        let on_pod = view.annotations.get(manifest::POD_STAMP_ANNOTATION);
        let fields = manifest::drift(on_pod.map_or("", String::as_str), &stamp);
        match r.drift {
            _ if fields.is_empty() => return Ok(Vec::new()),
            DriftChoice::Keep => return Ok(Vec::new()),
            DriftChoice::Ask => return Ok(fields),
            DriftChoice::Recreate => {
                delete_and_wait(r, ObjectKind::Pod, &r.names.pod(), &view.uid).await?
            }
        }
    }
}

/// Delete by name and UID and wait until the object is gone (as Stop does).
async fn delete_and_wait(
    r: &StartRequest<'_>,
    kind: ObjectKind,
    name: &str,
    uid: &str,
) -> Result<(), KubeFailure> {
    r.cluster.delete(namespace(r), kind, name, uid).await?;
    while r.cluster.get(namespace(r), kind, name).await?.is_some() {
        tokio::time::sleep(POLL).await;
    }
    Ok(())
}

/// Poll the pod's status until `done` accepts it, failing on first sight of a failure the
/// cluster shows (FR-012).
async fn await_pod(
    r: &StartRequest<'_>,
    attempt: &Attempt,
    done: impl Fn(&crate::kube::PodStatusView) -> bool,
) -> Result<(), KubeFailure> {
    let pod = r.names.pod();
    loop {
        match r.cluster.pod_status(namespace(r), &pod).await? {
            None => {
                return Err(KubeFailure::Api {
                    status: 404,
                    message: format!("pod {pod} is gone"),
                })
            }
            Some(status) if done(&status) => return Ok(()),
            Some(status) => {
                // A failure the cluster already shows is reported now, not at the phase bound.
                if let Some(failure) = KubeFailure::from_pod(&status, r.image, &r.names.pvc()) {
                    return Err(failure);
                }
                // Only a claim this attempt created: an older claim's events may be stale.
                if attempt
                    .created
                    .iter()
                    .any(|c| c.kind == ObjectKind::PersistentVolumeClaim)
                {
                    check_claim(r).await?;
                }
                tokio::time::sleep(POLL).await;
            }
        }
    }
}

/// The longest identification line read; a longer one is not an SSH server's (RFC 4253 §4.2).
const BANNER_MAX: usize = 255;

/// Open one port-forward to the pod's SSH port and read its identification line. Unlike the other
/// phases, running out of the bound is an [`KubeFailure::ImageContract`], not a `PhaseTimeout`.
async fn tunnel(r: &StartRequest<'_>) -> Result<(), KubeFailure> {
    let (pod, ns) = (r.names.pod(), namespace(r));
    let contract = |missing: String| KubeFailure::ImageContract {
        image: r.image.to_string(),
        missing,
    };
    // A pod that just started may not have its SSH server listening yet: the stream then closes
    // at once, so open another until the bound.
    let read = async {
        loop {
            let mut stream = r.cluster.port_forward(ns, &pod, SSH_PORT).await?;
            let mut line = Vec::new();
            let mut byte = [0u8; 1];
            let mut complete = false;
            while line.len() < BANNER_MAX {
                match stream.read(&mut byte).await {
                    Ok(1) if byte[0] == b'\n' => {
                        complete = true;
                        break;
                    }
                    Ok(1) => line.push(byte[0]),
                    _ => break,
                }
            }
            if complete || line.len() >= BANNER_MAX {
                return Ok::<_, KubeFailure>(Some(
                    String::from_utf8_lossy(&line).trim_end().to_string(),
                ));
            }
            tokio::time::sleep(POLL).await;
        }
    };
    let line = tokio::time::timeout(StartPhase::Tunnel.bound(), read).await;
    match line {
        Ok(Err(failure)) => Err(failure),
        Ok(Ok(Some(line))) if line.starts_with(SSH_IDENT) => Ok(()),
        Ok(Ok(Some(line))) => Err(contract(format!(
            "port {SSH_PORT} answered `{line}`, not an SSH server"
        ))),
        Ok(Ok(None)) | Err(_) => {
            let status = r.cluster.pod_status(ns, &pod).await?;
            Err(contract(match status {
                Some(s) if !s.running => {
                    let reason = s.waiting.map_or(s.phase, |(reason, _)| reason);
                    format!("the container exited: {reason}")
                }
                _ => format!("no SSH server answered on port {SSH_PORT}"),
            }))
        }
    }
}

/// The daemon's state directory inside the pod, on the volume (a pod path: never `Path::join`ed).
pub const STATE_DIR: &str = "/var/lib/micold-ai-ide";

/// The daemon's environment: it outlives the client (FR-010), so its idle stop is off.
pub fn daemon_env() -> Vec<(String, String)> {
    vec![(
        crate::spawn::IDLE_STOP_ENV.to_string(),
        crate::spawn::IDLE_STOP_OFF.to_string(),
    )]
}

/// The shared sequence's target for the pod: SSH through the relay's loopback port, AI CLIs
/// installed by M6 (none selected here), the 120 s step bounds and the 15 minute install.
pub fn pod_target(reach: PinnedReach) -> ProvisionTarget {
    ProvisionTarget {
        reach: Reach::Pinned(reach),
        owned: crate::provision::Ownership::AppOwned {
            ai_clis: AiCliSelection(Default::default()),
            table: AiCliTable::pinned(),
            state_dir: STATE_DIR.into(),
            egress: Egress::Direct,
        },
        bounds: StepBounds {
            install: StartPhase::Install.bound(),
            ..StepBounds::container()
        },
        env: daemon_env(),
    }
}

/// Everything the `Copy`, `Install` and `Connect` phases need besides the pod.
pub struct ConnectRequest<'a> {
    /// Runs `ssh`/`scp` toward the relay's loopback port.
    pub host: &'a dyn SshHost,
    /// The target ([`pod_target`]).
    pub target: &'a ProvisionTarget,
    /// The client's platform and build.
    pub opts: &'a ProvisionOptions,
    /// Stops the sequence.
    pub cancel: &'a CancelToken,
}

/// How the `Copy`/`Install`/`Connect` phases ended.
pub enum ConnectOutcome {
    /// The daemon accepted the handshake (a refusal is [`ConnectOutcome::Failed`]).
    Ready(Box<Provisioned>),
    /// The start failed; this attempt was rolled back.
    Failed(KubeFailure),
}

/// The phase a provisioning step belongs to.
fn phase_of(step: Step) -> StartPhase {
    match step {
        Step::InstallAiClis => StartPhase::Install,
        Step::Start | Step::VersionCheck => StartPhase::Connect,
        _ => StartPhase::Copy,
    }
}

/// A provisioning failure as the start's: a changed host key names the pod (US4), anything else
/// carries 806's reason text.
fn provision_failure(
    failure: &crate::provision::ProvisionFailure,
    c: &ConnectRequest<'_>,
    pod: &str,
) -> KubeFailure {
    match &failure.cause {
        FailureCause::Cancelled => KubeFailure::Cancelled,
        FailureCause::Ssh(SshFailure::HostKeyChanged | SshFailure::HostKeyUnknown) => {
            KubeFailure::HostKeyMismatch {
                pod: pod.to_string(),
            }
        }
        _ => {
            let (host, port) = match &c.target.reach {
                Reach::Pinned(p) => (p.host.clone(), p.port),
                Reach::UserConfig(t) => (t.host.clone(), t.port.unwrap_or(22)),
            };
            KubeFailure::Provision {
                reason: failure.reason(&ReasonContext {
                    host,
                    port,
                    timeout_secs: c.opts.timeout_secs,
                    platform: c.opts.platform,
                }),
            }
        }
    }
}

/// Copy the daemon, install the AI CLIs and connect, after [`start`] returned `Ready` and the
/// relay is open: one `provision::run` call, its steps shown as the `Copy`, `Install` and
/// `Connect` phases. A failure undoes what `started` created (FR-013). `on_event` sees every
/// provisioning event (copy percentage, installer lines) for the daemon's diagnostics.
pub async fn connect_pod(
    request: &StartRequest<'_>,
    started: &Started,
    c: &ConnectRequest<'_>,
    on_phase: &mut (dyn FnMut(StartPhase) + Send),
    on_event: &mut (dyn FnMut(&ProvisionEvent) + Send),
) -> ConnectOutcome {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let sink: EventSink = std::sync::Arc::new(move |event| {
        let _ = tx.send(event);
    });
    on_phase(StartPhase::Copy);
    let mut shown = Some(StartPhase::Copy);
    let mut show = |event: ProvisionEvent, on_phase: &mut (dyn FnMut(StartPhase) + Send)| {
        if let ProvisionEvent::Entered(step) = &event {
            let phase = phase_of(*step);
            if shown != Some(phase) {
                shown = Some(phase);
                on_phase(phase);
            }
        }
        on_event(&event);
    };
    let result = {
        let work = crate::provision::run(c.host, c.target, c.opts, c.cancel, &sink);
        tokio::pin!(work);
        loop {
            tokio::select! {
                done = &mut work => break done,
                Some(event) = rx.recv() => show(event, on_phase),
            }
        }
    };
    drop(sink);
    while let Ok(event) = rx.try_recv() {
        show(event, on_phase);
    }
    let failure = match result {
        Ok(done) if matches!(done.connected, Connected::Ready(..)) => {
            return ConnectOutcome::Ready(Box::new(done));
        }
        Ok(done) => {
            let Connected::Refused(reason) = done.connected else {
                unreachable!("matched above")
            };
            KubeFailure::VersionMismatch {
                reason: format!("the daemon in the pod refused this client: {reason:?}"),
            }
        }
        Err(failure) => provision_failure(&failure, c, &started.pod),
    };
    rollback(request, &started.attempt).await;
    ConnectOutcome::Failed(failure.at(&CallTarget {
        context: &request.settings.context.0,
        server: request.server,
        namespace: namespace(request),
    }))
}

/// Undo what the attempt created, newest first, and the key it added to an adopted secret.
/// Best effort: a failure here must not hide the failure that caused the rollback.
pub async fn rollback(r: &StartRequest<'_>, attempt: &Attempt) {
    for made in attempt.created.iter().rev() {
        let _ = r
            .cluster
            .delete(namespace(r), made.kind, &made.name, &made.uid)
            .await;
    }
    if let Some(id) = &attempt.added_key {
        remove_key(r, id).await;
    }
}

async fn remove_key(r: &StartRequest<'_>, id: &str) {
    let name = r.names.access_secret();
    if let Ok(Some(view)) = r.cluster.get(namespace(r), ObjectKind::Secret, &name).await {
        let _ = update_access(r, view, |current| {
            current
                .filter(|s| s.authorized.iter().any(|k| k.client_id == id))
                .map(|s| s.without_client(id))
        })
        .await;
    }
}

#[cfg(test)]
#[path = "start_tests.rs"]
mod tests;
