//! Feature 806 against a real container runtime: the basic image, started the way the client starts
//! it, provisioned over SSH by the same `provision::run` the client calls (T048, T059, T067).
//!
//! What the unit tests cannot say is whether the sequence holds against a real `sshd`, a real bind
//! mount and a real runtime: that a second start finds the build where the first left it, that a
//! container recreated under the same state directory copies and installs nothing, that a
//! container of a released image is recognised for what it is, and that an image with no SSH
//! server fails at the step that names it. The app-level pieces (the form, the notices, the
//! migration) are in the client crate's tests; the runtime calls the client makes around this
//! (`pull`, `create`) are exercised here directly through the CLI.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_provision -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::path::Path;
use std::time::{Duration, Instant};

use micold_core::connect::Connected;
use micold_core::provision::{FailureCause, Provisioned, Step, StepBounds};
use micold_core::sandbox::lifecycle::{adopt, Adoption};
use micold_core::sandbox::parse::ContainerFacts;
use micold_core::session::AiCli;

use sandbox_real_support::{
    cli, cli_out, image, provision_outcome, runtime_cmd, seed, start_sandbox, start_sandbox_from,
    wait_for_accept, SandboxSpec,
};

const FRESH_CONTAINER: &str = "micold-provision-probe";
const FRESH_NETWORK: &str = "micold-provision-probe-net";
const FRESH_PORT: u16 = 17761;

const LEGACY_CONTAINER: &str = "micold-provision-legacy-probe";
const LEGACY_NETWORK: &str = "micold-provision-legacy-probe-net";
const LEGACY_PORT: u16 = 17762;
/// A name under the released repository, given to the basic image locally: `is_released_image`
/// matches the repository, and a container created from this tag reports it as its image.
const LEGACY_TAG: &str = "ghcr.io/jaroslawherod/micold-daemon:provision-probe";

const OWN_CONTAINER: &str = "micold-provision-own-probe";
const OWN_NETWORK: &str = "micold-provision-own-probe-net";
const OWN_PORT: u16 = 17763;

const NO_SSH_CONTAINER: &str = "micold-provision-nossh-probe";
const NO_SSH_NETWORK: &str = "micold-provision-nossh-probe-net";
const NO_SSH_PORT: u16 = 17764;

const MIGRATED_CONTAINER: &str = "micold-provision-migrated-probe";
const MIGRATED_NETWORK: &str = "micold-provision-migrated-probe-net";
const MIGRATED_PORT: u16 = 17765;

/// The slowest a start that finds everything in place may take (SC-003, US1 #4).
const WARM_START: Duration = Duration::from_secs(15);

struct Dirs {
    _tmp: tempfile::TempDir,
    data: std::path::PathBuf,
    project: std::path::PathBuf,
    home: String,
}

fn dirs(label: &str) -> Dirs {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("data");
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, label);
    Dirs {
        _tmp: tmp,
        data,
        project,
        home: std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()),
    }
}

fn spec<'a>(d: &'a Dirs, container: &'a str, network: &'a str, port: u16) -> SandboxSpec<'a> {
    SandboxSpec {
        container,
        network,
        port,
        data_home: &d.data,
        project: &d.project,
        home: &d.home,
        survive_logout: false,
        extra: &[],
    }
}

/// The client's exact build, as the daemon names it in its welcome.
fn client_build() -> String {
    format!(
        "micold-daemon {}",
        micold_core::protocol::version::PACKAGE_VERSION
    )
}

/// Provision the container on `port` and require a daemon that reports this client's version.
async fn connected_with_client_version(port: u16) -> (Provisioned, Vec<Step>, Duration) {
    let started = Instant::now();
    let (container, outcome, steps) = provision_outcome(port, StepBounds::container()).await;
    let elapsed = started.elapsed();
    let provisioned = outcome.unwrap_or_else(|f| {
        panic!(
            "provisioning {container} failed at {:?}: {:?}\n{}",
            f.step, f.cause, f.output
        )
    });
    let Connected::Ready(_, welcome) = &provisioned.connected else {
        panic!("the daemon in {container} refused this client");
    };
    assert_eq!(
        welcome.daemon_build,
        client_build(),
        "the daemon in {container} must be the build this client carries (SC-001)"
    );
    (provisioned, steps, elapsed)
}

/// Whether the stand-in AI CLI the install step recorded runs inside `container`.
fn installed_cli_runs(container: &str, state: &Path) {
    let offered =
        micold_core::provision::ai_cli::read_offered(&state.join("micold-ai-ide/ai-clis"))
            .expect("offered.json is readable")
            .expect("the install step wrote offered.json");
    let entry = offered
        .clis
        .iter()
        .find(|c| c.cli == AiCli::ClaudeCode)
        .expect("the install step recorded the stand-in claude");
    let path = entry.bin.join(AiCli::ClaudeCode.provider().command());
    let out = runtime_cmd(sandbox_real_support::dialect().program)
        .args(["exec", container])
        .arg(&path)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run the runtime");
    assert!(
        out.status.success(),
        "the stand-in AI CLI at {} does not run: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// T048, SC-001, SC-003, FR-025: a fresh start reports the client's version and installs the
/// stand-in AI CLI from the table (no network); a second start with the container running copies
/// and installs nothing and connects inside the warm bound.
#[tokio::test]
async fn sandbox_real_provision_fresh_then_warm_start() {
    let d = dirs("fresh");
    let _sandbox = start_sandbox(&spec(&d, FRESH_CONTAINER, FRESH_NETWORK, FRESH_PORT));

    let (first, steps, _) = connected_with_client_version(FRESH_PORT).await;
    assert!(first.copied, "a fresh container has no daemon build yet");
    assert!(
        steps.contains(&Step::InstallAiClis),
        "a fresh container has no AI CLI yet; steps: {steps:?}"
    );
    installed_cli_runs(FRESH_CONTAINER, &d.data);
    drop(first);

    let (second, steps, elapsed) = connected_with_client_version(FRESH_PORT).await;
    assert!(!second.copied, "the build is already in the container");
    assert!(
        !steps.contains(&Step::Copy) && !steps.contains(&Step::InstallAiClis),
        "a warm start copies and installs nothing; steps: {steps:?}"
    );
    assert!(
        elapsed < WARM_START,
        "a warm start took {elapsed:?}, over {WARM_START:?} (SC-003)"
    );
}

/// T048, FR-011, edge case "Container recreated": after the container is removed the next start
/// recreates it from the same state directory and copies and installs nothing, because the build
/// and the CLIs live in the host-mounted state directory.
#[tokio::test]
async fn sandbox_real_provision_recreated_container_reuses_the_state_dir() {
    let d = dirs("recreated");
    let sandbox = start_sandbox(&spec(&d, FRESH_CONTAINER, FRESH_NETWORK, FRESH_PORT + 10));
    drop(connected_with_client_version(FRESH_PORT + 10).await);
    cli(&["rm", "-f", FRESH_CONTAINER]);
    drop(sandbox);

    let _again = start_sandbox(&spec(&d, FRESH_CONTAINER, FRESH_NETWORK, FRESH_PORT + 10));
    let (second, steps, _) = connected_with_client_version(FRESH_PORT + 10).await;
    assert!(
        !second.copied,
        "the recreated container finds the build in the state dir"
    );
    assert!(
        !steps.contains(&Step::Copy) && !steps.contains(&Step::InstallAiClis),
        "a recreated container copies and installs nothing; steps: {steps:?}"
    );
}

/// The facts the client reads off a container (`find`), from the runtime's own `inspect`.
fn facts_of(container: &str) -> ContainerFacts {
    let raw = cli_out(&[
        "inspect",
        "-f",
        "{{.Id}} {{.State.Running}} {{.Config.Image}}",
        container,
    ]);
    let mut parts = raw.split_whitespace();
    ContainerFacts {
        id: parts.next().unwrap_or_default().to_string(),
        running: parts.next() == Some("true"),
        image: parts.next().unwrap_or_default().to_string(),
        fingerprint: None,
        mount_destinations: Vec::new(),
    }
}

/// T059, US4 #3/#4, FR-017: a running container of a released image with a session is attached
/// (`Adoption::Legacy`), not recreated, and its session keeps running; once nothing runs, the
/// restart replaces it with a basic-image container reporting the client's version.
#[tokio::test]
async fn sandbox_real_provision_a_running_legacy_container_is_attached_then_replaced() {
    let d = dirs("legacy");
    cli(&["tag", &image(), LEGACY_TAG]);
    let _sandbox = start_sandbox_from(
        &spec(&d, LEGACY_CONTAINER, LEGACY_NETWORK, LEGACY_PORT),
        LEGACY_TAG,
        &[],
    );
    // A daemon with a session in it, standing for the released image's own.
    let (conn, catalog) = wait_for_accept(LEGACY_PORT).await;
    assert_eq!(
        catalog
            .projects
            .iter()
            .map(|p| p.sessions.len())
            .sum::<usize>(),
        1,
        "the seeded session is in the container"
    );

    let before = facts_of(LEGACY_CONTAINER);
    assert_eq!(
        adopt(Some(&before), &image()),
        Adoption::Legacy(micold_core::sandbox::runtime::ContainerId(
            before.id.clone()
        )),
        "a running released-image container is attached as it is"
    );
    assert_eq!(
        facts_of(LEGACY_CONTAINER).id,
        before.id,
        "attaching must not recreate the container"
    );
    assert!(
        sandbox_real_support::daemon_running(LEGACY_CONTAINER),
        "its session keeps running"
    );
    drop(conn);

    // The session has ended: the restart replaces the container with a basic-image one.
    sandbox_real_support::purge(LEGACY_CONTAINER, LEGACY_NETWORK);
    let _replacement = start_sandbox(&spec(&d, LEGACY_CONTAINER, LEGACY_NETWORK, LEGACY_PORT));
    assert_eq!(facts_of(LEGACY_CONTAINER).image, image());
    drop(connected_with_client_version(LEGACY_PORT).await);
    let _ = runtime_cmd(sandbox_real_support::dialect().program)
        .args(["rmi", LEGACY_TAG])
        .output();
}

/// T067, US3 #1: the basic image named as the user's own image connects with the client's version.
#[tokio::test]
async fn sandbox_real_provision_own_image_that_runs_ssh_connects() {
    let d = dirs("own");
    let _sandbox = start_sandbox_from(
        &spec(&d, OWN_CONTAINER, OWN_NETWORK, OWN_PORT),
        micold_core::sandbox::basic_image::DEV_BASIC_IMAGE,
        &[],
    );
    drop(connected_with_client_version(OWN_PORT).await);
}

/// T067, US3 #2: an image with no SSH server fails in `Reach` as `NoSshServer`.
#[tokio::test]
async fn sandbox_real_provision_own_image_without_ssh_fails_in_reach() {
    let d = dirs("nossh");
    let _sandbox = start_sandbox_from(
        &spec(&d, NO_SSH_CONTAINER, NO_SSH_NETWORK, NO_SSH_PORT),
        "docker.io/library/alpine:3",
        &["sleep", "600"],
    );
    let bounds = StepBounds {
        reach: Duration::from_secs(10),
        ..StepBounds::container()
    };
    let (_, outcome, steps) = provision_outcome(NO_SSH_PORT, bounds).await;
    let Err(failure) = outcome else {
        panic!("an image with no SSH server must not provision");
    };
    assert_eq!(
        (failure.step, &failure.cause),
        (Step::Reach, &FailureCause::NoSshServer),
        "steps entered: {steps:?}; output: {}",
        failure.output
    );
}

/// T067, US3 #3: a reference that does not exist fails to pull through the client's own runtime
/// driver, and the error names the image with the runtime's output.
#[test]
fn sandbox_real_provision_a_nonexistent_image_reference_names_itself() {
    use micold_core::sandbox::exec::SystemRunner;
    use micold_core::sandbox::image::{ImageChoice, OwnImage};
    use micold_core::sandbox::runtime::{ContainerRuntime, RuntimeError};

    let reference = "micold-nonexistent.invalid/probe:0";
    let runtime = micold_core::sandbox::cli::CliRuntime::new(
        sandbox_real_support::runtime_kind(),
        SystemRunner,
    );
    let choice = ImageChoice::Own(OwnImage::Reference(reference.to_string()));
    let error = runtime
        .acquire_image(&choice, &mut |_| {})
        .expect_err("pulling a nonexistent reference must fail");
    let named = match &error {
        RuntimeError::ImageNotFound { reference: r } => r.clone(),
        RuntimeError::ImagePullFailed {
            reference: r,
            detail,
        } => format!("{r} {detail}"),
        other => panic!("expected the pull to fail naming the image, got {other:?}"),
    };
    assert!(
        named.contains("micold-nonexistent.invalid"),
        "the error must name the image so the row can: {error:?}"
    );
}

/// T059, US4 #1: settings naming a released-image reference read as the basic image with the
/// notice set; the container started from that choice connects with the client's version, and
/// clearing the notice (what a successful start does) leaves none.
#[tokio::test]
async fn sandbox_real_provision_a_released_image_setting_starts_on_the_basic_image() {
    use micold_core::daemons::{DaemonRuntime, MigrationNotice};
    use micold_core::sandbox::image::ImageChoice;
    use micold_core::settings::{JsonFileSettingsStore, SettingsStore};

    let d = dirs("migrated");
    let path = d.data.join("settings.json");
    std::fs::write(
        &path,
        r#"{ "settings_version": 4, "daemon": { "placement": "local_sandbox", "sandbox": {
            "runtime": "docker", "network": "outbound", "survive_logout": false, "credentials": [],
            "budget": { "cpus_milli": 2000, "memory_bytes": 4294967296, "pids": 512, "storage_bytes": null },
            "image": { "kind": "registry", "path": null,
                       "reference": "ghcr.io/jaroslawherod/micold-daemon:0.9.0" } } } }"#,
    )
    .unwrap();
    let settings = JsonFileSettingsStore::at(path).load().settings;
    assert_eq!(settings.daemon.sandbox.image, ImageChoice::Basic);
    let entry = settings
        .daemons
        .iter()
        .find(|e| matches!(&e.runtime, DaemonRuntime::Container(_)))
        .expect("the migrated container daemon");
    let id = entry.id;
    let DaemonRuntime::Container(c) = &entry.runtime else {
        unreachable!()
    };
    assert_eq!(c.migration, Some(MigrationNotice::MovedOffReleasedImage));

    let _sandbox = start_sandbox(&spec(
        &d,
        MIGRATED_CONTAINER,
        MIGRATED_NETWORK,
        MIGRATED_PORT,
    ));
    drop(connected_with_client_version(MIGRATED_PORT).await);
    let mut registry = micold_core::daemons::DaemonRegistry::new(settings.daemons.clone(), 2);
    assert!(registry.clear_migration(id), "a start clears the notice");
    assert!(registry.entries().iter().all(|e| match &e.runtime {
        DaemonRuntime::Container(c) => c.migration.is_none(),
        _ => true,
    }));
}
