//! The orchestrator's integration suite (#1724): a fake daemon dials a real local socket in a
//! temporary directory, with a controllable clock. One binary, one module per user story.

mod support;

mod dispatch;
mod heartbeat;
mod link;
mod link_and_list;
mod netns;
mod no_outbound;
mod not_hosted;
mod one_link_per_id;
mod restart;
mod version_mismatch;
