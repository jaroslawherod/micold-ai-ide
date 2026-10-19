//! Hook-fed agent activity (feature 813, research R10): the current todo list and prompt an AI
//! CLI's hooks report for a session, shown on its location's Overview card.
//!
//! Carried on the wire as `SessionSummary::hook_activity`. It holds a user's prompt and todo text,
//! so nothing here may be logged (FR-020): the types derive `Debug` only for tests' sake and no
//! caller formats them.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::task_progress::Ratio;

/// At most this many todos are kept.
pub const MAX_TODOS: usize = 50;
/// A todo's text, its active form and the prompt's readable words are cut to this many characters.
pub const MAX_TEXT_CHARS: usize = 200;

/// A todo's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

/// One todo of the agent's list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todo {
    pub text: String,
    pub active: Option<String>,
    pub status: TodoStatus,
}

/// What a session's hooks have reported.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookActivity {
    pub todos: Vec<Todo>,
    pub prompt: Option<String>,
}

/// The one line a card shows and the completed / total ratio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentActivity {
    pub line: String,
    pub ratio: Option<Ratio>,
}

fn clip(s: &str) -> String {
    s.chars().take(MAX_TEXT_CHARS).collect()
}

/// A todo's readable words, clipped after sanitising like the prompt (research R8); `None` when
/// nothing readable is left.
fn readable_clip(s: &str) -> Option<String> {
    crate::markup::readable(s).map(|s| clip(&s))
}

/// One todo item; an item whose content has nothing readable is skipped like an empty one.
fn parse_todo(item: &Value) -> Option<Todo> {
    let text = readable_clip(item.get("content")?.as_str()?)?;
    let status = match item.get("status")?.as_str()? {
        "pending" => TodoStatus::Pending,
        "in_progress" => TodoStatus::InProgress,
        "completed" => TodoStatus::Completed,
        _ => return None,
    };
    let active = item
        .get("activeForm")
        .and_then(Value::as_str)
        .and_then(readable_clip);
    Some(Todo {
        text,
        active,
        status,
    })
}

impl HookActivity {
    /// Whether anything is held (the service's `Some(..)` condition).
    pub fn is_empty(&self) -> bool {
        self.todos.is_empty() && self.prompt.is_none()
    }

    /// Fold one hook body in. Returns `true` only when the value changed.
    pub fn apply(&mut self, body: &Value) -> bool {
        match body.get("hook_event_name").and_then(Value::as_str) {
            Some("UserPromptSubmit") => {
                let Some(prompt) = body.get("prompt").and_then(Value::as_str) else {
                    return false;
                };
                // Sanitise the whole prompt before clipping, so a clip never cuts a tag in half
                // (research R8); nothing readable clears the earlier prompt.
                let prompt = readable_clip(prompt);
                if prompt == self.prompt {
                    return false;
                }
                self.prompt = prompt;
                true
            }
            Some("PostToolUse")
                if body.get("tool_name").and_then(Value::as_str) == Some("TodoWrite") =>
            {
                let Some(items) = body.pointer("/tool_input/todos").and_then(Value::as_array)
                else {
                    return false;
                };
                let todos: Vec<Todo> = items
                    .iter()
                    .filter_map(parse_todo)
                    .take(MAX_TODOS)
                    .collect();
                if todos == self.todos {
                    return false;
                }
                self.todos = todos;
                true
            }
            _ => false,
        }
    }

    /// The line to show (in-progress `active`, else its text, else the prompt) and the ratio.
    pub fn current(&self) -> Option<CurrentActivity> {
        let ratio = (!self.todos.is_empty()).then(|| Ratio {
            done: self
                .todos
                .iter()
                .filter(|t| t.status == TodoStatus::Completed)
                .count() as u32,
            total: self.todos.len() as u32,
        });
        let line = self
            .todos
            .iter()
            .find(|t| t.status == TodoStatus::InProgress)
            .map(|t| t.active.clone().unwrap_or_else(|| t.text.clone()))
            .or_else(|| self.prompt.clone())?;
        Some(CurrentActivity { line, ratio })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn todo_write(items: Value) -> Value {
        json!({"hook_event_name": "PostToolUse", "tool_name": "TodoWrite",
               "tool_input": {"todos": items}})
    }

    fn five() -> Value {
        json!([
            {"content": "a", "activeForm": "doing a", "status": "completed"},
            {"content": "b", "activeForm": "doing b", "status": "completed"},
            {"content": "c", "activeForm": "doing c", "status": "in_progress"},
            {"content": "d", "activeForm": "doing d", "status": "pending"},
            {"content": "e", "activeForm": "doing e", "status": "pending"},
        ])
    }

    fn prompt(text: &str) -> Value {
        json!({"hook_event_name": "UserPromptSubmit", "prompt": text})
    }

    /// C10: the whole prompt's words on one line, markup removed; the same again is no change.
    #[test]
    fn prompt_is_its_readable_words_on_one_line() {
        let mut h = HookActivity::default();
        let body = prompt("<agent-message from=\"x\">\nContinue M2\n</agent-message>");
        assert!(h.apply(&body));
        assert_eq!(h.prompt.as_deref(), Some("Continue M2"));
        assert!(!h.apply(&body), "same prompt is no change");
        assert!(h.apply(&prompt("\n  fix the bug\nmore")));
        assert_eq!(h.prompt.as_deref(), Some("fix the bug more"));
    }

    /// C10: a prompt with nothing readable clears the earlier one, so it does not linger.
    #[test]
    fn a_markup_only_prompt_clears_the_prompt() {
        let mut h = HookActivity::default();
        assert!(h.apply(&prompt("Continue M2")));
        assert!(h.apply(&prompt("<agent-message from=\"x\"></agent-message>")));
        assert_eq!(h.prompt, None);
        assert!(!h.apply(&prompt(" \n ")), "still nothing: no change");
    }

    #[test]
    fn prompt_is_clipped_after_sanitising() {
        let mut h = HookActivity::default();
        let long = format!(
            "<agent-message from=\"x\">{}</agent-message>",
            "x".repeat(300)
        );
        assert!(h.apply(&prompt(&long)));
        assert_eq!(h.prompt.as_deref(), Some("x".repeat(200).as_str()));
        assert!(!h.apply(&json!({"hook_event_name": "UserPromptSubmit"})));
    }

    /// C10: todo content and `activeForm` are sanitised before clipping, like the prompt; a
    /// markup-only todo is skipped, so it never hides the prompt.
    #[test]
    fn todo_text_is_sanitised_before_clipping() {
        let mut h = HookActivity::default();
        let long = format!("<b>{}</b>", "y".repeat(300));
        assert!(h.apply(&todo_write(json!([
            {"content": long, "activeForm": "<i>\nFixing it\n</i>", "status": "in_progress"},
            {"content": "<agent-message from=\"x\"></agent-message>", "status": "in_progress"},
        ]))));
        assert_eq!(h.todos.len(), 1, "the markup-only todo is skipped");
        assert_eq!(h.todos[0].text, "y".repeat(200));
        assert_eq!(h.todos[0].active.as_deref(), Some("Fixing it"));

        let mut h = HookActivity::default();
        h.apply(&prompt("Continue M2"));
        h.apply(&todo_write(json!([
            {"content": "<agent-message from=\"x\"></agent-message>", "status": "in_progress"},
        ])));
        assert_eq!(h.current().map(|c| c.line).as_deref(), Some("Continue M2"));
    }

    #[test]
    fn todo_write_replaces_todos_and_current_uses_active_form() {
        let mut h = HookActivity::default();
        assert!(h.apply(&todo_write(five())));
        assert_eq!(h.todos.len(), 5);
        let cur = h.current().unwrap();
        assert_eq!(cur.line, "doing c");
        assert_eq!(cur.ratio, Some(Ratio { done: 2, total: 5 }));
        assert!(!h.apply(&todo_write(five())), "identical list is no change");
    }

    #[test]
    fn other_tool_or_event_is_unchanged() {
        let mut h = HookActivity::default();
        assert!(!h.apply(
            &json!({"hook_event_name": "PostToolUse", "tool_name": "Bash",
            "tool_input": {"todos": five()}})
        ));
        assert!(!h.apply(&json!({"hook_event_name": "Stop"})));
        assert!(!h.apply(&json!({})));
        assert!(h.is_empty());
    }

    #[test]
    fn malformed_items_skipped_and_non_array_unchanged() {
        let mut h = HookActivity::default();
        assert!(h.apply(&todo_write(json!([
            {"activeForm": "no content", "status": "pending"},
            {"content": "ok", "status": "pending"},
            {"content": "bad status", "status": "weird"},
            "string",
        ]))));
        assert_eq!(h.todos.len(), 1);
        assert_eq!(h.todos[0].active, None);
        assert!(!h.apply(&todo_write(json!("nope"))));
        assert_eq!(h.todos.len(), 1);
    }

    #[test]
    fn bounds_apply() {
        let mut h = HookActivity::default();
        let items: Vec<Value> = (0..80)
            .map(|_| json!({"content": "y".repeat(500), "status": "pending"}))
            .collect();
        assert!(h.apply(&todo_write(Value::Array(items))));
        assert_eq!(h.todos.len(), 50);
        assert_eq!(h.todos[0].text.chars().count(), 200);
    }

    #[test]
    fn current_precedence() {
        let mut h = HookActivity::default();
        assert_eq!(h.current(), None);
        h.prompt = Some("the prompt".into());
        assert_eq!(
            h.current(),
            Some(CurrentActivity {
                line: "the prompt".into(),
                ratio: None
            })
        );
        h.todos = vec![Todo {
            text: "t".into(),
            active: None,
            status: TodoStatus::InProgress,
        }];
        let cur = h.current().unwrap();
        assert_eq!(cur.line, "t");
        assert_eq!(cur.ratio, Some(Ratio { done: 0, total: 1 }));
        h.todos[0].status = TodoStatus::Pending;
        let cur = h.current().unwrap();
        assert_eq!(cur.line, "the prompt");
        assert_eq!(cur.ratio, Some(Ratio { done: 0, total: 1 }));
    }
}
