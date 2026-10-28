//! A structured session's reasoning effort (#1554, research R2): the level names the app accepts,
//! the user's choice, what the CLI reports a model supports, and the level a launch or a change
//! uses. Render-free and process-free.

use super::model::ModelChoice;
use super::ReportedModel;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The longest level name the app accepts, in bytes.
pub const MAX_EFFORT_LEVEL_BYTES: usize = 16;

/// Claude Code's levels, least to most: offered when the CLI reports none for the session model
/// (FR-002, clarification D4).
pub const EFFORT_LEVELS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// Names that are never a level: `default` and `auto` mean [`EffortChoice::Default`] (spec D3),
/// `ultracode` is a mode out of this feature's scope.
const NOT_LEVELS: [&str; 3] = ["default", "auto", "ultracode"];

/// A level name: lowercase ASCII letters only, 1 to [`MAX_EFFORT_LEVEL_BYTES`] bytes, never
/// `default`, `auto` or `ultracode` (so it is never confused with Default or another mode, and never
/// read as a flag after `--effort`). A newtype, not an enum, so a level a later CLI reports is
/// offered without an app release (spec Assumption 4). Checked again when read back.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EffortLevel(String);

impl EffortLevel {
    /// `name` lowercased, when it is a level name.
    pub fn parse(name: &str) -> Result<Self, InvalidEffortLevel> {
        let name = name.to_ascii_lowercase();
        let valid = !name.is_empty()
            && name.len() <= MAX_EFFORT_LEVEL_BYTES
            && name.bytes().all(|b| b.is_ascii_lowercase())
            && !NOT_LEVELS.contains(&name.as_str());
        if valid {
            Ok(Self(name))
        } else {
            Err(InvalidEffortLevel)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EffortLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for EffortLevel {
    type Error = InvalidEffortLevel;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        Self::parse(&name)
    }
}

impl From<EffortLevel> for String {
    fn from(level: EffortLevel) -> Self {
        level.0
    }
}

/// A name [`EffortLevel::parse`] refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidEffortLevel;

impl fmt::Display for InvalidEffortLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "an effort level is one word of lowercase letters, at most {MAX_EFFORT_LEVEL_BYTES} bytes"
        )
    }
}

impl std::error::Error for InvalidEffortLevel {}

/// The effort a user chose for a session (FR-010): Claude Code's own default, or a level.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffortChoice {
    #[default]
    Default,
    Level(EffortLevel),
}

impl EffortChoice {
    /// Whether this is [`EffortChoice::Default`]; a store skips it.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }
}

impl fmt::Display for EffortChoice {
    /// `Default`, or the level's name.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => f.write_str("Default"),
            Self::Level(level) => f.write_str(level.as_str()),
        }
    }
}

/// What the CLI reported about one model's effort (`supportsEffort`, `supportedEffortLevels`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffortSupport {
    pub supported: bool,
    /// In the CLI's order; a name [`EffortLevel::parse`] refuses is dropped.
    pub levels: Vec<EffortLevel>,
}

impl EffortSupport {
    /// Whether `level` is one the model takes: it supports effort and lists the level, or lists
    /// none (then the levels are not known, and [`offered_levels`] offers all of
    /// [`EFFORT_LEVELS`]).
    fn takes(&self, level: &EffortLevel) -> bool {
        self.supported && (self.levels.is_empty() || self.levels.contains(level))
    }
}

/// What the CLI reported about `model`'s effort: the reported model whose `value` is the choice's
/// [`cli_value`](ModelChoice::cli_value) (`default` for a Default choice). `None` (unknown) when no
/// reported model matches, or the one that does carried no effort fields.
pub fn effort_support<'a>(
    models: &'a [ReportedModel],
    model: &ModelChoice,
) -> Option<&'a EffortSupport> {
    models
        .iter()
        .find(|m| m.value == model.cli_value())
        .and_then(|m| m.effort.as_ref())
}

/// The levels the picker offers (FR-002): the reported ones in the CLI's order when the model
/// supports effort and lists some; otherwise [`EFFORT_LEVELS`] (none reported, or the model does
/// not support effort, clarification D4).
pub fn offered_levels(support: Option<&EffortSupport>) -> Vec<EffortLevel> {
    match support {
        Some(support) if support.supported && !support.levels.is_empty() => support.levels.clone(),
        _ => EFFORT_LEVELS
            .iter()
            .map(|name| EffortLevel(name.to_string()))
            .collect(),
    }
}

/// The level a launch or a change uses for `choice` (FR-003, FR-010a): none for `Default`; the
/// chosen level when support is unknown or the model takes it; none when the model does not
/// support effort or does not list the level (Claude Code's default then applies).
pub fn effective_level<'a>(
    choice: &'a EffortChoice,
    support: Option<&EffortSupport>,
) -> Option<&'a EffortLevel> {
    match choice {
        EffortChoice::Default => None,
        EffortChoice::Level(level) => support
            .is_none_or(|support| support.takes(level))
            .then_some(level),
    }
}

/// Whether the chosen effort applies to the session model, for the view's words (FR-003, R7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffortAvailability {
    /// The choice applies (or support is unknown, so it is sent as is).
    Applies,
    /// The model does not support reasoning effort; Claude Code's default applies.
    ModelHasNoEffort,
    /// The model does not take this level; Claude Code's default applies.
    LevelNotForModel(EffortLevel),
}

/// Whether `choice` applies to a model with `support` (FR-003).
pub fn availability(choice: &EffortChoice, support: Option<&EffortSupport>) -> EffortAvailability {
    match (choice, support) {
        (EffortChoice::Level(_), Some(support)) if !support.supported => {
            EffortAvailability::ModelHasNoEffort
        }
        (EffortChoice::Level(level), Some(support)) if !support.takes(level) => {
            EffortAvailability::LevelNotForModel(level.clone())
        }
        _ => EffortAvailability::Applies,
    }
}

/// An `/effort` argument the app does not accept (FR-007).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffortArgError {
    /// The argument, trimmed.
    pub given: String,
    /// `default`, then the offered levels.
    pub accepted: Vec<String>,
}

impl fmt::Display for EffortArgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Use /effort with one of: {}", self.accepted.join(", "))
    }
}

impl std::error::Error for EffortArgError {}

/// The choice an `/effort` argument names (FR-007): trimmed, any letter case; `default` and `auto`
/// are [`EffortChoice::Default`]; a level in `offered` is that level; anything else is refused,
/// naming the accepted values.
pub fn parse_effort_arg(
    arg: &str,
    offered: &[EffortLevel],
) -> Result<EffortChoice, EffortArgError> {
    let given = arg.trim();
    let lowered = given.to_ascii_lowercase();
    if lowered == "default" || lowered == "auto" {
        return Ok(EffortChoice::Default);
    }
    match EffortLevel::parse(&lowered) {
        Ok(level) if offered.contains(&level) => Ok(EffortChoice::Level(level)),
        _ => Err(EffortArgError {
            given: given.to_string(),
            accepted: std::iter::once("default".to_string())
                .chain(offered.iter().map(|level| level.as_str().to_string()))
                .collect(),
        }),
    }
}

/// Why an effort change did not take (FR-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffortRefusal {
    /// The CLI refused it, with its reason.
    Refused { message: String },
    /// The CLI gave no answer in time.
    NoAnswer,
}

impl fmt::Display for EffortRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused { message } => f.write_str(message),
            Self::NoAnswer => f.write_str("Claude Code did not answer the effort change"),
        }
    }
}

impl std::error::Error for EffortRefusal {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structured::model::ModelName;

    fn level(name: &str) -> EffortLevel {
        EffortLevel::parse(name).expect("a level")
    }

    fn levels(names: &[&str]) -> Vec<EffortLevel> {
        names.iter().map(|name| level(name)).collect()
    }

    fn support(supported: bool, names: &[&str]) -> EffortSupport {
        EffortSupport {
            supported,
            levels: levels(names),
        }
    }

    fn model(value: &str, effort: Option<EffortSupport>) -> ReportedModel {
        ReportedModel {
            value: value.into(),
            display_name: None,
            description: None,
            effort,
        }
    }

    #[test]
    fn a_level_is_lowercase_letters_of_at_most_sixteen_bytes() {
        for (name, parsed) in [
            ("high", "high"),
            ("xhigh", "xhigh"),
            ("MAX", "max"),
            ("Low", "low"),
            ("a", "a"),
            ("abcdefghijklmnop", "abcdefghijklmnop"),
        ] {
            assert_eq!(
                EffortLevel::parse(name).map(|l| l.as_str().to_string()),
                Ok(parsed.to_string()),
                "{name}"
            );
        }
        for name in [
            "",
            " ",
            "abcdefghijklmnopq",
            "-x",
            "x-high",
            "high on",
            " high",
            "hi9h",
            "default",
            "auto",
            "Auto",
            "ultracode",
            "ULTRACODE",
            "émax",
        ] {
            assert_eq!(
                EffortLevel::parse(name),
                Err(InvalidEffortLevel),
                "{name:?}"
            );
        }
    }

    #[test]
    fn a_level_read_back_is_checked_again() {
        let json = |name: &str| serde_json::to_string(name).unwrap();
        assert_eq!(
            serde_json::from_str::<EffortLevel>(&json("high")).unwrap(),
            level("high")
        );
        for name in [
            "",
            "abcdefghijklmnopq",
            "-x",
            "x-high",
            "default",
            "Auto",
            "ultracode",
        ] {
            assert!(
                serde_json::from_str::<EffortLevel>(&json(name)).is_err(),
                "{name:?}"
            );
        }
        assert_eq!(serde_json::to_string(&level("max")).unwrap(), "\"max\"");
    }

    #[test]
    fn the_default_choice_is_default() {
        assert_eq!(EffortChoice::default(), EffortChoice::Default);
        assert!(EffortChoice::default().is_default());
        assert!(!EffortChoice::Level(level("high")).is_default());
        assert_eq!(EffortChoice::Default.to_string(), "Default");
        assert_eq!(EffortChoice::Level(level("high")).to_string(), "high");
    }

    #[test]
    fn support_is_the_reported_model_the_model_choice_names() {
        let models = vec![
            model("default", Some(support(true, &["low", "high"]))),
            model("haiku", Some(support(false, &[]))),
            model("opus", None),
        ];
        let named = |name: &str| ModelChoice::Named(ModelName::parse(name).unwrap());
        assert_eq!(
            effort_support(&models, &ModelChoice::Default),
            Some(&support(true, &["low", "high"]))
        );
        assert_eq!(
            effort_support(&models, &named("haiku")),
            Some(&support(false, &[]))
        );
        // Listed without effort fields, and not listed at all: unknown.
        assert_eq!(effort_support(&models, &named("opus")), None);
        assert_eq!(effort_support(&models, &named("claude-opus-5-5")), None);
        assert_eq!(effort_support(&[], &ModelChoice::Default), None);
    }

    #[test]
    fn the_offered_levels_are_the_reported_ones_or_all_five() {
        let all = levels(&EFFORT_LEVELS);
        let cases: &[(Option<EffortSupport>, Vec<EffortLevel>)] = &[
            (
                Some(support(true, &["max", "low", "high"])),
                levels(&["max", "low", "high"]),
            ),
            (Some(support(true, &[])), all.clone()),
            (Some(support(false, &[])), all.clone()),
            (Some(support(false, &["low"])), all.clone()),
            (None, all.clone()),
        ];
        for (support, offered) in cases {
            assert_eq!(&offered_levels(support.as_ref()), offered, "{support:?}");
        }
    }

    #[test]
    fn the_effective_level_is_the_choice_where_the_model_takes_it() {
        let high = EffortChoice::Level(level("high"));
        let cases: &[(&EffortChoice, Option<EffortSupport>, Option<&str>)] = &[
            (&EffortChoice::Default, None, None),
            (&EffortChoice::Default, Some(support(true, &["high"])), None),
            (&high, None, Some("high")),
            (&high, Some(support(true, &["low", "high"])), Some("high")),
            (&high, Some(support(true, &[])), Some("high")),
            (&high, Some(support(true, &["low", "max"])), None),
            (&high, Some(support(false, &[])), None),
            (&high, Some(support(false, &["high"])), None),
        ];
        for (choice, support, effective) in cases {
            assert_eq!(
                effective_level(choice, support.as_ref()).map(EffortLevel::as_str),
                *effective,
                "{choice:?} with {support:?}"
            );
        }
    }

    #[test]
    fn availability_says_why_a_choice_does_not_apply() {
        let high = EffortChoice::Level(level("high"));
        let cases: &[(&EffortChoice, Option<EffortSupport>, EffortAvailability)] = &[
            (&EffortChoice::Default, None, EffortAvailability::Applies),
            (
                &EffortChoice::Default,
                Some(support(false, &[])),
                EffortAvailability::Applies,
            ),
            (&high, None, EffortAvailability::Applies),
            (
                &high,
                Some(support(true, &["high"])),
                EffortAvailability::Applies,
            ),
            (
                &high,
                Some(support(false, &[])),
                EffortAvailability::ModelHasNoEffort,
            ),
            (
                &high,
                Some(support(true, &["low"])),
                EffortAvailability::LevelNotForModel(level("high")),
            ),
        ];
        for (choice, support, expected) in cases {
            assert_eq!(
                &availability(choice, support.as_ref()),
                expected,
                "{choice:?} with {support:?}"
            );
        }
    }

    #[test]
    fn an_effort_argument_is_default_or_an_offered_level() {
        let offered = levels(&EFFORT_LEVELS);
        for (arg, choice) in [
            ("high", EffortChoice::Level(level("high"))),
            (" Max ", EffortChoice::Level(level("max"))),
            ("XHIGH", EffortChoice::Level(level("xhigh"))),
            ("default", EffortChoice::Default),
            ("AUTO", EffortChoice::Default),
            (" auto\n", EffortChoice::Default),
        ] {
            assert_eq!(parse_effort_arg(arg, &offered), Ok(choice), "{arg:?}");
        }
        for arg in ["extreme", "ultracode", "high on", "", "-x"] {
            let error = parse_effort_arg(arg, &offered).expect_err(arg);
            assert_eq!(error.given, arg.trim());
            assert_eq!(
                error.to_string(),
                "Use /effort with one of: default, low, medium, high, xhigh, max",
                "{arg:?}"
            );
        }
    }

    #[test]
    fn a_level_the_model_is_not_offered_is_refused_naming_the_offered_ones() {
        let offered = levels(&["low", "high"]);
        let error = parse_effort_arg("max", &offered).expect_err("not offered");
        assert_eq!(
            error,
            EffortArgError {
                given: "max".into(),
                accepted: vec!["default".into(), "low".into(), "high".into()],
            }
        );
        assert_eq!(
            error.to_string(),
            "Use /effort with one of: default, low, high"
        );
        assert_eq!(
            parse_effort_arg("Low", &offered),
            Ok(EffortChoice::Level(level("low")))
        );
    }

    #[test]
    fn a_refusal_says_why_in_words() {
        let refused = EffortRefusal::Refused {
            message: "apply_flag_settings is not supported in this context".into(),
        };
        assert_eq!(
            refused.to_string(),
            "apply_flag_settings is not supported in this context"
        );
        assert_eq!(
            EffortRefusal::NoAnswer.to_string(),
            "Claude Code did not answer the effort change"
        );
    }
}
