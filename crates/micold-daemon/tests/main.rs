//! The integration tests of `micold-daemon`, linked as one executable (#1631).
//!
//! Each top-level `tests/*.rs` used to be its own test target, so a workspace test run linked hundreds of
//! executables. They are modules of this one now. The tests that change process-global state
//! (`std::env::set_var`, the working directory), time a budget, or need a container runtime stay separate
//! targets, listed in `Cargo.toml`. `every_test_file_is_wired` fails for a file that is neither.

#[path = "support/history.rs"]
mod history;
#[path = "support/mcp.rs"]
mod mcp;
#[path = "support/runs.rs"]
mod runs_support;
#[allow(dead_code)]
#[path = "../../micold-core/tests/support/test_wiring.rs"]
mod test_wiring;

mod config_check;
mod config_load;
mod config_projects;
mod config_reload;
mod every_test_file_is_wired;
mod log_redaction;
mod pasted_cleanup;
mod project_registered_before_create;
mod run_group_persist;
