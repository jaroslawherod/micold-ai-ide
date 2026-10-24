//! The `PATH` a spawn in an app-owned sandbox gets (feature 1289, FR-001 to FR-004, research
//! R1–R3; `specs/1289-sandbox-project-setup/contracts/toolchain-path.md`).
//!
//! **Pure**: it names folders and never looks at the filesystem, so a folder that does not exist
//! yet is on the path all the same, and a tool installed there later is found without a restart
//! (FR-003). The daemon decides whether the rule applies (only with an `offered.json`, R2) and
//! where the inputs come from.

use std::ffi::OsString;
use std::path::PathBuf;

/// What [`toolchain_path`] orders. An empty `home`, `mise_data_dir` or `xdg_data_home` counts as
/// unset.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToolchainPathInput {
    /// The offered AI CLIs' `bin` folders, in `offered.json` order.
    pub offered_bins: Vec<PathBuf>,
    /// The sandbox home: the resolved `HOME`, else the daemon's own.
    pub home: Option<PathBuf>,
    /// `MISE_DATA_DIR`, when set: its `shims` folder is the one mise uses (FR-002).
    pub mise_data_dir: Option<PathBuf>,
    /// `XDG_DATA_HOME`, when set and `MISE_DATA_DIR` is not: mise's data is in its `mise` folder.
    pub xdg_data_home: Option<PathBuf>,
    /// The `PATH` after environment-include, else the daemon's own.
    pub rest: OsString,
}

/// `input` as one `PATH`: the offered bins, then `home/.local/bin`, then mise's shims folder, then
/// `rest`. A folder keeps only its first position, compared as exact path text (FR-004); empty
/// entries are left out. Without `home` the two home folders are left out (the shims folder still
/// comes from `MISE_DATA_DIR` or `XDG_DATA_HOME` when one is set). Should the folders not join (a
/// name holding the separator), `rest` is returned unchanged.
pub fn toolchain_path(input: ToolchainPathInput) -> OsString {
    let ToolchainPathInput {
        offered_bins,
        home,
        mise_data_dir,
        xdg_data_home,
        rest,
    } = input;
    let set = |p: Option<PathBuf>| p.filter(|p| !p.as_os_str().is_empty());
    let home = set(home);
    let shims = set(mise_data_dir)
        .map(|dir| dir.join("shims"))
        .or_else(|| set(xdg_data_home).map(|dir| dir.join("mise").join("shims")))
        .or_else(|| {
            home.as_ref()
                .map(|home| home.join(".local").join("share").join("mise").join("shims"))
        });
    let local_bin = home.map(|home| home.join(".local").join("bin"));
    let mut ordered: Vec<PathBuf> = Vec::new();
    let candidates = offered_bins
        .into_iter()
        .chain(local_bin)
        .chain(shims)
        .chain(std::env::split_paths(&rest));
    for folder in candidates {
        let text = folder.as_os_str();
        if !text.is_empty() && !ordered.iter().any(|kept| kept.as_os_str() == text) {
            ordered.push(folder);
        }
    }
    std::env::join_paths(ordered).unwrap_or(rest)
}
