//! A prompt-tool call: what the CLI sent ([`parse_call`], research R1) and what kind of request it
//! is ([`classify`], research R2, FR-002, FR-004).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The `_meta` key the CLI repeats the tool call's id under (research P-1).
const META_TOOL_USE_ID: &str = "claudecode/toolUseId";

/// The CLI's plan-exit tool (research P-3).
pub const PLAN_TOOL: &str = "ExitPlanMode";

/// The CLI's ask-the-user tool (research P-4).
pub const QUESTION_TOOL: &str = "AskUserQuestion";

/// One prompt-tool call (data-model *PromptCall*).
#[derive(Debug, Clone, PartialEq)]
pub struct PromptCall {
    pub tool_name: String,
    /// The tool's input, exactly as the CLI sent it.
    pub input: Map<String, Value>,
    /// The tool call it asks about: `arguments.tool_use_id`, else `_meta["claudecode/toolUseId"]`.
    pub tool_use_id: Option<String>,
}

/// A call that cannot become a request: answered deny at once with the reason, logged without its
/// input, never pending (FR-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Malformed {
    /// `"no tool name"` or `"the input is not an object"`.
    pub reason: &'static str,
}

/// The call of `tools/call`'s `arguments` and `_meta` (contracts/claude-permission.md).
pub fn parse_call(arguments: &Value, meta: Option<&Value>) -> Result<PromptCall, Malformed> {
    let Some(tool_name) = arguments.get("tool_name").and_then(Value::as_str) else {
        return Err(Malformed {
            reason: "no tool name",
        });
    };
    let Some(input) = arguments.get("input").and_then(Value::as_object) else {
        return Err(Malformed {
            reason: "the input is not an object",
        });
    };
    let id_of = |value: Option<&Value>| value.and_then(Value::as_str).map(str::to_string);
    let tool_use_id = id_of(arguments.get("tool_use_id"))
        .or_else(|| id_of(meta.and_then(|meta| meta.get(META_TOOL_USE_ID))));
    Ok(PromptCall {
        tool_name: tool_name.to_string(),
        input: input.clone(),
        tool_use_id,
    })
}

/// What a request asks the user (data-model *RequestKind*). Plan text and questions travel whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestKind {
    /// May this tool use run? Allow, Always allow or Deny.
    Tool,
    /// Claude asks to leave plan mode with this plan.
    PlanApproval { plan: String },
    /// Claude asks the user these questions.
    Question { questions: Vec<Question> },
}

/// One question of a question request (research R2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub header: String,
    pub text: String,
    /// At least two.
    pub options: Vec<QuestionOption>,
    /// Several options may be chosen.
    pub multi_select: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionOption {
    pub label: String,
    /// Empty when the CLI sent none.
    pub description: String,
}

/// The kind of `call`: a plan approval or a question when the tool and its input have that shape,
/// otherwise a tool request, so no call is ever dropped (FR-002, spec edge cases).
pub fn classify(call: &PromptCall) -> RequestKind {
    let special = match call.tool_name.as_str() {
        PLAN_TOOL => plan_of(&call.input).map(|plan| RequestKind::PlanApproval { plan }),
        QUESTION_TOOL => {
            questions_of(&call.input).map(|questions| RequestKind::Question { questions })
        }
        _ => None,
    };
    special.unwrap_or(RequestKind::Tool)
}

/// The plan of an `ExitPlanMode` input: a string that is not blank.
fn plan_of(input: &Map<String, Value>) -> Option<String> {
    let plan = input.get("plan")?.as_str()?;
    (!plan.trim().is_empty()).then(|| plan.to_string())
}

/// The questions of an `AskUserQuestion` input, when every one has the question shape.
fn questions_of(input: &Map<String, Value>) -> Option<Vec<Question>> {
    let questions: Vec<Question> = input
        .get("questions")?
        .as_array()?
        .iter()
        .map(question_of)
        .collect::<Option<_>>()?;
    (!questions.is_empty()).then_some(questions)
}

fn question_of(question: &Value) -> Option<Question> {
    let multi_select = match question.get("multiSelect") {
        None | Some(Value::Null) => false,
        Some(flag) => flag.as_bool()?,
    };
    let options: Vec<QuestionOption> = question
        .get("options")?
        .as_array()?
        .iter()
        .map(option_of)
        .collect::<Option<_>>()?;
    (options.len() >= 2).then_some(())?;
    Some(Question {
        header: question.get("header")?.as_str()?.to_string(),
        text: question.get("question")?.as_str()?.to_string(),
        options,
        multi_select,
    })
}

fn option_of(option: &Value) -> Option<QuestionOption> {
    let description = match option.get("description") {
        None | Some(Value::Null) => String::new(),
        Some(text) => text.as_str()?.to_string(),
    };
    Some(QuestionOption {
        label: option.get("label")?.as_str()?.to_string(),
        description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_call_keeps_its_tool_input_and_id() {
        let call = parse_call(
            &json!({"tool_name": "Bash", "input": {"command": "ls"}, "tool_use_id": "toolu_1"}),
            Some(&json!({"claudecode/toolUseId": "toolu_meta"})),
        )
        .expect("a well-formed call");
        assert_eq!(call.tool_name, "Bash");
        assert_eq!(Value::Object(call.input), json!({"command": "ls"}));
        assert_eq!(
            call.tool_use_id.as_deref(),
            Some("toolu_1"),
            "the argument wins over _meta"
        );
    }

    #[test]
    fn the_tool_use_id_falls_back_to_meta_then_to_none() {
        let arguments = json!({"tool_name": "Bash", "input": {}});
        let from_meta = parse_call(
            &arguments,
            Some(&json!({"claudecode/toolUseId": "toolu_2"})),
        );
        assert_eq!(
            from_meta.expect("well-formed").tool_use_id.as_deref(),
            Some("toolu_2")
        );
        assert_eq!(
            parse_call(&arguments, None)
                .expect("well-formed")
                .tool_use_id,
            None
        );
        let not_a_string = json!({"tool_name": "Bash", "input": {}, "tool_use_id": 3});
        assert_eq!(
            parse_call(&not_a_string, Some(&json!({"claudecode/toolUseId": 4})))
                .expect("an odd id does not cost the call")
                .tool_use_id,
            None
        );
    }

    #[test]
    fn a_call_without_a_tool_name_is_malformed() {
        for arguments in [
            json!({"input": {}}),
            json!({"tool_name": 7, "input": {}}),
            json!({"tool_name": null, "input": {}}),
            json!("not an object"),
        ] {
            assert_eq!(
                parse_call(&arguments, None),
                Err(Malformed {
                    reason: "no tool name"
                }),
                "{arguments}"
            );
        }
    }

    #[test]
    fn a_call_whose_input_is_not_an_object_is_malformed() {
        for arguments in [
            json!({"tool_name": "Bash"}),
            json!({"tool_name": "Bash", "input": "ls"}),
            json!({"tool_name": "Bash", "input": ["ls"]}),
            json!({"tool_name": "Bash", "input": null}),
        ] {
            assert_eq!(
                parse_call(&arguments, None),
                Err(Malformed {
                    reason: "the input is not an object"
                }),
                "{arguments}"
            );
        }
    }

    fn call(tool_name: &str, input: Value) -> PromptCall {
        PromptCall {
            tool_name: tool_name.to_string(),
            input: input.as_object().expect("an object").clone(),
            tool_use_id: None,
        }
    }

    #[test]
    fn a_plan_with_text_is_a_plan_approval() {
        assert_eq!(
            classify(&call(
                PLAN_TOOL,
                json!({"plan": "1. a\n2. b", "planFilePath": "/p"})
            )),
            RequestKind::PlanApproval {
                plan: "1. a\n2. b".to_string()
            }
        );
    }

    #[test]
    fn a_question_option_without_a_description_has_an_empty_one() {
        let kind = classify(&call(
            QUESTION_TOOL,
            json!({"questions": [{"question": "Q?", "header": "H",
                   "options": [{"label": "a"}, {"label": "b", "description": "B"}]}]}),
        ));
        assert_eq!(
            kind,
            RequestKind::Question {
                questions: vec![Question {
                    header: "H".into(),
                    text: "Q?".into(),
                    options: vec![
                        QuestionOption {
                            label: "a".into(),
                            description: String::new()
                        },
                        QuestionOption {
                            label: "b".into(),
                            description: "B".into()
                        },
                    ],
                    multi_select: false,
                }]
            },
            "multiSelect absent is single choice"
        );
    }

    #[test]
    fn a_question_that_does_not_fit_its_shape_is_a_tool_request() {
        let two = json!([{"label": "a"}, {"label": "b"}]);
        for questions in [
            json!([]),
            json!("Q?"),
            json!([{"question": "Q?", "header": "H", "options": [], "multiSelect": false}]),
            json!([{"question": "Q?", "header": "H", "options": [{"label": "a"}]}]),
            json!([{"question": 1, "header": "H", "options": two}]),
            json!([{"question": "Q?", "options": two}]),
            json!([{"question": "Q?", "header": "H", "options": two, "multiSelect": "yes"}]),
            json!([{"question": "Q?", "header": "H", "options": [{"label": "a"}, {"label": 2}]}]),
            json!([{"question": "Q?", "header": "H",
                    "options": [{"label": "a", "description": 1}, {"label": "b"}]}]),
        ] {
            assert_eq!(
                classify(&call(QUESTION_TOOL, json!({ "questions": questions }))),
                RequestKind::Tool,
                "never dropped: {questions}"
            );
        }
        assert_eq!(classify(&call(QUESTION_TOOL, json!({}))), RequestKind::Tool);
    }

    #[test]
    fn the_shapes_belong_to_their_tools_only() {
        assert_eq!(
            classify(&call("Bash", json!({"plan": "x"}))),
            RequestKind::Tool
        );
        assert_eq!(
            classify(&call(
                "Other",
                json!({"questions": [{"question": "Q?", "header": "H",
                       "options": [{"label": "a"}, {"label": "b"}]}]})
            )),
            RequestKind::Tool
        );
    }
}
