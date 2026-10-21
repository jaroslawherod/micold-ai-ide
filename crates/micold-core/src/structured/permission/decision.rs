//! Deciding a request (research R5, FR-008, FR-010, FR-012): which choices fit its kind
//! ([`validate`]), and the answer JSON the prompt tool returns to the CLI ([`answer`]). The daemon
//! only ever returns the CLI's own input, extended with the answers for a question, never one a
//! client sent.

use super::request::{PromptCall, Question, RequestKind};
use super::rules::AllowRule;
use crate::protocol::messages::{PromptChoice, QuestionAnswer};
use serde_json::{json, Map, Value};
use std::fmt;

/// The most of a request's input JSON text the wire carries (research R10); the rest is fetched
/// when the user opens it.
pub const PREVIEW_BYTES: usize = 64 * 1024;

/// What Claude is told when the user denies without a message.
pub const DENIED: &str = "The user denied this request.";
/// What Claude is told when the user keeps planning without a note.
pub const KEEP_PLANNING: &str = "The user wants to keep planning.";
/// What Claude is told when the user declines to answer its questions.
pub const DECLINED: &str = "The user declined to answer.";

/// Why a decision is refused; the daemon replies `OperationError` with the sentence and changes
/// nothing (data-model *DecisionRefusal*, FR-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionRefusal {
    /// Answered, withdrawn, or never existed.
    NotPending,
    /// The request belongs to another session than the one named.
    OtherSession,
    /// The choice is for another kind of request.
    WrongKind,
    /// Not every question has an answer from its options.
    Incomplete,
    /// The session is not a structured session.
    NotStructured,
}

impl fmt::Display for DecisionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotPending => "This request is no longer waiting for an answer.",
            Self::OtherSession => "This request belongs to another session.",
            Self::WrongKind => "That answer does not fit this request.",
            Self::Incomplete => "Answer every question before submitting.",
            Self::NotStructured => "Only a structured session asks for permission this way.",
        })
    }
}

/// Whether `choice` may answer a request of `kind`.
pub fn validate(kind: &RequestKind, choice: &PromptChoice) -> Result<(), DecisionRefusal> {
    match (kind, choice) {
        (
            RequestKind::Tool,
            PromptChoice::Allow | PromptChoice::AlwaysAllow | PromptChoice::Deny { .. },
        )
        | (
            RequestKind::PlanApproval { .. },
            PromptChoice::ApprovePlan { .. } | PromptChoice::KeepPlanning { .. },
        )
        | (RequestKind::Question { .. }, PromptChoice::DeclineQuestion) => Ok(()),
        (RequestKind::Question { questions }, PromptChoice::Answer { answers }) => {
            let complete = answers.len() == questions.len()
                && questions
                    .iter()
                    .zip(answers)
                    .all(|(q, a)| answers_question(q, a));
            complete.then_some(()).ok_or(DecisionRefusal::Incomplete)
        }
        _ => Err(DecisionRefusal::WrongKind),
    }
}

/// Whether `answer` answers `question`: every label is one of its options, and there is at least
/// one label or a non-blank free answer, and only one of them on a single-choice question.
fn answers_question(question: &Question, answer: &QuestionAnswer) -> bool {
    let known = answer
        .labels
        .iter()
        .all(|label| question.options.iter().any(|option| &option.label == label));
    let chosen = answer.labels.len() + usize::from(free_answer(answer).is_some());
    known && chosen >= 1 && (question.multi_select || chosen == 1)
}

/// The free answer, when it is not blank.
fn free_answer(answer: &QuestionAnswer) -> Option<&str> {
    answer
        .other
        .as_deref()
        .filter(|text| !text.trim().is_empty())
}

/// The answer JSON for `choice` on `call` of `kind` (contracts/claude-permission.md): what the
/// prompt tool returns in `content[0].text`. Call it after [`validate`].
pub fn answer(call: &PromptCall, kind: &RequestKind, choice: &PromptChoice) -> Value {
    match choice {
        PromptChoice::Allow | PromptChoice::ApprovePlan { .. } => allow(call.input.clone()),
        PromptChoice::AlwaysAllow => {
            let mut answer = allow(call.input.clone());
            if let Some(rule) = AllowRule::for_call(call).and_then(|rule| rule.cli_rule()) {
                answer["updatedPermissions"] = json!([{
                    "type": "addRules",
                    "rules": [rule],
                    "behavior": "allow",
                    "destination": "session",
                }]);
            }
            answer
        }
        PromptChoice::Deny { message } => deny(message.as_deref(), DENIED),
        PromptChoice::KeepPlanning { note } => deny(note.as_deref(), KEEP_PLANNING),
        PromptChoice::DeclineQuestion => deny(None, DECLINED),
        PromptChoice::Answer { answers } => {
            let questions = match kind {
                RequestKind::Question { questions } => questions.as_slice(),
                _ => &[],
            };
            let texts: Map<String, Value> = questions
                .iter()
                .zip(answers)
                .map(|(question, answer)| (question.text.clone(), answer_text(answer).into()))
                .collect();
            let mut input = call.input.clone();
            input.insert("answers".to_string(), Value::Object(texts));
            allow(input)
        }
    }
}

/// One question's answer as the CLI reads it: the labels, then the free answer, joined by ", "
/// (research P-4).
fn answer_text(answer: &QuestionAnswer) -> String {
    let mut parts: Vec<&str> = answer.labels.iter().map(String::as_str).collect();
    parts.extend(free_answer(answer));
    parts.join(", ")
}

fn allow(input: Map<String, Value>) -> Value {
    json!({"behavior": "allow", "updatedInput": input})
}

/// A deny answer with `text`, or `standard` when it is absent or blank.
fn deny(text: Option<&str>, standard: &str) -> Value {
    let message = text
        .filter(|text| !text.trim().is_empty())
        .unwrap_or(standard);
    json!({"behavior": "deny", "message": message})
}

/// The input's JSON text cut to at most [`PREVIEW_BYTES`] at a char boundary, and the whole text's
/// size in bytes (data-model *PermissionRequest* `input_preview`, `input_bytes`).
pub fn input_preview(input: &Map<String, Value>) -> (String, u64) {
    let mut text = Value::Object(input.clone()).to_string();
    let bytes = text.len() as u64;
    if text.len() > PREVIEW_BYTES {
        let mut end = PREVIEW_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    (text, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structured::permission::request::QuestionOption;

    fn call(tool_name: &str, input: Value) -> PromptCall {
        PromptCall {
            tool_name: tool_name.to_string(),
            input: input.as_object().expect("an object").clone(),
            tool_use_id: Some("toolu_1".to_string()),
        }
    }

    fn plan() -> RequestKind {
        RequestKind::PlanApproval {
            plan: "1. do it".to_string(),
        }
    }

    fn question(text: &str, labels: &[&str], multi_select: bool) -> Question {
        Question {
            header: "H".to_string(),
            text: text.to_string(),
            options: labels
                .iter()
                .map(|label| QuestionOption {
                    label: label.to_string(),
                    description: String::new(),
                })
                .collect(),
            multi_select,
        }
    }

    fn questions() -> RequestKind {
        RequestKind::Question {
            questions: vec![
                question("Which color?", &["red", "green"], false),
                question("Which fruits?", &["apple", "pear", "plum"], true),
            ],
        }
    }

    fn labels(labels: &[&str]) -> QuestionAnswer {
        QuestionAnswer {
            labels: labels.iter().map(|l| l.to_string()).collect(),
            other: None,
        }
    }

    fn other(text: &str) -> QuestionAnswer {
        QuestionAnswer {
            labels: vec![],
            other: Some(text.to_string()),
        }
    }

    fn tool_choices() -> [PromptChoice; 3] {
        [
            PromptChoice::Allow,
            PromptChoice::AlwaysAllow,
            PromptChoice::Deny { message: None },
        ]
    }

    fn plan_choices() -> [PromptChoice; 2] {
        [
            PromptChoice::ApprovePlan { accept_edits: true },
            PromptChoice::KeepPlanning { note: None },
        ]
    }

    fn question_choices() -> [PromptChoice; 2] {
        [
            PromptChoice::Answer {
                answers: vec![labels(&["red"]), labels(&["apple"])],
            },
            PromptChoice::DeclineQuestion,
        ]
    }

    #[test]
    fn tool_choices_fit_a_tool_request_only() {
        for choice in tool_choices() {
            assert_eq!(validate(&RequestKind::Tool, &choice), Ok(()), "{choice:?}");
            assert_eq!(validate(&plan(), &choice), Err(DecisionRefusal::WrongKind));
            assert_eq!(
                validate(&questions(), &choice),
                Err(DecisionRefusal::WrongKind)
            );
        }
    }

    #[test]
    fn plan_choices_fit_a_plan_approval_only() {
        for choice in plan_choices() {
            assert_eq!(validate(&plan(), &choice), Ok(()), "{choice:?}");
            assert_eq!(
                validate(&RequestKind::Tool, &choice),
                Err(DecisionRefusal::WrongKind)
            );
            assert_eq!(
                validate(&questions(), &choice),
                Err(DecisionRefusal::WrongKind)
            );
        }
    }

    #[test]
    fn question_choices_fit_a_question_only() {
        for choice in question_choices() {
            assert_eq!(validate(&questions(), &choice), Ok(()), "{choice:?}");
            assert_eq!(
                validate(&RequestKind::Tool, &choice),
                Err(DecisionRefusal::WrongKind)
            );
            assert_eq!(validate(&plan(), &choice), Err(DecisionRefusal::WrongKind));
        }
    }

    #[test]
    fn an_answer_must_answer_every_question_from_its_options() {
        let refused = |answers: Vec<QuestionAnswer>| {
            assert_eq!(
                validate(
                    &questions(),
                    &PromptChoice::Answer {
                        answers: answers.clone()
                    }
                ),
                Err(DecisionRefusal::Incomplete),
                "{answers:?} (FR-008)"
            );
        };
        refused(vec![labels(&["red"])]);
        refused(vec![
            labels(&["red"]),
            labels(&["apple"]),
            labels(&["pear"]),
        ]);
        refused(vec![labels(&["purple"]), labels(&["apple"])]);
        refused(vec![labels(&["red", "green"]), labels(&["apple"])]);
        refused(vec![labels(&["red"]), labels(&[])]);
        refused(vec![labels(&["red"]), other("   ")]);
        refused(vec![
            QuestionAnswer {
                labels: vec!["red".into()],
                other: Some("teal".into()),
            },
            labels(&["apple"]),
        ]);
        for answers in [
            vec![other("teal"), labels(&["apple", "plum"])],
            vec![
                labels(&["green"]),
                QuestionAnswer {
                    labels: vec!["pear".into()],
                    other: Some("kiwi".into()),
                },
            ],
        ] {
            assert_eq!(
                validate(
                    &questions(),
                    &PromptChoice::Answer {
                        answers: answers.clone()
                    }
                ),
                Ok(()),
                "{answers:?}"
            );
        }
    }

    #[test]
    fn each_refusal_has_its_sentence() {
        let sentences: Vec<String> = [
            DecisionRefusal::NotPending,
            DecisionRefusal::OtherSession,
            DecisionRefusal::WrongKind,
            DecisionRefusal::Incomplete,
            DecisionRefusal::NotStructured,
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            sentences,
            [
                "This request is no longer waiting for an answer.",
                "This request belongs to another session.",
                "That answer does not fit this request.",
                "Answer every question before submitting.",
                "Only a structured session asks for permission this way.",
            ]
        );
    }

    #[test]
    fn a_deny_carries_the_message_or_the_standard_one() {
        let bash = call("Bash", json!({"command": "rm -rf /"}));
        let deny = |message: Option<&str>| {
            answer(
                &bash,
                &RequestKind::Tool,
                &PromptChoice::Deny {
                    message: message.map(str::to_string),
                },
            )
        };
        assert_eq!(
            deny(Some("Not that one")),
            json!({"behavior": "deny", "message": "Not that one"})
        );
        for blank in [None, Some(""), Some("  \n")] {
            assert_eq!(
                deny(blank),
                json!({"behavior": "deny", "message": "The user denied this request."}),
                "{blank:?}"
            );
        }
    }

    #[test]
    fn always_allow_without_a_cli_rule_is_a_plain_allow() {
        let write = call("Write", json!({"file_path": "/w/a", "content": "x"}));
        assert_eq!(
            answer(&write, &RequestKind::Tool, &PromptChoice::AlwaysAllow),
            json!({"behavior": "allow", "updatedInput": {"file_path": "/w/a", "content": "x"}}),
            "the daemon's own rule does the work"
        );
    }

    #[test]
    fn keep_planning_carries_the_note_or_the_standard_one() {
        let exit = call("ExitPlanMode", json!({"plan": "1. do it"}));
        let keep = |note: Option<&str>| {
            answer(
                &exit,
                &plan(),
                &PromptChoice::KeepPlanning {
                    note: note.map(str::to_string),
                },
            )
        };
        assert_eq!(
            keep(Some("Use tabs")),
            json!({"behavior": "deny", "message": "Use tabs"})
        );
        assert_eq!(
            keep(None),
            json!({"behavior": "deny", "message": "The user wants to keep planning."})
        );
    }

    #[test]
    fn declining_a_question_says_so() {
        let ask = call("AskUserQuestion", json!({"questions": []}));
        assert_eq!(
            answer(&ask, &questions(), &PromptChoice::DeclineQuestion),
            json!({"behavior": "deny", "message": "The user declined to answer."})
        );
    }

    #[test]
    fn a_free_answer_follows_the_labels() {
        let ask = call("AskUserQuestion", json!({"questions": "kept"}));
        let answered = answer(
            &ask,
            &questions(),
            &PromptChoice::Answer {
                answers: vec![
                    other("teal"),
                    QuestionAnswer {
                        labels: vec!["apple".into(), "plum".into()],
                        other: Some("kiwi".into()),
                    },
                ],
            },
        );
        assert_eq!(
            answered,
            json!({"behavior": "allow", "updatedInput": {"questions": "kept", "answers": {
                "Which color?": "teal", "Which fruits?": "apple, plum, kiwi"}}})
        );
    }

    #[test]
    fn a_small_input_is_previewed_whole() {
        let input = json!({"command": "ls"});
        let (preview, bytes) = input_preview(input.as_object().expect("object"));
        assert_eq!(preview, r#"{"command":"ls"}"#);
        assert_eq!(bytes, preview.len() as u64);
    }

    #[test]
    fn a_large_input_is_cut_at_a_char_boundary() {
        // `{"t":"` is 6 bytes: pad so a 2-byte `é` straddles the limit.
        let pad = "a".repeat(PREVIEW_BYTES - 6 - 1);
        let input = json!({ "t": format!("{pad}é and more") });
        let whole = input.to_string();
        let (preview, bytes) = input_preview(input.as_object().expect("object"));
        assert_eq!(bytes, whole.len() as u64, "the whole size");
        assert_eq!(
            preview.len(),
            PREVIEW_BYTES - 1,
            "cut before the straddling char"
        );
        assert!(whole.starts_with(&preview));
        let exact = json!({ "t": "a".repeat(PREVIEW_BYTES - 8) });
        let (preview, bytes) = input_preview(exact.as_object().expect("object"));
        assert_eq!(
            (preview.len(), bytes),
            (PREVIEW_BYTES, PREVIEW_BYTES as u64)
        );
    }
}
