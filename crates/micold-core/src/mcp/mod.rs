//! The render-free half of the session service's tool server (feature 034).
//!
//! The daemon exposes an MCP (Model Context Protocol) server to the AI sessions it runs, so an agent
//! can read and manage its own project's worktrees and sessions. Everything here is pure logic with
//! no I/O of its own: the JSON-RPC envelope and method routing, the tool catalog and argument
//! validation, the result and error shapes, and the per-CLI launch binding plan.
//!
//! Contracts: `specs/034-daemon-mcp-server/contracts/binding.md` (endpoint, authentication, launch
//! wiring) and `contracts/mcp-tools.md` (tools, rows, results).

pub mod errors;
pub mod jsonrpc;
pub mod binding;
