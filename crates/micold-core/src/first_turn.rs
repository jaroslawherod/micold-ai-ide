//! A session's label from the first thing the user typed in it (feature 032).
//!
//! When an AI CLI never titles a conversation, the row still needs something better than "New
//! session". This module turns the conversation's record file into that label: read a bounded
//! prefix (FR-014), find the first **typed** turn by each CLI's own record rules (FR-002, FR-012),
//! and shape it into one line of at most 80 user-perceived characters (FR-003).
//!
//! Record-format knowledge stays below the provider seam: only `provider.rs` calls this. The
//! contract is `specs/032-untitled-session-labels/contracts/first-turn-label.md`; the clause ids
//! (C2, C3, C5) below are its.

use std::io::Read;
use std::path::Path;

use serde_json::Value;
use unicode_segmentation::UnicodeSegmentation;

/// How much of a record file a label is worth reading: 1 MiB (C2.1, research R7). A first turn
/// sits in the first few records of every conversation observed; a conversation whose first turn
/// starts later yields no label rather than a read of the whole file (C2.4).
pub const LABEL_BUDGET_BYTES: u64 = 1024 * 1024;

/// The longest label, in extended grapheme clusters (FR-003).
const MAX_GRAPHEMES: usize = 80;

/// What a cut label ends with, so the reader can see it was cut (C5.4).
const ELLIPSIS: &str = "…";

/// The first [`LABEL_BUDGET_BYTES`] of `path`, cut back to the last complete line (C2.1, C2.2).
///
/// A trailing line with no `\n` — one still being written, or one the bound cut through — is
/// dropped, never parsed. `None` for a file that cannot be opened or read: a label read never fails
/// the session (FR-011).
pub fn read_prefix(path: &Path) -> Option<Vec<u8>> {
    let mut prefix = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(LABEL_BUDGET_BYTES)
        .read_to_end(&mut prefix)
        .ok()?;
    prefix.truncate(complete_len(&prefix));
    Some(prefix)
}

/// `text` as a row label: one line, at most 80 grapheme clusters, or `None` when nothing is left
/// (C5).
///
/// Every run of whitespace, line breaks included, becomes one space and the ends are trimmed
/// (C5.1). A result over the bound keeps its first 79 clusters and gains `…` (C5.4). Counting and
/// cutting by grapheme cluster, never by `char`, is what keeps an emoji ZWJ sequence, a base with
/// its combining marks, or a CJK character whole (C5.5).
pub fn shape_label(text: &str) -> Option<String> {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.is_empty() {
        return None;
    }
    let graphemes: Vec<&str> = one_line.graphemes(true).collect();
    if graphemes.len() <= MAX_GRAPHEMES {
        return Some(one_line);
    }
    let mut cut = graphemes[..MAX_GRAPHEMES - 1].concat();
    cut.push_str(ELLIPSIS);
    Some(cut)
}

/// The label of a `claude` transcript prefix: its first typed turn, shaped (C3), or `None`.
///
/// A record is a **candidate** when it is a `user` record that `claude` did not write itself — not
/// `isMeta`, not `isCompactSummary`, no `toolUseResult`, and no `tool_result` part (C3.1–C3.3).
/// Its text then says what it is:
///
/// - output or notices `claude` or this application inserted are **not turns** (C3.4);
/// - a slash command is a turn only when it sent a prompt: its follower is the expanded prompt,
///   not `<local-command-stdout>` (C3.5a, C3.5b). A command still waiting for its follower on the
///   last line of a running transcript is judged by its tag order (C3.5b′). Its label source is its
///   arguments, else its name (C3.5c);
/// - anything else is a **prompt**, and its text is the label source (C3.6).
///
/// The first turn whose label source is non-empty after [`shape_label`] is the label (C3.7).
pub fn claude_first_turn(prefix: &[u8]) -> Option<String> {
    let records: Vec<Value> = complete_lines(prefix)
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect();
    for (at, record) in records.iter().enumerate() {
        let Some(text) = claude_candidate_text(record) else {
            continue;
        };
        let label = match ClaudeTurn::of(&text) {
            ClaudeTurn::Inserted => continue,
            ClaudeTurn::Prompt => shape_label(&text),
            ClaudeTurn::Command {
                name,
                args,
                opens_with_message,
            } => {
                let sent_a_prompt = match claude_follower(&records[at + 1..]) {
                    Some(follower) => !claude_answered_locally(follower),
                    None => opens_with_message,
                };
                if !sent_a_prompt {
                    continue;
                }
                args.and_then(shape_label).or_else(|| shape_label(name))
            }
        };
        if label.is_some() {
            return label;
        }
    }
    None
}

/// The label of a Copilot event-log prefix: its first typed turn, shaped (C4), or `None`.
///
/// Copilot's log is flatter than `claude`'s — one `user.message` record per turn, with the text of
/// a slash command already resolved into it (C4.4), so there is no command syntax left to
/// interpret. What has to be recognised instead is the text Copilot itself put into the
/// conversation, and it marks that in the **record** rather than in the text: a `data.source`
/// (skill context, instruction discovery) or `data.isAutopilotContinuation` (C4.1).
///
/// The label source is `data.content` — what the user typed — never `data.transformedContent`,
/// which is the same text wrapped in a datetime header and system reminders for the model (C4.2).
pub fn copilot_first_turn(prefix: &[u8]) -> Option<String> {
    complete_lines(prefix)
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .filter_map(|record| copilot_turn_text(&record))
        .find_map(|text| shape_label(&text))
}

/// A Copilot turn's label source, or `None` when the record is not a turn (C4.1, C4.2).
fn copilot_turn_text(record: &Value) -> Option<String> {
    if record.get("type").and_then(Value::as_str) != Some("user.message") {
        return None;
    }
    let data = record.get("data")?;
    // Absent *or* null is a turn: Copilot writes `"source": null` on ordinary messages as often as
    // it omits the key, and treating the explicit null as a source would label nothing at all.
    if data.get("source").is_some_and(|source| !source.is_null()) {
        return None;
    }
    if data.get("isAutopilotContinuation").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    data.get("content")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Every `\n`-terminated line of `bytes`; a trailing partial line is not one (C2.2).
fn complete_lines(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes[..complete_len(bytes)]
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
}

/// The length of `bytes` up to and including its last `\n`: the part made of complete lines.
fn complete_len(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |last| last + 1)
}

/// Text that opens with one of these was inserted by `claude` or this application: command
/// output, shell-mode input and output, and notices (C3.4).
const CLAUDE_INSERTED_PREFIXES: [&str; 4] = [
    "<local-command-",
    "<bash-",
    "<task-notification>",
    "<system-reminder>",
];

/// What a candidate record's text is.
enum ClaudeTurn<'a> {
    /// Not something the user typed (C3.4, C3.5d).
    Inserted,
    /// A slash command (C3.5).
    Command {
        name: &'a str,
        args: Option<&'a str>,
        /// `<command-message>` before `<command-name>` — how a prompt-sending command is written.
        opens_with_message: bool,
    },
    /// Typed text (C3.6).
    Prompt,
}

impl<'a> ClaudeTurn<'a> {
    fn of(text: &'a str) -> Self {
        let text = text.trim_start();
        if CLAUDE_INSERTED_PREFIXES
            .iter()
            .any(|prefix| text.starts_with(prefix))
        {
            return ClaudeTurn::Inserted;
        }
        let opens_with_message = text.starts_with("<command-message>");
        if !opens_with_message && !text.starts_with("<command-name>") {
            return ClaudeTurn::Prompt;
        }
        match tag_text(text, "command-name").map(str::trim) {
            Some(name) if !name.is_empty() => ClaudeTurn::Command {
                name,
                args: tag_text(text, "command-args"),
                opens_with_message,
            },
            _ => ClaudeTurn::Inserted,
        }
    }
}

/// The text between `<tag>` and `</tag>` in `text`, if both are there.
fn tag_text<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let len = text[start..].find(&close)?;
    Some(&text[start..start + len])
}

/// A candidate record's text, or `None` when `claude` or a tool wrote the record (C3.1–C3.3).
fn claude_candidate_text(record: &Value) -> Option<String> {
    let flag = |key: &str| record.get(key).and_then(Value::as_bool) == Some(true);
    if record.get("type").and_then(Value::as_str) != Some("user")
        || flag("isMeta")
        || flag("isCompactSummary")
        || record.get("toolUseResult").is_some()
    {
        return None;
    }
    let content = record.get("message")?.get("content")?;
    if content.as_array().is_some_and(|parts| {
        parts
            .iter()
            .any(|part| part_type(part) == Some("tool_result"))
    }) {
        return None;
    }
    content_text(content)
}

/// `content` as text: a string as it is, or a list's `text` parts joined with a space (an image
/// part contributes nothing, C3.3).
fn content_text(content: &Value) -> Option<String> {
    match content {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => Some(
            parts
                .iter()
                .filter(|part| part_type(part) == Some("text"))
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(" "),
        ),
        _ => None,
    }
}

fn part_type(part: &Value) -> Option<&str> {
    part.get("type").and_then(Value::as_str)
}

/// A slash command's follower: the next `user` record, or `system` record with subtype
/// `local_command`, in the prefix (C3.5a).
fn claude_follower(rest: &[Value]) -> Option<&Value> {
    rest.iter()
        .find(|record| match record.get("type").and_then(Value::as_str) {
            Some("user") => true,
            Some("system") => {
                record.get("subtype").and_then(Value::as_str) == Some("local_command")
            }
            _ => false,
        })
}

/// Whether a command's follower is `claude`'s own answer to it — output of a command it handled
/// itself, which sent no prompt (C3.5a).
fn claude_answered_locally(follower: &Value) -> bool {
    let text = match follower.get("type").and_then(Value::as_str) {
        Some("user") => follower
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(content_text),
        _ => follower
            .get("content")
            .and_then(Value::as_str)
            .map(str::to_string),
    };
    text.is_some_and(|text| {
        let text = text.trim_start();
        text.starts_with("<local-command-stdout>") || text.starts_with("<local-command-stderr>")
    })
}
