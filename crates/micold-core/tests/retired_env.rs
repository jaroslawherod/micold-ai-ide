//! `MICOLD_DATA_DIR` and `MICOLD_TOKEN_PATH` are retired (#1537, T084b): a release build ignores
//! them and names the configuration key that replaces each; the plumbing variables stay.
//!
//! `MICOLD_LOG` is in `RETIRED` as the *daemon's* filter. The client has its own filter read from
//! the same name (`crates/micold-client/src/shell/startup.rs`, `init_tracing`); that is not retired
//! and is not read through [`resolve`].

use std::ffi::OsString;

use micold_core::paths::data_dir_from_env;
use micold_core::retired_env::{config_key, resolve, RETIRED};

const EIGHT: [&str; 8] = [
    "MICOLD_HOME",
    "MICOLD_IMAGE_REFERENCE",
    "MICOLD_LISTEN_ADDR",
    "MICOLD_IDLE_STOP",
    "MICOLD_LOG",
    "MICOLD_REPOS_ROOT",
    "MICOLD_DATA_DIR",
    "MICOLD_TOKEN_PATH",
];

#[test]
fn retired_names_exactly_the_eight_variables_once() {
    let mut names: Vec<&str> = RETIRED.iter().map(|r| r.name).collect();
    names.sort_unstable();
    let mut want = EIGHT;
    want.sort_unstable();
    assert_eq!(names, want);
}

#[test]
fn a_release_build_ignores_every_retired_variable_and_names_its_key() {
    for r in RETIRED {
        let (name, key) = (r.name, r.key);
        let got = resolve(name, Some(OsString::from("/x")), false);
        assert_eq!(got.value, None, "{name} must have no effect");
        let warning = got.warning.unwrap_or_else(|| panic!("{name} must warn"));
        assert!(warning.contains(name), "{warning}");
        assert!(warning.contains(&format!("`{key}`")), "{warning}");
        assert_eq!(config_key(name), Some(key));
    }
}

#[test]
fn a_test_env_build_honours_a_retired_variable_and_an_unset_one_is_silent() {
    for r in RETIRED {
        let set = resolve(r.name, Some(OsString::from("/x")), true);
        assert_eq!(set.value, Some(OsString::from("/x")), "{}", r.name);
        assert_eq!(set.warning, None, "{}", r.name);
        for enabled in [false, true] {
            let unset = resolve(r.name, None, enabled);
            assert_eq!(unset.value, None, "{}", r.name);
            assert_eq!(unset.warning, None, "{} must not warn when unset", r.name);
        }
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
