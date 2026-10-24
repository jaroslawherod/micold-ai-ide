//! The daemon's `sessions` integration tests, linked as one test binary (#1175): one module per former test file.
#[path = "../support/attention.rs"]
mod attention;
use attention::conn; // attention.rs loads support/conn.rs once

mod activity_ended;
mod agent_runs;
mod ai_cli_start_one_resolution;
mod attention_claims;
mod attention_events;
mod attention_write_retry;
mod busy_session_start;
mod copilot_activity;
mod daemon_lifecycle;
mod daemon_singleton;
mod diagnostics;
mod liveness;
mod mutation_atomicity;
mod mutation_semantics;
mod no_socket_activation;
mod pi_activity;
mod session_cwd_guard;
mod session_isolation;
mod session_name_persistence;
mod session_origin_removal;
mod session_reveal;
mod shell_instances;
mod shell_open_refused_reported;
mod supervision_respawn_refused;
mod supervision_restart;
