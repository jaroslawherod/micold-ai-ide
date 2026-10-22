//! Control requests the daemon writes to a structured session's stdin (#942, research R6): one
//! stream-json line each, answered by the CLI's `control_response` line for the same request id
//! ([`StructuredEvent::ControlResponse`](super::StructuredEvent::ControlResponse)).
//! Render-free and process-free; the daemon writes the line and waits for the answer.

use super::model::{ModelChoice, ModelRefusal};
use super::ControlOutcome;
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

/// The line that asks the CLI to cancel the turn in progress (contracts/claude-input.md
/// *Interrupt*, P-4), ending in `\n`. The CLI answers it and ends the turn with a `result`.
pub fn encode_interrupt(request_id: &str) -> String {
    encode_bare_request(request_id, "interrupt")
}

/// The line that asks the CLI for its commands and models (#943 R1, contracts/claude-commands.md),
/// written once per structured spawn before any user message; ends in `\n`.
pub fn encode_initialize(request_id: &str) -> String {
    encode_bare_request(request_id, "initialize")
}

/// The line that asks the CLI to answer later turns with `choice` (#943 R7); `Default` is
/// `"default"`. Ends in `\n`.
pub fn encode_set_model(request_id: &str, choice: &ModelChoice) -> String {
    let request_id = serde_json::Value::from(request_id);
    let model = serde_json::Value::from(choice.cli_value());
    format!(
        "{{\"type\":\"control_request\",\"request_id\":{request_id},\"request\":{{\"subtype\":\"set_model\",\"model\":{model}}}}}\n"
    )
}

/// The model the CLI resolved a `set_model` to, or why it did not change: its refusal, or
/// [`ModelRefusal::NoAnswer`] for a success that names no model (#943 R7).
pub fn parse_set_model(outcome: &ControlOutcome) -> Result<String, ModelRefusal> {
    match outcome {
        ControlOutcome::Success(answer) => serde_json::from_str::<serde_json::Value>(answer)
            .ok()
            .and_then(|answer| {
                answer["model"]
                    .as_str()
                    .filter(|model| !model.trim().is_empty())
                    .map(str::to_string)
            })
            .ok_or(ModelRefusal::NoAnswer),
        ControlOutcome::Error { message, code } => Err(ModelRefusal::Refused {
            message: message.clone(),
            code: code.clone(),
        }),
    }
}

/// A `control_request` line whose request carries only its `subtype`; ends in `\n`.
fn encode_bare_request(request_id: &str, subtype: &str) -> String {
    let request_id = serde_json::Value::from(request_id);
    let subtype = serde_json::Value::from(subtype);
    format!(
        "{{\"type\":\"control_request\",\"request_id\":{request_id},\"request\":{{\"subtype\":{subtype}}}}}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_is_the_contracts_one_line_with_a_fresh_id() {
        let id = next_request_id();
        assert_eq!(
            encode_interrupt(&id),
            format!(
                "{{\"type\":\"control_request\",\"request_id\":\"{id}\",\"request\":{{\"subtype\":\"interrupt\"}}}}\n"
            )
        );
        assert_eq!(
            encode_interrupt("micold-1"),
            "{\"type\":\"control_request\",\"request_id\":\"micold-1\",\"request\":{\"subtype\":\"interrupt\"}}\n"
        );
    }

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
    fn initialize_and_set_model_are_the_contracts_one_lines() {
        use crate::structured::model::{ModelChoice, ModelName};
        assert_eq!(
            encode_initialize("micold-1"),
            "{\"type\":\"control_request\",\"request_id\":\"micold-1\",\"request\":{\"subtype\":\"initialize\"}}\n"
        );
        assert_eq!(
            encode_set_model("micold-2", &ModelChoice::Named(ModelName::parse("haiku").unwrap())),
            "{\"type\":\"control_request\",\"request_id\":\"micold-2\",\"request\":{\"subtype\":\"set_model\",\"model\":\"haiku\"}}\n"
        );
        assert_eq!(
            encode_set_model("micold-3", &ModelChoice::Default),
            "{\"type\":\"control_request\",\"request_id\":\"micold-3\",\"request\":{\"subtype\":\"set_model\",\"model\":\"default\"}}\n"
        );
        let odd = ModelChoice::Named(ModelName::parse("a\"b\\c").unwrap());
        let value: serde_json::Value =
            serde_json::from_str(&encode_set_model("x", &odd)).expect("JSON");
        assert_eq!(value["request"]["model"], "a\"b\\c");
    }

    /// The outcome of the fixture's one control request.
    fn recorded_outcome(fixture: &str) -> crate::structured::ControlOutcome {
        fixture
            .lines()
            .find_map(|line| match crate::structured::parse_line(line) {
                crate::structured::StructuredEvent::ControlResponse { outcome, .. } => {
                    Some(outcome)
                }
                _ => None,
            })
            .expect("a control response")
    }

    #[test]
    fn a_set_model_answer_is_the_resolved_model_or_the_clis_refusal() {
        use crate::structured::model::ModelRefusal;
        use crate::structured::ControlOutcome;
        let changed = recorded_outcome(include_str!(
            "../../tests/fixtures/stream-json/model-change.jsonl"
        ));
        assert_eq!(
            parse_set_model(&changed),
            Ok("claude-haiku-5-5".to_string())
        );
        let refused = recorded_outcome(include_str!(
            "../../tests/fixtures/stream-json/model-refused.jsonl"
        ));
        assert_eq!(
            parse_set_model(&refused),
            Err(ModelRefusal::Refused {
                message: "Model 'no-such-model-xyz' not found".into(),
                code: Some("not_found".into()),
            })
        );
        for answer in [
            "null",
            "{}",
            r#"{"model":7}"#,
            r#"{"model":""}"#,
            "not json",
        ] {
            assert_eq!(
                parse_set_model(&ControlOutcome::Success(answer.into())),
                Err(ModelRefusal::NoAnswer),
                "{answer}"
            );
        }
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
