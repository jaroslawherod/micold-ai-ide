//! BUG-868 against a **real** container runtime: a container daemon added through the Daemons form
//! gets its container created with the form's own settings (name, picked port, image), and the
//! container answers as an SSH server on that port. Nothing here creates the container by hand.
//!
//! Behind `sandbox-real-runtime`: `mise run image && mise run test-sandbox`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use micold_client::daemon::{actor_specs_answered, ContainerStarter, Dial};
use micold_client::features::daemons::{self, DraftRuntime, Model, State};
use micold_core::daemons::{DaemonRegistry, DaemonStates};
use micold_core::provision::CancelToken;
use micold_core::sandbox::image::{ImageChoice, OwnImage};
use micold_core::sandbox::runtime::RuntimeKind;

fn runtime_kind() -> RuntimeKind {
    match std::env::var("MICOLD_TEST_RUNTIME").as_deref() {
        Err(_) | Ok("") | Ok("docker") => RuntimeKind::Docker,
        Ok("podman") => RuntimeKind::Podman,
        Ok(other) => panic!("MICOLD_TEST_RUNTIME={other:?}: expected `docker` or `podman`"),
    }
}

/// Removes the container whatever the test does.
struct Cleanup {
    program: &'static str,
    container: String,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = micold_core::process::no_window(std::process::Command::new(self.program).args([
            "rm",
            "-f",
            self.container.as_str(),
        ]))
        .output();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn sandbox_real_a_container_daemon_added_in_the_form_gets_its_container() {
    let runtime = runtime_kind();
    let image = std::env::var("MICOLD_BASIC_IMAGE_TAG")
        .ok()
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| micold_core::sandbox::basic_image::DEV_BASIC_IMAGE.to_string());
    let container = format!("micold-868-{}", std::process::id());
    let _cleanup = Cleanup {
        program: runtime.program(),
        container: container.clone(),
    };
    micold_client::daemon::install_ssh_host(Arc::new(crate::shell::daemons::system_ssh()));
    let state_dir = tempfile::tempdir().unwrap();

    // The form, filled as a user does: a name, a container name, the port left empty.
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let mut links = micold_client::links::DaemonLinks::at_launch(&registry);
    let (mut bindings, sessions) = (BTreeMap::new(), BTreeMap::new());
    let next_port = || micold_core::daemons::os_free_loopback_port();
    let mut form = State::default();
    daemons::begin_add(&mut form);
    let draft = form.draft.as_mut().unwrap();
    draft.name = "Added".into();
    draft.runtime = DraftRuntime::Container;
    draft.container_name = container.clone();
    draft.profile.runtime = runtime;
    draft.profile.image = ImageChoice::Own(OwnImage::Reference(image.clone()));
    draft.image_reference = image;
    daemons::save(
        &mut form,
        &mut Model {
            registry: &mut registry,
            links: &mut links,
            bindings: &mut bindings,
            legacy_default: None,
            legacy_container: None,
            locations: &[],
            sessions: &sessions,
            taken_elsewhere: Vec::new(),
            next_port: &next_port,
        },
    )
    .expect("the form saves");

    // The actor of the added daemon is the one that creates its container.
    let mut states = DaemonStates::default();
    for e in registry.entries() {
        states.insert(e.id, micold_core::daemons::DaemonState::Starting);
    }
    let spec = actor_specs_answered(
        &registry,
        &states,
        state_dir.path(),
        None,
        None,
        &BTreeMap::new(),
    )
    .pop()
    .expect("the added daemon has an actor");
    let Dial::ContainerSsh {
        port,
        container_name,
        bring_up: Some(setup),
        ..
    } = spec.dial
    else {
        panic!("the added container daemon is dialled over SSH and creates its container");
    };

    let starter = crate::shell::sandbox::DaemonContainers;
    let cancel = CancelToken::new();
    let name = container_name.clone();
    let ledger = tokio::task::spawn_blocking(move || {
        starter.bring_up(&name, port, &setup, &cancel, &|_| {})
    })
    .await
    .unwrap()
    .unwrap_or_else(|reason| panic!("the container did not come up: {reason}"));
    assert!(ledger.created_container.is_some(), "this start created it");

    // It is a running container of the form's name, and its SSH server answers on the form's port.
    let host = micold_client::daemon::ssh_host().expect("an ssh runner");
    let login = micold_core::provision::login::wait_for_login(
        &*host,
        runtime.program(),
        &container_name,
        Duration::from_secs(60),
    )
    .await
    .unwrap_or_else(|reason| panic!("the container {container_name} is not ready: {reason}"));
    assert!(!login.is_empty());
    let banner = tokio::time::timeout(Duration::from_secs(30), async {
        use tokio::io::AsyncReadExt;
        loop {
            if let Ok(mut s) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
                let mut buf = [0u8; 4];
                if s.read_exact(&mut buf).await.is_ok() {
                    return buf;
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    })
    .await
    .expect("sshd answers on the port the form picked");
    assert_eq!(&banner, b"SSH-");
}
