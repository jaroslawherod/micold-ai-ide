//! The cluster objects a Kubernetes daemon runs as (feature 686, contract §1, research K7). Pure
//! builders returning `serde_json::Value`; the security context and credential mounts come with
//! the credentials milestone (M3).

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::daemon_resources::DaemonResources;
use crate::kube::names::{ResourceNames, DAEMON_NAME_ANNOTATION};
use crate::kube::settings::{KubeSettings, Quantity, TaintEffect, Toleration, TolerationOperator};
use crate::sandbox::{Bytes, MilliCpus};

/// The port the image's SSH server listens on, on loopback only (shared-ssh-image §1).
pub const SSH_PORT: u16 = 2222;

/// The UID the image runs as.
pub const IMAGE_USER_UID: u32 = 10001;

/// Where the daemon's volume is mounted.
pub const DATA_MOUNT: &str = "/data";

/// Where the access secret is mounted, read-only.
pub const ACCESS_MOUNT: &str = "/run/micold/ssh";

/// The environment variable that tells the image's SSH server where [`ACCESS_MOUNT`] is.
pub const ACCESS_DIR_ENV: &str = "MICOLD_SSH_DIR";

/// The container's name.
pub const CONTAINER: &str = "daemon";

/// The pod annotation that holds the [`PodStamp`].
pub const POD_STAMP_ANNOTATION: &str = "micold.dev/pod-settings";

/// The settings that shape the pod, as canonical JSON (data model "PodStamp").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodStamp(String);

impl PodStamp {
    /// The stamp of a pod running `image` with `settings`.
    pub fn of(settings: &KubeSettings, image: &str) -> Self {
        let value = json!({
            "image": image,
            "resources": serde_json::to_value(settings.resources).unwrap_or(Value::Null),
            "node_selector": settings.node_selector,
            "tolerations": settings.tolerations,
            "allowed": settings.allowed,
        });
        Self(value.to_string())
    }

    /// The annotation value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The settings that differ between the stamp on a running pod and the one now, in form order
/// (FR-005); empty means the pod is reused.
pub fn drift(on_pod: &str, now: &PodStamp) -> Vec<&'static str> {
    const KEYS: [(&str, &str); 5] = [
        ("image", "image"),
        ("resources", "resources"),
        ("node_selector", "node selector"),
        ("tolerations", "tolerations"),
        ("allowed", "allowed credentials"),
    ];
    let on_pod: Value = serde_json::from_str(on_pod).unwrap_or(Value::Null);
    let now: Value = serde_json::from_str(now.as_str()).unwrap_or(Value::Null);
    KEYS.iter()
        .filter(|(key, _)| on_pod.get(key).is_none() || on_pod.get(key) != now.get(key))
        .map(|(_, label)| *label)
        .collect()
}

/// The metadata every object carries: `name`, our labels and the daemon-name annotation.
fn metadata(names: &ResourceNames, name: String, extra: &[(&str, &str)]) -> Value {
    let mut annotations = BTreeMap::from([(DAEMON_NAME_ANNOTATION, names.daemon_name())]);
    annotations.extend(extra.iter().copied());
    json!({ "name": name, "labels": names.labels(), "annotations": annotations })
}

fn toleration(t: &Toleration) -> Value {
    let effect = match t.effect {
        TaintEffect::NoSchedule => "NoSchedule",
        TaintEffect::PreferNoSchedule => "PreferNoSchedule",
        TaintEffect::NoExecute => "NoExecute",
    };
    let mut out = json!({ "key": t.key, "effect": effect });
    match &t.operator {
        TolerationOperator::Equal(value) => {
            out["operator"] = json!("Equal");
            if let Some(v) = value {
                out["value"] = json!(v);
            }
        }
        TolerationOperator::Exists => out["operator"] = json!("Exists"),
    }
    out
}

/// One of `requests`/`limits`, or `None` when neither field is set.
fn quantities(cpu: Option<MilliCpus>, memory: Option<Bytes>) -> Option<Value> {
    let mut map = serde_json::Map::new();
    if let Some(c) = cpu {
        map.insert("cpu".into(), json!(format!("{}m", c.0)));
    }
    if let Some(m) = memory {
        map.insert("memory".into(), json!(Quantity(m.0).to_k8s()));
    }
    (!map.is_empty()).then_some(Value::Object(map))
}

/// The container's `resources`, or `None` when no field is set.
fn resources(r: &DaemonResources) -> Option<Value> {
    let mut out = serde_json::Map::new();
    if let Some(v) = quantities(r.cpu_request, r.memory_request) {
        out.insert("requests".into(), v);
    }
    if let Some(v) = quantities(r.cpu_limit, r.memory_limit) {
        out.insert("limits".into(), v);
    }
    (!out.is_empty()).then_some(Value::Object(out))
}

fn container(settings: &KubeSettings, image: &str) -> Value {
    let mut out = json!({
        "name": CONTAINER,
        "image": image,
        "env": [
            { "name": "HOME", "value": "/data/home" },
            { "name": "TMPDIR", "value": "/tmp" },
            { "name": ACCESS_DIR_ENV, "value": ACCESS_MOUNT },
        ],
        "volumeMounts": [
            { "name": "data", "mountPath": DATA_MOUNT },
            { "name": "tmp", "mountPath": "/tmp" },
            { "name": "ssh-access", "mountPath": ACCESS_MOUNT, "readOnly": true },
        ],
    });
    if let Some(r) = resources(&settings.resources) {
        out["resources"] = r;
    }
    out
}

/// The access reviews the start asks before it creates anything (contract §3): verb and resource.
pub fn access_reviews() -> [(&'static str, &'static str); 6] {
    [
        ("create", "pods"),
        ("create", "persistentvolumeclaims"),
        ("create", "secrets"),
        ("update", "secrets"),
        ("create", "pods/portforward"),
        ("delete", "pods"),
    ]
}

/// The pod that runs `image` for the daemon `names`.
pub fn pod(settings: &KubeSettings, names: &ResourceNames, image: &str, stamp: &PodStamp) -> Value {
    let mut spec = json!({
        "restartPolicy": "Always",
        "automountServiceAccountToken": false,
        "enableServiceLinks": false,
        "containers": [container(settings, image)],
        "volumes": [
            { "name": "data", "persistentVolumeClaim": { "claimName": names.pvc() } },
            { "name": "tmp", "emptyDir": {} },
            { "name": "ssh-access",
              "secret": { "secretName": names.access_secret(), "defaultMode": 0o440 } },
        ],
    });
    if !settings.node_selector.is_empty() {
        spec["nodeSelector"] = json!(settings.node_selector);
    }
    if !settings.tolerations.is_empty() {
        spec["tolerations"] = settings.tolerations.iter().map(toleration).collect();
    }
    json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": metadata(names, names.pod(), &[(POD_STAMP_ANNOTATION, stamp.as_str())]),
        "spec": spec,
    })
}

/// The daemon's volume claim.
pub fn pvc(settings: &KubeSettings, names: &ResourceNames) -> Value {
    let mut spec = json!({
        "accessModes": ["ReadWriteOnce"],
        "resources": { "requests": { "storage": settings.volume_size.to_k8s() } },
    });
    if let Some(class) = &settings.storage_class {
        spec["storageClassName"] = json!(class);
    }
    json!({
        "apiVersion": "v1",
        "kind": "PersistentVolumeClaim",
        "metadata": metadata(names, names.pvc(), &[]),
        "spec": spec,
    })
}

/// An Opaque secret `name` with `data` as text (`stringData`).
pub fn secret(names: &ResourceNames, name: &str, data: &BTreeMap<String, String>) -> Value {
    json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "type": "Opaque",
        "metadata": metadata(names, name.to_owned(), &[]),
        "stringData": data,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::daemon_resources::DaemonResources;
    use crate::kube::names::{DAEMON_LABEL, MANAGED_BY_LABEL, MANAGED_BY_VALUE};
    use crate::kube::settings::{LabelKey, LabelValue, Quantity, Toleration, GIB};
    use crate::sandbox::{Bytes, MilliCpus};

    const IMAGE: &str = "ghcr.io/acme/micold-daemon:1.2.3";

    fn settings() -> KubeSettings {
        KubeSettings::new("kind-dev", "dev", &[])
    }

    fn names() -> ResourceNames {
        ResourceNames::new("kind-dev", "dev", "Build box")
    }

    fn all() -> [Value; 3] {
        let s = settings();
        [
            pod(&s, &names(), IMAGE, &PodStamp::of(&s, IMAGE)),
            pvc(&s, &names()),
            secret(&names(), "x-ssh-access", &BTreeMap::new()),
        ]
    }

    /// Every key anywhere in `value`, depth first.
    fn keys(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    out.push(k.clone());
                    keys(v, out);
                }
            }
            Value::Array(items) => items.iter().for_each(|v| keys(v, out)),
            _ => {}
        }
    }

    fn env(pod: &Value) -> BTreeMap<String, String> {
        pod["spec"]["containers"][0]["env"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e["name"].as_str().unwrap().to_string(),
                    e["value"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn no_object_is_a_service_or_opens_the_host() {
        for object in all() {
            let kind = object["kind"].as_str().unwrap();
            assert!(!["Service", "Ingress"].contains(&kind), "{kind}");
            let mut all = Vec::new();
            keys(&object, &mut all);
            for banned in [
                "hostPort",
                "hostNetwork",
                "hostPID",
                "privileged",
                "containerPort",
                "ports",
            ] {
                assert!(!all.iter().any(|k| k == banned), "{banned} in {kind}");
            }
        }
        assert_eq!(all()[0]["kind"], "Pod");
        assert_eq!(all()[1]["kind"], "PersistentVolumeClaim");
        assert_eq!(all()[2]["kind"], "Secret");
    }

    #[test]
    fn the_pod_restarts_and_carries_no_service_account_token_or_links() {
        let p = &all()[0];
        assert_eq!(p["apiVersion"], "v1");
        assert_eq!(p["metadata"]["name"], names().pod());
        assert_eq!(p["spec"]["restartPolicy"], "Always");
        assert_eq!(p["spec"]["automountServiceAccountToken"], false);
        assert_eq!(p["spec"]["enableServiceLinks"], false);
        let containers = p["spec"]["containers"].as_array().unwrap();
        assert_eq!(containers.len(), 1, "one container");
        assert_eq!(containers[0]["name"], CONTAINER);
        assert_eq!(containers[0]["image"], IMAGE);
        let other = pod(
            &settings(),
            &names(),
            "other:1",
            &PodStamp::of(&settings(), "other:1"),
        );
        assert_eq!(other["spec"]["containers"][0]["image"], "other:1");
        assert_eq!((SSH_PORT, IMAGE_USER_UID), (2222, 10001));
    }

    #[test]
    fn the_container_env_is_home_tmp_and_the_access_secret_directory_only() {
        let want: BTreeMap<String, String> = [
            ("HOME", "/data/home"),
            ("TMPDIR", "/tmp"),
            (ACCESS_DIR_ENV, ACCESS_MOUNT),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        assert_eq!(env(&all()[0]), want);
        assert_eq!(ACCESS_MOUNT, "/run/micold/ssh");
        assert!(all()[0]["spec"]["containers"][0].get("command").is_none());
    }

    #[test]
    fn the_claim_is_mounted_at_data_with_an_empty_dir_at_tmp() {
        let p = &all()[0];
        let volumes = p["spec"]["volumes"].as_array().unwrap();
        assert_eq!(volumes.len(), 3);
        assert_eq!(volumes[0]["name"], "data");
        assert_eq!(
            volumes[0]["persistentVolumeClaim"]["claimName"],
            names().pvc()
        );
        assert_eq!(volumes[1]["name"], "tmp");
        assert_eq!(volumes[1]["emptyDir"], json!({}));
        assert_eq!(volumes[2]["name"], "ssh-access");
        assert_eq!(volumes[2]["secret"]["secretName"], names().access_secret());
        assert_eq!(volumes[2]["secret"]["defaultMode"], 0o440);
        let mounts = p["spec"]["containers"][0]["volumeMounts"]
            .as_array()
            .unwrap();
        assert_eq!(mounts[0], json!({"name": "data", "mountPath": "/data"}));
        assert_eq!(mounts[1], json!({"name": "tmp", "mountPath": "/tmp"}));
        assert_eq!(
            mounts[2],
            json!({"name": "ssh-access", "mountPath": "/run/micold/ssh", "readOnly": true})
        );
    }

    #[test]
    fn the_claim_is_read_write_once_with_the_size_and_class_when_given() {
        let mut s = settings();
        let claim = pvc(&s, &names());
        assert_eq!(claim["apiVersion"], "v1");
        assert_eq!(claim["metadata"]["name"], names().pvc());
        assert_eq!(claim["spec"]["accessModes"], json!(["ReadWriteOnce"]));
        assert_eq!(claim["spec"]["resources"]["requests"]["storage"], "10Gi");
        assert!(
            claim["spec"].get("storageClassName").is_none(),
            "no class: the cluster default"
        );
        s.volume_size = Quantity(25 * GIB);
        s.storage_class = Some("fast-ssd".into());
        let claim = pvc(&s, &names());
        assert_eq!(claim["spec"]["resources"]["requests"]["storage"], "25Gi");
        assert_eq!(claim["spec"]["storageClassName"], "fast-ssd");
    }

    #[test]
    fn every_object_carries_both_labels_and_the_name_annotation() {
        for object in all() {
            let meta = &object["metadata"];
            assert_eq!(meta["labels"][MANAGED_BY_LABEL], MANAGED_BY_VALUE);
            assert_eq!(meta["labels"][DAEMON_LABEL], names().key().as_str());
            assert_eq!(meta["annotations"][DAEMON_NAME_ANNOTATION], "Build box");
        }
    }

    #[test]
    fn node_selector_and_tolerations_appear_as_given_and_not_at_all_when_none() {
        let p = &all()[0];
        assert!(p["spec"].get("nodeSelector").is_none());
        assert!(p["spec"].get("tolerations").is_none());
        let mut s = settings();
        s.node_selector = BTreeMap::from([(LabelKey("pool".into()), LabelValue("agents".into()))]);
        s.tolerations = vec![
            Toleration::parse("gpu=true:NoSchedule").unwrap(),
            Toleration::parse("spot:Exists:NoExecute").unwrap(),
            Toleration::parse("dedicated:PreferNoSchedule").unwrap(),
        ];
        let p = pod(&s, &names(), IMAGE, &PodStamp::of(&s, IMAGE));
        assert_eq!(p["spec"]["nodeSelector"], json!({"pool": "agents"}));
        assert_eq!(
            p["spec"]["tolerations"],
            json!([
                {"key": "gpu", "operator": "Equal", "value": "true", "effect": "NoSchedule"},
                {"key": "spot", "operator": "Exists", "effect": "NoExecute"},
                {"key": "dedicated", "operator": "Equal", "effect": "PreferNoSchedule"},
            ])
        );
    }

    #[test]
    fn resources_hold_exactly_the_set_fields() {
        let mut s = settings();
        s.resources = DaemonResources {
            cpu_request: Some(MilliCpus(500)),
            cpu_limit: Some(MilliCpus(2000)),
            memory_request: None,
            memory_limit: Some(Bytes(4 * GIB)),
        };
        let p = pod(&s, &names(), IMAGE, &PodStamp::of(&s, IMAGE));
        let r = &p["spec"]["containers"][0]["resources"];
        assert_eq!(r["requests"], json!({"cpu": "500m"}));
        assert_eq!(r["limits"], json!({"cpu": "2000m", "memory": "4Gi"}));
        s.resources = DaemonResources {
            cpu_request: None,
            cpu_limit: None,
            memory_request: None,
            memory_limit: None,
        };
        let p = pod(&s, &names(), IMAGE, &PodStamp::of(&s, IMAGE));
        assert!(p["spec"]["containers"][0].get("resources").is_none());
    }

    #[test]
    fn the_secret_carries_its_data_as_text_under_the_labels() {
        let data = BTreeMap::from([("authorized_keys".to_string(), "k\n".to_string())]);
        let s = secret(&names(), &names().access_secret(), &data);
        assert_eq!(s["metadata"]["name"], names().access_secret());
        assert_eq!(s["type"], "Opaque");
        assert_eq!(s["stringData"], json!({"authorized_keys": "k\n"}));
    }

    #[test]
    fn the_pod_carries_its_stamp_and_the_stamp_follows_the_pod_shaping_settings() {
        let s = settings();
        let stamp = PodStamp::of(&s, IMAGE);
        let p = pod(&s, &names(), IMAGE, &stamp);
        assert_eq!(
            p["metadata"]["annotations"][POD_STAMP_ANNOTATION],
            stamp.as_str()
        );
        assert_eq!(POD_STAMP_ANNOTATION, "micold.dev/pod-settings");
        assert_eq!(
            stamp,
            PodStamp::of(&s, IMAGE),
            "equal settings, equal stamp"
        );
        let json: Value = serde_json::from_str(stamp.as_str()).unwrap();
        for key in [
            "image",
            "resources",
            "node_selector",
            "tolerations",
            "allowed",
        ] {
            assert!(json.get(key).is_some(), "{key}");
        }
        let mut changed = s.clone();
        changed.volume_size = Quantity(99 * GIB);
        changed.providers.clear();
        assert_eq!(
            PodStamp::of(&changed, IMAGE),
            stamp,
            "the volume is not pod-shaping"
        );
    }

    #[test]
    fn drift_names_the_differing_settings_in_form_order_and_is_empty_when_equal() {
        let s = settings();
        let on_pod = PodStamp::of(&s, IMAGE);
        assert!(drift(on_pod.as_str(), &PodStamp::of(&s, IMAGE)).is_empty());
        let mut t = s.clone();
        t.allowed
            .insert(crate::kube::credentials::KubeCredential::GitConfig);
        t.tolerations = vec![Toleration::parse("gpu:Exists:NoExecute").unwrap()];
        t.node_selector = BTreeMap::from([(LabelKey("a".into()), LabelValue("b".into()))]);
        t.resources.cpu_limit = Some(MilliCpus(1));
        let now = PodStamp::of(&t, "other:2");
        assert_eq!(
            drift(on_pod.as_str(), &now),
            [
                "image",
                "resources",
                "node selector",
                "tolerations",
                "allowed credentials"
            ]
        );
        assert_eq!(
            drift("not json", &on_pod).len(),
            5,
            "an unreadable stamp differs in all"
        );
    }
}
