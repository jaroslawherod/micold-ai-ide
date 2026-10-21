//! #942 T015, T019, T020 — permission-prompt calls recorded from a real CLI
//! (`tests/fixtures/permission-prompt/README.md`): every call parses and classifies as its kind,
//! and the answer core builds for the recorded choice equals the answer the CLI acted on (SC-004).

use micold_core::protocol::messages::{PromptChoice, QuestionAnswer};
use micold_core::structured::permission::{
    answer, classify, parse_call, validate, AllowRule, DecisionRefusal, PromptCall, RequestKind,
};
use serde_json::{json, Value};

/// The fixture `<name>.json`.
fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/tests/fixtures/permission-prompt/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} reads: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} is JSON: {e}"))
}

/// The recorded call of one exchange (`call`, or an entry of `later`).
fn call_of(exchange: &Value) -> PromptCall {
    parse_call(
        &exchange["call"]["arguments"],
        exchange["call"].get("_meta"),
    )
    .expect("a recorded call is well formed")
}

fn call(tool_name: &str, input: Value) -> PromptCall {
    PromptCall {
        tool_name: tool_name.to_string(),
        input: input.as_object().expect("an object").clone(),
        tool_use_id: None,
    }
}

#[test]
fn a_recorded_tool_request_is_a_tool_request_and_allow_is_its_answer() {
    let recorded = fixture("tool-request");
    let call = call_of(&recorded);
    assert_eq!(call.tool_name, "Bash");
    assert_eq!(
        call.tool_use_id.as_deref(),
        recorded["call"]["_meta"]["claudecode/toolUseId"].as_str(),
        "the call names its tool call"
    );
    let kind = classify(&call);
    assert_eq!(kind, RequestKind::Tool, "SC-004");
    assert_eq!(validate(&kind, &PromptChoice::Allow), Ok(()));
    let allowed = answer(&call, &kind, &PromptChoice::Allow);
    assert_eq!(
        allowed, recorded["answer"],
        "the answer the CLI acted on (SC-004)"
    );
    assert_eq!(
        allowed["updatedInput"], recorded["call"]["arguments"]["input"],
        "the input unchanged (US1 scenario 2)"
    );
}

#[test]
fn a_recorded_always_allow_answer_carries_the_session_rule() {
    let recorded = fixture("always-allow");
    let call = call_of(&recorded);
    let kind = classify(&call);
    assert_eq!(kind, RequestKind::Tool, "SC-004");
    assert_eq!(
        answer(&call, &kind, &PromptChoice::AlwaysAllow),
        recorded["answer"],
        "allow + updatedPermissions addRules, destination session (US1 scenario 4)"
    );
    let rule = AllowRule::offered(&call, &kind).expect("a Bash command has a rule");
    assert!(
        rule.matches(&call),
        "the identical later call is allowed without asking (R9)"
    );
    assert_eq!(
        rule.describe(),
        format!(
            "Bash: \"{}\"",
            recorded["call"]["arguments"]["input"]["command"]
                .as_str()
                .expect("a command")
        )
    );
}

#[test]
fn a_deny_tells_claude_the_standard_message() {
    let recorded = fixture("tool-request");
    let call = call_of(&recorded);
    assert_eq!(
        answer(
            &call,
            &RequestKind::Tool,
            &PromptChoice::Deny { message: None }
        ),
        json!({"behavior": "deny", "message": "The user denied this request."}),
        "US1 scenario 3"
    );
}

#[test]
fn a_plan_exit_or_a_question_that_does_not_fit_is_a_tool_request() {
    for input in [
        json!({}),
        json!({"plan": ""}),
        json!({"plan": " \n "}),
        json!({"plan": 3}),
    ] {
        assert_eq!(
            classify(&call("ExitPlanMode", input.clone())),
            RequestKind::Tool,
            "never dropped: {input}"
        );
    }
    let recorded = fixture("question-multi");
    let mut no_options = recorded["call"]["arguments"]["input"].clone();
    no_options["questions"][0]["options"] = json!([]);
    let mut one_option = recorded["call"]["arguments"]["input"].clone();
    one_option["questions"][1]["options"] = json!([{"label": "apple"}]);
    let mut number_question = recorded["call"]["arguments"]["input"].clone();
    number_question["questions"][0]["question"] = json!(42);
    for input in [no_options, one_option, number_question] {
        assert_eq!(
            classify(&call("AskUserQuestion", input.clone())),
            RequestKind::Tool,
            "never dropped: {input}"
        );
    }
}

// T019 — US3: plan approval.

#[test]
fn a_recorded_plan_exit_is_a_plan_approval_with_the_whole_plan() {
    let recorded = fixture("plan-approval");
    let call = call_of(&recorded);
    let kind = classify(&call);
    assert_eq!(
        kind,
        RequestKind::PlanApproval {
            plan: recorded["call"]["arguments"]["input"]["plan"]
                .as_str()
                .expect("a plan")
                .to_string()
        },
        "US3 scenario 1, SC-004"
    );
    for accept_edits in [true, false] {
        let choice = PromptChoice::ApprovePlan { accept_edits };
        assert_eq!(validate(&kind, &choice), Ok(()));
        assert_eq!(
            answer(&call, &kind, &choice),
            recorded["answer"],
            "allow with the input unchanged (US3 scenario 2)"
        );
    }
    assert_eq!(
        answer(&call, &kind, &PromptChoice::KeepPlanning { note: None }),
        json!({"behavior": "deny", "message": "The user wants to keep planning."}),
        "US3 scenario 3"
    );
    for choice in [
        PromptChoice::Allow,
        PromptChoice::AlwaysAllow,
        PromptChoice::Deny { message: None },
    ] {
        assert_eq!(validate(&kind, &choice), Err(DecisionRefusal::WrongKind));
    }
    assert_eq!(
        AllowRule::offered(&call, &kind),
        None,
        "Always allow is not offered on a plan"
    );
}

#[test]
fn the_write_after_an_approved_plan_is_an_ordinary_tool_request() {
    let recorded = fixture("plan-approval");
    let later = &recorded["later"][0];
    let call = call_of(later);
    assert_eq!(call.tool_name, "Write");
    let kind = classify(&call);
    assert_eq!(kind, RequestKind::Tool);
    assert_eq!(answer(&call, &kind, &PromptChoice::Allow), later["answer"]);
}

// T020 — US4: questions.

#[test]
fn a_recorded_question_shows_every_question_and_option() {
    let recorded = fixture("question-multi");
    let call = call_of(&recorded);
    let RequestKind::Question { questions } = classify(&call) else {
        panic!("a question request (SC-004)");
    };
    let input = &recorded["call"]["arguments"]["input"]["questions"];
    assert_eq!(questions.len(), 2);
    for (question, recorded) in questions.iter().zip(input.as_array().expect("questions")) {
        assert_eq!(
            question.header,
            recorded["header"].as_str().expect("header")
        );
        assert_eq!(question.text, recorded["question"].as_str().expect("text"));
        assert_eq!(
            question.multi_select,
            recorded["multiSelect"].as_bool().unwrap_or(false)
        );
        let options = recorded["options"].as_array().expect("options");
        assert_eq!(question.options.len(), options.len());
        for (option, recorded) in question.options.iter().zip(options) {
            assert_eq!(option.label, recorded["label"].as_str().expect("label"));
            assert_eq!(
                option.description,
                recorded["description"].as_str().unwrap_or("")
            );
        }
    }
    assert!(!questions[0].multi_select && questions[1].multi_select);
}

/// The labels the stub chose for each question, read back from the recorded answer.
fn recorded_labels(recorded: &Value, questions: usize) -> Vec<QuestionAnswer> {
    let input = &recorded["call"]["arguments"]["input"]["questions"];
    (0..questions)
        .map(|at| {
            let text = input[at]["question"].as_str().expect("text");
            let joined = recorded["answer"]["updatedInput"]["answers"][text]
                .as_str()
                .expect("an answer per question");
            QuestionAnswer {
                labels: joined.split(", ").map(str::to_string).collect(),
                other: None,
            }
        })
        .collect()
}

#[test]
fn answering_every_question_gives_the_recorded_answer() {
    let recorded = fixture("question-multi");
    let call = call_of(&recorded);
    let kind = classify(&call);
    let choice = PromptChoice::Answer {
        answers: recorded_labels(&recorded, 2),
    };
    assert_eq!(validate(&kind, &choice), Ok(()));
    assert_eq!(
        answer(&call, &kind, &choice),
        recorded["answer"],
        "the input plus the answers map the CLI accepted (US4 scenario 2, SC-004)"
    );
    assert_eq!(
        answer(&call, &kind, &PromptChoice::DeclineQuestion),
        json!({"behavior": "deny", "message": "The user declined to answer."}),
        "US4 scenario 3"
    );
}

#[test]
fn a_question_cannot_be_submitted_until_every_one_is_answered() {
    let recorded = fixture("question-multi");
    let kind = classify(&call_of(&recorded));
    let answers = recorded_labels(&recorded, 2);
    let incomplete = [
        vec![answers[0].clone()],
        vec![answers[0].clone(), QuestionAnswer::default()],
        vec![
            QuestionAnswer {
                labels: vec!["not an option".into()],
                other: None,
            },
            answers[1].clone(),
        ],
        vec![
            QuestionAnswer {
                labels: vec!["red".into(), "green".into()],
                other: None,
            },
            answers[1].clone(),
        ],
        vec![
            answers[0].clone(),
            QuestionAnswer {
                labels: vec![],
                other: Some("  ".into()),
            },
        ],
    ];
    for answers in incomplete {
        assert_eq!(
            validate(
                &kind,
                &PromptChoice::Answer {
                    answers: answers.clone()
                }
            ),
            Err(DecisionRefusal::Incomplete),
            "US4 scenario 4: {answers:?}"
        );
    }
}
