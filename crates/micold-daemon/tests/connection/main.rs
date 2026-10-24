//! The daemon's `connection` integration tests, linked as one test binary (#1175): one module per former test file.
#[path = "../support/attention.rs"]
mod attention;
#[path = "../support/conn.rs"]
mod conn;
#[path = "../support/mod.rs"]
mod support;

mod busy_connection;
mod drive_loop;
mod frame_coalescing;
mod framing;
mod handshake_flow;
mod logging_sink;
mod pane_terminals;
mod presence_counting;
mod reattach_snapshot;
mod sandbox_transport;
mod scroll_cost;
mod scrollback_range;
mod slow_client;
mod status_relay;
mod stream_view;
mod terminal_color_scheme;
mod two_viewers;
mod unread_state;
mod version_recovery;
mod viewed_session_memory;
mod vt_color_queries;
