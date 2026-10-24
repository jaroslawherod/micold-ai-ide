//! The daemon's `sandbox_real` integration tests, linked as one test binary (#1175): one module per former test file.
#[path = "../support/mcp.rs"]
mod mcp_support;
mod sandbox_real_support;

mod sandbox_real_ai_cli;
mod sandbox_real_ai_cli_sessions;
mod sandbox_real_boundary;
mod sandbox_real_fingerprint;
mod sandbox_real_history;
mod sandbox_real_idle;
mod sandbox_real_limits;
mod sandbox_real_mcp;
mod sandbox_real_multi_daemon;
mod sandbox_real_multi_daemon_container;
mod sandbox_real_multi_daemon_host;
mod sandbox_real_no_user_credentials;
mod sandbox_real_parity;
mod sandbox_real_provision;
mod sandbox_real_session_start;
mod sandbox_real_staleness;
