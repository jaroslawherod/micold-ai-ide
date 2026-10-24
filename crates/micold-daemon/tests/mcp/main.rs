//! The daemon's `mcp` integration tests, linked as one test binary (#1175): one module per former test file.
#[path = "../support/conn.rs"]
mod conn;
#[path = "../support/mcp.rs"]
mod mcp_support;

mod env_include_cache_coherence;
mod env_include_failure_report;
mod env_include_restart_refresh;
mod label_mapping;
mod mcp_binding_file_mode;
mod mcp_confirmations;
mod mcp_create_worktree;
mod mcp_endpoint;
mod mcp_lifecycle_tools;
mod mcp_read_tools;
mod merged_branch_check;
mod ops_extraction;
mod remote_list;
mod review_edit;
mod scoped_git_credential;
mod worktree_guard_sessions;
mod worktree_guard_setting;
mod worktree_refresh;
