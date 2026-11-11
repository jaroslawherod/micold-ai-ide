//! FR-010: `micold-daemon` with no argument is unchanged; only `orchestrator` selects the new mode.

use std::ffi::OsString;

use micold_daemon::orchestrator::{parse_mode, Mode};

fn args(list: &[&str]) -> Vec<OsString> {
    list.iter().map(OsString::from).collect()
}

#[test]
fn the_orchestrator_argument_selects_orchestrator_mode() {
    assert_eq!(parse_mode(&args(&["orchestrator"])), Mode::Orchestrator);
}

#[test]
fn no_argument_is_the_daemon() {
    assert_eq!(parse_mode(&[]), Mode::Daemon);
}

#[test]
fn any_other_argument_is_the_daemon() {
    for list in [
        &["--config", "orchestrator"][..],
        &["Orchestrator"],
        &["orchestrator-x"],
        &[""],
        &["status-line"],
    ] {
        assert_eq!(parse_mode(&args(list)), Mode::Daemon, "{list:?}");
    }
}

#[test]
fn only_the_first_argument_counts() {
    assert_eq!(
        parse_mode(&args(&["orchestrator", "extra"])),
        Mode::Orchestrator
    );
}
