//! Feature 1289, test A1: the `PATH` a sandboxed spawn gets (contract toolchain-path P2, P3;
//! FR-001 to FR-004). Pure: nothing here touches the filesystem, and no folder named exists.
#![cfg(unix)]

use std::ffi::OsString;
use std::path::PathBuf;

use micold_core::sandbox::toolchain_path::{toolchain_path, ToolchainPathInput};

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
