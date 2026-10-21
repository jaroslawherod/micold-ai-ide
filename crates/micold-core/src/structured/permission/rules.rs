//! The rule Always allow adds to a session (research R9, data-model *AllowRule*): narrower than the
//! CLI's own rule syntax, never allowing more than the user saw. The daemon keeps a session's rules
//! and answers a matching later call allow without asking.

use super::request::{PromptCall, RequestKind};
use serde_json::{json, Value};

/// A session's always-allow rule: a later call of the same tool with the same matched input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowRule {
    tool: String,
    matcher: Matcher,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Matcher {
    /// The input field `key` holds exactly `value` (`Bash` `command`, a file tool's path).
    Field { key: &'static str, value: String },
    /// A `WebFetch` URL on this host (lower-cased).
    Host(String),
    /// A `WebFetch` URL whose host could not be read: exactly this URL.
    Url(String),
    /// Any input of the tool (MCP tools and every tool not named here).
    AnyInput,
}

/// The input field a tool's rule matches on, for the tools that have one.
fn field_of(tool: &str) -> Option<&'static str> {
    match tool {
        "Bash" => Some("command"),
        "Edit" | "MultiEdit" | "Write" => Some("file_path"),
        "NotebookEdit" => Some("notebook_path"),
        "WebFetch" => Some("url"),
        _ => None,
    }
}

impl AllowRule {
    /// The rule Always allow on `call` would add, or `None` when the field the tool's rule needs is
    /// missing (a `Bash` call without `command`): Always allow is then not offered.
    pub fn for_call(call: &PromptCall) -> Option<AllowRule> {
        let tool = call.tool_name.clone();
        let Some(key) = field_of(&tool) else {
            return Some(AllowRule {
                tool,
                matcher: Matcher::AnyInput,
            });
        };
        let value = call.input.get(key)?.as_str()?.to_string();
        let matcher = match key {
            "url" => match host_of(&value) {
                Some(host) => Matcher::Host(host),
                None => Matcher::Url(value),
            },
            key => Matcher::Field { key, value },
        };
        Some(AllowRule { tool, matcher })
    }

    /// The rule Always allow offers on a request: only a tool request has one, never a plan
    /// approval or a question (research R9).
    pub fn offered(call: &PromptCall, kind: &RequestKind) -> Option<AllowRule> {
        matches!(kind, RequestKind::Tool)
            .then(|| AllowRule::for_call(call))
            .flatten()
    }

    /// Whether this rule allows `call` without asking.
    pub fn matches(&self, call: &PromptCall) -> bool {
        if call.tool_name != self.tool {
            return false;
        }
        let text = |key: &str| call.input.get(key).and_then(Value::as_str);
        match &self.matcher {
            Matcher::Field { key, value } => text(key) == Some(value.as_str()),
            Matcher::Host(host) => text("url").and_then(host_of).as_deref() == Some(host.as_str()),
            Matcher::Url(url) => text("url") == Some(url.as_str()),
            Matcher::AnyInput => true,
        }
    }

    /// The rule as the user is shown it: `Bash: "npm test"`.
    pub fn describe(&self) -> String {
        match &self.matcher {
            Matcher::Field { value, .. } | Matcher::Url(value) => {
                format!("{}: \"{value}\"", self.tool)
            }
            Matcher::Host(host) => format!("{}: any address on {host}", self.tool),
            Matcher::AnyInput => format!("{}: any input", self.tool),
        }
    }

    /// The rule in the CLI's `updatedPermissions` syntax, for the answer's `addRules` (research
    /// R5, R9), or `None` where the CLI's syntax would match more than this rule: a command or path
    /// holding a pattern character, or a file path, whose CLI rule is a path pattern relative to a
    /// settings root, or an exact URL, which the CLI cannot name. The daemon's own rule still
    /// applies either way.
    pub fn cli_rule(&self) -> Option<Value> {
        match &self.matcher {
            Matcher::Field {
                key: "command",
                value,
            } if !value.contains(['*', '?', '[']) => {
                Some(json!({"toolName": self.tool, "ruleContent": value}))
            }
            Matcher::Host(host) => {
                Some(json!({"toolName": self.tool, "ruleContent": format!("domain:{host}")}))
            }
            Matcher::AnyInput => Some(json!({"toolName": self.tool})),
            Matcher::Field { .. } | Matcher::Url(_) => None,
        }
    }
}

/// The host of `url`: the text between `://` and the next `/`, `?` or `#`, lower-cased; `None`
/// when there is no `://` or nothing after it (`url::Url` is not a dependency, research R9).
fn host_of(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    (!host.is_empty()).then(|| host.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(tool_name: &str, input: Value) -> PromptCall {
        PromptCall {
            tool_name: tool_name.to_string(),
            input: input.as_object().expect("an object").clone(),
            tool_use_id: None,
        }
    }

    fn rule(tool_name: &str, input: Value) -> AllowRule {
        AllowRule::for_call(&call(tool_name, input)).expect("a rule")
    }

    #[test]
    fn bash_matches_the_same_command_only() {
        let npm = rule("Bash", json!({"command": "npm test", "description": "Run"}));
        assert!(npm.matches(&call(
            "Bash",
            json!({"command": "npm test", "description": "Again"})
        )));
        assert!(!npm.matches(&call("Bash", json!({"command": "npm test -- --all"}))));
        assert!(!npm.matches(&call("Bash", json!({}))));
        assert_eq!(npm.describe(), "Bash: \"npm test\"");
    }

    #[test]
    fn a_bash_call_without_a_command_gets_no_rule() {
        assert_eq!(AllowRule::for_call(&call("Bash", json!({}))), None);
        assert_eq!(
            AllowRule::for_call(&call("Bash", json!({"command": 1}))),
            None
        );
    }

    #[test]
    fn file_tools_match_the_same_file_path() {
        for tool in ["Edit", "MultiEdit", "Write"] {
            let file = rule(tool, json!({"file_path": "/w/a.rs", "old_string": "x"}));
            assert!(
                file.matches(&call(tool, json!({"file_path": "/w/a.rs", "content": "y"}))),
                "{tool}"
            );
            assert!(
                !file.matches(&call(tool, json!({"file_path": "/w/b.rs"}))),
                "{tool}"
            );
            assert_eq!(file.describe(), format!("{tool}: \"/w/a.rs\""));
            assert_eq!(AllowRule::for_call(&call(tool, json!({}))), None, "{tool}");
        }
        assert!(
            !rule("Edit", json!({"file_path": "/w/a.rs"}))
                .matches(&call("Write", json!({"file_path": "/w/a.rs"}))),
            "a rule is for its own tool"
        );
    }

    #[test]
    fn notebook_edit_matches_the_same_notebook_path() {
        let notebook = rule("NotebookEdit", json!({"notebook_path": "/w/n.ipynb"}));
        assert!(notebook.matches(&call(
            "NotebookEdit",
            json!({"notebook_path": "/w/n.ipynb", "cell_id": "2"})
        )));
        assert!(!notebook.matches(&call(
            "NotebookEdit",
            json!({"notebook_path": "/w/m.ipynb"})
        )));
        assert!(!notebook.matches(&call("NotebookEdit", json!({"file_path": "/w/n.ipynb"}))));
    }

    #[test]
    fn web_fetch_matches_the_same_lower_cased_host() {
        let docs = rule(
            "WebFetch",
            json!({"url": "https://Docs.Example.com/a?b#c", "prompt": "p"}),
        );
        for url in [
            "https://docs.example.com/other",
            "http://DOCS.EXAMPLE.COM",
            "https://docs.example.com?q",
            "https://docs.example.com#top",
        ] {
            assert!(
                docs.matches(&call("WebFetch", json!({ "url": url }))),
                "{url}"
            );
        }
        assert!(!docs.matches(&call("WebFetch", json!({"url": "https://example.com/a"}))));
        assert!(!docs.matches(&call("WebFetch", json!({"url": "not a url"}))));
        assert_eq!(docs.describe(), "WebFetch: any address on docs.example.com");
    }

    #[test]
    fn an_unparseable_url_matches_the_same_url_only() {
        for url in ["not a url", "https://", "https:///path"] {
            let exact = rule("WebFetch", json!({ "url": url }));
            assert!(
                exact.matches(&call("WebFetch", json!({ "url": url }))),
                "{url}"
            );
            assert!(!exact.matches(&call("WebFetch", json!({"url": format!("{url}x")}))));
        }
    }

    #[test]
    fn any_other_tool_matches_by_name() {
        let mcp = rule("mcp__github__create_issue", json!({"title": "a"}));
        assert!(mcp.matches(&call("mcp__github__create_issue", json!({"title": "b"}))));
        assert!(!mcp.matches(&call("mcp__github__close_issue", json!({}))));
        assert_eq!(mcp.describe(), "mcp__github__create_issue: any input");
    }

    #[test]
    fn only_a_tool_request_offers_a_rule() {
        let exit = call("ExitPlanMode", json!({"plan": "p"}));
        assert_eq!(
            AllowRule::offered(&exit, &RequestKind::PlanApproval { plan: "p".into() }),
            None
        );
        let ask = call("AskUserQuestion", json!({}));
        assert_eq!(
            AllowRule::offered(&ask, &RequestKind::Question { questions: vec![] }),
            None
        );
        assert_eq!(
            AllowRule::offered(&exit, &RequestKind::Tool),
            AllowRule::for_call(&exit),
            "a plan exit that does not fit its shape is an ordinary tool request"
        );
    }

    #[test]
    fn the_cli_rule_never_names_more_than_the_rule() {
        assert_eq!(
            rule("Bash", json!({"command": "npm test"})).cli_rule(),
            Some(json!({"toolName": "Bash", "ruleContent": "npm test"}))
        );
        assert_eq!(
            rule("Bash", json!({"command": "rm *.txt"})).cli_rule(),
            None,
            "the CLI reads * as a wildcard"
        );
        assert_eq!(
            rule("WebFetch", json!({"url": "https://a.org/x"})).cli_rule(),
            Some(json!({"toolName": "WebFetch", "ruleContent": "domain:a.org"}))
        );
        assert_eq!(rule("WebFetch", json!({"url": "nope"})).cli_rule(), None);
        assert_eq!(rule("Write", json!({"file_path": "/a"})).cli_rule(), None);
        assert_eq!(
            rule("mcp__x__y", json!({})).cli_rule(),
            Some(json!({"toolName": "mcp__x__y"}))
        );
    }
}
