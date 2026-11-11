//! The orchestrator's integration suite (#1724): a fake daemon dials a real local socket in a
//! temporary directory, with a controllable clock. One binary; each later milestone fills in its
//! own stub below.

mod support;

mod dispatch;
mod heartbeat;
mod link_and_list;
mod netns;
mod no_outbound;
mod not_hosted;
mod one_link_per_id;
mod restart;
mod version_mismatch;
