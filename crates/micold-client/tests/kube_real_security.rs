//! Feature 686, T092 (US3; SC-004): the live pod runs non-root with a read-only root filesystem and
//! mounts exactly its access secret when no credential is allowed; its SSH server accepts only the
//! access secret's keys.
#![cfg(feature = "kube-real-runtime")]

mod kube_real_support;

use kube_real_support::*;
use micold_core::kube::names::ResourceNames;

const NAME: &str = "Secure box";

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_the_pod_is_locked_down_and_accepts_only_the_access_keys() {
    let test = "kube_real_the_pod_is_locked_down_and_accepts_only_the_access_keys";
    let Some(_suite) = suite_or_skip(test) else {
        return;
    };
    let ns = Namespace::create("kt-sec");
    let client = client();
    let events = Events::default();
    let (_conn, _welcome, relay) = start(&client, NAME, &settings(&ns.0, &[]), &events)
        .await
        .expect("starts");
    let names = ResourceNames::new(CONTEXT, &ns.0, NAME);
    let json = kubectl(
        &["-n", &ns.0, "get", "pod", &names.pod(), "-o", "json"],
        None,
    )
    .unwrap();
    let pod: serde_json::Value = serde_json::from_str(&json).unwrap();
    let container = &pod["spec"]["containers"][0]["securityContext"];
    assert_eq!(container["readOnlyRootFilesystem"], true, "{container}");
    assert_eq!(container["allowPrivilegeEscalation"], false, "{container}");
    let run_as_non_root =
        pod["spec"]["securityContext"]["runAsNonRoot"] == true || container["runAsNonRoot"] == true;
    assert!(
        run_as_non_root,
        "the pod runs as non-root: {}",
        pod["spec"]["securityContext"]
    );
    let secrets: Vec<String> = pod["spec"]["volumes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["secret"]["secretName"].as_str().map(str::to_string))
        .collect();
    assert_eq!(
        secrets,
        vec![names.access_secret()],
        "only the access secret is mounted"
    );
    // A key the access secret does not list is refused, and a password is never offered.
    let stray = tempfile::tempdir().unwrap();
    let key = stray.path().join("stray");
    assert!(std::process::Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&key)
        .status()
        .unwrap()
        .success());
    let addr = relay.addr();
    let out = std::process::Command::new("ssh")
        .args(["-i"])
        .arg(&key)
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "UserKnownHostsFile=/dev/null",
            "-o",
            "ConnectTimeout=20",
            "-p",
            &addr.port().to_string(),
            "dev@127.0.0.1",
            "true",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a stray key was accepted");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("Permission denied"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
