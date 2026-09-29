//! Finding `gh` the way a desktop-launched app must (feature 034, FR-026,
//! contracts/github-issue-source.md §1, research R3).
//!
//! Every case names its `HostOs` explicitly and probes a fake file set, so the macOS, Linux and
//! Windows tables are all exercised on every CI host.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use micold_core::github::{candidate_dirs, env_include_path, locate_gh, HostOs, LocateInputs};

/// The `PATH` a Dock-launched macOS app inherits from `launchd`.
const DOCK_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";

fn no_env(_: &str) -> Option<String> {
    None
}

fn nothing_exists(_: &Path) -> bool {
    false
}

#[test]
fn separator_and_exe_come_from_host_os() {
    assert_eq!(
        (HostOs::Linux.path_separator(), HostOs::Linux.exe_name()),
        (':', "gh")
    );
    assert_eq!(
        (HostOs::MacOs.path_separator(), HostOs::MacOs.exe_name()),
        (':', "gh")
    );
    assert_eq!(
        (HostOs::Windows.path_separator(), HostOs::Windows.exe_name()),
        (';', "gh.exe")
    );
}

#[test]
fn candidate_order() {
    let inputs = LocateInputs {
        os: HostOs::Linux,
        env_include_path: Some("/from/profile::/shared"),
        process_path: "/shared:/from/process:",
        home: None,
        env: &no_env,
        exists: &nothing_exists,
    };
    let dirs = candidate_dirs(&inputs);
    let position = |d: &str| {
        dirs.iter()
            .position(|p| p == Path::new(d))
            .unwrap_or_else(|| panic!("{d} is a candidate: {dirs:?}"))
    };
    assert_eq!(
        dirs.iter().take(3).cloned().collect::<Vec<_>>(),
        [
            PathBuf::from("/from/profile"),
            PathBuf::from("/shared"),
            PathBuf::from("/from/process"),
        ],
        "env-include PATH first, then process PATH; a duplicate keeps its first position and \
         empty components are dropped"
    );
    assert!(
        position("/from/process") < position("/snap/bin"),
        "the well-known directories come after both PATHs"
    );
    assert_eq!(
        dirs.iter().filter(|d| *d == Path::new("/shared")).count(),
        1,
        "each directory is probed once"
    );
    assert!(
        !dirs.iter().any(|d| d.as_os_str().is_empty()),
        "no empty candidate: {dirs:?}"
    );
}

#[test]
fn macos_dock_launch_finds_homebrew_gh() {
    let homebrew_gh = Path::new("/opt/homebrew/bin").join("gh");
    let exists = |p: &Path| p == homebrew_gh;
    let inputs = LocateInputs {
        os: HostOs::MacOs,
        env_include_path: None,
        process_path: DOCK_PATH,
        home: Some(Path::new("/Users/me")),
        env: &no_env,
        exists: &exists,
    };
    assert_eq!(
        locate_gh(&inputs),
        Some(homebrew_gh.clone()),
        "a Dock launch cannot see Homebrew on its PATH, so the well-known table finds it"
    );
}

#[test]
fn windows_finds_winget_gh() {
    let vars: HashMap<&str, &str> = HashMap::from([
        ("LOCALAPPDATA", r"C:\Users\me\AppData\Local"),
        ("ProgramFiles", r"C:\Program Files"),
        ("USERPROFILE", r"C:\Users\me"),
    ]);
    let env = |k: &str| vars.get(k).map(|v| v.to_string());
    let winget_gh =
        PathBuf::from(r"C:\Users\me\AppData\Local\Microsoft\WinGet\Links").join("gh.exe");
    let exists = |p: &Path| p == winget_gh;
    let inputs = LocateInputs {
        os: HostOs::Windows,
        env_include_path: None,
        process_path: r"C:\Windows\system32;C:\Windows",
        home: Some(Path::new(r"C:\Users\me")),
        env: &env,
        exists: &exists,
    };
    assert_eq!(locate_gh(&inputs), Some(winget_gh.clone()));

    let dirs = candidate_dirs(&inputs);
    for expected in [
        r"C:\Windows\system32",
        r"C:\Program Files\GitHub CLI",
        r"C:\Users\me\AppData\Local\Microsoft\WinGet\Links",
        r"C:\Users\me\scoop\shims",
    ] {
        assert!(
            dirs.contains(&PathBuf::from(expected)),
            "{expected} is a Windows candidate, split on `;`: {dirs:?}"
        );
    }
    assert!(
        !dirs
            .iter()
            .any(|d| d.to_string_lossy().contains("chocolatey")),
        "a well-known directory whose variable is unset is skipped, not guessed: {dirs:?}"
    );
}

#[test]
fn linux_well_known_dirs() {
    let home = Path::new("/home/me");
    let dirs = candidate_dirs(&LocateInputs {
        os: HostOs::Linux,
        env_include_path: None,
        process_path: "/usr/bin:/bin",
        home: Some(home),
        env: &no_env,
        exists: &nothing_exists,
    });
    for expected in [
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/snap/bin"),
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin"),
        home.join(".linuxbrew").join("bin"),
        home.join(".local").join("bin"),
        home.join("bin"),
    ] {
        assert!(
            dirs.contains(&expected),
            "{} is searched on Linux: {dirs:?}",
            expected.display()
        );
    }
    let local_gh = home.join(".local").join("bin").join("gh");
    let exists = |p: &Path| p == local_gh;
    assert_eq!(
        locate_gh(&LocateInputs {
            os: HostOs::Linux,
            env_include_path: None,
            process_path: "/usr/bin:/bin",
            home: Some(home),
            env: &no_env,
            exists: &exists,
        }),
        Some(local_gh.clone()),
        "a `.desktop` launch without ~/.local/bin on PATH still finds gh there"
    );
}

#[test]
fn path_key_is_matched_ignoring_case() {
    let vars = vec![
        ("HOME".to_string(), r"C:\Users\me".to_string()),
        ("Path".to_string(), r"C:\tools\gh".to_string()),
    ];
    assert_eq!(
        env_include_path(&vars),
        Some(r"C:\tools\gh"),
        "Windows spells the key `Path`"
    );
    assert_eq!(
        env_include_path(&[("PATH".to_string(), "/opt/x".to_string())]),
        Some("/opt/x")
    );
    assert_eq!(env_include_path(&[]), None, "no PATH contributed");
}

#[test]
fn none_when_absent() {
    assert_eq!(
        locate_gh(&LocateInputs {
            os: HostOs::MacOs,
            env_include_path: Some("/x"),
            process_path: DOCK_PATH,
            home: Some(Path::new("/Users/me")),
            env: &no_env,
            exists: &nothing_exists,
        }),
        None
    );
}
