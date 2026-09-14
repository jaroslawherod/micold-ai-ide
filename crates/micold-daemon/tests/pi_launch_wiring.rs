//! Feature 029 — what a Pi session is actually started with (quickstart §C.4 and §C.7, FR-012e).
//!
//! `activity_pipeline.rs` proves a declined component opens no tail, and `pi_provider.rs` proves the
//! provider's launch vector. Neither sees the process: the `-e` and `MICOLD_PI_ACTIVITY_LOG` are
//! added by the session service at spawn, which is the wiring the quickstart otherwise checks by
//! hand with `ps` and `/proc/<pid>/environ`. So a `pi` on `PATH` records its own arguments and the
//! environment it was given, and the test reads that record.

// unix-only: keeps the materialised Pi component out of the real data directory through `XDG_DATA_HOME`, which the Windows data directory (a known folder) ignores, so it would write into the signed-in user's own; its recording `pi` is a `#!/bin/sh` script
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::provider::ActivitySource;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::terminal::LaunchMode;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

const WITH_COMPONENT: u128 = 0x0029_0001;
const DECLINED: u128 = 0x0029_0002;

/// The component as it is shipped, to compare with what the session service wrote.
const COMPONENT: &str = include_str!("../assets/pi-activity.ts");

/// Every variable this test changes, restored on drop so a failure leaves nothing behind.
struct Env {
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl Env {
    fn set(vars: &[(&'static str, std::ffi::OsString)]) -> Self {
        let saved = vars
            .iter()
            .map(|(name, value)| {
                let previous = std::env::var_os(name);
                std::env::set_var(name, value);
                (*name, previous)
            })
            .collect();
        Self { saved }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for (name, previous) in self.saved.drain(..) {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// A `pi` that appends one line per launch: its arguments, tab separated, then the variables under
/// test, `|` separated. It then waits on its terminal, so it is still a live session when the record
/// is read. Tabs, because the macOS data directory is `Application Support`.
fn install_recording_pi(bin: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let launches = bin.join("launches");
    let command = bin.join(AiCli::Pi.provider().command());
    std::fs::write(
        &command,
        format!(
            "#!/bin/sh\nIFS=\"$(printf '\\t')\"\n\
             printf '%s|%s|%s|%s|%s\\n' \"$*\" \"${{MICOLD_PI_ACTIVITY_LOG-unset}}\" \
             \"${{PI_OFFLINE-unset}}\" \"${{PI_SKIP_VERSION_CHECK-unset}}\" \"${{PI_TELEMETRY-unset}}\" \
             >> '{}'\ncat > /dev/null\n",
            launches.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o755)).unwrap();
    launches
}

fn catalog_with_pi_sessions(project_dir: &Path, store_dir: &Path) -> Catalog {
    let session = |id: u128| {
        Session::restored(
            SessionId::from_uuid(Uuid::from_u128(id)),
            SessionLocation::Default,
            SessionLabel::Named("Pi".into()),
            TerminalMode::AiCli,
            AiCli::Pi,
        )
    };
    let mut sessions = BTreeMap::new();
    sessions.insert(
        project_dir.to_path_buf(),
        vec![session(WITH_COMPONENT), session(DECLINED)],
    );
    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace {
            projects: vec![Project::new(
                project_dir.to_path_buf(),
                true,
                Availability::Available,
            )],
            active: Some(project_dir.to_path_buf()),
            sessions,
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

/// The `n`th recorded launch, split into its fields, once it has been written.
fn launch(launches: &Path, n: usize) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(line) = std::fs::read_to_string(launches)
            .ok()
            .and_then(|body| body.lines().nth(n).map(str::to_string))
        {
            return line.split('|').map(str::to_string).collect();
        }
        assert!(Instant::now() < deadline, "launch {n} never reached `pi`");
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Both cases in one test: they change the same process-wide variables, and two tests doing that in
/// parallel would read each other's `PATH`.
#[test]
fn a_pi_session_carries_the_component_only_while_the_switch_is_on() {
    let bin = tempfile::tempdir().unwrap();
    let pi_home = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let launches = install_recording_pi(bin.path());

    let mut path = vec![bin.path().to_path_buf()];
    path.extend(
        std::env::var_os("PATH")
            .iter()
            .flat_map(std::env::split_paths),
    );
    let _env = Env::set(&[
        ("PATH", std::env::join_paths(path).unwrap()),
        ("PI_CODING_AGENT_DIR", pi_home.path().into()),
        // The component is materialised under the data directory; keep it out of the real one. That
        // is `$XDG_DATA_HOME` on Linux and under `$HOME/Library` on macOS.
        ("XDG_DATA_HOME", data_home.path().into()),
        ("HOME", data_home.path().into()),
    ]);
    assert!(AiCli::Pi.provider().is_available());

    let state = DaemonState::new(catalog_with_pi_sessions(project.path(), store.path()));

    // On, the default.
    let on = SessionId::from_uuid(Uuid::from_u128(WITH_COMPONENT));
    state.start_session(on, LaunchMode::Fresh).expect("starts");
    let fields = launch(&launches, 0);
    let args: Vec<&str> = fields[0].split('\t').collect();
    let ActivitySource::Extension { log } =
        AiCli::Pi
            .provider()
            .activity_source(pi_home.path(), project.path(), on.0)
    else {
        panic!("Pi reports activity through its component's log");
    };
    assert_eq!(args[..2], ["--session-id", on.0.to_string().as_str()]);
    let e = args
        .iter()
        .position(|arg| *arg == "-e")
        .expect("the component is loaded with `-e`");
    let component = Path::new(args[e + 1]);
    assert!(
        component.starts_with(data_home.path()),
        "materialised under the data directory, never in Pi's own extension folders: {component:?}"
    );
    assert_eq!(std::fs::read_to_string(component).unwrap(), COMPONENT);
    assert_eq!(fields[1], log.to_str().unwrap(), "the log the tail reads");
    assert!(
        log.parent().unwrap().is_dir(),
        "its directory exists before Pi starts"
    );
    assert_eq!(
        fields[2..],
        ["1", "1", "0"],
        "PI_OFFLINE, PI_SKIP_VERSION_CHECK, PI_TELEMETRY"
    );
    assert!(
        !pi_home.path().join("extensions").exists(),
        "nothing is installed into the user's own Pi"
    );
    state.live_session(on).unwrap().kill().unwrap();

    // Off: the same session in every other respect.
    state.set_pi_activity_component(false).expect("persists");
    let off = SessionId::from_uuid(Uuid::from_u128(DECLINED));
    state
        .start_session(off, LaunchMode::Fresh)
        .expect("still starts");
    let fields = launch(&launches, 1);
    assert_eq!(
        fields[0],
        format!("--session-id\t{}", off.0),
        "no `-e` and nothing else"
    );
    assert_eq!(fields[1], "unset", "no activity log variable");
    assert_eq!(
        fields[2..],
        ["1", "1", "0"],
        "the offline launch is not part of the switch"
    );
    state.live_session(off).unwrap().kill().unwrap();
}
