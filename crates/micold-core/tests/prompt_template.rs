//! The default prompt template (feature 1161): parse, render, and the fixture shared with the
//! dialog and MCP tests (FR-011).

use micold_core::prompt_template::{PromptTemplate, WorktreeFacts};

use micold_core::prompt_template::shared_fixture::{facts as shared_facts, OUTPUT as SHARED_OUTPUT, TEMPLATE as SHARED_TEMPLATE};

fn render(template: &str, facts: &WorktreeFacts) -> Option<String> {
    PromptTemplate::parse(template).unwrap().render(facts)
}

#[test]
fn renders_the_shared_fixture() {
    assert_eq!(
        render(SHARED_TEMPLATE, &shared_facts()).as_deref(),
        Some(SHARED_OUTPUT)
    );
}

#[test]
fn renders_all_six_placeholders() {
    let out = render("{issue}|{issue_title}|{issue_body}|{branch}|{type}|{name}", &shared_facts());
    assert_eq!(
        out.as_deref(),
        Some("42|Crash on save|It crashes. {issue}|fix/42_crash|fix|crash")
    );
}

#[test]
fn missing_value_renders_empty() {
    let out = render("/speckit-autopilot #{issue}", &WorktreeFacts::default());
    assert_eq!(out.as_deref(), Some("/speckit-autopilot #"));
}

#[test]
fn doubled_braces_are_literal() {
    assert_eq!(
        render("{{x}} {{", &WorktreeFacts::default()).as_deref(),
        Some("{x} {")
    );
}

#[test]
fn inserted_body_is_not_expanded_again() {
    let facts = WorktreeFacts {
        issue: "7".into(),
        issue_body: "see {issue}".into(),
        ..WorktreeFacts::default()
    };
    assert_eq!(render("{issue_body}", &facts).as_deref(), Some("see {issue}"));
}

#[test]
fn blank_result_is_none() {
    assert_eq!(render("", &shared_facts()), None);
    assert_eq!(render("  \n {issue_title} ", &WorktreeFacts::default()), None);
}

#[test]
fn unknown_placeholder_is_an_error_with_the_contract_text() {
    let err = PromptTemplate::parse("a {issu} b").unwrap_err();
    assert_eq!(
        err.to_string(),
        "Unknown placeholder {issu}; valid: {issue}, {issue_title}, {issue_body}, {branch}, {type}, {name}"
    );
}

#[test]
fn unbalanced_braces_are_an_error_with_the_contract_text() {
    for bad in ["{", "}", "a {issue", "x } y"] {
        let err = PromptTemplate::parse(bad).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Unbalanced brace in the template; use {{ and }} for literal braces",
            "{bad}"
        );
    }
}

#[test]
fn placeholders_list_matches_the_message_order() {
    assert_eq!(
        PromptTemplate::PLACEHOLDERS,
        ["issue", "issue_title", "issue_body", "branch", "type", "name"]
    );
}
