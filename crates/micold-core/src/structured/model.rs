//! A structured session's model (#943 FR-013–FR-018, research R7): the user's choice, how it is
//! launched and the model the session runs.

use super::{SequencedEvent, StructuredEvent};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The longest model name the app accepts, in bytes (FR-014).
pub const MAX_MODEL_NAME_BYTES: usize = 128;

/// The model names Claude Code accepts as shortcuts, offered when the CLI reports no models
/// (FR-014).
pub const MODEL_SHORTCUTS: [&str; 3] = ["sonnet", "opus", "haiku"];

/// A model name: non-blank, no whitespace, at most [`MAX_MODEL_NAME_BYTES`]. Checked again when
/// read back from the store.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ModelName(String);

impl ModelName {
    pub fn parse(name: &str) -> Result<Self, ModelRefusal> {
        let valid = !name.is_empty()
            && name.len() <= MAX_MODEL_NAME_BYTES
            && !name.contains(char::is_whitespace);
        if valid {
            Ok(Self(name.to_string()))
        } else {
            Err(ModelRefusal::Invalid)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ModelName {
    type Error = ModelRefusal;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        Self::parse(&name)
    }
}

impl From<ModelName> for String {
    fn from(name: ModelName) -> Self {
        name.0
    }
}

/// The model a user chose for a session (FR-017): the CLI's default, or a name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelChoice {
    #[default]
    Default,
    Named(ModelName),
}

impl ModelChoice {
    /// Whether this is [`ModelChoice::Default`]; a store skips it.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }

    /// The value `set_model` takes: `default`, or the name (contracts/claude-commands.md).
    pub fn cli_value(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Named(name) => name.as_str(),
        }
    }
}

/// Why a model choice did not take (FR-016).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelRefusal {
    /// Not a model name the app accepts.
    Invalid,
    /// The CLI refused it, with its reason and code.
    Refused {
        message: String,
        code: Option<String>,
    },
    /// The CLI gave no usable answer.
    NoAnswer,
}

impl fmt::Display for ModelRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => write!(
                f,
                "a model name is one word of at most {MAX_MODEL_NAME_BYTES} bytes"
            ),
            Self::Refused { message, .. } => f.write_str(message),
            Self::NoAnswer => f.write_str("Claude Code did not answer the model change"),
        }
    }
}

impl std::error::Error for ModelRefusal {}

/// The arguments a launch adds for `choice` (FR-017): none for the default.
pub fn launch_args(choice: &ModelChoice) -> Vec<String> {
    match choice {
        ModelChoice::Default => Vec::new(),
        ModelChoice::Named(name) => vec!["--model".to_string(), name.as_str().to_string()],
    }
}

/// The model the session runs (FR-013): the latest an `init` reported or a change the CLI
/// accepted (`ModelChanged`) named.
pub fn session_model(events: &[SequencedEvent]) -> Option<String> {
    events.iter().rev().find_map(|e| match &e.event {
        StructuredEvent::Init {
            model: Some(model), ..
        }
        | StructuredEvent::ModelChanged { model } => Some(model.clone()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structured::{parse_line, SequencedEvent, StructuredEvent};
    use serde::{Deserialize, Serialize};

    #[test]
    fn a_model_name_is_non_blank_without_whitespace_and_at_most_128_bytes() {
        assert_eq!(
            ModelName::parse("haiku").map(|n| n.as_str().to_string()),
            Ok("haiku".into())
        );
        assert!(ModelName::parse("claude-opus-5-5[1m]").is_ok());
        assert!(ModelName::parse(&"m".repeat(128)).is_ok());
        for bad in ["", "   ", "two words", "tab\there", "line\n", " haiku"] {
            assert_eq!(ModelName::parse(bad), Err(ModelRefusal::Invalid), "{bad:?}");
        }
        assert_eq!(
            ModelName::parse(&"m".repeat(129)),
            Err(ModelRefusal::Invalid)
        );
        assert_eq!(
            ModelName::parse(&"é".repeat(65)),
            Err(ModelRefusal::Invalid),
            "130 bytes"
        );
    }

    #[test]
    fn only_a_named_choice_adds_launch_arguments() {
        assert!(launch_args(&ModelChoice::Default).is_empty());
        let haiku = ModelChoice::Named(ModelName::parse("haiku").unwrap());
        assert_eq!(launch_args(&haiku), ["--model", "haiku"]);
    }

    #[test]
    fn the_shortcuts_are_the_three_claude_code_accepts() {
        assert_eq!(MODEL_SHORTCUTS, ["sonnet", "opus", "haiku"]);
    }

    fn history(events: Vec<StructuredEvent>) -> Vec<SequencedEvent> {
        events
            .into_iter()
            .enumerate()
            .map(|(seq, event)| SequencedEvent {
                seq: seq as u64,
                event,
            })
            .collect()
    }

    fn init(model: Option<&str>) -> StructuredEvent {
        let model = model.map_or("null".to_string(), |m| format!("\"{m}\""));
        parse_line(&format!(
            r#"{{"type":"system","subtype":"init","session_id":"s","model":{model}}}"#
        ))
    }

    #[test]
    fn the_session_model_is_the_latest_the_cli_reported_or_accepted() {
        assert_eq!(session_model(&[]), None);
        let changed = |m: &str| StructuredEvent::ModelChanged { model: m.into() };
        assert_eq!(
            session_model(&history(vec![init(Some("a"))])).as_deref(),
            Some("a")
        );
        assert_eq!(
            session_model(&history(vec![init(Some("a")), changed("b")])).as_deref(),
            Some("b")
        );
        assert_eq!(
            session_model(&history(vec![changed("b"), init(Some("c"))])).as_deref(),
            Some("c")
        );
        assert_eq!(
            session_model(&history(vec![init(Some("a")), init(None)])).as_deref(),
            Some("a"),
            "an init without a model does not forget it"
        );
        let refused = StructuredEvent::ModelRefused {
            model: "x".into(),
            message: "no".into(),
        };
        assert_eq!(
            session_model(&history(vec![init(Some("a")), refused])).as_deref(),
            Some("a"),
            "a refusal changes nothing"
        );
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Stored {
        name: String,
        #[serde(default, skip_serializing_if = "ModelChoice::is_default")]
        model: ModelChoice,
    }

    #[test]
    fn a_default_choice_is_not_stored_and_a_missing_one_reads_default() {
        let default = Stored {
            name: "s".into(),
            model: ModelChoice::Default,
        };
        assert_eq!(serde_json::to_string(&default).unwrap(), r#"{"name":"s"}"#);
        let read: Stored = serde_json::from_str(r#"{"name":"s"}"#).unwrap();
        assert_eq!(read, default);
        let named = Stored {
            name: "s".into(),
            model: ModelChoice::Named(ModelName::parse("haiku").unwrap()),
        };
        let text = serde_json::to_string(&named).unwrap();
        assert_eq!(serde_json::from_str::<Stored>(&text).unwrap(), named);
        assert!(
            serde_json::from_str::<Stored>(r#"{"name":"s","model":{"Named":"two words"}}"#)
                .is_err(),
            "a stored name is checked as a typed one"
        );
        let bytes = postcard::to_allocvec(&named.model).unwrap();
        assert_eq!(
            postcard::from_bytes::<ModelChoice>(&bytes).unwrap(),
            named.model
        );
    }
}
