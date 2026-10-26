//! Feature 1289, test A1: the `PATH` a sandboxed spawn gets (contract toolchain-path P2, P3;
//! FR-001 to FR-004). Pure: nothing here touches the filesystem, and no folder named exists.
//! Also #1439 A8, core half: the install defaults of contract sandbox-install-env I2 (FR-015).
#![cfg(unix)]

use std::ffi::OsString;
use std::path::PathBuf;

use micold_core::sandbox::toolchain_path::{
    install_env_defaults, toolchain_path, ToolchainPathInput,
};

const CLAUDE_BIN: &str = "/s/ai-clis/claude/bin";
const REST: &str = "/usr/local/bin:/usr/bin:/bin";

/// P3's first row: one offered bin, `HOME=/home/u`, nothing else set.
fn input(rest: &str) -> ToolchainPathInput {
    ToolchainPathInput {
        offered_bins: vec![PathBuf::from(CLAUDE_BIN)],
        home: Some(PathBuf::from("/home/u")),
        mise_data_dir: None,
        xdg_data_home: None,
        rest: OsString::from(rest),
    }
}

fn path(input: ToolchainPathInput) -> String {
    toolchain_path(input).into_string().unwrap()
}

#[test]
fn p3_offered_bins_then_local_bin_then_default_shims_then_rest() {
    assert_eq!(
        path(input(REST)),
        "/s/ai-clis/claude/bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/local/bin:/usr/bin:/bin"
    );
}

#[test]
fn p3_mise_data_dir_names_the_shims_folder() {
    let mut input = input(REST);
    input.mise_data_dir = Some(PathBuf::from("/opt/mise"));
    assert_eq!(
        path(input),
        "/s/ai-clis/claude/bin:/home/u/.local/bin:/opt/mise/shims:/usr/local/bin:/usr/bin:/bin"
    );
}

#[test]
fn p3_xdg_data_home_names_the_shims_folder_without_mise_data_dir() {
    let mut input = input(REST);
    input.xdg_data_home = Some(PathBuf::from("/x"));
    assert_eq!(
        path(input),
        "/s/ai-clis/claude/bin:/home/u/.local/bin:/x/mise/shims:/usr/local/bin:/usr/bin:/bin"
    );
}

#[test]
fn p3_a_home_folder_already_in_rest_moves_to_its_place() {
    assert_eq!(
        path(input("/home/u/.local/bin:/usr/bin")),
        "/s/ai-clis/claude/bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/bin"
    );
}

#[test]
fn p3_no_offered_bins_still_adds_the_home_folders() {
    let mut input = input(REST);
    input.offered_bins.clear();
    assert_eq!(
        path(input),
        "/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/local/bin:/usr/bin:/bin"
    );
}

#[test]
fn a_folder_repeated_in_rest_keeps_only_its_first_position() {
    assert_eq!(
        path(input(
            "/usr/bin:/home/u/.local/share/mise/shims:/usr/bin:/bin"
        )),
        "/s/ai-clis/claude/bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/bin:/bin"
    );
}

#[test]
fn an_offered_bin_repeated_in_rest_is_dropped_from_rest() {
    assert_eq!(
        path(input("/usr/bin:/s/ai-clis/claude/bin:/bin")),
        "/s/ai-clis/claude/bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/bin:/bin"
    );
}

#[test]
fn without_home_both_home_folders_are_left_out() {
    let mut input = input(REST);
    input.home = None;
    assert_eq!(
        path(input),
        "/s/ai-clis/claude/bin:/usr/local/bin:/usr/bin:/bin"
    );
}

#[test]
fn an_empty_home_counts_as_unset() {
    let mut input = input(REST);
    input.home = Some(PathBuf::new());
    assert_eq!(
        path(input),
        "/s/ai-clis/claude/bin:/usr/local/bin:/usr/bin:/bin"
    );
}

#[test]
fn without_home_mise_data_dir_still_names_the_shims_folder() {
    let mut input = input(REST);
    input.home = None;
    input.mise_data_dir = Some(PathBuf::from("/opt/mise"));
    assert_eq!(
        path(input),
        "/s/ai-clis/claude/bin:/opt/mise/shims:/usr/local/bin:/usr/bin:/bin"
    );
}

#[test]
fn empty_mise_data_dir_and_xdg_data_home_count_as_unset() {
    let mut input = input(REST);
    input.mise_data_dir = Some(PathBuf::new());
    input.xdg_data_home = Some(PathBuf::new());
    assert_eq!(path(input), path(self::input(REST)));
}

#[test]
fn mise_data_dir_wins_over_xdg_data_home() {
    let mut input = input(REST);
    input.mise_data_dir = Some(PathBuf::from("/opt/mise"));
    input.xdg_data_home = Some(PathBuf::from("/x"));
    assert_eq!(
        path(input),
        "/s/ai-clis/claude/bin:/home/u/.local/bin:/opt/mise/shims:/usr/local/bin:/usr/bin:/bin"
    );
}

#[test]
fn an_empty_rest_adds_no_empty_entry() {
    assert_eq!(
        path(input("")),
        "/s/ai-clis/claude/bin:/home/u/.local/bin:/home/u/.local/share/mise/shims"
    );
}

#[test]
fn folders_are_named_whether_or_not_they_exist() {
    // FR-003: none of these exists on any test machine, and every one is still on the path.
    let input = ToolchainPathInput {
        offered_bins: vec![PathBuf::from("/nonexistent-1289/bin")],
        home: Some(PathBuf::from("/nonexistent-1289/home")),
        mise_data_dir: None,
        xdg_data_home: Some(PathBuf::from("/nonexistent-1289/xdg")),
        rest: OsString::from("/nonexistent-1289/rest"),
    };
    assert_eq!(
        path(input),
        "/nonexistent-1289/bin:/nonexistent-1289/home/.local/bin:/nonexistent-1289/xdg/mise/shims:/nonexistent-1289/rest"
    );
}

#[test]
fn a_home_holding_the_separator_leaves_out_only_the_home_folders() {
    // Review M1 A-F1: neither home folder can be one entry, and the offered bin stays first.
    let mut input = input(REST);
    input.home = Some(PathBuf::from("/a:b"));
    assert_eq!(
        path(input),
        "/s/ai-clis/claude/bin:/usr/local/bin:/usr/bin:/bin"
    );
}

#[test]
fn an_xdg_data_home_holding_the_separator_leaves_out_only_the_shims_folder() {
    let mut input = input(REST);
    input.xdg_data_home = Some(PathBuf::from("/a:/b"));
    assert_eq!(
        path(input),
        "/s/ai-clis/claude/bin:/home/u/.local/bin:/usr/local/bin:/usr/bin:/bin"
    );
}

/// `install_env_defaults` for `home`, with `set` as the variables the environment already has.
fn install_env(home: Option<&str>, set: &[(&str, &str)]) -> Vec<(String, String)> {
    let set: Vec<(String, OsString)> = set
        .iter()
        .map(|(k, v)| (k.to_string(), OsString::from(v)))
        .collect();
    let home = home.map(PathBuf::from);
    install_env_defaults(home.as_deref(), |name| {
        set.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    })
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.into_os_string().into_string().unwrap()))
    .collect()
}

fn pairs(want: &[(&str, &str)]) -> Vec<(String, String)> {
    want.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn install_env_defaults_put_npm_cargo_and_go_installs_under_the_home() {
    assert_eq!(
        install_env(Some("/home/u"), &[]),
        pairs(&[
            ("NPM_CONFIG_PREFIX", "/home/u/.local"),
            ("CARGO_INSTALL_ROOT", "/home/u/.local"),
            ("GOBIN", "/home/u/.local/bin"),
        ])
    );
}

#[test]
fn install_env_leaves_out_a_variable_already_set_and_keeps_the_others() {
    assert_eq!(
        install_env(Some("/home/u"), &[("CARGO_INSTALL_ROOT", "/opt/cargo")]),
        pairs(&[
            ("NPM_CONFIG_PREFIX", "/home/u/.local"),
            ("GOBIN", "/home/u/.local/bin"),
        ])
    );
    assert_eq!(
        install_env(
            Some("/home/u"),
            &[("GOBIN", "/g"), ("NPM_CONFIG_PREFIX", "/n")]
        ),
        pairs(&[("CARGO_INSTALL_ROOT", "/home/u/.local")])
    );
}

#[test]
fn install_env_counts_an_empty_value_as_unset() {
    assert_eq!(
        install_env(Some("/home/u"), &[("GOBIN", "")]),
        install_env(Some("/home/u"), &[])
    );
}

#[test]
fn install_env_counts_a_lower_case_npm_config_prefix_as_set() {
    assert_eq!(
        install_env(Some("/home/u"), &[("npm_config_prefix", "/n")]),
        pairs(&[
            ("CARGO_INSTALL_ROOT", "/home/u/.local"),
            ("GOBIN", "/home/u/.local/bin"),
        ])
    );
    assert_eq!(
        install_env(Some("/home/u"), &[("npm_config_prefix", "")]).len(),
        3,
        "an empty lower-case prefix is unset too"
    );
}

#[test]
fn install_env_without_a_home_sets_nothing() {
    assert_eq!(install_env(None, &[]), pairs(&[]));
    assert_eq!(install_env(Some(""), &[]), pairs(&[]));
}
