//! A tool result's parts, read from its `User` event's line (research R5, FR-011).

use super::entry::ResultPart;
use serde_json::Value;

/// The parts of the tool result at `block` of the `User` event whose line is `user_raw`, its
/// `content` as #938 parsed it. A string content is one text part, even when the text is JSON; a
/// block list is its `text` blocks as text parts, in order, and each other block named as a part
/// that is not shown. A line that cannot be read gives one text part of `content`.
pub fn result_parts(user_raw: &str, block: usize, content: &str) -> Vec<ResultPart> {
    let line = serde_json::from_str::<Value>(user_raw).unwrap_or(Value::Null);
    match &line["message"]["content"][block]["content"] {
        Value::Array(parts) => parts.iter().map(part_of).collect(),
        _ => vec![ResultPart::Text(content.to_string())],
    }
}

fn part_of(part: &Value) -> ResultPart {
    match (part["type"].as_str(), part["text"].as_str()) {
        (Some("text"), Some(text)) => ResultPart::Text(text.to_string()),
        (kind, _) => ResultPart::Unshown {
            kind: kind.unwrap_or("unknown").to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn line(content: serde_json::Value) -> String {
        json!({
            "type": "user",
            "message": {"role": "user", "content": [
                {"type": "text", "text": "first block"},
                {"type": "tool_result", "tool_use_id": "t", "content": content},
            ]},
        })
        .to_string()
    }

    #[test]
    fn a_string_content_is_one_text_part_even_when_it_is_json() {
        let content = r#"[{"type":"text","text":"x"}]"#;
        assert_eq!(
            result_parts(&line(json!(content)), 1, content),
            vec![ResultPart::Text(content.to_string())]
        );
    }

    #[test]
    fn a_block_list_is_its_text_parts_and_names_the_rest() {
        let content = json!([
            {"type": "text", "text": "one"},
            {"type": "image", "source": {"type": "base64", "data": "AAAA"}},
            {"type": "text", "text": "two"},
            {"no_type": true},
        ]);
        assert_eq!(
            result_parts(&line(content.clone()), 1, &content.to_string()),
            vec![
                ResultPart::Text("one".to_string()),
                ResultPart::Unshown {
                    kind: "image".to_string()
                },
                ResultPart::Text("two".to_string()),
                ResultPart::Unshown {
                    kind: "unknown".to_string()
                },
            ]
        );
    }

    #[test]
    fn an_unreadable_line_gives_the_content_as_text() {
        assert_eq!(
            result_parts("not json", 0, "the text"),
            vec![ResultPart::Text("the text".to_string())]
        );
        assert_eq!(
            result_parts(&line(json!("x")), 5, "the text"),
            vec![ResultPart::Text("the text".to_string())],
            "no such block"
        );
    }
}
