//! A tool card's one-line input summary (research R6, FR-009).

use serde_json::{Map, Value};

/// The summary's longest length, in characters, the ellipsis included.
pub const SUMMARY_MAX_CHARS: usize = 240;

/// The input field that identifies the work for the tools Claude Code uses most, else a generic
/// `key: value, …` summary of the input, else the input text itself; on one line, cut with `…`.
pub fn input_summary(name: &str, input_json: &str) -> String {
    let text = match serde_json::from_str::<Value>(input_json) {
        Ok(Value::Object(input)) => work_str(name, &input)
            .map(str::to_string)
            .unwrap_or_else(|| generic(&input)),
        _ => input_json.to_string(),
    };
    one_line(&text)
}

/// The full value the summary is drawn from (feature 1550 FR-015): the work field's string,
/// uncut and on all its lines; `None` for a tool without one, a missing or non-string field and
/// input that is not a JSON object.
pub fn work_value(name: &str, input_json: &str) -> Option<String> {
    match serde_json::from_str::<Value>(input_json) {
        Ok(Value::Object(input)) => work_str(name, &input).map(str::to_string),
        _ => None,
    }
}

/// The string in `input`'s work field, when the tool has one and it holds a string.
fn work_str<'a>(name: &str, input: &'a Map<String, Value>) -> Option<&'a str> {
    work_field(name)
        .and_then(|field| input.get(field))
        .and_then(Value::as_str)
}

/// The input field that names a common tool's work.
fn work_field(name: &str) -> Option<&'static str> {
    Some(match name {
        "Bash" => "command",
        "Read" | "Write" | "Edit" | "MultiEdit" => "file_path",
        "NotebookEdit" => "notebook_path",
        "Grep" | "Glob" => "pattern",
        "WebFetch" => "url",
        "WebSearch" => "query",
        "Task" | "Agent" => "description",
        _ => return None,
    })
}

/// The input's top-level `key: value` pairs, strings unquoted.
fn generic(input: &Map<String, Value>) -> String {
    let pairs: Vec<String> = input
        .iter()
        .map(|(key, value)| match value {
            Value::String(text) => format!("{key}: {text}"),
            other => format!("{key}: {other}"),
        })
        .collect();
    pairs.join(", ")
}

/// `text` with every whitespace run one space, cut to [`SUMMARY_MAX_CHARS`] with `…`.
fn one_line(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let line = words.join(" ");
    if line.chars().count() <= SUMMARY_MAX_CHARS {
        return line;
    }
    let mut cut: String = line.chars().take(SUMMARY_MAX_CHARS - 1).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_common_tool_is_summarised_by_the_field_that_names_its_work() {
        for (name, input, summary) in [
            (
                "Bash",
                r#"{"command":"cargo test","description":"run"}"#,
                "cargo test",
            ),
            (
                "Read",
                r#"{"file_path":"/src/a.rs","limit":5}"#,
                "/src/a.rs",
            ),
            ("Write", r#"{"file_path":"/b","content":"x"}"#, "/b"),
            ("Edit", r#"{"file_path":"/c","old_string":"a"}"#, "/c"),
            ("MultiEdit", r#"{"file_path":"/d","edits":[]}"#, "/d"),
            (
                "NotebookEdit",
                r#"{"notebook_path":"/n.ipynb"}"#,
                "/n.ipynb",
            ),
            ("Grep", r#"{"pattern":"fn main","path":"src"}"#, "fn main"),
            ("Glob", r#"{"pattern":"**/*.rs"}"#, "**/*.rs"),
            (
                "WebFetch",
                r#"{"url":"https://a.example","prompt":"p"}"#,
                "https://a.example",
            ),
            ("WebSearch", r#"{"query":"rust iced"}"#, "rust iced"),
            (
                "Task",
                r#"{"description":"look around","prompt":"p"}"#,
                "look around",
            ),
            (
                "Agent",
                r#"{"description":"review","prompt":"p"}"#,
                "review",
            ),
        ] {
            assert_eq!(input_summary(name, input), summary, "{name}");
        }
    }

    #[test]
    fn another_tool_or_a_missing_field_gets_the_generic_summary() {
        assert_eq!(
            input_summary("mcp__x__y", r#"{"a":"one","b":2,"c":[true]}"#),
            "a: one, b: 2, c: [true]",
            "strings unquoted"
        );
        assert_eq!(input_summary("Bash", r#"{"cmd":"ls"}"#), "cmd: ls");
        assert_eq!(
            input_summary("Read", r#"{"file_path":7}"#),
            "file_path: 7",
            "a field of another type"
        );
        assert_eq!(input_summary("Bash", "{}"), "");
    }

    #[test]
    fn input_that_is_not_an_object_is_its_own_text() {
        assert_eq!(input_summary("Bash", "null"), "null");
        assert_eq!(input_summary("Bash", r#"[1, 2]"#), "[1, 2]");
        assert_eq!(
            input_summary("Read", r#"{"file_path": "/src/a"#),
            r#"{"file_path": "/src/a"#,
            "a partial input while it streams"
        );
    }

    #[test]
    fn whitespace_runs_collapse_to_one_space() {
        assert_eq!(
            input_summary("Bash", r#"{"command":"cd x &&\n\tcargo   test\r\n"}"#),
            "cd x && cargo test"
        );
    }

    #[test]
    fn a_long_summary_is_cut_with_an_ellipsis() {
        let long = "é".repeat(500);
        let summary = input_summary("Bash", &format!(r#"{{"command":"{long}"}}"#));
        assert_eq!(summary.chars().count(), SUMMARY_MAX_CHARS);
        assert!(summary.ends_with('…'));
        assert!(summary.starts_with("éé"));
        let exact = "a".repeat(SUMMARY_MAX_CHARS);
        assert_eq!(
            input_summary("Bash", &format!(r#"{{"command":"{exact}"}}"#)),
            exact,
            "one that fits is not cut"
        );
    }

    #[test]
    fn work_value_is_the_work_field_uncut_on_all_its_lines() {
        let long = format!("cat {}", "x/".repeat(SUMMARY_MAX_CHARS));
        for (name, field) in [
            ("Bash", "command"),
            ("Read", "file_path"),
            ("Write", "file_path"),
            ("Edit", "file_path"),
            ("MultiEdit", "file_path"),
            ("NotebookEdit", "notebook_path"),
            ("Grep", "pattern"),
            ("Glob", "pattern"),
            ("WebFetch", "url"),
            ("WebSearch", "query"),
            ("Task", "description"),
            ("Agent", "description"),
        ] {
            for value in ["one", "cd x &&\n\tcargo   test\n", long.as_str()] {
                let input = serde_json::json!({ field: value, "other": "o" }).to_string();
                assert_eq!(
                    work_value(name, &input).as_deref(),
                    Some(value),
                    "{name}: {value:?}"
                );
            }
        }
        assert!(
            long.chars().count() > SUMMARY_MAX_CHARS,
            "longer than the cut"
        );
    }

    #[test]
    fn work_value_is_none_without_a_work_field() {
        assert_eq!(
            work_value("mcp__x__y", r#"{"command":"ls"}"#),
            None,
            "no work field"
        );
        assert_eq!(work_value("Bash", r#"{"cmd":"ls"}"#), None, "missing");
        assert_eq!(
            work_value("Read", r#"{"file_path":7}"#),
            None,
            "not a string"
        );
        assert_eq!(work_value("Bash", r#"{"command": "l"#), None, "malformed");
        assert_eq!(work_value("Bash", "null"), None, "not an object");
    }
}
