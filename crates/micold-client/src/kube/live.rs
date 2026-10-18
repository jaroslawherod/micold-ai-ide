//! The live [`Cluster`]: the `kube` crate against the user's kubeconfig (feature 686, research K0).
//! Everything that decides something is a pure helper below, tested without a cluster; the
//! `Cluster` methods only move values between those helpers and the API.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine;
use k8s_openapi::api::authorization::v1::SelfSubjectAccessReview;
use k8s_openapi::api::core::v1::{Namespace, PersistentVolumeClaim, Pod, Secret};
use kube::api::{
    Api, ApiResource, DeleteParams, DynamicObject, ListParams, LogParams, PostParams, Preconditions,
};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::Client;
use micold_core::kube::classify::KubeFailure;
use micold_core::kube::kubeconfig::{
    candidates, home_var, parse_login_shell_output, search_path, PATH_BEGIN, PATH_END,
};
use micold_core::kube::{
    ApiFailure, AsyncReadWrite, Cluster, ClusterFuture, Created, EventView, ObjectKind, ObjectView,
    PodStatusView,
};
use micold_core::ssh::argv::ClientPlatform;

/// How long Stop gives a pod to end (contract §5).
pub const DELETE_GRACE_SECS: u32 = 10;

/// How long the login shell may take to print its `PATH`.
const LOGIN_SHELL_TIMEOUT: Duration = Duration::from_secs(5);

/// A context of the merged kubeconfig, as the form lists it. No credentials (FR-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextEntry {
    /// The context's name.
    pub name: String,
    /// Its default namespace, when it has one.
    pub namespace: Option<String>,
}

/// The contexts of `config`, in the order the files and the file lists give them.
pub fn list_contexts(config: &Kubeconfig) -> Vec<ContextEntry> {
    config
        .contexts
        .iter()
        .map(|c| ContextEntry {
            name: c.name.clone(),
            namespace: c.context.as_ref().and_then(|x| x.namespace.clone()),
        })
        .collect()
}

/// The first file named `command` (or, on Windows, with an executable extension) in `dirs`.
fn find_in(command: &str, dirs: &[PathBuf], platform: ClientPlatform) -> Option<PathBuf> {
    let extensions: &[&str] = match platform {
        ClientPlatform::Windows => &["", ".exe", ".cmd", ".bat"],
        _ => &[""],
    };
    dirs.iter()
        .flat_map(|dir| {
            extensions
                .iter()
                .map(move |ext| dir.join(format!("{command}{ext}")))
        })
        .find(|path| path.is_file())
}

/// Turn every non-absolute `users[].exec.command` into the first match on `dirs`, so a plugin
/// resolves the same whether the app started from a shell or from the desktop (FR-022). Absolute
/// commands stay. The first command found nowhere is [`KubeFailure::ExecPlugin`].
pub fn rewrite_exec_commands(
    config: &mut Kubeconfig,
    dirs: &[PathBuf],
    platform: ClientPlatform,
) -> Result<(), KubeFailure> {
    let exec = config
        .auth_infos
        .iter_mut()
        .filter_map(|u| u.auth_info.as_mut())
        .filter_map(|a| a.exec.as_mut());
    for exec in exec {
        let Some(command) = exec.command.clone() else {
            continue;
        };
        if Path::new(&command).is_absolute() {
            continue;
        }
        match find_in(&command, dirs, platform) {
            Some(found) => exec.command = Some(found.display().to_string()),
            None => {
                return Err(KubeFailure::ExecPlugin {
                    command,
                    error: "not found".to_string(),
                    search_path: dirs.to_vec(),
                })
            }
        }
    }
    Ok(())
}

/// An API failure as the `Cluster` port reports it: the server's `Status` kept whole; anything that
/// never got an answer is code 0, `Unreachable`.
pub fn to_api_failure(error: &kube::Error) -> ApiFailure {
    match error {
        kube::Error::Api(status) => ApiFailure {
            code: status.code,
            reason: status.reason.clone(),
            message: status.message.clone(),
        },
        other => ApiFailure {
            code: 0,
            reason: "Unreachable".to_string(),
            message: other.to_string(),
        },
    }
}

/// What a `create` answered: `409 AlreadyExists` is a result, not a failure.
pub fn created_from(result: Result<String, ApiFailure>) -> Result<Created, ApiFailure> {
    match result {
        Ok(uid) => Ok(Created::Created { uid }),
        Err(f) if f.code == 409 && f.reason == "AlreadyExists" => Ok(Created::AlreadyExists),
        Err(f) => Err(f),
    }
}

/// A delete's result: `404` counts as done.
pub fn deleted_from(result: Result<(), ApiFailure>) -> Result<(), ApiFailure> {
    match result {
        Err(f) if f.code == 404 => Ok(()),
        other => other,
    }
}

/// The failure to show for a call that never reached the server; `None` for any other.
pub fn unreachable_failure(
    context: &str,
    server: &str,
    failure: &ApiFailure,
) -> Option<KubeFailure> {
    (failure.code == 0).then(|| KubeFailure::Unreachable {
        context: context.to_string(),
        server: server.to_string(),
        error: failure.message.clone(),
    })
}

/// The parameters of a delete by name: only the object with this `uid`, ending within the grace.
pub fn delete_params(uid: &str) -> DeleteParams {
    DeleteParams {
        grace_period_seconds: Some(DELETE_GRACE_SECS),
        preconditions: Some(Preconditions {
            uid: Some(uid.to_string()),
            resource_version: None,
        }),
        ..DeleteParams::default()
    }
}

fn text_map(value: &serde_json::Value) -> BTreeMap<String, String> {
    value
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
        .collect()
}

/// The parts of an object the sequences decide on; a secret's `data` is decoded.
pub fn view_of(object: &serde_json::Value) -> ObjectView {
    let meta = &object["metadata"];
    let text = |v: &serde_json::Value| v.as_str().unwrap_or_default().to_string();
    let engine = base64::engine::general_purpose::STANDARD;
    let data = object["data"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| {
            let bytes = engine.decode(v.as_str()?).ok()?;
            Some((k.clone(), String::from_utf8(bytes).ok()?))
        })
        .collect();
    ObjectView {
        uid: text(&meta["uid"]),
        resource_version: text(&meta["resourceVersion"]),
        labels: text_map(&meta["labels"]),
        deleting: !meta["deletionTimestamp"].is_null(),
        storage: object["spec"]["resources"]["requests"]["storage"]
            .as_str()
            .map(str::to_string),
        annotations: text_map(&meta["annotations"]),
        data,
    }
}

/// A pod's status reduced to what start decides on. A container that ended counts as waiting with
/// its termination reason, so `ErrImagePull` and `Error` read alike.
pub fn pod_status_of(pod: &serde_json::Value) -> PodStatusView {
    let status = &pod["status"];
    let pair = |v: &serde_json::Value| {
        v["reason"].as_str().map(|reason| {
            (
                reason.to_string(),
                v["message"].as_str().unwrap_or_default().to_string(),
            )
        })
    };
    let scheduled = status["conditions"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|c| c["type"] == "PodScheduled");
    let state = &status["containerStatuses"][0]["state"];
    PodStatusView {
        phase: status["phase"].as_str().unwrap_or_default().to_string(),
        scheduled: scheduled.map(|c| c["status"] == "True"),
        schedule_reason: scheduled.filter(|c| c["status"] != "True").and_then(pair),
        waiting: pair(&state["waiting"]).or_else(|| pair(&state["terminated"])),
        running: !state["running"].is_null(),
        deleting: !pod["metadata"]["deletionTimestamp"].is_null(),
        reason: pair(status),
        node: pod["spec"]["nodeName"].as_str().map(str::to_string),
    }
}

/// Events oldest first, by their last time, else their creation.
pub fn events_of(items: &[serde_json::Value]) -> Vec<EventView> {
    let when = |e: &serde_json::Value| {
        ["lastTimestamp", "eventTime"]
            .iter()
            .find_map(|k| e[k].as_str())
            .or_else(|| e["metadata"]["creationTimestamp"].as_str())
            .unwrap_or_default()
            .to_string()
    };
    let mut items: Vec<_> = items.iter().collect();
    items.sort_by_key(|e| when(e));
    items
        .into_iter()
        .map(|e| EventView {
            reason: e["reason"].as_str().unwrap_or_default().to_string(),
            message: e["message"].as_str().unwrap_or_default().to_string(),
        })
        .collect()
}

/// The `SelfSubjectAccessReview` for `verb` on `resource` (`pods/portforward` is resource `pods`,
/// subresource `portforward`).
pub fn access_review_body(namespace: &str, verb: &str, resource: &str) -> serde_json::Value {
    let (resource, sub) = resource.split_once('/').unwrap_or((resource, ""));
    serde_json::json!({
        "apiVersion": "authorization.k8s.io/v1",
        "kind": "SelfSubjectAccessReview",
        "spec": {"resourceAttributes": {
            "namespace": namespace, "verb": verb, "resource": resource, "subresource": sub
        }}
    })
}

/// The kubeconfig files that exist, merged in order. None existing is `NoKubeconfig`.
pub fn read_kubeconfigs(paths: &[PathBuf]) -> Result<Kubeconfig, KubeFailure> {
    let mut merged: Option<Kubeconfig> = None;
    for path in paths.iter().filter(|p| p.is_file()) {
        let unreadable = |e: &dyn std::fmt::Display| KubeFailure::Api {
            status: 0,
            message: format!("cannot read {}: {e}", path.display()),
        };
        let next = Kubeconfig::read_from(path).map_err(|e| unreadable(&e))?;
        merged = Some(match merged {
            None => next,
            Some(so_far) => so_far.merge(next).map_err(|e| unreadable(&e))?,
        });
    }
    merged.ok_or_else(|| KubeFailure::NoKubeconfig {
        looked_in: paths.to_vec(),
    })
}

/// The login shell's `PATH`, read once per run (not on Windows), within five seconds.
async fn login_shell_path() -> Option<OsString> {
    static CELL: tokio::sync::OnceCell<Option<OsString>> = tokio::sync::OnceCell::const_new();
    if cfg!(windows) {
        return None;
    }
    CELL.get_or_init(|| async {
        let shell = std::env::var_os("SHELL")?;
        let script = format!("printf '%s%s%s' '{PATH_BEGIN}' \"$PATH\" '{PATH_END}'");
        let mut command = tokio::process::Command::new(shell);
        command
            .args(["-l", "-c", &script])
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true);
        let out = tokio::time::timeout(LOGIN_SHELL_TIMEOUT, command.output())
            .await
            .ok()?
            .ok()?;
        parse_login_shell_output(&String::from_utf8_lossy(&out.stdout))
    })
    .await
    .clone()
}

/// The cluster of one kubeconfig context.
pub struct LiveCluster {
    client: Client,
    context: String,
    server: String,
}

impl LiveCluster {
    /// Read the kubeconfig the way FR-022 says, resolve its credential plugins and build a client
    /// for `context`.
    pub async fn connect(context: &str) -> Result<Self, KubeFailure> {
        let platform = ClientPlatform::current();
        let home = std::env::var_os(home_var(platform))
            .map(PathBuf::from)
            .unwrap_or_default();
        let files = candidates(std::env::var_os("KUBECONFIG").as_deref(), &home, platform);
        let mut config = read_kubeconfigs(&files)?;
        let named = config.contexts.iter().find(|c| c.name == context);
        let user = named.and_then(|c| c.context.as_ref()?.user.clone());
        if named.is_none() {
            return Err(KubeFailure::ContextMissing {
                context: context.to_string(),
                looked_in: files,
            });
        }
        // Only this context's user matters: another user's missing plugin is not our failure.
        config.auth_infos.retain(|u| Some(&u.name) == user.as_ref());
        let dirs = search_path(
            std::env::var_os("PATH").as_deref(),
            login_shell_path().await.as_deref(),
            &home,
            platform,
        );
        rewrite_exec_commands(&mut config, &dirs, platform)?;
        let options = KubeConfigOptions {
            context: Some(context.to_string()),
            ..KubeConfigOptions::default()
        };
        let config = kube::Config::from_custom_kubeconfig(config, &options)
            .await
            .map_err(|e| KubeFailure::Api {
                status: 0,
                message: e.to_string(),
            })?;
        let server = config.cluster_url.to_string();
        let client = Client::try_from(config).map_err(|e| KubeFailure::Api {
            status: 0,
            message: e.to_string(),
        })?;
        Ok(Self {
            client,
            context: context.to_string(),
            server,
        })
    }

    /// The context's name and API server, for [`unreachable_failure`].
    pub fn endpoint(&self) -> (&str, &str) {
        (&self.context, &self.server)
    }

    fn api(&self, namespace: &str, kind: ObjectKind) -> Api<DynamicObject> {
        let resource = match kind {
            ObjectKind::Pod => ApiResource::erase::<Pod>(&()),
            ObjectKind::PersistentVolumeClaim => ApiResource::erase::<PersistentVolumeClaim>(&()),
            ObjectKind::Secret => ApiResource::erase::<Secret>(&()),
        };
        Api::namespaced_with(self.client.clone(), namespace, &resource)
    }

    fn pods(&self, namespace: &str) -> Api<Pod> {
        Api::namespaced(self.client.clone(), namespace)
    }
}

fn value_of(object: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(object).unwrap_or_default()
}

fn object_of(manifest: serde_json::Value) -> Result<DynamicObject, ApiFailure> {
    serde_json::from_value(manifest).map_err(|e| ApiFailure {
        code: 0,
        reason: "InvalidManifest".to_string(),
        message: e.to_string(),
    })
}

impl Cluster for LiveCluster {
    fn create(
        &self,
        namespace: &str,
        kind: ObjectKind,
        manifest: serde_json::Value,
    ) -> ClusterFuture<'_, Created> {
        let api = self.api(namespace, kind);
        Box::pin(async move {
            let object = object_of(manifest)?;
            let made = api.create(&PostParams::default(), &object).await;
            created_from(
                made.map(|o| o.metadata.uid.unwrap_or_default())
                    .map_err(|e| to_api_failure(&e)),
            )
        })
    }

    fn replace(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
        resource_version: &str,
        manifest: serde_json::Value,
    ) -> ClusterFuture<'_, ()> {
        let api = self.api(namespace, kind);
        let (name, version) = (name.to_string(), resource_version.to_string());
        Box::pin(async move {
            let mut object = object_of(manifest)?;
            object.metadata.resource_version = Some(version);
            api.replace(&name, &PostParams::default(), &object)
                .await
                .map(|_| ())
                .map_err(|e| to_api_failure(&e))
        })
    }

    fn get(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
    ) -> ClusterFuture<'_, Option<ObjectView>> {
        let api = self.api(namespace, kind);
        let name = name.to_string();
        Box::pin(async move {
            let found = api.get_opt(&name).await.map_err(|e| to_api_failure(&e))?;
            Ok(found.map(|o| view_of(&value_of(&o))))
        })
    }

    fn delete(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
        uid: &str,
    ) -> ClusterFuture<'_, ()> {
        let api = self.api(namespace, kind);
        let (name, params) = (name.to_string(), delete_params(uid));
        Box::pin(async move {
            deleted_from(
                api.delete(&name, &params)
                    .await
                    .map(|_| ())
                    .map_err(|e| to_api_failure(&e)),
            )
        })
    }

    fn namespace_exists(&self, namespace: &str) -> ClusterFuture<'_, bool> {
        let api: Api<Namespace> = Api::all(self.client.clone());
        let name = namespace.to_string();
        Box::pin(async move {
            let found = api.get_opt(&name).await.map_err(|e| to_api_failure(&e))?;
            Ok(found.is_some())
        })
    }

    fn access_review(
        &self,
        namespace: &str,
        verb: &str,
        resource: &str,
    ) -> ClusterFuture<'_, bool> {
        let api: Api<SelfSubjectAccessReview> = Api::all(self.client.clone());
        let body = access_review_body(namespace, verb, resource);
        Box::pin(async move {
            let review: SelfSubjectAccessReview =
                serde_json::from_value(body).map_err(|e| ApiFailure {
                    code: 0,
                    reason: "InvalidManifest".to_string(),
                    message: e.to_string(),
                })?;
            let answer = api
                .create(&PostParams::default(), &review)
                .await
                .map_err(|e| to_api_failure(&e))?;
            Ok(answer.status.is_some_and(|s| s.allowed))
        })
    }

    fn pod_status(&self, namespace: &str, pod: &str) -> ClusterFuture<'_, Option<PodStatusView>> {
        let api = self.pods(namespace);
        let pod = pod.to_string();
        Box::pin(async move {
            let found = api.get_opt(&pod).await.map_err(|e| to_api_failure(&e))?;
            Ok(found.map(|p| pod_status_of(&value_of(&p))))
        })
    }

    fn events(&self, namespace: &str, name: &str) -> ClusterFuture<'_, Vec<EventView>> {
        let resource = ApiResource::erase::<k8s_openapi::api::core::v1::Event>(&());
        let api: Api<DynamicObject> =
            Api::namespaced_with(self.client.clone(), namespace, &resource);
        let selector = ListParams::default().fields(&format!("involvedObject.name={name}"));
        Box::pin(async move {
            let list = api.list(&selector).await.map_err(|e| to_api_failure(&e))?;
            let items: Vec<_> = list.items.iter().map(value_of).collect();
            Ok(events_of(&items))
        })
    }

    fn port_forward(
        &self,
        namespace: &str,
        pod: &str,
        port: u16,
    ) -> ClusterFuture<'_, Box<dyn AsyncReadWrite>> {
        let api = self.pods(namespace);
        let pod = pod.to_string();
        Box::pin(async move {
            let mut forwarder = api
                .portforward(&pod, &[port])
                .await
                .map_err(|e| to_api_failure(&e))?;
            let stream = forwarder.take_stream(port).ok_or_else(|| ApiFailure {
                code: 0,
                reason: "Unreachable".to_string(),
                message: format!("no stream for port {port}"),
            })?;
            // The forwarder's task ends with the stream; this keeps it owned until then.
            tokio::spawn(async move {
                let _ = forwarder.join().await;
            });
            Ok(Box::new(stream) as Box<dyn AsyncReadWrite>)
        })
    }

    fn logs(&self, namespace: &str, pod: &str, tail: u32) -> ClusterFuture<'_, String> {
        let api = self.pods(namespace);
        let pod = pod.to_string();
        Box::pin(async move {
            let params = LogParams {
                tail_lines: Some(i64::from(tail)),
                ..LogParams::default()
            };
            api.logs(&pod, &params)
                .await
                .map_err(|e| to_api_failure(&e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const LINUX: ClientPlatform = ClientPlatform::Linux;

    fn config(users: &str) -> Kubeconfig {
        Kubeconfig::from_yaml(&format!(
            "apiVersion: v1\nkind: Config\nclusters: []\ncontexts:\n- name: a\n  context: {{cluster: c, user: u1, namespace: dev}}\n- name: b\n  context: {{cluster: c, user: u2}}\nusers:\n{users}"
        ))
        .unwrap()
    }

    fn commands(config: &Kubeconfig) -> Vec<Option<String>> {
        config
            .auth_infos
            .iter()
            .map(|u| u.auth_info.as_ref()?.exec.as_ref()?.command.clone())
            .collect()
    }

    fn plugin_dir(name: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(name), "#!/bin/sh\n").unwrap();
        dir
    }

    const USERS: &str = "- name: u1\n  user: {exec: {apiVersion: client.authentication.k8s.io/v1, command: aws}}\n- name: u2\n  user: {exec: {apiVersion: client.authentication.k8s.io/v1, command: /opt/abs/gke}}\n";

    #[test]
    fn a_plugin_named_without_a_path_becomes_its_first_match_and_absolute_ones_stay() {
        let (first, second) = (plugin_dir("aws"), plugin_dir("aws"));
        let dirs = vec![first.path().to_path_buf(), second.path().to_path_buf()];
        let mut config = config(USERS);
        rewrite_exec_commands(&mut config, &dirs, LINUX).unwrap();
        assert_eq!(
            commands(&config),
            [
                Some(first.path().join("aws").display().to_string()),
                Some("/opt/abs/gke".to_string())
            ]
        );
    }

    #[test]
    fn a_plugin_found_nowhere_names_the_command_and_where_it_looked() {
        let empty = tempfile::tempdir().unwrap();
        let dirs = vec![empty.path().to_path_buf()];
        let mut config = config(USERS);
        let failure = rewrite_exec_commands(&mut config, &dirs, LINUX).unwrap_err();
        assert_eq!(
            failure,
            KubeFailure::ExecPlugin {
                command: "aws".into(),
                error: "not found".into(),
                search_path: dirs
            }
        );
    }

    #[test]
    fn contexts_come_in_order_with_their_namespace_and_no_user_data() {
        let contexts = list_contexts(&config(USERS));
        assert_eq!(
            contexts,
            [
                ContextEntry {
                    name: "a".into(),
                    namespace: Some("dev".into())
                },
                ContextEntry {
                    name: "b".into(),
                    namespace: None
                }
            ]
        );
    }

    #[test]
    fn files_that_exist_merge_in_order_and_none_is_no_kubeconfig() {
        let dir = tempfile::tempdir().unwrap();
        let (one, two) = (dir.path().join("one"), dir.path().join("two"));
        let file = |name: &str| {
            format!("apiVersion: v1\nkind: Config\nclusters: []\nusers: []\ncontexts:\n- name: {name}\n  context: {{cluster: c}}\n")
        };
        std::fs::write(&one, file("x")).unwrap();
        std::fs::write(&two, file("y")).unwrap();
        let missing = dir.path().join("missing");
        let merged = read_kubeconfigs(&[one, missing.clone(), two]).unwrap();
        let names: Vec<_> = list_contexts(&merged).into_iter().map(|c| c.name).collect();
        assert_eq!(names, ["x", "y"]);
        assert_eq!(
            read_kubeconfigs(std::slice::from_ref(&missing)).unwrap_err(),
            KubeFailure::NoKubeconfig {
                looked_in: vec![missing]
            }
        );
    }

    fn status(code: u16, reason: &str, message: &str) -> kube::Error {
        kube::Error::Api(Box::new(kube::core::Status {
            code,
            reason: reason.into(),
            message: message.into(),
            ..kube::core::Status::default()
        }))
    }

    #[test]
    fn an_api_status_keeps_its_code_reason_and_message() {
        let f = to_api_failure(&status(403, "Forbidden", "no pods"));
        assert_eq!(
            (f.code, f.reason.as_str(), f.message.as_str()),
            (403, "Forbidden", "no pods")
        );
    }

    #[test]
    fn a_create_that_answers_already_exists_is_a_result_but_a_replace_conflict_stays_a_failure() {
        let exists = to_api_failure(&status(409, "AlreadyExists", "x"));
        assert_eq!(created_from(Err(exists)), Ok(Created::AlreadyExists));
        let conflict = to_api_failure(&status(409, "Conflict", "stale"));
        assert_eq!(created_from(Err(conflict.clone())).unwrap_err(), conflict);
        assert_eq!(
            created_from(Ok("u1".into())),
            Ok(Created::Created { uid: "u1".into() })
        );
    }

    #[test]
    fn a_delete_of_a_missing_object_is_done_and_other_failures_stay() {
        let gone = to_api_failure(&status(404, "NotFound", "x"));
        assert_eq!(deleted_from(Err(gone)), Ok(()));
        let denied = to_api_failure(&status(403, "Forbidden", "x"));
        assert_eq!(deleted_from(Err(denied.clone())), Err(denied));
    }

    #[test]
    fn a_call_that_never_reached_the_server_is_unreachable_naming_context_and_server() {
        let f = to_api_failure(&kube::Error::LinesCodecMaxLineLengthExceeded);
        assert_eq!((f.code, f.reason.as_str()), (0, "Unreachable"));
        assert_eq!(
            unreachable_failure("kind-dev", "https://k:6443", &f),
            Some(KubeFailure::Unreachable {
                context: "kind-dev".into(),
                server: "https://k:6443".into(),
                error: f.message.clone()
            })
        );
        let answered = to_api_failure(&status(500, "InternalError", "x"));
        assert_eq!(unreachable_failure("c", "s", &answered), None);
    }

    #[test]
    fn a_delete_carries_the_uid_precondition_and_the_grace_period() {
        let params = delete_params("uid-1");
        assert_eq!(params.grace_period_seconds, Some(10));
        let pre = params.preconditions.unwrap();
        assert_eq!(pre.uid.as_deref(), Some("uid-1"));
    }

    #[test]
    fn an_object_view_reads_identity_labels_storage_and_decodes_secret_data() {
        let object = json!({
            "metadata": {"uid": "u", "resourceVersion": "7", "labels": {"a": "b"},
                "annotations": {"n": "v"}, "deletionTimestamp": "2026-01-01T00:00:00Z"},
            "spec": {"resources": {"requests": {"storage": "10Gi"}}},
            "data": {"k": "aGVsbG8="}
        });
        let view = view_of(&object);
        assert_eq!(
            (view.uid.as_str(), view.resource_version.as_str()),
            ("u", "7")
        );
        assert!(view.deleting);
        assert_eq!(view.storage.as_deref(), Some("10Gi"));
        assert_eq!(view.data["k"], "hello");
        assert_eq!(view.labels["a"], "b");
        assert_eq!(view.annotations["n"], "v");
        assert!(!view_of(&json!({"metadata": {}})).deleting);
    }

    #[test]
    fn a_pod_status_reads_scheduling_waiting_and_running() {
        let pending = json!({"status": {"phase": "Pending",
            "conditions": [{"type": "PodScheduled", "status": "False",
                "reason": "Unschedulable", "message": "0/3 nodes"}],
            "containerStatuses": [{"state": {"waiting": {"reason": "ErrImagePull", "message": "denied"}}}]}});
        let s = pod_status_of(&pending);
        assert_eq!(s.scheduled, Some(false));
        assert_eq!(
            s.schedule_reason,
            Some(("Unschedulable".into(), "0/3 nodes".into()))
        );
        assert_eq!(s.waiting, Some(("ErrImagePull".into(), "denied".into())));
        assert!(!s.running);
        let up = json!({"status": {"phase": "Running",
            "conditions": [{"type": "PodScheduled", "status": "True"}],
            "containerStatuses": [{"state": {"running": {}}}]}});
        let s = pod_status_of(&up);
        assert_eq!(
            (s.scheduled, s.running, s.waiting),
            (Some(true), true, None)
        );
        let exited = json!({"status": {"phase": "Failed",
            "containerStatuses": [{"state": {"terminated": {"reason": "Error"}}}]}});
        assert_eq!(
            pod_status_of(&exited).waiting,
            Some(("Error".into(), String::new()))
        );
    }

    #[test]
    fn events_come_oldest_first() {
        let items = [
            json!({"reason": "B", "message": "m2", "lastTimestamp": "2026-01-02T00:00:00Z"}),
            json!({"reason": "A", "message": "m1", "metadata": {"creationTimestamp": "2026-01-01T00:00:00Z"}}),
        ];
        let reasons: Vec<_> = events_of(&items).into_iter().map(|e| e.reason).collect();
        assert_eq!(reasons, ["A", "B"]);
    }

    #[test]
    fn a_port_forward_permission_is_asked_as_the_subresource() {
        let body = access_review_body("dev", "create", "pods/portforward");
        let attrs = &body["spec"]["resourceAttributes"];
        assert_eq!(
            (
                attrs["resource"].as_str(),
                attrs["subresource"].as_str(),
                attrs["namespace"].as_str()
            ),
            (Some("pods"), Some("portforward"), Some("dev"))
        );
    }
}
