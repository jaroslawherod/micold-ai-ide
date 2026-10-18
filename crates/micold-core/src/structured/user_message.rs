//! A user message to a structured session (research R7, FR-008): why one is refused, and the
//! stream-json line that carries it to the CLI.

use std::fmt;

use serde::Serialize;

/// Why a user message was not delivered (data-model *UserMessageRefusal*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserMessageRefusal {
    /// The text is empty or only whitespace.
    Blank,
    /// The session's CLI is not running.
    NotRunning,
    /// The CLI has not finished answering the previous message.
    TurnInProgress,
    /// The session is a terminal session: type into its terminal instead.
    NotStructured,
}

impl fmt::Display for UserMessageRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Blank => "The message is empty: write something to send",
            Self::NotRunning => "The session is not running: start it to send a message",
            Self::TurnInProgress => {
                "Claude Code is still answering the previous message: send this one when it is done"
            }
            Self::NotStructured => {
                "This is a terminal session: type into its terminal to send it a message"
            }
        })
    }
}

/// The stream-json line that delivers `text` as one user message, newline included. The text goes
/// through `serde_json` escaping, so lines, quotes and non-ASCII text arrive unchanged.
pub fn encode_user_message(text: &str) -> String {
    // Typed rather than `json!`, so the keys keep the contract's order (a `serde_json::Map` sorts).
    #[derive(Serialize)]
    struct Line<'a> {
        r#type: &'static str,
        message: Message<'a>,
    }
    #[derive(Serialize)]
    struct Message<'a> {
        role: &'static str,
        content: [Text<'a>; 1],
    }
    #[derive(Serialize)]
    struct Text<'a> {
        r#type: &'static str,
        text: &'a str,
    }
    let line = Line {
        r#type: "user",
        message: Message {
            role: "user",
            content: [Text {
                r#type: "text",
                text,
            }],
        },
    };
    // Serializing plain strings into JSON cannot fail.
    let mut line = serde_json::to_string(&line).unwrap_or_default();
    line.push('\n');
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_refusal_has_a_user_sentence() {
        for refusal in [
            UserMessageRefusal::Blank,
            UserMessageRefusal::NotRunning,
            UserMessageRefusal::TurnInProgress,
            UserMessageRefusal::NotStructured,
        ] {
            let sentence = refusal.to_string();
            assert!(
                sentence.len() > 10 && sentence.chars().next().is_some_and(char::is_uppercase),
                "{refusal:?} has no sentence: {sentence:?}"
            );
        }
    }

    #[test]
    fn a_message_is_exactly_one_stream_json_user_line() {
        assert_eq!(
            encode_user_message("hi"),
            "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"hi\"}]}}\n"
        );
    }

    #[test]
    fn multi_line_quoted_and_non_ascii_text_decodes_back_unchanged() {
        for text in [
            "two\nlines\r\nand a tab\t",
            "say \"hi\" \\ back",
            "zażółć 🦀 日本",
        ] {
            let line = encode_user_message(text);
            assert!(line.ends_with('\n'));
            assert_eq!(line.matches('\n').count(), 1, "one line: {line:?}");
            let value: serde_json::Value = serde_json::from_str(&line).expect("json");
            assert_eq!(value["message"]["content"][0]["text"], text);
        }
    }
}
