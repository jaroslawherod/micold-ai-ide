//! The session service's tool server (feature 034): a loopback MCP endpoint every bound AI session
//! reaches with its own bearer credential.
//!
//! The protocol and catalog logic lives in `micold_core::mcp`; this module owns the listener, the
//! credential registry, and the tool handlers that read the daemon's state.
//!
//! Contracts: `specs/034-daemon-mcp-server/contracts/binding.md` and `contracts/mcp-tools.md`.
