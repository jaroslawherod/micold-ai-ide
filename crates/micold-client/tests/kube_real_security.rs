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
        vec![names.access_secret(), names.startup_secret()],
        "only the access secret and the optional startup secret (#1533) are mounted"
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
            &format!("{}@127.0.0.1", micold_client::daemon::POD_SSH_USER),
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

/// #1528 (daemon-config-file quickstart §2, pod row): the pod's env has no `MICOLD_*`, its
/// `daemon.toml` is the config map mounted read-only (no `subPath`), the daemon started from it, and a
/// config map the application replaces reaches the file in the pod (the kubelet's own delay, printed).
#[tokio::test(flavor = "multi_thread")]
async fn kube_real_the_pod_reads_its_configuration_from_the_config_map() {
    let test = "kube_real_the_pod_reads_its_configuration_from_the_config_map";
    let Some(_suite) = suite_or_skip(test) else {
        return;
    };
    let ns = Namespace::create("kt-cfg");
    let client = client();
    let events = Events::default();
    let (_conn, _welcome, _relay) = start(&client, "Config box", &settings(&ns.0, &[]), &events)
        .await
        .expect("starts");
    let names = ResourceNames::new(CONTEXT, &ns.0, "Config box");
    let get = |kind: &str, name: &str| -> serde_json::Value {
        let json = kubectl(&["-n", &ns.0, "get", kind, name, "-o", "json"], None).unwrap();
        serde_json::from_str(&json).unwrap()
    };
    let pod = get("pod", &names.pod());
    let container = &pod["spec"]["containers"][0];
    let env: Vec<&str> = container["env"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["name"].as_str())
        .collect();
    assert!(env.iter().all(|n| !n.starts_with("MICOLD_")), "{env:?}");
    assert!(env.contains(&"LANG"), "{env:?}");
    let mount = container["volumeMounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["mountPath"] == "/run/micold/config")
        .expect("the config map is mounted");
    assert_eq!(mount["readOnly"], true, "{mount}");
    assert!(mount.get("subPath").is_none(), "{mount}");

    let map = get("configmap", &names.config_map());
    let file = map["data"]["daemon.toml"].as_str().unwrap().to_string();
    let in_pod = kubectl(
        &[
            "-n",
            &ns.0,
            "exec",
            &names.pod(),
            "--",
            "cat",
            "/run/micold/config/daemon.toml",
        ],
        None,
    )
    .unwrap();
    assert_eq!(in_pod, file, "the pod sees the config map's file");
    assert!(file.contains("kind = \"kubernetes\""), "{file}");

    // The application updates the config map; the kubelet's own propagation delay is measured.
    let edited = format!("{file}# edited\n");
    let patch = serde_json::json!({ "data": { "daemon.toml": edited } }).to_string();
    kubectl(
        &[
            "-n",
            &ns.0,
            "patch",
            "configmap",
            &names.config_map(),
            "--type=merge",
            "-p",
            &patch,
        ],
        None,
    )
    .unwrap();
    let began = std::time::Instant::now();
    loop {
        let now = kubectl(
            &[
                "-n",
                &ns.0,
                "exec",
                &names.pod(),
                "--",
                "cat",
                "/run/micold/config/daemon.toml",
            ],
            None,
        )
        .unwrap();
        if now == edited {
            break;
        }
        assert!(
            began.elapsed() < std::time::Duration::from_secs(180),
            "the edited file never reached the pod"
        );
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    println!(
        "config map update reached the pod after {:?}",
        began.elapsed()
    );
}
