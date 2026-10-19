//! Stop, Remove and Forget locally of a Kubernetes daemon (feature 686, contract §5, FR-014,
//! FR-015). Deletes name the object and carry the UID read in the same step (research K5); a `404`
//! counts as done, and an object that is not ours is never deleted.

use std::collections::BTreeSet;
use std::time::Duration;

use crate::kube::classify::KubeFailure;
use crate::kube::credentials::KubeCredential;
use crate::kube::names::{is_ours, ResourceNames};
use crate::kube::start::POLL;
use crate::kube::{Cluster, ObjectKind};

/// The pod's termination grace on Stop and Remove; the live adapter's delete parameters use it.
pub const POD_GRACE: Duration = Duration::from_secs(10);

/// How long Stop waits for the pod to be gone.
pub const STOP_WAIT: Duration = Duration::from_secs(crate::kube::classify::STOP_WAIT_SECS);

/// What stays in the cluster when the daemon is forgotten on this computer only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leftovers {
    /// The pod.
    pub pod: String,
    /// The volume claim.
    pub pvc: String,
    /// The secrets: the access secret first, then one per allowed credential.
    pub secrets: Vec<String>,
}

/// Delete `name` if it exists and is ours, by the UID read now. The UID deleted, if anything was;
/// a `404` at any step counts as done.
async fn delete_ours(
    cluster: &dyn Cluster,
    namespace: &str,
    names: &ResourceNames,
    kind: ObjectKind,
    name: &str,
) -> Result<Option<String>, KubeFailure> {
    let Some(view) = cluster.get(namespace, kind, name).await? else {
        return Ok(None);
    };
    if !is_ours(&view.labels, names.key()) {
        return Ok(None);
    }
    match cluster.delete(namespace, kind, name, &view.uid).await {
        Ok(()) => Ok(Some(view.uid)),
        Err(failure) if failure.code == 404 => Ok(None),
        Err(failure) => Err(failure.into()),
    }
}

/// Wait until the pod of `uid` is gone (a pod of the same name with another UID is a new one),
/// else `StopTimeout`.
async fn wait_gone(
    cluster: &dyn Cluster,
    namespace: &str,
    pod: &str,
    uid: &str,
) -> Result<(), KubeFailure> {
    let gone = async {
        while let Some(view) = cluster.get(namespace, ObjectKind::Pod, pod).await? {
            if view.uid != uid {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
        Ok::<_, KubeFailure>(())
    };
    tokio::time::timeout(STOP_WAIT, gone)
        .await
        .unwrap_or_else(|_| {
            Err(KubeFailure::StopTimeout {
                pod: pod.to_string(),
            })
        })
}

/// Stop: delete the pod and wait until it is gone; the claim and the secrets stay.
pub async fn stop(
    cluster: &dyn Cluster,
    namespace: &str,
    names: &ResourceNames,
) -> Result<(), KubeFailure> {
    let pod = names.pod();
    match delete_ours(cluster, namespace, names, ObjectKind::Pod, &pod).await? {
        Some(uid) => wait_gone(cluster, namespace, &pod, &uid).await,
        None => Ok(()),
    }
}

/// Remove: delete the pod and wait for it, every secret the daemon can own (the access secret
/// included) and the claim last, ours only. `allowed` names the credential secrets.
pub async fn remove(
    cluster: &dyn Cluster,
    namespace: &str,
    names: &ResourceNames,
    allowed: &BTreeSet<KubeCredential>,
) -> Result<(), KubeFailure> {
    stop(cluster, namespace, names).await?;
    for secret in names
        .all_secrets(allowed)
        .into_iter()
        .chain(names.retired_secrets())
    {
        delete_ours(cluster, namespace, names, ObjectKind::Secret, &secret).await?;
    }
    let pvc = names.pvc();
    delete_ours(
        cluster,
        namespace,
        names,
        ObjectKind::PersistentVolumeClaim,
        &pvc,
    )
    .await?;
    Ok(())
}

/// Forget locally: the names the cluster keeps, from the allowed credentials alone, with no
/// cluster call.
pub fn leftovers(allowed: &BTreeSet<KubeCredential>, names: &ResourceNames) -> Leftovers {
    Leftovers {
        pod: names.pod(),
        pvc: names.pvc(),
        secrets: names.all_secrets(allowed),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tokio::time::Instant;

    use super::*;
    use crate::kube::credentials::KubeCredential;
    use crate::kube::fake::FakeCluster;
    use crate::kube::ObjectKind;
    use crate::session::AiCli;

    const NS: &str = "dev";

    fn names() -> ResourceNames {
        ResourceNames::new("kind-dev", NS, "Build box")
    }

    /// The credentials the daemon was allowed.
    fn allowed() -> BTreeSet<KubeCredential> {
        [
            KubeCredential::GitConfig,
            KubeCredential::GitCredentials,
            KubeCredential::SignIn {
                cli: AiCli::ClaudeCode,
            },
        ]
        .into_iter()
        .collect()
    }

    /// A cluster holding the daemon's pod, claim, access secret and two credential secrets, all
    /// ours.
    fn running_daemon() -> FakeCluster {
        let fake = FakeCluster::new();
        fake.add_namespace(NS);
        let n = names();
        fake.insert(NS, ObjectKind::Pod, &n.pod(), n.labels());
        fake.insert(NS, ObjectKind::PersistentVolumeClaim, &n.pvc(), n.labels());
        for secret in [
            n.access_secret(),
            n.secret(&KubeCredential::GitConfig),
            n.secret(&KubeCredential::SignIn {
                cli: AiCli::ClaudeCode,
            }),
        ] {
            fake.insert(NS, ObjectKind::Secret, &secret, n.labels());
        }
        fake
    }

    #[test]
    fn the_pod_gets_ten_seconds_grace_and_stop_waits_two_minutes() {
        assert_eq!(POD_GRACE, Duration::from_secs(10));
        assert_eq!(STOP_WAIT, Duration::from_secs(120));
    }

    #[tokio::test(start_paused = true)]
    async fn stop_deletes_the_pod_by_uid_and_keeps_the_claim_and_secrets() {
        let fake = running_daemon();
        stop(&fake, NS, &names()).await.unwrap();
        assert!(fake.names(NS, ObjectKind::Pod).is_empty());
        assert_eq!(fake.names(NS, ObjectKind::PersistentVolumeClaim).len(), 1);
        assert_eq!(fake.names(NS, ObjectKind::Secret).len(), 3);
        let calls = fake.calls();
        let pod = names().pod();
        assert_eq!(
            calls,
            [
                format!("get Pod {pod}"),
                format!("delete Pod {pod}"),
                format!("get Pod {pod}")
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn stop_waits_for_a_terminating_pod_and_gives_up_after_its_bound() {
        let fake = running_daemon();
        fake.hold_on_delete(NS, ObjectKind::Pod, &names().pod());
        let t0 = Instant::now();
        let failure = stop(&fake, NS, &names()).await.unwrap_err();
        assert_eq!(failure, KubeFailure::StopTimeout { pod: names().pod() });
        assert!(t0.elapsed() >= STOP_WAIT);
        assert!(t0.elapsed() < STOP_WAIT + Duration::from_secs(5));
    }

    #[tokio::test(start_paused = true)]
    async fn stop_of_a_pod_already_gone_is_done() {
        let fake = FakeCluster::new();
        stop(&fake, NS, &names()).await.unwrap();
        assert!(!fake.calls().iter().any(|c| c.starts_with("delete")));
    }

    #[tokio::test(start_paused = true)]
    async fn remove_deletes_pod_secrets_and_claim_that_are_ours() {
        let fake = running_daemon();
        let foreign = names().secret(&KubeCredential::GitCredentials);
        fake.insert(NS, ObjectKind::Secret, &foreign, BTreeMap::new());
        remove(&fake, NS, &names(), &allowed()).await.unwrap();
        assert!(fake.names(NS, ObjectKind::Pod).is_empty());
        assert!(fake.names(NS, ObjectKind::PersistentVolumeClaim).is_empty());
        assert_eq!(
            fake.names(NS, ObjectKind::Secret),
            [foreign],
            "a secret not ours is kept"
        );
        let deletes: Vec<String> = fake
            .calls()
            .into_iter()
            .filter(|c| c.starts_with("delete"))
            .collect();
        assert_eq!(
            deletes.first().unwrap(),
            &format!("delete Pod {}", names().pod())
        );
        assert_eq!(
            deletes.last().unwrap(),
            &format!("delete PersistentVolumeClaim {}", names().pvc()),
            "the claim goes last"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn remove_counts_a_404_as_done() {
        let fake = running_daemon();
        fake.fail_next("delete", crate::kube::fake::not_found("pod"));
        remove(&fake, NS, &names(), &allowed()).await.unwrap();
        let empty = FakeCluster::new();
        remove(&empty, NS, &names(), &allowed()).await.unwrap();
    }

    #[test]
    fn forget_locally_lists_the_deterministic_names_without_a_cluster() {
        let allowed: BTreeSet<KubeCredential> = [
            KubeCredential::GitConfig,
            KubeCredential::SignIn { cli: AiCli::Codex },
        ]
        .into_iter()
        .collect();
        let n = names();
        assert_eq!(
            leftovers(&allowed, &n),
            Leftovers {
                pod: n.pod(),
                pvc: n.pvc(),
                secrets: vec![
                    n.access_secret(),
                    n.secret(&KubeCredential::GitConfig),
                    n.secret(&KubeCredential::SignIn { cli: AiCli::Codex }),
                ],
            },
            "the access secret is among the names the cluster keeps"
        );
    }
}
