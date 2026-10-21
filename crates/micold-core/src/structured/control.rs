//! Control requests the daemon writes to a structured session's stdin (#942, research R6): one
//! stream-json line each, answered by the CLI's `control_response` line for the same request id
//! ([`StructuredEvent::ControlResponse`](super::StructuredEvent::ControlResponse)).
//! Render-free and process-free; the daemon writes the line and waits for the answer.

use crate::session::PermissionMode;
use std::sync::atomic::{AtomicU64, Ordering};

/// The counter behind [`next_request_id`]: one per process, so no two requests of a daemon run
/// share an id.
static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

/// A fresh request id, `micold-<n>`, `n` increasing within the process.
pub fn next_request_id() -> String {
    format!("micold-{}", NEXT_REQUEST.fetch_add(1, Ordering::Relaxed))
}

/// The line that asks the CLI to switch to `mode` (contracts/claude-permission.md *Mode change*),
/// ending in `\n`. The fields are in the contract's order, so the line is the contract's text.
pub fn encode_set_mode(request_id: &str, mode: PermissionMode) -> String {
    let request_id = serde_json::Value::from(request_id);
    format!(
        "{{\"type\":\"control_request\",\"request_id\":{request_id},\"request\":{{\"subtype\":\"set_permission_mode\",\"mode\":\"{}\"}}}}\n",
        mode.cli_name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_mode_is_the_contracts_one_line() {
        assert_eq!(
            encode_set_mode("micold-7", PermissionMode::AcceptEdits),
            "{\"type\":\"control_request\",\"request_id\":\"micold-7\",\"request\":{\"subtype\":\"set_permission_mode\",\"mode\":\"acceptEdits\"}}\n"
        );
        for mode in PermissionMode::ALL {
            let line = encode_set_mode("micold-1", mode);
            assert_eq!(line.matches('\n').count(), 1, "one line: {line}");
            assert!(line.ends_with('\n'));
            let value: serde_json::Value = serde_json::from_str(&line).expect("JSON");
            assert_eq!(value["request"]["mode"], mode.cli_name());
        }
    }

    #[test]
    fn a_request_id_is_escaped_as_json() {
        let line = encode_set_mode("a\"b", PermissionMode::Plan);
        let value: serde_json::Value = serde_json::from_str(&line).expect("JSON");
        assert_eq!(value["request_id"], "a\"b");
    }

    #[test]
    fn request_ids_are_micold_n_and_increase() {
        let number = |id: String| -> u64 {
            id.strip_prefix("micold-")
                .unwrap_or_else(|| panic!("micold-<n>: {id}"))
                .parse()
                .expect("a number")
        };
        let first = number(next_request_id());
        let second = number(next_request_id());
        assert!(second > first, "{first} then {second}");
    }
}
