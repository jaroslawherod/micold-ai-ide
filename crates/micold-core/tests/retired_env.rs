//! `MICOLD_DATA_DIR` and `MICOLD_TOKEN_PATH` are retired (#1537, T084b): a release build ignores
//! them and names the configuration key that replaces each; the plumbing variables stay.

use std::ffi::OsString;

use micold_core::paths::{data_dir_from_env, DATA_DIR_ENV};
use micold_core::protocol::auth::TOKEN_PATH_ENV;
use micold_core::retired_env::{config_key, resolve};

const CORE_SIDE: &[(&str, &str)] = &[
    (DATA_DIR_ENV, "daemon.data_dir"),
    (TOKEN_PATH_ENV, "auth.client_token_file"),
];

#[test]
fn a_release_build_ignores_the_core_side_variables_and_names_their_keys() {
    for (name, key) in CORE_SIDE {
        let got = resolve(name, Some(OsString::from("/x")), false);
        assert_eq!(got.value, None, "{name} must have no effect");
        let warning = got.warning.unwrap_or_else(|| panic!("{name} must warn"));
        assert!(warning.contains(name), "{warning}");
        assert!(warning.contains(&format!("`{key}`")), "{warning}");
        assert_eq!(config_key(name), Some(*key));
    }
}

#[test]
fn the_data_directory_variable_counts_only_in_a_test_env_build() {
    let set = || Some(OsString::from("/var/lib/micold-ai-ide"));
    assert_eq!(data_dir_from_env(set(), false), None);
    assert_eq!(
        data_dir_from_env(set(), true),
        Some("/var/lib/micold-ai-ide".into())
    );
    assert_eq!(data_dir_from_env(Some("".into()), true), None);
    assert_eq!(data_dir_from_env(None, true), None);
}

#[test]
fn the_plumbing_variables_still_work() {
    for name in [
        "MICOLD_DAEMON_CONFIG",
        "MICOLD_STATUS_RELAY",
        "MICOLD_PI_ACTIVITY_LOG",
    ] {
        let got = resolve(name, Some(OsString::from("/p")), false);
        assert_eq!(got.value, Some(OsString::from("/p")), "{name}");
        assert_eq!(got.warning, None, "{name}");
    }
}
