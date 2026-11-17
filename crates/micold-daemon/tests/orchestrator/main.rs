//! The orchestrator's integration suite (#1724): a fake daemon dials a real local socket in a
//! temporary directory, with a controllable clock. One binary, one module per user story.

// The real-daemon helpers of `structured_history.rs`, for the one module that drives a daemon state.
#[path = "../support/conn.rs"]
mod conn;
#[path = "../support/history.rs"]
mod history;
#[path = "../support/mcp.rs"]
mod mcp;
#[path = "../support/structured.rs"]
mod structured;

// The real linked daemon of the `orchestrator_route` suite, for the client API tests.
#[path = "../support/linked_daemon.rs"]
mod linked_daemon;
mod store_support;
mod support;
mod ws_support;

mod dispatch;
mod heartbeat;
mod link;
mod link_and_list;
mod netns;
mod no_outbound;
mod not_hosted;
mod one_link_per_id;
mod read_structured_events;
mod restart;
mod store_config;
mod store_import;
mod store_migrations;
mod store_scan;
mod store_slow;
mod store_sql_errors;
mod store_suite;
mod version_mismatch;
mod ws_heartbeat;
mod ws_listener;
mod ws_origin;
mod ws_protocol;
mod ws_read_session;
mod ws_resume;
