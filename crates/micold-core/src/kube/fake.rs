//! A scriptable in-memory [`Cluster`] for sequence tests (feature 686). Compiled for core tests and,
//! through the `kube-fake` feature, for client tests.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::Mutex;

use futures_util::FutureExt;
use tokio::io::{AsyncWriteExt, DuplexStream};

use super::AsyncReadWrite;
use super::{
    ApiFailure, Cluster, ClusterFuture, Created, EventView, ObjectKind, ObjectView, PodStatusView,
};

type Key = (String, ObjectKind, String);

#[derive(Default)]
struct Inner {
    objects: BTreeMap<Key, (ObjectView, serde_json::Value)>,
    namespaces: BTreeSet<String>,
    denied: BTreeSet<(String, String)>,
    failures: HashMap<&'static str, VecDeque<ApiFailure>>,
    held: BTreeSet<Key>,
    progressions: HashMap<String, VecDeque<PodStatusView>>,
    events: HashMap<String, Vec<EventView>>,
    logs: HashMap<String, String>,
    tunnels: VecDeque<DuplexStream>,
    banner: Option<Vec<u8>>,
    calls: Vec<String>,
    next_uid: u64,
    next_version: u64,
}

/// An in-memory cluster: objects keyed by namespace, kind and name; per-operation scripted
/// failures; pod phase progressions; a call log; tunnels built from `tokio::io::duplex`.
#[derive(Default)]
pub struct FakeCluster {
    inner: Mutex<Inner>,
}

/// The status of a pod that is scheduled and running.
pub fn running() -> PodStatusView {
    PodStatusView {
        phase: "Running".into(),
        scheduled: Some(true),
        running: true,
        ..PodStatusView::default()
    }
}

/// A `404 NotFound` failure.
pub fn not_found(name: &str) -> ApiFailure {
    ApiFailure {
        code: 404,
        reason: "NotFound".into(),
        message: format!("\"{name}\" not found"),
    }
}

impl FakeCluster {
    /// A cluster with the namespace `default` only.
    pub fn new() -> Self {
        let fake = Self::default();
        fake.add_namespace("default");
        fake.set_banner(Some("SSH-2.0-FakeSshd\r\n"));
        fake
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Add a namespace.
    pub fn add_namespace(&self, namespace: &str) {
        self.lock().namespaces.insert(namespace.into());
    }

    /// Answer `false` to access reviews of `verb` on `resource`.
    pub fn deny(&self, verb: &str, resource: &str) {
        self.lock().denied.insert((verb.into(), resource.into()));
    }

    /// Fail the next call of `op` (`create`, `get`, `delete`, `namespace`, `review`, `pod_status`,
    /// `events`, `replace`, `port_forward`, `logs`) with `failure`. Queued per operation.
    pub fn fail_next(&self, op: &'static str, failure: ApiFailure) {
        self.lock()
            .failures
            .entry(op)
            .or_default()
            .push_back(failure);
    }

    /// Put an object there as if another party had created it.
    pub fn insert(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
        labels: BTreeMap<String, String>,
    ) -> String {
        let mut inner = self.lock();
        let uid = Self::uid(&mut inner);
        let view = ObjectView {
            uid: uid.clone(),
            resource_version: Self::version(&mut inner),
            labels,
            deleting: false,
            storage: None,
            annotations: BTreeMap::new(),
            data: BTreeMap::new(),
        };
        inner.objects.insert(
            (namespace.into(), kind, name.into()),
            (view, serde_json::Value::Null),
        );
        uid
    }

    /// Mark an object as being deleted; it is gone after the next `get` that sees it.
    pub fn mark_deleting(&self, namespace: &str, kind: ObjectKind, name: &str) {
        if let Some((view, _)) = self
            .lock()
            .objects
            .get_mut(&(namespace.into(), kind, name.into()))
        {
            view.deleting = true;
        }
    }

    /// Make a delete of this object only mark it as being deleted, as a finalizer that has not
    /// finished does, until [`Self::release`].
    pub fn hold_on_delete(&self, namespace: &str, kind: ObjectKind, name: &str) {
        self.lock()
            .held
            .insert((namespace.into(), kind, name.into()));
    }

    /// Let a held object go: it is gone after the next `get` that sees it being deleted.
    pub fn release(&self, namespace: &str, kind: ObjectKind, name: &str) {
        self.lock()
            .held
            .remove(&(namespace.into(), kind, name.into()));
    }

    /// The statuses `pod_status` returns for `pod`, one per call; the last one stays.
    pub fn progress(&self, pod: &str, statuses: Vec<PodStatusView>) {
        self.lock().progressions.insert(pod.into(), statuses.into());
    }

    /// Events reported for `name`.
    pub fn set_events(&self, name: &str, events: Vec<EventView>) {
        self.lock().events.insert(name.into(), events);
    }

    /// The log a pod returns.
    pub fn set_logs(&self, pod: &str, log: &str) {
        self.lock().logs.insert(pod.into(), log.into());
    }

    /// What a tunnel's daemon side has already written when it opens: an SSH server's identification
    /// line by default; `None` for a port nothing answers on.
    pub fn set_banner(&self, banner: Option<&str>) {
        self.lock().banner = banner.map(|b| b.as_bytes().to_vec());
    }

    /// The daemon-side half of the oldest tunnel opened and not yet taken.
    pub fn take_tunnel_peer(&self) -> Option<DuplexStream> {
        self.lock().tunnels.pop_front()
    }

    /// Every call so far, as `"<op> <kind> <name>"` lines.
    pub fn calls(&self) -> Vec<String> {
        self.lock().calls.clone()
    }

    /// The manifest an object was created from, `None` when absent or inserted.
    pub fn manifest(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
    ) -> Option<serde_json::Value> {
        self.lock()
            .objects
            .get(&(namespace.into(), kind, name.into()))
            .map(|(_, m)| m.clone())
            .filter(|m| !m.is_null())
    }

    /// The names of the objects of `kind` in `namespace`.
    pub fn names(&self, namespace: &str, kind: ObjectKind) -> Vec<String> {
        self.lock()
            .objects
            .keys()
            .filter(|(ns, k, _)| ns == namespace && *k == kind)
            .map(|(_, _, n)| n.clone())
            .collect()
    }

    fn uid(inner: &mut Inner) -> String {
        inner.next_uid += 1;
        format!("uid-{}", inner.next_uid)
    }

    fn version(inner: &mut Inner) -> String {
        inner.next_version += 1;
        inner.next_version.to_string()
    }

    /// Log the call and take its scripted failure, if any.
    fn enter(
        &self,
        op: &'static str,
        what: String,
    ) -> Result<std::sync::MutexGuard<'_, Inner>, ApiFailure> {
        let mut inner = self.lock();
        inner
            .calls
            .push(format!("{op} {what}").trim_end().to_string());
        match inner.failures.get_mut(op).and_then(VecDeque::pop_front) {
            Some(failure) => Err(failure),
            None => Ok(inner),
        }
    }
}

fn now<'a, T: Send + 'a>(result: Result<T, ApiFailure>) -> ClusterFuture<'a, T> {
    Box::pin(std::future::ready(result))
}

fn labels_of(manifest: &serde_json::Value) -> BTreeMap<String, String> {
    strings_of(&manifest["metadata"]["labels"])
}

fn strings_of(object: &serde_json::Value) -> BTreeMap<String, String> {
    object
        .as_object()
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

impl Cluster for FakeCluster {
    fn create(
        &self,
        namespace: &str,
        kind: ObjectKind,
        manifest: serde_json::Value,
    ) -> ClusterFuture<'_, Created> {
        let name = manifest["metadata"]["name"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let result = self
            .enter("create", format!("{kind} {name}"))
            .map(|mut inner| {
                let key = (namespace.to_string(), kind, name);
                if inner.objects.contains_key(&key) {
                    return Created::AlreadyExists;
                }
                let uid = Self::uid(&mut inner);
                let view = ObjectView {
                    uid: uid.clone(),
                    resource_version: Self::version(&mut inner),
                    labels: labels_of(&manifest),
                    deleting: false,
                    storage: manifest["spec"]["resources"]["requests"]["storage"]
                        .as_str()
                        .map(str::to_string),
                    annotations: strings_of(&manifest["metadata"]["annotations"]),
                    data: strings_of(&manifest["stringData"]),
                };
                inner.objects.insert(key, (view, manifest));
                Created::Created { uid }
            });
        now(result)
    }

    fn replace(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
        resource_version: &str,
        manifest: serde_json::Value,
    ) -> ClusterFuture<'_, ()> {
        let result = self
            .enter("replace", format!("{kind} {name}"))
            .and_then(|mut inner| {
                let key = (namespace.to_string(), kind, name.to_string());
                let current = match inner.objects.get(&key) {
                    None => return Err(not_found(name)),
                    Some((view, _)) => view.resource_version.clone(),
                };
                if current != resource_version {
                    return Err(ApiFailure {
                        code: 409,
                        reason: "Conflict".into(),
                        message: format!(
                            "the object has been modified; resourceVersion {resource_version} is \
                             not {current}"
                        ),
                    });
                }
                let version = Self::version(&mut inner);
                if let Some((view, stored)) = inner.objects.get_mut(&key) {
                    view.resource_version = version;
                    view.labels = labels_of(&manifest);
                    view.annotations = strings_of(&manifest["metadata"]["annotations"]);
                    view.data = strings_of(&manifest["stringData"]);
                    *stored = manifest;
                }
                Ok(())
            });
        now(result)
    }

    fn get(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
    ) -> ClusterFuture<'_, Option<ObjectView>> {
        let result = self
            .enter("get", format!("{kind} {name}"))
            .map(|mut inner| {
                let key = (namespace.to_string(), kind, name.to_string());
                let view = inner.objects.get(&key).map(|(v, _)| v.clone());
                if view.as_ref().is_some_and(|v| v.deleting) && !inner.held.contains(&key) {
                    inner.objects.remove(&key);
                }
                view
            });
        now(result)
    }

    fn delete(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
        uid: &str,
    ) -> ClusterFuture<'_, ()> {
        let result = self
            .enter("delete", format!("{kind} {name}"))
            .and_then(|mut inner| {
                let key = (namespace.to_string(), kind, name.to_string());
                match inner.objects.get(&key) {
                    None => Ok(()),
                    Some((view, _)) if view.uid != uid => Err(ApiFailure {
                        code: 409,
                        reason: "Conflict".into(),
                        message: format!("precondition failed: UID in precondition: {uid}"),
                    }),
                    Some(_) if inner.held.contains(&key) => {
                        if let Some((view, _)) = inner.objects.get_mut(&key) {
                            view.deleting = true;
                        }
                        Ok(())
                    }
                    Some(_) => {
                        inner.objects.remove(&key);
                        Ok(())
                    }
                }
            });
        now(result)
    }

    fn namespace_exists(&self, namespace: &str) -> ClusterFuture<'_, bool> {
        let result = self
            .enter("namespace", namespace.to_string())
            .map(|inner| inner.namespaces.contains(namespace));
        now(result)
    }

    fn access_review(
        &self,
        _namespace: &str,
        verb: &str,
        resource: &str,
    ) -> ClusterFuture<'_, bool> {
        let result = self
            .enter("review", format!("{verb} {resource}"))
            .map(|inner| {
                !inner
                    .denied
                    .contains(&(verb.to_string(), resource.to_string()))
            });
        now(result)
    }

    fn pod_status(&self, namespace: &str, pod: &str) -> ClusterFuture<'_, Option<PodStatusView>> {
        let result = self.enter("pod_status", pod.to_string()).map(|mut inner| {
            let key = (namespace.to_string(), ObjectKind::Pod, pod.to_string());
            if !inner.objects.contains_key(&key) {
                return None;
            }
            let Some(queue) = inner.progressions.get_mut(pod) else {
                return Some(running());
            };
            if queue.len() > 1 {
                queue.pop_front()
            } else {
                queue.front().cloned().or_else(|| Some(running()))
            }
        });
        now(result)
    }

    fn events(&self, _namespace: &str, name: &str) -> ClusterFuture<'_, Vec<EventView>> {
        let result = self
            .enter("events", name.to_string())
            .map(|inner| inner.events.get(name).cloned().unwrap_or_default());
        now(result)
    }

    fn port_forward(
        &self,
        namespace: &str,
        pod: &str,
        port: u16,
    ) -> ClusterFuture<'_, Box<dyn AsyncReadWrite>> {
        let result = self
            .enter("port_forward", format!("{pod} {port}"))
            .and_then(|mut inner| {
                if !inner.objects.contains_key(&(
                    namespace.to_string(),
                    ObjectKind::Pod,
                    pod.to_string(),
                )) {
                    return Err(not_found(pod));
                }
                let (client, mut daemon) = tokio::io::duplex(64 * 1024);
                if let Some(banner) = &inner.banner {
                    // A fresh duplex has room: this write completes at once.
                    let _ = daemon.write_all(banner).now_or_never();
                }
                inner.tunnels.push_back(daemon);
                Ok(Box::new(client) as Box<dyn AsyncReadWrite>)
            });
        now(result)
    }

    fn logs(&self, _namespace: &str, pod: &str, tail: u32) -> ClusterFuture<'_, String> {
        let result = self.enter("logs", format!("{pod} {tail}")).map(|inner| {
            let log = inner.logs.get(pod).cloned().unwrap_or_default();
            let lines: Vec<&str> = log.lines().collect();
            let from = lines.len().saturating_sub(tail as usize);
            lines[from..].join("\n")
        });
        now(result)
    }
}

/// Key generation that returns fixed pairs: the n-th call yields pair n, so tests can predict it.
#[derive(Default)]
pub struct FakeKeyGen {
    next: std::sync::atomic::AtomicU64,
}

impl crate::kube::access::KeyGen for FakeKeyGen {
    fn generate(&self, comment: &str) -> crate::kube::access::KeyPair {
        let n = self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        crate::kube::access::KeyPair {
            private_openssh: format!("-----BEGIN FAKE KEY {n}-----"),
            public_line: format!("ssh-ed25519 AAAAfake{n} {comment}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pod(name: &str) -> serde_json::Value {
        serde_json::json!({"kind": "Pod", "metadata": {"name": name, "labels": {"a": "b"}}})
    }

    #[tokio::test]
    async fn a_second_create_by_the_same_name_is_already_exists() {
        let fake = FakeCluster::new();
        let first = fake
            .create("default", ObjectKind::Pod, pod("p"))
            .await
            .unwrap();
        assert!(matches!(first, Created::Created { .. }));
        let second = fake
            .create("default", ObjectKind::Pod, pod("p"))
            .await
            .unwrap();
        assert_eq!(second, Created::AlreadyExists);
        let got = fake
            .get("default", ObjectKind::Pod, "p")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.labels.get("a").map(String::as_str), Some("b"));
    }

    #[tokio::test]
    async fn delete_checks_the_uid_and_a_missing_object_counts_as_done() {
        let fake = FakeCluster::new();
        let Created::Created { uid } = fake
            .create("default", ObjectKind::Pod, pod("p"))
            .await
            .unwrap()
        else {
            panic!("created");
        };
        let wrong = fake.delete("default", ObjectKind::Pod, "p", "other").await;
        assert_eq!(wrong.unwrap_err().code, 409);
        fake.delete("default", ObjectKind::Pod, "p", &uid)
            .await
            .unwrap();
        fake.delete("default", ObjectKind::Pod, "p", &uid)
            .await
            .unwrap();
        assert!(fake.names("default", ObjectKind::Pod).is_empty());
    }

    #[tokio::test]
    async fn scripted_failures_and_progressions_are_consumed_in_order() {
        let fake = FakeCluster::new();
        fake.fail_next("namespace", not_found("ns"));
        assert_eq!(
            fake.namespace_exists("default").await.unwrap_err().code,
            404
        );
        assert!(fake.namespace_exists("default").await.unwrap());
        fake.create("default", ObjectKind::Pod, pod("p"))
            .await
            .unwrap();
        let pending = PodStatusView {
            phase: "Pending".into(),
            ..PodStatusView::default()
        };
        fake.progress("p", vec![pending.clone(), running()]);
        assert_eq!(
            fake.pod_status("default", "p").await.unwrap(),
            Some(pending)
        );
        assert_eq!(
            fake.pod_status("default", "p").await.unwrap(),
            Some(running())
        );
        assert_eq!(
            fake.pod_status("default", "p").await.unwrap(),
            Some(running())
        );
        assert_eq!(fake.calls()[0], "namespace default");
    }

    #[tokio::test]
    async fn replace_with_the_current_version_stores_it_under_a_new_one() {
        let fake = FakeCluster::new();
        fake.create("default", ObjectKind::Secret, pod("s"))
            .await
            .unwrap();
        let before = fake
            .get("default", ObjectKind::Secret, "s")
            .await
            .unwrap()
            .unwrap();
        let new =
            serde_json::json!({"kind": "Secret", "metadata": {"name": "s", "labels": {"a": "c"}}});
        fake.replace(
            "default",
            ObjectKind::Secret,
            "s",
            &before.resource_version,
            new.clone(),
        )
        .await
        .unwrap();
        let after = fake
            .get("default", ObjectKind::Secret, "s")
            .await
            .unwrap()
            .unwrap();
        assert_ne!(after.resource_version, before.resource_version);
        assert_eq!(after.uid, before.uid);
        assert_eq!(after.labels.get("a").map(String::as_str), Some("c"));
        assert_eq!(fake.manifest("default", ObjectKind::Secret, "s"), Some(new));
    }

    #[tokio::test]
    async fn replace_with_a_stale_version_is_a_conflict_and_changes_nothing() {
        let fake = FakeCluster::new();
        fake.create("default", ObjectKind::Secret, pod("s"))
            .await
            .unwrap();
        let before = fake
            .get("default", ObjectKind::Secret, "s")
            .await
            .unwrap()
            .unwrap();
        let new = serde_json::json!({"kind": "Secret", "metadata": {"name": "s"}});
        let stale = fake
            .replace("default", ObjectKind::Secret, "s", "stale", new)
            .await
            .unwrap_err();
        assert_eq!(stale.code, 409);
        let after = fake
            .get("default", ObjectKind::Secret, "s")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after, before);
        assert_eq!(
            fake.manifest("default", ObjectKind::Secret, "s"),
            Some(pod("s"))
        );
    }

    #[tokio::test]
    async fn a_held_pod_stays_terminating_after_its_delete() {
        let fake = FakeCluster::new();
        let Created::Created { uid } = fake
            .create("default", ObjectKind::Pod, pod("p"))
            .await
            .unwrap()
        else {
            panic!("created");
        };
        fake.hold_on_delete("default", ObjectKind::Pod, "p");
        fake.delete("default", ObjectKind::Pod, "p", &uid)
            .await
            .unwrap();
        for _ in 0..3 {
            let view = fake.get("default", ObjectKind::Pod, "p").await.unwrap();
            assert!(view.is_some_and(|v| v.deleting));
        }
        fake.release("default", ObjectKind::Pod, "p");
        assert!(fake
            .get("default", ObjectKind::Pod, "p")
            .await
            .unwrap()
            .is_some_and(|v| v.deleting));
        assert_eq!(
            fake.get("default", ObjectKind::Pod, "p").await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn port_forward_records_the_pod_and_port() {
        let fake = FakeCluster::new();
        fake.create("default", ObjectKind::Pod, pod("p"))
            .await
            .unwrap();
        let _stream = fake.port_forward("default", "p", 2222).await.unwrap();
        assert!(fake.calls().contains(&"port_forward p 2222".to_string()));
        assert!(fake.take_tunnel_peer().is_some());
    }
}
