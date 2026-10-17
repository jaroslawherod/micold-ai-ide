//! Where the kubeconfig and its credential plugins are looked for (feature 686, FR-022, research
//! K9). Pure: the platform, the environment and the home directory are arguments, so one CI
//! platform checks all three.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::ssh::argv::ClientPlatform;

/// Printed before the login shell's `PATH` by the probe the client runs.
pub const PATH_BEGIN: &str = "__MICOLD_PATH_BEGIN__";

/// Printed after it.
pub const PATH_END: &str = "__MICOLD_PATH_END__";

/// The directories credential plugins are commonly installed in, searched after both `PATH`s.
/// `~/` is the user's home.
pub const TOOL_DIRS: [&str; 4] = [
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "~/.local/bin",
    "~/google-cloud-sdk/bin",
];

/// The environment variable that names the home directory on `platform`.
pub fn home_var(platform: ClientPlatform) -> &'static str {
    match platform {
        ClientPlatform::Windows => "USERPROFILE",
        _ => "HOME",
    }
}

/// The separator of path lists (`KUBECONFIG`, `PATH`) on `platform`.
fn list_separator(platform: ClientPlatform) -> char {
    match platform {
        ClientPlatform::Windows => ';',
        _ => ':',
    }
}

/// The non-empty entries of a path list, in order.
fn split_list(list: &OsStr, separator: char) -> Vec<PathBuf> {
    list.to_string_lossy()
        .split(separator)
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// The kubeconfig files to merge, in order: `KUBECONFIG`'s entries when it is set and not empty,
/// else `<home>/.kube/config`.
pub fn candidates(
    kubeconfig_env: Option<&OsStr>,
    home: &Path,
    platform: ClientPlatform,
) -> Vec<PathBuf> {
    match kubeconfig_env.filter(|env| !env.is_empty()) {
        Some(env) => split_list(env, list_separator(platform)),
        None => vec![home.join(".kube").join("config")],
    }
}

/// Where a credential plugin named without a path is looked for: the process `PATH`, the login
/// shell's (not on Windows), then [`TOOL_DIRS`], each directory once, first occurrence kept.
pub fn search_path(
    process_path: Option<&OsStr>,
    login_shell_path: Option<&OsStr>,
    home: &Path,
    platform: ClientPlatform,
) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut push = |dir: PathBuf| {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    };
    if let Some(path) = process_path {
        split_list(path, list_separator(platform))
            .into_iter()
            .for_each(&mut push);
    }
    if platform != ClientPlatform::Windows {
        if let Some(path) = login_shell_path {
            split_list(path, ':').into_iter().for_each(&mut push);
        }
    }
    for dir in TOOL_DIRS {
        match dir.strip_prefix("~/") {
            Some(rest) => push(home.join(rest)),
            None => push(PathBuf::from(dir)),
        }
    }
    dirs
}

/// The login shell's `PATH` from the probe's output: the text between [`PATH_BEGIN`] and
/// [`PATH_END`], `None` without both.
pub fn parse_login_shell_output(stdout: &str) -> Option<OsString> {
    let (_, after) = stdout.split_once(PATH_BEGIN)?;
    let (path, _) = after.split_once(PATH_END)?;
    Some(OsString::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [ClientPlatform; 3] = [
        ClientPlatform::Linux,
        ClientPlatform::MacOs,
        ClientPlatform::Windows,
    ];

    fn sep(platform: ClientPlatform) -> &'static str {
        if platform == ClientPlatform::Windows {
            ";"
        } else {
            ":"
        }
    }

    fn os(s: &str) -> OsString {
        OsString::from(s)
    }

    #[test]
    fn kubeconfig_splits_on_the_platform_separator_keeping_order() {
        for platform in ALL {
            let s = sep(platform);
            let env = os(&format!("/b/one{s}/a/two{s}{s}/c/three"));
            assert_eq!(
                candidates(Some(&env), Path::new("/home/u"), platform),
                vec![
                    PathBuf::from("/b/one"),
                    PathBuf::from("/a/two"),
                    PathBuf::from("/c/three")
                ],
                "{platform:?}: in order, empty entries skipped"
            );
        }
        let colon = os("C:\\k\\one;C:\\k\\two");
        assert_eq!(
            candidates(
                Some(&colon),
                Path::new("C:\\Users\\u"),
                ClientPlatform::Windows
            ),
            vec![PathBuf::from("C:\\k\\one"), PathBuf::from("C:\\k\\two")],
            "a drive letter's colon is not a separator on Windows"
        );
    }

    #[test]
    fn without_kubeconfig_the_candidate_is_the_home_config() {
        for platform in ALL {
            let home = Path::new("/home/u");
            let want = vec![home.join(".kube").join("config")];
            assert_eq!(candidates(None, home, platform), want, "{platform:?}");
            assert_eq!(
                candidates(Some(&os("")), home, platform),
                want,
                "{platform:?}: empty is unset"
            );
        }
        assert_eq!(home_var(ClientPlatform::Windows), "USERPROFILE");
        assert_eq!(home_var(ClientPlatform::MacOs), "HOME");
        assert_eq!(home_var(ClientPlatform::Linux), "HOME");
    }

    #[test]
    fn the_search_path_is_process_then_login_shell_then_tool_dirs_first_kept() {
        let home = Path::new("/home/u");
        for platform in [ClientPlatform::Linux, ClientPlatform::MacOs] {
            let got = search_path(
                Some(&os("/usr/bin:/usr/local/bin")),
                Some(&os("/opt/homebrew/bin:/usr/bin:/home/u/bin")),
                home,
                platform,
            );
            let want: Vec<PathBuf> = [
                "/usr/bin",
                "/usr/local/bin",
                "/opt/homebrew/bin",
                "/home/u/bin",
                "/home/u/.local/bin",
                "/home/u/google-cloud-sdk/bin",
            ]
            .iter()
            .map(PathBuf::from)
            .collect();
            assert_eq!(got, want, "{platform:?}");
        }
    }

    #[test]
    fn windows_ignores_the_login_shell_path() {
        let home = Path::new("C:\\Users\\u");
        let got = search_path(
            Some(&os("C:\\bin;C:\\tools")),
            Some(&os("/should/not/appear")),
            home,
            ClientPlatform::Windows,
        );
        assert_eq!(
            got[..2],
            [PathBuf::from("C:\\bin"), PathBuf::from("C:\\tools")]
        );
        assert!(!got.contains(&PathBuf::from("/should/not/appear")));
        assert!(got.contains(&home.join(".local/bin")));
        assert_eq!(got.len(), 2 + TOOL_DIRS.len());
    }

    #[test]
    fn no_path_at_all_leaves_the_tool_dirs() {
        let got = search_path(None, None, Path::new("/h"), ClientPlatform::Linux);
        assert_eq!(got.len(), TOOL_DIRS.len());
        assert_eq!(got[0], PathBuf::from("/opt/homebrew/bin"));
    }

    #[test]
    fn login_shell_output_yields_the_text_between_the_markers() {
        let out = format!("motd line\n{PATH_BEGIN}/usr/bin:/opt/bin{PATH_END}\ntrailing");
        assert_eq!(
            parse_login_shell_output(&out),
            Some(os("/usr/bin:/opt/bin"))
        );
        assert_eq!(parse_login_shell_output("/usr/bin:/opt/bin"), None);
        assert_eq!(
            parse_login_shell_output(&format!("{PATH_BEGIN}/usr/bin")),
            None,
            "no end marker"
        );
        assert_eq!(
            parse_login_shell_output(&format!("{PATH_END}x{PATH_BEGIN}")),
            None,
            "end before begin"
        );
    }
}
