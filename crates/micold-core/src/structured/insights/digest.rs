//! Event → digest: the little of each event the insights need, read tolerantly (research R2, R3,
//! R8). A missing or malformed field leaves its figure `None`; a line of another shape yields no
//! digest; nothing here fails (FR-024).

use super::tasks::{kind_of, reported_state, LiveTask, TaskEvent};
use super::usage::{ModelFigures, Tokens};
use crate::structured::{ContentBlock, StructuredEvent, Usage};
use serde_json::Value;

/// What one held event contributes to the insights, kept by its `seq` (data-model.md *Digest*).
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Digest {
    /// A main-scope model response: the context it used (R2).
    AssistantUsage {
        model: Option<String>,
        used: Option<u64>,
    },
    /// A turn result: its own tokens and the CLI's running figures (R3).
    Result {
        tokens: Option<Tokens>,
        cost_cum: Option<f64>,
        models_cum: Option<Vec<(String, ModelFigures)>>,
        /// Each listed model's context window, read apart from its other figures.
        windows: Vec<(String, u64)>,
    },
    /// The daemon's note that a turn ended without a result.
    TurnEndedWithoutResult,
    /// An `autocompact_state` line: the threshold when compaction is enabled.
    Autocompact { threshold: Option<u64> },
    /// The CLI process started or restarted (R6): an `init` line with `startup_timing`. The `init`
    /// the same process writes at the start of each later turn has none, and is no digest.
    Init,
    /// A `task_started`, `task_updated`, `task_progress` or `task_notification` line (R6).
    Task(TaskEvent),
    /// A `background_tasks_changed` line: the CLI's live set of background tasks (R6).
    LiveSet(Vec<LiveTask>),
    /// A subagent's message: its parent tool call and the tool calls it holds, id and name (R7).
    Parented {
        parent: String,
        tool_uses: Vec<(String, String)>,
    },
}

/// The digest of `event`, or `None` when the event does not matter to the insights.
pub(super) fn digest(event: &StructuredEvent) -> Option<Digest> {
    match event {
        StructuredEvent::Assistant {
            parent_tool_use_id: None,
            raw,
            ..
        } => assistant_usage(raw),
        StructuredEvent::Result {
            total_cost_usd,
            usage,
            raw,
            ..
        } => Some(result(usage.as_ref(), *total_cost_usd, raw)),
        StructuredEvent::Assistant {
            parent_tool_use_id: Some(parent),
            blocks,
            ..
        }
        | StructuredEvent::User {
            parent_tool_use_id: Some(parent),
            blocks,
            ..
        } => Some(parented(parent, blocks)),
        StructuredEvent::TurnEndedWithoutResult => Some(Digest::TurnEndedWithoutResult),
        StructuredEvent::Init { raw, .. } => process_start(raw).then_some(Digest::Init),
        StructuredEvent::Unrecognised { raw } if raw.contains("\"autocompact_state\"") => {
            autocompact(raw)
        }
        StructuredEvent::Unrecognised { raw }
            if raw.contains("\"task_") || raw.contains("\"background_tasks_changed\"") =>
        {
            task_line(raw)
        }
        _ => None,
    }
}

/// Whether an `init` line starts the CLI process: 2.1.306 writes `startup_timing` only then.
fn process_start(raw: &str) -> bool {
    raw.contains("\"startup_timing\"")
        && parsed(raw).is_some_and(|line| line.get("startup_timing").is_some())
}

fn parented(parent: &str, blocks: &[ContentBlock]) -> Digest {
    let tool_uses = blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, .. } => Some((id.clone(), name.clone())),
            _ => None,
        })
        .collect();
    Digest::Parented {
        parent: parent.to_string(),
        tool_uses,
    }
}

/// A string field, or `None` for anything else.
fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

/// A `system` task line (R6). No digest for a line without a string `task_id`, a patch that is
/// not an object, a report without a known status or a live set that is not a list.
fn task_line(raw: &str) -> Option<Digest> {
    let line = parsed(raw)?;
    if line["type"].as_str() != Some("system") {
        return None;
    }
    let subtype = line["subtype"].as_str()?;
    if subtype == "background_tasks_changed" {
        let live = line["tasks"]
            .as_array()?
            .iter()
            .filter_map(|task| {
                Some(LiveTask {
                    task_id: text(&task["task_id"])?,
                    description: text(&task["description"]),
                    kind: kind_of(task["task_type"].as_str()),
                })
            })
            .collect();
        return Some(Digest::LiveSet(live));
    }
    let task_id = text(&line["task_id"])?;
    let event = match subtype {
        "task_started" => TaskEvent::Started {
            task_id,
            tool_use_id: text(&line["tool_use_id"]),
            description: text(&line["description"]),
            kind: kind_of(line["task_type"].as_str()),
            background: line["is_backgrounded"].as_bool() == Some(true),
        },
        "task_updated" => {
            let patch = line["patch"].as_object()?;
            TaskEvent::Updated {
                task_id,
                state: patch
                    .get("status")
                    .and_then(Value::as_str)
                    .and_then(reported_state),
                backgrounded: patch.get("is_backgrounded").and_then(Value::as_bool) == Some(true),
                description: patch.get("description").and_then(text),
            }
        }
        "task_progress" => TaskEvent::Progress {
            task_id,
            activity: text(&line["summary"])
                .or_else(|| text(&line["description"]))
                .or_else(|| text(&line["last_tool_name"])),
        },
        "task_notification" => TaskEvent::Notification {
            task_id,
            tool_use_id: text(&line["tool_use_id"]),
            state: reported_state(line["status"].as_str()?)?,
            outcome: text(&line["summary"]),
        },
        _ => return None,
    };
    Some(Digest::Task(event))
}

fn parsed(raw: &str) -> Option<Value> {
    serde_json::from_str(raw).ok()
}

/// A non-negative whole number, or `None` for anything else (a string, a negative, a fraction).
fn count(value: &Value) -> Option<u64> {
    value.as_u64()
}

/// A finite, non-negative amount.
fn amount(value: &Value) -> Option<f64> {
    value.as_f64().filter(|v| v.is_finite() && *v >= 0.0)
}

/// Context used = input + cache creation + cache read + output tokens of the message (R2). A
/// line with no `message.usage` object is no digest; a usage with a figure unreadable leaves
/// `used` `None`.
fn assistant_usage(raw: &str) -> Option<Digest> {
    let line = parsed(raw)?;
    let message = line.get("message")?;
    let usage = message.get("usage").filter(|u| u.is_object())?;
    let used = [
        "input_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
        "output_tokens",
    ]
    .iter()
    .try_fold(0u64, |sum, key| sum.checked_add(count(&usage[*key])?));
    Some(Digest::AssistantUsage {
        model: message["model"].as_str().map(str::to_string),
        used,
    })
}

fn result(usage: Option<&Usage>, total_cost_usd: Option<f64>, raw: &str) -> Digest {
    let tokens = usage.and_then(|u| {
        let tokens = Tokens {
            input: u.input_tokens,
            output: u.output_tokens,
            cache_read: u.cache_read_input_tokens,
            cache_creation: u.cache_creation_input_tokens,
        };
        (!tokens.is_empty()).then_some(tokens)
    });
    let model_usage = parsed(raw).and_then(|line| match line.get("modelUsage") {
        Some(Value::Object(models)) => Some(models.clone()),
        _ => None,
    });
    let windows = model_usage
        .iter()
        .flatten()
        .filter_map(|(model, figures)| Some((model.clone(), count(&figures["contextWindow"])?)))
        .collect();
    let models_cum = model_usage.map(|models| {
        models
            .iter()
            .filter_map(|(model, figures)| Some((model.clone(), model_figures(figures)?)))
            .collect()
    });
    Digest::Result {
        tokens,
        cost_cum: total_cost_usd.filter(|v| v.is_finite() && *v >= 0.0),
        models_cum,
        windows,
    }
}

/// One `modelUsage` entry; `None` when any of its numbers is missing or unreadable.
fn model_figures(figures: &Value) -> Option<ModelFigures> {
    Some(ModelFigures {
        input: count(&figures["inputTokens"])?,
        output: count(&figures["outputTokens"])?,
        cache_read: count(&figures["cacheReadInputTokens"])?,
        cache_creation: count(&figures["cacheCreationInputTokens"])?,
        cost_usd: amount(&figures["costUSD"])?,
        context_window: count(&figures["contextWindow"]),
    })
}

/// `value.threshold` when `value.enabled` is `true`; `None` when disabled or malformed.
fn autocompact(raw: &str) -> Option<Digest> {
    let line = parsed(raw)?;
    if line["type"].as_str() != Some("autocompact_state") {
        return None;
    }
    let value = &line["value"];
    let threshold = (value["enabled"].as_bool() == Some(true))
        .then(|| count(&value["threshold"]))
        .flatten();
    Some(Digest::Autocompact { threshold })
}
