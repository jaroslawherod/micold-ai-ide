//! SC-004, measured: how long a first enable takes, and whether it ever goes quiet.
//!
//! *"First-time enablement, including preparing the sandbox on a working network connection,
//! completes within 5 minutes and shows continuous progress throughout, so the user never has to
//! guess whether the application has stopped responding."*
//!
//! ## Which "preparing" this measures
//!
//! The application can reach an image three ways ([`ImageSourceKind`]): pull it from a registry,
//! import it from a file, or be told to build it locally. Only two of those can be measured on this
//! repository today.
//!
//! - **Registry** is unmeasurable: nothing is published yet, so there is no reference to pull. That
//!   gap is exactly why SC-004a and SC-004b exist as separate criteria.
//! - **LocalBuild** is `mise run image`, and `acquire_image` deliberately refuses to run it — a
//!   cross-compiled Linux binary staged beside a Containerfile is a build-system job. Its duration
//!   belongs to SC-004b and is recorded in the evidence by hand.
//! - **ImportedFile** is the route this drives: the documented no-network procedure of SC-004a, and
//!   the one acquisition path that both streams progress and can be run from a cold reference here.
//!
//! The cold state is made by tagging the built image under a throwaway reference, saving that to a
//! tar, and deleting the tag. It is honestly *a* cold reference and not a cold machine: the layers
//! stay in the local store, so `load` moves less data than a first pull would. The evidence says so.
//! What is bounded here is the application's own enable sequence — acquire, create, start, answer —
//! with the transfer size called out rather than hidden inside the total.
//!
//! Behind `sandbox-real-runtime` for the reason every real-runtime target here is: the default
//! suite must need nothing installed on any platform (Principle VI).

#![cfg(feature = "sandbox-real-runtime")]

use std::path::PathBuf;
/// The host home path, which is what the sandbox's `HOME` is set to (research R2).
fn host_home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

/// The application-owned directory mounted at that path (FR-004d), created before the container is.
///
/// The runtime would otherwise create the missing bind source itself, as **root** — leaving the uid
/// the container runs as unable to write to its own `HOME`, which is the failure the mount exists to
/// fix arriving by another route. `shell::sandbox::start` creates it for the same reason.
fn make_sandbox_home(state: &std::path::Path) {
    std::fs::create_dir_all(state.join(micold_core::sandbox::SANDBOX_HOME_DIR)).unwrap();
}

use std::process::Command;
use std::time::{Duration, Instant};

use micold_core::protocol::auth::Token;
use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::SystemRunner;
use micold_core::sandbox::image::{ImageSource, ImageSourceKind};
use micold_core::sandbox::runtime::{ContainerRuntime, Progress, RuntimeKind};
use micold_core::sandbox::{CredentialLayout, MountSet, SandboxProfile, SandboxSpec, SecretMount};

const IMAGE: &str = "micold-daemon:dev";
/// A throwaway tag, so making a reference cold never touches the one a developer is using.
const PROBE_TAG: &str = "micold-enable-probe:sc004";
const CONTAINER: &str = "micold-real-enable";
const NETWORK: &str = "micold-real-enable-net";
/// Nothing connects over this; it exists because every spec publishes a control port. Distinct from
/// the default so a developer's own sandbox cannot lose the bind while this runs.
const PORT: u16 = 17730;

/// The whole enable must fit in this. SC-004's number, not a tolerance chosen to pass.
const BUDGET: Duration = Duration::from_secs(5 * 60);

/// The longest the indicator may stand still during acquisition.
///
/// SC-004's second half is about a user's confidence that the application is alive, which is a
/// claim about *gaps*, not about totals — an enable that finishes in a minute after four silent
/// ones still fails it. Ten seconds is the threshold at which a still indicator starts reading as a
/// hang rather than as work.
const MAX_SILENCE: Duration = Duration::from_secs(10);

fn docker(args: &[&str]) -> std::process::Output {
    Command::new("docker")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("docker {args:?}: {e}"))
}

fn docker_ok(args: &[&str]) {
    let out = docker(args);
    assert!(
        out.status.success(),
        "docker {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Removes everything this test brought into being, whichever way the test leaves.
struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = docker(&["rm", "-f", CONTAINER]);
        let _ = docker(&["network", "rm", NETWORK]);
        let _ = docker(&["rmi", "-f", PROBE_TAG]);
    }
}

#[test]
fn sandbox_real_first_enable_is_under_five_minutes_and_never_goes_quiet() {
    let _cleanup = Cleanup;
    let _ = docker(&["rm", "-f", CONTAINER]);
    let _ = docker(&["network", "rm", NETWORK]);

    let dir = tempfile::tempdir().expect("temp dir");
    let state = dir.path().join("state");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&state).unwrap();
    make_sandbox_home(&state);
    std::fs::create_dir_all(&project).unwrap();

    // --- make the reference cold -------------------------------------------------------------
    // Not timed: this is the test building a starting state, not the application doing work.
    let archive = dir.path().join("sandbox-image.tar");
    docker_ok(&["tag", IMAGE, PROBE_TAG]);
    docker_ok(&["save", "-o", archive.to_str().unwrap(), PROBE_TAG]);
    docker_ok(&["rmi", PROBE_TAG]);
    let archive_bytes = std::fs::metadata(&archive).unwrap().len();
    assert!(
        docker(&["image", "inspect", PROBE_TAG]).status.code() != Some(0),
        "the probe reference must be absent, or nothing is being acquired"
    );

    let token = Token::generate();
    let token_path = state.join("sandbox.token");
    token.write_to(&token_path).unwrap();

    let profile = SandboxProfile {
        runtime: RuntimeKind::Docker,
        image: ImageSource {
            kind: ImageSourceKind::ImportedFile,
            reference: PROBE_TAG.to_string(),
            path: Some(archive.clone()),
        },
        ..SandboxProfile::default()
    };
    let (uid, gid) = micold_core::sandbox::host_identity();
    let spec = SandboxSpec {
        name: CONTAINER.to_string(),
        profile: profile.clone(),
        mounts: MountSet::build(
            std::slice::from_ref(&project),
            &profile,
            &CredentialLayout::default(),
            state.clone(),
            &host_home(),
            SecretMount {
                host: token_path,
                container: PathBuf::from("/run/micold/token"),
            },
        ),
        uid,
        gid,
        control_port: PORT,
        published_ports: Vec::new(),
        network_name: NETWORK.to_string(),
        home: host_home(),
    };

    let runtime = CliRuntime::new(RuntimeKind::Docker, SystemRunner);

    // --- the enable, timed --------------------------------------------------------------------
    let started = Instant::now();

    let mut reports: Vec<(Duration, Progress)> = Vec::new();
    runtime
        .acquire_image(&profile.image, &mut |p| {
            reports.push((started.elapsed(), p))
        })
        .unwrap_or_else(|e| panic!("acquire: {}", e.reason()));
    let acquired = started.elapsed();

    docker_ok(&["network", "create", "--driver", "bridge", NETWORK]);
    let id = runtime
        .create(&spec)
        .unwrap_or_else(|e| panic!("create: {}", e.reason()));
    let created = started.elapsed();

    runtime
        .start(&id)
        .unwrap_or_else(|e| panic!("start: {}", e.reason()));
    let launched = started.elapsed();

    // "Enabled" is the daemon answering, not the container existing. The daemon writes its own log
    // into the state directory, which is a host path here, so this reads the daemon's word for it
    // rather than the runtime's.
    let log = state.join("micold-daemon.log");
    let ready_deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if std::fs::read_to_string(&log)
            .map(|s| s.contains("listening (sandboxed)"))
            .unwrap_or(false)
        {
            break;
        }
        assert!(
            Instant::now() < ready_deadline,
            "the daemon never reported itself listening within 60s of start; log so far:\n{}",
            std::fs::read_to_string(&log).unwrap_or_else(|_| "<no log written>".into())
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let total = started.elapsed();

    // --- what it did ---------------------------------------------------------------------------
    println!(
        "SC-004 enable: total {}ms — acquire {}ms, create {}ms, start {}ms, answer {}ms \
         (archive {:.1} MiB)",
        total.as_millis(),
        acquired.as_millis(),
        (created - acquired).as_millis(),
        (launched - created).as_millis(),
        (total - launched).as_millis(),
        archive_bytes as f64 / (1024.0 * 1024.0),
    );
    println!(
        "SC-004 progress: {} reports during acquisition, stages {:?}",
        reports.len(),
        reports
            .iter()
            .map(|(_, p)| p.stage.as_str())
            .collect::<Vec<_>>(),
    );

    assert!(
        total <= BUDGET,
        "SC-004: the enable took {}s, over the 5 minute budget",
        total.as_secs()
    );

    // --- and whether it ever went quiet --------------------------------------------------------
    assert!(
        !reports.is_empty(),
        "acquisition reported no progress at all — a user watching this sees a still indicator for \
         its whole duration"
    );
    let mut previous = Duration::ZERO;
    let mut longest = Duration::ZERO;
    for (at, _) in &reports {
        longest = longest.max(*at - previous);
        previous = *at;
    }
    longest = longest.max(acquired - previous);
    println!(
        "SC-004 longest silence during acquisition: {}ms",
        longest.as_millis()
    );
    assert!(
        longest <= MAX_SILENCE,
        "the progress indicator stood still for {}ms during acquisition",
        longest.as_millis()
    );
}
