//! When nobody is using this daemon, and what it does about it (feature 028, data-model G1/G2/G4).
//!
//! Three things live here: `Presence` — how many clients are connected and, when none are, since
//! when; `IdleWindow` — the 30-minute rule read against that count; and `StopReason` — why the
//! daemon is unwinding, so the last line in the log says which of the ways out this was.
//!
//! Deliberately not `state.rs`: the rule is a pure function of a count and a clock reading, and
//! keeping it away from the lock-guarded session catalogue is what makes it testable without a
//! daemon.
//!
//! Filled in by T009–T010; the module exists from Phase 1 so that every later task touches one file.
